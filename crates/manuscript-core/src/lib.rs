mod dialogue;
mod journal_match;
mod knowledge;
mod readiness;
mod revision;
mod structure;
mod workspace;

pub use dialogue::{
    KnowledgeAnswerRecord, KnowledgeDialogueItem, KnowledgeDialogueLedger, KnowledgeInquiryOrigin,
    KnowledgeInquiryRecord, KnowledgeInquiryStance, KnowledgeInquiryTarget,
    KNOWLEDGE_DIALOGUE_SCHEMA_VERSION,
};
pub use journal_match::{
    recommend_journals, ArticleTypePreference, JournalFitScores, JournalMatchPreferences,
    JournalRecommendation, JournalRecommendationRun, JournalRegion, OpenAccessPreference,
    PublicationLanguagePreference, ResearchTopic, TargetStrategy, JOURNAL_CATALOG_VERSION,
    JOURNAL_MATCH_ALGORITHM_VERSION, JOURNAL_MATCH_SCHEMA_VERSION,
};
pub use knowledge::{
    discipline_catalog, local_knowledge_body_snapshot, AcademicKnowledgeBodySnapshot,
    AiReviewReportHistory, AiReviewReportVersion, AiReviewStatus, AssertionBasis, AssertionStatus,
    ClaimElementReference, ClaimFiveTuple, DisciplineCatalogItem, DisciplineClassification,
    ElementState, KnowledgeBodyError, KnowledgeBodyNetwork, KnowledgeBodyNode,
    KnowledgeBodyObjectSet, KnowledgeBodyRole, KnowledgeObjectType, NetworkAssertion, RelationKind,
    RelationProtocol, VersionedObjectReference, DISCIPLINE_INDEX_SCHEME, DISCIPLINE_INDEX_VERSION,
    KNOWLEDGE_BODY_SCHEMA_VERSION,
};
pub use readiness::{
    bundled_rule_pack_catalog, bundled_submission_element_catalog, ExternalTransmission,
    FindingStatus, ReadinessError, ReadinessEvaluation, ReadinessOutcome, ReadinessReport,
    RuleClassification, RuleFinding, RulePackCatalog, RulePackCatalogItem, RulePackReference,
    SubmissionElementCatalog, SubmissionElementCatalogItem, SubmissionElementRequirement,
};
pub use revision::{
    RevisionApplication, RevisionChange, RevisionChangeInput, RevisionDraft, RevisionError,
    RevisionField, RevisionFieldKind, RevisionSet,
};
pub use structure::{
    AnalysisQuality, SectionSummary, StructureAnalysis, StructureError, StructureReport,
};
pub use workspace::{
    KnowledgeBodyRecord, LocalAttestation, ManuscriptVersionSummary, SubmissionExport,
    SubmissionRecord, VersionComparison, VersionCreation, VersionHistory, VersionOrigin,
    WorkspaceCatalog, WorkspaceCreation, WorkspaceError, WorkspaceLifecycle, WorkspaceStore,
    WorkspaceSummary,
};

use serde::{Deserialize, Serialize};
use std::{error::Error, fmt, path::Path, time::UNIX_EPOCH};

pub const MAX_MANUSCRIPT_SIZE_BYTES: u64 = 250 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ManuscriptKind {
    Word,
    Pdf,
    Latex,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ManuscriptSummary {
    pub name: String,
    pub extension: String,
    pub kind: ManuscriptKind,
    pub size_bytes: u64,
    pub modified_unix_ms: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum ManuscriptSelection {
    Selected {
        #[serde(rename = "selectionId")]
        selection_id: String,
        manuscript: ManuscriptSummary,
    },
    Cancelled,
    Rejected {
        message: String,
    },
}

#[derive(Debug)]
pub enum ManuscriptError {
    Io(std::io::Error),
    MissingFileName,
    NotAFile,
    UnsupportedFormat,
    FileTooLarge { size_bytes: u64, limit_bytes: u64 },
}

impl fmt::Display for ManuscriptError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(formatter, "无法读取所选文件：{error}"),
            Self::MissingFileName => write!(formatter, "所选文件没有可显示的文件名"),
            Self::NotAFile => write!(formatter, "请选择一个论文文件，而不是文件夹"),
            Self::UnsupportedFormat => write!(formatter, "当前仅支持 DOCX、PDF 和 TEX 格式"),
            Self::FileTooLarge {
                size_bytes,
                limit_bytes,
            } => write!(
                formatter,
                "文件大小为 {size_bytes} 字节，超过 {limit_bytes} 字节的本地处理上限"
            ),
        }
    }
}

impl Error for ManuscriptError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            _ => None,
        }
    }
}

impl From<std::io::Error> for ManuscriptError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}

pub fn inspect_manuscript(path: &Path) -> Result<ManuscriptSummary, ManuscriptError> {
    let metadata = path.metadata()?;
    if !metadata.is_file() {
        return Err(ManuscriptError::NotAFile);
    }

    if metadata.len() > MAX_MANUSCRIPT_SIZE_BYTES {
        return Err(ManuscriptError::FileTooLarge {
            size_bytes: metadata.len(),
            limit_bytes: MAX_MANUSCRIPT_SIZE_BYTES,
        });
    }

    let (extension, kind) = classify_path(path).ok_or(ManuscriptError::UnsupportedFormat)?;
    let name = path
        .file_name()
        .and_then(|file_name| file_name.to_str())
        .ok_or(ManuscriptError::MissingFileName)?
        .to_owned();
    let modified_unix_ms = metadata
        .modified()
        .ok()
        .and_then(|modified| modified.duration_since(UNIX_EPOCH).ok())
        .and_then(|duration| u64::try_from(duration.as_millis()).ok());

    Ok(ManuscriptSummary {
        name,
        extension,
        kind,
        size_bytes: metadata.len(),
        modified_unix_ms,
    })
}

fn classify_path(path: &Path) -> Option<(String, ManuscriptKind)> {
    let extension = path.extension()?.to_str()?.to_ascii_lowercase();
    let kind = match extension.as_str() {
        "docx" => ManuscriptKind::Word,
        "pdf" => ManuscriptKind::Pdf,
        "tex" => ManuscriptKind::Latex,
        _ => return None,
    };

    Some((extension, kind))
}

#[cfg(test)]
mod tests {
    use super::{
        classify_path, inspect_manuscript, ManuscriptError, ManuscriptKind, ManuscriptSelection,
        ManuscriptSummary, MAX_MANUSCRIPT_SIZE_BYTES,
    };
    use std::{
        fs::{self, File},
        io::Write,
        path::{Path, PathBuf},
        time::{SystemTime, UNIX_EPOCH},
    };

    struct SyntheticFile(PathBuf);

    impl SyntheticFile {
        fn create(extension: &str) -> Self {
            let nonce = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system time should follow the Unix epoch")
                .as_nanos();
            let path = std::env::temp_dir().join(format!(
                "manuscriptdock-synthetic-{}-{nonce}.{extension}",
                std::process::id()
            ));
            Self(path)
        }

        fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for SyntheticFile {
        fn drop(&mut self) {
            let _ = fs::remove_file(&self.0);
        }
    }

    #[test]
    fn classifies_supported_formats_case_insensitively() {
        assert_eq!(
            classify_path(Path::new("paper.DOCX")),
            Some(("docx".to_owned(), ManuscriptKind::Word))
        );
        assert_eq!(
            classify_path(Path::new("paper.pdf")),
            Some(("pdf".to_owned(), ManuscriptKind::Pdf))
        );
        assert_eq!(
            classify_path(Path::new("paper.TeX")),
            Some(("tex".to_owned(), ManuscriptKind::Latex))
        );
    }

    #[test]
    fn rejects_unknown_or_missing_extensions() {
        assert_eq!(classify_path(Path::new("paper.doc")), None);
        assert_eq!(classify_path(Path::new("manuscript")), None);
    }

    #[test]
    fn inspects_a_synthetic_file_without_exposing_its_path() {
        let file = SyntheticFile::create("docx");
        let mut handle = File::create(file.path()).expect("synthetic file should be created");
        handle
            .write_all(b"synthetic manuscript fixture")
            .expect("synthetic fixture should be written");

        let summary = inspect_manuscript(file.path()).expect("synthetic DOCX should be accepted");

        assert!(summary.name.starts_with("manuscriptdock-synthetic-"));
        assert_eq!(summary.extension, "docx");
        assert_eq!(summary.kind, ManuscriptKind::Word);
        assert_eq!(summary.size_bytes, 28);
    }

    #[test]
    fn rejects_a_synthetic_file_over_the_local_limit() {
        let file = SyntheticFile::create("pdf");
        let handle = File::create(file.path()).expect("synthetic file should be created");
        handle
            .set_len(MAX_MANUSCRIPT_SIZE_BYTES + 1)
            .expect("synthetic sparse file should be resized");

        let error = inspect_manuscript(file.path()).expect_err("oversized file should be rejected");
        assert!(matches!(
            error,
            ManuscriptError::FileTooLarge {
                size_bytes,
                limit_bytes: MAX_MANUSCRIPT_SIZE_BYTES
            } if size_bytes == MAX_MANUSCRIPT_SIZE_BYTES + 1
        ));
    }

    #[test]
    fn selection_contract_uses_tagged_status_and_safe_camel_case_metadata() {
        let selection = ManuscriptSelection::Selected {
            selection_id: "synthetic-selection".to_owned(),
            manuscript: ManuscriptSummary {
                name: "synthetic-study.tex".to_owned(),
                extension: "tex".to_owned(),
                kind: ManuscriptKind::Latex,
                size_bytes: 42,
                modified_unix_ms: Some(1_777_777_777_000),
            },
        };

        let value = serde_json::to_value(selection).expect("selection should serialize");
        assert_eq!(value["status"], "selected");
        assert_eq!(value["selectionId"], "synthetic-selection");
        assert_eq!(value["manuscript"]["sizeBytes"], 42);
        assert_eq!(value["manuscript"]["modifiedUnixMs"], 1_777_777_777_000_u64);
        assert!(value["manuscript"].get("path").is_none());
    }
}
