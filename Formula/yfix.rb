class Yfix < Formula
  desc "Clean and copy terminal text"
  homepage "https://github.com/shimt/yfix"
  license any_of: ["MIT", "Apache-2.0"]

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/shimt/yfix/releases/download/v1.3.3/yfix-aarch64-apple-darwin.tar.gz"
      sha256 "65637c68ad4ae917a59549c41f21cd9a35b0f3830861a0c6c6b12f8388b5d945"
    end
  end

  on_linux do
    if Hardware::CPU.intel?
      url "https://github.com/shimt/yfix/releases/download/v1.3.3/yfix-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "e8bfa78f758d7dc175a95ac4972b90b9e42fa2af7bf81088b214766986d6d3ac"
    elsif Hardware::CPU.arm?
      url "https://github.com/shimt/yfix/releases/download/v1.3.3/yfix-aarch64-unknown-linux-gnu.tar.gz"
      sha256 "3ea829442ee5671ca1891e9fc603f6ec730cdf011093d8a1979971dfd9768b38"
    end
  end

  def install
    bin.install "yfix"
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/yfix --version 2>&1")
  end
end
