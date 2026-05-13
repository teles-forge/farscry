#!/usr/bin/env node
'use strict';


const https = require('https');
const fs = require('fs');
const path = require('path');
const crypto = require('crypto');
const { execSync } = require('child_process');
const { promisify } = require('util');
const stream = require('stream');
const pipeline = promisify(stream.pipeline);

const PKG = require('./package.json');
const VERSION = PKG.version;
const REPO = 'teles-forge/farscry';
const BASE_URL = `https://github.com/${REPO}/releases/download/v${VERSION}`;


function getAssetName() {
  const { platform, arch } = process;
  if (platform === 'darwin' && arch === 'arm64') return 'farscry-aarch64-apple-darwin';
  if (platform === 'darwin' && arch === 'x64')   return 'farscry-x86_64-apple-darwin';
  if (platform === 'linux'  && arch === 'x64')   return 'farscry-x86_64-unknown-linux-gnu';
  if (platform === 'win32'  && arch === 'x64')   return 'farscry-x86_64-pc-windows-msvc';
  throw new Error(
    `Unsupported platform: ${platform} ${arch}\n` +
    `farscry supports: darwin/linux x64, darwin arm64, win32 x64.\n` +
    `Open an issue: https://github.com/${REPO}/issues`
  );
}


function downloadToFile(url, dest) {
  return new Promise((resolve, reject) => {
    const file = fs.createWriteStream(dest);

    function follow(currentUrl) {
      https.get(currentUrl, { headers: { 'User-Agent': `farscry-npm/${VERSION}` } }, res => {
        const { statusCode, headers } = res;

        if ([301, 302, 303, 307, 308].includes(statusCode)) {
          res.resume();
          return follow(headers.location);
        }
        if (statusCode !== 200) {
          res.resume();
          file.destroy();
          return reject(new Error(`HTTP ${statusCode} downloading ${currentUrl}`));
        }

        res.pipe(file);
        file.on('finish', () => file.close(resolve));
        file.on('error', err => { fs.unlink(dest, () => {}); reject(err); });
        res.on('error', reject);
      }).on('error', reject);
    }

    follow(url);
  });
}


function sha256OfFile(filePath) {
  const hash = crypto.createHash('sha256');
  hash.update(fs.readFileSync(filePath));
  return hash.digest('hex');
}


function extract(archivePath, destDir, assetName, isWindows) {
  if (isWindows) {
    execSync(
      `powershell -NoProfile -Command "` +
        `Expand-Archive -Path '${archivePath}' ` +
        `-DestinationPath '${destDir}' -Force"`,
      { stdio: 'inherit' }
    );
  } else {
    execSync(`tar -xzf "${archivePath}" -C "${destDir}"`, { stdio: 'inherit' });
  }


  const nestedDir = path.join(destDir, assetName);
  if (!fs.existsSync(nestedDir)) {
    throw new Error(`Expected directory ${nestedDir} inside archive`);
  }
  return nestedDir;
}


async function main() {
  const assetName  = getAssetName();
  const isWindows  = process.platform === 'win32';
  const archiveExt = isWindows ? 'zip' : 'tar.gz';
  const binaryName = isWindows ? 'farscry.exe' : 'farscry';

  const archiveUrl = `${BASE_URL}/${assetName}.${archiveExt}`;
  const sha256Url  = `${BASE_URL}/${assetName}.sha256`;

  const binDir      = path.join(__dirname, 'bin');
  const archivePath = path.join(binDir, `${assetName}.${archiveExt}`);
  const sha256Path  = path.join(binDir, `${assetName}.sha256`);
  const binaryPath  = path.join(binDir, binaryName);

  fs.mkdirSync(binDir, { recursive: true });


  console.log(`[farscry] Installing v${VERSION} (${assetName})…`);
  console.log(`[farscry] Downloading: ${archiveUrl}`);
  await downloadToFile(archiveUrl, archivePath);
  console.log(`[farscry] Archive downloaded (${(fs.statSync(archivePath).size / 1024 / 1024).toFixed(1)} MB)`);


  await downloadToFile(sha256Url, sha256Path);
  const expectedSha256 = fs.readFileSync(sha256Path, 'utf8').trim().split(/\s+/)[0].toLowerCase();
  fs.unlinkSync(sha256Path);


  const nestedDir = extract(archivePath, binDir, assetName, isWindows);


  const extractedBinary = path.join(nestedDir, binaryName);
  if (fs.existsSync(binaryPath)) fs.unlinkSync(binaryPath);
  fs.renameSync(extractedBinary, binaryPath);


  for (const entry of fs.readdirSync(nestedDir)) {
    if (entry.startsWith('libonnxruntime') || entry.startsWith('onnxruntime')) {
      const src = path.join(nestedDir, entry);
      const dst = path.join(binDir, entry);
      if (fs.existsSync(dst)) fs.unlinkSync(dst);
      fs.renameSync(src, dst);
      console.log(`[farscry] Bundled ORT library: ${entry}`);
    }
  }


  fs.rmSync(archivePath, { force: true });
  fs.rmSync(nestedDir, { recursive: true, force: true });


  const actualSha256 = sha256OfFile(binaryPath).toLowerCase();

  if (actualSha256 !== expectedSha256) {
    fs.unlinkSync(binaryPath);
    const msg =
      `[farscry] *** SHA256 MISMATCH - installation aborted ***\n` +
      `  expected : ${expectedSha256}\n` +
      `  actual   : ${actualSha256}\n` +
      `\n` +
      `  The binary has been removed. This may indicate a corrupted download\n` +
      `  or a man-in-the-middle attack. Please retry the installation, or\n` +
      `  report the issue at https://github.com/${REPO}/issues`;
    console.error(msg);
    process.exit(1);
  }

  console.log(`[farscry] SHA256 verified OK`);


  if (!isWindows) {
    fs.chmodSync(binaryPath, 0o755);

    for (const entry of fs.readdirSync(binDir)) {
      if (entry.startsWith('libonnxruntime')) {
        fs.chmodSync(path.join(binDir, entry), 0o755);
      }
    }
  }

  console.log(`[farscry] OK Installed to ${binaryPath}`);
  console.log(`[farscry] Run: farscry setup`);
}

main().catch(err => {
  console.error('[farscry] Installation failed:', err.message);
  process.exit(1);
});
