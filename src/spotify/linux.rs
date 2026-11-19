use super::Playback;
use std::{collections::HashMap, time::Duration};
use zbus::{
    blocking::{Connection, connection::Builder},
    zvariant::OwnedValue,
};
type Properties = HashMap<String, OwnedValue>;

#[derive(Default)]
pub(super) struct Reader(Option<Connection>);

impl Reader {
    pub fn read(&mut self) -> Option<Playback> {
        if self.0.is_none() {
            self.0 = Builder::session()
                .ok()?
                .method_timeout(Duration::from_secs(2))
                .build()
                .ok();
        }
        let conn = self.0.as_ref()?;
        // Resolve the unique owner first so querying metadata can never launch Spotify.
        let owner = match conn.call_method(
            Some("org.freedesktop.DBus"),
            "/org/freedesktop/DBus",
            Some("org.freedesktop.DBus"),
            "GetNameOwner",
            &"org.mpris.MediaPlayer2.spotify",
        ) {
            Ok(reply) => reply.body().deserialize::<String>().ok()?,
            Err(zbus::Error::MethodError(..)) => return None,
            Err(_) => {
                self.0 = None;
                return None;
            }
        };
        let reply = conn
            .call_method(
                Some(owner.as_str()),
                "/org/mpris/MediaPlayer2",
                Some("org.freedesktop.DBus.Properties"),
                "GetAll",
                &"org.mpris.MediaPlayer2.Player",
            )
            .ok()?;
        let mut props: Properties = reply.body().deserialize().ok()?;
        let status = String::try_from(props.remove("PlaybackStatus")?).ok()?;
        if status != "Playing" && status != "Paused" {
            return None;
        }
        let mut meta = Properties::try_from(props.remove("Metadata")?).ok()?;
        let title = String::try_from(meta.remove("xesam:title")?).ok()?;
        let artist = meta
            .remove("xesam:artist")
            .and_then(|v| Vec::<String>::try_from(v).ok())
            .unwrap_or_default()
            .join(", ");
        Some(Playback {
            title,
            artist,
            paused: status == "Paused",
        })
    }
}
