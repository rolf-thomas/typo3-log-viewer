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
  version "0.11.0"
  license "MIT"

  RELEASE_BASE = "https://github.com/rolf-thomas/typo3-log-viewer/releases/download/v#{version}".freeze

  on_macos do
    if Hardware::CPU.arm?
      url "#{RELEASE_BASE}/typo3-log-viewer-#{version}-macos-arm64.tar.gz"
      sha256 "cdae1bc86772f007c7f2bf6947fc3ef057fa8ca4b59cb315b4b43daf89d18087"
    else
      url "#{RELEASE_BASE}/typo3-log-viewer-#{version}-macos-x86_64.tar.gz"
      sha256 "d0a7caf084721a305b3fff8e783c94bbb63dd5bdaef4572b9203dfbeef787d0c"
    end
  end

  on_linux do
    if Hardware::CPU.arm?
      url "#{RELEASE_BASE}/typo3-log-viewer-#{version}-linux-arm64.tar.gz"
      sha256 "5020676caf91c5b028074741c9bd2d43cefba8f603948909bbb7693b0baf75f3"
    else
      # Statische musl-Binary für maximale Portabilität
      url "#{RELEASE_BASE}/typo3-log-viewer-#{version}-linux-x86_64-musl.tar.gz"
      sha256 "37a21ebb044f1a1281d89f229334f760150e56f6929771a256d6751c7fc443a3"
    end
  end

  def install
    bin.install "typo3-log-viewer"
  end

  test do
    assert_match "TYPO3 Log Viewer", shell_output("#{bin}/typo3-log-viewer --help 2>&1", 0)
  end
end
