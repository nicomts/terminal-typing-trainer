//! Loading and saving the `Profile` as JSON. The only file I/O in the app.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use color_eyre::Result;
use color_eyre::eyre::{WrapErr, bail};
use directories::ProjectDirs;

use crate::stats::Profile;

const APP_NAME: &str = "terminal-typing-trainer";
const FILE_NAME: &str = "stats.json";

/// Where the stats live, in the platform's data folder, e.g.
/// `~/.local/share/terminal-typing-trainer/stats.json` on Linux or
/// `~/Library/Application Support/terminal-typing-trainer/stats.json` on macOS.
pub fn data_file() -> Result<PathBuf> {
    let Some(dirs) = ProjectDirs::from("", "", APP_NAME) else {
        bail!("could not find a home directory to save stats in");
    };
    Ok(dirs.data_dir().join(FILE_NAME))
}

/// Reads the profile at `path`. A missing file is not an error: it just
/// means no rounds have been played yet.
pub fn load(path: &Path) -> Result<Profile> {
    let contents = match fs::read_to_string(path) {
        Ok(contents) => contents,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Profile::default()),
        Err(error) => {
            return Err(error).wrap_err_with(|| format!("could not read {}", path.display()));
        }
    };
    serde_json::from_str(&contents).wrap_err_with(|| {
        format!(
            "{} is not a valid stats file; fix it or delete it to start over",
            path.display()
        )
    })
}

/// Writes the profile to `path`, creating its folder if needed.
///
/// Writes to a temporary file first, then renames it over the real one.
/// A rename replaces the file in one step, so a crash halfway through
/// leaves the old stats intact instead of a half-written file.
pub fn save(path: &Path, profile: &Profile) -> Result<()> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir).wrap_err_with(|| format!("could not create {}", dir.display()))?;
    }
    let json = serde_json::to_string_pretty(profile)?;

    let temp = path.with_extension("json.tmp");
    fs::write(&temp, json).wrap_err_with(|| format!("could not write {}", temp.display()))?;
    fs::rename(&temp, path).wrap_err_with(|| format!("could not replace {}", path.display()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stats::SymbolRecord;

    /// A fresh folder in the system temp dir, unique to this test run.
    /// (Avoids adding the `tempfile` crate just for tests.)
    fn temp_dir(test: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("{APP_NAME}-{}-{test}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        dir
    }

    fn sample_profile() -> Profile {
        let mut profile = Profile {
            rounds: 3,
            best_wpm: 42.5,
            ..Profile::default()
        };
        profile
            .symbols
            .insert('|', SymbolRecord { seen: 4, missed: 1 });
        profile
            .symbols
            .insert('"', SymbolRecord { seen: 2, missed: 2 });
        profile
    }

    #[test]
    fn save_then_load_gives_the_same_profile() {
        let dir = temp_dir("roundtrip");
        // A folder that doesn't exist yet: `save` must create it.
        let path = dir.join("nested").join(FILE_NAME);
        save(&path, &sample_profile()).unwrap();
        assert_eq!(load(&path).unwrap(), sample_profile());
        // The temporary file was renamed away.
        assert!(!path.with_extension("json.tmp").exists());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn missing_file_is_an_empty_profile() {
        let path = temp_dir("missing").join(FILE_NAME);
        assert_eq!(load(&path).unwrap(), Profile::default());
    }

    #[test]
    fn corrupt_file_is_an_error_naming_the_file() {
        let dir = temp_dir("corrupt");
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join(FILE_NAME);
        fs::write(&path, "{ not json").unwrap();

        let error = load(&path).unwrap_err();
        assert!(format!("{error}").contains(&path.display().to_string()));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn symbols_are_saved_as_json_string_keys() {
        let json = serde_json::to_string(&sample_profile()).unwrap();
        assert!(json.contains(r#""|":{"seen":4,"missed":1}"#), "{json}");
        // A quote key must be escaped to stay valid JSON.
        assert!(json.contains(r#""\"":{"seen":2,"missed":2}"#), "{json}");
    }

    #[test]
    fn missing_fields_get_defaults() {
        // As if written by a version that only knew about `rounds`.
        let profile: Profile = serde_json::from_str(r#"{"rounds": 7}"#).unwrap();
        assert_eq!(profile.rounds, 7);
        assert!(profile.symbols.is_empty());
    }
}
