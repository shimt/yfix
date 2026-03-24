class Yfix < Formula
  desc "Clean and copy terminal text"
  homepage "https://github.com/shimt/yfix"
  license any_of: ["MIT", "Apache-2.0"]

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/shimt/yfix/releases/download/v1.0.0/yfix-aarch64-apple-darwin.tar.gz"
      sha256 "e3f06222b70925c4b8769e091ff94b10abeab3a6780d06c9de2c58e485f7c8ee"
    end
  end

  on_linux do
    if Hardware::CPU.intel?
      url "https://github.com/shimt/yfix/releases/download/v1.0.0/yfix-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "3d4d418869ff386bf9c81f57cd3ce6d09180c9f11cba54ceee7aadd06eadf1ae"
    elsif Hardware::CPU.arm?
      url "https://github.com/shimt/yfix/releases/download/v1.0.0/yfix-aarch64-unknown-linux-gnu.tar.gz"
      sha256 "6b0dbed878bef32c8e9d037dd7369dbb031a1816e1ff13c313e1b5082f4a4590"
    end
  end

  def install
    bin.install "yfix"
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/yfix --version 2>&1")
  end
end
