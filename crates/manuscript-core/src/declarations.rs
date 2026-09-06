use serde::{Deserialize, Serialize};

use crate::{JournalRequirementCategory, JournalRequirementItem};

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SubmissionStage {
    #[default]
    Initial,
    Revision,
    Accepted,
    Unknown,
}

impl SubmissionStage {
    pub fn is_due(self, current: Self) -> bool {
        let rank = |stage| match stage {
            Self::Initial => 0,
            Self::Revision => 1,
            Self::Accepted => 2,
            Self::Unknown => 0,
        };
        rank(self) <= rank(current)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeclarationDelivery {
    Manuscript,
    Attachment,
    SubmissionSystem,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeclarationApplicability {
    Applicable,
    NeedsReview,
    NotApplicable,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeclarationRequirement {
    pub delivery: Vec<DeclarationDelivery>,
    pub stage: SubmissionStage,
    pub condition: Option<String>,
    pub applicability: DeclarationApplicability,
    pub file_count: usize,
    pub allowed_extensions: Vec<String>,
    pub signature_required: bool,
    pub stamp_required: bool,
    pub shared_file_allowed: bool,
    pub template_url: Option<String>,
    pub author_note: Option<String>,
}

pub fn is_declaration_category(category: JournalRequirementCategory) -> bool {
    matches!(
        category,
        JournalRequirementCategory::Ethics
            | JournalRequirementCategory::ConflictOfInterest
            | JournalRequirementCategory::DataAvailability
            | JournalRequirementCategory::AuthorContributions
            | JournalRequirementCategory::OtherSupportingFiles
    )
}

pub fn declaration_material_kind(
    category: JournalRequirementCategory,
) -> crate::SubmissionMaterialKind {
    if category == JournalRequirementCategory::OtherSupportingFiles {
        crate::SubmissionMaterialKind::Other
    } else {
        crate::SubmissionMaterialKind::Declaration
    }
}

pub fn declaration_requirement(item: &JournalRequirementItem) -> Option<DeclarationRequirement> {
    if !is_declaration_category(item.category) {
        return None;
    }
    Some(
        item.declaration
            .clone()
            .unwrap_or_else(|| infer_declaration(&item.evidence_excerpt)),
    )
}

pub fn expand_declaration_items(item: JournalRequirementItem) -> Vec<JournalRequirementItem> {
    let value = item.evidence_excerpt.to_lowercase();
    let names: &[(&str, &str, &[&str])] = match item.category {
        JournalRequirementCategory::OtherSupportingFiles => &[
            (
                "投稿声明",
                "Submission declaration",
                &["投稿声明", "submission declaration"],
            ),
            (
                "不涉密证明",
                "Non-confidentiality certificate",
                &["不涉密证明"],
            ),
            (
                "版权转让协议",
                "Copyright transfer agreement",
                &[
                    "著作权转让",
                    "版权协议",
                    "copyright form",
                    "copyright transfer",
                ],
            ),
            (
                "作者协议",
                "Author agreement",
                &["作者协议", "author agreement"],
            ),
            (
                "资金声明",
                "Funding statement",
                &["资金声明", "funding statement", "funding declaration"],
            ),
            (
                "AI 使用声明",
                "AI-use declaration",
                &[
                    "ai 使用声明",
                    "ai使用声明",
                    "ai use",
                    "use of ai",
                    "generative ai",
                ],
            ),
            (
                "报告清单",
                "Reporting checklist",
                &["报告清单", "reporting checklist"],
            ),
            (
                "授权文件",
                "Permission form",
                &["授权文件", "permission form"],
            ),
        ],
        JournalRequirementCategory::Ethics => &[
            (
                "知情同意声明",
                "Informed consent statement",
                &["知情同意", "informed consent"],
            ),
            (
                "伦理批准文件",
                "Ethics approval",
                &["伦理", "ethics approval", "ethical approval"],
            ),
        ],
        _ => &[],
    };
    let matches = names
        .iter()
        .filter(|(_, _, words)| words.iter().any(|word| value.contains(word)))
        .collect::<Vec<_>>();
    if matches.is_empty() {
        return vec![item];
    }
    matches
        .iter()
        .enumerate()
        .map(|(index, (label, label_en, _))| {
            let mut expanded = item.clone();
            expanded.id = format!("{}-document-{}", item.id, index + 1);
            expanded.label = (*label).into();
            expanded.label_en = (*label_en).into();
            if matches.len() > 1 {
                if let Some(declaration) = expanded.declaration.as_mut() {
                    declaration.file_count = 1;
                    // A sentence naming several documents may assign different formats or actions.
                    // Retain its evidence and require review instead of distributing guesses.
                    declaration.applicability = DeclarationApplicability::NeedsReview;
                }
            }
            expanded
        })
        .collect()
}

pub fn infer_declaration(evidence: &str) -> DeclarationRequirement {
    let value = evidence.to_lowercase();
    let has = |words: &[&str]| words.iter().any(|word| value.contains(word));
    let mut delivery = Vec::new();
    if has(&[
        "in the manuscript",
        "in your manuscript",
        "in the article",
        "正文",
        "文中",
        "文内",
    ]) {
        delivery.push(DeclarationDelivery::Manuscript);
    }
    if has(&[
        "separate file",
        "separate document",
        "attachment",
        "upload",
        "signed",
        "scan",
        "独立文件",
        "单独提交",
        "附件",
        "上传",
        "扫描",
        "签字",
        "签署",
        "盖章",
    ]) && !has(&[
        "no separate",
        "do not upload",
        "无需上传",
        "不需上传",
        "不必上传",
    ]) {
        delivery.push(DeclarationDelivery::Attachment);
    }
    if has(&[
        "complete in the submission system",
        "submission system field",
        "online form",
        "系统填写",
        "系统中填写",
        "在线填写",
    ]) {
        delivery.push(DeclarationDelivery::SubmissionSystem);
    }
    if delivery.is_empty() {
        delivery.push(DeclarationDelivery::Unknown);
    }
    let stage = if has(&[
        "after acceptance",
        "upon acceptance",
        "accepted manuscript",
        "录用后",
        "接收后",
        "评审通过后",
    ]) {
        SubmissionStage::Accepted
    } else if has(&[
        "at revision",
        "revised submission",
        "with the revision",
        "返修时",
        "修回时",
        "返修阶段",
    ]) {
        SubmissionStage::Revision
    } else if has(&[
        "initial submission",
        "at submission",
        "when submitting",
        "投稿时",
        "初次投稿",
        "初投",
    ]) {
        SubmissionStage::Initial
    } else {
        SubmissionStage::Unknown
    };
    let conditional = has(&[
        "if applicable",
        "where applicable",
        "if the study",
        "studies involving",
        "research involving",
        "如适用",
        "若涉及",
        "如涉及",
        "涉及人体",
        "涉及动物",
        "适用时",
    ]);
    // Negation or alternatives need author review rather than an invented delivery rule.
    let uncertain = has(&[
        "either ",
        " or ",
        "或者",
        "或另",
        "无需",
        "不必",
        "not required",
        "do not",
    ]);
    let subject_groups: &[&[&str]] = &[
        &["conflict of interest", "competing interest", "利益冲突"],
        &["data availability", "数据可用"],
        &["author contribution", "作者贡献"],
        &["ethics approval", "伦理"],
        &["informed consent", "知情同意"],
    ];
    let multiple_subjects = subject_groups.iter().filter(|words| has(words)).count() > 1;
    if uncertain || (multiple_subjects && delivery.len() > 1) {
        delivery = vec![DeclarationDelivery::Unknown];
    }
    let extensions = [
        "pdf", "docx", "doc", "odt", "rtf", "tex", "txt", "jpg", "jpeg", "png", "tif", "tiff",
    ];
    let mut allowed_extensions = Vec::new();
    if has(&["format", "格式", "only", "仅接受"]) {
        for extension in extensions {
            if value
                .split(|c: char| !c.is_ascii_alphanumeric())
                .any(|word| word == extension)
            {
                allowed_extensions.push(extension.to_owned());
            }
        }
    }
    let file_count = if has(&[
        "two separate files",
        "two separate documents",
        "两个独立文件",
        "两份独立文件",
    ]) {
        2
    } else if has(&[
        "three separate files",
        "three separate documents",
        "三个独立文件",
        "三份独立文件",
    ]) {
        3
    } else {
        1
    };
    DeclarationRequirement {
        delivery,
        stage,
        condition: conditional.then(|| evidence.to_owned()),
        applicability: if conditional {
            DeclarationApplicability::NeedsReview
        } else {
            DeclarationApplicability::Applicable
        },
        file_count,
        allowed_extensions,
        signature_required: has(&["signed", "signature", "签字", "签署", "签名"]),
        stamp_required: has(&["official stamp", "institutional stamp", "盖章"]),
        shared_file_allowed: has(&[
            "combined file",
            "combined document",
            "single combined",
            "合并为一个文件",
            "合并提交",
        ]),
        template_url: None,
        author_note: None,
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeclarationPlan {
    pub manuscript_version: u32,
    pub target_selection_id: String,
    pub requirement_snapshot_id: String,
    pub stage: SubmissionStage,
    pub requirements: Vec<JournalRequirementItem>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeclarationPlanUpdate {
    pub manuscript_version: u32,
    pub target_selection_id: String,
    pub requirement_snapshot_id: String,
    pub requirement: Option<JournalRequirementItem>,
    pub stage: Option<SubmissionStage>,
    pub material_id: Option<String>,
    pub checklist_item_id: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn separates_delivery_stage_conditions_and_formats_in_both_languages() {
        for text in [
            "At submission authors must include a conflict of interest statement in the manuscript and upload a signed attachment in PDF format.",
            "投稿时必须在正文提供利益冲突声明并上传签字附件，格式为 PDF。",
        ] {
            let requirement = infer_declaration(text);
            assert_eq!(requirement.delivery, vec![DeclarationDelivery::Manuscript, DeclarationDelivery::Attachment]);
            assert_eq!(requirement.stage, SubmissionStage::Initial);
            assert!(requirement.signature_required);
            assert_eq!(requirement.allowed_extensions, vec!["pdf"]);
        }
        let later = infer_declaration("After acceptance, studies involving human subjects must upload two separate files bearing an institutional stamp.");
        assert_eq!(later.stage, SubmissionStage::Accepted);
        assert!(!later.stage.is_due(SubmissionStage::Initial));
        assert_eq!(later.applicability, DeclarationApplicability::NeedsReview);
        assert_eq!(later.file_count, 2);
        assert!(later.stamp_required);
        assert_eq!(
            infer_declaration("Authors must provide a data availability statement.").delivery,
            vec![DeclarationDelivery::Unknown]
        );
        assert_eq!(infer_declaration("Authors must either include a statement in the manuscript or upload a separate file.").delivery, vec![DeclarationDelivery::Unknown]);
    }
}
