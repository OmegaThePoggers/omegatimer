use std::{sync::mpsc, thread, time::Duration};
use unicode_width::UnicodeWidthChar;

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "windows")]
mod windows;

#[derive(Default, PartialEq, Eq, Clone)]
pub struct Playback {
    title: String,
    artist: String,
    paused: bool,
}

impl Playback {
    pub fn row(&self, width: u16) -> String {
        let mut text = String::from("Spotify   ");
        if self.title.trim().is_empty() {
            text.push_str("·   —");
        } else {
            text.push_str(if self.paused { "Ⅱ   " } else { "▶   " });
            text.push_str(&self.title);
            if !self.artist.trim().is_empty() {
                text.push_str(" — ");
                text.push_str(&self.artist);
            }
        }
        let mut row = String::new();
        let mut columns = 0;
        for c in text.chars().filter(|c| !c.is_control()) {
            let size = c.width().unwrap_or(0);
            if columns + size > usize::from(width) {
                break;
            }
            row.push(c);
            columns += size;
        }
        row
    }
}

pub fn watch() -> mpsc::Receiver<Playback> {
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        #[cfg(target_os = "linux")]
        let mut reader = linux::Reader::default();
        #[cfg(target_os = "windows")]
        let mut reader = windows::Reader::new();

        loop {
            #[cfg(any(target_os = "linux", target_os = "windows"))]
            let track = reader.read();
            #[cfg(target_os = "macos")]
            let track = macos::read();
            #[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "macos")))]
            let track: Option<Playback> = None;

            if tx.send(track.unwrap_or_default()).is_err() {
                break;
            }
            thread::sleep(Duration::from_secs(2));
        }
    });
    rx
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn row_fits_tile_and_removes_terminal_controls() {
        let mut p = Playback {
            title: "界\x1b\nZ".into(),
            artist: "Artist".into(),
            paused: false,
        };
        assert_eq!(p.row(15), "Spotify   ▶   ");
        assert_eq!(p.row(16), "Spotify   ▶   界");
        p.paused = true;
        assert_eq!(p.row(60), "Spotify   Ⅱ   界Z — Artist");
        assert_eq!(Playback::default().row(60), "Spotify   ·   —");
    }
}
