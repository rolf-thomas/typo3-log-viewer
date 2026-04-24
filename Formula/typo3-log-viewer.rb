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
  version "0.1.1"
  license "MIT"

  RELEASE_BASE = "https://github.com/rolf-thomas/typo3-log-viewer/releases/download/v#{version}".freeze

  on_macos do
    if Hardware::CPU.arm?
      url "#{RELEASE_BASE}/typo3-log-viewer-#{version}-macos-arm64.tar.gz"
      sha256 "b6250f2ffb2351cf0f2822299acb23c5fb475c589b30c7aa7f8c6b089cea1d1b"
    else
      url "#{RELEASE_BASE}/typo3-log-viewer-#{version}-macos-x86_64.tar.gz"
      sha256 "ea37d75fb27f47536d18802015a91dabcf1d257f0c9edca6137fd8311bb73d0a"
    end
  end

  on_linux do
    if Hardware::CPU.arm?
      url "#{RELEASE_BASE}/typo3-log-viewer-#{version}-linux-arm64.tar.gz"
      sha256 "0681a6a3b7559db2fd16a73d9d91aae7f39491018c10cab776010653d93f59ec"
    else
      # Statische musl-Binary für maximale Portabilität
      url "#{RELEASE_BASE}/typo3-log-viewer-#{version}-linux-x86_64-musl.tar.gz"
      sha256 "5719016233137c21579b152388b7b60bce207efd8a9a82c8aa6673c67a6d8e3b"
    end
  end

  def install
    bin.install "typo3-log-viewer"
  end

  test do
    assert_match "TYPO3 Log Viewer", shell_output("#{bin}/typo3-log-viewer --help 2>&1", 0)
  end
end
