# Homebrew-Formel für typo3-log-viewer
#
# Diese Datei gehört in ein Tap-Repository, z.B.:
#   github.com/rolf-thomas/homebrew-tools/Formula/typo3-log-viewer.rb
#
# Installation durch Nutzer:
#   brew tap rolf-thomas/tools
#   brew install typo3-log-viewer
#
# Bei jedem Release:
#   1. ./release.sh ausführen → erzeugt dist/ mit Tarballs + SHA256
#   2. GitHub Release anlegen und Tarballs hochladen
#   3. version und sha256 unten aktualisieren
#   4. Diese Datei ins Tap-Repo committen und pushen

class Typo3LogViewer < Formula
  desc "Interactive viewer for TYPO3 log files"
  homepage "https://github.com/rolf-thomas/typo3-log-viewer"
  version "0.4.0"
  license "MIT"

  RELEASE_BASE = "https://github.com/rolf-thomas/typo3-log-viewer/releases/download/v#{version}".freeze

  on_macos do
    if Hardware::CPU.arm?
      url "#{RELEASE_BASE}/typo3-log-viewer-#{version}-macos-arm64.tar.gz"
      sha256 "b96f1a16a9e6e8275bbf68c3bb7dd4704fac2a59afadc3b109e235fc1c5e5951"
    else
      url "#{RELEASE_BASE}/typo3-log-viewer-#{version}-macos-x86_64.tar.gz"
      sha256 "78868b4e9632758efdd11a117a23765330bf64e6358fd1dc20244b44fb645303"
    end
  end

  on_linux do
    if Hardware::CPU.arm?
      url "#{RELEASE_BASE}/typo3-log-viewer-#{version}-linux-arm64.tar.gz"
      sha256 "8f1f58dcd771a0632c8b8c3e8f855242626442b97d728f1f5fab80eaf534bd5a"
    else
      # Statische musl-Binary für maximale Portabilität
      url "#{RELEASE_BASE}/typo3-log-viewer-#{version}-linux-x86_64-musl.tar.gz"
      sha256 "7317996658ce95f221185450e53a0324ceea7185eda035e67caa2c8db4004c06"
    end
  end

  def install
    bin.install "typo3-log-viewer"
  end

  test do
    assert_match "TYPO3 Log Viewer", shell_output("#{bin}/typo3-log-viewer --help 2>&1", 0)
  end
end
