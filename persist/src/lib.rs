use serde::{Deserialize, Serialize};
use std::{
    fs::{self, File},
    path::PathBuf,
};

fn caw_tmp_dir() -> PathBuf {
    let tmp_dir = std::env::temp_dir();
    tmp_dir.join("caw")
}

fn containing_dir(name: &str) -> PathBuf {
    caw_tmp_dir().join(name)
}

fn file_path(name: &str, title: impl AsRef<str>) -> PathBuf {
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
    containing_dir(name).join(format!("{}-{}.json", prefix, hexxed_title))
}

fn save(
    name: &str,
    data: &(impl Serialize + for<'a> Deserialize<'a>),
    title: impl AsRef<str>,
) -> anyhow::Result<()> {
    use std::io::Write;
    fs::create_dir_all(&containing_dir(name))?;
    let json_string = serde_json::to_string(data)?;
    let mut file = File::create(file_path(name, title))?;
    write!(file, "{}", json_string)?;
    Ok(())
}

/// Like `save` but prints a warning on failure rather than returning an error value.
fn save_(
    name: &str,
    data: &(impl Serialize + for<'a> Deserialize<'a>),
    title: impl AsRef<str>,
) {
    if let Err(e) = save(name, data, &title) {
        log::warn!("Failed to save {} for {}: {}", name, title.as_ref(), e);
    }
}

fn load<T>(name: &str, title: impl AsRef<str>) -> anyhow::Result<T>
where
    T: Serialize + for<'a> Deserialize<'a>,
{
    let json_string = fs::read_to_string(file_path(name, title))?;
    let t = serde_json::from_str(json_string.as_str())?;
    Ok(t)
}

/// Like `load` but prints a warning on failure rather than returning an error value.
fn load_<T>(name: &str, title: impl AsRef<str>) -> Option<T>
where
    T: Serialize + for<'a> Deserialize<'a>,
{
    match load(name, &title) {
        Ok(t) => Some(t),
        Err(e) => {
            log::warn!("Failed to load {} for {}: {}", name, title.as_ref(), e);
            None
        }
    }
}

/// Implement this when there may be multiple different directories where values of the type will
/// be persisted. It's probably more convenient to just use the `&'static str` impl of this trait
/// in such a case.
pub trait Persist<T>
where
    T: Serialize + for<'a> Deserialize<'a>,
{
    fn name(&self) -> &'static str;

    fn save(&self, data: &T, title: impl AsRef<str>) -> anyhow::Result<()> {
        save(self.name(), data, title)
    }

    /// Like `save` but prints a warning on failure rather than returning an error value.
    fn save_(&self, data: &T, title: impl AsRef<str>) {
        save_(self.name(), data, title)
    }

    fn load(&self, title: impl AsRef<str>) -> anyhow::Result<T> {
        load(self.name(), title)
    }

    /// Like `load` but prints a warning on failure rather than returning an error value.
    fn load_(&self, title: impl AsRef<str>) -> Option<T> {
        load_(self.name(), title)
    }
}

impl<T> Persist<T> for &'static str
where
    T: Serialize + for<'a> Deserialize<'a>,
{
    fn name(&self) -> &'static str {
        self
    }
}

/// Implement this when the type uniquely determines the directory where values of that type will
/// be persisted.
pub trait PersistData: Serialize + for<'a> Deserialize<'a> {
    const NAME: &'static str;

    fn save(&self, title: impl AsRef<str>) -> anyhow::Result<()> {
        Self::NAME.save(self, title)
    }

    /// Like `save` but prints a warning on failure rather than returning an error value.
    fn save_(&self, title: impl AsRef<str>) {
        Self::NAME.save_(self, title)
    }

    fn load(title: impl AsRef<str>) -> anyhow::Result<Self> {
        Self::NAME.load(title)
    }

    /// Like `load` but prints a warning on failure rather than returning an error value.
    fn load_(title: impl AsRef<str>) -> Option<Self> {
        Self::NAME.load_(title)
    }
}
