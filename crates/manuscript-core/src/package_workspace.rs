use crate::{AppError, CompiledPackage, ProjectStore};
use serde::Serialize;
use std::{
    fs, io,
    path::{Component, Path, PathBuf},
};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceEntry {
    pub relative_path: String,
    pub directory: bool,
    pub size_bytes: u64,
    pub modified_at_unix_ms: u64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceListing {
    pub root_path: String,
    pub entries: Vec<WorkspaceEntry>,
}

fn failure() -> AppError {
    AppError::new("WORKSPACE_IO_FAILED", true)
}

pub fn workspace_root(store: &ProjectStore, project_id: &str) -> Result<PathBuf, AppError> {
    let root = store.package_workspace_path(project_id)?;
    let initializing = !root.exists();
    // Refuse links even at the root; externally replacing a folder must not redirect writes.
    // Check the managed project subtree. System aliases such as macOS /var -> /private/var are valid.
    for path in root.ancestors().take(4) {
        if fs::symlink_metadata(path).is_ok_and(|m| m.file_type().is_symlink()) {
            return Err(AppError::new("WORKSPACE_PATH_INVALID", true));
        }
    }
    fs::create_dir_all(&root).map_err(|_| failure())?;
    for name in ["submission", "author-tools"] {
        let path = workspace_path(&root, name)?;
        fs::create_dir_all(path).map_err(|_| failure())?;
    }
    if initializing {
        let previous = store.previous_package_workspace_path(project_id)?;
        if previous.is_dir()
            && !fs::symlink_metadata(&previous)
                .map_err(|_| failure())?
                .file_type()
                .is_symlink()
        {
            let mut prior_entries = Vec::new();
            walk(&previous, &previous, 0, &mut prior_entries)?;
            for entry in prior_entries {
                let destination = workspace_path(&root, &entry.relative_path)?;
                if entry.directory {
                    fs::create_dir_all(&destination).map_err(|_| failure())?;
                } else {
                    fs::create_dir_all(destination.parent().ok_or_else(failure)?)
                        .map_err(|_| failure())?;
                    copy_new(
                        &workspace_path(&previous, &entry.relative_path)?,
                        &destination,
                    )?;
                }
            }
        }
        let project = store.get(project_id)?;
        let destination = workspace_path(
            &root,
            &format!("submission/{}", project.active_source.file_name),
        )?;
        if !destination.exists() {
            copy_new(&store.source_path(&project), &destination)?;
        }
        for name in ["submission/figures", "submission/supplementary"] {
            fs::create_dir_all(workspace_path(&root, name)?).map_err(|_| failure())?;
        }
    }
    root.canonicalize().map_err(|_| failure())
}

pub fn workspace_path(root: &Path, relative: &str) -> Result<PathBuf, AppError> {
    for parent in root.ancestors().take(4) {
        if fs::symlink_metadata(parent).is_ok_and(|metadata| metadata.file_type().is_symlink()) {
            return Err(AppError::new("WORKSPACE_PATH_INVALID", true));
        }
    }
    if relative.contains('\\')
        || Path::new(relative)
            .components()
            .any(|c| !matches!(c, Component::Normal(_)))
    {
        return Err(AppError::new("WORKSPACE_PATH_INVALID", true));
    }
    let mut path = root.to_path_buf();
    for component in Path::new(relative).components() {
        path.push(component);
        if fs::symlink_metadata(&path).is_ok_and(|m| m.file_type().is_symlink()) {
            return Err(AppError::new("WORKSPACE_PATH_INVALID", true));
        }
    }
    Ok(path)
}

pub fn list_workspace(
    store: &ProjectStore,
    project_id: &str,
) -> Result<WorkspaceListing, AppError> {
    let root = workspace_root(store, project_id)?;
    let mut entries = Vec::new();
    walk(&root, &root, 0, &mut entries)?;
    entries.sort_by(|a, b| {
        b.directory
            .cmp(&a.directory)
            .then(a.relative_path.cmp(&b.relative_path))
    });
    Ok(WorkspaceListing {
        root_path: root.to_string_lossy().into(),
        entries,
    })
}

fn walk(
    root: &Path,
    folder: &Path,
    depth: usize,
    entries: &mut Vec<WorkspaceEntry>,
) -> Result<(), AppError> {
    if depth > 16 {
        return Err(AppError::new("LIMIT_EXCEEDED", true));
    }
    for entry in fs::read_dir(folder).map_err(|_| failure())? {
        let entry = entry.map_err(|_| failure())?;
        let metadata = fs::symlink_metadata(entry.path()).map_err(|_| failure())?;
        if metadata.file_type().is_symlink() || (!metadata.is_file() && !metadata.is_dir()) {
            continue;
        }
        if entries.len() >= 2000 {
            return Err(AppError::new("LIMIT_EXCEEDED", true));
        }
        entries.push(WorkspaceEntry {
            relative_path: entry
                .path()
                .strip_prefix(root)
                .map_err(|_| failure())?
                .to_string_lossy()
                .replace('\\', "/"),
            directory: metadata.is_dir(),
            size_bytes: if metadata.is_file() {
                metadata.len()
            } else {
                0
            },
            modified_at_unix_ms: metadata
                .modified()
                .ok()
                .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
                .map_or(0, |duration| duration.as_millis() as u64),
        });
        if metadata.is_dir() {
            walk(root, &entry.path(), depth + 1, entries)?;
        }
    }
    Ok(())
}

pub fn import_workspace_file(
    store: &ProjectStore,
    project_id: &str,
    source: &Path,
    directory: &str,
) -> Result<(), AppError> {
    let root = workspace_root(store, project_id)?;
    let folder = workspace_path(&root, directory)?;
    if !folder.is_dir() {
        return Err(failure());
    }
    let metadata = fs::symlink_metadata(source).map_err(|_| failure())?;
    if !metadata.is_file() || metadata.len() > crate::MAX_MANUSCRIPT_SIZE_BYTES {
        return Err(AppError::new("INPUT_UNREADABLE", true));
    }
    let name = source
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(failure)?;
    let relative = if directory.is_empty() {
        name.to_owned()
    } else {
        format!("{directory}/{name}")
    };
    let destination = workspace_path(&root, &relative)?;
    copy_new(source, &destination)
}

pub(crate) fn copy_new(source: &Path, destination: &Path) -> Result<(), AppError> {
    let mut input = fs::File::open(source).map_err(|_| failure())?;
    let mut output = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(destination)
        .map_err(|error| {
            if error.kind() == io::ErrorKind::AlreadyExists {
                AppError::new("WORKSPACE_FILE_EXISTS", true)
            } else {
                failure()
            }
        })?;
    if io::copy(&mut input, &mut output)
        .and_then(|_| output.sync_all())
        .is_err()
    {
        let _ = fs::remove_file(destination);
        return Err(failure());
    }
    Ok(())
}

pub fn move_workspace_file(
    store: &ProjectStore,
    project_id: &str,
    source: &str,
    directory: &str,
) -> Result<(), AppError> {
    let root = workspace_root(store, project_id)?;
    let from = workspace_path(&root, source)?;
    if !from.is_file() {
        return Err(AppError::new("WORKSPACE_PATH_INVALID", true));
    }
    let folder = workspace_path(&root, directory)?;
    if !folder.is_dir() {
        return Err(failure());
    }
    let destination = folder.join(from.file_name().ok_or_else(failure)?);
    if from == destination {
        return Ok(());
    }
    copy_new(&from, &destination)?;
    fs::remove_file(from).map_err(|_| failure())
}

pub fn copy_package_to_workspace(
    store: &ProjectStore,
    package: &CompiledPackage,
) -> Result<(), AppError> {
    let root = workspace_root(store, &package.project_id)?;
    // Every generation is a new editable copy; never replace a user's edits.
    let folder = workspace_path(&root, &format!("drafts/{}", package.id))?;
    fs::create_dir_all(&folder).map_err(|_| failure())?;
    for file in &package.files {
        let destination = workspace_path(&folder, &file.relative_path)?;
        fs::create_dir_all(destination.parent().ok_or_else(failure)?).map_err(|_| failure())?;
        copy_new(
            &store
                .staging_path(&package.project_id, &package.id)
                .join(&file.relative_path),
            &destination,
        )?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_escape_paths() {
        for path in [
            "../secret",
            "/tmp/secret",
            "a/../../secret",
            "a\\..\\secret",
        ] {
            assert!(workspace_path(Path::new("/tmp/root"), path).is_err());
        }
    }
    #[test]
    fn copy_preserves_existing_files() {
        let root = std::env::temp_dir().join(uuid::Uuid::new_v4().to_string());
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("a"), b"new").unwrap();
        fs::write(root.join("b"), b"author edit").unwrap();
        assert_eq!(
            copy_new(&root.join("a"), &root.join("b")).unwrap_err().code,
            "WORKSPACE_FILE_EXISTS"
        );
        assert_eq!(fs::read(root.join("b")).unwrap(), b"author edit");
        fs::remove_dir_all(root).unwrap();
    }
}
