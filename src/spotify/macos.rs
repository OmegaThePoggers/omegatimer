use super::Playback;
use std::process::Command;

// Check running applications before constructing the scripting target.
const SCRIPT: &str = r#"
ObjC.import('AppKit');
function run() {
  if (!$.NSRunningApplication.runningApplicationsWithBundleIdentifier('com.spotify.client').count) return '';
  const app = Application('com.spotify.client');
  const state = app.playerState();
  if (state !== 'playing' && state !== 'paused') return '';
  const track = app.currentTrack();
  return JSON.stringify([state, track.name(), track.artist()]);
}
"#;

pub(super) fn read() -> Option<Playback> {
    let output = Command::new("/usr/bin/osascript")
        .args(["-l", "JavaScript", "-e", SCRIPT])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let [state, title, artist]: [String; 3] = serde_json::from_slice(&output.stdout).ok()?;
    Some(Playback {
        title,
        artist,
        paused: state == "paused",
    })
}
