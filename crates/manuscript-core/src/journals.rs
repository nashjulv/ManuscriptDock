use crate::{
    AppError, AuthorConstraints, ConstraintResult, ConstraintStatus, JournalCatalog,
    JournalRecommendation, JournalRecord, LocalizedText, RecommendationResult, RecommendationRole,
    RequirementRule,
};
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, HashSet};

const CATALOG_JSON: &str = include_str!("../journal-data/catalog.json");
const RULES_JSON: &str = include_str!("../journal-data/rules.json");
const RESOURCE_MANIFEST: &[u8] = include_bytes!("../journal-data/resource-manifest.json");
const RESOURCE_MANIFEST_SIGNATURE: &str = include_str!("../journal-data/resource-manifest.sig");
const RESOURCE_PUBLIC_KEY: &str = include_str!("../journal-data/resource-public-key.hex");

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ResourceManifest {
    schema_version: u32,
    bundle_id: String,
    bundle_version: String,
    resources: Vec<ResourceManifestEntry>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ResourceManifestEntry {
    path: String,
    sha256: String,
    license_record: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RuleBundle {
    schema_version: u32,
    packs: BTreeMap<String, RulePack>,
}

#[derive(Deserialize)]
struct RulePack {
    #[serde(default)]
    extends: Vec<String>,
    #[serde(default)]
    rules: Vec<RequirementRule>,
}

pub fn catalog() -> Result<JournalCatalog, AppError> {
    verify_resource_bundle()?;
    let catalog: JournalCatalog =
        serde_json::from_str(CATALOG_JSON).map_err(|_| AppError::new("CATALOG_INVALID", false))?;
    if catalog.schema_version != crate::CATALOG_SCHEMA_VERSION
        || catalog
            .journals
            .iter()
            .any(|journal| journal.id.is_empty() || journal.display_name.is_empty())
    {
        return Err(AppError::new("CATALOG_INVALID", false));
    }
    Ok(catalog)
}

pub fn find_journals(query: &str) -> Result<Vec<JournalRecord>, AppError> {
    let normalized = query.trim().to_lowercase();
    let mut journals = catalog()?.journals;
    journals.retain(|journal| {
        normalized.is_empty()
            || journal.id.to_lowercase().contains(&normalized)
            || journal.display_name.to_lowercase().contains(&normalized)
            || chinese_search_name(&journal.id).contains(&normalized)
            || journal
                .aliases
                .iter()
                .any(|alias| alias.to_lowercase().contains(&normalized))
            || journal
                .issn
                .as_deref()
                .is_some_and(|issn| issn.replace('-', "").contains(&normalized.replace('-', "")))
            || journal
                .eissn
                .as_deref()
                .is_some_and(|issn| issn.replace('-', "").contains(&normalized.replace('-', "")))
    });
    journals.sort_by(|left, right| {
        left.display_name
            .cmp(&right.display_name)
            .then(left.id.cmp(&right.id))
    });
    Ok(journals)
}

// Search translations only; these do not change the signed publisher records or submission languages.
fn chinese_search_name(id: &str) -> &'static str {
    match id {
        "elsevier-artificial-intelligence" => "人工智能",
        "elsevier-expert-systems-with-applications" => "专家系统及其应用专家系统与应用",
        "elsevier-knowledge-based-systems" => "基于知识的系统知识系统",
        "ieee-tpami" => "模式分析与机器智能汇刊",
        "jmlr" => "机器学习研究杂志",
        "tacl" => "计算语言学汇刊",
        "elsevier-pattern-recognition" => "模式识别",
        "jair" => "人工智能研究杂志",
        "elsevier-engineering-applications-of-ai" => "人工智能工程应用",
        "ieee-tnnls" => "神经网络与学习系统汇刊",
        "nature-machine-intelligence" => "自然机器智能",
        _ => "",
    }
}

pub fn rules_for(journal_id: &str) -> Result<Vec<RequirementRule>, AppError> {
    verify_resource_bundle()?;
    rules_from_bytes(journal_id, RULES_JSON.as_bytes())
}

fn rules_from_bytes(
    journal_id: &str,
    rules_bytes: &[u8],
) -> Result<Vec<RequirementRule>, AppError> {
    let bundle: RuleBundle =
        serde_json::from_slice(rules_bytes).map_err(|_| AppError::new("RULES_INVALID", false))?;
    if bundle.schema_version != 1 {
        return Err(AppError::new("RULES_INVALID", false));
    }
    if !bundle.packs.contains_key(journal_id) {
        return Err(AppError::new("RULES_UNVERIFIED", true)
            .param("journalId", journal_id)
            .recover("choose_supported_journal"));
    }
    let mut visiting = HashSet::new();
    let mut composed = HashSet::new();
    let mut rules = Vec::new();
    compose_rule_pack(
        journal_id,
        &bundle.packs,
        &mut visiting,
        &mut composed,
        &mut rules,
    )?;
    if rules.is_empty() {
        return Err(AppError::new("RULES_INVALID", false));
    }
    let mut rule_ids = HashSet::new();
    if rules
        .iter()
        .any(|rule| rule.id.trim().is_empty() || !rule_ids.insert(rule.id.clone()))
    {
        return Err(AppError::new("RULES_INVALID", false));
    }
    Ok(rules)
}

fn compose_rule_pack(
    pack_id: &str,
    packs: &BTreeMap<String, RulePack>,
    visiting: &mut HashSet<String>,
    composed: &mut HashSet<String>,
    output: &mut Vec<RequirementRule>,
) -> Result<(), AppError> {
    if composed.contains(pack_id) {
        return Ok(());
    }
    if !visiting.insert(pack_id.to_owned()) {
        return Err(AppError::new("RULES_INVALID", false));
    }
    let pack = packs
        .get(pack_id)
        .ok_or_else(|| AppError::new("RULES_INVALID", false))?;
    for parent in &pack.extends {
        compose_rule_pack(parent, packs, visiting, composed, output)?;
    }
    output.extend(pack.rules.iter().cloned());
    visiting.remove(pack_id);
    composed.insert(pack_id.to_owned());
    Ok(())
}

fn verify_resource_bundle() -> Result<(), AppError> {
    verify_resource_bundle_parts(
        RESOURCE_MANIFEST,
        RESOURCE_MANIFEST_SIGNATURE,
        RESOURCE_PUBLIC_KEY,
        CATALOG_JSON.as_bytes(),
        RULES_JSON.as_bytes(),
    )
}

fn verify_resource_bundle_parts(
    manifest_bytes: &[u8],
    signature_hex: &str,
    public_key_hex: &str,
    catalog_bytes: &[u8],
    rules_bytes: &[u8],
) -> Result<(), AppError> {
    let invalid = || AppError::new("RESOURCE_BUNDLE_INVALID", false);
    let public_key: [u8; 32] = hex::decode(public_key_hex.trim())
        .map_err(|_| invalid())?
        .try_into()
        .map_err(|_| invalid())?;
    let verifying_key = VerifyingKey::from_bytes(&public_key).map_err(|_| invalid())?;
    let signature_bytes = hex::decode(signature_hex.trim()).map_err(|_| invalid())?;
    let signature = Signature::from_slice(&signature_bytes).map_err(|_| invalid())?;
    verifying_key
        .verify(manifest_bytes, &signature)
        .map_err(|_| invalid())?;
    let manifest: ResourceManifest =
        serde_json::from_slice(manifest_bytes).map_err(|_| invalid())?;
    if manifest.schema_version != 1
        || manifest.bundle_id.trim().is_empty()
        || manifest.bundle_version.trim().is_empty()
        || manifest.resources.len() != 2
    {
        return Err(invalid());
    }
    let expected = BTreeMap::from([("catalog.json", catalog_bytes), ("rules.json", rules_bytes)]);
    let mut seen = HashSet::new();
    for resource in manifest.resources {
        let bytes = expected.get(resource.path.as_str()).ok_or_else(invalid)?;
        if !seen.insert(resource.path.clone())
            || resource.license_record.trim().is_empty()
            || resource.sha256 != hex::encode(Sha256::digest(bytes))
        {
            return Err(invalid());
        }
    }
    if seen.len() != expected.len() {
        return Err(invalid());
    }
    Ok(())
}

pub fn rules_hash_for(journal_id: &str) -> Result<String, AppError> {
    let rules = rules_for(journal_id)?;
    let encoded = serde_json::to_vec(&rules).map_err(|_| AppError::new("RULES_INVALID", false))?;
    Ok(hex::encode(Sha256::digest(encoded)))
}

pub fn recommend(
    constraints: &AuthorConstraints,
    facts: &crate::DocumentFacts,
) -> Result<RecommendationResult, AppError> {
    let catalog = catalog()?;
    let version = catalog.data_version.clone();
    let mut eligible = Vec::<(
        i32,
        u8,
        JournalRecord,
        Vec<ConstraintResult>,
        Vec<LocalizedText>,
        Vec<LocalizedText>,
    )>::new();
    let mut unknown = Vec::new();
    let mut excluded = 0usize;
    for journal in catalog
        .journals
        .into_iter()
        .filter(|journal| journal.status == "active")
    {
        let mut results = Vec::new();
        let mut score = 0i32;
        let mut known = 0u8;
        let mut total = 0u8;
        let mut reasons = Vec::new();
        let mut risks = Vec::new();
        if let Some(language) = &constraints.required_language {
            total += 1;
            known += 1;
            let pass = journal
                .languages
                .iter()
                .any(|value| value.eq_ignore_ascii_case(language));
            results.push(constraint(
                "language",
                Some(pass),
                "语言要求相符",
                "Language requirement matches",
                "期刊语言不符合要求",
                "Journal language does not match",
            ));
            if pass {
                score += 25;
            } else {
                excluded += 1;
                continue;
            }
        }
        if let Some(article_type) = constraints
            .article_type
            .as_deref()
            .or(facts.article_type.as_deref())
        {
            total += 1;
            known += 1;
            let pass = journal
                .article_types
                .iter()
                .any(|value| value == article_type);
            results.push(constraint(
                "article_type",
                Some(pass),
                "接收该文章类型",
                "Accepts this article type",
                "不接收该文章类型",
                "Does not accept this article type",
            ));
            if pass {
                score += 20;
            } else {
                excluded += 1;
                continue;
            }
        }
        if !constraints.required_indexing.is_empty() {
            total += 1;
            known += 1;
            let pass = constraints.required_indexing.iter().all(|required| {
                journal
                    .indexing
                    .iter()
                    .any(|value| value.eq_ignore_ascii_case(required))
            });
            results.push(constraint(
                "indexing",
                Some(pass),
                "满足名单要求",
                "Meets indexing requirements",
                "不满足名单要求",
                "Does not meet indexing requirements",
            ));
            if pass {
                score += 20;
            } else {
                excluded += 1;
                continue;
            }
        }
        if constraints.require_open_access {
            total += 1;
            known += 1;
            let pass = matches!(journal.publication_route.as_str(), "open_access" | "hybrid");
            results.push(constraint(
                "open_access",
                Some(pass),
                "提供开放获取路径",
                "Offers an open-access route",
                "没有已核验的开放获取路径",
                "No verified open-access route",
            ));
            if pass {
                score += 10;
            } else {
                excluded += 1;
                continue;
            }
        }
        if let Some(maximum) = constraints.maximum_apc {
            total += 1;
            let comparable = constraints
                .currency
                .as_deref()
                .zip(journal.apc_currency.as_deref())
                .filter(|(left, right)| left.eq_ignore_ascii_case(right));
            let status =
                comparable.and_then(|_| journal.apc_amount.map(|amount| amount <= maximum));
            results.push(constraint(
                "budget",
                status,
                "费用在预算内",
                "Fee is within budget",
                "费用超过预算",
                "Fee exceeds budget",
            ));
            if let Some(pass) = status {
                known += 1;
                if pass {
                    score += 15;
                } else {
                    excluded += 1;
                    continue;
                }
            } else {
                risks.push(LocalizedText::new(
                    "费用币种或金额尚需核对",
                    "Fee currency or amount needs verification",
                ));
            }
        }
        let topics = constraints
            .topic_keywords
            .iter()
            .chain(facts.keywords.iter())
            .map(|v| v.to_lowercase())
            .collect::<Vec<_>>();
        let topic_hits = topics
            .iter()
            .filter(|topic| {
                journal.topics.iter().any(|candidate| {
                    candidate.to_lowercase().contains(topic.as_str())
                        || topic.contains(&candidate.to_lowercase())
                })
            })
            .count();
        if topic_hits > 0 {
            score += (topic_hits.min(4) * 8) as i32;
            reasons.push(LocalizedText::new(
                "论文关键词与期刊范围有明确交集",
                "Manuscript keywords overlap the journal scope",
            ));
        }
        if journal.generation_coverage == "supported" {
            score += 8;
            reasons.push(LocalizedText::new(
                "已内置首批投稿包规则",
                "Pilot package rules are built in",
            ));
        }
        if constraints.prefer_fast_first_decision {
            if let Some(days) = journal.first_decision_days {
                score += 12_i32.saturating_sub((days / 15).min(12) as i32);
                reasons.push(LocalizedText::new(
                    format!("已记录首轮决定约 {days} 天"),
                    format!("Recorded first decision is about {days} days"),
                ));
            } else {
                risks.push(LocalizedText::new(
                    "首轮决定时效尚无可核验数据，未用于排序加分",
                    "No verified first-decision timing is available, so it did not improve the ranking",
                ));
            }
        }
        let coverage = if total == 0 {
            100
        } else {
            ((known as u16 * 100) / total as u16) as u8
        };
        if journal.generation_coverage != "supported"
            || results
                .iter()
                .any(|result| result.status == ConstraintStatus::Unknown)
        {
            unknown.push(journal);
        } else {
            eligible.push((score, coverage, journal, results, reasons, risks));
        }
    }
    eligible.sort_by(|left, right| {
        right
            .0
            .cmp(&left.0)
            .then(right.1.cmp(&left.1))
            .then(left.2.id.cmp(&right.2.id))
    });
    let roles = [
        RecommendationRole::BestOverallFit,
        RecommendationRole::AmbitiousOption,
        RecommendationRole::LessPreparationNeeded,
    ];
    let recommendations = eligible.into_iter().take(3).enumerate().map(|(index, (_, coverage, journal, constraints, mut reasons, risks))| { if reasons.is_empty() { reasons.push(LocalizedText::new("满足当前已知硬条件", "Meets the currently known hard constraints")); } let preparation = vec![LocalizedText::new("按内置试点规则补齐作者事实并生成核对材料", "Complete author facts and generate review materials from the built-in pilot rules")]; JournalRecommendation { journal, role: roles[index].clone(), reasons, risks, preparation, constraints, evidence_coverage: coverage } }).collect();
    Ok(RecommendationResult {
        run_id: format!("recommendation-{}", uuid::Uuid::new_v4()),
        catalog_version: version,
        recommendations,
        needs_verification: unknown,
        excluded_count: excluded,
    })
}

fn constraint(
    id: &str,
    pass: Option<bool>,
    pass_zh: &str,
    pass_en: &str,
    fail_zh: &str,
    fail_en: &str,
) -> ConstraintResult {
    match pass {
        Some(true) => ConstraintResult {
            constraint_id: id.into(),
            status: ConstraintStatus::Pass,
            explanation: LocalizedText::new(pass_zh, pass_en),
        },
        Some(false) => ConstraintResult {
            constraint_id: id.into(),
            status: ConstraintStatus::Fail,
            explanation: LocalizedText::new(fail_zh, fail_en),
        },
        None => ConstraintResult {
            constraint_id: id.into(),
            status: ConstraintStatus::Unknown,
            explanation: LocalizedText::new(
                "证据不足，尚需核对",
                "Insufficient evidence; verification is needed",
            ),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn chinese_names_abbreviations_and_both_issns_find_targets() {
        for query in ["专家系统", "ESWA", "0957-4174", "18736793"] {
            assert!(find_journals(query)
                .unwrap()
                .iter()
                .any(|j| j.id == "elsevier-expert-systems-with-applications"));
        }
        assert!(!find_journals("人工智能").unwrap().is_empty());
        assert!(find_journals("不存在的期刊").unwrap().is_empty());
    }
    #[test]
    fn catalog_is_valid_and_has_supported_rules() {
        let catalog = catalog().unwrap();
        assert!(catalog.journals.len() >= 10);
        for journal in catalog
            .journals
            .iter()
            .filter(|v| v.generation_coverage == "supported")
        {
            assert!(!rules_for(&journal.id).unwrap().is_empty());
        }
        assert_eq!(
            find_journals("elsevier-artificial-intelligence")
                .unwrap()
                .first()
                .map(|journal| journal.display_name.as_str()),
            Some("Artificial Intelligence")
        );
    }
    #[test]
    fn bundled_resources_require_a_valid_signature_and_matching_hashes() {
        verify_resource_bundle().unwrap();
        let mut changed_catalog = CATALOG_JSON.as_bytes().to_vec();
        changed_catalog.push(b' ');
        assert_eq!(
            verify_resource_bundle_parts(
                RESOURCE_MANIFEST,
                RESOURCE_MANIFEST_SIGNATURE,
                RESOURCE_PUBLIC_KEY,
                &changed_catalog,
                RULES_JSON.as_bytes(),
            )
            .unwrap_err()
            .code,
            "RESOURCE_BUNDLE_INVALID"
        );
        assert_eq!(
            verify_resource_bundle_parts(
                RESOURCE_MANIFEST,
                "00",
                RESOURCE_PUBLIC_KEY,
                CATALOG_JSON.as_bytes(),
                RULES_JSON.as_bytes(),
            )
            .unwrap_err()
            .code,
            "RESOURCE_BUNDLE_INVALID"
        );
    }
    #[test]
    fn rule_packs_compose_parents_and_reject_missing_or_cyclic_parents() {
        let rules =
            rules_from_bytes("elsevier-artificial-intelligence", RULES_JSON.as_bytes()).unwrap();
        assert_eq!(rules.len(), 12);
        assert_eq!(
            rules.first().map(|rule| rule.id.as_str()),
            Some("elsevier.authors")
        );
        let keyword_rule = rules.iter().find(|rule| rule.id == "ai.keywords").unwrap();
        assert_eq!(
            keyword_rule
                .constraint
                .as_ref()
                .and_then(|constraint| constraint.max_items),
            Some(10)
        );
        for invalid in [
            br#"{"schemaVersion":1,"packs":{"journal":{"extends":["missing"],"rules":[]}}}"#.as_slice(),
            br#"{"schemaVersion":1,"packs":{"journal":{"extends":["parent"],"rules":[]},"parent":{"extends":["journal"],"rules":[]}}}"#.as_slice(),
        ] {
            assert_eq!(
                rules_from_bytes("journal", invalid).unwrap_err().code,
                "RULES_INVALID"
            );
        }
    }
    #[test]
    fn recommendations_never_exceed_three() {
        let result = recommend(
            &AuthorConstraints::default(),
            &crate::DocumentFacts {
                article_type: Some("research_article".into()),
                ..Default::default()
            },
        )
        .unwrap();
        assert!(result.recommendations.len() <= 3);
        assert!(result
            .recommendations
            .iter()
            .all(|item| item.journal.generation_coverage == "supported"));
        assert!(result
            .needs_verification
            .iter()
            .any(|journal| journal.generation_coverage == "catalog_only"));
    }
}
