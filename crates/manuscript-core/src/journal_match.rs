use crate::{
    JournalDirectoryCatalog, JournalDirectoryEvidence, JournalMetricScheme, StructureReport,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub const JOURNAL_MATCH_SCHEMA_VERSION: u32 = 5;
pub const JOURNAL_MATCH_ALGORITHM_VERSION: &str = "local-fit-v1.4";
pub const JOURNAL_CATALOG_VERSION: &str = "computer-ai-2025.1";
pub const JOURNAL_PROFILE_SCHEMA_VERSION: u32 = 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ManuscriptPurpose {
    DegreeRequirement,
    Graduation,
    ProfessionalTitle,
    ProjectCompletion,
    AcademicCommunication,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum InstitutionRuleStatus {
    SearchRequired,
    CandidateSourcesFound,
    Verified,
    NoOfficialRuleFound,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstitutionRuleEvidence {
    pub status: InstitutionRuleStatus,
    pub rule_set_id: Option<String>,
    pub rule_set_version: Option<String>,
    pub source_urls: Vec<String>,
    pub verified_at: Option<String>,
    pub recognized_rank_tiers: Vec<String>,
    pub blocked_rank_tiers: Vec<String>,
    #[serde(default)]
    pub source_text_hash: Option<String>,
    #[serde(default)]
    pub source_kind: Option<String>,
    #[serde(default)]
    pub extraction_model: Option<String>,
    #[serde(default)]
    pub extracted_conditions: Vec<String>,
    #[serde(default)]
    pub minimum_cas_partition: Option<u8>,
    #[serde(default)]
    pub requires_cas_top: bool,
    #[serde(default)]
    pub author_attested_official: bool,
    #[serde(default)]
    pub cas_partition_data_status: Option<String>,
}

impl Default for InstitutionRuleEvidence {
    fn default() -> Self {
        Self {
            status: InstitutionRuleStatus::SearchRequired,
            rule_set_id: None,
            rule_set_version: None,
            source_urls: Vec::new(),
            verified_at: None,
            recognized_rank_tiers: Vec::new(),
            blocked_rank_tiers: Vec::new(),
            source_text_hash: None,
            source_kind: None,
            extraction_model: None,
            extracted_conditions: Vec::new(),
            minimum_cas_partition: None,
            requires_cas_top: false,
            author_attested_official: false,
            cas_partition_data_status: Some("licensed_official_api_not_configured".into()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JournalRecommendationProfileInput {
    pub author_name: String,
    pub institution: String,
    pub specialty: String,
    pub manuscript_purpose: ManuscriptPurpose,
    pub submission_deadline: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JournalRecommendationProfile {
    pub schema_version: u32,
    pub profile_id: String,
    pub profile_version: u32,
    pub workspace_id: String,
    pub author_name: String,
    pub institution: String,
    pub specialty: String,
    pub manuscript_purpose: ManuscriptPurpose,
    pub submission_deadline: String,
    pub saved_unix_ms: u64,
    pub institution_rule_evidence: InstitutionRuleEvidence,
    pub external_transmission: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JournalRecommendationProfileSummary {
    pub profile_id: String,
    pub profile_version: u32,
    pub institution: String,
    pub specialty: String,
    pub manuscript_purpose: ManuscriptPurpose,
    pub submission_deadline: String,
}

impl JournalRecommendationProfileInput {
    pub fn normalized(mut self) -> Result<Self, &'static str> {
        self.author_name = self.author_name.trim().to_owned();
        self.institution = self.institution.trim().to_owned();
        self.specialty = self.specialty.trim().to_owned();
        self.submission_deadline = self.submission_deadline.trim().to_owned();
        if self.author_name.chars().count() > 120
            || self.institution.is_empty()
            || self.institution.chars().count() > 200
            || self.specialty.is_empty()
            || self.specialty.chars().count() > 160
            || parse_iso_date_days(&self.submission_deadline).is_none()
        {
            return Err("投稿背景档案不完整或字段格式无效");
        }
        Ok(self)
    }
}

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
    pub institution_rules: Option<u8>,
    pub topic_scope: u8,
    pub specialty_fit: u8,
    pub article_type: u8,
    pub content_readiness: u8,
    pub language: u8,
    pub target_level: u8,
    pub open_access: u8,
    pub purpose_fit: u8,
    pub time_feasibility: u8,
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
    pub estimated_submission_preparation_days: u32,
    pub deadline_status: String,
    pub institution_eligibility: String,
    pub scores: JournalFitScores,
    pub reasons: Vec<String>,
    pub ranking_source_url: String,
    pub homepage_url: String,
    pub open_access_status: String,
    #[serde(default)]
    pub directory_evidence: Vec<JournalDirectoryEvidence>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JournalRecommendationPortfolio {
    pub sprint: Vec<JournalRecommendation>,
    pub matching: Vec<JournalRecommendation>,
    pub safeguard: Vec<JournalRecommendation>,
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
    pub evaluated_unix_ms: u64,
    pub recommendation_profile: JournalRecommendationProfileSummary,
    pub deadline_days_remaining: u32,
    pub preferences: JournalMatchPreferences,
    pub domestic: JournalRecommendationPortfolio,
    pub international: JournalRecommendationPortfolio,
    pub school_rule_status: String,
    pub institution_directory_status: String,
    #[serde(default)]
    pub journal_directory_version: Option<String>,
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

struct ScoreContext<'a> {
    topic: ResearchTopic,
    specialty_topic: ResearchTopic,
    article_type: ArticleTypePreference,
    maturity: u8,
    deadline_days_remaining: u32,
    purpose: ManuscriptPurpose,
    institution_rules: &'a InstitutionRuleEvidence,
    preferences: &'a JournalMatchPreferences,
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
    profile: JournalRecommendationProfile,
    preferences: JournalMatchPreferences,
    evaluated_unix_ms: u64,
) -> JournalRecommendationRun {
    recommend_journals_with_directory(report, profile, preferences, evaluated_unix_ms, None)
}

pub fn recommend_journals_with_directory(
    report: &StructureReport,
    profile: JournalRecommendationProfile,
    preferences: JournalMatchPreferences,
    evaluated_unix_ms: u64,
    directory: Option<&JournalDirectoryCatalog>,
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
    let specialty_topic = infer_topic_from_text(&profile.specialty).0;
    let deadline_days_remaining =
        deadline_days_remaining(&profile.submission_deadline, evaluated_unix_ms).unwrap_or(0);
    let maturity = maturity_score(report);
    let score_context = ScoreContext {
        topic,
        specialty_topic,
        article_type,
        maturity,
        deadline_days_remaining,
        purpose: profile.manuscript_purpose,
        institution_rules: &profile.institution_rule_evidence,
        preferences: &preferences,
    };
    let mut scored: Vec<_> = CANDIDATES
        .iter()
        .map(|candidate| {
            let mut evidence = directory
                .map(|catalog| catalog.evidence_for_title(candidate.name_en))
                .unwrap_or_default();
            if evidence.is_empty() {
                evidence = directory
                    .map(|catalog| catalog.evidence_for_title(candidate.name))
                    .unwrap_or_default();
            }
            (
                *candidate,
                score_candidate(*candidate, &score_context, evidence),
            )
        })
        .collect();
    scored.sort_by(|a, b| {
        b.1.overall_fit
            .cmp(&a.1.overall_fit)
            .then_with(|| a.1.name_en.cmp(&b.1.name_en))
    });
    let domestic = build_recommendation_portfolio(&scored, JournalRegion::Domestic);
    let international = build_recommendation_portfolio(&scored, JournalRegion::International);
    let directory_summary = directory.map(JournalDirectoryCatalog::summary);
    let journal_directory_version = directory_summary
        .as_ref()
        .and_then(|summary| summary.catalog_fingerprint.clone());
    let encoded = serde_json::to_vec(&(
        report.workspace_id.as_str(),
        report.source_content_hash.as_str(),
        report.source_snapshot_version,
        evaluated_unix_ms / 86_400_000,
        &profile,
        &preferences,
        &journal_directory_version,
    ))
    .unwrap_or_default();
    let run_id = format!(
        "jmr-{}",
        hex::encode(Sha256::digest(encoded))
            .chars()
            .take(20)
            .collect::<String>()
    );
    let directory_available = directory_summary
        .as_ref()
        .is_some_and(|summary| summary.available);
    let cas_directory_available = directory_summary.as_ref().is_some_and(|summary| {
        summary
            .records_by_scheme
            .get("cas_partition")
            .is_some_and(|count| *count > 0)
    });
    let institution_directory_status = if cas_directory_available {
        "local_user_supplied_directory_available".to_owned()
    } else if directory_available {
        "local_directory_without_cas_partition_data".to_owned()
    } else {
        profile
            .institution_rule_evidence
            .cas_partition_data_status
            .clone()
            .unwrap_or_else(|| "local_directory_not_imported".into())
    };
    let external_transmission = profile.external_transmission.clone();
    let profile_summary = JournalRecommendationProfileSummary {
        profile_id: profile.profile_id.clone(),
        profile_version: profile.profile_version,
        institution: profile.institution.clone(),
        specialty: profile.specialty.clone(),
        manuscript_purpose: profile.manuscript_purpose,
        submission_deadline: profile.submission_deadline.clone(),
    };
    let school_rule_status = match profile.institution_rule_evidence.status {
        InstitutionRuleStatus::Verified
            if (profile
                .institution_rule_evidence
                .minimum_cas_partition
                .is_some()
                || profile.institution_rule_evidence.requires_cas_top)
                && !cas_directory_available =>
        {
            "verified_rule_waiting_for_institution_directory_data"
        }
        InstitutionRuleStatus::Verified
            if (profile
                .institution_rule_evidence
                .minimum_cas_partition
                .is_some()
                || profile.institution_rule_evidence.requires_cas_top)
                && cas_directory_available =>
        {
            "verified_rule_set_applied_with_local_directory"
        }
        InstitutionRuleStatus::Verified => "verified_rule_set_applied",
        InstitutionRuleStatus::CandidateSourcesFound => "candidate_sources_require_verification",
        InstitutionRuleStatus::NoOfficialRuleFound => "no_official_rule_found_excluded_from_score",
        InstitutionRuleStatus::SearchRequired => {
            "official_source_search_required_excluded_from_score"
        }
    }
    .to_owned();
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
        evaluated_unix_ms,
        recommendation_profile: profile_summary,
        deadline_days_remaining,
        preferences,
        domestic,
        international,
        school_rule_status,
        institution_directory_status,
        journal_directory_version,
        limitations: vec![
            "适配分计算当前投稿准备的最适合度，不是录用概率，也不替代期刊官网的最新投稿要求。".into(),
            "截止日期只评估完成投稿准备的内部规划余量，不预测同行评审、录用、见刊或数据库收录时间。".into(),
            "姓名仅用于本地记录归属；学校排名和导师名气不参与声誉打分，学校正式规则只参与资格与用途判断。".into(),
            "当前内容完备度是结构信号，不是论文创新性或学术贡献评分；取得版本化 PWC 审核档案前不得据此宣称顶刊成功率。".into(),
            "国内 T1/T2/T3 与国际 CCF A/B/C 是相互独立的目录，不做等级等同。".into(),
            "当前候选范围仅覆盖内置的计算机与人工智能期刊快照。".into(),
            if directory_available {
                "本地期刊目录来自用户提供的工作簿；仅在本机参与离线辅助并显示来源年份，不自动视为官方核验。".into()
            } else {
                "机构评价目录未导入；涉及分区的资格条件不推断，也不计入得分。".into()
            },
        ],
        external_transmission,
    }
}

fn build_recommendation_portfolio(
    scored: &[(Candidate, JournalRecommendation)],
    region: JournalRegion,
) -> JournalRecommendationPortfolio {
    let mut selected_ids = Vec::<&str>::new();
    let eligible = |recommendation: &JournalRecommendation| {
        !recommendation
            .institution_eligibility
            .starts_with("blocked_by_verified")
    };

    let mut sprint = select_recommendations(scored, &mut selected_ids, 2, |candidate| {
        candidate.region == region && candidate.level == 3
    });
    if sprint.len() < 2 {
        sprint.extend(select_recommendations(
            scored,
            &mut selected_ids,
            2 - sprint.len(),
            |candidate| candidate.region == region,
        ));
    }

    let matching = select_recommendations(scored, &mut selected_ids, 3, |candidate| {
        candidate.region == region
    });

    let mut safeguard_candidates: Vec<_> = scored
        .iter()
        .filter(|(candidate, recommendation)| {
            candidate.region == region
                && candidate.level <= 2
                && eligible(recommendation)
                && !selected_ids.contains(&candidate.id)
        })
        .collect();
    safeguard_candidates.sort_by(|(left_candidate, left), (right_candidate, right)| {
        left_candidate
            .level
            .cmp(&right_candidate.level)
            .then_with(|| right.scores.topic_scope.cmp(&left.scores.topic_scope))
            .then_with(|| right.scores.specialty_fit.cmp(&left.scores.specialty_fit))
            .then_with(|| right.scores.article_type.cmp(&left.scores.article_type))
            .then_with(|| {
                right
                    .scores
                    .time_feasibility
                    .cmp(&left.scores.time_feasibility)
            })
            .then_with(|| right.overall_fit.cmp(&left.overall_fit))
            .then_with(|| left.name_en.cmp(&right.name_en))
    });
    let mut safeguard: Vec<_> = safeguard_candidates
        .into_iter()
        .take(3)
        .map(|(candidate, recommendation)| {
            selected_ids.push(candidate.id);
            recommendation.clone()
        })
        .collect();
    if safeguard.len() < 3 {
        safeguard.extend(select_recommendations(
            scored,
            &mut selected_ids,
            3 - safeguard.len(),
            |candidate| candidate.region == region,
        ));
    }

    JournalRecommendationPortfolio {
        sprint,
        matching,
        safeguard,
    }
}

fn select_recommendations(
    scored: &[(Candidate, JournalRecommendation)],
    selected_ids: &mut Vec<&'static str>,
    count: usize,
    predicate: impl Fn(Candidate) -> bool,
) -> Vec<JournalRecommendation> {
    let mut result = Vec::new();
    for (candidate, recommendation) in scored {
        if result.len() == count {
            break;
        }
        if predicate(*candidate)
            && !recommendation
                .institution_eligibility
                .starts_with("blocked_by_verified")
            && !selected_ids.contains(&candidate.id)
        {
            selected_ids.push(candidate.id);
            result.push(recommendation.clone());
        }
    }
    result
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
    infer_topic_from_text(&text)
}

fn infer_topic_from_text(text: &str) -> (ResearchTopic, String) {
    let text = text.to_lowercase();
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
    context: &ScoreContext<'_>,
    directory_evidence: Vec<JournalDirectoryEvidence>,
) -> JournalRecommendation {
    let ScoreContext {
        topic,
        specialty_topic,
        article_type,
        maturity,
        deadline_days_remaining,
        purpose,
        institution_rules,
        preferences,
    } = context;
    let topic_score = if candidate.topics.contains(topic) {
        100
    } else if candidate.topics.contains(&ResearchTopic::GeneralAi)
        || *topic == ResearchTopic::GeneralAi
    {
        70
    } else {
        30
    };
    let article_score = if candidate.article_types.contains(article_type) {
        100
    } else {
        55
    };
    let specialty = if *specialty_topic == ResearchTopic::GeneralAi {
        75
    } else if candidate.topics.contains(specialty_topic) {
        100
    } else if candidate.topics.contains(&ResearchTopic::GeneralAi) {
        70
    } else {
        35
    };
    let threshold = match candidate.level {
        3 => 85,
        2 => 70,
        _ => 55,
    };
    let readiness = if *maturity >= threshold {
        100
    } else {
        (100i16 - 2 * (threshold as i16 - *maturity as i16)).clamp(20, 100) as u8
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
    let purpose_fit = match purpose {
        ManuscriptPurpose::AcademicCommunication => [0, 65, 85, 100][candidate.level as usize],
        ManuscriptPurpose::DegreeRequirement => [0, 85, 100, 72][candidate.level as usize],
        ManuscriptPurpose::Graduation => [0, 100, 86, 55][candidate.level as usize],
        ManuscriptPurpose::ProfessionalTitle => [0, 72, 100, 88][candidate.level as usize],
        ManuscriptPurpose::ProjectCompletion => [0, 100, 82, 58][candidate.level as usize],
    };
    let estimated_submission_preparation_days = match candidate.level {
        3 => 28,
        2 => 18,
        _ => 10,
    } + if preferences.language
        != PublicationLanguagePreference::Auto
        && preferences.language != candidate.language
    {
        7
    } else {
        0
    };
    let time_feasibility = if *deadline_days_remaining >= estimated_submission_preparation_days {
        100
    } else if *deadline_days_remaining == 0 {
        10
    } else {
        ((*deadline_days_remaining * 100 / estimated_submission_preparation_days).max(10)) as u8
    };
    let has_traceable_source = !institution_rules.source_urls.is_empty()
        || (institution_rules.author_attested_official
            && institution_rules.source_text_hash.is_some());
    let cas_evidence = directory_evidence
        .iter()
        .find(|evidence| evidence.scheme == JournalMetricScheme::CasPartition);
    let requires_cas_data =
        institution_rules.minimum_cas_partition.is_some() || institution_rules.requires_cas_top;
    let cas_data_ready = !requires_cas_data
        || cas_evidence.is_some_and(|evidence| {
            institution_rules.minimum_cas_partition.is_none() || evidence.partition.is_some()
        }) && (!institution_rules.requires_cas_top
            || cas_evidence.is_some_and(|evidence| evidence.top.is_some()));
    let verified_rules = institution_rules.status == InstitutionRuleStatus::Verified
        && institution_rules.author_attested_official
        && has_traceable_source
        && institution_rules.rule_set_version.is_some()
        && cas_data_ready;
    let cas_partition_blocked = institution_rules
        .minimum_cas_partition
        .is_some_and(|minimum| {
            cas_evidence
                .and_then(|evidence| evidence.partition)
                .is_some_and(|partition| partition > minimum)
        });
    let cas_top_blocked = institution_rules.requires_cas_top
        && cas_evidence.is_some_and(|evidence| evidence.top == Some(false));
    let (institution_score, institution_eligibility) = if !cas_data_ready {
        (None, "requires_local_cas_directory_data".to_owned())
    } else if !verified_rules {
        (None, "requires_verified_official_rules".to_owned())
    } else if cas_partition_blocked || cas_top_blocked {
        (Some(0), "blocked_by_verified_cas_rule".to_owned())
    } else if institution_rules
        .blocked_rank_tiers
        .iter()
        .any(|tier| tier == candidate.tier)
    {
        (Some(0), "blocked_by_verified_rule".to_owned())
    } else if institution_rules
        .recognized_rank_tiers
        .iter()
        .any(|tier| tier == candidate.tier)
    {
        (Some(100), "recognized_by_verified_rule".to_owned())
    } else if requires_cas_data {
        (Some(100), "recognized_by_verified_cas_rule".to_owned())
    } else {
        (Some(40), "not_mapped_by_verified_rule".to_owned())
    };
    let scores = JournalFitScores {
        institution_rules: institution_score,
        topic_scope: topic_score,
        specialty_fit: specialty,
        article_type: article_score,
        content_readiness: readiness,
        language,
        target_level: target,
        open_access: oa,
        purpose_fit,
        time_feasibility,
    };
    let non_institution_total = topic_score as u32 * 18
        + specialty as u32 * 5
        + article_score as u32 * 8
        + readiness as u32 * 10
        + language as u32 * 6
        + target as u32 * 8
        + oa as u32 * 3
        + purpose_fit as u32 * 8
        + time_feasibility as u32 * 10;
    let overall = if let Some(institution_score) = institution_score {
        ((non_institution_total + institution_score as u32 * 24) / 100) as u8
    } else {
        (non_institution_total / 76) as u8
    };
    let source = if candidate.region == JournalRegion::Domestic {
        RANK_DOMESTIC
    } else {
        RANK_INTERNATIONAL
    };
    let readiness_reason = if *maturity >= threshold {
        format!(
            "当前版本结构完备度 {}，达到该层级的投稿准备门槛 {}",
            maturity, threshold
        )
    } else {
        format!(
            "当前版本结构完备度 {}，距离该层级的投稿准备门槛还差 {}",
            maturity,
            threshold - *maturity
        )
    };
    let mut reasons = vec![
        format!("主题范围适配 {} 分", topic_score),
        format!("作者专业背景适配 {} 分", specialty),
        format!("论文用途适配 {} 分", purpose_fit),
        format!(
            "投稿准备时间适配 {} 分（内部规划 {} 天）",
            time_feasibility, estimated_submission_preparation_days
        ),
        format!("当前稿件完备度适配 {} 分；{}", readiness, readiness_reason),
        format!("目标策略适配 {} 分", target),
    ];
    for evidence in &directory_evidence {
        let scheme = match evidence.scheme {
            JournalMetricScheme::CasPartition => "中科院分区",
            JournalMetricScheme::ClarivateJcr => "JCR",
            JournalMetricScheme::EmergingPartition => "新锐分区",
        };
        let partition = evidence
            .partition
            .map(|value| format!("{value}区"))
            .unwrap_or_else(|| "分区缺失".to_owned());
        reasons.push(format!(
            "本地目录：{scheme} {} · {partition}{}",
            evidence.release_year,
            if evidence.top == Some(true) {
                " · Top"
            } else {
                ""
            }
        ));
    }
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
        estimated_submission_preparation_days,
        deadline_status: if time_feasibility == 100 {
            "planning_window_sufficient".into()
        } else {
            "planning_window_tight".into()
        },
        institution_eligibility,
        scores,
        reasons,
        ranking_source_url: source.into(),
        homepage_url: candidate.homepage.into(),
        open_access_status: candidate.oa.into(),
        directory_evidence,
    }
}

pub fn deadline_days_remaining(deadline: &str, saved_unix_ms: u64) -> Option<u32> {
    let deadline_days = parse_iso_date_days(deadline)?;
    let saved_days = (saved_unix_ms / 86_400_000) as i64;
    u32::try_from(deadline_days - saved_days).ok()
}

fn parse_iso_date_days(value: &str) -> Option<i64> {
    let mut parts = value.split('-');
    let year = parts.next()?.parse::<i32>().ok()?;
    let month = parts.next()?.parse::<u32>().ok()?;
    let day = parts.next()?.parse::<u32>().ok()?;
    if parts.next().is_some() || !(1970..=9999).contains(&year) || !(1..=12).contains(&month) {
        return None;
    }
    let month_days = [
        31,
        if is_leap_year(year) { 29 } else { 28 },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ];
    if day == 0 || day > month_days[(month - 1) as usize] {
        return None;
    }
    let adjusted_year = year - i32::from(month <= 2);
    let era = adjusted_year.div_euclid(400);
    let year_of_era = adjusted_year - era * 400;
    let adjusted_month = month as i32 + if month > 2 { -3 } else { 9 };
    let day_of_year = (153 * adjusted_month + 2) / 5 + day as i32 - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    Some((era * 146_097 + day_of_era - 719_468) as i64)
}

fn is_leap_year(year: i32) -> bool {
    year % 4 == 0 && (year % 100 != 0 || year % 400 == 0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AnalysisQuality, SectionSummary};
    const EVALUATED_AT: u64 = 1_788_048_000_000;
    fn profile() -> JournalRecommendationProfile {
        JournalRecommendationProfile {
            schema_version: JOURNAL_PROFILE_SCHEMA_VERSION,
            profile_id: "jmp-synthetic".into(),
            profile_version: 1,
            workspace_id: "test-workspace".into(),
            author_name: "Test Author".into(),
            institution: "Synthetic University".into(),
            specialty: "Artificial intelligence".into(),
            manuscript_purpose: ManuscriptPurpose::AcademicCommunication,
            submission_deadline: "2026-12-30".into(),
            saved_unix_ms: 1_788_048_000_000,
            institution_rule_evidence: InstitutionRuleEvidence::default(),
            external_transmission: "not_performed".into(),
        }
    }
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
            semantic_candidates: Vec::new(),
            source_fragments: Vec::new(),
            extraction_coverage: Default::default(),
            pdf_processing: None,
            warnings: vec![],
        }
    }

    fn portfolio(run: &JournalRecommendationRun) -> Vec<&JournalRecommendation> {
        run.domestic
            .sprint
            .iter()
            .chain(run.domestic.matching.iter())
            .chain(run.domestic.safeguard.iter())
            .chain(run.international.sprint.iter())
            .chain(run.international.matching.iter())
            .chain(run.international.safeguard.iter())
            .collect()
    }

    fn portfolio_ids(run: &JournalRecommendationRun) -> Vec<&str> {
        portfolio(run)
            .into_iter()
            .map(|item| item.id.as_str())
            .collect()
    }

    #[test]
    fn traceable_structure_improvement_raises_readiness_but_version_number_alone_does_not() {
        let mut incomplete = report("computer vision");
        incomplete.title = None;
        incomplete.abstract_present = false;
        incomplete.abstract_text = None;
        incomplete.keywords_present = false;
        incomplete.sections.clear();
        incomplete.figure_count = 0;
        incomplete.table_count = 0;
        incomplete.references_present = false;
        incomplete.word_count = 400;
        let complete = report("computer vision");
        let preferences = JournalMatchPreferences::default();
        let rules = InstitutionRuleEvidence::default();
        let candidate = CANDIDATES
            .iter()
            .find(|candidate| candidate.level == 3)
            .copied()
            .expect("catalog should include a top-level synthetic candidate");
        let score = |report: &StructureReport| {
            score_candidate(
                candidate,
                &ScoreContext {
                    topic: ResearchTopic::ComputerVision,
                    specialty_topic: ResearchTopic::ComputerVision,
                    article_type: ArticleTypePreference::Research,
                    maturity: maturity_score(report),
                    deadline_days_remaining: 120,
                    purpose: ManuscriptPurpose::AcademicCommunication,
                    institution_rules: &rules,
                    preferences: &preferences,
                },
                Vec::new(),
            )
        };

        let incomplete_score = score(&incomplete);
        let mut renumbered_only = incomplete.clone();
        renumbered_only.source_snapshot_version = 99;
        let renumbered_score = score(&renumbered_only);
        let complete_score = score(&complete);

        assert_eq!(
            incomplete_score.scores.content_readiness,
            renumbered_score.scores.content_readiness
        );
        assert_eq!(incomplete_score.overall_fit, renumbered_score.overall_fit);
        assert!(
            complete_score.scores.content_readiness > incomplete_score.scores.content_readiness
        );
        assert!(complete_score.overall_fit > incomplete_score.overall_fit);
    }
    #[test]
    fn returns_two_three_three_per_region_without_duplicates() {
        let run = recommend_journals(
            &report("general artificial intelligence"),
            profile(),
            JournalMatchPreferences::default(),
            EVALUATED_AT,
        );
        assert_eq!(run.domestic.sprint.len(), 2);
        assert_eq!(run.domestic.matching.len(), 3);
        assert_eq!(run.domestic.safeguard.len(), 3);
        assert_eq!(run.international.sprint.len(), 2);
        assert_eq!(run.international.matching.len(), 3);
        assert_eq!(run.international.safeguard.len(), 3);
        let ids = portfolio_ids(&run);
        assert_eq!(
            ids.iter()
                .copied()
                .collect::<std::collections::HashSet<_>>()
                .len(),
            16
        );
        assert!(run
            .domestic
            .sprint
            .iter()
            .chain(run.domestic.matching.iter())
            .chain(run.domestic.safeguard.iter())
            .all(|item| item.region == JournalRegion::Domestic));
        assert!(run
            .international
            .sprint
            .iter()
            .chain(run.international.matching.iter())
            .chain(run.international.safeguard.iter())
            .all(|item| item.region == JournalRegion::International));
        assert!(portfolio(&run).iter().all(|item| item.overall_fit <= 100));
    }
    #[test]
    fn author_topic_adjustment_changes_recommendations() {
        let mut p = JournalMatchPreferences {
            topic: ResearchTopic::ComputerVision,
            ..JournalMatchPreferences::default()
        };
        let cv = recommend_journals(
            &report("general artificial intelligence"),
            profile(),
            p.clone(),
            EVALUATED_AT,
        );
        p.topic = ResearchTopic::NaturalLanguageProcessing;
        let nlp = recommend_journals(
            &report("general artificial intelligence"),
            profile(),
            p,
            EVALUATED_AT,
        );
        assert_ne!(cv.run_id, nlp.run_id);
        assert!(portfolio(&cv).iter().any(|j| j.id == "tpami"));
        assert!(portfolio(&nlp).iter().any(|j| j.id == "tacl"));
        assert!(portfolio(&nlp).iter().any(|j| j.id == "jcip"));
    }
    #[test]
    fn same_inputs_are_deterministic() {
        let a = recommend_journals(
            &report("machine learning"),
            profile(),
            JournalMatchPreferences::default(),
            EVALUATED_AT,
        );
        let b = recommend_journals(
            &report("machine learning"),
            profile(),
            JournalMatchPreferences::default(),
            EVALUATED_AT,
        );
        assert_eq!(a, b);
    }

    #[test]
    fn profile_identity_changes_create_a_new_attributed_run_without_prestige_scoring() {
        let first = recommend_journals(
            &report("machine learning"),
            profile(),
            JournalMatchPreferences::default(),
            EVALUATED_AT,
        );
        let mut changed = profile();
        changed.author_name = "Another Author".into();
        changed.institution = "Another University".into();
        changed.profile_id = "jmp-another".into();
        let second = recommend_journals(
            &report("machine learning"),
            changed,
            JournalMatchPreferences::default(),
            EVALUATED_AT,
        );
        assert_ne!(first.run_id, second.run_id);
        assert_eq!(portfolio_ids(&first), portfolio_ids(&second));
        assert!(second.school_rule_status.contains("excluded_from_score"));
    }

    #[test]
    fn specialty_purpose_and_deadline_change_the_recommendation_result() {
        let mut cv_profile = profile();
        cv_profile.specialty = "computer vision".into();
        let cv = recommend_journals(
            &report("general artificial intelligence"),
            cv_profile,
            JournalMatchPreferences::default(),
            EVALUATED_AT,
        );
        let mut nlp_profile = profile();
        nlp_profile.specialty = "natural language processing".into();
        nlp_profile.manuscript_purpose = ManuscriptPurpose::Graduation;
        nlp_profile.submission_deadline = "2026-09-05".into();
        nlp_profile.profile_id = "jmp-urgent-nlp".into();
        let nlp = recommend_journals(
            &report("general artificial intelligence"),
            nlp_profile,
            JournalMatchPreferences::default(),
            EVALUATED_AT,
        );
        assert_ne!(cv.run_id, nlp.run_id);
        assert_ne!(portfolio_ids(&cv), portfolio_ids(&nlp));
        assert!(nlp.deadline_days_remaining < cv.deadline_days_remaining);
        assert!(portfolio(&nlp)
            .iter()
            .any(|item| item.scores.time_feasibility < 100));
    }

    #[test]
    fn validates_profile_fields_and_iso_deadline() {
        let input = JournalRecommendationProfileInput {
            author_name: "  Test Author  ".into(),
            institution: " Synthetic University ".into(),
            specialty: " Computer vision ".into(),
            manuscript_purpose: ManuscriptPurpose::DegreeRequirement,
            submission_deadline: "2026-02-29".into(),
        };
        assert!(input.normalized().is_err());
        assert_eq!(
            deadline_days_remaining("2026-09-05", 1_788_048_000_000),
            Some(6)
        );
    }

    #[test]
    fn only_verified_official_institution_rules_receive_the_twenty_four_percent_weight() {
        let mut verified = profile();
        let evidence = InstitutionRuleEvidence {
            status: InstitutionRuleStatus::Verified,
            rule_set_id: Some("school-rule-synthetic".into()),
            rule_set_version: Some("2026.1".into()),
            source_urls: vec!["https://university.example.edu/rules/2026".into()],
            verified_at: Some("2026-08-30".into()),
            recognized_rank_tiers: vec!["T1".into(), "CCF A".into()],
            blocked_rank_tiers: vec!["T2".into()],
            author_attested_official: true,
            ..InstitutionRuleEvidence::default()
        };
        verified.institution_rule_evidence = evidence.clone();
        let run = recommend_journals(
            &report("general artificial intelligence"),
            verified,
            JournalMatchPreferences::default(),
            EVALUATED_AT,
        );

        assert_eq!(run.school_rule_status, "verified_rule_set_applied");
        assert!(portfolio(&run)
            .iter()
            .any(|item| item.scores.institution_rules == Some(100)));
        let preferences = JournalMatchPreferences::default();
        let context = ScoreContext {
            topic: ResearchTopic::GeneralAi,
            specialty_topic: ResearchTopic::GeneralAi,
            article_type: ArticleTypePreference::Research,
            maturity: 80,
            deadline_days_remaining: 60,
            purpose: ManuscriptPurpose::AcademicCommunication,
            institution_rules: &evidence,
            preferences: &preferences,
        };
        let blocked = score_candidate(
            CANDIDATES
                .iter()
                .find(|item| item.tier == "T2")
                .copied()
                .unwrap(),
            &context,
            Vec::new(),
        );
        assert_eq!(blocked.scores.institution_rules, Some(0));
        assert_eq!(blocked.institution_eligibility, "blocked_by_verified_rule");
    }

    #[test]
    fn verified_rule_blocked_candidates_never_enter_the_main_portfolio() {
        let mut constrained = profile();
        constrained.institution_rule_evidence = InstitutionRuleEvidence {
            status: InstitutionRuleStatus::Verified,
            rule_set_id: Some("school-rule-synthetic".into()),
            rule_set_version: Some("2026.1".into()),
            source_urls: vec!["https://university.example.edu/rules/2026".into()],
            verified_at: Some("2026-08-30".into()),
            blocked_rank_tiers: vec!["T1".into(), "T2".into()],
            author_attested_official: true,
            ..InstitutionRuleEvidence::default()
        };
        let run = recommend_journals(
            &report("general artificial intelligence"),
            constrained,
            JournalMatchPreferences::default(),
            EVALUATED_AT,
        );

        assert_eq!(portfolio(&run).len(), 8);
        assert!(portfolio(&run).iter().all(|item| {
            !item
                .institution_eligibility
                .starts_with("blocked_by_verified")
        }));
    }

    #[test]
    fn a_cas_partition_requirement_waits_for_licensed_official_data() {
        let mut constrained = profile();
        constrained.institution_rule_evidence = InstitutionRuleEvidence {
            status: InstitutionRuleStatus::Verified,
            rule_set_id: Some("school-rule-cas-synthetic".into()),
            rule_set_version: Some("2026.1".into()),
            source_text_hash: Some("a".repeat(64)),
            source_kind: Some("author_supplied_institution_requirement".into()),
            extracted_conditions: vec!["毕业成果须为中科院二区及以上".into()],
            minimum_cas_partition: Some(2),
            author_attested_official: true,
            ..InstitutionRuleEvidence::default()
        };
        let run = recommend_journals(
            &report("general artificial intelligence"),
            constrained,
            JournalMatchPreferences::default(),
            EVALUATED_AT,
        );
        assert_eq!(
            run.school_rule_status,
            "verified_rule_waiting_for_institution_directory_data"
        );
        assert_eq!(
            run.institution_directory_status,
            "licensed_official_api_not_configured"
        );
        assert!(portfolio(&run)
            .iter()
            .all(|item| item.scores.institution_rules.is_none()));
        let webview_projection = serde_json::to_string(&run).unwrap();
        assert!(!webview_projection.contains("minimumCasPartition"));
        assert!(!webview_projection.contains("extractedConditions"));
        assert!(!webview_projection.contains("sourceTextHash"));
    }
}
