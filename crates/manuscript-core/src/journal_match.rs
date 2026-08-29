use crate::StructureReport;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub const JOURNAL_MATCH_SCHEMA_VERSION: u32 = 1;
pub const JOURNAL_MATCH_ALGORITHM_VERSION: &str = "local-fit-v1.0";
pub const JOURNAL_CATALOG_VERSION: &str = "computer-ai-2025.1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ResearchTopic {
    Auto,
    GeneralAi,
    MachineLearning,
    ComputerVision,
    NaturalLanguageProcessing,
    DataMining,
    SoftwareSystems,
    RoboticsControl,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ArticleTypePreference {
    Auto,
    Research,
    Review,
    Application,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PublicationLanguagePreference {
    Auto,
    Chinese,
    English,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TargetStrategy {
    Reach,
    Balanced,
    Pragmatic,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum OpenAccessPreference {
    NoPreference,
    Prefer,
    Require,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JournalMatchPreferences {
    pub topic: ResearchTopic,
    pub article_type: ArticleTypePreference,
    pub language: PublicationLanguagePreference,
    pub target_strategy: TargetStrategy,
    pub open_access: OpenAccessPreference,
}

impl Default for JournalMatchPreferences {
    fn default() -> Self {
        Self {
            topic: ResearchTopic::Auto,
            article_type: ArticleTypePreference::Auto,
            language: PublicationLanguagePreference::Auto,
            target_strategy: TargetStrategy::Balanced,
            open_access: OpenAccessPreference::NoPreference,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum JournalRegion {
    Domestic,
    International,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JournalFitScores {
    pub topic_scope: u8,
    pub article_type: u8,
    pub content_readiness: u8,
    pub language: u8,
    pub target_level: u8,
    pub open_access: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JournalRecommendation {
    pub id: String,
    pub name: String,
    pub name_en: String,
    pub region: JournalRegion,
    pub publisher: String,
    pub rank_system: String,
    pub rank_tier: String,
    pub overall_fit: u8,
    pub scores: JournalFitScores,
    pub reasons: Vec<String>,
    pub ranking_source_url: String,
    pub homepage_url: String,
    pub open_access_status: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JournalRecommendationRun {
    pub schema_version: u32,
    pub run_id: String,
    pub workspace_id: String,
    pub manuscript_version: u32,
    pub manuscript_hash: String,
    pub algorithm_version: String,
    pub catalog_version: String,
    pub catalog_verified_date: String,
    pub inferred_topic: ResearchTopic,
    pub topic_basis: String,
    pub maturity_score: u8,
    pub preferences: JournalMatchPreferences,
    pub domestic: Vec<JournalRecommendation>,
    pub international: Vec<JournalRecommendation>,
    pub school_rule_status: String,
    pub limitations: Vec<String>,
    pub external_transmission: String,
}

#[derive(Clone, Copy)]
struct Candidate {
    id: &'static str,
    name: &'static str,
    name_en: &'static str,
    region: JournalRegion,
    publisher: &'static str,
    tier: &'static str,
    level: u8,
    topics: &'static [ResearchTopic],
    language: PublicationLanguagePreference,
    article_types: &'static [ArticleTypePreference],
    oa: &'static str,
    homepage: &'static str,
}

const RANK_DOMESTIC: &str = "https://www.ccf.org.cn/ccftjgjxskwml/";
const RANK_INTERNATIONAL: &str = "https://www.ccf.org.cn/Academic_Evaluation/AI/";

const CANDIDATES: &[Candidate] = &[
    Candidate {
        id: "cjc",
        name: "计算机学报",
        name_en: "Chinese Journal of Computers",
        region: JournalRegion::Domestic,
        publisher: "中国科学院计算技术研究所 / 中国计算机学会",
        tier: "T1",
        level: 3,
        topics: &[
            ResearchTopic::GeneralAi,
            ResearchTopic::SoftwareSystems,
            ResearchTopic::DataMining,
        ],
        language: PublicationLanguagePreference::Chinese,
        article_types: &[
            ArticleTypePreference::Research,
            ArticleTypePreference::Review,
        ],
        oa: "verify",
        homepage: "https://cjc.ict.ac.cn/",
    },
    Candidate {
        id: "crad",
        name: "计算机研究与发展",
        name_en: "Journal of Computer Research and Development",
        region: JournalRegion::Domestic,
        publisher: "中国科学院计算技术研究所 / 中国计算机学会",
        tier: "T1",
        level: 3,
        topics: &[
            ResearchTopic::GeneralAi,
            ResearchTopic::DataMining,
            ResearchTopic::SoftwareSystems,
        ],
        language: PublicationLanguagePreference::Chinese,
        article_types: &[
            ArticleTypePreference::Research,
            ArticleTypePreference::Review,
        ],
        oa: "verify",
        homepage: "https://crad.ict.ac.cn/",
    },
    Candidate {
        id: "jos",
        name: "软件学报",
        name_en: "Journal of Software",
        region: JournalRegion::Domestic,
        publisher: "中国科学院软件研究所 / 中国计算机学会",
        tier: "T1",
        level: 3,
        topics: &[
            ResearchTopic::SoftwareSystems,
            ResearchTopic::GeneralAi,
            ResearchTopic::DataMining,
        ],
        language: PublicationLanguagePreference::Chinese,
        article_types: &[
            ArticleTypePreference::Research,
            ArticleTypePreference::Review,
        ],
        oa: "open",
        homepage: "https://jos.org.cn/",
    },
    Candidate {
        id: "aau",
        name: "自动化学报",
        name_en: "Acta Automatica Sinica",
        region: JournalRegion::Domestic,
        publisher: "中国科学院自动化研究所 / 中国自动化学会",
        tier: "T1",
        level: 3,
        topics: &[
            ResearchTopic::RoboticsControl,
            ResearchTopic::GeneralAi,
            ResearchTopic::MachineLearning,
        ],
        language: PublicationLanguagePreference::Chinese,
        article_types: &[
            ArticleTypePreference::Research,
            ArticleTypePreference::Application,
        ],
        oa: "verify",
        homepage: "https://www.aas.net.cn/",
    },
    Candidate {
        id: "jcip",
        name: "中文信息学报",
        name_en: "Journal of Chinese Information Processing",
        region: JournalRegion::Domestic,
        publisher: "中国中文信息学会 / 中国科学院软件研究所",
        tier: "T1",
        level: 3,
        topics: &[
            ResearchTopic::NaturalLanguageProcessing,
            ResearchTopic::GeneralAi,
        ],
        language: PublicationLanguagePreference::Chinese,
        article_types: &[
            ArticleTypePreference::Research,
            ArticleTypePreference::Application,
        ],
        oa: "verify",
        homepage: "http://jcip.cipsc.org.cn/",
    },
    Candidate {
        id: "prai",
        name: "模式识别与人工智能",
        name_en: "Pattern Recognition and Artificial Intelligence",
        region: JournalRegion::Domestic,
        publisher: "中国自动化学会 / 国家智能计算机研究开发中心",
        tier: "T2",
        level: 2,
        topics: &[
            ResearchTopic::ComputerVision,
            ResearchTopic::MachineLearning,
            ResearchTopic::GeneralAi,
        ],
        language: PublicationLanguagePreference::Chinese,
        article_types: &[
            ArticleTypePreference::Research,
            ArticleTypePreference::Application,
        ],
        oa: "verify",
        homepage: "https://prai.hfcas.ac.cn/",
    },
    Candidate {
        id: "cjig",
        name: "中国图象图形学报",
        name_en: "Journal of Image and Graphics",
        region: JournalRegion::Domestic,
        publisher: "中国科学院空天信息创新研究院 / 中国图象图形学学会",
        tier: "T2",
        level: 2,
        topics: &[
            ResearchTopic::ComputerVision,
            ResearchTopic::MachineLearning,
        ],
        language: PublicationLanguagePreference::Chinese,
        article_types: &[
            ArticleTypePreference::Research,
            ArticleTypePreference::Application,
        ],
        oa: "open",
        homepage: "https://www.cjig.cn/",
    },
    Candidate {
        id: "jis",
        name: "智能系统学报",
        name_en: "CAAI Transactions on Intelligent Systems",
        region: JournalRegion::Domestic,
        publisher: "中国人工智能学会 / 哈尔滨工程大学",
        tier: "T2",
        level: 2,
        topics: &[
            ResearchTopic::GeneralAi,
            ResearchTopic::MachineLearning,
            ResearchTopic::RoboticsControl,
            ResearchTopic::NaturalLanguageProcessing,
        ],
        language: PublicationLanguagePreference::Chinese,
        article_types: &[
            ArticleTypePreference::Research,
            ArticleTypePreference::Application,
        ],
        oa: "open",
        homepage: "https://tis.hrbeu.edu.cn/",
    },
    Candidate {
        id: "ai",
        name: "Artificial Intelligence",
        name_en: "Artificial Intelligence",
        region: JournalRegion::International,
        publisher: "Elsevier",
        tier: "CCF A",
        level: 3,
        topics: &[ResearchTopic::GeneralAi, ResearchTopic::MachineLearning],
        language: PublicationLanguagePreference::English,
        article_types: &[
            ArticleTypePreference::Research,
            ArticleTypePreference::Review,
        ],
        oa: "hybrid",
        homepage: "https://www.sciencedirect.com/journal/artificial-intelligence",
    },
    Candidate {
        id: "tpami",
        name: "IEEE 模式分析与机器智能汇刊",
        name_en: "IEEE Transactions on Pattern Analysis and Machine Intelligence",
        region: JournalRegion::International,
        publisher: "IEEE",
        tier: "CCF A",
        level: 3,
        topics: &[
            ResearchTopic::ComputerVision,
            ResearchTopic::MachineLearning,
        ],
        language: PublicationLanguagePreference::English,
        article_types: &[ArticleTypePreference::Research],
        oa: "hybrid",
        homepage: "https://www.computer.org/csdl/journal/tp",
    },
    Candidate {
        id: "ijcv",
        name: "国际计算机视觉期刊",
        name_en: "International Journal of Computer Vision",
        region: JournalRegion::International,
        publisher: "Springer Nature",
        tier: "CCF A",
        level: 3,
        topics: &[
            ResearchTopic::ComputerVision,
            ResearchTopic::MachineLearning,
        ],
        language: PublicationLanguagePreference::English,
        article_types: &[
            ArticleTypePreference::Research,
            ArticleTypePreference::Review,
        ],
        oa: "hybrid",
        homepage: "https://link.springer.com/journal/11263",
    },
    Candidate {
        id: "jmlr",
        name: "机器学习研究期刊",
        name_en: "Journal of Machine Learning Research",
        region: JournalRegion::International,
        publisher: "JMLR",
        tier: "CCF A",
        level: 3,
        topics: &[ResearchTopic::MachineLearning, ResearchTopic::DataMining],
        language: PublicationLanguagePreference::English,
        article_types: &[ArticleTypePreference::Research],
        oa: "open",
        homepage: "https://www.jmlr.org/",
    },
    Candidate {
        id: "tacl",
        name: "计算语言学协会汇刊",
        name_en: "Transactions of the Association for Computational Linguistics",
        region: JournalRegion::International,
        publisher: "Association for Computational Linguistics",
        tier: "CCF B",
        level: 2,
        topics: &[
            ResearchTopic::NaturalLanguageProcessing,
            ResearchTopic::MachineLearning,
        ],
        language: PublicationLanguagePreference::English,
        article_types: &[ArticleTypePreference::Research],
        oa: "open",
        homepage: "https://transacl.org/",
    },
    Candidate {
        id: "tnnls",
        name: "IEEE 神经网络与学习系统汇刊",
        name_en: "IEEE Transactions on Neural Networks and Learning Systems",
        region: JournalRegion::International,
        publisher: "IEEE",
        tier: "CCF B",
        level: 2,
        topics: &[ResearchTopic::MachineLearning, ResearchTopic::GeneralAi],
        language: PublicationLanguagePreference::English,
        article_types: &[
            ArticleTypePreference::Research,
            ArticleTypePreference::Application,
        ],
        oa: "hybrid",
        homepage: "https://cis.ieee.org/publications/t-neural-networks-and-learning-systems",
    },
    Candidate {
        id: "pr",
        name: "模式识别",
        name_en: "Pattern Recognition",
        region: JournalRegion::International,
        publisher: "Elsevier",
        tier: "CCF B",
        level: 2,
        topics: &[
            ResearchTopic::ComputerVision,
            ResearchTopic::MachineLearning,
        ],
        language: PublicationLanguagePreference::English,
        article_types: &[
            ArticleTypePreference::Research,
            ArticleTypePreference::Application,
        ],
        oa: "hybrid",
        homepage: "https://www.sciencedirect.com/journal/pattern-recognition",
    },
    Candidate {
        id: "jair",
        name: "人工智能研究期刊",
        name_en: "Journal of Artificial Intelligence Research",
        region: JournalRegion::International,
        publisher: "AI Access Foundation",
        tier: "CCF B",
        level: 2,
        topics: &[
            ResearchTopic::GeneralAi,
            ResearchTopic::MachineLearning,
            ResearchTopic::RoboticsControl,
        ],
        language: PublicationLanguagePreference::English,
        article_types: &[ArticleTypePreference::Research],
        oa: "open",
        homepage: "https://www.jair.org/",
    },
    Candidate {
        id: "kbs",
        name: "知识系统",
        name_en: "Knowledge-Based Systems",
        region: JournalRegion::International,
        publisher: "Elsevier",
        tier: "CCF C",
        level: 1,
        topics: &[
            ResearchTopic::GeneralAi,
            ResearchTopic::DataMining,
            ResearchTopic::MachineLearning,
        ],
        language: PublicationLanguagePreference::English,
        article_types: &[
            ArticleTypePreference::Research,
            ArticleTypePreference::Application,
            ArticleTypePreference::Review,
        ],
        oa: "hybrid",
        homepage: "https://www.sciencedirect.com/journal/knowledge-based-systems",
    },
];

pub fn recommend_journals(
    report: &StructureReport,
    preferences: JournalMatchPreferences,
) -> JournalRecommendationRun {
    let (inferred_topic, topic_basis) = infer_topic(report);
    let topic = if preferences.topic == ResearchTopic::Auto {
        inferred_topic
    } else {
        preferences.topic
    };
    let article_type = if preferences.article_type == ArticleTypePreference::Auto {
        infer_article_type(report)
    } else {
        preferences.article_type
    };
    let maturity = maturity_score(report);
    let mut scored: Vec<_> = CANDIDATES
        .iter()
        .map(|candidate| score_candidate(*candidate, topic, article_type, maturity, &preferences))
        .collect();
    scored.sort_by(|a, b| {
        b.overall_fit
            .cmp(&a.overall_fit)
            .then_with(|| a.name_en.cmp(&b.name_en))
    });
    let domestic = scored
        .iter()
        .filter(|item| item.region == JournalRegion::Domestic)
        .take(3)
        .cloned()
        .collect();
    let international = scored
        .iter()
        .filter(|item| item.region == JournalRegion::International)
        .take(3)
        .cloned()
        .collect();
    let encoded = serde_json::to_vec(&(
        report.workspace_id.as_str(),
        report.source_content_hash.as_str(),
        report.source_snapshot_version,
        &preferences,
    ))
    .unwrap_or_default();
    let run_id = format!(
        "jmr-{}",
        hex::encode(Sha256::digest(encoded))
            .chars()
            .take(20)
            .collect::<String>()
    );
    JournalRecommendationRun {
        schema_version: JOURNAL_MATCH_SCHEMA_VERSION,
        run_id,
        workspace_id: report.workspace_id.clone(),
        manuscript_version: report.source_snapshot_version,
        manuscript_hash: report.source_content_hash.clone(),
        algorithm_version: JOURNAL_MATCH_ALGORITHM_VERSION.into(),
        catalog_version: JOURNAL_CATALOG_VERSION.into(),
        catalog_verified_date: "2025-04-16".into(),
        inferred_topic: topic,
        topic_basis: if preferences.topic == ResearchTopic::Auto {
            topic_basis
        } else {
            "author_adjusted".into()
        },
        maturity_score: maturity,
        preferences,
        domestic,
        international,
        school_rule_status: "not_configured_excluded_from_score".into(),
        limitations: vec![
            "适配分不是录用概率，也不替代期刊官网的最新投稿要求。".into(),
            "国内 T1/T2/T3 与国际 CCF A/B/C 是相互独立的目录，不做等级等同。".into(),
            "当前候选范围仅覆盖内置的计算机与人工智能期刊快照。".into(),
        ],
        external_transmission: "not_performed".into(),
    }
}

fn infer_topic(report: &StructureReport) -> (ResearchTopic, String) {
    let text = format!(
        "{} {} {}",
        report.title.as_deref().unwrap_or(""),
        report.abstract_text.as_deref().unwrap_or(""),
        report
            .sections
            .iter()
            .map(|s| s.heading.as_str())
            .collect::<Vec<_>>()
            .join(" ")
    )
    .to_lowercase();
    let groups = [
        (
            ResearchTopic::ComputerVision,
            [
                "vision",
                "image",
                "segmentation",
                "detection",
                "图像",
                "视觉",
                "分割",
                "识别",
            ]
            .as_slice(),
        ),
        (
            ResearchTopic::NaturalLanguageProcessing,
            [
                "language model",
                "nlp",
                "translation",
                "text generation",
                "语言模型",
                "自然语言",
                "翻译",
                "文本",
            ]
            .as_slice(),
        ),
        (
            ResearchTopic::RoboticsControl,
            ["robot", "control", "automation", "机器人", "控制", "自动化"].as_slice(),
        ),
        (
            ResearchTopic::SoftwareSystems,
            [
                "software",
                "system",
                "compiler",
                "database",
                "软件",
                "系统",
                "编译",
                "数据库",
            ]
            .as_slice(),
        ),
        (
            ResearchTopic::DataMining,
            [
                "data mining",
                "recommendation",
                "knowledge graph",
                "数据挖掘",
                "推荐系统",
                "知识图谱",
            ]
            .as_slice(),
        ),
        (
            ResearchTopic::MachineLearning,
            [
                "machine learning",
                "deep learning",
                "neural",
                "机器学习",
                "深度学习",
                "神经网络",
            ]
            .as_slice(),
        ),
    ];
    let mut best = (ResearchTopic::GeneralAi, 0usize);
    for (topic, words) in groups {
        let count = words.iter().filter(|word| text.contains(**word)).count();
        if count > best.1 {
            best = (topic, count);
        }
    }
    (
        best.0,
        if best.1 == 0 {
            "broad_default_low_confidence"
        } else if best.1 == 1 {
            "local_keywords_medium_confidence"
        } else {
            "local_keywords_high_confidence"
        }
        .into(),
    )
}

fn infer_article_type(report: &StructureReport) -> ArticleTypePreference {
    let text = format!(
        "{} {}",
        report.title.as_deref().unwrap_or(""),
        report.abstract_text.as_deref().unwrap_or("")
    )
    .to_lowercase();
    if ["review", "survey", "综述", "述评"]
        .iter()
        .any(|v| text.contains(v))
    {
        ArticleTypePreference::Review
    } else if [
        "application",
        "deployment",
        "case study",
        "应用",
        "部署",
        "工程",
    ]
    .iter()
    .any(|v| text.contains(v))
    {
        ArticleTypePreference::Application
    } else {
        ArticleTypePreference::Research
    }
}

fn maturity_score(report: &StructureReport) -> u8 {
    let mut score: i16 = 20;
    if report.title.is_some() {
        score += 10
    }
    if report.abstract_present {
        score += 15
    }
    if report.keywords_present {
        score += 5
    }
    if report.sections.len() >= 4 {
        score += 15
    }
    if report.figure_count + report.table_count > 0 {
        score += 10
    }
    if report.references_present {
        score += 15
    }
    if report.word_count >= 3000 {
        score += 10
    }
    score -= (report.warnings.len().min(4) as i16) * 3;
    score.clamp(0, 100) as u8
}

fn score_candidate(
    candidate: Candidate,
    topic: ResearchTopic,
    article_type: ArticleTypePreference,
    maturity: u8,
    preferences: &JournalMatchPreferences,
) -> JournalRecommendation {
    let topic_score = if candidate.topics.contains(&topic) {
        100
    } else if candidate.topics.contains(&ResearchTopic::GeneralAi)
        || topic == ResearchTopic::GeneralAi
    {
        70
    } else {
        30
    };
    let article_score = if candidate.article_types.contains(&article_type) {
        100
    } else {
        55
    };
    let threshold = match candidate.level {
        3 => 85,
        2 => 70,
        _ => 55,
    };
    let readiness = if maturity >= threshold {
        100
    } else {
        (100i16 - 2 * (threshold as i16 - maturity as i16)).clamp(20, 100) as u8
    };
    let language = match preferences.language {
        PublicationLanguagePreference::Auto => 100,
        chosen if chosen == candidate.language => 100,
        _ => 25,
    };
    let target = match preferences.target_strategy {
        TargetStrategy::Reach => [0, 45, 75, 100][candidate.level as usize],
        TargetStrategy::Balanced => [0, 70, 100, 80][candidate.level as usize],
        TargetStrategy::Pragmatic => [0, 100, 85, 55][candidate.level as usize],
    };
    let oa = match preferences.open_access {
        OpenAccessPreference::NoPreference => 100,
        OpenAccessPreference::Prefer => match candidate.oa {
            "open" => 100,
            "hybrid" => 80,
            _ => 60,
        },
        OpenAccessPreference::Require => match candidate.oa {
            "open" => 100,
            "hybrid" => 55,
            _ => 25,
        },
    };
    let scores = JournalFitScores {
        topic_scope: topic_score,
        article_type: article_score,
        content_readiness: readiness,
        language,
        target_level: target,
        open_access: oa,
    };
    let overall = ((topic_score as u32 * 35
        + article_score as u32 * 15
        + readiness as u32 * 20
        + language as u32 * 10
        + target as u32 * 15
        + oa as u32 * 5)
        / 100) as u8;
    let source = if candidate.region == JournalRegion::Domestic {
        RANK_DOMESTIC
    } else {
        RANK_INTERNATIONAL
    };
    JournalRecommendation {
        id: candidate.id.into(),
        name: candidate.name.into(),
        name_en: candidate.name_en.into(),
        region: candidate.region,
        publisher: candidate.publisher.into(),
        rank_system: if candidate.region == JournalRegion::Domestic {
            "CCF 中国计算机领域高质量科技期刊分级目录".into()
        } else {
            "CCF 推荐国际学术刊物目录（人工智能）".into()
        },
        rank_tier: candidate.tier.into(),
        overall_fit: overall,
        scores,
        reasons: vec![
            format!("主题范围适配 {} 分", topic_score),
            format!("当前稿件完备度适配 {} 分", readiness),
            format!("目标策略适配 {} 分", target),
        ],
        ranking_source_url: source.into(),
        homepage_url: candidate.homepage.into(),
        open_access_status: candidate.oa.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AnalysisQuality, SectionSummary};
    fn report(text: &str) -> StructureReport {
        StructureReport {
            analysis_version: 4,
            workspace_id: "test-workspace".into(),
            source_content_hash: "a".repeat(64),
            source_snapshot_version: 2,
            quality: AnalysisQuality::Complete,
            title: Some(text.into()),
            authors: vec!["Test Author".into()],
            abstract_present: true,
            abstract_text: Some(text.into()),
            keywords_present: true,
            sections: vec![
                SectionSummary {
                    level: 1,
                    heading: "Method".into()
                };
                5
            ],
            figure_count: 2,
            table_count: 1,
            references_present: true,
            declarations: vec![],
            page_count: Some(12),
            word_count: 5000,
            pdf_processing: None,
            warnings: vec![],
        }
    }
    #[test]
    fn returns_three_per_region() {
        let run = recommend_journals(
            &report("general artificial intelligence"),
            JournalMatchPreferences::default(),
        );
        assert_eq!(run.domestic.len(), 3);
        assert_eq!(run.international.len(), 3);
        assert!(run.domestic.iter().all(|j| j.overall_fit <= 100));
    }
    #[test]
    fn author_topic_adjustment_changes_recommendations() {
        let mut p = JournalMatchPreferences {
            topic: ResearchTopic::ComputerVision,
            ..JournalMatchPreferences::default()
        };
        let cv = recommend_journals(&report("general artificial intelligence"), p.clone());
        p.topic = ResearchTopic::NaturalLanguageProcessing;
        let nlp = recommend_journals(&report("general artificial intelligence"), p);
        assert_ne!(cv.run_id, nlp.run_id);
        assert!(cv.international.iter().any(|j| j.id == "tpami"));
        assert!(nlp.international.iter().any(|j| j.id == "tacl"));
        assert!(nlp.domestic.iter().any(|j| j.id == "jcip"));
    }
    #[test]
    fn same_inputs_are_deterministic() {
        let a = recommend_journals(
            &report("machine learning"),
            JournalMatchPreferences::default(),
        );
        let b = recommend_journals(
            &report("machine learning"),
            JournalMatchPreferences::default(),
        );
        assert_eq!(a, b);
    }
}
