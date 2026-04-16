cask "mux" do
  version "0.1.1"
  sha256 "9fd8b24203bb6a12c1e462abb32303a4ab4801372cabe41c0ba6f3b6d428f003"

  url "https://github.com/kay404/Mux/releases/download/v#{version}/Mux_#{version}_aarch64.dmg"
  name "Mux"
  desc "macOS menu bar app for switching between developer tool windows"
  homepage "https://github.com/kay404/Mux"

  app "Mux.app"

  zap trash: [
    "~/Library/Application Support/com.kay.mux",
    "~/Library/Preferences/com.kay.mux.plist",
    "~/Library/Saved Application State/com.kay.mux.savedState",
    "~/.cache/mux",
  ]
end
