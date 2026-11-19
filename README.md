# Ω OmegaTimer

A clickable study timer for the terminal. Chain study and rest sessions, choose the character that draws the clock, and a dancing ASCII cat fills spare space as the window resizes.

## Download

[Download the latest release](https://github.com/OmegaThePoggers/omegatimer/releases/latest). Choose the archive for your operating system and processor. The prebuilt downloads run without Rust.

| System | Archive | Run after extraction |
| --- | --- | --- |
| Windows, Intel or AMD 64-bit | `omegatimer-windows-x64.zip` | Open PowerShell in the extracted folder and run `.\omegatimer.exe 25:5` |
| Windows on ARM64 | `omegatimer-windows-arm64.zip` | Same PowerShell command |
| macOS, Intel | `omegatimer-macos-intel.tar.gz` | Run `./omegatimer 25:5` in the extracted folder |
| Linux, Intel or AMD 64-bit | `omegatimer-linux-x64.tar.gz` | Run `./omegatimer 25:5` in the extracted folder |
| Linux, ARM64 | `omegatimer-linux-arm64.tar.gz` | Same Linux command |

Use Windows Terminal for the colors, Unicode clock, and clickable controls. To run `omegatimer` from any folder, put the extracted executable in a directory on your PATH.

With a current Rust toolchain, you can also build and install from source on any of these platforms.

```sh
cargo install --locked --git https://github.com/OmegaThePoggers/omegatimer.git --tag v0.2.0
```

## Use

```sh
omegatimer                    # Click to configure your first session
omegatimer 25:5 50:10         # 25 min study, 5 min rest, then 50 and 10
omegatimer --loop 25:5 50:10  # Repeat the whole chain
omegatimer --glyph '#' 25:5   # Choose the clock fill
```

Click **Queue** to append the displayed study/rest pair. Space pauses or resumes, L toggles looping, and Q quits.

## Spotify Desktop

Start Spotify Desktop on the same computer and play a track. OmegaTimer reads the title, artist, and play/pause state locally and refreshes the Spotify row about every two seconds. Closing Spotify clears the row. You can open either app first.

Linux uses the MPRIS session bus, Windows uses system media sessions, and Intel macOS uses Spotify’s scripting interface. On macOS, allow Automation access from your terminal if prompted. No Spotify developer account, API key, or separate login is required. Phone-only and browser playback are outside the desktop integration.
