class Yfix < Formula
  desc "Clean and copy terminal text"
  homepage "https://github.com/shimt/yfix"
  license any_of: ["MIT", "Apache-2.0"]

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/shimt/yfix/releases/download/v1.3.2/yfix-aarch64-apple-darwin.tar.gz"
      sha256 "1e0fe7784fde5d3670aa68c312661f8fdf78d2cc8a65d2f48792501e8b5480e6"
    end
  end

  on_linux do
    if Hardware::CPU.intel?
      url "https://github.com/shimt/yfix/releases/download/v1.3.2/yfix-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "a11218884e6f2f6817d31de1ddf597d7cea976c8a4883f818e2117951700e6a0"
    elsif Hardware::CPU.arm?
      url "https://github.com/shimt/yfix/releases/download/v1.3.2/yfix-aarch64-unknown-linux-gnu.tar.gz"
      sha256 "04fff9d4e3f786c039229d2fc3d8365f8ceb6daf8a2e883c9c5aed29c5aa0efa"
    end
  end

  def install
    bin.install "yfix"
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/yfix --version 2>&1")
  end
end
