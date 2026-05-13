
class Farscry < Formula
  desc "Image interpreter for automation workflows - local, offline, 8x fewer tokens"
  homepage "https://farscry.dev"
  license "Apache-2.0"
  version "0.1.0"

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/teles-forge/farscry/releases/download/v#{version}/farscry-aarch64-apple-darwin.tar.gz"
      sha256 "__PLACEHOLDER_AARCH64_DARWIN__"
    else
      url "https://github.com/teles-forge/farscry/releases/download/v#{version}/farscry-x86_64-apple-darwin.tar.gz"
      sha256 "__PLACEHOLDER_X86_64_DARWIN__"
    end
  end

  on_linux do
    url "https://github.com/teles-forge/farscry/releases/download/v#{version}/farscry-x86_64-unknown-linux-gnu.tar.gz"
    sha256 "__PLACEHOLDER_X86_64_LINUX__"
  end

  def install
    arch_dir = Dir["farscry-*"].first
    raise "Expected farscry-* directory in archive" if arch_dir.nil?

    bin.install "#{arch_dir}/farscry"

    ort_libs = Dir["#{arch_dir}/libonnxruntime*"] + Dir["#{arch_dir}/onnxruntime*.so*"]
    ort_libs.each do |lib|
      lib_name = File.basename(lib)
      (bin/lib_name).write File.read(lib)
      chmod 0755, bin/lib_name
    end
  end

  def caveats
    <<~EOS
      farscry uses ONNX Runtime for cross-platform OCR. The ORT library
      is bundled alongside the binary in

      For MCP integration:
        farscry setup

      Docs: https://farscry.dev/docs
    EOS
  end

  test do
    assert_match "farscry #{version}", shell_output("#{bin}/farscry --version")
    output = shell_output("#{bin}/farscry extract /nonexistent.png 2>&1", 1)
    assert_match "not found", output
  end
end
