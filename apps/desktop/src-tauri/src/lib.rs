mod ui_preferences;
mod window_geometry;

use manuscript_core::{
    AppError, AuthorConstraints, CompiledPackage, DocumentFacts, ExportDestination, JobRecord,
    JournalRecord, PreparationView, Project, ProjectStore, TargetOrigin, TaskKind,
};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{Arc, Mutex},
};
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_dialog::DialogExt;

#[derive(Debug, Clone)]
struct SelectionEntry {
    path: PathBuf,
    kind: String,
}
#[derive(Clone, Default)]
struct RuntimeState {
    selections: Arc<Mutex<HashMap<String, SelectionEntry>>>,
    destinations: Arc<Mutex<HashMap<String, PathBuf>>>,
    packages: Arc<Mutex<HashMap<String, CompiledPackage>>>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SelectedInput {
    token: String,
    name: String,
    extension: String,
    size_bytes: u64,
    kind: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SelectedFolder {
    token: String,
    name: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SelectTargetInput {
    project_id: String,
    expected_revision: u64,
    journal_id: String,
    article_type: String,
    origin: TargetOrigin,
    recommendation_ref: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
enum SelectionResponse {
    Selected {
        items: Vec<SelectedInput>,
        #[serde(skip_serializing_if = "Option::is_none")]
        folder: Option<SelectedFolder>,
    },
    Cancelled,
}

fn store(app: &AppHandle) -> Result<ProjectStore, AppError> {
    let root = app
        .path()
        .app_data_dir()
        .map_err(|_| AppError::new("STORAGE_UNAVAILABLE", true))?
        .join("workspace-next");
    ProjectStore::new(root)
}
fn selected_input(
    path: PathBuf,
    kind: &str,
    state: &RuntimeState,
) -> Result<SelectedInput, AppError> {
    let link_metadata = path
        .symlink_metadata()
        .map_err(|_| AppError::new("INPUT_UNREADABLE", true))?;
    if link_metadata.file_type().is_symlink() {
        return Err(
            AppError::new("INPUT_SYMLINK_NOT_ALLOWED", true).recover("choose_original_file")
        );
    }
    let metadata = path
        .metadata()
        .map_err(|_| AppError::new("INPUT_UNREADABLE", true))?;
    if !metadata.is_file() {
        return Err(AppError::new("INPUT_UNREADABLE", true));
    }
    let name = path
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| AppError::new("INPUT_UNREADABLE", true))?
        .to_owned();
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    let token = uuid::Uuid::new_v4().to_string();
    state
        .selections
        .lock()
        .map_err(|_| AppError::new("SELECTION_STATE_UNAVAILABLE", true))?
        .insert(
            token.clone(),
            SelectionEntry {
                path,
                kind: kind.to_owned(),
            },
        );
    Ok(SelectedInput {
        token,
        name,
        extension,
        size_bytes: metadata.len(),
        kind: kind.to_owned(),
    })
}

fn selected_folder(path: PathBuf, state: &RuntimeState) -> Result<SelectedFolder, AppError> {
    let link_metadata = path
        .symlink_metadata()
        .map_err(|_| AppError::new("PROJECT_FOLDER_UNAVAILABLE", true))?;
    if link_metadata.file_type().is_symlink() || !link_metadata.is_dir() {
        return Err(AppError::new("PROJECT_FOLDER_UNAVAILABLE", true));
    }
    let path = path
        .canonicalize()
        .map_err(|_| AppError::new("PROJECT_FOLDER_UNAVAILABLE", true))?;
    let name = path
        .file_name()
        .and_then(|value| value.to_str())
        .filter(|value| !value.is_empty())
        .unwrap_or("ManuscriptDock")
        .to_owned();
    let token = uuid::Uuid::new_v4().to_string();
    state
        .selections
        .lock()
        .map_err(|_| AppError::new("SELECTION_STATE_UNAVAILABLE", true))?
        .insert(
            token.clone(),
            SelectionEntry {
                path,
                kind: "folder".into(),
            },
        );
    Ok(SelectedFolder { token, name })
}
fn take_selection(
    state: &RuntimeState,
    token: &str,
    expected_kind: Option<&str>,
) -> Result<SelectionEntry, AppError> {
    let entry = state
        .selections
        .lock()
        .map_err(|_| AppError::new("SELECTION_STATE_UNAVAILABLE", true))?
        .remove(token)
        .ok_or_else(|| AppError::new("SELECTION_EXPIRED", true).recover("choose_another_file"))?;
    if expected_kind.is_some_and(|kind| entry.kind != kind) {
        return Err(AppError::new("SELECTION_KIND_INVALID", true));
    }
    Ok(entry)
}

const JOB_EVENT_NAME: &str = "manuscriptdock://job";

fn dispatch_job<T, F>(
    app: AppHandle,
    request_id: String,
    operation: &'static str,
    project_id: Option<String>,
    context_hash: Option<String>,
    action: F,
) -> Result<JobRecord, AppError>
where
    T: Clone + DeserializeOwned + Serialize + Send + 'static,
    F: FnOnce(ProjectStore) -> Result<T, AppError> + Send + 'static,
{
    let project_store = store(&app)?;
    let (job, should_start) = project_store.reserve_job(
        &request_id,
        operation,
        project_id.as_deref(),
        context_hash.as_deref(),
    )?;
    let _ = app.emit(JOB_EVENT_NAME, &job);
    if should_start {
        let event_app = app.clone();
        tauri::async_runtime::spawn_blocking(move || {
            let action_store = project_store.clone();
            let _ = project_store.execute_reserved_job(
                &request_id,
                operation,
                || action(action_store),
                |event| {
                    let _ = event_app.emit(JOB_EVENT_NAME, event);
                },
            );
        });
    }
    Ok(job)
}

#[tauri::command]
async fn choose_local_inputs(
    app: AppHandle,
    state: State<'_, RuntimeState>,
    kind: String,
) -> Result<SelectionResponse, AppError> {
    if kind == "folder" {
        let dialog_app = app.clone();
        let folder = tauri::async_runtime::spawn_blocking(move || {
            dialog_app.dialog().file().blocking_pick_folder()
        })
        .await
        .map_err(|_| AppError::new("DIALOG_UNAVAILABLE", true))?;
        let Some(folder) = folder else {
            return Ok(SelectionResponse::Cancelled);
        };
        let folder = folder
            .into_path()
            .map_err(|_| AppError::new("INPUT_UNREADABLE", true))?;
        let folder = selected_folder(folder, &state)?;
        let folder_path = state
            .selections
            .lock()
            .map_err(|_| AppError::new("SELECTION_STATE_UNAVAILABLE", true))?
            .get(&folder.token)
            .map(|entry| entry.path.clone())
            .ok_or_else(|| AppError::new("SELECTION_STATE_UNAVAILABLE", true))?;
        let mut paths = std::fs::read_dir(folder_path)
            .map_err(|_| AppError::new("INPUT_UNREADABLE", true))?
            .filter_map(Result::ok)
            .filter(|entry| entry.file_type().is_ok_and(|file_type| file_type.is_file()))
            .map(|entry| entry.path())
            .filter(|path| {
                !path
                    .file_name()
                    .and_then(|v| v.to_str())
                    .is_some_and(|v| v.starts_with('.') || v.starts_with("~$"))
            })
            .collect::<Vec<_>>();
        paths.sort();
        if paths.len() > manuscript_core::MAX_MATERIAL_FILES {
            return Err(AppError::new("LIMIT_EXCEEDED", true)
                .param("fileLimit", manuscript_core::MAX_MATERIAL_FILES.to_string()));
        }
        let mut items = Vec::new();
        for path in paths {
            let item_kind =
                if path.extension().and_then(|v| v.to_str()).is_some_and(|v| {
                    v.eq_ignore_ascii_case("docx") || v.eq_ignore_ascii_case("pdf")
                }) {
                    "manuscript"
                } else {
                    "material"
                };
            items.push(selected_input(path, item_kind, &state)?);
        }
        return Ok(SelectionResponse::Selected {
            items,
            folder: Some(folder),
        });
    }
    let dialog_app = app.clone();
    let manuscript = kind == "manuscript";
    let editable_manuscript = kind == "editable_manuscript";
    let selection = tauri::async_runtime::spawn_blocking(move || {
        if manuscript {
            dialog_app
                .dialog()
                .file()
                .add_filter("PDF or DOCX", &["pdf", "docx"])
                .blocking_pick_file()
        } else if editable_manuscript {
            dialog_app
                .dialog()
                .file()
                .add_filter("DOCX", &["docx"])
                .blocking_pick_file()
        } else {
            dialog_app.dialog().file().blocking_pick_file()
        }
    })
    .await
    .map_err(|_| AppError::new("DIALOG_UNAVAILABLE", true))?;
    let Some(selection) = selection else {
        return Ok(SelectionResponse::Cancelled);
    };
    let path = selection
        .into_path()
        .map_err(|_| AppError::new("INPUT_UNREADABLE", true))?;
    Ok(SelectionResponse::Selected {
        items: vec![selected_input(path, &kind, &state)?],
        folder: None,
    })
}

#[tauri::command]
fn open_project(
    app: AppHandle,
    state: State<'_, RuntimeState>,
    request_id: String,
    manuscript_token: String,
    folder_token: Option<String>,
    task: TaskKind,
) -> Result<JobRecord, AppError> {
    let runtime = state.inner().clone();
    dispatch_job(
        app,
        request_id,
        "open_project",
        None,
        None,
        move |project_store| {
            let selection = take_selection(&runtime, &manuscript_token, Some("manuscript"))?;
            if let Some(folder_token) = folder_token {
                let folder = take_selection(&runtime, &folder_token, Some("folder"))?;
                project_store.open_or_create_folder_project(&folder.path, &selection.path, task)
            } else {
                project_store.create_from_manuscript(&selection.path, task)
            }
        },
    )
}

#[tauri::command]
fn get_project_view(app: AppHandle, project_id: String) -> Result<Project, AppError> {
    store(&app)?.get(&project_id)
}

#[tauri::command]
fn list_recent_projects(app: AppHandle) -> Result<Vec<Project>, AppError> {
    store(&app)?.recent()
}

#[tauri::command]
fn list_removed_projects(app: AppHandle) -> Result<Vec<Project>, AppError> {
    store(&app)?.removed_recent()
}

#[tauri::command]
fn hide_recent_project(
    app: AppHandle,
    request_id: String,
    project_id: String,
) -> Result<JobRecord, AppError> {
    let job_project_id = project_id.clone();
    dispatch_job(
        app,
        request_id,
        "hide_recent_project",
        Some(job_project_id),
        None,
        move |project_store| project_store.hide_recent_project(&project_id),
    )
}

#[tauri::command]
fn restore_recent_project(
    app: AppHandle,
    request_id: String,
    project_id: String,
) -> Result<JobRecord, AppError> {
    let job_project_id = project_id.clone();
    dispatch_job(
        app,
        request_id,
        "restore_recent_project",
        Some(job_project_id),
        None,
        move |project_store| project_store.restore_recent_project(&project_id),
    )
}

#[tauri::command]
fn list_journals(query: String) -> Result<Vec<JournalRecord>, AppError> {
    manuscript_core::find_journals(&query)
}

#[tauri::command]
fn recommend_journals(
    app: AppHandle,
    request_id: String,
    project_id: String,
    constraints: AuthorConstraints,
) -> Result<JobRecord, AppError> {
    dispatch_job(
        app,
        request_id,
        "recommend_journals",
        Some(project_id.clone()),
        None,
        move |project_store| {
            let project = project_store.get(&project_id)?;
            manuscript_core::recommend(&constraints, &project.facts)
        },
    )
}

#[tauri::command]
fn select_target(
    app: AppHandle,
    request_id: String,
    input: SelectTargetInput,
) -> Result<JobRecord, AppError> {
    dispatch_job(
        app,
        request_id,
        "select_target",
        Some(input.project_id.clone()),
        None,
        move |project_store| {
            let rules_hash = manuscript_core::rules_hash_for(&input.journal_id)?;
            project_store.select_target(
                &input.project_id,
                input.expected_revision,
                manuscript_core::TargetSelection {
                    id: uuid::Uuid::new_v4().to_string(),
                    journal_id: input.journal_id,
                    article_type: input.article_type,
                    stage: "initial_submission".into(),
                    origin: input.origin,
                    recommendation_ref: input.recommendation_ref,
                    rules_hash,
                },
            )
        },
    )
}

#[tauri::command]
fn get_preparation(app: AppHandle, project_id: String) -> Result<PreparationView, AppError> {
    let project = store(&app)?.get(&project_id)?;
    manuscript_core::prepare(&project)
}

#[tauri::command]
fn update_document_facts(
    app: AppHandle,
    request_id: String,
    project_id: String,
    expected_revision: u64,
    facts: DocumentFacts,
) -> Result<JobRecord, AppError> {
    dispatch_job(
        app,
        request_id,
        "update_document_facts",
        Some(project_id.clone()),
        None,
        move |project_store| project_store.update_facts(&project_id, expected_revision, facts),
    )
}

#[tauri::command]
fn add_material(
    app: AppHandle,
    state: State<'_, RuntimeState>,
    request_id: String,
    project_id: String,
    expected_revision: u64,
    token: String,
    kind: String,
) -> Result<JobRecord, AppError> {
    let runtime = state.inner().clone();
    dispatch_job(
        app,
        request_id,
        "add_material",
        Some(project_id.clone()),
        None,
        move |project_store| {
            let selection = take_selection(&runtime, &token, None)?;
            project_store.add_material(&project_id, expected_revision, &selection.path, &kind)
        },
    )
}

#[tauri::command]
fn build_package(
    app: AppHandle,
    state: State<'_, RuntimeState>,
    request_id: String,
    project_id: String,
    context_hash: String,
    mode: String,
) -> Result<JobRecord, AppError> {
    let runtime = state.inner().clone();
    dispatch_job(
        app,
        request_id,
        "build_package",
        Some(project_id.clone()),
        Some(context_hash.clone()),
        move |project_store| {
            let project = project_store.get(&project_id)?;
            let package =
                manuscript_core::build_package(&project_store, &project, &context_hash, &mode)?;
            project_store.save_compiled_package(&package)?;
            runtime
                .packages
                .lock()
                .map_err(|_| AppError::new("PACKAGE_STATE_UNAVAILABLE", true))?
                .insert(package.id.clone(), package.clone());
            Ok(package)
        },
    )
}

#[tauri::command]
async fn choose_export_folder(
    app: AppHandle,
    state: State<'_, RuntimeState>,
) -> Result<SelectionResponse, AppError> {
    let dialog_app = app.clone();
    let folder = tauri::async_runtime::spawn_blocking(move || {
        dialog_app.dialog().file().blocking_pick_folder()
    })
    .await
    .map_err(|_| AppError::new("DIALOG_UNAVAILABLE", true))?;
    let Some(folder) = folder else {
        return Ok(SelectionResponse::Cancelled);
    };
    let path = folder
        .into_path()
        .map_err(|_| AppError::new("OUTPUT_PERMISSION_DENIED", true))?;
    let token = uuid::Uuid::new_v4().to_string();
    state
        .destinations
        .lock()
        .map_err(|_| AppError::new("SELECTION_STATE_UNAVAILABLE", true))?
        .insert(token.clone(), path);
    Ok(SelectionResponse::Selected {
        items: vec![SelectedInput {
            token,
            name: "export".into(),
            extension: String::new(),
            size_bytes: 0,
            kind: "destination".into(),
        }],
        folder: None,
    })
}

#[tauri::command]
fn choose_project_folder_for_export(
    app: AppHandle,
    state: State<'_, RuntimeState>,
    project_id: String,
) -> Result<SelectionResponse, AppError> {
    let project_store = store(&app)?;
    let path = project_store.folder_path_for_project(&project_id)?;
    let name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("ManuscriptDock")
        .to_owned();
    let token = uuid::Uuid::new_v4().to_string();
    state
        .destinations
        .lock()
        .map_err(|_| AppError::new("SELECTION_STATE_UNAVAILABLE", true))?
        .insert(token.clone(), path);
    Ok(SelectionResponse::Selected {
        items: vec![SelectedInput {
            token,
            name,
            extension: String::new(),
            size_bytes: 0,
            kind: "destination".into(),
        }],
        folder: None,
    })
}

#[tauri::command]
fn export_package(
    app: AppHandle,
    state: State<'_, RuntimeState>,
    request_id: String,
    package_id: String,
    context_hash: String,
    destination_token: String,
) -> Result<JobRecord, AppError> {
    let project_store = store(&app)?;
    let runtime = state.inner().clone();
    let cached = runtime
        .packages
        .lock()
        .map_err(|_| AppError::new("PACKAGE_STATE_UNAVAILABLE", true))?
        .get(&package_id)
        .cloned();
    let package = match cached {
        Some(package) => package,
        None => project_store.get_compiled_package(&package_id)?,
    };
    if package.context_hash != context_hash {
        return Err(AppError::new("CONTEXT_CHANGED", true));
    }
    dispatch_job(
        app,
        request_id.clone(),
        "export_package",
        Some(package.project_id.clone()),
        Some(context_hash),
        move |project_store| {
            if let Some(receipt) = project_store.export_receipt_for_request(&request_id)? {
                return Ok(receipt);
            }
            let destination = runtime
                .destinations
                .lock()
                .map_err(|_| AppError::new("SELECTION_STATE_UNAVAILABLE", true))?
                .remove(&destination_token)
                .ok_or_else(|| AppError::new("SELECTION_EXPIRED", true))?;
            let mut receipt =
                manuscript_core::export_package(&package, ExportDestination(destination))?;
            receipt.request_id = Some(request_id.clone());
            let _ = project_store.record_export_receipt(&mut receipt);
            Ok(receipt)
        },
    )
}

#[tauri::command]
fn get_job(
    app: AppHandle,
    project_id: Option<String>,
    job_id: String,
) -> Result<JobRecord, AppError> {
    let project_store = store(&app)?;
    match project_id {
        Some(project_id) => project_store.get_job(&project_id, &job_id),
        None => project_store.get_job_unscoped(&job_id),
    }
}

#[tauri::command]
fn cancel_job(
    app: AppHandle,
    project_id: Option<String>,
    job_id: String,
) -> Result<JobRecord, AppError> {
    let project_store = store(&app)?;
    let job = match project_id {
        Some(project_id) => project_store.cancel_job(&project_id, &job_id),
        None => project_store.cancel_job_unscoped(&job_id),
    }?;
    let _ = app.emit(JOB_EVENT_NAME, &job);
    Ok(job)
}

#[tauri::command]
fn get_ui_preferences(app: AppHandle) -> Result<ui_preferences::UiPreferences, &'static str> {
    let root = app
        .path()
        .app_config_dir()
        .map_err(|_| "UI_PREFERENCES_READ_FAILED")?;
    ui_preferences::load(&root)
}

#[tauri::command]
fn save_ui_preferences(
    app: AppHandle,
    text_size: ui_preferences::TextSize,
) -> Result<(), &'static str> {
    let root = app
        .path()
        .app_config_dir()
        .map_err(|_| "UI_PREFERENCES_WRITE_FAILED")?;
    ui_preferences::save(&root, text_size)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(RuntimeState::default())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            store(app.handle())?.recover_interrupted_jobs()?;
            window_geometry::install(app);
            Ok(())
        })
        .on_window_event(window_geometry::event)
        .invoke_handler(tauri::generate_handler![
            choose_local_inputs,
            open_project,
            get_project_view,
            list_recent_projects,
            list_removed_projects,
            hide_recent_project,
            restore_recent_project,
            list_journals,
            recommend_journals,
            select_target,
            get_preparation,
            update_document_facts,
            add_material,
            build_package,
            choose_export_folder,
            choose_project_folder_for_export,
            export_package,
            get_job,
            cancel_job,
            get_ui_preferences,
            save_ui_preferences
        ])
        .build(tauri::generate_context!())
        .expect("failed to build ManuscriptDock")
        .run(|app, event| match event {
            tauri::RunEvent::ExitRequested { .. } => window_geometry::flush(app),
            tauri::RunEvent::Exit => window_geometry::stop(app),
            _ => {}
        });
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn selection_tokens_are_not_paths() {
        let state = RuntimeState::default();
        let path = std::path::Path::new("/tmp/private manuscript.docx").to_path_buf();
        state.selections.lock().unwrap().insert(
            "token".into(),
            SelectionEntry {
                path: path.clone(),
                kind: "manuscript".into(),
            },
        );
        let entry = take_selection(&state, "token", Some("manuscript")).unwrap();
        assert_eq!(entry.path, path);
        assert!(state.selections.lock().unwrap().is_empty());
    }

    #[cfg(unix)]
    #[test]
    fn selected_input_rejects_symbolic_links() {
        use std::os::unix::fs::symlink;
        let root =
            std::env::temp_dir().join(format!("manuscriptdock-selection-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&root).unwrap();
        let source = root.join("source.docx");
        std::fs::write(&source, b"fixture").unwrap();
        let link = root.join("linked.docx");
        symlink(&source, &link).unwrap();
        let error = selected_input(link, "manuscript", &RuntimeState::default()).unwrap_err();
        assert_eq!(error.code, "INPUT_SYMLINK_NOT_ALLOWED");
        let _ = std::fs::remove_dir_all(root);
    }
}
