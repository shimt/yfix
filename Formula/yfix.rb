class Yfix < Formula
  desc "Clean and copy terminal text"
  homepage "https://github.com/shimt/yfix"
  license any_of: ["MIT", "Apache-2.0"]

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/shimt/yfix/releases/download/v1.1.0/yfix-aarch64-apple-darwin.tar.gz"
      sha256 "5e6a68299bc12b66ddffccabd8aa6a6bee7374a8aefc3aff88d914d6bff7a004"
    end
  end

  on_linux do
    if Hardware::CPU.intel?
      url "https://github.com/shimt/yfix/releases/download/v1.1.0/yfix-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "6cfa8880deed1df1c0d9d4a966f61f281aace78547cd1756ba0cee2670b356a5"
    elsif Hardware::CPU.arm?
      url "https://github.com/shimt/yfix/releases/download/v1.1.0/yfix-aarch64-unknown-linux-gnu.tar.gz"
      sha256 "d48ab92bc189424239ceba3a134cb802debfba606240c65db8cbf3b55923ff35"
    end
  end

  def install
    bin.install "yfix"
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/yfix --version 2>&1")
  end
end
