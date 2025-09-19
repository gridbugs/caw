use std::path::PathBuf;

fn caw_tmp_dir() -> PathBuf {
    let tmp_dir = std::env::temp_dir();
    tmp_dir.join("caw")
}

pub mod persisten_window_position {
    use super::*;
    use serde::{Deserialize, Serialize};
    use std::fs::{self, File};

    #[derive(Serialize, Deserialize)]
    struct WindowPosition {
        x: i32,
        y: i32,
    }

    fn window_position_dir() -> PathBuf {
        caw_tmp_dir().join("window_positions")
    }

    fn file_path(title: impl AsRef<str>) -> PathBuf {
        let hexxed_title = hex::encode(title.as_ref());
        window_position_dir().join(format!("{}.json", hexxed_title))
    }

    pub fn save(title: impl AsRef<str>, x: i32, y: i32) -> anyhow::Result<()> {
        use std::io::Write;
        fs::create_dir_all(&window_position_dir())?;
        // Hex the title so we can safely use it as part of a file path (ie. so it contains no
        // slashes or other characters that have meaning within a file path).
        let json_string = serde_json::to_string(&WindowPosition { x, y })?;
        let mut file = File::create(file_path(title))?;
        write!(file, "{}", json_string)?;
        Ok(())
    }

    pub fn load(title: impl AsRef<str>) -> anyhow::Result<(i32, i32)> {
        let json_string = fs::read_to_string(file_path(title))?;
        let WindowPosition { x, y } =
            serde_json::from_str(json_string.as_str())?;
        Ok((x, y))
    }
}
