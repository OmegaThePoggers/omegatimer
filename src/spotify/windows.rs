use super::Playback;
use ::windows::{
    Media::Control::{
        GlobalSystemMediaTransportControlsSessionManager as Manager,
        GlobalSystemMediaTransportControlsSessionPlaybackStatus as Status,
    },
    Win32::System::WinRT::{RO_INIT_MULTITHREADED, RoInitialize, RoUninitialize},
};

pub(super) struct Reader {
    initialized: bool,
    manager: Option<Manager>,
}

impl Reader {
    pub fn new() -> Self {
        // The media worker owns this apartment for its whole lifetime.
        Self {
            initialized: unsafe { RoInitialize(RO_INIT_MULTITHREADED) }.is_ok(),
            manager: None,
        }
    }

    pub fn read(&mut self) -> Option<Playback> {
        if !self.initialized {
            return None;
        }
        if self.manager.is_none() {
            self.manager = Manager::RequestAsync().ok()?.get().ok();
        }
        let sessions = match self.manager.as_ref()?.GetSessions() {
            Ok(s) => s,
            Err(_) => {
                self.manager = None;
                return None;
            }
        };
        for session in sessions {
            let Ok(id) = session.SourceAppUserModelId() else {
                continue;
            };
            let id = id.to_string_lossy().to_ascii_lowercase();
            if !(id == "spotify.exe"
                || id == "spotify"
                || id.starts_with("spotifyab.spotifymusic_")
                || id.ends_with(r"\spotify.exe"))
            {
                continue;
            }
            let status = session.GetPlaybackInfo().ok()?.PlaybackStatus().ok()?;
            if status != Status::Playing && status != Status::Paused {
                continue;
            }
            let media = session.TryGetMediaPropertiesAsync().ok()?.get().ok()?;
            return Some(Playback {
                title: media.Title().ok()?.to_string_lossy(),
                artist: media.Artist().ok()?.to_string_lossy(),
                paused: status == Status::Paused,
            });
        }
        None
    }
}

impl Drop for Reader {
    fn drop(&mut self) {
        self.manager = None;
        if self.initialized {
            unsafe { RoUninitialize() };
        }
    }
}
