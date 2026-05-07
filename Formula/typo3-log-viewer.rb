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
  version "0.7.0"
  license "MIT"

  RELEASE_BASE = "https://github.com/rolf-thomas/typo3-log-viewer/releases/download/v#{version}".freeze

  on_macos do
    if Hardware::CPU.arm?
      url "#{RELEASE_BASE}/typo3-log-viewer-#{version}-macos-arm64.tar.gz"
      sha256 "3d42748f4b6f2f5b9f96c23e5f1a9617b660d935c39828d6df9fd7e68d42150e"
    else
      url "#{RELEASE_BASE}/typo3-log-viewer-#{version}-macos-x86_64.tar.gz"
      sha256 "26d77cba4250edf533f081297edbce0125c6c9f59529d8797b2556cad4374407"
    end
  end

  on_linux do
    if Hardware::CPU.arm?
      url "#{RELEASE_BASE}/typo3-log-viewer-#{version}-linux-arm64.tar.gz"
      sha256 "3658367d3b34eabd09fc2692eea0bb3b743b621f7a4c8615410e3f4483ad5948"
    else
      # Statische musl-Binary für maximale Portabilität
      url "#{RELEASE_BASE}/typo3-log-viewer-#{version}-linux-x86_64-musl.tar.gz"
      sha256 "19b280e6dc839921b5caf8c0d67aaf70c277a8516afb5bcb48a1e7a6bdc1a1c7"
    end
  end

  def install
    bin.install "typo3-log-viewer"
  end

  test do
    assert_match "TYPO3 Log Viewer", shell_output("#{bin}/typo3-log-viewer --help 2>&1", 0)
  end
end
