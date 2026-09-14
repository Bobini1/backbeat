cask "backbeat" do
  version "0.5.0"
  sha256 "cec9afefc780a8628c5ccb3061c351e0c406c49b3803ab13948f3f1e158bb022"

  url "https://github.com/zkldi/backbeat/releases/download/v#{version}/Backbeat_#{version}_universal.dmg"
  name "Backbeat"
  desc "Manage and install rhythm game charts"
  homepage "https://backbeat.ac/"

  depends_on macos: :big_sur

  app "Backbeat.app"

  caveats <<~EOS
    Backbeat is not signed with an Apple Developer ID. If macOS blocks the
    first launch, try opening Backbeat and then allow it under:

      System Settings → Privacy & Security → Open Anyway
  EOS
end
