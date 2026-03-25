class Yfix < Formula
  desc "Clean and copy terminal text"
  homepage "https://github.com/shimt/yfix"
  license any_of: ["MIT", "Apache-2.0"]

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/shimt/yfix/releases/download/v1.2.1/yfix-aarch64-apple-darwin.tar.gz"
      sha256 "db73eeb8010d00c449aed1361af304c2a8a5b8465862ef54d6c00ac8f6ce6d35"
    end
  end

  on_linux do
    if Hardware::CPU.intel?
      url "https://github.com/shimt/yfix/releases/download/v1.2.1/yfix-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "0c2e68e3f82ecb313ff79ef7d3e088b6a10eefbe2669c2ae096f8ecfb25afbd2"
    elsif Hardware::CPU.arm?
      url "https://github.com/shimt/yfix/releases/download/v1.2.1/yfix-aarch64-unknown-linux-gnu.tar.gz"
      sha256 "eabbcc1db73ce5585b4ca2c12868515924e9c8f4bedfd5e94e42a138d2edfc4b"
    end
  end

  def install
    bin.install "yfix"
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/yfix --version 2>&1")
  end
end
