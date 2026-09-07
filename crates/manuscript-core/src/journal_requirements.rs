use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub const JOURNAL_REQUIREMENT_SCHEMA_VERSION: u32 = 3;
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
    SubmissionDeadline,
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
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub additional_evidence: Vec<RequirementEvidence>,
    pub id: String,
    pub category: JournalRequirementCategory,
    pub label: String,
    pub label_en: String,
    pub obligation: JournalRequirementObligation,
    pub detail: String,
    pub source_url: String,
    pub evidence_excerpt: String,
    // Omission preserves the serialized hash of pre-structured immutable snapshots.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub declaration: Option<crate::DeclarationRequirement>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RequirementEvidence {
    pub source_url: String,
    pub excerpt: String,
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
        category: JournalRequirementCategory::SubmissionDeadline,
        label: "注册与提交时限",
        label_en: "Registration and submission deadline",
        keywords: &[
            "天内",
            "小时内",
            "不超过",
            "deadline",
            "within",
            "no more than",
            "not exceed",
        ],
    },
    RequirementPattern {
        category: JournalRequirementCategory::ManuscriptFile,
        label: "主稿文件与格式",
        label_en: "Manuscript file and format",
        keywords: &[
            "file format",
            "manuscript file",
            "manuscript language",
            ".docx",
            "latex file",
            "source files",
            "稿件格式",
            "主稿文件",
            "中文稿",
            "英文稿",
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
            "figures",
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
        keywords: &["table", "tables", "表格"],
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
            "投稿声明",
            "不涉密证明",
            "著作权转让",
            "修改说明",
            "作者协议",
            "submission declaration",
            "copyright transfer",
            "funding statement",
            "funding declaration",
            "资金声明",
            "ai 使用声明",
            "ai使用声明",
            "ai use",
            "use of ai",
            "generative ai",
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
                    .filter(move |excerpt| match pattern.category {
                        JournalRequirementCategory::LengthLimit => is_length_constraint(excerpt),
                        JournalRequirementCategory::SubmissionDeadline => {
                            is_submission_deadline(excerpt)
                        }
                        _ => true,
                    })
                    .map(move |excerpt| (document, excerpt))
            })
            .take(match pattern.category {
                JournalRequirementCategory::LengthLimit
                | JournalRequirementCategory::SubmissionDeadline => 12,
                category if crate::declarations::is_declaration_category(category) => 32,
                _ => 1,
            })
            .collect::<Vec<_>>();
        for (index, (document, excerpt)) in matches.into_iter().enumerate() {
            let obligation = detect_obligation(&excerpt);
            let (label, label_en) = if pattern.category == JournalRequirementCategory::LengthLimit {
                length_limit_label(&excerpt)
            } else {
                (pattern.label.to_owned(), pattern.label_en.to_owned())
            };
            let mut item = JournalRequirementItem {
                additional_evidence: Vec::new(),
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
                declaration: None,
            };
            item.declaration = crate::declarations::declaration_requirement(&item);
            if let Some(declaration) = item.declaration.as_mut() {
                declaration.template_url =
                    explicit_template_url(&document.text, &item.evidence_excerpt);
            }
            for item in crate::declarations::expand_declaration_items(item) {
                if let Some(previous) =
                    requirements
                        .iter_mut()
                        .find(|previous: &&mut JournalRequirementItem| {
                            previous.category == item.category
                                && previous.label == item.label
                                && semantic_requirement_key(previous)
                                    == semantic_requirement_key(&item)
                        })
                {
                    if previous.evidence_excerpt != item.evidence_excerpt
                        || previous.source_url != item.source_url
                    {
                        previous.additional_evidence.push(RequirementEvidence {
                            source_url: item.source_url.clone(),
                            excerpt: item.evidence_excerpt.clone(),
                        });
                    }
                    continue;
                }
                if !requirements
                    .iter()
                    .any(|previous: &JournalRequirementItem| {
                        previous.category == item.category
                            && previous.label == item.label
                            && previous.evidence_excerpt == item.evidence_excerpt
                    })
                {
                    requirements.push(item);
                }
            }
        }
    }
    (sources, requirements)
}

#[cfg(test)]
mod remediation_tests {
    use super::*;
    #[test]
    fn deadlines_are_not_length_limits_and_duplicate_deadlines_keep_both_sources() {
        let docs = vec![JournalRequirementSourceDocument { url: "https://example.test/guide".into(), title: "Synthetic".into(), official_host_matched: true, text: "在注册稿件编号后最迟不超过1天内提交论文。网上注册登记与稿件提交应同步，时间相差不超过1天。论文篇幅有基本要求，但请注意并非越长越好。摘要字数必须不超过200个字。".into() }];
        let (_, items) = extract_journal_requirements(&docs, 1);
        let deadlines: Vec<_> = items
            .iter()
            .filter(|item| item.category == JournalRequirementCategory::SubmissionDeadline)
            .collect();
        assert_eq!(deadlines.len(), 1);
        assert_eq!(deadlines[0].additional_evidence.len(), 1);
        assert!(items
            .iter()
            .filter(|item| item.category == JournalRequirementCategory::LengthLimit)
            .all(|item| !item.evidence_excerpt.contains("1天")));
        assert!(items.iter().any(|item| item.label == "摘要篇幅限制"));
    }
    #[test]
    fn english_time_and_page_constraints_remain_separate() {
        let docs = vec![JournalRequirementSourceDocument { url: "https://example.test/guide".into(), title: "Synthetic".into(), official_host_matched: true, text: "Authors must submit within 2 days of registration. The manuscript must not exceed 12 pages. Abstract word count must not exceed 250 words.".into() }];
        let (_, items) = extract_journal_requirements(&docs, 1);
        assert_eq!(
            items
                .iter()
                .filter(|item| item.category == JournalRequirementCategory::SubmissionDeadline)
                .count(),
            1
        );
        assert_eq!(
            items
                .iter()
                .filter(|item| item.category == JournalRequirementCategory::LengthLimit)
                .count(),
            2
        );
    }
}

fn is_submission_deadline(text: &str) -> bool {
    let text = text.to_lowercase();
    [
        "提交",
        "注册",
        "submission",
        "submit",
        "register",
        "registration",
    ]
    .iter()
    .any(|v| text.contains(v))
        && ["天", "小时", " day", " hour", "deadline"]
            .iter()
            .any(|v| text.contains(v))
}

fn is_length_constraint(text: &str) -> bool {
    let text = text.to_lowercase();
    [
        "篇幅",
        "字数",
        "页数",
        "个字",
        "字以内",
        " word",
        "character",
        " page",
        "maximum length",
    ]
    .iter()
    .any(|v| text.contains(v))
        || (text.contains('页') && text.chars().any(|v| v.is_ascii_digit()))
}

fn semantic_requirement_key(item: &JournalRequirementItem) -> String {
    if item.category == JournalRequirementCategory::SubmissionDeadline {
        let text = &item.evidence_excerpt;
        // Only merge the known semantic form: registration -> submission, same upper bound.
        if (text.contains("注册") || text.to_lowercase().contains("registration"))
            && (text.contains("提交") || text.to_lowercase().contains("submission"))
            && (text.contains("不超过") || text.to_lowercase().contains("within"))
        {
            let bound = text.split("不超过").nth(1).unwrap_or(text).trim();
            let number: String = bound
                .chars()
                .skip_while(|c| !c.is_ascii_digit())
                .take_while(|c| c.is_ascii_digit() || *c == '.')
                .collect();
            let unit = if text.contains("小时") || text.contains("hour") {
                "hours"
            } else {
                "days"
            };
            if !number.is_empty() {
                return format!(
                    "registration-to-submission:maximum:{number}:{unit}:{:?}",
                    item.obligation
                );
            }
        }
    }
    item.evidence_excerpt
        .split_whitespace()
        .collect::<String>()
        .to_lowercase()
}

fn explicit_template_url(text: &str, evidence: &str) -> Option<String> {
    for line in text.lines() {
        let normalized = line.split_whitespace().collect::<Vec<_>>().join(" ");
        let lower = normalized.to_lowercase();
        if !normalized.contains(evidence) || !(lower.contains("template") || lower.contains("模板"))
        {
            continue;
        }
        let urls = normalized
            .split_whitespace()
            .filter_map(|token| {
                let start = token.find("https://").or_else(|| token.find("http://"))?;
                let url = token[start..].trim_end_matches([
                    '.', ',', ';', ')', ']', '>', '"', '\'', '。', '，', '；', '）',
                ]);
                (url.len() < 2048
                    && url
                        .split("://")
                        .nth(1)
                        .is_some_and(|host| host.contains('.') && !host.starts_with('/')))
                .then(|| url.to_owned())
            })
            .collect::<std::collections::BTreeSet<_>>();
        if urls.len() == 1 {
            return urls.into_iter().next();
        }
    }
    None
}

fn find_evidence_excerpts(text: &str, keywords: &[&str]) -> Vec<String> {
    let normalized = text
        .lines()
        .map(|line| line.split_whitespace().collect::<Vec<_>>().join(" "))
        .collect::<Vec<_>>()
        .join("\n");
    normalized
        .split(['\n', '.', '!', '?', '。', '！', '？', ';', '；'])
        .map(str::trim)
        .filter(|sentence| sentence.chars().count() >= 12)
        .filter(|sentence| {
            let lowercase = sentence.to_lowercase();
            keywords
                .iter()
                .any(|keyword| keyword_matches(&lowercase, keyword))
        })
        .filter(|sentence| {
            is_requirement_statement(sentence) || is_explicit_numeric_constraint(sentence)
        })
        .map(|sentence| truncate_chars(sentence, 360))
        .collect()
}

fn is_requirement_statement(value: &str) -> bool {
    let lowercase = value.to_lowercase();
    [
        "must",
        "required",
        "mandatory",
        "shall",
        "should",
        "need to",
        "needs to",
        "have to",
        "may not",
        "must not",
        "do not",
        "not exceed",
        "no more than",
        "recommended",
        "encouraged",
        "optional",
        "preferably",
        "必须",
        "应当",
        "应提供",
        "应提交",
        "应包含",
        "应使用",
        "须提供",
        "须提交",
        "须包含",
        "须使用",
        "须上传",
        "须隐去",
        "须填写",
        "需同时",
        "作者需",
        "稿件需",
        "论文需",
        "投稿时需",
        "需要",
        "要求",
        "不得",
        "禁止",
        "不超过",
        "不受理",
        "不予受理",
        "不接受",
        "只接收",
        "请作者",
        "请提供",
        "请提交",
        "请使用",
        "请注明",
        "建议",
        "鼓励",
        "可选",
        "推荐",
        "自愿",
    ]
    .iter()
    .any(|marker| lowercase.contains(marker))
}

fn is_explicit_numeric_constraint(value: &str) -> bool {
    let lowercase = value.to_lowercase();
    lowercase
        .chars()
        .any(|character| character.is_ascii_digit())
        && [
            " word",
            "words",
            "character",
            " page",
            "pages",
            " dpi",
            " keyword",
            "keywords",
            "字以内",
            "个字",
            "页以内",
            "页",
            "个关键词",
            "幅图",
            "张图",
        ]
        .iter()
        .any(|unit| lowercase.contains(unit))
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

pub(crate) fn allows_applicability_review(excerpt: &str) -> bool {
    let text = excerpt.trim().to_lowercase();
    // Conditional scope can refer to article type, research participants, or a
    // submission stage, not just the literal phrase "if applicable".
    if ["regardless of", "不论"]
        .iter()
        .any(|marker| text.contains(marker))
    {
        return false;
    }
    let conditional = (text.starts_with("for ") && !text.starts_with("for submission"))
        || [
            "if ",
            "when ",
            "where ",
            "for review",
            "for research",
            "for clinical",
            "review articles",
            "research articles",
            "clinical studies",
            "case reports",
            "studies involving",
            "articles reporting",
            "only ",
            "如",
            "若",
            "涉及",
            "针对",
            "对于",
            "仅",
            "时，",
            "时需",
            "类文章",
            "类稿件",
            "综述文章",
            "临床研究",
            "病例报告",
        ]
        .iter()
        .any(|marker| text.contains(marker));
    if conditional {
        return true;
    }
    // Unknown applicability can be reviewed with a recorded basis. Explicit,
    // unconditional obligations cannot be waived through this control.
    ![
        "must",
        "required",
        "mandatory",
        "shall",
        "all manuscripts",
        "every submission",
        "必须",
        "须",
        "一律",
        "所有稿件",
        "不得",
        "不超过",
    ]
    .iter()
    .any(|marker| text.contains(marker))
}

pub(crate) fn presence_only_requirement(item: &JournalRequirementItem) -> bool {
    let text = item
        .evidence_excerpt
        .trim()
        .trim_end_matches(['.', '。'])
        .to_lowercase();
    match item.category {
        JournalRequirementCategory::Abstract => [
            "an abstract is required",
            "the abstract is required",
            "the manuscript must include an abstract",
            "必须包含摘要",
            "稿件须包含摘要",
        ]
        .contains(&text.as_str()),
        JournalRequirementCategory::Keywords => [
            "keywords are required",
            "the manuscript must include keywords",
            "必须包含关键词",
            "稿件须包含关键词",
        ]
        .contains(&text.as_str()),
        _ => false,
    }
}

fn detect_obligation(excerpt: &str) -> JournalRequirementObligation {
    let lowercase = excerpt.to_lowercase();
    if [
        "recommended",
        "encouraged",
        "optional",
        "preferably",
        "建议",
        "鼓励",
        "可选",
        "推荐",
        "自愿",
    ]
    .iter()
    .any(|marker| lowercase.contains(marker))
    {
        JournalRequirementObligation::Recommended
    } else if [
        "must",
        "required",
        "mandatory",
        "shall",
        "should",
        "need to",
        "needs to",
        "have to",
        "may not",
        "must not",
        "do not",
        "not exceed",
        "no more than",
        "必须",
        "应当",
        "应提供",
        "应提交",
        "应包含",
        "应使用",
        "须提供",
        "须提交",
        "须包含",
        "须使用",
        "须上传",
        "须隐去",
        "须填写",
        "需同时",
        "作者需",
        "稿件需",
        "论文需",
        "投稿时需",
        "需要",
        "要求",
        "不得",
        "禁止",
        "不超过",
        "不受理",
        "不予受理",
        "不接受",
        "只接收",
        "请作者",
        "请提供",
        "请提交",
        "请使用",
        "请注明",
    ]
    .iter()
    .any(|marker| lowercase.contains(marker))
    {
        JournalRequirementObligation::Required
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
        JournalRequirementCategory::SubmissionDeadline => "submission-deadline",
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
    fn rejects_navigation_article_cards_and_citation_tools_as_requirements() {
        let documents = vec![JournalRequirementSourceDocument {
            url: "https://crad.example/".to_owned(),
            title: "Journal home".to_owned(),
            text: "所有 标题 作者 关键词 摘要 DOI 栏目 首页 投稿须知 高级检索\n202550462 摘要 HTML全文 PDF PathRoute：基于全切片图像的癌症生存预测方法\n引用参考文献格式 You can copy and paste references from this page".to_owned(),
            official_host_matched: true,
        }];
        let (_, requirements) = extract_journal_requirements(&documents, 1_000);
        assert!(requirements.is_empty());
    }

    #[test]
    fn retains_distinct_declaration_instructions_and_explicit_template_links() {
        let text = "At submission authors must include a conflict of interest statement in the manuscript. After acceptance authors must upload a signed conflict of interest statement using the template https://publisher.example/coi.pdf\nAt submission authors must provide a funding declaration in the manuscript.";
        let document = JournalRequirementSourceDocument {
            url: "https://publisher.example/guide".into(),
            title: "Synthetic instructions".into(),
            text: text.into(),
            official_host_matched: true,
        };
        let (_, requirements) = extract_journal_requirements(&[document.clone(), document], 1000);
        let conflicts = requirements
            .iter()
            .filter(|item| item.category == JournalRequirementCategory::ConflictOfInterest)
            .collect::<Vec<_>>();
        assert_eq!(
            conflicts.len(),
            2,
            "retain distinct instructions, deduplicate repeated source text"
        );
        assert_eq!(
            conflicts[1]
                .declaration
                .as_ref()
                .unwrap()
                .template_url
                .as_deref(),
            Some("https://publisher.example/coi.pdf")
        );
        assert_eq!(
            conflicts[1].declaration.as_ref().unwrap().stage,
            crate::SubmissionStage::Accepted
        );
        assert!(requirements
            .iter()
            .any(|item| item.label_en == "Funding statement"));
        assert!(super::explicit_template_url("Authors must use the template at https://one.example/form and https://two.example/form", "Authors must use the template").is_none());
    }

    #[test]
    fn extracts_only_explicit_requirements_from_crad_author_guidance() {
        let documents = vec![JournalRequirementSourceDocument {
            url: "https://crad.example/tougaozhinan".to_owned(),
            title: "投稿须知".to_owned(),
            text: "本刊只接收中文稿，不受理英文稿。学术论文建议不超过15页，综述不超过20页，短文不超过4页。作者在投稿系统上传稿件时，需同时向编辑部提交所有作者手写签字确认的投稿声明扫描版，如不提供不予受理。本刊双盲评审，上传系统的电子版中需要隐去作者、单位、基金、作者简介等信息。稿件评审通过后，作者需提交单位审核盖章的不涉密证明和所有作者签字确认的著作权转让声明。".to_owned(),
            official_host_matched: true,
        }];
        let (_, requirements) = extract_journal_requirements(&documents, 1_000);

        assert!(requirements.iter().any(|item| {
            item.category == JournalRequirementCategory::ManuscriptFile
                && item.evidence_excerpt.contains("只接收中文稿")
        }));
        assert!(requirements.iter().any(|item| {
            item.category == JournalRequirementCategory::AnonymousReview
                && item.obligation == JournalRequirementObligation::Required
        }));
        assert!(requirements.iter().any(|item| {
            item.category == JournalRequirementCategory::LengthLimit
                && item.obligation == JournalRequirementObligation::Recommended
        }));
        assert_eq!(
            requirements
                .iter()
                .filter(|item| item.category == JournalRequirementCategory::OtherSupportingFiles)
                .count(),
            3
        );
        assert!(!requirements.iter().any(|item| matches!(
            item.category,
            JournalRequirementCategory::Abstract
                | JournalRequirementCategory::Keywords
                | JournalRequirementCategory::Figures
                | JournalRequirementCategory::References
        )));
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
