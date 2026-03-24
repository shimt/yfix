class Yfix < Formula
  desc "Clean and copy terminal text"
  homepage "https://github.com/shimt/yfix"
  license any_of: ["MIT", "Apache-2.0"]

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/shimt/yfix/releases/download/v1.2.0/yfix-aarch64-apple-darwin.tar.gz"
      sha256 "8b84ecea663495c3e2d8293d968c3a0ad2c950842cc341bd42fd0dc4b8ba79ab"
    end
  end

  on_linux do
    if Hardware::CPU.intel?
      url "https://github.com/shimt/yfix/releases/download/v1.2.0/yfix-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "121ed06b7384b4fe75632094e5e322da22f8d2faffe0f214fb7e5d450eb5570b"
    elsif Hardware::CPU.arm?
      url "https://github.com/shimt/yfix/releases/download/v1.2.0/yfix-aarch64-unknown-linux-gnu.tar.gz"
      sha256 "29efcb215a50a893305ad6d5837bac62d692996945d8e8d06f4c418168f68ba6"
    end
  end

  def install
    bin.install "yfix"
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/yfix --version 2>&1")
  end
end
