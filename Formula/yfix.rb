class Yfix < Formula
  desc "Clean and copy terminal text"
  homepage "https://github.com/shimt/yfix"
  license any_of: ["MIT", "Apache-2.0"]

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/shimt/yfix/releases/download/v1.3.1/yfix-aarch64-apple-darwin.tar.gz"
      sha256 "95b4221b63b8d26d34efbf30117abc235435f4f2fac0ef4550dd3361757bc7e4"
    end
  end

  on_linux do
    if Hardware::CPU.intel?
      url "https://github.com/shimt/yfix/releases/download/v1.3.1/yfix-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "9ea3a10aa086d8cf0233d7dfe5e1f6e6e39d9e45d44e4e94631b32d5e531ff04"
    elsif Hardware::CPU.arm?
      url "https://github.com/shimt/yfix/releases/download/v1.3.1/yfix-aarch64-unknown-linux-gnu.tar.gz"
      sha256 "ecc30098374703c65a630280b993c6fe2bf58ec2cae10516e169c184bfd87ba7"
    end
  end

  def install
    bin.install "yfix"
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/yfix --version 2>&1")
  end
end
