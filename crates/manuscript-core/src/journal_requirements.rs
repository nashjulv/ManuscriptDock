use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub const JOURNAL_REQUIREMENT_SCHEMA_VERSION: u32 = 1;
pub const JOURNAL_REQUIREMENT_FRESHNESS_DAYS: u64 = 90;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum JournalRequirementSourceMode {
    OfficialNetworkFetch,
    AuthorProvidedOfficialText,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum JournalRequirementStatus {
    OfficialSourcesCaptured,
    AuthorAttestedOfficial,
    RequiresManualReview,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum JournalRequirementObligation {
    Required,
    Recommended,
    Verify,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum JournalRequirementCategory {
    ManuscriptFile,
    Template,
    AnonymousReview,
    TitlePage,
    Abstract,
    Keywords,
    LengthLimit,
    Figures,
    Tables,
    SupplementaryFiles,
    CoverLetter,
    References,
    Ethics,
    ConflictOfInterest,
    DataAvailability,
    AuthorContributions,
    Orcid,
    FeesAndOpenAccess,
    OtherSupportingFiles,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JournalRequirementSourceDocument {
    pub url: String,
    pub title: String,
    pub text: String,
    pub official_host_matched: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JournalRequirementSource {
    pub url: String,
    pub title: String,
    pub content_hash: String,
    pub captured_unix_ms: u64,
    pub official_host_matched: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JournalRequirementItem {
    pub id: String,
    pub category: JournalRequirementCategory,
    pub label: String,
    pub label_en: String,
    pub obligation: JournalRequirementObligation,
    pub detail: String,
    pub source_url: String,
    pub evidence_excerpt: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JournalRequirementSnapshot {
    pub schema_version: u32,
    pub snapshot_id: String,
    pub workspace_id: String,
    pub target_selection_id: String,
    pub journal_id: String,
    pub journal_name: String,
    pub source_mode: JournalRequirementSourceMode,
    pub status: JournalRequirementStatus,
    pub sources: Vec<JournalRequirementSource>,
    pub requirements: Vec<JournalRequirementItem>,
    pub limitations: Vec<String>,
    pub captured_unix_ms: u64,
    pub fresh_until_unix_ms: u64,
    pub record_hash: String,
    pub external_transmission: String,
}

struct RequirementPattern {
    category: JournalRequirementCategory,
    label: &'static str,
    label_en: &'static str,
    keywords: &'static [&'static str],
}

const REQUIREMENT_PATTERNS: &[RequirementPattern] = &[
    RequirementPattern {
        category: JournalRequirementCategory::ManuscriptFile,
        label: "主稿文件与格式",
        label_en: "Manuscript file and format",
        keywords: &[
            "file format",
            "manuscript file",
            ".docx",
            "latex file",
            "source files",
            "稿件格式",
            "主稿文件",
        ],
    },
    RequirementPattern {
        category: JournalRequirementCategory::Template,
        label: "期刊模板",
        label_en: "Journal template",
        keywords: &[
            "manuscript template",
            "article template",
            "download template",
            "期刊模板",
            "论文模板",
        ],
    },
    RequirementPattern {
        category: JournalRequirementCategory::AnonymousReview,
        label: "匿名审稿",
        label_en: "Anonymous review",
        keywords: &[
            "double-blind",
            "double blind",
            "double anonym",
            "blinded manuscript",
            "anonymous review",
            "匿名审稿",
            "双盲",
        ],
    },
    RequirementPattern {
        category: JournalRequirementCategory::TitlePage,
        label: "标题页",
        label_en: "Title page",
        keywords: &["title page", "separate title", "标题页"],
    },
    RequirementPattern {
        category: JournalRequirementCategory::Abstract,
        label: "摘要",
        label_en: "Abstract",
        keywords: &["abstract", "摘要"],
    },
    RequirementPattern {
        category: JournalRequirementCategory::Keywords,
        label: "关键词",
        label_en: "Keywords",
        keywords: &["keywords", "key words", "关键词"],
    },
    RequirementPattern {
        category: JournalRequirementCategory::LengthLimit,
        label: "篇幅限制",
        label_en: "Length limit",
        keywords: &[
            "word limit",
            "word count",
            "page limit",
            "maximum length",
            "words maximum",
            "words or fewer",
            "words or less",
            "characters maximum",
            "not exceed",
            "no more than",
            "篇幅",
            "字数",
            "页数",
            "字以内",
            "不超过",
        ],
    },
    RequirementPattern {
        category: JournalRequirementCategory::Figures,
        label: "图片与分辨率",
        label_en: "Figures and resolution",
        keywords: &[
            "figure",
            "artwork",
            "dpi",
            "tiff",
            "eps",
            "图片",
            "图像",
            "分辨率",
        ],
    },
    RequirementPattern {
        category: JournalRequirementCategory::Tables,
        label: "表格",
        label_en: "Tables",
        keywords: &["table", "表格"],
    },
    RequirementPattern {
        category: JournalRequirementCategory::SupplementaryFiles,
        label: "补充材料",
        label_en: "Supplementary files",
        keywords: &[
            "supplementary",
            "supporting information",
            "supplemental",
            "补充材料",
            "附加材料",
        ],
    },
    RequirementPattern {
        category: JournalRequirementCategory::CoverLetter,
        label: "投稿附信",
        label_en: "Cover letter",
        keywords: &["cover letter", "submission letter", "投稿信", "投稿附信"],
    },
    RequirementPattern {
        category: JournalRequirementCategory::References,
        label: "参考文献格式",
        label_en: "Reference style",
        keywords: &[
            "reference style",
            "references should",
            "bibliograph",
            "参考文献",
        ],
    },
    RequirementPattern {
        category: JournalRequirementCategory::Ethics,
        label: "伦理与知情同意",
        label_en: "Ethics and consent",
        keywords: &[
            "ethics approval",
            "ethical approval",
            "informed consent",
            "human subjects",
            "animal welfare",
            "伦理",
            "知情同意",
        ],
    },
    RequirementPattern {
        category: JournalRequirementCategory::ConflictOfInterest,
        label: "利益冲突声明",
        label_en: "Conflict-of-interest statement",
        keywords: &["conflict of interest", "competing interest", "利益冲突"],
    },
    RequirementPattern {
        category: JournalRequirementCategory::DataAvailability,
        label: "数据可用性声明",
        label_en: "Data-availability statement",
        keywords: &[
            "data availability",
            "data sharing",
            "research data",
            "数据可用",
            "数据共享",
        ],
    },
    RequirementPattern {
        category: JournalRequirementCategory::AuthorContributions,
        label: "作者贡献声明",
        label_en: "Author-contribution statement",
        keywords: &[
            "author contribution",
            "credit taxonomy",
            "crédit taxonomy",
            "作者贡献",
        ],
    },
    RequirementPattern {
        category: JournalRequirementCategory::Orcid,
        label: "ORCID",
        label_en: "ORCID",
        keywords: &["orcid"],
    },
    RequirementPattern {
        category: JournalRequirementCategory::FeesAndOpenAccess,
        label: "费用与开放获取",
        label_en: "Fees and open access",
        keywords: &[
            "article processing charge",
            "publication fee",
            "open access fee",
            "apc",
            "开放获取",
            "版面费",
            "发表费",
        ],
    },
    RequirementPattern {
        category: JournalRequirementCategory::OtherSupportingFiles,
        label: "其他支持文件",
        label_en: "Other supporting files",
        keywords: &[
            "supporting document",
            "additional file",
            "reporting checklist",
            "permission form",
            "copyright form",
            "author agreement",
            "其他文件",
            "支持文件",
            "报告清单",
            "授权文件",
            "版权协议",
            "作者协议",
        ],
    },
];

pub fn extract_journal_requirements(
    documents: &[JournalRequirementSourceDocument],
    captured_unix_ms: u64,
) -> (Vec<JournalRequirementSource>, Vec<JournalRequirementItem>) {
    let sources = documents
        .iter()
        .map(|document| JournalRequirementSource {
            url: document.url.clone(),
            title: document.title.clone(),
            content_hash: hex::encode(Sha256::digest(document.text.as_bytes())),
            captured_unix_ms,
            official_host_matched: document.official_host_matched,
        })
        .collect::<Vec<_>>();
    let mut requirements = Vec::new();
    for pattern in REQUIREMENT_PATTERNS {
        let matches = documents
            .iter()
            .flat_map(|document| {
                find_evidence_excerpts(&document.text, pattern.keywords)
                    .into_iter()
                    .map(move |excerpt| (document, excerpt))
            })
            .take(
                if pattern.category == JournalRequirementCategory::LengthLimit {
                    12
                } else {
                    1
                },
            )
            .collect::<Vec<_>>();
        for (index, (document, excerpt)) in matches.into_iter().enumerate() {
            let obligation = detect_obligation(&excerpt);
            let (label, label_en) = if pattern.category == JournalRequirementCategory::LengthLimit {
                length_limit_label(&excerpt)
            } else {
                (pattern.label.to_owned(), pattern.label_en.to_owned())
            };
            requirements.push(JournalRequirementItem {
                id: format!(
                    "requirement-{}{}",
                    category_slug(pattern.category),
                    if index == 0 {
                        String::new()
                    } else {
                        format!("-{}", index + 1)
                    }
                ),
                category: pattern.category,
                label,
                label_en,
                obligation,
                detail: obligation_detail(obligation).to_owned(),
                source_url: document.url.clone(),
                evidence_excerpt: excerpt,
            });
        }
    }
    (sources, requirements)
}

fn find_evidence_excerpts(text: &str, keywords: &[&str]) -> Vec<String> {
    let normalized = text.split_whitespace().collect::<Vec<_>>().join(" ");
    normalized
        .split(['.', '!', '?', '。', '！', '？', ';', '；'])
        .map(str::trim)
        .filter(|sentence| sentence.chars().count() >= 12)
        .filter(|sentence| {
            let lowercase = sentence.to_lowercase();
            keywords
                .iter()
                .any(|keyword| keyword_matches(&lowercase, keyword))
        })
        .map(|sentence| truncate_chars(sentence, 360))
        .collect()
}

fn length_limit_label(excerpt: &str) -> (String, String) {
    let lowercase = excerpt.to_lowercase();
    for (needles, zh, en) in [
        (
            &["abstract", "摘要"][..],
            "摘要篇幅限制",
            "Abstract length limit",
        ),
        (&["title", "标题"][..], "标题篇幅限制", "Title length limit"),
        (
            &["introduction", "引言", "绪论"][..],
            "引言篇幅限制",
            "Introduction length limit",
        ),
        (
            &["method", "方法"][..],
            "方法篇幅限制",
            "Methods length limit",
        ),
        (
            &["result", "结果"][..],
            "结果篇幅限制",
            "Results length limit",
        ),
        (
            &["discussion", "讨论"][..],
            "讨论篇幅限制",
            "Discussion length limit",
        ),
        (
            &["conclusion", "结论"][..],
            "结论篇幅限制",
            "Conclusion length limit",
        ),
        (
            &["main text", "full text", "正文"][..],
            "正文篇幅限制",
            "Main-text length limit",
        ),
    ] {
        if needles.iter().any(|needle| lowercase.contains(needle)) {
            return (zh.to_owned(), en.to_owned());
        }
    }
    ("篇幅限制".to_owned(), "Length limit".to_owned())
}

fn keyword_matches(value: &str, keyword: &str) -> bool {
    if !keyword
        .chars()
        .all(|character| character.is_ascii_alphabetic())
    {
        return value.contains(keyword);
    }
    value.match_indices(keyword).any(|(index, _)| {
        let before = value[..index].chars().next_back();
        let after = value[index + keyword.len()..].chars().next();
        before.is_none_or(|character| !character.is_ascii_alphanumeric())
            && after.is_none_or(|character| !character.is_ascii_alphanumeric())
    })
}

fn detect_obligation(excerpt: &str) -> JournalRequirementObligation {
    let lowercase = excerpt.to_lowercase();
    if [
        "must",
        "required",
        "mandatory",
        "shall",
        "need to",
        "必须",
        "应当",
        "须提供",
    ]
    .iter()
    .any(|marker| lowercase.contains(marker))
    {
        JournalRequirementObligation::Required
    } else if [
        "recommended",
        "encouraged",
        "optional",
        "建议",
        "鼓励",
        "可选",
    ]
    .iter()
    .any(|marker| lowercase.contains(marker))
    {
        JournalRequirementObligation::Recommended
    } else {
        JournalRequirementObligation::Verify
    }
}

fn obligation_detail(obligation: JournalRequirementObligation) -> &'static str {
    match obligation {
        JournalRequirementObligation::Required => "官方原文含明确义务词；提交前仍需由作者逐项核对",
        JournalRequirementObligation::Recommended => "官方原文将其表述为建议或可选项",
        JournalRequirementObligation::Verify => "已发现相关说明，但义务强度需要作者确认",
    }
}

fn category_slug(category: JournalRequirementCategory) -> &'static str {
    match category {
        JournalRequirementCategory::ManuscriptFile => "manuscript-file",
        JournalRequirementCategory::Template => "template",
        JournalRequirementCategory::AnonymousReview => "anonymous-review",
        JournalRequirementCategory::TitlePage => "title-page",
        JournalRequirementCategory::Abstract => "abstract",
        JournalRequirementCategory::Keywords => "keywords",
        JournalRequirementCategory::LengthLimit => "length-limit",
        JournalRequirementCategory::Figures => "figures",
        JournalRequirementCategory::Tables => "tables",
        JournalRequirementCategory::SupplementaryFiles => "supplementary-files",
        JournalRequirementCategory::CoverLetter => "cover-letter",
        JournalRequirementCategory::References => "references",
        JournalRequirementCategory::Ethics => "ethics",
        JournalRequirementCategory::ConflictOfInterest => "conflict-of-interest",
        JournalRequirementCategory::DataAvailability => "data-availability",
        JournalRequirementCategory::AuthorContributions => "author-contributions",
        JournalRequirementCategory::Orcid => "orcid",
        JournalRequirementCategory::FeesAndOpenAccess => "fees-open-access",
        JournalRequirementCategory::OtherSupportingFiles => "other-supporting-files",
    }
}

fn truncate_chars(value: &str, limit: usize) -> String {
    let mut output = value.chars().take(limit).collect::<String>();
    if value.chars().count() > limit {
        output.push('…');
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_source_backed_requirements_and_obligations() {
        let documents = vec![JournalRequirementSourceDocument {
            url: "https://journal.example/guide-for-authors".to_owned(),
            title: "Guide for authors".to_owned(),
            text: "A separate title page is required. Figures should be supplied at 300 dpi. Authors are encouraged to provide an ORCID.".to_owned(),
            official_host_matched: true,
        }];
        let (sources, requirements) = extract_journal_requirements(&documents, 1_000);
        assert_eq!(sources.len(), 1);
        assert_eq!(requirements.len(), 3);
        assert_eq!(
            requirements[0].obligation,
            JournalRequirementObligation::Required
        );
        assert!(requirements
            .iter()
            .all(|item| !item.evidence_excerpt.is_empty()));
    }

    #[test]
    fn does_not_invent_requirements_without_matching_evidence() {
        let documents = vec![JournalRequirementSourceDocument {
            url: "https://journal.example".to_owned(),
            title: "Journal home".to_owned(),
            text: "Welcome to the journal home page.".to_owned(),
            official_host_matched: true,
        }];
        let (_, requirements) = extract_journal_requirements(&documents, 1_000);
        assert!(requirements.is_empty());
    }

    #[test]
    fn keeps_section_length_limits_and_supporting_files_as_separate_evidence() {
        let documents = vec![JournalRequirementSourceDocument {
            url: "https://journal.example/instructions".to_owned(),
            title: "Instructions for authors".to_owned(),
            text: "The abstract must not exceed 250 words. The main text must be no more than 5,000 words. Authors must upload the completed reporting checklist as a supporting document.".to_owned(),
            official_host_matched: true,
        }];
        let (_, requirements) = extract_journal_requirements(&documents, 1_000);
        let length_limits = requirements
            .iter()
            .filter(|item| item.category == JournalRequirementCategory::LengthLimit)
            .collect::<Vec<_>>();
        assert_eq!(length_limits.len(), 2);
        assert!(length_limits
            .iter()
            .any(|item| item.label_en == "Abstract length limit"));
        assert!(length_limits
            .iter()
            .any(|item| item.label_en == "Main-text length limit"));
        assert!(requirements.iter().any(|item| {
            item.category == JournalRequirementCategory::OtherSupportingFiles
                && item.evidence_excerpt.contains("reporting checklist")
                && item.obligation == JournalRequirementObligation::Required
        }));
    }
}
