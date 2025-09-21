use serde::{Deserialize, Serialize};
use std::{
    fs::{self, File},
    path::PathBuf,
};

fn caw_tmp_dir() -> PathBuf {
    let tmp_dir = std::env::temp_dir();
    tmp_dir.join("caw")
}

pub trait PersistentData: Serialize + for<'a> Deserialize<'a> {
    const NAME: &'static str;

    fn containing_dir() -> PathBuf {
        caw_tmp_dir().join(Self::NAME)
    }

    fn file_path(title: impl AsRef<str>) -> PathBuf {
        let title = title.as_ref();
        // Hex the title so we can safely use it as part of a file path (ie. so it contains no
        // slashes or other characters that have meaning within a file path).
        let hexxed_title = hex::encode(title);
        let prefix = title
            .chars()
            .take_while(|&c| {
                c.is_ascii_alphanumeric() || c == '_' || c == '-' || c == ' '
            })
            .map(|c| if c == ' ' { '-' } else { c })
            .collect::<String>();
        Self::containing_dir().join(format!("{}-{}.json", prefix, hexxed_title))
    }

    fn save(&self, title: impl AsRef<str>) -> anyhow::Result<()> {
        use std::io::Write;
        fs::create_dir_all(&Self::containing_dir())?;
        let json_string = serde_json::to_string(self)?;
        let mut file = File::create(Self::file_path(title))?;
        write!(file, "{}", json_string)?;
        Ok(())
    }

    /// Like `save` but prints a warning on failure rather than returning an error value.
    fn save_(&self, title: impl AsRef<str>) {
        if let Err(e) = self.save(&title) {
            log::warn!(
                "Failed to save {} for {}: {}",
                Self::NAME,
                title.as_ref(),
                e
            );
        }
    }

    fn load(title: impl AsRef<str>) -> anyhow::Result<Self> {
        let json_string = fs::read_to_string(Self::file_path(title))?;
        let t = serde_json::from_str(json_string.as_str())?;
        Ok(t)
    }

    /// Like `load` but prints a warning on failure rather than returning an error value.
    fn load_(title: impl AsRef<str>) -> Option<Self> {
        match Self::load(&title) {
            Ok(t) => Some(t),
            Err(e) => {
                log::warn!(
                    "Failed to load {} for {}: {}",
                    Self::NAME,
                    title.as_ref(),
                    e
                );
                None
            }
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
pub struct WindowPosition {
    pub x: i32,
    pub y: i32,
}

impl PersistentData for WindowPosition {
    const NAME: &'static str = "window_position";
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
pub struct WindowSize {
    pub width: u32,
    pub height: u32,
}

impl PersistentData for WindowSize {
    const NAME: &'static str = "window_size";
}
