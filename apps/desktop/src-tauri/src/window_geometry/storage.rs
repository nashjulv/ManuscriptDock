use super::policy::Saved;
use std::{
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
};

const FILE: &str = "window-geometry-v1.json";

pub fn load(root: &Path) -> Saved {
    // Preserve the old physical-pixel file; it has no source DPI for a safe conversion.
    let Ok(file) = fs::File::open(root.join(FILE)) else {
        return Saved::default();
    };
    let mut bytes = Vec::new();
    if file.take(256 * 1024 + 1).read_to_end(&mut bytes).is_err() || bytes.len() > 256 * 1024 {
        return Saved::default();
    }
    serde_json::from_slice::<Saved>(&bytes)
        .ok()
        .filter(Saved::valid)
        .unwrap_or_default()
}

pub fn save(root: &Path, saved: &Saved) -> std::io::Result<()> {
    if !saved.valid() {
        return Err(std::io::ErrorKind::InvalidData.into());
    }
    fs::create_dir_all(root)?;
    // Backup migration input once, without renaming/deleting the original.
    let old = root.join(".window-state.json");
    if old.is_file() {
        if let Ok(mut backup) = fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(root.join(".window-state.pre-logical.json"))
        {
            if let Ok(file) = fs::File::open(old) {
                let _ = std::io::copy(&mut file.take(256 * 1024), &mut backup);
                let _ = backup.sync_all();
            }
        }
    }
    let temporary: PathBuf = root.join(format!(".window-geometry-{}.tmp", uuid::Uuid::new_v4()));
    let result = (|| {
        let bytes = serde_json::to_vec_pretty(saved)?;
        let mut file = fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&temporary)?;
        file.write_all(&bytes)?;
        file.sync_all()?;
        drop(file);
        fs::rename(&temporary, root.join(FILE))
    })();
    if result.is_err() {
        let _ = fs::remove_file(temporary);
    }
    result
}
