//! App-wide presentation preferences. Paths are owned by Rust, never supplied by the WebView.
use serde::{Deserialize, Serialize};
use std::{fs, io::Write, path::Path};

#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum TextSize {
    Small,
    #[default]
    Default,
    Large,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UiPreferences {
    schema_version: u8,
    pub text_size: TextSize,
}

impl Default for UiPreferences {
    fn default() -> Self {
        Self {
            schema_version: 1,
            text_size: TextSize::Default,
        }
    }
}

pub fn load(root: &Path) -> Result<UiPreferences, &'static str> {
    let bytes = match fs::read(root.join("ui-preferences.json")) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(UiPreferences::default())
        }
        Err(_) => return Err("UI_PREFERENCES_READ_FAILED"),
    };
    let preferences: UiPreferences =
        serde_json::from_slice(&bytes).map_err(|_| "UI_PREFERENCES_INVALID")?;
    if preferences.schema_version != 1 {
        return Err("UI_PREFERENCES_INVALID");
    }
    Ok(preferences)
}

pub fn save(root: &Path, text_size: TextSize) -> Result<(), &'static str> {
    fs::create_dir_all(root).map_err(|_| "UI_PREFERENCES_WRITE_FAILED")?;
    let temporary = root.join(format!(".ui-preferences-{}.tmp", uuid::Uuid::new_v4()));
    let result = (|| {
        let bytes = serde_json::to_vec(&UiPreferences {
            text_size,
            ..UiPreferences::default()
        })
        .map_err(|_| "UI_PREFERENCES_WRITE_FAILED")?;
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)
            .map_err(|_| "UI_PREFERENCES_WRITE_FAILED")?;
        file.write_all(&bytes)
            .and_then(|()| file.sync_all())
            .map_err(|_| "UI_PREFERENCES_WRITE_FAILED")?;
        drop(file);
        fs::rename(&temporary, root.join("ui-preferences.json"))
            .map_err(|_| "UI_PREFERENCES_WRITE_FAILED")
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_and_all_sizes_survive_reload_without_touching_other_settings() {
        let root = std::env::temp_dir().join(format!("manuscriptdock-ui-{}", uuid::Uuid::new_v4()));
        assert_eq!(load(&root).unwrap().text_size, TextSize::Default);
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("other-settings.json"), "synthetic").unwrap();
        for size in [TextSize::Small, TextSize::Large, TextSize::Default] {
            save(&root, size).unwrap();
            assert_eq!(load(&root).unwrap().text_size, size);
        }
        assert_eq!(
            fs::read_to_string(root.join("other-settings.json")).unwrap(),
            "synthetic"
        );
        assert_eq!(fs::read_dir(&root).unwrap().count(), 2);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn invalid_data_and_unwritable_locations_return_stable_codes() {
        let root = std::env::temp_dir().join(format!("manuscriptdock-ui-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&root).unwrap();
        for value in [
            r#"{"schemaVersion":1,"textSize":"huge"}"#,
            r#"{"schemaVersion":2,"textSize":"large"}"#,
            "invalid",
        ] {
            fs::write(root.join("ui-preferences.json"), value).unwrap();
            assert_eq!(load(&root).unwrap_err(), "UI_PREFERENCES_INVALID");
        }
        let file = root.join("not-a-directory");
        fs::write(&file, "synthetic").unwrap();
        assert_eq!(
            save(&file, TextSize::Large),
            Err("UI_PREFERENCES_WRITE_FAILED")
        );
        assert_eq!(load(&file).unwrap_err(), "UI_PREFERENCES_READ_FAILED");
        fs::remove_dir_all(root).unwrap();
    }
}
