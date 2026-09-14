use manuscript_core::{
    build_package, export_package, prepare, ExportDestination, PreparationStatus, ProjectStore,
    TargetOrigin, TargetSelection, TaskKind,
};
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::{Arc, Barrier},
};
use zip::{write::SimpleFileOptions, CompressionMethod, ZipArchive, ZipWriter};

#[test]
fn final_package_is_transactional_and_keeps_internal_files_out_of_publisher_zip() {
    let root = temporary_root("final-package");
    let source = root.join("synthetic-manuscript.docx");
    write_synthetic_docx(&source);
    let source_before = fs::read(&source).unwrap();
    let store = ProjectStore::new(root.join("workspace")).unwrap();
    let mut project = store
        .create_from_docx(&source, TaskKind::PreparePackage)
        .unwrap();
    project = store
        .select_target(
            &project.id,
            project.revision,
            TargetSelection {
                id: uuid::Uuid::new_v4().to_string(),
                journal_id: "elsevier-artificial-intelligence".into(),
                article_type: "research_article".into(),
                stage: "initial_submission".into(),
                origin: TargetOrigin::Catalog,
                recommendation_ref: None,
                rules_hash: manuscript_core::rules_hash_for("elsevier-artificial-intelligence")
                    .unwrap(),
            },
        )
        .unwrap();
    let mut facts = project.facts.clone();
    facts.authors = vec!["=2+2".into(), "张三".into()];
    facts.affiliations = vec!["Example University 大学".into()];
    facts.corresponding_email = Some("ada@example.invalid".into());
    facts.conflict_of_interest = Some("The authors declare no competing interests.".into());
    facts.funding = Some("Supported by synthetic grant G-1.".into());
    facts.data_availability = Some("Synthetic data are included with the test.".into());
    facts.ethics_statement = Some("No human or animal participants.".into());
    facts.highlights = vec![
        "First synthetic highlight".into(),
        "Second highlight".into(),
        "Third highlight".into(),
    ];
    facts.credit_contributions = Some(
        "Ada Example: Conceptualization, Writing – original draft; Zhang San: Validation.".into(),
    );
    facts.author_confirmed_fields = vec![
        "title".into(),
        "abstract_text".into(),
        "keywords".into(),
        "authors".into(),
        "affiliations".into(),
        "corresponding_email".into(),
        "conflict_of_interest".into(),
        "funding".into(),
        "data_availability".into(),
        "ethics_statement".into(),
        "highlights".into(),
        "credit_contributions".into(),
        "generative_ai_disclosure".into(),
    ];
    project = store
        .update_facts(&project.id, project.revision, facts)
        .unwrap();
    assert_eq!(project.author_decisions.len(), 11);
    assert!(project
        .author_decisions
        .iter()
        .all(|decision| !decision.context_hash.is_empty()
            && decision.evidence_ref.starts_with("https://")));
    let mut stale_decision_project = project.clone();
    stale_decision_project
        .facts
        .authors
        .push("Unconfirmed author".into());
    assert_ne!(
        prepare(&stale_decision_project).unwrap().status,
        PreparationStatus::ReadyToExport
    );
    let first_attachment_dir = root.join("attachment-a");
    let second_attachment_dir = root.join("attachment-b");
    fs::create_dir_all(&first_attachment_dir).unwrap();
    fs::create_dir_all(&second_attachment_dir).unwrap();
    fs::write(
        first_attachment_dir.join("manuscript.docx"),
        b"first attachment",
    )
    .unwrap();
    fs::write(
        second_attachment_dir.join("manuscript.docx"),
        b"second attachment",
    )
    .unwrap();
    project = store
        .add_material(
            &project.id,
            project.revision,
            &first_attachment_dir.join("manuscript.docx"),
            "supplement",
        )
        .unwrap();
    project = store
        .add_material(
            &project.id,
            project.revision,
            &second_attachment_dir.join("manuscript.docx"),
            "supplement",
        )
        .unwrap();
    let preparation = prepare(&project).unwrap();
    assert_eq!(preparation.status, PreparationStatus::ReadyToExport);
    let package = build_package(&store, &project, &preparation.context_hash, "final").unwrap();
    assert_eq!(
        preparation
            .package_plan
            .file_plan
            .iter()
            .map(|file| file.relative_path.as_str())
            .collect::<std::collections::BTreeSet<_>>(),
        package
            .files
            .iter()
            .map(|file| file.relative_path.as_str())
            .collect::<std::collections::BTreeSet<_>>()
    );
    store.save_compiled_package(&package).unwrap();
    let recovered_package = ProjectStore::new(root.join("workspace"))
        .unwrap()
        .get_compiled_package(&package.id)
        .unwrap();
    assert_eq!(recovered_package.files, package.files);
    assert_eq!(recovered_package.staging_dir, package.staging_dir);

    for relative in [
        "submission/manuscript.docx",
        "submission/declaration-of-competing-interests.docx",
        "submission/highlights.docx",
        "author-tools/title-page-DRAFT.docx",
        "author-tools/cover-letter-DRAFT.docx",
        "author-tools/highlights-DRAFT.docx",
        "author-tools/declarations-DRAFT.docx",
        "author-tools/submission-fields.xlsx",
        "author-tools/compliance-report.docx",
        "records/manifest.json",
        "records/file-manifest.json",
        "records/resource-manifest.json",
        "records/resource-manifest.sig",
        "records/resource-public-key.hex",
        "publisher-files.zip",
        "README.txt",
    ] {
        assert!(
            package
                .files
                .iter()
                .any(|file| file.relative_path == relative),
            "missing {relative}"
        );
    }
    assert!(package.validation_passed);
    let submission_docx = package
        .files
        .iter()
        .filter(|file| {
            file.relative_path.starts_with("submission/") && file.relative_path.ends_with(".docx")
        })
        .map(|file| file.relative_path.clone())
        .collect::<Vec<_>>();
    assert_eq!(submission_docx.len(), 5);
    assert_eq!(
        submission_docx
            .iter()
            .map(|name| name.to_lowercase())
            .collect::<std::collections::HashSet<_>>()
            .len(),
        5
    );
    assert_eq!(fs::read(&source).unwrap(), source_before);
    assert_eq!(
        fs::read(store.source_path(&project)).unwrap(),
        source_before
    );

    let workbook_file =
        fs::File::open(Path::new(&package.staging_dir).join("author-tools/submission-fields.xlsx"))
            .unwrap();
    let mut workbook = ZipArchive::new(workbook_file).unwrap();
    let mut worksheet_xml = String::new();
    workbook
        .by_name("xl/worksheets/sheet1.xml")
        .unwrap()
        .read_to_string(&mut worksheet_xml)
        .unwrap();
    assert!(
        !worksheet_xml.contains("<f>"),
        "author text became a formula"
    );
    let mut shared_strings = String::new();
    workbook
        .by_name("xl/sharedStrings.xml")
        .unwrap()
        .read_to_string(&mut shared_strings)
        .unwrap();
    assert!(shared_strings.contains("=2+2"));
    assert!(shared_strings.contains("张三"));
    assert!(shared_strings.contains("Supported by synthetic grant G-1."));
    assert!(shared_strings.contains("First synthetic highlight"));
    assert!(shared_strings.contains("Ada Example: Conceptualization"));

    let declaration_xml = read_docx_document_xml(
        &Path::new(&package.staging_dir).join("author-tools/declarations-DRAFT.docx"),
    );
    assert!(declaration_xml.contains("Synthetic data are included with the test."));
    assert!(declaration_xml.contains("No human or animal participants."));
    let highlights_xml = read_docx_document_xml(
        &Path::new(&package.staging_dir).join("author-tools/highlights-DRAFT.docx"),
    );
    assert!(highlights_xml.contains("First synthetic highlight"));

    let zip_file =
        fs::File::open(Path::new(&package.staging_dir).join("publisher-files.zip")).unwrap();
    let mut archive = ZipArchive::new(zip_file).unwrap();
    let names = (0..archive.len())
        .map(|index| archive.by_index(index).unwrap().name().to_owned())
        .collect::<Vec<_>>();
    assert!(names.iter().all(|name| name.starts_with("submission/")));
    assert!(!names.iter().any(|name| name.contains("author-tools")));

    let mut receipt =
        export_package(&recovered_package, ExportDestination(root.join("exports"))).unwrap();
    let export_request_id = uuid::Uuid::new_v4().to_string();
    receipt.request_id = Some(export_request_id.clone());
    assert_eq!(receipt.file_count, package.files.len());
    assert!(!receipt.record_persisted);
    store.record_export_receipt(&mut receipt).unwrap();
    assert!(receipt.record_persisted);
    let persisted_receipt = fs::read_to_string(
        root.join("workspace")
            .join("receipts")
            .join(format!("{}.json", receipt.id)),
    )
    .unwrap();
    assert!(persisted_receipt.contains("\"recordPersisted\": true"));
    assert_eq!(
        store
            .export_receipt_for_request(&export_request_id)
            .unwrap()
            .unwrap()
            .id,
        receipt.id
    );
    let second = export_package(&package, ExportDestination(root.join("exports"))).unwrap_err();
    assert_eq!(second.code, "OUTPUT_ALREADY_EXISTS");

    fs::remove_file(Path::new(&package.staging_dir).join("README.txt")).unwrap();
    let failed_destination = root.join("failed-exports");
    let failed =
        export_package(&package, ExportDestination(failed_destination.clone())).unwrap_err();
    assert_eq!(failed.code, "OUTPUT_VERIFICATION_FAILED");
    assert!(!failed_destination
        .join(format!(".ManuscriptDock-PACKAGE-{}.tmp", &package.id[..8]))
        .exists());
    assert!(Path::new(&receipt.output_directory).is_dir());
    assert_eq!(fs::read(&source).unwrap(), source_before);
    finish_test_root(root);
}

#[test]
fn changed_or_unconfirmed_facts_block_a_final_package() {
    let root = temporary_root("unconfirmed");
    let source = root.join("synthetic.docx");
    write_synthetic_docx(&source);
    let store = ProjectStore::new(root.join("workspace")).unwrap();
    let mut project = store
        .create_from_docx(&source, TaskKind::PreparePackage)
        .unwrap();
    project = store
        .select_target(
            &project.id,
            project.revision,
            TargetSelection {
                id: uuid::Uuid::new_v4().to_string(),
                journal_id: "elsevier-artificial-intelligence".into(),
                article_type: "research_article".into(),
                stage: "initial_submission".into(),
                origin: TargetOrigin::Catalog,
                recommendation_ref: None,
                rules_hash: manuscript_core::rules_hash_for("elsevier-artificial-intelligence")
                    .unwrap(),
            },
        )
        .unwrap();
    let preparation = prepare(&project).unwrap();
    assert_ne!(preparation.status, PreparationStatus::ReadyToExport);
    let error = build_package(&store, &project, &preparation.context_hash, "final").unwrap_err();
    assert_eq!(error.code, "MATERIAL_REQUIRED");
    finish_test_root(root);
}

#[test]
fn a_tampered_local_snapshot_is_never_packaged() {
    let root = temporary_root("tampered-snapshot");
    let source = root.join("synthetic.docx");
    write_synthetic_docx(&source);
    let source_before = fs::read(&source).unwrap();
    let store = ProjectStore::new(root.join("workspace")).unwrap();
    let mut project = store
        .create_from_docx(&source, TaskKind::PreparePackage)
        .unwrap();
    project = store
        .select_target(
            &project.id,
            project.revision,
            TargetSelection {
                id: uuid::Uuid::new_v4().to_string(),
                journal_id: "elsevier-artificial-intelligence".into(),
                article_type: "research_article".into(),
                stage: "initial_submission".into(),
                origin: TargetOrigin::Catalog,
                recommendation_ref: None,
                rules_hash: manuscript_core::rules_hash_for("elsevier-artificial-intelligence")
                    .unwrap(),
            },
        )
        .unwrap();
    let preparation = prepare(&project).unwrap();
    fs::write(store.source_path(&project), b"tampered local snapshot").unwrap();
    let error = build_package(&store, &project, &preparation.context_hash, "draft").unwrap_err();
    assert_eq!(error.code, "SNAPSHOT_CHANGED");
    assert_eq!(fs::read(&source).unwrap(), source_before);
    finish_test_root(root);
}

#[test]
fn eswa_requires_a_constrained_fact_set_and_uses_the_author_verified_anonymized_docx() {
    let root = temporary_root("eswa-anonymized");
    let source = root.join("identified.docx");
    let anonymous = root.join("anonymous.docx");
    let leaky_anonymous = root.join("leaky-anonymous.docx");
    write_synthetic_docx_named(&source, "Identified Research Manuscript");
    write_synthetic_docx_named(&anonymous, "Anonymous Review Copy");
    write_synthetic_docx_named(
        &leaky_anonymous,
        "Synthetic Author · Example University, Example City, Example Country · author@example.invalid",
    );
    let source_bytes = fs::read(&source).unwrap();
    let anonymous_bytes = fs::read(&anonymous).unwrap();
    assert_ne!(source_bytes, anonymous_bytes);
    let store = ProjectStore::new(root.join("workspace")).unwrap();
    let mut project = store
        .create_from_docx(&source, TaskKind::PreparePackage)
        .unwrap();
    project = store
        .select_target(
            &project.id,
            project.revision,
            TargetSelection {
                id: uuid::Uuid::new_v4().to_string(),
                journal_id: "elsevier-expert-systems-with-applications".into(),
                article_type: "research_article".into(),
                stage: "initial_submission".into(),
                origin: TargetOrigin::Catalog,
                recommendation_ref: None,
                rules_hash: manuscript_core::rules_hash_for(
                    "elsevier-expert-systems-with-applications",
                )
                .unwrap(),
            },
        )
        .unwrap();
    let mut facts = project.facts.clone();
    facts.authors = vec!["Synthetic Author".into()];
    facts.affiliations = vec!["Example University, Example City, Example Country".into()];
    facts.corresponding_email = Some("author@example.invalid".into());
    facts.conflict_of_interest = Some("The author declares no competing interests.".into());
    facts.funding = Some("No external funding was received.".into());
    facts.data_availability = Some("Synthetic data are available in the test fixture.".into());
    facts.keywords = (1..=8).map(|index| format!("keyword-{index}")).collect();
    facts.author_confirmed_fields = vec![
        "title".into(),
        "abstract_text".into(),
        "keywords".into(),
        "authors".into(),
        "affiliations".into(),
        "corresponding_email".into(),
        "conflict_of_interest".into(),
        "funding".into(),
        "data_availability".into(),
    ];
    project = store
        .update_facts(&project.id, project.revision, facts)
        .unwrap();
    let constrained = prepare(&project).unwrap();
    assert!(constrained
        .blockers
        .iter()
        .any(|item| item.requirement_id == "eswa.keywords"));
    assert!(constrained
        .blockers
        .iter()
        .any(|item| item.requirement_id == "eswa.anonymized-manuscript"));

    let mut facts = project.facts.clone();
    facts.keywords.truncate(7);
    project = store
        .update_facts(&project.id, project.revision, facts)
        .unwrap();
    project = store
        .add_material(
            &project.id,
            project.revision,
            &leaky_anonymous,
            "anonymized_manuscript",
        )
        .unwrap();
    let leak_blocked = prepare(&project).unwrap();
    let leak_item = leak_blocked
        .blockers
        .iter()
        .find(|item| item.requirement_id == "eswa.anonymized-manuscript")
        .unwrap();
    assert!(leak_item.description.zh_cn.contains("作者姓名"));
    assert!(leak_item.description.en.contains("author names"));
    let leak_error =
        build_package(&store, &project, &leak_blocked.context_hash, "draft").unwrap_err();
    assert_eq!(leak_error.code, "ANONYMITY_UNVERIFIED");

    project = store
        .add_material(
            &project.id,
            project.revision,
            &anonymous,
            "anonymized_manuscript",
        )
        .unwrap();
    let mut facts = project.facts.clone();
    facts.affiliations = vec!["Anonymous Review Copy".into()];
    project = store
        .update_facts(&project.id, project.revision, facts)
        .unwrap();
    assert!(prepare(&project).unwrap().blockers.iter().any(|item| {
        item.requirement_id == "eswa.anonymized-manuscript"
            && item.description.en.contains("affiliations")
    }));
    let mut facts = project.facts.clone();
    facts.affiliations = vec!["Example University, Example City, Example Country".into()];
    project = store
        .update_facts(&project.id, project.revision, facts)
        .unwrap();
    let preparation = prepare(&project).unwrap();
    assert_eq!(preparation.status, PreparationStatus::ReadyToExport);
    assert_eq!(
        preparation.package_plan.file_plan[0].operation,
        "use_author_verified_anonymized_manuscript"
    );
    let package = build_package(&store, &project, &preparation.context_hash, "final").unwrap();
    let package_root = Path::new(&package.staging_dir);
    assert_eq!(
        fs::read(package_root.join("submission/manuscript.docx")).unwrap(),
        anonymous_bytes
    );
    assert!(package_root.join("submission/title-page.docx").is_file());
    assert_eq!(fs::read(&source).unwrap(), source_bytes);
    finish_test_root(root);
}

#[test]
fn pdf_sources_support_matching_and_drafts_but_final_packages_use_an_explicit_docx() {
    let root = temporary_root("pdf-source");
    let source = root.join("synthetic.pdf");
    let editable = root.join("editable.docx");
    write_simple_pdf(&source);
    write_synthetic_docx_named(&editable, "Author-reviewed editable manuscript");
    let source_bytes = fs::read(&source).unwrap();
    let editable_bytes = fs::read(&editable).unwrap();
    let store = ProjectStore::new(root.join("workspace")).unwrap();
    let mut project = store
        .create_from_manuscript(&source, TaskKind::PreparePackage)
        .unwrap();
    assert_eq!(project.active_source.format, "pdf");
    assert_eq!(
        project.facts.title.as_deref(),
        Some("Synthetic Local Paper")
    );
    assert!(project.facts.authors.is_empty());
    project = store
        .select_target(
            &project.id,
            project.revision,
            TargetSelection {
                id: uuid::Uuid::new_v4().to_string(),
                journal_id: "elsevier-artificial-intelligence".into(),
                article_type: "research_article".into(),
                stage: "initial_submission".into(),
                origin: TargetOrigin::Catalog,
                recommendation_ref: None,
                rules_hash: manuscript_core::rules_hash_for("elsevier-artificial-intelligence")
                    .unwrap(),
            },
        )
        .unwrap();
    let preparation = prepare(&project).unwrap();
    assert!(preparation
        .blockers
        .iter()
        .any(|item| item.requirement_id == "local.editable-manuscript"));
    assert_eq!(
        preparation.package_plan.file_plan[0].operation,
        "provide_editable_manuscript"
    );
    let draft = build_package(&store, &project, &preparation.context_hash, "draft").unwrap();
    assert_eq!(
        fs::read(Path::new(&draft.staging_dir).join("submission/manuscript-DRAFT.pdf")).unwrap(),
        source_bytes
    );
    assert_eq!(
        build_package(&store, &project, &preparation.context_hash, "final")
            .unwrap_err()
            .code,
        "MATERIAL_REQUIRED"
    );

    project = store
        .add_material(
            &project.id,
            project.revision,
            &editable,
            "editable_manuscript",
        )
        .unwrap();
    let mut facts = project.facts.clone();
    facts.authors = vec!["Synthetic Author".into()];
    facts.affiliations = vec!["Example University".into()];
    facts.corresponding_email = Some("author@example.invalid".into());
    facts.conflict_of_interest = Some("The author declares no competing interests.".into());
    facts.funding = Some("No external funding was received.".into());
    facts.data_availability = Some("Synthetic data are included with the test.".into());
    facts.ethics_statement = Some("No human or animal participants.".into());
    facts.highlights = vec![
        "First synthetic highlight".into(),
        "Second synthetic highlight".into(),
        "Third synthetic highlight".into(),
    ];
    facts.credit_contributions = Some("Synthetic Author: Conceptualization.".into());
    facts.author_confirmed_fields = vec![
        "title".into(),
        "abstract_text".into(),
        "keywords".into(),
        "authors".into(),
        "affiliations".into(),
        "corresponding_email".into(),
        "conflict_of_interest".into(),
        "funding".into(),
        "data_availability".into(),
        "ethics_statement".into(),
        "highlights".into(),
        "credit_contributions".into(),
        "generative_ai_disclosure".into(),
    ];
    project = store
        .update_facts(&project.id, project.revision, facts)
        .unwrap();
    let preparation = prepare(&project).unwrap();
    assert_eq!(preparation.status, PreparationStatus::ReadyToExport);
    assert_eq!(
        preparation.package_plan.file_plan[0].operation,
        "use_author_verified_editable_manuscript"
    );
    let package = build_package(&store, &project, &preparation.context_hash, "final").unwrap();
    assert_eq!(
        fs::read(Path::new(&package.staging_dir).join("submission/manuscript.docx")).unwrap(),
        editable_bytes
    );
    assert!(!Path::new(&package.staging_dir)
        .join("submission/manuscript.pdf")
        .exists());
    assert_eq!(fs::read(&source).unwrap(), source_bytes);
    finish_test_root(root);
}

#[test]
fn concurrent_updates_cannot_both_commit_the_same_revision() {
    let root = temporary_root("concurrent-revision");
    let source = root.join("synthetic.docx");
    write_synthetic_docx(&source);
    let store = ProjectStore::new(root.join("workspace")).unwrap();
    let project = store
        .create_from_docx(&source, TaskKind::PreparePackage)
        .unwrap();
    let barrier = Arc::new(Barrier::new(3));
    let handles = ["First title", "Second title"].map(|title| {
        let store = store.clone();
        let barrier = barrier.clone();
        let project = project.clone();
        std::thread::spawn(move || {
            let mut facts = project.facts;
            facts.title = Some(title.into());
            barrier.wait();
            store.update_facts(&project.id, project.revision, facts)
        })
    });
    barrier.wait();
    let results = handles.map(|handle| handle.join().unwrap());
    assert_eq!(results.iter().filter(|result| result.is_ok()).count(), 1);
    assert_eq!(
        results
            .iter()
            .filter_map(|result| result.as_ref().err())
            .filter(|error| error.code == "CONTEXT_CHANGED")
            .count(),
        1
    );
    assert_eq!(
        store.get(&project.id).unwrap().revision,
        project.revision + 1
    );
    finish_test_root(root);
}

fn write_simple_pdf(path: &Path) {
    let content = concat!(
        "BT\n/F1 18 Tf\n72 740 Td\n(Synthetic Local Paper) Tj\n",
        "0 -30 Td\n/F1 11 Tf\n(Abstract) Tj\n",
        "0 -18 Td\n(This study evaluates local document processing.) Tj\n",
        "0 -18 Td\n(Keywords: local; privacy; journals) Tj\nET\n"
    );
    let objects = [
        "<< /Type /Catalog /Pages 2 0 R >>".to_owned(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Resources << /Font << /F1 4 0 R >> >> /Contents 5 0 R >>".to_owned(),
        "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_owned(),
        format!("<< /Length {} >>\nstream\n{}endstream", content.len(), content),
    ];
    let mut bytes = b"%PDF-1.4\n".to_vec();
    let mut offsets = vec![0_usize];
    for (index, object) in objects.iter().enumerate() {
        offsets.push(bytes.len());
        write!(&mut bytes, "{} 0 obj\n{}\nendobj\n", index + 1, object).unwrap();
    }
    let xref = bytes.len();
    write!(&mut bytes, "xref\n0 {}\n", objects.len() + 1).unwrap();
    bytes.extend_from_slice(b"0000000000 65535 f \n");
    for offset in offsets.iter().skip(1) {
        writeln!(&mut bytes, "{offset:010} 00000 n ").unwrap();
    }
    write!(
        &mut bytes,
        "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n",
        objects.len() + 1
    )
    .unwrap();
    fs::write(path, bytes).unwrap();
}

fn temporary_root(name: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!("manuscriptdock-{name}-{}", uuid::Uuid::new_v4()));
    fs::create_dir_all(&root).unwrap();
    root
}

fn finish_test_root(root: PathBuf) {
    if std::env::var_os("MANUSCRIPTDOCK_KEEP_TEST_OUTPUT").is_some() {
        eprintln!("kept synthetic output at {}", root.display());
    } else {
        let _ = fs::remove_dir_all(root);
    }
}

fn read_docx_document_xml(path: &Path) -> String {
    let mut archive = ZipArchive::new(fs::File::open(path).unwrap()).unwrap();
    let mut xml = String::new();
    archive
        .by_name("word/document.xml")
        .unwrap()
        .read_to_string(&mut xml)
        .unwrap();
    xml
}

fn write_synthetic_docx(path: &Path) {
    write_synthetic_docx_named(path, "Synthetic Local AI Study");
}

fn write_synthetic_docx_named(path: &Path, title: &str) {
    let file = fs::File::create(path).unwrap();
    let mut writer = ZipWriter::new(file);
    let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
    let core = format!(
        r#"<?xml version="1.0"?><cp:coreProperties xmlns:cp="http://schemas.openxmlformats.org/package/2006/metadata/core-properties" xmlns:dc="http://purl.org/dc/elements/1.1/"><dc:title>{title}</dc:title></cp:coreProperties>"#
    );
    let document = format!(
        r#"<?xml version="1.0"?><w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:body><w:p><w:r><w:t>{title}</w:t></w:r></w:p><w:p><w:r><w:t>Abstract</w:t></w:r></w:p><w:p><w:r><w:t>This synthetic manuscript tests a local package.</w:t></w:r></w:p><w:p><w:r><w:t>Keywords: local AI; reproducibility</w:t></w:r></w:p></w:body></w:document>"#
    );
    let entries = [
        (
            "[Content_Types].xml",
            r#"<?xml version="1.0"?><Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"/>"#.to_owned(),
        ),
        (
            "_rels/.rels",
            r#"<?xml version="1.0"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"/>"#.to_owned(),
        ),
        ("docProps/core.xml", core),
        ("word/document.xml", document),
    ];
    for (name, contents) in entries {
        writer.start_file(name, options).unwrap();
        writer.write_all(contents.as_bytes()).unwrap();
    }
    writer.finish().unwrap();
    let digest = Sha256::digest(fs::read(path).unwrap());
    assert!(!digest.is_empty());
}
