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
  version "0.12.1"
  license "MIT"

  RELEASE_BASE = "https://github.com/rolf-thomas/typo3-log-viewer/releases/download/v#{version}".freeze

  on_macos do
    if Hardware::CPU.arm?
      url "#{RELEASE_BASE}/typo3-log-viewer-#{version}-macos-arm64.tar.gz"
      sha256 "7280e65d88b1b7ca49edd5acb85e60a32e09791d69981ad52c06d3a8653f2c05"
    else
      url "#{RELEASE_BASE}/typo3-log-viewer-#{version}-macos-x86_64.tar.gz"
      sha256 "82ac0005cf0377a3254aaa15f345b9aeb9e0c81f85bbc703009f1fbf716581ce"
    end
  end

  on_linux do
    if Hardware::CPU.arm?
      url "#{RELEASE_BASE}/typo3-log-viewer-#{version}-linux-arm64.tar.gz"
      sha256 "6f9618a72fe1c8e82432e7f25bd67691a1d3235a281bbeea854495984b1f4bb6"
    else
      # Statische musl-Binary für maximale Portabilität
      url "#{RELEASE_BASE}/typo3-log-viewer-#{version}-linux-x86_64-musl.tar.gz"
      sha256 "8cec09cc90d1ac3444b762dc3b06320159de38ba31450a0a5717771467188847"
    end
  end

  def install
    bin.install "typo3-log-viewer"
  end

  test do
    assert_match "TYPO3 Log Viewer", shell_output("#{bin}/typo3-log-viewer --help 2>&1", 0)
  end
end
