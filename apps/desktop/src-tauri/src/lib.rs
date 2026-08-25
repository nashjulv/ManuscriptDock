mod model_service;

use manuscript_core::{
    bundled_rule_pack_catalog, bundled_submission_element_catalog, discipline_catalog,
    AcademicKnowledgeBodySnapshot, DisciplineCatalogItem, KnowledgeBodyRecord,
    KnowledgeDialogueLedger, KnowledgeInquiryStance, KnowledgeInquiryTarget, LocalAttestation,
    ManuscriptSelection, ReadinessEvaluation, RevisionApplication, RevisionChangeInput,
    RevisionDraft, RulePackCatalog, StructureAnalysis, SubmissionElementCatalog, SubmissionExport,
    SubmissionRecord, VersionComparison, VersionCreation, VersionHistory, WorkspaceCatalog,
    WorkspaceCreation, WorkspaceLifecycle, WorkspaceStore,
};
use model_service::{ModelSettingsSummary, ModelSlotInput};
use serde_json::json;
use std::{collections::HashMap, path::PathBuf, sync::Mutex};
use tauri::{AppHandle, Manager, State};
use tauri_plugin_dialog::DialogExt;
use uuid::Uuid;

#[derive(Default)]
struct PendingSelections(Mutex<HashMap<String, PathBuf>>);

#[tauri::command]
async fn select_manuscript(
    app: AppHandle,
    pending: State<'_, PendingSelections>,
) -> Result<ManuscriptSelection, String> {
    let selection = app
        .dialog()
        .file()
        .add_filter("论文稿件", &["docx", "pdf", "tex"])
        .blocking_pick_file();

    let Some(selection) = selection else {
        return Ok(ManuscriptSelection::Cancelled);
    };

    let path = match selection.into_path() {
        Ok(path) => path,
        Err(error) => {
            return Ok(ManuscriptSelection::Rejected {
                message: format!("无法读取所选文件路径：{error}"),
            });
        }
    };

    Ok(match manuscript_core::inspect_manuscript(&path) {
        Ok(manuscript) => {
            let selection_id = Uuid::new_v4().to_string();
            match pending.0.lock() {
                Ok(mut selections) => {
                    selections.clear();
                    selections.insert(selection_id.clone(), path);
                    ManuscriptSelection::Selected {
                        selection_id,
                        manuscript,
                    }
                }
                Err(_) => ManuscriptSelection::Rejected {
                    message: "本地选择状态不可用，请重启应用后再试".to_owned(),
                },
            }
        }
        Err(error) => ManuscriptSelection::Rejected {
            message: error.to_string(),
        },
    })
}

#[tauri::command]
async fn create_workspace(
    selection_id: String,
    app: AppHandle,
    pending: State<'_, PendingSelections>,
) -> Result<WorkspaceCreation, String> {
    let source_path = match pending.0.lock() {
        Ok(selections) => selections.get(&selection_id).cloned(),
        Err(_) => {
            return Ok(WorkspaceCreation::Rejected {
                message: "本地选择状态不可用，请重启应用后再试".to_owned(),
            });
        }
    };
    let Some(source_path) = source_path else {
        return Ok(WorkspaceCreation::Rejected {
            message: "该文件选择已失效，请重新选择论文".to_owned(),
        });
    };

    let root = match workspace_root(&app) {
        Ok(root) => root,
        Err(message) => return Ok(WorkspaceCreation::Rejected { message }),
    };
    Ok(
        match WorkspaceStore::new(root).create_from_source(&source_path) {
            Ok(workspace) => {
                if let Ok(mut selections) = pending.0.lock() {
                    selections.remove(&selection_id);
                }
                WorkspaceCreation::Created { workspace }
            }
            Err(error) => WorkspaceCreation::Rejected {
                message: error.to_string(),
            },
        },
    )
}

#[tauri::command]
async fn list_workspaces(app: AppHandle) -> Result<WorkspaceCatalog, String> {
    let root = workspace_root(&app)?;
    WorkspaceStore::new(root)
        .list()
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn archive_workspace(
    workspace_id: String,
    app: AppHandle,
) -> Result<WorkspaceCatalog, String> {
    let root = workspace_root(&app)?;
    WorkspaceStore::new(root)
        .archive_workspace(&workspace_id)
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn restore_workspace(
    workspace_id: String,
    app: AppHandle,
) -> Result<WorkspaceCatalog, String> {
    let root = workspace_root(&app)?;
    WorkspaceStore::new(root)
        .restore_workspace(&workspace_id)
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn delete_workspace(
    workspace_id: String,
    archived: bool,
    author_confirmed: bool,
    app: AppHandle,
) -> Result<WorkspaceCatalog, String> {
    let root = workspace_root(&app)?;
    WorkspaceStore::new(root)
        .delete_workspace(&workspace_id, archived, author_confirmed)
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn get_version_history(
    workspace_id: String,
    app: AppHandle,
) -> Result<VersionHistory, String> {
    let root = workspace_root(&app)?;
    WorkspaceStore::new(root)
        .version_history(&workspace_id)
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn get_knowledge_body_snapshot(
    workspace_id: String,
    app: AppHandle,
) -> Result<AcademicKnowledgeBodySnapshot, String> {
    let root = workspace_root(&app)?;
    WorkspaceStore::new(root)
        .knowledge_body_snapshot(&workspace_id)
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn get_workspace_lifecycle(
    workspace_id: String,
    app: AppHandle,
) -> Result<WorkspaceLifecycle, String> {
    let root = workspace_root(&app)?;
    WorkspaceStore::new(root)
        .lifecycle(&workspace_id)
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn create_local_attestation(
    workspace_id: String,
    author_confirmed: bool,
    app: AppHandle,
) -> Result<LocalAttestation, String> {
    let root = workspace_root(&app)?;
    WorkspaceStore::new(root)
        .create_local_attestation(&workspace_id, author_confirmed)
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn export_submission_package(
    workspace_id: String,
    app: AppHandle,
) -> Result<Option<SubmissionExport>, String> {
    let Some(folder) = app.dialog().file().blocking_pick_folder() else {
        return Ok(None);
    };
    let destination = folder
        .into_path()
        .map_err(|error| format!("无法读取导出文件夹：{error}"))?;
    let root = workspace_root(&app)?;
    WorkspaceStore::new(root)
        .export_submission_package(&workspace_id, &destination)
        .map(Some)
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn record_manual_submission(
    workspace_id: String,
    target: String,
    receipt: Option<String>,
    author_confirmed: bool,
    app: AppHandle,
) -> Result<SubmissionRecord, String> {
    let root = workspace_root(&app)?;
    WorkspaceStore::new(root)
        .record_manual_submission(&workspace_id, &target, receipt.as_deref(), author_confirmed)
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn finalize_knowledge_body(
    workspace_id: String,
    discipline_code: String,
    app: AppHandle,
) -> Result<KnowledgeBodyRecord, String> {
    let root = workspace_root(&app)?;
    WorkspaceStore::new(root)
        .finalize_knowledge_body(&workspace_id, &discipline_code)
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn list_discipline_index() -> Result<Vec<DisciplineCatalogItem>, String> {
    Ok(discipline_catalog())
}

#[tauri::command]
async fn get_model_settings(app: AppHandle) -> Result<ModelSettingsSummary, String> {
    model_service::load_summary(&model_settings_root(&app)?)
}

#[tauri::command]
async fn save_model_settings(
    slots: Vec<ModelSlotInput>,
    app: AppHandle,
) -> Result<ModelSettingsSummary, String> {
    model_service::save_settings(&model_settings_root(&app)?, slots)
}

#[tauri::command]
async fn get_knowledge_dialogue(
    workspace_id: String,
    app: AppHandle,
) -> Result<KnowledgeDialogueLedger, String> {
    let root = workspace_root(&app)?;
    WorkspaceStore::new(root)
        .knowledge_dialogue(&workspace_id)
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn ask_knowledge_body(
    workspace_id: String,
    stance: KnowledgeInquiryStance,
    target: KnowledgeInquiryTarget,
    question: String,
    author_confirmed_external_transmission: bool,
    app: AppHandle,
) -> Result<KnowledgeDialogueLedger, String> {
    if !author_confirmed_external_transmission {
        return Err("需要作者确认本次模型外发后才能提问".to_owned());
    }
    let root = workspace_root(&app)?;
    let store = WorkspaceStore::new(&root);
    let lifecycle = store
        .lifecycle(&workspace_id)
        .map_err(|error| error.to_string())?;
    let knowledge = lifecycle
        .knowledge_body
        .as_ref()
        .ok_or_else(|| "当前论文版本尚未固化知识体".to_owned())?;
    let inquiry = store
        .create_owner_inquiry(&workspace_id, stance, target, &question, true)
        .map_err(|error| error.to_string())?;
    let structure = lifecycle.structure_report.as_ref();
    let projection = json!({
        "knowledgeBodyRecordId": knowledge.record_id,
        "knowledgeBodyHash": knowledge.record_hash,
        "snapshotVersion": knowledge.snapshot.snapshot_version,
        "discipline": knowledge.discipline_classification,
        "manuscriptVersion": knowledge.manuscript_version,
        "title": structure.and_then(|report| report.title.as_deref()),
        "authors": structure.map(|report| report.authors.as_slice()).unwrap_or(&[]),
        "abstract": structure.and_then(|report| report.abstract_text.as_deref()),
        "sections": structure.map(|report| &report.sections),
        "claim": knowledge.snapshot.claim,
        "objects": knowledge.snapshot.objects,
        "aiReviewReport": knowledge.snapshot.ai_review_report,
        "externalTransmissionNotice": "This projection is sent only for this author-confirmed question."
    });
    let system_prompt = "You are the author's configured academic assistant. Answer only from the supplied KnowledgeBody projection. Distinguish established objects from pending v0 objects. Do not invent evidence, methods, results, reviews, citations, or scientific truth. If the projection is insufficient, state exactly what is missing. Reply in the language of the question and keep object names such as Claim, Scope, Method, Result, EvidenceRelation, SourceAnchor, and AIReviewReport explicit.";
    let user_prompt = format!(
        "Target: {}\nStance: {}\nQuestion: {}\n\nKnowledgeBody projection:\n{}",
        serde_json::to_string(&target).unwrap_or_else(|_| "knowledge_body".to_owned()),
        serde_json::to_string(&stance).unwrap_or_else(|_| "question".to_owned()),
        inquiry.question,
        serde_json::to_string_pretty(&projection)
            .map_err(|error| format!("无法生成最小知识体投影：{error}"))?
    );
    let model_answer =
        model_service::ask_with_failover(&model_settings_root(&app)?, system_prompt, &user_prompt)
            .await?;
    store
        .record_model_answer(
            &workspace_id,
            &inquiry.inquiry_id,
            model_answer.slot.as_str(),
            &model_answer.provider_label,
            &model_answer.model,
            &model_answer.content,
            std::slice::from_ref(&knowledge.snapshot.objects.source_anchor),
        )
        .map_err(|error| error.to_string())?;
    store
        .knowledge_dialogue(&workspace_id)
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn save_manuscript_version(
    workspace_id: String,
    selection_id: String,
    note: String,
    app: AppHandle,
    pending: State<'_, PendingSelections>,
) -> Result<VersionCreation, String> {
    let source_path = pending
        .0
        .lock()
        .map_err(|_| "本地选择状态不可用，请重启应用后再试".to_owned())?
        .get(&selection_id)
        .cloned()
        .ok_or_else(|| "该文件选择已失效，请重新选择修改稿".to_owned())?;
    let root = workspace_root(&app)?;
    let result = WorkspaceStore::new(root)
        .create_version_from_source(&workspace_id, &source_path, &note)
        .map_err(|error| error.to_string())?;
    if let Ok(mut selections) = pending.0.lock() {
        selections.remove(&selection_id);
    }
    Ok(result)
}

#[tauri::command]
async fn restore_manuscript_version(
    workspace_id: String,
    version: u32,
    app: AppHandle,
) -> Result<VersionCreation, String> {
    let root = workspace_root(&app)?;
    WorkspaceStore::new(root)
        .restore_version(&workspace_id, version)
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn compare_manuscript_versions(
    workspace_id: String,
    from_version: u32,
    to_version: u32,
    app: AppHandle,
) -> Result<VersionComparison, String> {
    let root = workspace_root(&app)?;
    WorkspaceStore::new(root)
        .compare_versions(&workspace_id, from_version, to_version)
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn analyze_workspace(
    workspace_id: String,
    app: AppHandle,
) -> Result<StructureAnalysis, String> {
    let root = workspace_root(&app)?;
    Ok(
        match WorkspaceStore::new(root).analyze_structure(&workspace_id) {
            Ok(report) => StructureAnalysis::Completed {
                report: Box::new(report),
            },
            Err(error) => StructureAnalysis::Rejected {
                message: error.to_string(),
            },
        },
    )
}

#[tauri::command]
async fn evaluate_readiness(
    workspace_id: String,
    rule_pack_ids: Vec<String>,
    app: AppHandle,
) -> Result<ReadinessEvaluation, String> {
    let root = workspace_root(&app)?;
    Ok(
        match WorkspaceStore::new(root).evaluate_readiness(&workspace_id, &rule_pack_ids) {
            Ok(report) => ReadinessEvaluation::Completed { report },
            Err(error) => ReadinessEvaluation::Rejected {
                message: error.to_string(),
            },
        },
    )
}

#[tauri::command]
async fn list_rule_packs() -> Result<RulePackCatalog, String> {
    bundled_rule_pack_catalog().map_err(|error| error.to_string())
}

#[tauri::command]
async fn list_submission_elements(
    rule_pack_ids: Vec<String>,
) -> Result<SubmissionElementCatalog, String> {
    bundled_submission_element_catalog(&rule_pack_ids).map_err(|error| error.to_string())
}

#[tauri::command]
async fn get_revision_draft(workspace_id: String, app: AppHandle) -> Result<RevisionDraft, String> {
    let root = workspace_root(&app)?;
    WorkspaceStore::new(root)
        .revision_draft(&workspace_id)
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn apply_manuscript_revision(
    workspace_id: String,
    base_version: u32,
    changes: Vec<RevisionChangeInput>,
    app: AppHandle,
) -> Result<RevisionApplication, String> {
    let root = workspace_root(&app)?;
    WorkspaceStore::new(root)
        .apply_revision(&workspace_id, base_version, &changes)
        .map_err(|error| error.to_string())
}

fn workspace_root(app: &AppHandle) -> Result<PathBuf, String> {
    app.path()
        .app_data_dir()
        .map(|path| path.join("workspace"))
        .map_err(|error| format!("无法定位本地应用数据目录：{error}"))
}

fn model_settings_root(app: &AppHandle) -> Result<PathBuf, String> {
    app.path()
        .app_config_dir()
        .map(|path| path.join("model-service"))
        .map_err(|error| format!("无法定位模型设置目录：{error}"))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(PendingSelections::default())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            select_manuscript,
            create_workspace,
            list_workspaces,
            archive_workspace,
            restore_workspace,
            delete_workspace,
            get_version_history,
            get_knowledge_body_snapshot,
            get_workspace_lifecycle,
            create_local_attestation,
            export_submission_package,
            record_manual_submission,
            finalize_knowledge_body,
            list_discipline_index,
            get_model_settings,
            save_model_settings,
            get_knowledge_dialogue,
            ask_knowledge_body,
            save_manuscript_version,
            restore_manuscript_version,
            compare_manuscript_versions,
            list_rule_packs,
            list_submission_elements,
            get_revision_draft,
            apply_manuscript_revision,
            analyze_workspace,
            evaluate_readiness
        ])
        .run(tauri::generate_context!())
        .expect("failed to run ManuscriptDock");
}
