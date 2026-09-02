use quick_xml::{escape::unescape, events::Event, Reader};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    error::Error,
    fmt,
    fs::{self, File},
    io::{self, Read, Write},
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};
use uuid::Uuid;
use zip::ZipArchive;

pub const JOURNAL_DIRECTORY_SCHEMA_VERSION: u32 = 1;
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
}

impl fmt::Display for JournalDirectoryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(formatter, "期刊目录读写失败：{error}"),
            Self::InvalidWorkbook(message) => write!(formatter, "期刊表格无效：{message}"),
            Self::UnsupportedWorkbook(message) => write!(formatter, "不支持的期刊表格：{message}"),
            Self::InvalidCatalog(message) => write!(formatter, "本地期刊目录无效：{message}"),
        }
    }
}

impl Error for JournalDirectoryError {}

impl From<io::Error> for JournalDirectoryError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
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

#[derive(Debug, Clone)]
pub struct JournalDirectoryStore {
    root: PathBuf,
}

impl JournalDirectoryStore {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub fn catalog_path(&self) -> PathBuf {
        self.root.join("catalog-v1.json")
    }

    pub fn load(&self) -> Result<JournalDirectoryCatalog, JournalDirectoryError> {
        let path = self.catalog_path();
        if !path.exists() {
            return Ok(JournalDirectoryCatalog::default());
        }
        let catalog: JournalDirectoryCatalog = serde_json::from_reader(File::open(path)?)
            .map_err(|error| JournalDirectoryError::InvalidCatalog(error.to_string()))?;
        if catalog.schema_version != JOURNAL_DIRECTORY_SCHEMA_VERSION {
            return Err(JournalDirectoryError::InvalidCatalog(
                "目录版本与当前应用不兼容".to_owned(),
            ));
        }
        Ok(catalog)
    }

    pub fn summary(&self) -> Result<JournalDirectorySummary, JournalDirectoryError> {
        Ok(self.load()?.summary())
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
            catalog.updated_unix_ms = unix_time_ms()?;
            self.write_catalog(&catalog)?;
        }
        Ok(JournalDirectoryImportResult {
            imported_source_count,
            imported_record_count,
            unchanged_source_count,
            summary: catalog.summary(),
        })
    }

    fn write_catalog(
        &self,
        catalog: &JournalDirectoryCatalog,
    ) -> Result<(), JournalDirectoryError> {
        fs::create_dir_all(&self.root)?;
        let temporary = self.root.join(format!(".{}.tmp", Uuid::new_v4()));
        let mut writer = File::create(&temporary)?;
        serde_json::to_writer(&mut writer, catalog)
            .map_err(|error| JournalDirectoryError::InvalidCatalog(error.to_string()))?;
        writer.flush()?;
        fs::rename(temporary, self.catalog_path())?;
        Ok(())
    }
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

fn normalize_issn(value: &str) -> Option<String> {
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
        assert_eq!(normalize_issn("-"), None);
        assert_eq!(normalize_issn("N/A"), None);
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
