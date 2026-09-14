use crate::{
    prepare, AppError, CompiledPackage, ExportReceipt, GeneratedFile, LocalizedText,
    PreparationStatus, Project, ProjectStore,
};
use rust_xlsxwriter::{Format, Workbook};
use sha2::{Digest, Sha256};
use std::{
    collections::HashSet,
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};
use zip::{write::SimpleFileOptions, CompressionMethod, ZipWriter};

#[derive(Debug, Clone)]
pub struct ExportDestination(pub PathBuf);

struct CleanupDirectory {
    path: PathBuf,
    armed: bool,
}

impl CleanupDirectory {
    fn new(path: PathBuf) -> Self {
        Self { path, armed: true }
    }

    fn preserve(&mut self) {
        self.armed = false;
    }
}

impl Drop for CleanupDirectory {
    fn drop(&mut self) {
        if self.armed {
            let _ = fs::remove_dir_all(&self.path);
        }
    }
}

pub fn build_package(
    store: &ProjectStore,
    project: &Project,
    context_hash: &str,
    mode: &str,
) -> Result<CompiledPackage, AppError> {
    let target = project
        .target
        .as_ref()
        .ok_or_else(|| AppError::new("TARGET_REQUIRED", true).recover("choose_journal"))?;
    let current_rules_hash = crate::rules_hash_for(&target.journal_id)?;
    if current_rules_hash != target.rules_hash {
        return Err(AppError::new("RULES_CHANGED", true).recover("reselect_target"));
    }
    let preparation = prepare(project)?;
    if preparation.context_hash != context_hash {
        return Err(AppError::new("CONTEXT_CHANGED", true).recover("reload_preparation"));
    }
    if mode == "final" && preparation.status != PreparationStatus::ReadyToExport {
        return Err(AppError::new("MATERIAL_REQUIRED", true).recover("complete_missing_items"));
    }
    if mode != "final" && mode != "draft" {
        return Err(AppError::new("PACKAGE_MODE_INVALID", false));
    }
    let anonymized_manuscript = project
        .materials
        .iter()
        .find(|material| material.included && material.kind == "anonymized_manuscript");
    let editable_manuscript =
        if anonymized_manuscript.is_none() && project.active_source.format == "pdf" {
            project
                .materials
                .iter()
                .find(|material| material.included && material.kind == "editable_manuscript")
        } else {
            None
        };
    let selected_manuscript = anonymized_manuscript.or(editable_manuscript);
    let (manuscript_source, manuscript_hash, manuscript_size, manuscript_extension) =
        selected_manuscript.map_or_else(
            || {
                (
                    store.source_path(project),
                    project.active_source.sha256.as_str(),
                    project.active_source.size_bytes,
                    project.active_source.format.as_str(),
                )
            },
            |material| {
                (
                    store.material_path(&project.id, material),
                    material.sha256.as_str(),
                    material.size_bytes,
                    "docx",
                )
            },
        );
    if mode == "final" && manuscript_extension != "docx" {
        return Err(AppError::new("EDITABLE_MANUSCRIPT_REQUIRED", true)
            .recover("choose_editable_manuscript"));
    }
    verify_snapshot(&manuscript_source, manuscript_hash, manuscript_size)?;
    if anonymized_manuscript.is_some() {
        let check = crate::documents::inspect_anonymity(&manuscript_source, &project.facts)?;
        if !check.detected_categories.is_empty() {
            return Err(AppError::new("ANONYMITY_UNVERIFIED", true)
                .param("categories", check.detected_categories.join(","))
                .recover("replace_anonymized_manuscript"));
        }
    }
    for material in project
        .materials
        .iter()
        .filter(|material| material.included && !is_manuscript_material(&material.kind))
    {
        verify_snapshot(
            &store.material_path(&project.id, material),
            &material.sha256,
            material.size_bytes,
        )?;
    }
    let package_id = uuid::Uuid::new_v4().to_string();
    let staging = store.staging_path(&project.id, &package_id);
    let mut staging_cleanup = CleanupDirectory::new(staging.clone());
    fs::create_dir_all(staging.join("submission"))
        .and_then(|_| fs::create_dir_all(staging.join("author-tools")))
        .and_then(|_| fs::create_dir_all(staging.join("records")))
        .map_err(|_| AppError::new("STORAGE_UNAVAILABLE", true))?;
    let manuscript_name = manuscript_name(mode, manuscript_extension);
    copy_verified(
        &manuscript_source,
        staging.join("submission").join(&manuscript_name),
        manuscript_hash,
        manuscript_size,
    )?;
    let material_names = planned_material_names(project, &manuscript_name);
    for (material, output_name) in project
        .materials
        .iter()
        .filter(|material| material.included && !is_manuscript_material(&material.kind))
        .zip(material_names)
    {
        copy_verified(
            &store.material_path(&project.id, material),
            staging.join("submission").join(output_name),
            &material.sha256,
            material.size_bytes,
        )?;
    }
    let title = project.facts.title.as_deref().unwrap_or("[TITLE REQUIRED]");
    let journal = crate::catalog()?
        .journals
        .into_iter()
        .find(|candidate| candidate.id == target.journal_id)
        .map(|candidate| candidate.display_name)
        .unwrap_or_else(|| target.journal_id.clone());
    write_docx(
        &staging.join("author-tools/title-page-DRAFT.docx"),
        "Title page draft",
        &title_page_paragraphs(project),
    )?;
    write_docx(
        &staging.join("author-tools/cover-letter-DRAFT.docx"),
        "Cover letter draft",
        &[
            "Dear Editor,".into(),
            format!("Please consider our manuscript, \"{title}\", for {journal}."),
            "This local draft contains no inferred novelty, ethics, funding, or exclusivity claims. The corresponding author must review and complete it before use.".into(),
            "Sincerely,".into(),
            project
                .facts
                .authors
                .first()
                .cloned()
                .unwrap_or_else(|| "[AUTHOR REQUIRED]".into()),
        ],
    )?;
    write_docx(
        &staging.join("author-tools/highlights-DRAFT.docx"),
        "Highlights draft",
        &if project.facts.highlights.is_empty() {
            vec!["[AUTHOR-PROVIDED HIGHLIGHTS REQUIRED]".into()]
        } else {
            project.facts.highlights.clone()
        },
    )?;
    write_docx(
        &staging.join("author-tools/declarations-DRAFT.docx"),
        "Declarations draft",
        &declaration_paragraphs(project),
    )?;
    write_docx(
        &staging.join("submission").join(if mode == "draft" {
            "declaration-of-competing-interests-DRAFT.docx"
        } else {
            "declaration-of-competing-interests.docx"
        }),
        "Declaration of competing interests",
        &[project
            .facts
            .conflict_of_interest
            .clone()
            .unwrap_or_else(|| "[AUTHOR CONFIRMATION REQUIRED]".into())],
    )?;
    if publishable_highlights(project) {
        write_docx(
            &staging.join("submission").join(if mode == "draft" {
                "highlights-DRAFT.docx"
            } else {
                "highlights.docx"
            }),
            "Highlights",
            &project.facts.highlights,
        )?;
    }
    if anonymized_manuscript.is_some() {
        write_docx(
            &staging.join("submission").join(if mode == "draft" {
                "title-page-DRAFT.docx"
            } else {
                "title-page.docx"
            }),
            "Title page",
            &title_page_paragraphs(project),
        )?;
    }
    write_submission_fields(
        &staging.join("author-tools/submission-fields.xlsx"),
        project,
    )?;
    let manuscript_note = if anonymized_manuscript.is_some() {
        "The author-verified anonymized DOCX replaces the identified source in publisher files and is copied without layout transformation.\n作者核对的匿名 DOCX 已替代实名原稿进入出版社材料，并按原格式复制。"
    } else if editable_manuscript.is_some() {
        "The author-verified editable DOCX replaces the PDF source in publisher files and is copied without layout transformation. The PDF was not converted.\n作者核对的可编辑 DOCX 已替代 PDF 原稿进入出版社材料，并按原格式复制；系统未转换 PDF。"
    } else {
        "The source manuscript is copied without layout transformation.\n主稿按原格式复制，未执行排版变换。"
    };
    let readme = format!("投稿舱 ManuscriptDock local package\nMode: {mode}\n\nThis folder was prepared locally and has not been submitted to a journal.\n{manuscript_note}\nFiles ending in -DRAFT require author review and are excluded from publisher-files.zip.\nThe publisher-files.zip archive contains submission/ only. Recheck the latest official journal requirements before submission.\n\n本文件夹仅在本机生成，应用未执行真实投稿。名称含 -DRAFT 的作者工具须由作者复核，且不会进入出版社 ZIP。投稿前请再次核对期刊官网最新要求。\n");
    write_file(&staging.join("README.txt"), readme.as_bytes())?;
    let record = serde_json::to_vec_pretty(&serde_json::json!({"projectId": project.id, "contextHash": context_hash, "mode": mode, "target": project.target, "compilerVersion": crate::COMPILER_VERSION})).map_err(|_| AppError::new("PROJECT_INVALID", false))?;
    write_file(&staging.join("records/manifest.json"), &record)?;
    let rule_record = serde_json::to_vec_pretty(&serde_json::json!({
        "journalId": target.journal_id,
        "rulesHash": current_rules_hash,
        "rules": crate::rules_for(&target.journal_id)?,
    }))
    .map_err(|_| AppError::new("RULES_INVALID", false))?;
    write_file(&staging.join("records/rules.json"), &rule_record)?;
    write_file(
        &staging.join("records/resource-manifest.json"),
        include_bytes!("../journal-data/resource-manifest.json"),
    )?;
    write_file(
        &staging.join("records/resource-manifest.sig"),
        include_bytes!("../journal-data/resource-manifest.sig"),
    )?;
    write_file(
        &staging.join("records/resource-public-key.hex"),
        include_bytes!("../journal-data/resource-public-key.hex"),
    )?;
    let confirmation_record = serde_json::to_vec_pretty(&serde_json::json!({
        "authorConfirmedFields": project.facts.author_confirmed_fields,
        "authorDecisions": project.author_decisions,
        "sourceHash": project.active_source.sha256,
        "projectRevision": project.revision,
    }))
    .map_err(|_| AppError::new("PROJECT_INVALID", false))?;
    write_file(
        &staging.join("records/author-confirmation.json"),
        &confirmation_record,
    )?;
    let mut checklist = vec![
        format!("Title: {title}"),
        format!("Target: {journal}"),
        format!("Mode: {mode}"),
        format!("Blockers remaining: {}", preparation.blockers.len()),
        if anonymized_manuscript.is_some() {
            "Manuscript: author-verified anonymized DOCX copied unchanged; no automatic identity removal claimed."
                .into()
        } else if editable_manuscript.is_some() {
            "Manuscript: author-verified editable DOCX copied unchanged; the PDF source was not converted."
                .into()
        } else {
            "Source manuscript: copied unchanged; no layout or anonymous transform claimed.".into()
        },
        "Official journal requirements: author must recheck before submission.".into(),
    ];
    checklist.extend(
        preparation
            .ready_items
            .iter()
            .map(|item| format!("READY: {}", item.label.en)),
    );
    checklist.extend(
        preparation
            .blockers
            .iter()
            .map(|item| format!("MISSING: {}", item.label.en)),
    );
    write_docx(
        &staging.join("author-tools/compliance-report.docx"),
        "Submission package compliance report",
        &checklist,
    )?;
    create_publisher_zip(&staging)?;
    let mut artifact_files = collect_files(&staging)?;
    artifact_files.sort_by(|left, right| left.relative_path.cmp(&right.relative_path));
    let file_manifest = artifact_files
        .iter()
        .map(|file| {
            serde_json::json!({
                "relativePath": file.relative_path,
                "sha256": file.sha256,
                "sizeBytes": file.size_bytes,
                "publisherFile": file.publisher_file,
            })
        })
        .collect::<Vec<_>>();
    let file_manifest = serde_json::to_vec_pretty(&serde_json::json!({
        "schemaVersion": 1,
        "files": file_manifest,
    }))
    .map_err(|_| AppError::new("PROJECT_INVALID", false))?;
    write_file(&staging.join("records/file-manifest.json"), &file_manifest)?;
    let mut files = collect_files(&staging)?;
    files.sort_by(|left, right| left.relative_path.cmp(&right.relative_path));
    let mut warnings = vec![LocalizedText::new(
        "所选主稿保留原格式；作者工具草稿不会进入出版社 ZIP",
        "The selected manuscript keeps its original formatting; author-tool drafts are excluded from the publisher ZIP",
    ), LocalizedText::new(
        "投稿前请在期刊官网复核最新要求",
        "Recheck the latest requirements on the journal website before submission",
    )];
    if mode == "draft" {
        warnings.push(LocalizedText::new(
            "草稿可能包含待补字段，不可直接投稿",
            "The draft may contain incomplete fields and is not ready for submission",
        ));
    }
    let package = CompiledPackage {
        id: package_id,
        project_id: project.id.clone(),
        context_hash: context_hash.into(),
        mode: mode.into(),
        staging_dir: staging.to_string_lossy().into_owned(),
        files,
        validation_passed: mode == "draft" || preparation.blockers.is_empty(),
        warnings,
    };
    staging_cleanup.preserve();
    Ok(package)
}

pub(crate) fn planned_package_files(project: &Project) -> Vec<crate::PlannedFile> {
    let main_name = manuscript_name("final", "docx");
    let has_anonymized = project
        .materials
        .iter()
        .any(|material| material.included && material.kind == "anonymized_manuscript");
    let has_editable = project
        .materials
        .iter()
        .any(|material| material.included && material.kind == "editable_manuscript");
    let mut files = vec![crate::PlannedFile {
        relative_path: format!("submission/{main_name}"),
        operation: if has_anonymized {
            "use_author_verified_anonymized_manuscript".into()
        } else if project.active_source.format == "pdf" && has_editable {
            "use_author_verified_editable_manuscript".into()
        } else if project.active_source.format == "pdf" {
            "provide_editable_manuscript".into()
        } else {
            "copy_source_unchanged".into()
        },
        publisher_file: true,
    }];
    files.extend(
        project
            .materials
            .iter()
            .filter(|material| material.included && !is_manuscript_material(&material.kind))
            .zip(planned_material_names(project, &main_name))
            .map(|(_material, name)| crate::PlannedFile {
                relative_path: format!("submission/{name}"),
                operation: "copy_attachment_unchanged".into(),
                publisher_file: true,
            }),
    );
    files.push(crate::PlannedFile {
        relative_path: "submission/declaration-of-competing-interests.docx".into(),
        operation: "generate_from_confirmed_fact".into(),
        publisher_file: true,
    });
    if publishable_highlights(project) {
        files.push(crate::PlannedFile {
            relative_path: "submission/highlights.docx".into(),
            operation: "generate_from_confirmed_fact".into(),
            publisher_file: true,
        });
    }
    if project
        .materials
        .iter()
        .any(|material| material.included && material.kind == "anonymized_manuscript")
    {
        files.push(crate::PlannedFile {
            relative_path: "submission/title-page.docx".into(),
            operation: "generate_from_confirmed_fact".into(),
            publisher_file: true,
        });
    }
    for relative_path in [
        "author-tools/title-page-DRAFT.docx",
        "author-tools/cover-letter-DRAFT.docx",
        "author-tools/highlights-DRAFT.docx",
        "author-tools/declarations-DRAFT.docx",
        "author-tools/submission-fields.xlsx",
        "author-tools/compliance-report.docx",
        "records/manifest.json",
        "records/rules.json",
        "records/author-confirmation.json",
        "records/resource-manifest.json",
        "records/resource-manifest.sig",
        "records/resource-public-key.hex",
        "records/file-manifest.json",
        "README.txt",
        "publisher-files.zip",
    ] {
        files.push(crate::PlannedFile {
            relative_path: relative_path.into(),
            operation: if relative_path == "publisher-files.zip" {
                "archive_submission_files".into()
            } else if relative_path == "README.txt" || relative_path.starts_with("records/") {
                "write_audit_record".into()
            } else {
                "generate_author_tool".into()
            },
            publisher_file: relative_path == "publisher-files.zip",
        });
    }
    files
}

fn manuscript_name(mode: &str, extension: &str) -> String {
    if mode == "draft" {
        format!("manuscript-DRAFT.{extension}")
    } else {
        format!("manuscript.{extension}")
    }
}

fn is_manuscript_material(kind: &str) -> bool {
    matches!(kind, "anonymized_manuscript" | "editable_manuscript")
}

fn planned_material_names(project: &Project, main_name: &str) -> Vec<String> {
    let mut used = HashSet::from([main_name.to_lowercase()]);
    project
        .materials
        .iter()
        .filter(|material| material.included && !is_manuscript_material(&material.kind))
        .map(|material| unique_submission_name(&material.file_name, &material.id, &mut used))
        .collect()
}

fn publishable_highlights(project: &Project) -> bool {
    let highlights = &project.facts.highlights;
    (3..=5).contains(&highlights.len())
        && highlights.iter().all(|item| item.chars().count() <= 85)
        && project
            .facts
            .author_confirmed_fields
            .iter()
            .any(|field| field == "highlights")
}

fn copy_verified(
    source: &Path,
    destination: PathBuf,
    expected_hash: &str,
    expected_size: u64,
) -> Result<(), AppError> {
    fs::copy(source, &destination).map_err(|_| AppError::new("STORAGE_UNAVAILABLE", true))?;
    verify_snapshot(&destination, expected_hash, expected_size)
}

fn verify_snapshot(path: &Path, expected_hash: &str, expected_size: u64) -> Result<(), AppError> {
    let bytes = fs::read(path).map_err(|_| AppError::new("STORAGE_UNAVAILABLE", true))?;
    if bytes.len() as u64 != expected_size || hex::encode(Sha256::digest(&bytes)) != expected_hash {
        return Err(AppError::new("SNAPSHOT_CHANGED", true).recover("create_new_task"));
    }
    Ok(())
}

fn title_page_paragraphs(project: &Project) -> Vec<String> {
    vec![
        project
            .facts
            .title
            .clone()
            .unwrap_or_else(|| "[TITLE REQUIRED]".into()),
        format!("Authors: {}", joined_or_placeholder(&project.facts.authors)),
        format!(
            "Affiliations: {}",
            joined_or_placeholder(&project.facts.affiliations)
        ),
        format!(
            "Corresponding-author email: {}",
            project
                .facts
                .corresponding_email
                .as_deref()
                .unwrap_or("[EMAIL REQUIRED]")
        ),
        format!(
            "Competing interests: {}",
            project
                .facts
                .conflict_of_interest
                .as_deref()
                .unwrap_or("[AUTHOR CONFIRMATION REQUIRED]")
        ),
    ]
}

fn declaration_paragraphs(project: &Project) -> Vec<String> {
    [
        (
            "Competing interests",
            project.facts.conflict_of_interest.as_deref(),
        ),
        ("Funding", project.facts.funding.as_deref()),
        (
            "Data availability",
            project.facts.data_availability.as_deref(),
        ),
        (
            "Ethics statement",
            project.facts.ethics_statement.as_deref(),
        ),
        (
            "CRediT author contributions",
            project.facts.credit_contributions.as_deref(),
        ),
        (
            "Generative-AI disclosure",
            project.facts.generative_ai_disclosure.as_deref(),
        ),
    ]
    .into_iter()
    .map(|(label, value)| {
        format!(
            "{label}: {}",
            value
                .filter(|item| !item.trim().is_empty())
                .unwrap_or("[AUTHOR CONFIRMATION REQUIRED]")
        )
    })
    .collect()
}

fn joined_or_placeholder(values: &[String]) -> String {
    if values.is_empty() {
        "[AUTHOR INPUT REQUIRED]".into()
    } else {
        values.join("; ")
    }
}

fn write_submission_fields(path: &Path, project: &Project) -> Result<(), AppError> {
    let mut workbook = Workbook::new();
    let worksheet = workbook.add_worksheet();
    let heading = Format::new().set_bold();
    worksheet
        .write_string_with_format(0, 0, "Field", &heading)
        .map_err(|_| AppError::new("XLSX_GENERATION_FAILED", true))?;
    worksheet
        .write_string_with_format(0, 1, "Confirmed value", &heading)
        .map_err(|_| AppError::new("XLSX_GENERATION_FAILED", true))?;
    let rows = [
        (
            "Manuscript title",
            project.facts.title.clone().unwrap_or_default(),
        ),
        ("Authors", project.facts.authors.join("; ")),
        ("Affiliations", project.facts.affiliations.join("; ")),
        (
            "Corresponding-author email",
            project
                .facts
                .corresponding_email
                .clone()
                .unwrap_or_default(),
        ),
        (
            "Competing interests",
            project
                .facts
                .conflict_of_interest
                .clone()
                .unwrap_or_default(),
        ),
        ("Funding", project.facts.funding.clone().unwrap_or_default()),
        (
            "Data availability",
            project.facts.data_availability.clone().unwrap_or_default(),
        ),
        (
            "Ethics statement",
            project.facts.ethics_statement.clone().unwrap_or_default(),
        ),
        ("Highlights", project.facts.highlights.join("; ")),
        (
            "CRediT author contributions",
            project
                .facts
                .credit_contributions
                .clone()
                .unwrap_or_default(),
        ),
        (
            "Generative-AI disclosure",
            project
                .facts
                .generative_ai_disclosure
                .clone()
                .unwrap_or_default(),
        ),
        (
            "Abstract",
            project.facts.abstract_text.clone().unwrap_or_default(),
        ),
        ("Keywords", project.facts.keywords.join("; ")),
    ];
    for (index, (field, value)) in rows.iter().enumerate() {
        let row =
            u32::try_from(index + 1).map_err(|_| AppError::new("XLSX_GENERATION_FAILED", false))?;
        worksheet
            .write_string(row, 0, *field)
            .map_err(|_| AppError::new("XLSX_GENERATION_FAILED", true))?;
        worksheet
            .write_string(row, 1, value)
            .map_err(|_| AppError::new("XLSX_GENERATION_FAILED", true))?;
    }
    worksheet
        .set_column_width(0, 30)
        .map_err(|_| AppError::new("XLSX_GENERATION_FAILED", true))?;
    worksheet
        .set_column_width(1, 76)
        .map_err(|_| AppError::new("XLSX_GENERATION_FAILED", true))?;
    worksheet.set_landscape().set_print_fit_to_pages(1, 1);
    workbook
        .save(path)
        .map_err(|_| AppError::new("XLSX_GENERATION_FAILED", true))
}

fn write_docx(path: &Path, document_title: &str, paragraphs: &[String]) -> Result<(), AppError> {
    let file = fs::File::create(path).map_err(|_| AppError::new("DOCX_GENERATION_FAILED", true))?;
    let mut writer = ZipWriter::new(file);
    let options = SimpleFileOptions::default()
        .compression_method(CompressionMethod::Deflated)
        .unix_permissions(0o644);
    let content_types = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/><Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/><Override PartName="/docProps/core.xml" ContentType="application/vnd.openxmlformats-package.core-properties+xml"/></Types>"#;
    let rels = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/><Relationship Id="rId2" Type="http://schemas.openxmlformats.org/package/2006/relationships/metadata/core-properties" Target="docProps/core.xml"/></Relationships>"#;
    let core = format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><cp:coreProperties xmlns:cp="http://schemas.openxmlformats.org/package/2006/metadata/core-properties" xmlns:dc="http://purl.org/dc/elements/1.1/"><dc:title>{}</dc:title><dc:creator>ManuscriptDock</dc:creator></cp:coreProperties>"#,
        xml_escape(document_title)
    );
    let body = paragraphs
        .iter()
        .map(|paragraph| {
            format!(
                r#"<w:p><w:r><w:t xml:space="preserve">{}</w:t></w:r></w:p>"#,
                xml_escape(paragraph)
            )
        })
        .collect::<String>();
    let document = format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:body>{body}<w:sectPr><w:pgSz w:w="12240" w:h="15840"/><w:pgMar w:top="1440" w:right="1440" w:bottom="1440" w:left="1440"/></w:sectPr></w:body></w:document>"#
    );
    for (name, contents) in [
        ("[Content_Types].xml", content_types.to_string()),
        ("_rels/.rels", rels.to_string()),
        ("docProps/core.xml", core),
        ("word/document.xml", document),
    ] {
        writer
            .start_file(name, options)
            .map_err(|_| AppError::new("DOCX_GENERATION_FAILED", true))?;
        writer
            .write_all(contents.as_bytes())
            .map_err(|_| AppError::new("DOCX_GENERATION_FAILED", true))?;
    }
    writer
        .finish()
        .map_err(|_| AppError::new("DOCX_GENERATION_FAILED", true))?;
    Ok(())
}

fn xml_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

pub fn export_package(
    package: &CompiledPackage,
    destination: ExportDestination,
) -> Result<ExportReceipt, AppError> {
    if !package.validation_passed {
        return Err(AppError::new("PACKAGE_VALIDATION_FAILED", true));
    }
    let destination = destination.0;
    fs::create_dir_all(&destination).map_err(|_| {
        AppError::new("OUTPUT_PERMISSION_DENIED", true).recover("choose_export_folder")
    })?;
    let folder_name = format!(
        "ManuscriptDock-{}-{}",
        if package.mode == "draft" {
            "DRAFT"
        } else {
            "PACKAGE"
        },
        &package.id[..8]
    );
    let temporary = destination.join(format!(".{folder_name}.tmp"));
    let final_path = destination.join(&folder_name);
    if final_path.exists() || temporary.exists() {
        return Err(AppError::new("OUTPUT_ALREADY_EXISTS", true).recover("choose_export_folder"));
    }
    fs::create_dir(&temporary).map_err(|_| {
        AppError::new("OUTPUT_PERMISSION_DENIED", true).recover("choose_export_folder")
    })?;
    let mut temporary_cleanup = CleanupDirectory::new(temporary.clone());
    copy_tree(Path::new(&package.staging_dir), &temporary)?;
    for file in &package.files {
        if hash_file(&temporary.join(&file.relative_path))? != file.sha256 {
            return Err(AppError::new("OUTPUT_VERIFICATION_FAILED", true));
        }
    }
    let package_hash = hex::encode(Sha256::digest(
        serde_json::to_vec(&package.files).map_err(|_| AppError::new("PROJECT_INVALID", false))?,
    ));
    let finished_at_unix_ms = now_ms()?;
    fs::rename(&temporary, &final_path)
        .map_err(|_| AppError::new("OUTPUT_PERMISSION_DENIED", true))?;
    temporary_cleanup.preserve();
    Ok(ExportReceipt {
        id: uuid::Uuid::new_v4().to_string(),
        request_id: None,
        package_id: package.id.clone(),
        output_directory: final_path.to_string_lossy().into_owned(),
        file_count: package.files.len(),
        package_hash,
        finished_at_unix_ms,
        record_persisted: false,
    })
}

fn create_publisher_zip(root: &Path) -> Result<(), AppError> {
    let file = fs::File::create(root.join("publisher-files.zip"))
        .map_err(|_| AppError::new("STORAGE_UNAVAILABLE", true))?;
    let mut writer = ZipWriter::new(file);
    let options = SimpleFileOptions::default()
        .compression_method(CompressionMethod::Deflated)
        .unix_permissions(0o644);
    let submission = root.join("submission");
    let mut paths = fs::read_dir(&submission)
        .map_err(|_| AppError::new("STORAGE_UNAVAILABLE", true))?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.is_file())
        .collect::<Vec<_>>();
    paths.sort();
    for path in paths {
        let name = path
            .file_name()
            .and_then(|value| value.to_str())
            .ok_or_else(|| AppError::new("OUTPUT_VERIFICATION_FAILED", false))?;
        writer
            .start_file(format!("submission/{name}"), options)
            .map_err(|_| AppError::new("STORAGE_UNAVAILABLE", true))?;
        let mut input =
            fs::File::open(&path).map_err(|_| AppError::new("STORAGE_UNAVAILABLE", true))?;
        std::io::copy(&mut input, &mut writer)
            .map_err(|_| AppError::new("STORAGE_UNAVAILABLE", true))?;
    }
    writer
        .finish()
        .map_err(|_| AppError::new("STORAGE_UNAVAILABLE", true))?;
    Ok(())
}
fn collect_files(root: &Path) -> Result<Vec<GeneratedFile>, AppError> {
    let mut paths = vec![root.to_path_buf()];
    let mut files = Vec::new();
    while let Some(path) = paths.pop() {
        for entry in fs::read_dir(&path).map_err(|_| AppError::new("STORAGE_UNAVAILABLE", true))? {
            let entry = entry.map_err(|_| AppError::new("STORAGE_UNAVAILABLE", true))?;
            let path = entry.path();
            if path.is_dir() {
                paths.push(path);
                continue;
            }
            let relative = path
                .strip_prefix(root)
                .map_err(|_| AppError::new("OUTPUT_VERIFICATION_FAILED", false))?
                .to_string_lossy()
                .replace('\\', "/");
            let metadata = path
                .metadata()
                .map_err(|_| AppError::new("STORAGE_UNAVAILABLE", true))?;
            files.push(GeneratedFile {
                id: uuid::Uuid::new_v4().to_string(),
                relative_path: relative.clone(),
                purpose: purpose(&relative),
                sha256: hash_file(&path)?,
                size_bytes: metadata.len(),
                publisher_file: relative.starts_with("submission/")
                    || relative == "publisher-files.zip",
            });
        }
    }
    Ok(files)
}
fn purpose(path: &str) -> LocalizedText {
    if path.starts_with("submission/") {
        LocalizedText::new("出版社投稿材料", "Publisher submission file")
    } else if path.starts_with("author-tools/") {
        LocalizedText::new("作者核对工具", "Author review tool")
    } else if path.starts_with("records/") {
        LocalizedText::new("本地内部记录", "Local internal record")
    } else if path.ends_with(".zip") {
        LocalizedText::new(
            "仅含投稿材料的压缩包",
            "Archive containing submission files only",
        )
    } else {
        LocalizedText::new("使用说明", "Usage notes")
    }
}
fn sanitize_name(value: &str) -> String {
    value
        .chars()
        .map(|c| {
            if matches!(c, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|') {
                '_'
            } else {
                c
            }
        })
        .collect()
}
fn unique_submission_name(value: &str, id: &str, used: &mut HashSet<String>) -> String {
    let sanitized = sanitize_name(value);
    let sanitized = if sanitized.is_empty() {
        "attachment".to_owned()
    } else {
        sanitized
    };
    if used.insert(sanitized.to_lowercase()) {
        return sanitized;
    }
    let path = Path::new(&sanitized);
    let stem = path
        .file_stem()
        .and_then(|value| value.to_str())
        .filter(|value| !value.is_empty())
        .unwrap_or("attachment");
    let extension = path.extension().and_then(|value| value.to_str());
    let short_id = id.get(..8).unwrap_or(id);
    for index in 0_u32.. {
        let suffix = if index == 0 {
            format!("-{short_id}")
        } else {
            format!("-{short_id}-{index}")
        };
        let candidate = extension.map_or_else(
            || format!("{stem}{suffix}"),
            |extension| format!("{stem}{suffix}.{extension}"),
        );
        if used.insert(candidate.to_lowercase()) {
            return candidate;
        }
    }
    unreachable!("u32 filename suffix space exhausted")
}
fn write_file(path: &Path, bytes: &[u8]) -> Result<(), AppError> {
    let mut file =
        fs::File::create(path).map_err(|_| AppError::new("STORAGE_UNAVAILABLE", true))?;
    file.write_all(bytes)
        .and_then(|_| file.sync_all())
        .map_err(|_| AppError::new("STORAGE_UNAVAILABLE", true))
}
fn hash_file(path: &Path) -> Result<String, AppError> {
    let mut file =
        fs::File::open(path).map_err(|_| AppError::new("OUTPUT_VERIFICATION_FAILED", true))?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let count = file
            .read(&mut buffer)
            .map_err(|_| AppError::new("OUTPUT_VERIFICATION_FAILED", true))?;
        if count == 0 {
            break;
        }
        hasher.update(&buffer[..count]);
    }
    Ok(hex::encode(hasher.finalize()))
}
fn copy_tree(source: &Path, destination: &Path) -> Result<(), AppError> {
    fs::create_dir_all(destination).map_err(|_| AppError::new("OUTPUT_PERMISSION_DENIED", true))?;
    for entry in
        fs::read_dir(source).map_err(|_| AppError::new("OUTPUT_PERMISSION_DENIED", true))?
    {
        let entry = entry.map_err(|_| AppError::new("OUTPUT_PERMISSION_DENIED", true))?;
        let target = destination.join(entry.file_name());
        if entry.path().is_dir() {
            copy_tree(&entry.path(), &target)?;
        } else {
            fs::copy(entry.path(), target)
                .map_err(|_| AppError::new("OUTPUT_PERMISSION_DENIED", true))?;
        }
    }
    Ok(())
}
fn now_ms() -> Result<u64, AppError> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| AppError::new("SYSTEM_TIME_INVALID", false))
        .and_then(|value| {
            u64::try_from(value.as_millis())
                .map_err(|_| AppError::new("SYSTEM_TIME_INVALID", false))
        })
}
