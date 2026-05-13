farscry Homebrew Tap

This directory contains the Homebrew formula for farscry.

Before release: fill in SHA256

The `Formula/farscry.rb` contains `__PLACEHOLDER__` values for SHA256.
After running the release workflow (`git tag v0.1.0 && git push --tags`):

1. Download the `.sha256` files from the GitHub Release:
   ```
   https://github.com/teles-forge/farscry/releases/download/v0.1.0/
   ```

2. Replace the placeholders in `Formula/farscry.rb`:
   ```ruby
   sha256 "__PLACEHOLDER_AARCH64_DARWIN__"  ->  sha256 "abc123..."
   sha256 "__PLACEHOLDER_X86_64_DARWIN__"   ->  sha256 "def456..."
   sha256 "__PLACEHOLDER_X86_64_LINUX__"    ->  sha256 "ghi789..."
   ```

Publishing the tap

```bash
Create the separate tap repository (one-time setup)
gh repo create teles-forge/homebrew-farscry --public
cd /tmp && git clone git@github.com:teles-forge/homebrew-farscry.git
mkdir Formula
cp /path/to/farscry/homebrew/Formula/farscry.rb Formula/
git add . && git commit -m "farscry 0.1.0"
git push

Users can now install:
brew tap teles-forge/farscry
brew install farscry
```

Testing the formula locally

```bash
Lint
brew style homebrew/Formula/farscry.rb

Audit (checks URLs, SHA256, metadata)
brew audit --new-formula homebrew/Formula/farscry.rb

Install from local file (after filling SHA256)
brew install --formula homebrew/Formula/farscry.rb
brew test farscry
```
