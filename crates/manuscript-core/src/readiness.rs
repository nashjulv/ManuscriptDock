use crate::{AnalysisQuality, StructureReport};
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, error::Error, fmt};

pub const READINESS_REPORT_VERSION: u32 = 2;
pub const OUTPUT_SNAPSHOT_VERSION: u32 = 1;

const RULE_PACK_PUBLIC_KEYS_HEX: &[&str] = &[
    "a76e0138d7ebdbbaf68323babd34eb50d414da97b925c1598cc708d48464f8b9",
    "2f3be904c57c5dd9b4f7598fdefb2303a63ed5b1292c22a9294967886230f6c5",
    "f94aa54a9f707fa66c450835f4f41fe2e06a3baf8415fafded6a1854ce88a0cc",
];
const CORE_PACK: &[u8] = include_bytes!("../rule-packs/core-structure-v1.json");
const CORE_SIGNATURE: &str = include_str!("../rule-packs/core-structure-v1.sig");
const INITIAL_PACK: &[u8] = include_bytes!("../rule-packs/initial-submission-v1.json");
const INITIAL_SIGNATURE: &str = include_str!("../rule-packs/initial-submission-v1.sig");

macro_rules! bundled_pack {
    ($pack:ident, $signature:ident, $name:literal) => {
        const $pack: &[u8] = include_bytes!(concat!("../rule-packs/", $name, ".json"));
        const $signature: &str = include_str!(concat!("../rule-packs/", $name, ".sig"));
    };
}

bundled_pack!(
    CHINA_ACADEMIC_PACK,
    CHINA_ACADEMIC_SIGNATURE,
    "china-academic-v1"
);
bundled_pack!(
    CHINA_REFERENCES_PACK,
    CHINA_REFERENCES_SIGNATURE,
    "china-references-2025-v1"
);
bundled_pack!(
    CHINA_DATA_PAPER_PACK,
    CHINA_DATA_PAPER_SIGNATURE,
    "china-data-paper-2025-v1"
);
bundled_pack!(ETHICS_PACK, ETHICS_SIGNATURE, "ethics-transparency-v1");
bundled_pack!(ELSEVIER_PACK, ELSEVIER_SIGNATURE, "publisher-elsevier-v1");
bundled_pack!(
    SPRINGER_PACK,
    SPRINGER_SIGNATURE,
    "publisher-springer-nature-v1"
);
bundled_pack!(WILEY_PACK, WILEY_SIGNATURE, "publisher-wiley-v1");
bundled_pack!(IEEE_PACK, IEEE_SIGNATURE, "publisher-ieee-v1");
bundled_pack!(ICMJE_PACK, ICMJE_SIGNATURE, "report-icmje-2026-v1");
bundled_pack!(CONSORT_PACK, CONSORT_SIGNATURE, "report-consort-2025-v1");
bundled_pack!(SPIRIT_PACK, SPIRIT_SIGNATURE, "report-spirit-2025-v1");
bundled_pack!(STROBE_PACK, STROBE_SIGNATURE, "report-strobe-v1");
bundled_pack!(PRISMA_PACK, PRISMA_SIGNATURE, "report-prisma-2020-v1");
bundled_pack!(CARE_PACK, CARE_SIGNATURE, "report-care-v1");
bundled_pack!(ARRIVE_PACK, ARRIVE_SIGNATURE, "report-arrive-2-v1");
bundled_pack!(
    QUALITATIVE_PACK,
    QUALITATIVE_SIGNATURE,
    "report-qualitative-v1"
);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RuleClassification {
    Must,
    Recommendation,
    AuthorConfirmation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FindingStatus {
    Passed,
    Warning,
    Blocked,
    Confirmation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ReadinessOutcome {
    Ready,
    NeedsAttention,
    Blocked,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RuleFinding {
    pub rule_id: String,
    pub rule_pack_id: String,
    pub classification: RuleClassification,
    pub status: FindingStatus,
    pub message: String,
    pub message_en: String,
    pub source_location: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RulePackReference {
    pub id: String,
    pub version: String,
    pub coverage: String,
    pub stage: String,
    pub source_label: String,
    pub source_label_en: String,
    pub source_urls: Vec<String>,
    pub verified_at: String,
    pub signature_verified: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RulePackCatalogItem {
    pub id: String,
    pub version: String,
    pub coverage: String,
    pub stage: String,
    pub region: String,
    pub category: String,
    pub source_label: String,
    pub source_label_en: String,
    pub description: String,
    pub description_en: String,
    pub source_urls: Vec<String>,
    pub verified_at: String,
    pub signature_verified: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RulePackCatalog {
    pub rule_packs: Vec<RulePackCatalogItem>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SubmissionElementRequirement {
    Required,
    Recommended,
    AuthorConfirmation,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SubmissionElementCatalogItem {
    pub id: String,
    pub group: String,
    pub label: String,
    pub label_en: String,
    pub description: String,
    pub description_en: String,
    pub requirement: SubmissionElementRequirement,
    pub editable_field: Option<String>,
    pub rule_pack_ids: Vec<String>,
    pub source_labels: Vec<String>,
    pub source_labels_en: Vec<String>,
    pub source_urls: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SubmissionElementCatalog {
    pub elements: Vec<SubmissionElementCatalogItem>,
    pub rule_packs: Vec<RulePackReference>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ExternalTransmission {
    NotPerformed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReadinessReport {
    pub report_version: u32,
    pub report_id: String,
    pub workspace_id: String,
    pub source_content_hash: String,
    pub source_snapshot_version: u32,
    pub output_snapshot_version: u32,
    pub generated_unix_ms: u64,
    pub outcome: ReadinessOutcome,
    pub passed_count: u32,
    pub warning_count: u32,
    pub blocked_count: u32,
    pub confirmation_count: u32,
    pub findings: Vec<RuleFinding>,
    pub rule_packs: Vec<RulePackReference>,
    pub external_transmission: ExternalTransmission,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum ReadinessEvaluation {
    Completed { report: ReadinessReport },
    Rejected { message: String },
}

#[derive(Debug)]
pub enum ReadinessError {
    InvalidTrustAnchor,
    InvalidSignature(String),
    InvalidRulePack(String),
}

impl fmt::Display for ReadinessError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidTrustAnchor => write!(formatter, "内置规则信任锚无效"),
            Self::InvalidSignature(pack) => write!(formatter, "规则包 {pack} 的签名验证失败"),
            Self::InvalidRulePack(message) => write!(formatter, "投稿规则包无效：{message}"),
        }
    }
}

impl Error for ReadinessError {}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RulePack {
    schema_version: u32,
    id: String,
    version: String,
    layer: String,
    coverage: String,
    stage: String,
    inherits: Vec<String>,
    source_label: String,
    #[serde(default)]
    source_label_en: String,
    #[serde(default)]
    description: String,
    #[serde(default)]
    description_en: String,
    #[serde(default)]
    region: String,
    #[serde(default)]
    category: String,
    #[serde(default)]
    source_urls: Vec<String>,
    #[serde(default)]
    verified_at: String,
    #[serde(default)]
    selectable: bool,
    #[serde(default)]
    submission_elements: Vec<SubmissionElement>,
    rules: Vec<Rule>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SubmissionElement {
    id: String,
    group: String,
    label: String,
    label_en: String,
    description: String,
    description_en: String,
    requirement: SubmissionElementRequirement,
    #[serde(default)]
    editable_field: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Rule {
    id: String,
    field: String,
    operator: RuleOperator,
    value: Option<u32>,
    classification: RuleClassification,
    message: String,
    #[serde(default)]
    message_en: String,
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
enum RuleOperator {
    Present,
    Minimum,
    Complete,
    Confirm,
}

#[derive(Debug)]
struct VerifiedPack {
    pack: RulePack,
    signature_verified: bool,
}

pub(crate) fn evaluate_readiness(
    structure: &StructureReport,
    report_id: String,
    generated_unix_ms: u64,
    selected_rule_pack_ids: &[String],
) -> Result<ReadinessReport, ReadinessError> {
    let packs = bundled_rule_packs()?;
    let ordered = compose_selected_rule_packs(&packs, selected_rule_pack_ids)?;
    let mut findings = Vec::new();
    let mut references = Vec::new();

    for index in ordered {
        let verified = &packs[index];
        references.push(RulePackReference {
            id: verified.pack.id.clone(),
            version: verified.pack.version.clone(),
            coverage: verified.pack.coverage.clone(),
            stage: verified.pack.stage.clone(),
            source_label: verified.pack.source_label.clone(),
            source_label_en: verified.pack.source_label_en.clone(),
            source_urls: verified.pack.source_urls.clone(),
            verified_at: verified.pack.verified_at.clone(),
            signature_verified: verified.signature_verified,
        });
        for rule in &verified.pack.rules {
            let passed = evaluate_rule(rule, structure)?;
            findings.push(RuleFinding {
                rule_id: rule.id.clone(),
                rule_pack_id: verified.pack.id.clone(),
                classification: rule.classification,
                status: finding_status(rule.classification, passed),
                message: rule.message.clone(),
                message_en: rule.message_en.clone(),
                source_location: source_location(&rule.field).to_owned(),
            });
        }
    }

    let passed_count = count_status(&findings, FindingStatus::Passed);
    let warning_count = count_status(&findings, FindingStatus::Warning);
    let blocked_count = count_status(&findings, FindingStatus::Blocked);
    let confirmation_count = count_status(&findings, FindingStatus::Confirmation);
    let outcome = if blocked_count > 0 {
        ReadinessOutcome::Blocked
    } else if warning_count > 0 || confirmation_count > 0 {
        ReadinessOutcome::NeedsAttention
    } else {
        ReadinessOutcome::Ready
    };

    Ok(ReadinessReport {
        report_version: READINESS_REPORT_VERSION,
        report_id,
        workspace_id: structure.workspace_id.clone(),
        source_content_hash: structure.source_content_hash.clone(),
        source_snapshot_version: structure.source_snapshot_version,
        output_snapshot_version: OUTPUT_SNAPSHOT_VERSION,
        generated_unix_ms,
        outcome,
        passed_count,
        warning_count,
        blocked_count,
        confirmation_count,
        findings,
        rule_packs: references,
        external_transmission: ExternalTransmission::NotPerformed,
    })
}

pub fn bundled_rule_pack_catalog() -> Result<RulePackCatalog, ReadinessError> {
    let mut rule_packs = bundled_rule_packs()?
        .into_iter()
        .filter(|verified| verified.pack.selectable)
        .map(|verified| RulePackCatalogItem {
            id: verified.pack.id,
            version: verified.pack.version,
            coverage: verified.pack.coverage,
            stage: verified.pack.stage,
            region: verified.pack.region,
            category: verified.pack.category,
            source_label: verified.pack.source_label,
            source_label_en: verified.pack.source_label_en,
            description: verified.pack.description,
            description_en: verified.pack.description_en,
            source_urls: verified.pack.source_urls,
            verified_at: verified.pack.verified_at,
            signature_verified: verified.signature_verified,
        })
        .collect::<Vec<_>>();
    rule_packs.sort_by(|left, right| {
        left.category
            .cmp(&right.category)
            .then_with(|| left.source_label.cmp(&right.source_label))
    });
    Ok(RulePackCatalog { rule_packs })
}

pub fn bundled_submission_element_catalog(
    selected_rule_pack_ids: &[String],
) -> Result<SubmissionElementCatalog, ReadinessError> {
    let packs = bundled_rule_packs()?;
    let ordered = compose_selected_rule_packs(&packs, selected_rule_pack_ids)?;
    let mut elements: Vec<SubmissionElementCatalogItem> = Vec::new();
    let mut references = Vec::new();

    for index in ordered {
        let verified = &packs[index];
        if !verified.pack.submission_elements.is_empty() {
            references.push(RulePackReference {
                id: verified.pack.id.clone(),
                version: verified.pack.version.clone(),
                coverage: verified.pack.coverage.clone(),
                stage: verified.pack.stage.clone(),
                source_label: verified.pack.source_label.clone(),
                source_label_en: verified.pack.source_label_en.clone(),
                source_urls: verified.pack.source_urls.clone(),
                verified_at: verified.pack.verified_at.clone(),
                signature_verified: verified.signature_verified,
            });
        }
        for element in &verified.pack.submission_elements {
            if let Some(existing) = elements.iter_mut().find(|item| item.id == element.id) {
                existing.requirement =
                    stronger_requirement(existing.requirement, element.requirement);
                if existing.editable_field.is_none() {
                    existing.editable_field = element.editable_field.clone();
                }
                push_unique_string(&mut existing.rule_pack_ids, &verified.pack.id);
                push_unique_string(&mut existing.source_labels, &verified.pack.source_label);
                push_unique_string(
                    &mut existing.source_labels_en,
                    &verified.pack.source_label_en,
                );
                for source in &verified.pack.source_urls {
                    push_unique_string(&mut existing.source_urls, source);
                }
            } else {
                elements.push(SubmissionElementCatalogItem {
                    id: element.id.clone(),
                    group: element.group.clone(),
                    label: element.label.clone(),
                    label_en: element.label_en.clone(),
                    description: element.description.clone(),
                    description_en: element.description_en.clone(),
                    requirement: element.requirement,
                    editable_field: element.editable_field.clone(),
                    rule_pack_ids: vec![verified.pack.id.clone()],
                    source_labels: vec![verified.pack.source_label.clone()],
                    source_labels_en: vec![verified.pack.source_label_en.clone()],
                    source_urls: verified.pack.source_urls.clone(),
                });
            }
        }
    }
    elements.sort_by(|left, right| left.group.cmp(&right.group).then(left.id.cmp(&right.id)));
    Ok(SubmissionElementCatalog {
        elements,
        rule_packs: references,
    })
}

fn stronger_requirement(
    left: SubmissionElementRequirement,
    right: SubmissionElementRequirement,
) -> SubmissionElementRequirement {
    fn rank(value: SubmissionElementRequirement) -> u8 {
        match value {
            SubmissionElementRequirement::Required => 3,
            SubmissionElementRequirement::Recommended => 2,
            SubmissionElementRequirement::AuthorConfirmation => 1,
        }
    }
    if rank(right) > rank(left) {
        right
    } else {
        left
    }
}

fn push_unique_string(values: &mut Vec<String>, value: &str) {
    if !values.iter().any(|existing| existing == value) {
        values.push(value.to_owned());
    }
}

pub(crate) fn render_readiness_html(report: &ReadinessReport, manuscript_name: &str) -> String {
    let outcome = match report.outcome {
        ReadinessOutcome::Ready => "已具备基础投稿条件",
        ReadinessOutcome::NeedsAttention => "仍有事项需要处理",
        ReadinessOutcome::Blocked => "存在阻断项",
    };
    let mut findings = String::new();
    for finding in &report.findings {
        let status = match finding.status {
            FindingStatus::Passed => "通过",
            FindingStatus::Warning => "建议",
            FindingStatus::Blocked => "阻断",
            FindingStatus::Confirmation => "作者确认",
        };
        findings.push_str(&format!(
            "<li><strong>{}</strong><span>{}</span><p>{}</p><code>{}</code></li>",
            escape_html(status),
            escape_html(&finding.rule_id),
            escape_html(&finding.message),
            escape_html(&finding.source_location)
        ));
    }
    let packs = report
        .rule_packs
        .iter()
        .map(|pack| {
            format!(
                "<li>{} · v{} · 覆盖等级 {} · 来源可信，内容未被篡改</li>",
                escape_html(&pack.source_label),
                escape_html(&pack.version),
                escape_html(&pack.coverage)
            )
        })
        .collect::<String>();

    format!(
        "<!doctype html><html lang=\"zh-CN\"><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width,initial-scale=1\"><title>投稿准备报告</title><style>{}</style></head><body><main><p class=\"eyebrow\">ManuscriptDock · 本地报告</p><h1>{}</h1><p class=\"file\">{}</p><section class=\"summary\"><strong>{}</strong><span>通过 {} · 建议 {} · 阻断 {} · 待确认 {}</span></section><h2>检查明细</h2><ol>{}</ol><h2>规则来源</h2><ul class=\"packs\">{}</ul><footer>源快照 v{} · 内容指纹 {} · 未发生外部传输</footer></main></body></html>",
        REPORT_CSS,
        escape_html(outcome),
        escape_html(manuscript_name),
        escape_html(outcome),
        report.passed_count,
        report.warning_count,
        report.blocked_count,
        report.confirmation_count,
        findings,
        packs,
        report.source_snapshot_version,
        escape_html(&report.source_content_hash[..12.min(report.source_content_hash.len())])
    )
}

fn bundled_rule_packs() -> Result<Vec<VerifiedPack>, ReadinessError> {
    [
        (CORE_PACK, CORE_SIGNATURE),
        (INITIAL_PACK, INITIAL_SIGNATURE),
        (CHINA_ACADEMIC_PACK, CHINA_ACADEMIC_SIGNATURE),
        (CHINA_REFERENCES_PACK, CHINA_REFERENCES_SIGNATURE),
        (CHINA_DATA_PAPER_PACK, CHINA_DATA_PAPER_SIGNATURE),
        (ETHICS_PACK, ETHICS_SIGNATURE),
        (ELSEVIER_PACK, ELSEVIER_SIGNATURE),
        (SPRINGER_PACK, SPRINGER_SIGNATURE),
        (WILEY_PACK, WILEY_SIGNATURE),
        (IEEE_PACK, IEEE_SIGNATURE),
        (ICMJE_PACK, ICMJE_SIGNATURE),
        (CONSORT_PACK, CONSORT_SIGNATURE),
        (SPIRIT_PACK, SPIRIT_SIGNATURE),
        (STROBE_PACK, STROBE_SIGNATURE),
        (PRISMA_PACK, PRISMA_SIGNATURE),
        (CARE_PACK, CARE_SIGNATURE),
        (ARRIVE_PACK, ARRIVE_SIGNATURE),
        (QUALITATIVE_PACK, QUALITATIVE_SIGNATURE),
    ]
    .into_iter()
    .map(|(bytes, signature)| verify_rule_pack(bytes, signature))
    .collect()
}

fn verify_rule_pack(bytes: &[u8], signature_hex: &str) -> Result<VerifiedPack, ReadinessError> {
    let pack: RulePack = serde_json::from_slice(bytes)
        .map_err(|error| ReadinessError::InvalidRulePack(error.to_string()))?;
    if pack.schema_version != 1 || pack.id.trim().is_empty() || pack.rules.is_empty() {
        return Err(ReadinessError::InvalidRulePack(format!(
            "{} 的元数据不完整",
            pack.id
        )));
    }
    if pack.layer.trim().is_empty() {
        return Err(ReadinessError::InvalidRulePack(format!(
            "{} 缺少规则层级",
            pack.id
        )));
    }
    if pack.selectable
        && (pack.source_label_en.trim().is_empty()
            || pack.description.trim().is_empty()
            || pack.description_en.trim().is_empty()
            || pack.region.trim().is_empty()
            || pack.category.trim().is_empty()
            || pack.verified_at.trim().is_empty()
            || pack.source_urls.is_empty()
            || pack
                .source_urls
                .iter()
                .any(|source| !source.starts_with("https://"))
            || pack
                .rules
                .iter()
                .any(|rule| rule.message_en.trim().is_empty())
            || pack.submission_elements.iter().any(|element| {
                element.id.trim().is_empty()
                    || element.group.trim().is_empty()
                    || element.label.trim().is_empty()
                    || element.label_en.trim().is_empty()
                    || element.description.trim().is_empty()
                    || element.description_en.trim().is_empty()
            }))
    {
        return Err(ReadinessError::InvalidRulePack(format!(
            "{} 缺少可选规则目录元数据",
            pack.id
        )));
    }
    let mut element_ids = HashMap::new();
    for element in &pack.submission_elements {
        if element_ids.insert(element.id.as_str(), ()).is_some() {
            return Err(ReadinessError::InvalidRulePack(format!(
                "{} 的投稿要素 {} 重复",
                pack.id, element.id
            )));
        }
    }
    let signature_bytes = hex::decode(signature_hex.trim())
        .map_err(|_| ReadinessError::InvalidSignature(pack.id.clone()))?;
    let signature = Signature::from_slice(&signature_bytes)
        .map_err(|_| ReadinessError::InvalidSignature(pack.id.clone()))?;
    let verified = RULE_PACK_PUBLIC_KEYS_HEX
        .iter()
        .try_fold(false, |verified, key_hex| {
            if verified {
                return Ok(true);
            }
            let public_key: [u8; 32] = hex::decode(key_hex)
                .map_err(|_| ReadinessError::InvalidTrustAnchor)?
                .try_into()
                .map_err(|_| ReadinessError::InvalidTrustAnchor)?;
            let verifying_key = VerifyingKey::from_bytes(&public_key)
                .map_err(|_| ReadinessError::InvalidTrustAnchor)?;
            Ok(verifying_key.verify(bytes, &signature).is_ok())
        })?;
    if !verified {
        return Err(ReadinessError::InvalidSignature(pack.id.clone()));
    }
    Ok(VerifiedPack {
        pack,
        signature_verified: true,
    })
}

fn compose_selected_rule_packs(
    packs: &[VerifiedPack],
    selected_rule_pack_ids: &[String],
) -> Result<Vec<usize>, ReadinessError> {
    let index_by_id: HashMap<&str, usize> = packs
        .iter()
        .enumerate()
        .map(|(index, pack)| (pack.pack.id.as_str(), index))
        .collect();
    if index_by_id.len() != packs.len() {
        return Err(ReadinessError::InvalidRulePack("规则包标识重复".to_owned()));
    }
    let mut roots = vec!["md.stage.initial-submission"];
    for id in selected_rule_pack_ids {
        if !roots.contains(&id.as_str()) {
            roots.push(id);
        }
    }
    let mut ordered = Vec::new();
    let mut visiting = Vec::new();
    for id in roots {
        let index = index_by_id
            .get(id)
            .copied()
            .ok_or_else(|| ReadinessError::InvalidRulePack(format!("未找到所选规则包 {id}")))?;
        if id != "md.stage.initial-submission" && !packs[index].pack.selectable {
            return Err(ReadinessError::InvalidRulePack(format!(
                "规则包 {id} 不能由界面直接选择"
            )));
        }
        visit_pack(index, packs, &index_by_id, &mut visiting, &mut ordered)?;
    }
    validate_rule_ownership(packs, &ordered)?;
    Ok(ordered)
}

#[cfg(test)]
fn compose_rule_packs(packs: &[VerifiedPack]) -> Result<Vec<usize>, ReadinessError> {
    let index_by_id: HashMap<&str, usize> = packs
        .iter()
        .enumerate()
        .map(|(index, pack)| (pack.pack.id.as_str(), index))
        .collect();
    if index_by_id.len() != packs.len() {
        return Err(ReadinessError::InvalidRulePack("规则包标识重复".to_owned()));
    }
    let mut ordered = Vec::new();
    let mut visiting = Vec::new();
    for index in 0..packs.len() {
        visit_pack(index, packs, &index_by_id, &mut visiting, &mut ordered)?;
    }

    validate_rule_ownership(packs, &ordered)?;
    Ok(ordered)
}

fn validate_rule_ownership(
    packs: &[VerifiedPack],
    ordered: &[usize],
) -> Result<(), ReadinessError> {
    let mut rule_owners = HashMap::new();
    for index in ordered {
        for rule in &packs[*index].pack.rules {
            if let Some(owner) =
                rule_owners.insert(rule.id.as_str(), packs[*index].pack.id.as_str())
            {
                return Err(ReadinessError::InvalidRulePack(format!(
                    "规则 {} 同时出现在 {} 和 {}",
                    rule.id, owner, packs[*index].pack.id
                )));
            }
        }
    }
    Ok(())
}

fn visit_pack<'a>(
    index: usize,
    packs: &'a [VerifiedPack],
    index_by_id: &HashMap<&'a str, usize>,
    visiting: &mut Vec<usize>,
    ordered: &mut Vec<usize>,
) -> Result<(), ReadinessError> {
    if ordered.contains(&index) {
        return Ok(());
    }
    if visiting.contains(&index) {
        return Err(ReadinessError::InvalidRulePack(
            "规则包继承存在循环".to_owned(),
        ));
    }
    visiting.push(index);
    for inherited in &packs[index].pack.inherits {
        let inherited_index = index_by_id
            .get(inherited.as_str())
            .copied()
            .ok_or_else(|| {
                ReadinessError::InvalidRulePack(format!("缺少继承规则包 {inherited}"))
            })?;
        visit_pack(inherited_index, packs, index_by_id, visiting, ordered)?;
    }
    visiting.pop();
    ordered.push(index);
    Ok(())
}

fn evaluate_rule(rule: &Rule, structure: &StructureReport) -> Result<bool, ReadinessError> {
    match (rule.field.as_str(), rule.operator) {
        ("title", RuleOperator::Present) => Ok(structure.title.is_some()),
        ("abstract", RuleOperator::Present) => Ok(structure.abstract_present),
        ("keywords", RuleOperator::Present) => Ok(structure.keywords_present),
        ("references", RuleOperator::Present) => Ok(structure.references_present),
        ("sections", RuleOperator::Minimum) => Ok(structure.sections.len()
            >= rule.value.ok_or_else(|| {
                ReadinessError::InvalidRulePack(format!("规则 {} 缺少最小值", rule.id))
            })? as usize),
        ("declaration.conflict_of_interest", RuleOperator::Present) => Ok(structure
            .declarations
            .iter()
            .any(|value| value == "conflict_of_interest")),
        ("declaration.data_availability", RuleOperator::Present) => Ok(structure
            .declarations
            .iter()
            .any(|value| value == "data_availability")),
        ("analysis_quality", RuleOperator::Complete) => {
            Ok(structure.quality == AnalysisQuality::Complete)
        }
        (field, RuleOperator::Present) if field.starts_with("declaration.") => Ok(structure
            .declarations
            .iter()
            .any(|value| value == field.trim_start_matches("declaration."))),
        (field, RuleOperator::Present) if field.starts_with("section.") => {
            Ok(has_section(structure, field.trim_start_matches("section.")))
        }
        (field, RuleOperator::Confirm) if field.starts_with("confirmation.") => Ok(false),
        _ => Err(ReadinessError::InvalidRulePack(format!(
            "规则 {} 使用了不支持的字段或操作符",
            rule.id
        ))),
    }
}

fn finding_status(classification: RuleClassification, passed: bool) -> FindingStatus {
    if passed {
        FindingStatus::Passed
    } else {
        match classification {
            RuleClassification::Must => FindingStatus::Blocked,
            RuleClassification::Recommendation => FindingStatus::Warning,
            RuleClassification::AuthorConfirmation => FindingStatus::Confirmation,
        }
    }
}

fn count_status(findings: &[RuleFinding], status: FindingStatus) -> u32 {
    u32::try_from(
        findings
            .iter()
            .filter(|finding| finding.status == status)
            .count(),
    )
    .unwrap_or(u32::MAX)
}

fn source_location(field: &str) -> &'static str {
    match field {
        "title" => "document.title",
        "abstract" => "document.abstract",
        "keywords" => "document.keywords",
        "references" => "document.references",
        "sections" => "document.sections",
        "declaration.conflict_of_interest" => "document.declarations.conflict_of_interest",
        "declaration.data_availability" => "document.declarations.data_availability",
        "analysis_quality" => "analysis.quality",
        field if field.starts_with("declaration.") => "document.declarations",
        field if field.starts_with("section.") => "document.sections",
        field if field.starts_with("confirmation.") => "author.confirmation",
        _ => "document",
    }
}

fn has_section(structure: &StructureReport, expected: &str) -> bool {
    let aliases: &[&str] = match expected {
        "introduction" => &["introduction", "background", "引言", "绪论"],
        "methods" => &[
            "methods",
            "materials and methods",
            "methodology",
            "方法",
            "材料与方法",
        ],
        "results" => &["results", "findings", "结果"],
        "discussion" => &["discussion", "讨论"],
        "conclusion" => &["conclusion", "conclusions", "结论"],
        "case_presentation" => &["case presentation", "case report", "病例介绍", "病例报告"],
        _ => &[],
    };
    structure.sections.iter().any(|section| {
        let normalized = section.heading.trim().to_ascii_lowercase();
        aliases.iter().any(|alias| normalized.contains(alias))
    })
}

fn escape_html(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

const REPORT_CSS: &str = "body{margin:0;background:#f7f7f5;color:#1f2321;font:15px/1.65 -apple-system,BlinkMacSystemFont,Segoe UI,sans-serif}main{max-width:760px;margin:0 auto;padding:64px 28px}.eyebrow{color:#176b52;font-size:12px;font-weight:700;letter-spacing:.1em}h1{margin:.2em 0;font:500 36px/1.15 Georgia,serif}.file{color:#666c68}.summary{display:flex;justify-content:space-between;gap:20px;margin:28px 0;padding:20px;border:1px solid #dedfdb;border-radius:12px;background:#fff}.summary span{color:#666c68}h2{margin-top:34px;font-size:17px}ol,.packs{padding:0;list-style:none}ol li{display:grid;grid-template-columns:90px 1fr;gap:5px 14px;padding:16px 0;border-top:1px solid #dedfdb}ol li strong{color:#176b52}ol li span{font-size:12px;color:#666c68}ol li p{grid-column:2;margin:0}ol li code{grid-column:2;color:#868c88;font-size:11px}.packs li{padding:7px 0;color:#666c68}footer{margin-top:40px;padding-top:18px;border-top:1px solid #dedfdb;color:#868c88;font-size:11px}@media(max-width:560px){main{padding:36px 18px}.summary{flex-direction:column}ol li{grid-template-columns:1fr}ol li p,ol li code{grid-column:1}}";

#[cfg(test)]
mod tests {
    use super::{
        bundled_rule_pack_catalog, bundled_submission_element_catalog, evaluate_readiness,
        render_readiness_html, verify_rule_pack, FindingStatus, ReadinessOutcome,
    };
    use crate::{AnalysisQuality, SectionSummary, StructureReport};

    #[test]
    fn verifies_and_composes_the_bundled_rule_packs() {
        let report =
            evaluate_readiness(&complete_structure(), "report-1".to_owned(), 123, &[]).unwrap();

        assert_eq!(report.rule_packs.len(), 2);
        assert!(report.rule_packs.iter().all(|pack| pack.signature_verified));
        assert_eq!(report.findings.len(), 8);
        assert_eq!(report.outcome, ReadinessOutcome::Ready);
    }

    #[test]
    fn classifies_blockers_warnings_and_author_confirmations() {
        let mut structure = complete_structure();
        structure.title = None;
        structure.keywords_present = false;
        structure.declarations.clear();
        structure.quality = AnalysisQuality::Limited;

        let report = evaluate_readiness(&structure, "report-2".to_owned(), 456, &[]).unwrap();

        assert_eq!(report.outcome, ReadinessOutcome::Blocked);
        assert_eq!(report.blocked_count, 1);
        assert_eq!(report.warning_count, 1);
        assert_eq!(report.confirmation_count, 3);
        assert!(report
            .findings
            .iter()
            .any(|finding| finding.status == FindingStatus::Blocked));
    }

    #[test]
    fn exposes_verified_selectable_standards_without_internal_base_packs() {
        let catalog = bundled_rule_pack_catalog().unwrap();

        assert_eq!(catalog.rule_packs.len(), 16);
        assert!(catalog
            .rule_packs
            .iter()
            .all(|pack| pack.signature_verified && !pack.source_urls.is_empty()));
        assert!(catalog
            .rule_packs
            .iter()
            .any(|pack| pack.id == "md.standard.cn.references-7714"
                && pack.version.starts_with("2025")));
        assert!(!catalog
            .rule_packs
            .iter()
            .any(|pack| pack.id == "md.core.structure"));
    }

    #[test]
    fn applies_only_selected_enhancements_and_their_dependencies() {
        let selected = vec!["md.publisher.ieee".to_owned()];
        let report = evaluate_readiness(
            &complete_structure(),
            "report-ieee".to_owned(),
            321,
            &selected,
        )
        .unwrap();

        assert_eq!(report.rule_packs.len(), 3);
        assert!(report
            .rule_packs
            .iter()
            .any(|pack| pack.id == "md.publisher.ieee"));
        assert!(report
            .findings
            .iter()
            .any(|finding| finding.rule_id == "ieee.abstract-style.confirm"));
        assert!(!report
            .findings
            .iter()
            .any(|finding| finding.rule_id.starts_with("prisma.")));
    }

    #[test]
    fn aggregates_signed_publisher_submission_elements_without_duplicates() {
        let selected = vec![
            "md.publisher.elsevier".to_owned(),
            "md.publisher.ieee".to_owned(),
        ];
        let catalog = bundled_submission_element_catalog(&selected).unwrap();

        assert_eq!(catalog.rule_packs.len(), 2);
        assert!(catalog
            .rule_packs
            .iter()
            .all(|pack| pack.signature_verified));
        assert_eq!(
            catalog
                .elements
                .iter()
                .filter(|element| element.id == "title")
                .count(),
            1
        );
        let title = catalog
            .elements
            .iter()
            .find(|element| element.id == "title")
            .unwrap();
        assert_eq!(title.editable_field.as_deref(), Some("title"));
        assert_eq!(title.rule_pack_ids.len(), 2);
        assert!(catalog.elements.iter().any(|element| element.id == "orcid"));
        assert!(catalog
            .elements
            .iter()
            .any(|element| element.id == "data_availability"));
    }

    #[test]
    fn rejects_an_unknown_selected_rule_pack() {
        let error = evaluate_readiness(
            &complete_structure(),
            "report-unknown".to_owned(),
            654,
            &["md.publisher.unknown".to_owned()],
        )
        .unwrap_err();

        assert!(error.to_string().contains("未找到所选规则包"));
    }

    #[test]
    fn rejects_a_tampered_signed_rule_pack() {
        let mut bytes = super::CORE_PACK.to_vec();
        let marker = b"ManuscriptDock";
        let position = bytes
            .windows(marker.len())
            .position(|window| window == marker)
            .unwrap();
        bytes[position] = b'X';

        let error = verify_rule_pack(&bytes, super::CORE_SIGNATURE).unwrap_err();

        assert!(error.to_string().contains("签名验证失败"));
    }

    #[test]
    fn rejects_cyclic_rule_pack_inheritance() {
        let packs = vec![
            test_pack("one", &["two"], "rule.one"),
            test_pack("two", &["one"], "rule.two"),
        ];

        let error = super::compose_rule_packs(&packs).unwrap_err();

        assert!(error.to_string().contains("循环"));
    }

    #[test]
    fn rejects_duplicate_rule_ids_across_layers() {
        let packs = vec![
            test_pack("base", &[], "shared.rule"),
            test_pack("stage", &["base"], "shared.rule"),
        ];

        let error = super::compose_rule_packs(&packs).unwrap_err();

        assert!(error.to_string().contains("同时出现在"));
    }

    #[test]
    fn escapes_manuscript_content_in_the_html_preview() {
        let report =
            evaluate_readiness(&complete_structure(), "report-3".to_owned(), 789, &[]).unwrap();

        let html = render_readiness_html(&report, "<script>alert(1)</script>.tex");

        assert!(!html.contains("<script>alert(1)</script>"));
        assert!(html.contains("&lt;script&gt;alert(1)&lt;/script&gt;"));
        assert!(html.contains("未发生外部传输"));
    }

    fn complete_structure() -> StructureReport {
        StructureReport {
            analysis_version: 1,
            workspace_id: "workspace-1".to_owned(),
            source_content_hash: "a".repeat(64),
            source_snapshot_version: 1,
            quality: AnalysisQuality::Complete,
            title: Some("Synthetic Study".to_owned()),
            authors: vec!["Synthetic Author".to_owned()],
            abstract_present: true,
            abstract_text: Some("Synthetic abstract.".to_owned()),
            keywords_present: true,
            sections: vec![
                SectionSummary {
                    level: 1,
                    heading: "Introduction".to_owned(),
                },
                SectionSummary {
                    level: 1,
                    heading: "Methods".to_owned(),
                },
            ],
            figure_count: 0,
            table_count: 0,
            references_present: true,
            declarations: vec![
                "conflict_of_interest".to_owned(),
                "data_availability".to_owned(),
            ],
            page_count: None,
            word_count: 1200,
            warnings: Vec::new(),
        }
    }

    fn test_pack(id: &str, inherits: &[&str], rule_id: &str) -> super::VerifiedPack {
        super::VerifiedPack {
            pack: super::RulePack {
                schema_version: 1,
                id: id.to_owned(),
                version: "1.0.0".to_owned(),
                layer: "test".to_owned(),
                coverage: "C".to_owned(),
                stage: "initial_submission".to_owned(),
                inherits: inherits.iter().map(|value| (*value).to_owned()).collect(),
                source_label: "Synthetic".to_owned(),
                source_label_en: "Synthetic".to_owned(),
                description: "Synthetic".to_owned(),
                description_en: "Synthetic".to_owned(),
                region: "test".to_owned(),
                category: "test".to_owned(),
                source_urls: Vec::new(),
                verified_at: "2026-08-24".to_owned(),
                selectable: false,
                submission_elements: Vec::new(),
                rules: vec![super::Rule {
                    id: rule_id.to_owned(),
                    field: "title".to_owned(),
                    operator: super::RuleOperator::Present,
                    value: None,
                    classification: super::RuleClassification::Must,
                    message: "Synthetic".to_owned(),
                    message_en: "Synthetic".to_owned(),
                }],
            },
            signature_verified: true,
        }
    }
}
