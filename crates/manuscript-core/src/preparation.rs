use crate::{
    rules_for, AppError, PackagePlan, PreparationItem, PreparationStatus, PreparationView, Project,
};
use sha2::{Digest, Sha256};

pub fn prepare(project: &Project) -> Result<PreparationView, AppError> {
    let target = project
        .target
        .as_ref()
        .ok_or_else(|| AppError::new("TARGET_REQUIRED", true).recover("choose_journal"))?;
    let rules = rules_for(&target.journal_id)?;
    let editable_evidence = rules
        .first()
        .map(|rule| rule.evidence.clone())
        .ok_or_else(|| AppError::new("RULES_UNVERIFIED", true))?;
    let mut blockers = Vec::new();
    let mut warnings = Vec::new();
    let mut ready_items = Vec::new();
    for rule in rules {
        let fact_present = match rule.fact_key.as_deref() {
            Some(key) => {
                fact_confirmed(project, &rule.id, key)?
                    && fact_matches_constraint(project, key, rule.constraint.as_ref())
            }
            None => false,
        };
        let mut anonymity_issue = None;
        let file_present = if let Some(kind) = rule.required_file_kind.as_deref() {
            let material = project
                .materials
                .iter()
                .find(|material| material.included && material.kind == kind);
            if kind == "anonymized_manuscript" {
                if let Some(material) = material {
                    let expected =
                        crate::documents::anonymity_identity_context_hash(&project.facts)?;
                    match material.anonymity_check.as_ref() {
                        Some(check)
                            if check.identity_context_hash == expected
                                && check.detected_categories.is_empty() =>
                        {
                            true
                        }
                        Some(check) if check.identity_context_hash == expected => {
                            anonymity_issue = Some(check.detected_categories.clone());
                            false
                        }
                        _ => false,
                    }
                } else {
                    false
                }
            } else {
                material.is_some()
            }
        } else {
            false
        };
        let present = fact_present || file_present;
        let description = anonymity_issue.map_or(rule.description, |categories| {
            let categories_zh = localized_identity_categories(&categories, true);
            let categories_en = localized_identity_categories(&categories, false);
            crate::LocalizedText::new(
                format!("本地检查发现匿名稿仍包含与已确认信息相同的内容：{categories_zh}。请在外部编辑后重新选择；系统不会自动删除。"),
                format!("The local check found content matching confirmed identity information: {categories_en}. Edit the file externally and choose it again; the app will not remove it automatically."),
            )
        });
        let item = PreparationItem {
            requirement_id: rule.id,
            label: rule.label,
            description,
            action: if rule.fact_key.is_some() {
                "fill".into()
            } else {
                "choose_file".into()
            },
            status: if present {
                "ready".into()
            } else {
                "missing".into()
            },
            required: rule.required,
            evidence: rule.evidence,
        };
        if present {
            ready_items.push(item);
        } else if item.required {
            blockers.push(item);
        } else {
            warnings.push(item);
        }
    }
    if project.active_source.format == "pdf" {
        let editable_present = project.materials.iter().any(|material| {
            material.included
                && matches!(
                    material.kind.as_str(),
                    "editable_manuscript" | "anonymized_manuscript"
                )
        });
        let item = PreparationItem {
            requirement_id: "local.editable-manuscript".into(),
            label: crate::LocalizedText::new(
                "作者核对的可编辑主稿 DOCX",
                "Author-verified editable manuscript DOCX",
            ),
            description: crate::LocalizedText::new(
                "PDF 可用于本地期刊匹配和草稿检查；正式投稿包还需要作者核对的可编辑 DOCX，系统不会把 PDF 反向转换或伪装为 Word 文件。",
                "The PDF can be used for local journal matching and draft review. A final package also needs an author-verified editable DOCX; the app will not reverse-convert or disguise the PDF as a Word file.",
            ),
            action: "choose_file".into(),
            status: if editable_present {
                "ready".into()
            } else {
                "missing".into()
            },
            required: true,
            evidence: editable_evidence,
        };
        if editable_present {
            ready_items.push(item);
        } else {
            blockers.push(item);
        }
    }
    let context_hash = context_hash(project)?;
    let status = if blockers.is_empty() {
        PreparationStatus::ReadyToExport
    } else if project.facts.title.is_some() {
        PreparationStatus::DraftReady
    } else {
        PreparationStatus::NeedsInput
    };
    let allowed_actions = match status {
        PreparationStatus::NeedsInput => vec!["save_progress".into()],
        PreparationStatus::DraftReady => vec!["build_draft".into()],
        PreparationStatus::ReadyToExport => vec!["build_draft".into(), "build_final".into()],
    };
    let mut capabilities = vec![
        "generate_author_tools".into(),
        "archive_submission_files".into(),
    ];
    if project
        .materials
        .iter()
        .any(|material| material.included && material.kind == "anonymized_manuscript")
    {
        capabilities.push("use_author_verified_anonymized_manuscript".into());
    } else if project
        .materials
        .iter()
        .any(|material| material.included && material.kind == "editable_manuscript")
    {
        capabilities.push("use_author_verified_editable_manuscript".into());
    } else if project.active_source.format == "pdf" {
        capabilities.push("copy_pdf_for_draft_review".into());
    } else {
        capabilities.push("copy_source_unchanged".into());
    }
    let package_plan = PackagePlan {
        id: format!("plan-{}", &context_hash[..16]),
        context_hash: context_hash.clone(),
        file_plan: crate::compiler::planned_package_files(project),
        transform_plan: Vec::new(),
        capabilities,
        status: if blockers.is_empty() {
            "ready".into()
        } else {
            "blocked".into()
        },
    };
    Ok(PreparationView {
        project_id: project.id.clone(),
        revision: project.revision,
        context_hash,
        status,
        blockers,
        warnings,
        ready_items,
        allowed_actions,
        package_plan,
    })
}

fn localized_identity_categories(categories: &[String], chinese: bool) -> String {
    categories
        .iter()
        .map(|category| match (category.as_str(), chinese) {
            ("authors", true) => "作者姓名",
            ("affiliations", true) => "作者单位",
            ("corresponding_email", true) => "通讯作者邮箱",
            ("authors", false) => "author names",
            ("affiliations", false) => "affiliations",
            ("corresponding_email", false) => "corresponding-author email",
            (_, true) => "身份信息",
            (_, false) => "identity information",
        })
        .collect::<Vec<_>>()
        .join(if chinese { "、" } else { ", " })
}

pub(crate) fn context_hash(project: &Project) -> Result<String, AppError> {
    let mut normalized = project.clone();
    normalized.updated_at_unix_ms = 0;
    let encoded =
        serde_json::to_vec(&normalized).map_err(|_| AppError::new("PROJECT_INVALID", false))?;
    Ok(hex::encode(Sha256::digest(encoded)))
}

fn fact_matches_constraint(
    project: &Project,
    key: &str,
    constraint: Option<&crate::RequirementConstraint>,
) -> bool {
    let Some(value) = crate::projects::confirmed_fact_value(&project.facts, key) else {
        return false;
    };
    let Some(constraint) = constraint else {
        return true;
    };
    let items = value.split('\u{1f}').collect::<Vec<_>>();
    if constraint
        .min_items
        .is_some_and(|minimum| items.len() < minimum)
        || constraint
            .max_items
            .is_some_and(|maximum| items.len() > maximum)
        || constraint
            .max_chars_per_item
            .is_some_and(|maximum| items.iter().any(|item| item.chars().count() > maximum))
        || constraint
            .max_words
            .is_some_and(|maximum| value.split_whitespace().count() > maximum)
    {
        return false;
    }
    true
}

fn fact_confirmed(project: &Project, requirement_id: &str, key: &str) -> Result<bool, AppError> {
    let Some(target) = project.target.as_ref() else {
        return Ok(false);
    };
    let Some(value) = crate::projects::confirmed_fact_value(&project.facts, key) else {
        return Ok(false);
    };
    let expected = crate::projects::author_decision_context_hash(
        &project.active_source.sha256,
        &target.rules_hash,
        requirement_id,
        &value,
    )?;
    Ok(project.author_decisions.iter().any(|decision| {
        decision.requirement_id == requirement_id
            && decision.value == value
            && decision.context_hash == expected
    }))
}
