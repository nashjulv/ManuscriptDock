use quick_xml::{escape::unescape, events::Event, Reader};
use rusqlite::{params, Connection, OptionalExtension, Transaction};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    error::Error,
    fmt,
    fs::{self, File},
    io::{self, BufRead, BufReader, Read},
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};
#[cfg(test)]
use uuid::Uuid;
use zip::ZipArchive;

pub const JOURNAL_DIRECTORY_SCHEMA_VERSION: u32 = 2;
const MAX_WORKBOOK_BYTES: u64 = 64 * 1024 * 1024;
const MAX_XML_BYTES: u64 = 96 * 1024 * 1024;
const MAX_ROWS_PER_SHEET: usize = 100_000;
const MAX_COLUMNS_PER_ROW: usize = 64;

#[derive(Debug)]
pub enum JournalDirectoryError {
    Io(io::Error),
    InvalidWorkbook(String),
    UnsupportedWorkbook(String),
    InvalidCatalog(String),
    Database(String),
}

impl fmt::Display for JournalDirectoryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(formatter, "期刊目录读写失败：{error}"),
            Self::InvalidWorkbook(message) => write!(formatter, "期刊表格无效：{message}"),
            Self::UnsupportedWorkbook(message) => write!(formatter, "不支持的期刊表格：{message}"),
            Self::InvalidCatalog(message) => write!(formatter, "本地期刊目录无效：{message}"),
            Self::Database(message) => write!(formatter, "本地期刊数据库失败：{message}"),
        }
    }
}

impl Error for JournalDirectoryError {}

impl From<io::Error> for JournalDirectoryError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<rusqlite::Error> for JournalDirectoryError {
    fn from(error: rusqlite::Error) -> Self {
        Self::Database(error.to_string())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum JournalMetricScheme {
    CasPartition,
    ClarivateJcr,
    EmergingPartition,
}

impl JournalMetricScheme {
    fn label(self) -> &'static str {
        match self {
            Self::CasPartition => "cas_partition",
            Self::ClarivateJcr => "clarivate_jcr",
            Self::EmergingPartition => "emerging_partition",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JournalDirectoryRecord {
    pub record_id: String,
    pub source_id: String,
    pub sheet_name: String,
    pub row_number: u32,
    pub scheme: JournalMetricScheme,
    pub release_year: u16,
    pub metric_year: Option<u16>,
    pub journal_name: String,
    pub normalized_name: String,
    pub issn: Option<String>,
    pub eissn: Option<String>,
    pub category: Option<String>,
    pub partition: Option<u8>,
    pub top: Option<bool>,
    pub open_access: Option<bool>,
    pub jif: Option<f64>,
    pub total_citations: Option<u64>,
    pub jif_rank: Option<String>,
    pub value_basis: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JournalDirectorySource {
    pub source_id: String,
    pub file_name: String,
    pub sha256: String,
    pub imported_unix_ms: u64,
    pub active: bool,
    pub data_origin: String,
    pub sheet_names: Vec<String>,
    pub formula_cell_count: u32,
    pub record_count: u32,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JournalDirectoryCatalog {
    pub schema_version: u32,
    pub updated_unix_ms: u64,
    pub sources: Vec<JournalDirectorySource>,
    pub records: Vec<JournalDirectoryRecord>,
}

impl Default for JournalDirectoryCatalog {
    fn default() -> Self {
        Self {
            schema_version: JOURNAL_DIRECTORY_SCHEMA_VERSION,
            updated_unix_ms: 0,
            sources: Vec::new(),
            records: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JournalDirectorySummary {
    pub schema_version: u32,
    pub available: bool,
    pub source_count: u32,
    pub record_count: u32,
    pub distinct_journal_count: u32,
    pub issn_count: u32,
    pub eissn_count: u32,
    pub publisher_count: u32,
    pub scope_count: u32,
    pub annual_volume_count: u32,
    pub review_process_count: u32,
    pub review_speed_count: u32,
    pub publication_cycle_count: u32,
    pub circulation_count: u32,
    pub oa_status_count: u32,
    pub latest_release_year: Option<u16>,
    pub records_by_scheme: BTreeMap<String, u32>,
    pub partition_counts: BTreeMap<String, u32>,
    pub top_count: u32,
    pub open_access_count: u32,
    pub formula_cell_count: u32,
    pub catalog_fingerprint: Option<String>,
    pub updated_unix_ms: u64,
    pub source_files: Vec<String>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JournalDirectoryEvidence {
    pub scheme: JournalMetricScheme,
    pub release_year: u16,
    pub metric_year: Option<u16>,
    pub issn: Option<String>,
    pub eissn: Option<String>,
    pub partition: Option<u8>,
    pub top: Option<bool>,
    pub open_access: Option<bool>,
    pub jif_tenths: Option<u32>,
    pub category: Option<String>,
    pub source_file: String,
    pub data_origin: String,
    pub value_basis: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JournalDirectoryImportResult {
    pub imported_source_count: u32,
    pub imported_record_count: u32,
    pub unchanged_source_count: u32,
    pub summary: JournalDirectorySummary,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JournalDirectoryProfile {
    pub journal_id: String,
    pub journal_name: String,
    pub issn: Option<String>,
    pub eissn: Option<String>,
    pub publisher: Option<String>,
    pub homepage_url: Option<String>,
    pub publication_scope_note: Option<String>,
    pub aims_scope_url: Option<String>,
    pub author_instructions_url: Option<String>,
    pub reported_print_circulation: Option<u64>,
    pub annual_publication_volume: Option<u64>,
    pub annual_publication_volume_year: Option<u16>,
    pub peer_review_process: Vec<String>,
    pub peer_review_policy_url: Option<String>,
    pub average_review_days: Option<f64>,
    pub submission_to_publication_days: Option<f64>,
    pub publication_frequency: Option<String>,
    pub apc_status: Option<String>,
    pub open_access_status: Option<String>,
    pub source_url: Option<String>,
    pub retrieved_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JournalProfileImportResult {
    pub imported_profile_count: u32,
    pub unmatched_profile_count: u32,
    pub summary: JournalDirectorySummary,
}

pub const JOURNAL_PROFILE_DISCOVERY_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JournalProfileDiscoveryRecord {
    pub schema_version: u32,
    pub discovery_id: String,
    pub workspace_id: String,
    pub target_selection_id: String,
    pub journal_id: String,
    pub journal_name: String,
    pub issn: Option<String>,
    pub eissn: Option<String>,
    pub publisher: Option<String>,
    pub scope_summary: Option<String>,
    pub reported_print_circulation: Option<u64>,
    pub average_review_days: Option<f64>,
    pub submission_to_publication_days: Option<f64>,
    pub publication_frequency: Option<String>,
    pub apc_status: Option<String>,
    pub open_access_status: Option<String>,
    pub official_homepage_url: Option<String>,
    pub aims_scope_url: Option<String>,
    pub author_instructions_url: Option<String>,
    pub source_urls: Vec<String>,
    pub missing_fields: Vec<String>,
    pub evidence_status: String,
    pub source_mode: String,
    pub provider_label: Option<String>,
    pub model: Option<String>,
    pub external_transmission: String,
    pub created_unix_ms: u64,
}

#[derive(Debug, Clone)]
pub struct JournalDirectoryStore {
    root: PathBuf,
}

impl JournalDirectoryStore {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub fn database_path(&self) -> PathBuf {
        self.root.join("journal-directory-v2.sqlite3")
    }

    pub fn legacy_catalog_path(&self) -> PathBuf {
        self.root.join("catalog-v1.json")
    }

    pub fn load(&self) -> Result<JournalDirectoryCatalog, JournalDirectoryError> {
        if self.database_path().exists() {
            return self.load_database();
        }
        let legacy_path = self.legacy_catalog_path();
        if !legacy_path.exists() {
            return Ok(JournalDirectoryCatalog::default());
        }
        let legacy: JournalDirectoryCatalog = serde_json::from_reader(File::open(&legacy_path)?)
            .map_err(|error| JournalDirectoryError::InvalidCatalog(error.to_string()))?;
        if legacy.schema_version != 1 {
            return Err(JournalDirectoryError::InvalidCatalog(
                "目录版本与当前应用不兼容".to_owned(),
            ));
        }
        let active_ids = legacy
            .sources
            .iter()
            .filter(|source| source.active)
            .map(|source| source.source_id.clone())
            .collect::<BTreeSet<_>>();
        let catalog = JournalDirectoryCatalog {
            schema_version: JOURNAL_DIRECTORY_SCHEMA_VERSION,
            updated_unix_ms: legacy.updated_unix_ms,
            sources: legacy
                .sources
                .into_iter()
                .filter(|source| source.active)
                .collect(),
            records: legacy
                .records
                .into_iter()
                .filter(|record| active_ids.contains(&record.source_id))
                .collect(),
        };
        self.write_catalog(&catalog)?;
        Ok(catalog)
    }

    pub fn summary(&self) -> Result<JournalDirectorySummary, JournalDirectoryError> {
        let catalog = self.load()?;
        let mut summary = catalog.summary();
        if self.database_path().exists() {
            let connection = self.open_database()?;
            apply_profile_counts(&connection, &mut summary)?;
        }
        Ok(summary)
    }

    pub fn import_workbooks(
        &self,
        paths: &[PathBuf],
    ) -> Result<JournalDirectoryImportResult, JournalDirectoryError> {
        if paths.is_empty() {
            return Err(JournalDirectoryError::UnsupportedWorkbook(
                "未选择 Excel 文件".to_owned(),
            ));
        }
        let mut catalog = self.load()?;
        let mut imported_source_count = 0_u32;
        let mut imported_record_count = 0_u32;
        let mut unchanged_source_count = 0_u32;

        for path in paths {
            let workbook = parse_workbook(path)?;
            if catalog
                .sources
                .iter()
                .any(|source| source.sha256 == workbook.source.sha256 && source.active)
            {
                unchanged_source_count = unchanged_source_count.saturating_add(1);
                continue;
            }
            let replaces_emerging_2026 = workbook.source.file_name.contains("新锐")
                && workbook.source.file_name.contains("2026")
                && records_are_emerging_2026(workbook.records.iter());
            let superseded_source_ids = if replaces_emerging_2026 {
                catalog
                    .sources
                    .iter()
                    .filter(|source| {
                        source.active
                            && source.file_name.contains("新锐")
                            && source.file_name.contains("2026")
                            && records_are_emerging_2026(
                                catalog
                                    .records
                                    .iter()
                                    .filter(|record| record.source_id == source.source_id),
                            )
                    })
                    .map(|source| source.source_id.clone())
                    .collect::<BTreeSet<_>>()
            } else {
                BTreeSet::new()
            };
            for source in &mut catalog.sources {
                if (source.file_name == workbook.source.file_name
                    || superseded_source_ids.contains(&source.source_id))
                    && source.active
                {
                    source.active = false;
                }
            }
            imported_source_count = imported_source_count.saturating_add(1);
            imported_record_count = imported_record_count
                .saturating_add(u32::try_from(workbook.records.len()).unwrap_or(u32::MAX));
            catalog.sources.push(workbook.source);
            catalog.records.extend(workbook.records);
        }

        if imported_source_count > 0 {
            catalog.schema_version = JOURNAL_DIRECTORY_SCHEMA_VERSION;
            catalog.updated_unix_ms = unix_time_ms()?;
            self.write_catalog(&catalog)?;
        }
        let mut summary = catalog.summary();
        if self.database_path().exists() {
            let connection = self.open_database()?;
            apply_profile_counts(&connection, &mut summary)?;
        }
        Ok(JournalDirectoryImportResult {
            imported_source_count,
            imported_record_count,
            unchanged_source_count,
            summary,
        })
    }

    pub fn import_profile_catalog(
        &self,
        path: &Path,
    ) -> Result<JournalProfileImportResult, JournalDirectoryError> {
        if !path.is_file() {
            return Err(JournalDirectoryError::InvalidCatalog(
                "期刊画像 JSONL 文件不存在".to_owned(),
            ));
        }
        let _ = self.load()?;
        let mut connection = self.open_database()?;
        let transaction = connection.transaction()?;
        let reader = BufReader::new(File::open(path)?);
        let mut imported = 0_u32;
        let mut unmatched = 0_u32;
        for (index, line) in reader.lines().enumerate() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }
            let value: serde_json::Value = serde_json::from_str(&line).map_err(|error| {
                JournalDirectoryError::InvalidCatalog(format!(
                    "期刊画像第 {} 行不是有效 JSON：{error}",
                    index + 1
                ))
            })?;
            if import_profile_value(&transaction, &value)? {
                imported = imported.saturating_add(1);
            } else {
                unmatched = unmatched.saturating_add(1);
            }
        }
        transaction.commit()?;
        Ok(JournalProfileImportResult {
            imported_profile_count: imported,
            unmatched_profile_count: unmatched,
            summary: self.summary()?,
        })
    }

    pub fn profile_for_identity(
        &self,
        title: &str,
        issn: Option<&str>,
        eissn: Option<&str>,
    ) -> Result<Option<JournalDirectoryProfile>, JournalDirectoryError> {
        let _ = self.load()?;
        let connection = self.open_database()?;
        let Some(journal_id) = find_journal_id(&connection, title, issn, eissn)? else {
            return Ok(None);
        };
        load_profile(&connection, &journal_id)
    }

    fn write_catalog(
        &self,
        catalog: &JournalDirectoryCatalog,
    ) -> Result<(), JournalDirectoryError> {
        fs::create_dir_all(&self.root)?;
        let existing_profiles = if self.database_path().exists() {
            let connection = self.open_database()?;
            read_profile_raw_values(&connection)?
        } else {
            Vec::new()
        };
        let mut connection = Connection::open(self.database_path())?;
        initialize_database(&connection)?;
        let transaction = connection.transaction()?;
        transaction.execute_batch(
            "DELETE FROM journal_profiles;
             DELETE FROM metric_records;
             DELETE FROM journal_identifiers;
             DELETE FROM journal_aliases;
             DELETE FROM journals;
             DELETE FROM directory_sources;",
        )?;
        write_catalog_transaction(&transaction, catalog)?;
        for profile in &existing_profiles {
            let _ = import_profile_value(&transaction, profile)?;
        }
        transaction.commit()?;
        Ok(())
    }

    fn open_database(&self) -> Result<Connection, JournalDirectoryError> {
        let connection = Connection::open(self.database_path())?;
        initialize_database(&connection)?;
        Ok(connection)
    }

    fn load_database(&self) -> Result<JournalDirectoryCatalog, JournalDirectoryError> {
        let connection = self.open_database()?;
        read_catalog_database(&connection)
    }
}

fn initialize_database(connection: &Connection) -> Result<(), JournalDirectoryError> {
    connection.execute_batch(
        "PRAGMA foreign_keys = ON;
         CREATE TABLE IF NOT EXISTS metadata (
           key TEXT PRIMARY KEY,
           value TEXT NOT NULL
         );
         CREATE TABLE IF NOT EXISTS directory_sources (
           source_id TEXT PRIMARY KEY,
           file_name TEXT NOT NULL,
           sha256 TEXT NOT NULL UNIQUE,
           imported_unix_ms INTEGER NOT NULL,
           active INTEGER NOT NULL CHECK (active IN (0, 1)),
           data_origin TEXT NOT NULL,
           sheet_names_json TEXT NOT NULL,
           formula_cell_count INTEGER NOT NULL,
           record_count INTEGER NOT NULL
         );
         CREATE TABLE IF NOT EXISTS journals (
           journal_id TEXT PRIMARY KEY,
           canonical_name TEXT NOT NULL,
           normalized_name TEXT NOT NULL,
           issn TEXT,
           eissn TEXT
         );
         CREATE UNIQUE INDEX IF NOT EXISTS journals_normalized_name_idx
           ON journals(normalized_name);
         CREATE INDEX IF NOT EXISTS journals_issn_idx ON journals(issn);
         CREATE INDEX IF NOT EXISTS journals_eissn_idx ON journals(eissn);
         CREATE TABLE IF NOT EXISTS journal_aliases (
           journal_id TEXT NOT NULL REFERENCES journals(journal_id) ON DELETE CASCADE,
           display_name TEXT NOT NULL,
           normalized_name TEXT NOT NULL,
           PRIMARY KEY (journal_id, normalized_name)
         );
         CREATE INDEX IF NOT EXISTS journal_aliases_name_idx
           ON journal_aliases(normalized_name);
         CREATE TABLE IF NOT EXISTS journal_identifiers (
           journal_id TEXT NOT NULL REFERENCES journals(journal_id) ON DELETE CASCADE,
           identifier_type TEXT NOT NULL CHECK (identifier_type IN ('issn', 'eissn')),
           value TEXT NOT NULL,
           PRIMARY KEY (journal_id, identifier_type, value),
           UNIQUE (value)
         );
         CREATE INDEX IF NOT EXISTS journal_identifiers_lookup_idx
           ON journal_identifiers(value);
         CREATE TABLE IF NOT EXISTS metric_records (
           record_id TEXT PRIMARY KEY,
           journal_id TEXT NOT NULL REFERENCES journals(journal_id) ON DELETE CASCADE,
           source_id TEXT NOT NULL REFERENCES directory_sources(source_id) ON DELETE CASCADE,
           sheet_name TEXT NOT NULL,
           row_number INTEGER NOT NULL,
           scheme TEXT NOT NULL,
           release_year INTEGER NOT NULL,
           metric_year INTEGER,
           journal_name TEXT NOT NULL,
           normalized_name TEXT NOT NULL,
           issn TEXT,
           eissn TEXT,
           category TEXT,
           partition INTEGER,
           top INTEGER,
           open_access INTEGER,
           jif REAL,
           total_citations INTEGER,
           jif_rank TEXT,
           value_basis TEXT NOT NULL
         );
         CREATE INDEX IF NOT EXISTS metric_records_journal_idx
           ON metric_records(journal_id);
         CREATE INDEX IF NOT EXISTS metric_records_source_idx
           ON metric_records(source_id);
         CREATE INDEX IF NOT EXISTS metric_records_scheme_year_idx
           ON metric_records(scheme, release_year);
         CREATE INDEX IF NOT EXISTS metric_records_issn_idx ON metric_records(issn);
         CREATE INDEX IF NOT EXISTS metric_records_eissn_idx ON metric_records(eissn);
         CREATE TABLE IF NOT EXISTS journal_profiles (
           journal_id TEXT PRIMARY KEY REFERENCES journals(journal_id) ON DELETE CASCADE,
           publisher TEXT,
           homepage_url TEXT,
           publication_scope_note TEXT,
           scope_evidence_available INTEGER NOT NULL DEFAULT 0 CHECK (scope_evidence_available IN (0, 1)),
           aims_scope_url TEXT,
           author_instructions_url TEXT,
           reported_print_circulation INTEGER,
           circulation_status TEXT,
           annual_publication_volume INTEGER,
           annual_publication_volume_year INTEGER,
           trailing_three_year_average REAL,
           peer_review_process_json TEXT,
           peer_review_policy_url TEXT,
           average_review_days REAL,
           review_speed_status TEXT,
           submission_to_publication_days REAL,
           publication_frequency TEXT,
           apc_status TEXT,
           open_access_status TEXT,
           source_url TEXT,
           retrieved_at TEXT,
           raw_json TEXT NOT NULL
         );",
    )?;
    ensure_database_column(
        connection,
        "journal_profiles",
        "scope_evidence_available",
        "INTEGER NOT NULL DEFAULT 0 CHECK (scope_evidence_available IN (0, 1))",
    )?;
    Ok(())
}

fn ensure_database_column(
    connection: &Connection,
    table: &str,
    column: &str,
    declaration: &str,
) -> Result<(), JournalDirectoryError> {
    let mut statement = connection.prepare(&format!("PRAGMA table_info({table})"))?;
    let columns = statement
        .query_map([], |row| row.get::<_, String>(1))?
        .collect::<Result<BTreeSet<_>, _>>()?;
    drop(statement);
    if !columns.contains(column) {
        connection.execute_batch(&format!(
            "ALTER TABLE {table} ADD COLUMN {column} {declaration}"
        ))?;
    }
    Ok(())
}

fn write_catalog_transaction(
    transaction: &Transaction<'_>,
    catalog: &JournalDirectoryCatalog,
) -> Result<(), JournalDirectoryError> {
    transaction.execute(
        "INSERT OR REPLACE INTO metadata(key, value) VALUES ('schema_version', ?1)",
        [JOURNAL_DIRECTORY_SCHEMA_VERSION.to_string()],
    )?;
    transaction.execute(
        "INSERT OR REPLACE INTO metadata(key, value) VALUES ('updated_unix_ms', ?1)",
        [catalog.updated_unix_ms.to_string()],
    )?;
    let active_sources = catalog
        .sources
        .iter()
        .filter(|source| source.active)
        .collect::<Vec<_>>();
    let active_ids = active_sources
        .iter()
        .map(|source| source.source_id.as_str())
        .collect::<BTreeSet<_>>();
    for source in active_sources {
        transaction.execute(
            "INSERT INTO directory_sources (
               source_id, file_name, sha256, imported_unix_ms, active, data_origin,
               sheet_names_json, formula_cell_count, record_count
             ) VALUES (?1, ?2, ?3, ?4, 1, ?5, ?6, ?7, ?8)",
            params![
                source.source_id,
                source.file_name,
                source.sha256,
                source.imported_unix_ms,
                source.data_origin,
                serde_json::to_string(&source.sheet_names)
                    .map_err(|error| JournalDirectoryError::InvalidCatalog(error.to_string()))?,
                source.formula_cell_count,
                source.record_count,
            ],
        )?;
    }
    for record in catalog
        .records
        .iter()
        .filter(|record| active_ids.contains(record.source_id.as_str()))
    {
        let journal_id = ensure_journal(transaction, record)?;
        transaction.execute(
            "INSERT INTO metric_records (
               record_id, journal_id, source_id, sheet_name, row_number, scheme,
               release_year, metric_year, journal_name, normalized_name, issn, eissn,
               category, partition, top, open_access, jif, total_citations, jif_rank,
               value_basis
             ) VALUES (
               ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14,
               ?15, ?16, ?17, ?18, ?19, ?20
             )",
            params![
                record.record_id,
                journal_id,
                record.source_id,
                record.sheet_name,
                record.row_number,
                record.scheme.label(),
                record.release_year,
                record.metric_year,
                record.journal_name,
                record.normalized_name,
                record.issn,
                record.eissn,
                record.category,
                record.partition,
                record.top.map(i64::from),
                record.open_access.map(i64::from),
                record.jif,
                record.total_citations,
                record.jif_rank,
                record.value_basis,
            ],
        )?;
    }
    Ok(())
}

fn ensure_journal(
    connection: &Connection,
    record: &JournalDirectoryRecord,
) -> Result<String, JournalDirectoryError> {
    let found_by_identifier =
        record
            .issn
            .iter()
            .chain(record.eissn.iter())
            .find_map(|identifier| {
                connection
                    .query_row(
                        "SELECT journal_id FROM journal_identifiers WHERE value = ?1",
                        [identifier],
                        |row| row.get::<_, String>(0),
                    )
                    .optional()
                    .ok()
                    .flatten()
            });
    let found_by_name = connection
        .query_row(
            "SELECT journal_id FROM journal_aliases WHERE normalized_name = ?1 LIMIT 1",
            [&record.normalized_name],
            |row| row.get::<_, String>(0),
        )
        .optional()?;
    let journal_id = found_by_identifier.or(found_by_name).unwrap_or_else(|| {
        format!(
            "jrn-{}",
            hex::encode(Sha256::digest(record.normalized_name.as_bytes()))
                .chars()
                .take(20)
                .collect::<String>()
        )
    });
    connection.execute(
        "INSERT INTO journals(journal_id, canonical_name, normalized_name, issn, eissn)
         VALUES (?1, ?2, ?3, ?4, ?5)
         ON CONFLICT(journal_id) DO UPDATE SET
           issn = COALESCE(journals.issn, excluded.issn),
           eissn = COALESCE(journals.eissn, excluded.eissn)",
        params![
            journal_id,
            record.journal_name,
            record.normalized_name,
            record.issn,
            record.eissn,
        ],
    )?;
    connection.execute(
        "INSERT OR IGNORE INTO journal_aliases(journal_id, display_name, normalized_name)
         VALUES (?1, ?2, ?3)",
        params![journal_id, record.journal_name, record.normalized_name],
    )?;
    if let Some(issn) = &record.issn {
        connection.execute(
            "INSERT OR IGNORE INTO journal_identifiers(journal_id, identifier_type, value)
             VALUES (?1, 'issn', ?2)",
            params![journal_id, issn],
        )?;
    }
    if let Some(eissn) = &record.eissn {
        connection.execute(
            "INSERT OR IGNORE INTO journal_identifiers(journal_id, identifier_type, value)
             VALUES (?1, 'eissn', ?2)",
            params![journal_id, eissn],
        )?;
    }
    Ok(journal_id)
}

fn read_catalog_database(
    connection: &Connection,
) -> Result<JournalDirectoryCatalog, JournalDirectoryError> {
    let schema_version = metadata_u64(connection, "schema_version")? as u32;
    if schema_version != JOURNAL_DIRECTORY_SCHEMA_VERSION {
        return Err(JournalDirectoryError::InvalidCatalog(format!(
            "目录版本 {schema_version} 与当前应用不兼容"
        )));
    }
    let updated_unix_ms = metadata_u64(connection, "updated_unix_ms")?;
    let mut source_statement = connection.prepare(
        "SELECT source_id, file_name, sha256, imported_unix_ms, active, data_origin,
                sheet_names_json, formula_cell_count, record_count
         FROM directory_sources WHERE active = 1 ORDER BY imported_unix_ms, source_id",
    )?;
    let sources = source_statement
        .query_map([], |row| {
            let sheet_names_json: String = row.get(6)?;
            Ok(JournalDirectorySource {
                source_id: row.get(0)?,
                file_name: row.get(1)?,
                sha256: row.get(2)?,
                imported_unix_ms: row.get(3)?,
                active: row.get::<_, i64>(4)? != 0,
                data_origin: row.get(5)?,
                sheet_names: serde_json::from_str(&sheet_names_json).unwrap_or_default(),
                formula_cell_count: row.get(7)?,
                record_count: row.get(8)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    let mut record_statement = connection.prepare(
        "SELECT record_id, source_id, sheet_name, row_number, scheme, release_year,
                metric_year, journal_name, normalized_name, issn, eissn, category,
                partition, top, open_access, jif, total_citations, jif_rank, value_basis
         FROM metric_records ORDER BY source_id, row_number, record_id",
    )?;
    let records = record_statement
        .query_map([], |row| {
            let scheme: String = row.get(4)?;
            Ok(JournalDirectoryRecord {
                record_id: row.get(0)?,
                source_id: row.get(1)?,
                sheet_name: row.get(2)?,
                row_number: row.get(3)?,
                scheme: scheme_from_label(&scheme).ok_or_else(|| {
                    rusqlite::Error::InvalidColumnType(
                        4,
                        "scheme".to_owned(),
                        rusqlite::types::Type::Text,
                    )
                })?,
                release_year: row.get(5)?,
                metric_year: row.get(6)?,
                journal_name: row.get(7)?,
                normalized_name: row.get(8)?,
                issn: row.get(9)?,
                eissn: row.get(10)?,
                category: row.get(11)?,
                partition: row.get(12)?,
                top: row.get::<_, Option<i64>>(13)?.map(|value| value != 0),
                open_access: row.get::<_, Option<i64>>(14)?.map(|value| value != 0),
                jif: row.get(15)?,
                total_citations: row.get(16)?,
                jif_rank: row.get(17)?,
                value_basis: row.get(18)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(JournalDirectoryCatalog {
        schema_version,
        updated_unix_ms,
        sources,
        records,
    })
}

fn metadata_u64(connection: &Connection, key: &str) -> Result<u64, JournalDirectoryError> {
    let value: String = connection
        .query_row("SELECT value FROM metadata WHERE key = ?1", [key], |row| {
            row.get(0)
        })
        .optional()?
        .ok_or_else(|| JournalDirectoryError::InvalidCatalog(format!("缺少数据库元数据：{key}")))?;
    value
        .parse()
        .map_err(|_| JournalDirectoryError::InvalidCatalog(format!("数据库元数据无效：{key}")))
}

fn scheme_from_label(value: &str) -> Option<JournalMetricScheme> {
    match value {
        "cas_partition" => Some(JournalMetricScheme::CasPartition),
        "clarivate_jcr" => Some(JournalMetricScheme::ClarivateJcr),
        "emerging_partition" => Some(JournalMetricScheme::EmergingPartition),
        _ => None,
    }
}

fn find_journal_id(
    connection: &Connection,
    title: &str,
    issn: Option<&str>,
    eissn: Option<&str>,
) -> Result<Option<String>, JournalDirectoryError> {
    for identifier in [issn, eissn]
        .into_iter()
        .flatten()
        .filter_map(normalize_issn)
    {
        if let Some(journal_id) = connection
            .query_row(
                "SELECT journal_id FROM journal_identifiers WHERE value = ?1",
                [&identifier],
                |row| row.get(0),
            )
            .optional()?
        {
            return Ok(Some(journal_id));
        }
    }
    let normalized = normalize_journal_name(title);
    if normalized.is_empty() {
        return Ok(None);
    }
    Ok(connection
        .query_row(
            "SELECT journal_id FROM journal_aliases WHERE normalized_name = ?1 LIMIT 1",
            [&normalized],
            |row| row.get(0),
        )
        .optional()?)
}

fn json_text(value: &serde_json::Value, pointer: &str) -> Option<String> {
    value
        .pointer(pointer)
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn json_u64(value: &serde_json::Value, pointer: &str) -> Option<u64> {
    value.pointer(pointer).and_then(serde_json::Value::as_u64)
}

fn json_f64(value: &serde_json::Value, pointer: &str) -> Option<f64> {
    value.pointer(pointer).and_then(serde_json::Value::as_f64)
}

fn import_profile_value(
    connection: &Connection,
    value: &serde_json::Value,
) -> Result<bool, JournalDirectoryError> {
    let title = json_text(value, "/title").unwrap_or_default();
    let identifiers = value
        .pointer("/issns")
        .and_then(serde_json::Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(serde_json::Value::as_str)
        .filter_map(normalize_issn)
        .collect::<Vec<_>>();
    let mut journal_id = None;
    for identifier in &identifiers {
        journal_id = connection
            .query_row(
                "SELECT journal_id FROM journal_identifiers WHERE value = ?1",
                [identifier],
                |row| row.get(0),
            )
            .optional()?;
        if journal_id.is_some() {
            break;
        }
    }
    if journal_id.is_none() {
        journal_id = find_journal_id(connection, &title, None, None)?;
    }
    let Some(journal_id) = journal_id else {
        return Ok(false);
    };
    let peer_review_process = value
        .pointer("/comparisonMetrics/peerReview/process")
        .and_then(serde_json::Value::as_array)
        .cloned()
        .unwrap_or_default();
    let scope_evidence_available = [
        "/publicationScope/frequentTopics",
        "/publicationScope/doajSubjects",
        "/publicationScope/doajKeywords",
        "/publicationScope/domains",
        "/publicationScope/fields",
    ]
    .into_iter()
    .any(|pointer| {
        value
            .pointer(pointer)
            .and_then(serde_json::Value::as_array)
            .is_some_and(|items| !items.is_empty())
    });
    let publication_days = json_f64(
        value,
        "/comparisonMetrics/publicationCycle/submissionToPublicationDays",
    )
    .or_else(|| {
        json_f64(
            value,
            "/comparisonMetrics/publicationCycle/submissionToPublicationWeeks",
        )
        .map(|weeks| weeks * 7.0)
    });
    connection.execute(
        "INSERT INTO journal_profiles (
           journal_id, publisher, homepage_url, publication_scope_note,
           scope_evidence_available, aims_scope_url, author_instructions_url,
           reported_print_circulation, circulation_status,
           annual_publication_volume, annual_publication_volume_year,
           trailing_three_year_average, peer_review_process_json, peer_review_policy_url,
           average_review_days, review_speed_status, submission_to_publication_days,
           publication_frequency, apc_status, open_access_status, source_url, retrieved_at,
           raw_json
         ) VALUES (
           ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15,
           ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23
         )
         ON CONFLICT(journal_id) DO UPDATE SET
           publisher = excluded.publisher,
           homepage_url = excluded.homepage_url,
           publication_scope_note = excluded.publication_scope_note,
           scope_evidence_available = excluded.scope_evidence_available,
           aims_scope_url = excluded.aims_scope_url,
           author_instructions_url = excluded.author_instructions_url,
           reported_print_circulation = excluded.reported_print_circulation,
           circulation_status = excluded.circulation_status,
           annual_publication_volume = excluded.annual_publication_volume,
           annual_publication_volume_year = excluded.annual_publication_volume_year,
           trailing_three_year_average = excluded.trailing_three_year_average,
           peer_review_process_json = excluded.peer_review_process_json,
           peer_review_policy_url = excluded.peer_review_policy_url,
           average_review_days = excluded.average_review_days,
           review_speed_status = excluded.review_speed_status,
           submission_to_publication_days = excluded.submission_to_publication_days,
           publication_frequency = excluded.publication_frequency,
           apc_status = excluded.apc_status,
           open_access_status = excluded.open_access_status,
           source_url = excluded.source_url,
           retrieved_at = excluded.retrieved_at,
           raw_json = excluded.raw_json",
        params![
            journal_id,
            json_text(value, "/publisher"),
            json_text(value, "/homepageUrl"),
            json_text(value, "/publicationScope/noteZh"),
            i64::from(scope_evidence_available),
            json_text(value, "/publicationScope/aimsAndScopeUrl"),
            json_text(value, "/publicationScope/authorInstructionsUrl"),
            json_u64(
                value,
                "/comparisonMetrics/circulation/reportedPrintCirculation"
            ),
            json_text(value, "/comparisonMetrics/circulation/status"),
            json_u64(
                value,
                "/comparisonMetrics/annualPublicationVolume/latestCompleteYearWorks"
            ),
            json_u64(
                value,
                "/comparisonMetrics/annualPublicationVolume/latestCompleteYear"
            ),
            json_f64(
                value,
                "/comparisonMetrics/annualPublicationVolume/trailingThreeCompleteYearsAverage"
            ),
            serde_json::to_string(&peer_review_process)
                .map_err(|error| JournalDirectoryError::InvalidCatalog(error.to_string()))?,
            json_text(value, "/comparisonMetrics/peerReview/policyUrl"),
            json_f64(value, "/comparisonMetrics/reviewSpeed/averageReviewDays"),
            json_text(value, "/comparisonMetrics/reviewSpeed/status"),
            publication_days,
            json_text(
                value,
                "/comparisonMetrics/publicationCycle/publicationFrequency"
            ),
            json_text(value, "/comparisonMetrics/fees/apcStatus"),
            json_text(value, "/comparisonMetrics/openAccessSupport/status"),
            json_text(value, "/provenance/sourceUrl"),
            json_text(value, "/provenance/retrievedAt"),
            serde_json::to_string(value)
                .map_err(|error| JournalDirectoryError::InvalidCatalog(error.to_string()))?,
        ],
    )?;
    Ok(true)
}

fn read_profile_raw_values(
    connection: &Connection,
) -> Result<Vec<serde_json::Value>, JournalDirectoryError> {
    let mut statement = connection.prepare("SELECT raw_json FROM journal_profiles")?;
    let rows = statement
        .query_map([], |row| row.get::<_, String>(0))?
        .collect::<Result<Vec<_>, _>>()?;
    rows.into_iter()
        .map(|raw| {
            serde_json::from_str(&raw)
                .map_err(|error| JournalDirectoryError::InvalidCatalog(error.to_string()))
        })
        .collect()
}

fn apply_profile_counts(
    connection: &Connection,
    summary: &mut JournalDirectorySummary,
) -> Result<(), JournalDirectoryError> {
    let counts = connection.query_row(
        "SELECT
           (SELECT COUNT(*) FROM journals),
           (SELECT COUNT(issn) FROM journals),
           (SELECT COUNT(eissn) FROM journals),
           COUNT(p.publisher),
           COALESCE(SUM(p.scope_evidence_available), 0),
           COUNT(p.annual_publication_volume),
           COALESCE(SUM(CASE WHEN p.peer_review_process_json IS NOT NULL
                         AND p.peer_review_process_json != '[]' THEN 1 ELSE 0 END), 0),
           COUNT(p.average_review_days),
           COUNT(p.submission_to_publication_days),
           COUNT(p.reported_print_circulation),
           COALESCE(SUM(CASE WHEN p.open_access_status IS NOT NULL
                         AND p.open_access_status != 'unknown' THEN 1 ELSE 0 END), 0)
         FROM journal_profiles p",
        [],
        |row| {
            Ok((
                row.get::<_, u32>(0)?,
                row.get::<_, u32>(1)?,
                row.get::<_, u32>(2)?,
                row.get::<_, u32>(3)?,
                row.get::<_, u32>(4)?,
                row.get::<_, u32>(5)?,
                row.get::<_, u32>(6)?,
                row.get::<_, u32>(7)?,
                row.get::<_, u32>(8)?,
                row.get::<_, u32>(9)?,
                row.get::<_, u32>(10)?,
            ))
        },
    )?;
    summary.distinct_journal_count = counts.0;
    summary.issn_count = counts.1;
    summary.eissn_count = counts.2;
    summary.publisher_count = counts.3;
    summary.scope_count = counts.4;
    summary.annual_volume_count = counts.5;
    summary.review_process_count = counts.6;
    summary.review_speed_count = counts.7;
    summary.publication_cycle_count = counts.8;
    summary.circulation_count = counts.9;
    summary.oa_status_count = counts.10;
    Ok(())
}

fn load_profile(
    connection: &Connection,
    journal_id: &str,
) -> Result<Option<JournalDirectoryProfile>, JournalDirectoryError> {
    Ok(connection
        .query_row(
            "SELECT j.journal_id, j.canonical_name, j.issn, j.eissn,
                    p.publisher, p.homepage_url, p.publication_scope_note, p.aims_scope_url,
                    p.author_instructions_url, p.reported_print_circulation,
                    p.annual_publication_volume, p.annual_publication_volume_year,
                    p.peer_review_process_json, p.peer_review_policy_url,
                    p.average_review_days, p.submission_to_publication_days,
                    p.publication_frequency, p.apc_status, p.open_access_status,
                    p.source_url, p.retrieved_at
             FROM journals j
             LEFT JOIN journal_profiles p ON p.journal_id = j.journal_id
             WHERE j.journal_id = ?1",
            [journal_id],
            |row| {
                let process_json: Option<String> = row.get(12)?;
                Ok(JournalDirectoryProfile {
                    journal_id: row.get(0)?,
                    journal_name: row.get(1)?,
                    issn: row.get(2)?,
                    eissn: row.get(3)?,
                    publisher: row.get(4)?,
                    homepage_url: row.get(5)?,
                    publication_scope_note: row.get(6)?,
                    aims_scope_url: row.get(7)?,
                    author_instructions_url: row.get(8)?,
                    reported_print_circulation: row.get(9)?,
                    annual_publication_volume: row.get(10)?,
                    annual_publication_volume_year: row.get(11)?,
                    peer_review_process: process_json
                        .and_then(|value| serde_json::from_str(&value).ok())
                        .unwrap_or_default(),
                    peer_review_policy_url: row.get(13)?,
                    average_review_days: row.get(14)?,
                    submission_to_publication_days: row.get(15)?,
                    publication_frequency: row.get(16)?,
                    apc_status: row.get(17)?,
                    open_access_status: row.get(18)?,
                    source_url: row.get(19)?,
                    retrieved_at: row.get(20)?,
                })
            },
        )
        .optional()?)
}

fn records_are_emerging_2026<'a>(
    records: impl Iterator<Item = &'a JournalDirectoryRecord>,
) -> bool {
    let mut found = false;
    for record in records {
        found = true;
        if record.scheme != JournalMetricScheme::EmergingPartition || record.release_year != 2026 {
            return false;
        }
    }
    found
}

impl JournalDirectoryCatalog {
    pub fn summary(&self) -> JournalDirectorySummary {
        let active_sources = self
            .sources
            .iter()
            .filter(|source| source.active)
            .collect::<Vec<_>>();
        let active_ids = active_sources
            .iter()
            .map(|source| source.source_id.as_str())
            .collect::<BTreeSet<_>>();
        let active_records = self
            .records
            .iter()
            .filter(|record| active_ids.contains(record.source_id.as_str()))
            .collect::<Vec<_>>();
        let distinct_journals = active_records
            .iter()
            .map(|record| record.normalized_name.as_str())
            .collect::<BTreeSet<_>>();
        let distinct_issns = active_records
            .iter()
            .filter_map(|record| record.issn.as_deref())
            .collect::<BTreeSet<_>>();
        let distinct_eissns = active_records
            .iter()
            .filter_map(|record| record.eissn.as_deref())
            .collect::<BTreeSet<_>>();
        let mut records_by_scheme = BTreeMap::new();
        let mut partition_counts = BTreeMap::new();
        let mut top_count = 0_u32;
        let mut open_access_count = 0_u32;
        for record in &active_records {
            *records_by_scheme
                .entry(record.scheme.label().to_owned())
                .or_insert(0) += 1;
            if let Some(partition) = record.partition {
                *partition_counts
                    .entry(format!("{}:{partition}", record.scheme.label()))
                    .or_insert(0) += 1;
            }
            top_count += u32::from(record.top == Some(true));
            open_access_count += u32::from(record.open_access == Some(true));
        }
        let fingerprint = if active_sources.is_empty() {
            None
        } else {
            let mut hashes = active_sources
                .iter()
                .map(|source| source.sha256.as_str())
                .collect::<Vec<_>>();
            hashes.sort_unstable();
            Some(
                hex::encode(Sha256::digest(hashes.join("|").as_bytes()))
                    .chars()
                    .take(16)
                    .collect(),
            )
        };
        let mut warnings = Vec::new();
        if active_sources
            .iter()
            .any(|source| source.formula_cell_count > 0)
        {
            warnings.push(
                "含公式的工作表只读取文件内已保存的缓存结果；未执行公式或外部链接。".to_owned(),
            );
        }
        if records_by_scheme.contains_key("emerging_partition") {
            warnings.push(
                "新锐分区作为独立的用户提供数据体系保存，不等同于中科院或 JCR 分区。".to_owned(),
            );
        }
        if !active_sources.is_empty() {
            warnings.push(
                "导入数据的来源身份为用户提供的本地文件；用于离线辅助，不自动视为官方核验。"
                    .to_owned(),
            );
        }
        JournalDirectorySummary {
            schema_version: JOURNAL_DIRECTORY_SCHEMA_VERSION,
            available: !active_records.is_empty(),
            source_count: u32::try_from(active_sources.len()).unwrap_or(u32::MAX),
            record_count: u32::try_from(active_records.len()).unwrap_or(u32::MAX),
            distinct_journal_count: u32::try_from(distinct_journals.len()).unwrap_or(u32::MAX),
            issn_count: u32::try_from(distinct_issns.len()).unwrap_or(u32::MAX),
            eissn_count: u32::try_from(distinct_eissns.len()).unwrap_or(u32::MAX),
            publisher_count: 0,
            scope_count: 0,
            annual_volume_count: 0,
            review_process_count: 0,
            review_speed_count: 0,
            publication_cycle_count: 0,
            circulation_count: 0,
            oa_status_count: 0,
            latest_release_year: active_records
                .iter()
                .map(|record| record.release_year)
                .max(),
            records_by_scheme,
            partition_counts,
            top_count,
            open_access_count,
            formula_cell_count: active_sources
                .iter()
                .map(|source| source.formula_cell_count)
                .sum(),
            catalog_fingerprint: fingerprint,
            updated_unix_ms: self.updated_unix_ms,
            source_files: active_sources
                .iter()
                .map(|source| source.file_name.clone())
                .collect(),
            warnings,
        }
    }

    pub fn evidence_for_title(&self, title: &str) -> Vec<JournalDirectoryEvidence> {
        let normalized = normalize_journal_name(title);
        if normalized.is_empty() {
            return Vec::new();
        }
        let active_sources = self
            .sources
            .iter()
            .filter(|source| source.active)
            .map(|source| (source.source_id.as_str(), source))
            .collect::<BTreeMap<_, _>>();
        let mut records = self
            .records
            .iter()
            .filter(|record| {
                record.normalized_name == normalized
                    && active_sources.contains_key(record.source_id.as_str())
            })
            .collect::<Vec<_>>();
        records.sort_by(|left, right| {
            right
                .release_year
                .cmp(&left.release_year)
                .then_with(|| left.scheme.cmp(&right.scheme))
        });
        let mut seen = BTreeSet::new();
        records
            .into_iter()
            .filter(|record| seen.insert(record.scheme))
            .filter_map(|record| {
                let source = active_sources.get(record.source_id.as_str())?;
                Some(JournalDirectoryEvidence {
                    scheme: record.scheme,
                    release_year: record.release_year,
                    metric_year: record.metric_year,
                    issn: record.issn.clone(),
                    eissn: record.eissn.clone(),
                    partition: record.partition,
                    top: record.top,
                    open_access: record.open_access,
                    jif_tenths: record.jif.map(|value| (value * 10.0).round() as u32),
                    category: record.category.clone(),
                    source_file: source.file_name.clone(),
                    data_origin: source.data_origin.clone(),
                    value_basis: record.value_basis.clone(),
                })
            })
            .collect()
    }
}

struct ParsedWorkbook {
    source: JournalDirectorySource,
    records: Vec<JournalDirectoryRecord>,
}

#[derive(Default)]
struct ParsedSheet {
    rows: Vec<Vec<String>>,
    formula_cell_count: u32,
}

fn parse_workbook(path: &Path) -> Result<ParsedWorkbook, JournalDirectoryError> {
    if path.extension().and_then(|value| value.to_str()) != Some("xlsx") {
        return Err(JournalDirectoryError::UnsupportedWorkbook(
            "仅支持 .xlsx 文件".to_owned(),
        ));
    }
    let metadata = fs::metadata(path)?;
    if metadata.len() == 0 || metadata.len() > MAX_WORKBOOK_BYTES {
        return Err(JournalDirectoryError::InvalidWorkbook(
            "文件为空或超过 64 MiB 限制".to_owned(),
        ));
    }
    let bytes = fs::read(path)?;
    let sha256 = hex::encode(Sha256::digest(&bytes));
    let source_id = format!("jds-{}", &sha256[..20]);
    let imported_unix_ms = unix_time_ms()?;
    let mut archive = ZipArchive::new(io::Cursor::new(bytes))
        .map_err(|error| JournalDirectoryError::InvalidWorkbook(error.to_string()))?;
    let shared_strings = read_shared_strings(&mut archive)?;
    let sheet_names = read_sheet_names(&mut archive)?;
    let mut records = Vec::new();
    let mut formula_cell_count = 0_u32;
    let mut imported_sheet_names = Vec::new();

    for (index, sheet_name) in sheet_names.iter().enumerate() {
        let member = format!("xl/worksheets/sheet{}.xml", index + 1);
        let sheet = match read_sheet(&mut archive, &member, &shared_strings) {
            Ok(sheet) => sheet,
            Err(JournalDirectoryError::InvalidWorkbook(message))
                if message.contains("工作表不存在") =>
            {
                continue;
            }
            Err(error) => return Err(error),
        };
        formula_cell_count = formula_cell_count.saturating_add(sheet.formula_cell_count);
        let parsed = parse_supported_sheet(&source_id, sheet_name, &sheet.rows)?;
        if !parsed.is_empty() {
            imported_sheet_names.push(sheet_name.clone());
            records.extend(parsed);
        }
    }
    if records.is_empty() {
        return Err(JournalDirectoryError::UnsupportedWorkbook(
            "未识别到中科院分区、JCR 或新锐分区字段".to_owned(),
        ));
    }
    let file_name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("journal-directory.xlsx")
        .to_owned();
    let record_count = u32::try_from(records.len()).unwrap_or(u32::MAX);
    Ok(ParsedWorkbook {
        source: JournalDirectorySource {
            source_id,
            file_name,
            sha256,
            imported_unix_ms,
            active: true,
            data_origin: "user_supplied_local_workbook".to_owned(),
            sheet_names: imported_sheet_names,
            formula_cell_count,
            record_count,
        },
        records,
    })
}

fn read_shared_strings<R: Read + io::Seek>(
    archive: &mut ZipArchive<R>,
) -> Result<Vec<String>, JournalDirectoryError> {
    let xml = match read_zip_member(archive, "xl/sharedStrings.xml") {
        Ok(xml) => xml,
        Err(JournalDirectoryError::InvalidWorkbook(message))
            if message.contains("工作表不存在") =>
        {
            return Ok(Vec::new());
        }
        Err(error) => return Err(error),
    };
    let mut reader = Reader::from_str(&xml);
    let mut values = Vec::new();
    let mut current = String::new();
    let mut in_item = false;
    let mut in_text = false;
    loop {
        match reader.read_event() {
            Ok(Event::Start(event)) if local_name(event.name().as_ref()) == b"si" => {
                in_item = true;
                current.clear();
            }
            Ok(Event::Start(event)) if in_item && local_name(event.name().as_ref()) == b"t" => {
                in_text = true;
            }
            Ok(Event::Text(text)) if in_text => {
                current.push_str(&decode_text(&reader, text.as_ref())?);
            }
            Ok(Event::End(event)) if local_name(event.name().as_ref()) == b"t" => {
                in_text = false;
            }
            Ok(Event::End(event)) if local_name(event.name().as_ref()) == b"si" => {
                values.push(current.clone());
                in_item = false;
                in_text = false;
            }
            Ok(Event::Eof) => break,
            Err(error) => {
                return Err(JournalDirectoryError::InvalidWorkbook(error.to_string()));
            }
            _ => {}
        }
    }
    Ok(values)
}

fn read_sheet_names<R: Read + io::Seek>(
    archive: &mut ZipArchive<R>,
) -> Result<Vec<String>, JournalDirectoryError> {
    let xml = read_zip_member(archive, "xl/workbook.xml")?;
    let mut reader = Reader::from_str(&xml);
    let mut names = Vec::new();
    loop {
        match reader.read_event() {
            Ok(Event::Start(event)) | Ok(Event::Empty(event))
                if local_name(event.name().as_ref()) == b"sheet" =>
            {
                if let Some(name) = attribute_value(&event, b"name")? {
                    names.push(name);
                }
            }
            Ok(Event::Eof) => break,
            Err(error) => {
                return Err(JournalDirectoryError::InvalidWorkbook(error.to_string()));
            }
            _ => {}
        }
    }
    Ok(names)
}

fn read_sheet<R: Read + io::Seek>(
    archive: &mut ZipArchive<R>,
    member: &str,
    shared_strings: &[String],
) -> Result<ParsedSheet, JournalDirectoryError> {
    let xml = read_zip_member(archive, member)?;
    let mut reader = Reader::from_str(&xml);
    reader.config_mut().trim_text(false);
    let mut sheet = ParsedSheet::default();
    let mut row = Vec::new();
    let mut cell_column = None;
    let mut cell_type = String::new();
    let mut cell_value = String::new();
    let mut in_value = false;
    let mut in_inline_text = false;
    loop {
        match reader.read_event() {
            Ok(Event::Start(event)) => match local_name(event.name().as_ref()) {
                b"row" => row.clear(),
                b"c" => {
                    cell_column = attribute_value(&event, b"r")?
                        .as_deref()
                        .and_then(column_index);
                    cell_type = attribute_value(&event, b"t")?.unwrap_or_default();
                    cell_value.clear();
                }
                b"v" => in_value = true,
                b"t" if cell_type == "inlineStr" => in_inline_text = true,
                b"f" => sheet.formula_cell_count = sheet.formula_cell_count.saturating_add(1),
                _ => {}
            },
            Ok(Event::Text(text)) if in_value || in_inline_text => {
                cell_value.push_str(&decode_text(&reader, text.as_ref())?);
            }
            Ok(Event::End(event)) => match local_name(event.name().as_ref()) {
                b"v" => in_value = false,
                b"t" => in_inline_text = false,
                b"c" => {
                    if let Some(column) = cell_column {
                        if column < MAX_COLUMNS_PER_ROW {
                            if row.len() <= column {
                                row.resize(column + 1, String::new());
                            }
                            row[column] = if cell_type == "s" {
                                cell_value
                                    .parse::<usize>()
                                    .ok()
                                    .and_then(|index| shared_strings.get(index))
                                    .cloned()
                                    .unwrap_or_default()
                            } else {
                                cell_value.clone()
                            };
                        }
                    }
                    cell_column = None;
                }
                b"row" => {
                    sheet.rows.push(row.clone());
                    if sheet.rows.len() > MAX_ROWS_PER_SHEET {
                        return Err(JournalDirectoryError::InvalidWorkbook(
                            "工作表超过 100,000 行限制".to_owned(),
                        ));
                    }
                }
                _ => {}
            },
            Ok(Event::Eof) => break,
            Err(error) => {
                return Err(JournalDirectoryError::InvalidWorkbook(error.to_string()));
            }
            _ => {}
        }
    }
    Ok(sheet)
}

fn parse_supported_sheet(
    source_id: &str,
    sheet_name: &str,
    rows: &[Vec<String>],
) -> Result<Vec<JournalDirectoryRecord>, JournalDirectoryError> {
    let Some(headers) = rows.first() else {
        return Ok(Vec::new());
    };
    let header = |name: &str| headers.iter().position(|value| value.trim() == name);
    let mut records = Vec::new();
    if let (Some(name), Some(partition), Some(top), Some(open_access)) = (
        header("期刊名称"),
        header("2025分区"),
        header("Top"),
        header("Open Access"),
    ) {
        for (offset, row) in rows.iter().enumerate().skip(1) {
            let journal_name = value(row, name);
            if journal_name.is_empty() {
                continue;
            }
            records.push(build_record(
                source_id,
                sheet_name,
                offset + 1,
                JournalMetricScheme::CasPartition,
                2025,
                None,
                journal_name,
                None,
                None,
                None,
                parse_partition(value(row, partition)),
                parse_bool(value(row, top)),
                parse_bool(value(row, open_access)),
                None,
                None,
                None,
                "stored_cell_value",
            ));
        }
    } else if let (Some(name), Some(issn), Some(eissn), Some(category), Some(jif)) = (
        header("期刊名"),
        header("ISSN"),
        header("eISSN"),
        header("Category"),
        header("2024JIF"),
    ) {
        let total_citations = header("Total citation");
        let quartile = header("2024分区").or_else(|| header("Quartile"));
        let jif_rank = header("JIF rank");
        for (offset, row) in rows.iter().enumerate().skip(1) {
            let journal_name = value(row, name);
            if journal_name.is_empty() {
                continue;
            }
            records.push(build_record(
                source_id,
                sheet_name,
                offset + 1,
                JournalMetricScheme::ClarivateJcr,
                2025,
                Some(2024),
                journal_name,
                normalize_issn(value(row, issn)),
                normalize_issn(value(row, eissn)),
                optional_text(value(row, category)),
                quartile.and_then(|index| parse_partition(value(row, index))),
                None,
                None,
                value(row, jif).parse().ok(),
                total_citations.and_then(|index| value(row, index).parse().ok()),
                jif_rank.and_then(|index| optional_text(value(row, index))),
                if header("2024分区").is_some() {
                    "cached_formula_value"
                } else {
                    "stored_cell_value"
                },
            ));
        }
    } else if let (
        Some(name),
        Some(issn),
        Some(eissn),
        Some(category),
        Some(partition),
        Some(top),
    ) = (
        header("刊名"),
        header("ISSN"),
        header("EISSN"),
        header("类型"),
        header("新锐分区"),
        header("TOP"),
    ) {
        for (offset, row) in rows.iter().enumerate().skip(1) {
            let journal_name = value(row, name);
            if journal_name.is_empty() {
                continue;
            }
            records.push(build_record(
                source_id,
                sheet_name,
                offset + 1,
                JournalMetricScheme::EmergingPartition,
                2026,
                None,
                journal_name,
                normalize_issn(value(row, issn)),
                normalize_issn(value(row, eissn)),
                optional_text(value(row, category)),
                parse_partition(value(row, partition)),
                parse_bool(value(row, top)),
                None,
                None,
                None,
                None,
                "stored_cell_value",
            ));
        }
    } else if let (Some(category), Some(name), Some(issn), Some(eissn), Some(partition)) = (
        header("学科"),
        header("期刊名称"),
        header("issn1"),
        header("issn2"),
        header("分区"),
    ) {
        for (offset, row) in rows.iter().enumerate().skip(1) {
            let journal_name = value(row, name);
            if journal_name.is_empty() {
                continue;
            }
            records.push(build_record(
                source_id,
                sheet_name,
                offset + 1,
                JournalMetricScheme::EmergingPartition,
                2026,
                None,
                journal_name,
                normalize_issn(value(row, issn)),
                normalize_issn(value(row, eissn)),
                optional_text(value(row, category)),
                parse_partition(value(row, partition)),
                None,
                None,
                None,
                None,
                None,
                "stored_cell_value",
            ));
        }
    }
    Ok(records)
}

#[allow(clippy::too_many_arguments)]
fn build_record(
    source_id: &str,
    sheet_name: &str,
    row_number: usize,
    scheme: JournalMetricScheme,
    release_year: u16,
    metric_year: Option<u16>,
    journal_name: &str,
    issn: Option<String>,
    eissn: Option<String>,
    category: Option<String>,
    partition: Option<u8>,
    top: Option<bool>,
    open_access: Option<bool>,
    jif: Option<f64>,
    total_citations: Option<u64>,
    jif_rank: Option<String>,
    value_basis: &str,
) -> JournalDirectoryRecord {
    let normalized_name = normalize_journal_name(journal_name);
    let encoded = format!(
        "{source_id}|{sheet_name}|{row_number}|{}|{normalized_name}",
        scheme.label()
    );
    JournalDirectoryRecord {
        record_id: format!(
            "jdr-{}",
            hex::encode(Sha256::digest(encoded.as_bytes()))
                .chars()
                .take(20)
                .collect::<String>()
        ),
        source_id: source_id.to_owned(),
        sheet_name: sheet_name.to_owned(),
        row_number: u32::try_from(row_number).unwrap_or(u32::MAX),
        scheme,
        release_year,
        metric_year,
        journal_name: journal_name.trim().to_owned(),
        normalized_name,
        issn,
        eissn,
        category,
        partition,
        top,
        open_access,
        jif,
        total_citations,
        jif_rank,
        value_basis: value_basis.to_owned(),
    }
}

fn read_zip_member<R: Read + io::Seek>(
    archive: &mut ZipArchive<R>,
    name: &str,
) -> Result<String, JournalDirectoryError> {
    let mut member = archive
        .by_name(name)
        .map_err(|_| JournalDirectoryError::InvalidWorkbook(format!("工作表不存在：{name}")))?;
    if member.size() > MAX_XML_BYTES {
        return Err(JournalDirectoryError::InvalidWorkbook(format!(
            "XML 部件过大：{name}"
        )));
    }
    let mut xml = String::new();
    member.read_to_string(&mut xml)?;
    Ok(xml)
}

fn decode_text(reader: &Reader<&[u8]>, bytes: &[u8]) -> Result<String, JournalDirectoryError> {
    let decoded = reader
        .decoder()
        .decode(bytes)
        .map_err(|error| JournalDirectoryError::InvalidWorkbook(error.to_string()))?;
    unescape(&decoded)
        .map(|value| value.into_owned())
        .map_err(|error| JournalDirectoryError::InvalidWorkbook(error.to_string()))
}

fn attribute_value(
    event: &quick_xml::events::BytesStart<'_>,
    wanted_local_name: &[u8],
) -> Result<Option<String>, JournalDirectoryError> {
    for attribute in event.attributes() {
        let attribute =
            attribute.map_err(|error| JournalDirectoryError::InvalidWorkbook(error.to_string()))?;
        if local_name(attribute.key.as_ref()) == wanted_local_name {
            return Ok(Some(
                String::from_utf8_lossy(attribute.value.as_ref()).into_owned(),
            ));
        }
    }
    Ok(None)
}

fn local_name(name: &[u8]) -> &[u8] {
    name.rsplit(|byte| *byte == b':').next().unwrap_or(name)
}

fn column_index(reference: &str) -> Option<usize> {
    let mut result = 0_usize;
    let mut found = false;
    for byte in reference.bytes() {
        if !byte.is_ascii_alphabetic() {
            break;
        }
        found = true;
        result = result
            .checked_mul(26)?
            .checked_add((byte.to_ascii_uppercase() - b'A' + 1) as usize)?;
    }
    found.then_some(result.saturating_sub(1))
}

fn value(row: &[String], index: usize) -> &str {
    row.get(index).map(String::as_str).unwrap_or("").trim()
}

fn optional_text(value: &str) -> Option<String> {
    let value = value.trim();
    (!value.is_empty() && !matches!(value, "-" | "N/A" | "n/a")).then(|| value.to_owned())
}

pub fn normalize_issn(value: &str) -> Option<String> {
    let normalized = value.trim().to_ascii_uppercase();
    if normalized.len() == 9
        && normalized.as_bytes()[4] == b'-'
        && normalized
            .chars()
            .enumerate()
            .all(|(index, character)| index == 4 || character.is_ascii_digit() || character == 'X')
    {
        Some(normalized)
    } else {
        None
    }
}

fn parse_partition(value: &str) -> Option<u8> {
    value
        .trim()
        .trim_start_matches(['Q', 'q'])
        .trim_end_matches('区')
        .trim()
        .parse::<u8>()
        .ok()
        .filter(|partition| (1..=4).contains(partition))
}

fn parse_bool(value: &str) -> Option<bool> {
    match value.trim().to_ascii_lowercase().as_str() {
        "是" | "top" | "yes" | "true" | "1" => Some(true),
        "否" | "no" | "false" | "0" | "非top" => Some(false),
        _ => None,
    }
}

pub fn normalize_journal_name(value: &str) -> String {
    value
        .chars()
        .filter_map(|character| {
            if character.is_alphanumeric() {
                character.to_lowercase().next()
            } else {
                None
            }
        })
        .collect()
}

fn unix_time_ms() -> Result<u64, JournalDirectoryError> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .map_err(|error| JournalDirectoryError::InvalidCatalog(error.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use zip::{write::SimpleFileOptions, ZipWriter};

    #[test]
    fn normalizes_titles_and_partitions() {
        assert_eq!(
            normalize_journal_name("IEEE Transactions on PAMI"),
            "ieeetransactionsonpami"
        );
        assert_eq!(parse_partition("Q1"), Some(1));
        assert_eq!(parse_partition("2区"), Some(2));
        assert_eq!(parse_partition("1 区"), Some(1));
        assert_eq!(parse_partition("Q5"), None);
    }

    #[test]
    fn imports_the_complete_emerging_directory_layout() {
        let rows = vec![
            vec![
                "学科".into(),
                "序号".into(),
                "期刊名称".into(),
                "issn1".into(),
                "issn2".into(),
                "分区".into(),
            ],
            vec![
                "计算机科学".into(),
                "1".into(),
                "Example Journal".into(),
                "1234-5678".into(),
                "8765-4321".into(),
                "1 区".into(),
            ],
        ];

        let records = parse_supported_sheet("jds-example", "Sheet1", &rows)
            .expect("complete emerging layout should parse");
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].scheme, JournalMetricScheme::EmergingPartition);
        assert_eq!(records[0].release_year, 2026);
        assert_eq!(records[0].category.as_deref(), Some("计算机科学"));
        assert_eq!(records[0].partition, Some(1));
        assert_eq!(records[0].top, None);
    }

    #[test]
    fn rejects_invalid_issn_values() {
        assert_eq!(normalize_issn("2041-1723"), Some("2041-1723".to_owned()));
        assert_eq!(normalize_issn("1234-567X"), Some("1234-567X".to_owned()));
        assert_eq!(normalize_issn("-"), None);
        assert_eq!(normalize_issn("N/A"), None);
    }

    #[test]
    fn indexes_issn_and_eissn_and_preserves_evidence_backed_profiles() {
        let temporary_root =
            std::env::temp_dir().join(format!("journal-profile-{}", Uuid::new_v4()));
        let store = JournalDirectoryStore::new(temporary_root.join("store"));
        let source = JournalDirectorySource {
            source_id: "jds-profile-test".into(),
            file_name: "synthetic-jcr.xlsx".into(),
            sha256: "abc123".into(),
            imported_unix_ms: 1,
            active: true,
            data_origin: "synthetic_test_fixture".into(),
            sheet_names: vec!["JCR".into()],
            formula_cell_count: 0,
            record_count: 1,
        };
        let record = build_record(
            &source.source_id,
            "JCR",
            2,
            JournalMetricScheme::ClarivateJcr,
            2025,
            Some(2024),
            "Example Journal",
            Some("1234-5678".into()),
            Some("8765-4321".into()),
            Some("COMPUTER SCIENCE".into()),
            Some(1),
            None,
            None,
            Some(3.2),
            None,
            None,
            "stored_cell_value",
        );
        let catalog = JournalDirectoryCatalog {
            schema_version: JOURNAL_DIRECTORY_SCHEMA_VERSION,
            updated_unix_ms: 2,
            sources: vec![source],
            records: vec![record],
        };
        store
            .write_catalog(&catalog)
            .expect("database catalog should be written");
        let profiles_path = temporary_root.join("profiles.jsonl");
        fs::create_dir_all(&temporary_root).expect("temporary root should exist");
        fs::write(
            &profiles_path,
            serde_json::json!({
                "title": "Example Journal",
                "issns": ["1234-5678", "8765-4321"],
                "publisher": "Example Publisher",
                "publicationScope": {
                    "frequentTopics": [{"name": "Machine learning"}],
                    "noteZh": "主要发表机器学习研究。",
                    "aimsAndScopeUrl": "https://example.test/scope"
                },
                "comparisonMetrics": {
                    "circulation": {"reportedPrintCirculation": null, "status": "unknown"},
                    "annualPublicationVolume": {"latestCompleteYear": 2025, "latestCompleteYearWorks": 42},
                    "peerReview": {"process": ["Double anonymous peer review"]},
                    "reviewSpeed": {"averageReviewDays": null, "status": "unknown"},
                    "publicationCycle": {"submissionToPublicationWeeks": 8},
                    "fees": {"apcStatus": "no_apc"},
                    "openAccessSupport": {"status": "fully_open_access"}
                },
                "provenance": {"sourceUrl": "https://example.test/source", "retrievedAt": "2026-09-03T00:00:00Z"}
            })
            .to_string(),
        )
        .expect("profile fixture should be written");
        let imported = store
            .import_profile_catalog(&profiles_path)
            .expect("profile should import");
        assert_eq!(imported.imported_profile_count, 1);
        assert_eq!(imported.summary.issn_count, 1);
        assert_eq!(imported.summary.eissn_count, 1);
        assert_eq!(imported.summary.scope_count, 1);
        assert_eq!(imported.summary.review_speed_count, 0);
        assert_eq!(imported.summary.publication_cycle_count, 1);
        assert_eq!(imported.summary.circulation_count, 0);
        let profile = store
            .profile_for_identity("Wrong title", None, Some("8765-4321"))
            .expect("profile lookup should succeed")
            .expect("EISSN should find the journal");
        assert_eq!(profile.issn.as_deref(), Some("1234-5678"));
        assert_eq!(profile.eissn.as_deref(), Some("8765-4321"));
        assert_eq!(profile.publisher.as_deref(), Some("Example Publisher"));
        assert_eq!(profile.submission_to_publication_days, Some(56.0));

        store
            .write_catalog(&catalog)
            .expect("catalog rewrite should preserve profile data");
        assert_eq!(
            store
                .summary()
                .expect("summary should load")
                .publisher_count,
            1
        );
        fs::remove_dir_all(temporary_root).expect("temporary root should be removed");
    }

    #[test]
    fn imports_a_synthetic_catalog_idempotently_without_evaluating_cells() {
        let temporary_root =
            std::env::temp_dir().join(format!("journal-directory-{}", Uuid::new_v4()));
        fs::create_dir_all(&temporary_root).expect("temporary root should be created");
        let workbook_path = temporary_root.join("synthetic.xlsx");
        let file = File::create(&workbook_path).expect("synthetic workbook should be created");
        let mut archive = ZipWriter::new(file);
        let options = SimpleFileOptions::default();
        archive
            .start_file("xl/workbook.xml", options)
            .expect("workbook member should start");
        archive
            .write_all(r#"<workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><sheets><sheet name="2025中科学院分区表" sheetId="1"/></sheets></workbook>"#.as_bytes())
            .expect("workbook xml should be written");
        archive
            .start_file("xl/worksheets/sheet1.xml", options)
            .expect("sheet member should start");
        archive
            .write_all(r#"<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><sheetData><row r="1"><c r="A1" t="inlineStr"><is><t>期刊名称</t></is></c><c r="B1" t="inlineStr"><is><t>2025分区</t></is></c><c r="C1" t="inlineStr"><is><t>Top</t></is></c><c r="D1" t="inlineStr"><is><t>Open Access</t></is></c></row><row r="2"><c r="A2" t="inlineStr"><is><t>Example Journal</t></is></c><c r="B2" t="inlineStr"><is><t>1</t></is></c><c r="C2" t="inlineStr"><is><t>是</t></is></c><c r="D2" t="inlineStr"><is><t>否</t></is></c></row></sheetData></worksheet>"#.as_bytes())
            .expect("sheet xml should be written");
        archive.finish().expect("synthetic workbook should finish");

        let store = JournalDirectoryStore::new(temporary_root.join("store"));
        let first = store
            .import_workbooks(std::slice::from_ref(&workbook_path))
            .expect("first import should succeed");
        assert_eq!(first.imported_record_count, 1);
        assert_eq!(first.summary.distinct_journal_count, 1);
        let second = store
            .import_workbooks(std::slice::from_ref(&workbook_path))
            .expect("second import should succeed");
        assert_eq!(second.imported_record_count, 0);
        assert_eq!(second.unchanged_source_count, 1);
        let evidence = store
            .load()
            .expect("catalog should load")
            .evidence_for_title("Example Journal");
        assert_eq!(evidence[0].partition, Some(1));
        assert_eq!(evidence[0].top, Some(true));
        fs::remove_dir_all(temporary_root).expect("temporary root should be removed");
    }
}
