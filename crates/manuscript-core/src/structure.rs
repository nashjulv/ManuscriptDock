use crate::{ManuscriptKind, ManuscriptSummary};
use lopdf::{Document, Object};
use quick_xml::{escape::unescape, events::Event, Reader};
use serde::{Deserialize, Serialize};
use std::{error::Error, fmt, fs::File, io::Read, path::Path};
use zip::ZipArchive;

pub const STRUCTURE_ANALYSIS_VERSION: u32 = 4;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AnalysisQuality {
    Complete,
    Limited,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SectionSummary {
    pub level: u8,
    pub heading: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StructureReport {
    pub analysis_version: u32,
    pub workspace_id: String,
    pub source_content_hash: String,
    pub source_snapshot_version: u32,
    pub quality: AnalysisQuality,
    pub title: Option<String>,
    pub authors: Vec<String>,
    pub abstract_present: bool,
    pub abstract_text: Option<String>,
    pub keywords_present: bool,
    pub sections: Vec<SectionSummary>,
    pub figure_count: u32,
    pub table_count: u32,
    pub references_present: bool,
    pub declarations: Vec<String>,
    pub page_count: Option<u32>,
    pub word_count: u64,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum StructureAnalysis {
    Completed { report: Box<StructureReport> },
    Rejected { message: String },
}

#[derive(Debug)]
pub enum StructureError {
    Io(std::io::Error),
    InvalidDocx(String),
    InvalidPdf(String),
    InvalidTextEncoding,
}

impl fmt::Display for StructureError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(formatter, "无法读取本地源快照：{error}"),
            Self::InvalidDocx(message) => write!(formatter, "DOCX 结构无法解析：{message}"),
            Self::InvalidPdf(message) => write!(formatter, "PDF 结构无法解析：{message}"),
            Self::InvalidTextEncoding => write!(formatter, "TEX 文件不是有效的 UTF-8 文本"),
        }
    }
}

impl Error for StructureError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            _ => None,
        }
    }
}

impl From<std::io::Error> for StructureError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}

#[derive(Default)]
struct ExtractedStructure {
    quality: Option<AnalysisQuality>,
    title: Option<String>,
    authors: Vec<String>,
    abstract_present: bool,
    abstract_text: Option<String>,
    abstract_inferred_from_front_matter: bool,
    keywords_present: bool,
    sections: Vec<SectionSummary>,
    figure_count: u32,
    table_count: u32,
    references_present: bool,
    declarations: Vec<String>,
    page_count: Option<u32>,
    word_count: u64,
    warnings: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PdfTextSource {
    EnhancedFontMapping,
    BasicContentStream,
    None,
}

pub(crate) fn extract_structure(
    snapshot_path: &Path,
    manuscript: &ManuscriptSummary,
    workspace_id: &str,
    content_hash: &str,
    snapshot_version: u32,
) -> Result<StructureReport, StructureError> {
    let extracted = match manuscript.kind {
        ManuscriptKind::Latex => extract_tex(snapshot_path)?,
        ManuscriptKind::Word => extract_docx(snapshot_path)?,
        ManuscriptKind::Pdf => extract_pdf(snapshot_path)?,
    };

    Ok(StructureReport {
        analysis_version: STRUCTURE_ANALYSIS_VERSION,
        workspace_id: workspace_id.to_owned(),
        source_content_hash: content_hash.to_owned(),
        source_snapshot_version: snapshot_version,
        quality: extracted.quality.unwrap_or(AnalysisQuality::Complete),
        title: extracted.title,
        authors: extracted.authors,
        abstract_present: extracted.abstract_present,
        abstract_text: extracted.abstract_text,
        keywords_present: extracted.keywords_present,
        sections: extracted.sections,
        figure_count: extracted.figure_count,
        table_count: extracted.table_count,
        references_present: extracted.references_present,
        declarations: extracted.declarations,
        page_count: extracted.page_count,
        word_count: extracted.word_count,
        warnings: extracted.warnings,
    })
}

fn extract_tex(path: &Path) -> Result<ExtractedStructure, StructureError> {
    let bytes = std::fs::read(path)?;
    let source = String::from_utf8(bytes).map_err(|_| StructureError::InvalidTextEncoding)?;
    let text = strip_tex_comments(&source);
    let mut extracted = ExtractedStructure {
        quality: Some(AnalysisQuality::Complete),
        title: tex_command_argument(&text, "title"),
        authors: tex_command_arguments(&text, "author")
            .into_iter()
            .flat_map(|value| split_author_candidates(&value))
            .collect(),
        abstract_text: tex_environment_text(&text, "abstract"),
        abstract_present: text.contains("\\begin{abstract}"),
        keywords_present: text.contains("\\keywords{") || text.contains("\\begin{keywords}"),
        figure_count: count_occurrences(&text, "\\begin{figure"),
        table_count: count_occurrences(&text, "\\begin{table"),
        references_present: text.contains("\\bibliography{")
            || text.contains("\\begin{thebibliography}"),
        word_count: count_words(&strip_tex_commands(&text)),
        ..ExtractedStructure::default()
    };

    for (command, level) in [("section", 1), ("subsection", 2), ("subsubsection", 3)] {
        extracted.sections.extend(
            tex_command_arguments(&text, command)
                .into_iter()
                .map(|heading| SectionSummary { level, heading }),
        );
    }
    extracted.sections.sort_by_key(|section| {
        text.find(&format!("{{{}}}", section.heading))
            .unwrap_or(usize::MAX)
    });
    extracted.declarations = declaration_types(
        extracted
            .sections
            .iter()
            .map(|section| section.heading.as_str()),
    );
    for declaration in declaration_types(text.lines()) {
        push_unique(&mut extracted.declarations, &declaration);
    }

    if extracted.title.is_none() {
        extracted.warnings.push("未检测到 \\title{}".to_owned());
    }
    if extracted.authors.is_empty() {
        extracted.warnings.push("未检测到 \\author{}".to_owned());
    }
    Ok(extracted)
}

fn extract_docx(path: &Path) -> Result<ExtractedStructure, StructureError> {
    let file = File::open(path)?;
    let mut archive =
        ZipArchive::new(file).map_err(|error| StructureError::InvalidDocx(error.to_string()))?;
    let mut document_xml = String::new();
    archive
        .by_name("word/document.xml")
        .map_err(|error| StructureError::InvalidDocx(error.to_string()))?
        .read_to_string(&mut document_xml)
        .map_err(StructureError::Io)?;
    parse_docx_xml(&document_xml)
}

fn parse_docx_xml(xml: &str) -> Result<ExtractedStructure, StructureError> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);
    let mut extracted = ExtractedStructure {
        quality: Some(AnalysisQuality::Complete),
        ..ExtractedStructure::default()
    };
    let mut in_paragraph = false;
    let mut in_text = false;
    let mut paragraph_text = String::new();
    let mut paragraph_style: Option<String> = None;

    loop {
        match reader.read_event() {
            Ok(Event::Start(event)) => match local_name(event.name().as_ref()) {
                b"p" => {
                    in_paragraph = true;
                    paragraph_text.clear();
                    paragraph_style = None;
                }
                b"t" if in_paragraph => in_text = true,
                b"tbl" => extracted.table_count = extracted.table_count.saturating_add(1),
                b"drawing" | b"pict" => {
                    extracted.figure_count = extracted.figure_count.saturating_add(1);
                }
                _ => {}
            },
            Ok(Event::Empty(event)) => match local_name(event.name().as_ref()) {
                b"pStyle" if in_paragraph => {
                    paragraph_style = attribute_value(&event, b"val")?;
                }
                b"drawing" | b"pict" => {
                    extracted.figure_count = extracted.figure_count.saturating_add(1);
                }
                _ => {}
            },
            Ok(Event::Text(text)) if in_text => {
                let decoded = reader
                    .decoder()
                    .decode(text.as_ref())
                    .map_err(|error| StructureError::InvalidDocx(error.to_string()))?;
                let decoded = unescape(&decoded)
                    .map_err(|error| StructureError::InvalidDocx(error.to_string()))?;
                paragraph_text.push_str(&decoded);
            }
            Ok(Event::End(event)) => match local_name(event.name().as_ref()) {
                b"t" => in_text = false,
                b"p" => {
                    consume_docx_paragraph(
                        &mut extracted,
                        paragraph_style.as_deref(),
                        paragraph_text.trim(),
                    );
                    in_paragraph = false;
                    in_text = false;
                }
                _ => {}
            },
            Ok(Event::Eof) => break,
            Err(error) => return Err(StructureError::InvalidDocx(error.to_string())),
            _ => {}
        }
    }

    if extracted.title.is_none() {
        extracted.warnings.push("未检测到 Word 标题样式".to_owned());
    }
    if extracted.authors.is_empty() {
        extracted
            .warnings
            .push("未检测到 Word 作者样式或可靠的首页作者行".to_owned());
    }
    Ok(extracted)
}

fn consume_docx_paragraph(extracted: &mut ExtractedStructure, style: Option<&str>, text: &str) {
    if text.is_empty() {
        return;
    }
    extracted.word_count = extracted.word_count.saturating_add(count_words(text));
    let normalized_style = style.unwrap_or_default().to_ascii_lowercase();
    let normalized_text = text.trim().to_ascii_lowercase();

    if normalized_style == "title" && extracted.title.is_none() {
        extracted.title = Some(text.to_owned());
    }
    let is_front_matter_author = normalized_style != "title"
        && extracted.title.is_some()
        && extracted.authors.is_empty()
        && extracted.sections.is_empty()
        && !extracted.abstract_present
        && looks_like_author_line(text);
    if normalized_style.contains("author") || is_front_matter_author {
        for author in split_author_candidates(text) {
            push_unique(&mut extracted.authors, &author);
        }
    }
    if let Some(level) = heading_level(&normalized_style) {
        extracted.sections.push(SectionSummary {
            level,
            heading: text.to_owned(),
        });
    }
    if normalized_style.contains("abstract")
        || normalized_text.starts_with("abstract")
        || normalized_text.starts_with("摘要")
    {
        extracted.abstract_present = true;
        if let Some(abstract_text) = abstract_content_from_line(text).or_else(|| {
            normalized_style
                .contains("abstract")
                .then(|| text.to_owned())
        }) {
            append_text(&mut extracted.abstract_text, &abstract_text, 5_000);
        }
    }
    if normalized_style.contains("keyword")
        || normalized_text.starts_with("keywords")
        || normalized_text.starts_with("关键词")
    {
        extracted.keywords_present = true;
    }
    if is_reference_heading(&normalized_text) {
        extracted.references_present = true;
    }
    if let Some(declaration) = declaration_type(&normalized_text) {
        push_unique(&mut extracted.declarations, declaration);
    }
}

fn extract_pdf(path: &Path) -> Result<ExtractedStructure, StructureError> {
    let document =
        Document::load(path).map_err(|error| StructureError::InvalidPdf(error.to_string()))?;
    let page_numbers: Vec<u32> = document.get_pages().keys().copied().collect();
    let metadata_title = pdf_metadata_title(&document);
    let metadata_authors = pdf_metadata_authors(&document);
    let has_metadata_authors = !metadata_authors.is_empty();
    let (text, text_source) = extract_pdf_text(path, &document, &page_numbers);
    let mut extracted = infer_from_plain_text(&text);
    extracted.title = choose_pdf_title(metadata_title, extracted.title.take());
    for author in metadata_authors {
        push_unique(&mut extracted.authors, &author);
    }
    let outline_sections = pdf_outline_sections(&document);
    let used_outline = !outline_sections.is_empty();
    if used_outline {
        extracted.sections = outline_sections;
        extracted.references_present |= extracted
            .sections
            .iter()
            .any(|section| is_reference_heading(&section.heading));
        let outline_declarations = declaration_types(
            extracted
                .sections
                .iter()
                .map(|section| section.heading.as_str()),
        );
        for declaration in outline_declarations {
            push_unique(&mut extracted.declarations, &declaration);
        }
    }
    extracted.quality = Some(AnalysisQuality::Limited);
    extracted.page_count = u32::try_from(page_numbers.len()).ok();
    match text_source {
        PdfTextSource::EnhancedFontMapping => extracted.warnings.push(
            "已使用增强字体映射读取 PDF 文本层；多栏顺序、公式和复杂版式仍需人工确认".to_owned(),
        ),
        PdfTextSource::BasicContentStream => extracted
            .warnings
            .push("已使用基础 PDF 内容流读取文本；字体映射不完整，结构结果需人工确认".to_owned()),
        PdfTextSource::None => {
            extracted
                .warnings
                .push("PDF 未包含可读取的文本层，已标记为 OCR 候选".to_owned());
            extracted
                .warnings
                .push("本次分析没有执行 OCR，源文件和现有快照均未改动".to_owned());
        }
    }
    if used_outline {
        extracted
            .warnings
            .push("章节层级优先采用 PDF 内置书签目录".to_owned());
    }
    if extracted.authors.is_empty() {
        extracted
            .warnings
            .push("未可靠识别作者；请核对首页作者行或 PDF 元数据".to_owned());
    } else if !has_metadata_authors {
        extracted
            .warnings
            .push("作者根据首页版式候选行识别，请作者核对姓名与顺序".to_owned());
    }
    if !extracted.abstract_present {
        extracted
            .warnings
            .push("未定位摘要标题或摘要正文；请核对首页与双语摘要页".to_owned());
    } else if extracted.abstract_inferred_from_front_matter {
        extracted
            .warnings
            .push("未找到显式摘要标题；已根据首页连续正文识别摘要候选，请作者确认".to_owned());
    }
    Ok(extracted)
}

fn extract_pdf_text(
    path: &Path,
    document: &Document,
    page_numbers: &[u32],
) -> (String, PdfTextSource) {
    let enhanced = std::panic::catch_unwind(|| pdf_extract::extract_text(path))
        .ok()
        .and_then(Result::ok)
        .unwrap_or_default();
    if !enhanced.trim().is_empty() {
        return (enhanced, PdfTextSource::EnhancedFontMapping);
    }

    let basic = document.extract_text(page_numbers).unwrap_or_default();
    if basic.trim().is_empty() {
        (String::new(), PdfTextSource::None)
    } else {
        (basic, PdfTextSource::BasicContentStream)
    }
}

fn pdf_metadata_title(document: &Document) -> Option<String> {
    pdf_metadata_value(document, b"Title")
}

fn pdf_metadata_authors(document: &Document) -> Vec<String> {
    pdf_metadata_value(document, b"Author")
        .map(|value| split_author_candidates(&value))
        .unwrap_or_default()
}

fn pdf_metadata_value(document: &Document, key: &[u8]) -> Option<String> {
    let info_reference = document.trailer.get(b"Info").ok()?.as_reference().ok()?;
    let info = document.get_object(info_reference).ok()?.as_dict().ok()?;
    let value = info.get(key).ok()?;
    let bytes = match value {
        Object::String(bytes, _) | Object::Name(bytes) => bytes.as_slice(),
        _ => return None,
    };
    let decoded = decode_pdf_text_string(bytes);
    let normalized = normalize_line(&decoded);
    (!normalized.is_empty()).then_some(normalized)
}

fn choose_pdf_title(metadata: Option<String>, visible: Option<String>) -> Option<String> {
    match (metadata, visible) {
        (Some(metadata), Some(visible))
            if visible.len() > metadata.len()
                && visible
                    .to_ascii_lowercase()
                    .starts_with(&metadata.to_ascii_lowercase()) =>
        {
            Some(visible)
        }
        (Some(metadata), _) => Some(metadata),
        (None, visible) => visible,
    }
}

fn pdf_outline_sections(document: &Document) -> Vec<SectionSummary> {
    let Ok(toc) = document.get_toc() else {
        return Vec::new();
    };
    let mut sections = Vec::new();
    for item in toc.toc {
        let heading = normalize_line(&item.title);
        if heading.is_empty() || heading.chars().count() > 300 {
            continue;
        }
        if sections
            .iter()
            .any(|existing: &SectionSummary| existing.heading.eq_ignore_ascii_case(&heading))
        {
            continue;
        }
        sections.push(SectionSummary {
            level: u8::try_from(item.level.clamp(1, 9)).unwrap_or(1),
            heading,
        });
        if sections.len() == 200 {
            break;
        }
    }
    sections
}

fn decode_pdf_text_string(bytes: &[u8]) -> String {
    if let Some(utf16) = bytes.strip_prefix(&[0xfe, 0xff]) {
        let units = utf16
            .chunks_exact(2)
            .map(|chunk| u16::from_be_bytes([chunk[0], chunk[1]]));
        String::from_utf16_lossy(&units.collect::<Vec<_>>())
    } else if let Some(utf16) = bytes.strip_prefix(&[0xff, 0xfe]) {
        let units = utf16
            .chunks_exact(2)
            .map(|chunk| u16::from_le_bytes([chunk[0], chunk[1]]));
        String::from_utf16_lossy(&units.collect::<Vec<_>>())
    } else {
        String::from_utf8_lossy(bytes).into_owned()
    }
}

fn infer_from_plain_text(text: &str) -> ExtractedStructure {
    let lines: Vec<&str> = text
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect();
    let title_match = lines
        .iter()
        .take(40)
        .enumerate()
        .find_map(|(index, line)| infer_title(line).map(|title| (index, title)));
    let title_index = title_match.as_ref().map(|(index, _)| *index);
    let title = title_match.map(|(_, title)| title);
    let (abstract_text, abstract_inferred_from_front_matter) =
        extract_abstract_text(&lines, title_index);
    let abstract_present =
        abstract_text.is_some() || lines.iter().take(160).any(|line| is_abstract_marker(line));
    let authors = infer_visible_authors(&lines, title_index);
    let lower = text.to_ascii_lowercase();
    let mut sections = Vec::new();
    for section in lines.iter().filter_map(|line| infer_heading(line)) {
        if !sections.iter().any(|existing: &SectionSummary| {
            existing.heading.eq_ignore_ascii_case(&section.heading)
        }) {
            sections.push(section);
        }
        if sections.len() == 100 {
            break;
        }
    }
    let declarations = declaration_types(lines.iter().copied());

    ExtractedStructure {
        title,
        authors,
        abstract_present,
        abstract_text,
        abstract_inferred_from_front_matter,
        keywords_present: lines.iter().take(80).any(|line| {
            let normalized = line.trim().to_ascii_lowercase();
            normalized == "keywords"
                || normalized.starts_with("keywords:")
                || normalized.starts_with("keywords：")
                || line.trim() == "关键词"
                || line.trim().starts_with("关键词：")
        }),
        sections,
        figure_count: count_numbered_labels(text, &["figure ", "fig. ", "图"]),
        table_count: count_numbered_labels(text, &["table ", "表"]),
        references_present: lower.contains("\nreferences")
            || lower.contains("\nbibliography")
            || text.contains("\n参考文献"),
        declarations,
        word_count: count_words(text),
        ..ExtractedStructure::default()
    }
}

fn infer_title(line: &str) -> Option<String> {
    let normalized = normalize_line(line);
    let lowercase = normalized.to_ascii_lowercase();
    let length = normalized.chars().count();
    let words = normalized.split_whitespace().count();
    let unsuitable = lowercase.starts_with("arxiv:")
        || lowercase.starts_with("doi:")
        || lowercase == "contents"
        || lowercase == "table of contents"
        || normalized.contains('@')
        || normalized.contains("http://")
        || normalized.contains("https://")
        || normalized
            .chars()
            .all(|character| character.is_ascii_digit());
    if (4..=300).contains(&length)
        && words <= 40
        && normalized.chars().any(char::is_alphabetic)
        && !unsuitable
    {
        Some(normalized)
    } else {
        None
    }
}

fn infer_visible_authors(lines: &[&str], title_index: Option<usize>) -> Vec<String> {
    let start = title_index.map_or(0, |index| index.saturating_add(1));
    let mut authors = Vec::new();
    for line in lines.iter().skip(start).take(14) {
        if is_abstract_marker(line) || is_keyword_marker(line) || infer_heading(line).is_some() {
            break;
        }
        if !looks_like_author_line(line) {
            continue;
        }
        for author in split_author_candidates(line) {
            push_unique(&mut authors, &author);
            if authors.len() == 30 {
                return authors;
            }
        }
    }
    authors
}

fn looks_like_author_line(line: &str) -> bool {
    let normalized = normalize_line(line);
    let lowercase = normalized.to_ascii_lowercase();
    let length = normalized.chars().count();
    if !(2..=180).contains(&length)
        || normalized.ends_with(['.', '。'])
        || normalized.contains('@')
        || lowercase.contains("http")
        || lowercase.contains("doi")
        || lowercase.contains("orcid")
        || [
            "university",
            "institute",
            "department",
            "laboratory",
            "hospital",
            "school of",
            "college of",
            "academy of",
            "research center",
            "corresponding author",
            "大学",
            "学院",
            "研究院",
            "实验室",
            "医院",
            "通讯作者",
            "基金",
        ]
        .iter()
        .any(|marker| lowercase.contains(marker))
    {
        return false;
    }
    let words = normalized.split_whitespace().collect::<Vec<_>>();
    let has_name_separator = [",", "，", ";", "；", "、", "·", " and ", " & "]
        .iter()
        .any(|separator| normalized.contains(separator));
    let title_case_words = words
        .iter()
        .filter(|word| {
            let cleaned = word.trim_matches(|character: char| {
                !character.is_alphabetic() && character != '-' && character != '\''
            });
            cleaned
                .chars()
                .next()
                .is_some_and(|first| first.is_uppercase())
        })
        .count();
    let cjk_count = normalized
        .chars()
        .filter(|character| matches!(*character as u32, 0x3400..=0x9fff))
        .count();
    (has_name_separator && normalized.chars().any(char::is_alphabetic))
        || ((2..=14).contains(&words.len()) && title_case_words >= 2)
        || ((2..=40).contains(&cjk_count) && words.len() <= 12)
}

fn split_author_candidates(value: &str) -> Vec<String> {
    let normalized = normalize_line(value)
        .replace(" and ", ",")
        .replace(" & ", ",")
        .replace(['，', '；', '、', '·'], ",")
        .replace(';', ",");
    let mut candidates = normalized
        .split(',')
        .filter_map(|candidate| {
            let cleaned = candidate
                .trim()
                .trim_matches(|character: char| {
                    character.is_ascii_digit()
                        || matches!(character, '*' | '†' | '‡' | '§' | '#' | '^')
                })
                .trim();
            let letter_count = cleaned
                .chars()
                .filter(|character| character.is_alphabetic())
                .count();
            let words = cleaned.split_whitespace().count();
            let cjk_count = cleaned
                .chars()
                .filter(|character| matches!(*character as u32, 0x3400..=0x9fff))
                .count();
            (letter_count >= 2
                && cleaned.chars().count() <= 100
                && ((2..=8).contains(&words) || (2..=6).contains(&cjk_count)))
            .then(|| cleaned.to_owned())
        })
        .collect::<Vec<_>>();
    if candidates.len() == 1 && !looks_like_author_line(&candidates[0]) {
        candidates.clear();
    }
    candidates
}

fn is_abstract_marker(line: &str) -> bool {
    let normalized = normalize_line(line);
    let lowercase = normalized.to_ascii_lowercase();
    let compact = normalized.split_whitespace().collect::<String>();
    let english = ["abstract", "summary"].iter().any(|marker| {
        lowercase == *marker
            || lowercase.strip_prefix(marker).is_some_and(|remainder| {
                remainder.starts_with([':', '：', '—', '–', '-', '.', ' '])
            })
    });
    english
        || ["摘要", "中文摘要", "内容摘要"].iter().any(|marker| {
            compact == *marker
                || compact.starts_with(&format!("{marker}："))
                || compact.starts_with(&format!("{marker}:"))
        })
}

fn is_keyword_marker(line: &str) -> bool {
    let normalized = normalize_line(line);
    let lowercase = normalized.to_ascii_lowercase();
    let compact = normalized.split_whitespace().collect::<String>();
    lowercase == "keywords"
        || lowercase.starts_with("keywords:")
        || lowercase.starts_with("keywords：")
        || compact == "关键词"
        || compact.starts_with("关键词：")
        || compact.starts_with("关键词:")
}

fn abstract_content_from_line(line: &str) -> Option<String> {
    let normalized = normalize_line(line);
    let lowercase = normalized.to_ascii_lowercase();
    let remainder = if lowercase.starts_with("abstract") {
        normalized.get("abstract".len()..)
    } else if lowercase.starts_with("summary") {
        normalized.get("summary".len()..)
    } else if let Some(remainder) = normalized.strip_prefix("中文摘要") {
        Some(remainder)
    } else if let Some(remainder) = normalized.strip_prefix("内容摘要") {
        Some(remainder)
    } else if let Some(remainder) = normalized.strip_prefix("摘要") {
        Some(remainder)
    } else if normalized.starts_with("摘 要") {
        normalized.get("摘 要".len()..)
    } else {
        None
    }?;
    let content = remainder
        .trim_start_matches(|character: char| {
            character.is_whitespace()
                || matches!(character, ':' | '：' | '—' | '–' | '-' | '.' | '·')
        })
        .trim();
    (!content.is_empty()).then(|| content.to_owned())
}

fn extract_abstract_text(lines: &[&str], title_index: Option<usize>) -> (Option<String>, bool) {
    if let Some(marker_index) = lines
        .iter()
        .take(200)
        .position(|line| is_abstract_marker(line))
    {
        return (
            collect_abstract_lines(
                lines,
                marker_index,
                abstract_content_from_line(lines[marker_index]),
            ),
            false,
        );
    }

    let start = title_index.map_or(0, |index| index.saturating_add(1));
    for (relative, line) in lines.iter().skip(start).take(90).enumerate() {
        let normalized = normalize_line(line);
        let lowercase = normalized.to_ascii_lowercase();
        let word_count = normalized.split_whitespace().count();
        if word_count < 12
            || looks_like_author_line(line)
            || is_affiliation_or_contact_line(line)
            || lowercase == "contents"
            || lowercase.starts_with("note:")
            || lowercase.contains("arxiv:")
            || infer_numbered_heading(line).is_some()
        {
            continue;
        }
        let index = start + relative;
        let abstract_text = collect_unlabelled_abstract_lines(lines, index);
        if abstract_text
            .as_deref()
            .is_some_and(|content| content.split_whitespace().count() >= 30)
        {
            return (abstract_text, true);
        }
    }
    (None, false)
}

fn collect_abstract_lines(
    lines: &[&str],
    marker_index: usize,
    mut abstract_text: Option<String>,
) -> Option<String> {
    for line in lines.iter().skip(marker_index + 1).take(30) {
        if is_keyword_marker(line)
            || is_reference_heading(line)
            || infer_numbered_heading(line).is_some()
            || matches!(
                line.trim().to_ascii_lowercase().as_str(),
                "introduction" | "methods" | "results"
            )
            || matches!(line.trim(), "引言" | "方法" | "结果")
        {
            break;
        }
        append_text(&mut abstract_text, &normalize_line(line), 5_000);
    }
    abstract_text.filter(|content| !content.trim().is_empty())
}

fn collect_unlabelled_abstract_lines(lines: &[&str], start: usize) -> Option<String> {
    let mut abstract_text = None;
    for line in lines.iter().skip(start).take(50) {
        let normalized = normalize_line(line);
        let lowercase = normalized.to_ascii_lowercase();
        if is_keyword_marker(line)
            || is_reference_heading(line)
            || lowercase == "contents"
            || lowercase.starts_with("main contact")
            || lowercase.starts_with("github:")
            || lowercase.starts_with("note:")
            || lowercase.contains("arxiv:")
            || infer_numbered_heading(line).is_some()
            || matches!(lowercase.as_str(), "introduction" | "methods" | "results")
            || matches!(normalized.as_str(), "引言" | "方法" | "结果")
        {
            break;
        }
        append_text(&mut abstract_text, &normalized, 5_000);
    }
    abstract_text
}

fn is_affiliation_or_contact_line(line: &str) -> bool {
    let lowercase = normalize_line(line).to_ascii_lowercase();
    [
        "affiliation",
        "university",
        "institute",
        "department",
        "laboratory",
        "hospital",
        "school of",
        "college of",
        "academy of",
        "research center",
        "corresponding author",
        "main contact",
        "github:",
        "大学",
        "学院",
        "研究院",
        "实验室",
        "医院",
        "通讯作者",
    ]
    .iter()
    .any(|marker| lowercase.contains(marker))
}

fn append_text(destination: &mut Option<String>, value: &str, maximum_chars: usize) {
    if value.trim().is_empty() {
        return;
    }
    let existing_chars = destination
        .as_deref()
        .map_or(0, |current| current.chars().count());
    if existing_chars >= maximum_chars {
        return;
    }
    let remaining = maximum_chars - existing_chars;
    let fragment = value.chars().take(remaining).collect::<String>();
    match destination {
        Some(current) => {
            current.push(' ');
            current.push_str(&fragment);
        }
        None => *destination = Some(fragment),
    }
}

fn infer_heading(line: &str) -> Option<SectionSummary> {
    let line = normalize_line(line);
    let length = line.chars().count();
    if !(2..=120).contains(&length) || line.ends_with(['.', '。', ',', '，', ';', '；']) {
        return None;
    }
    let lowercase = line.to_ascii_lowercase();
    let is_named = is_reference_heading(&lowercase)
        || [
            "abstract",
            "introduction",
            "methods",
            "results",
            "discussion",
            "conclusion",
        ]
        .contains(&lowercase.as_str())
        || ["摘要", "引言", "方法", "结果", "讨论", "结论"].contains(&line.as_str());
    if is_named {
        Some(SectionSummary {
            level: 1,
            heading: line,
        })
    } else {
        infer_numbered_heading(&line)
    }
}

fn infer_numbered_heading(line: &str) -> Option<SectionSummary> {
    let (number, heading) = line.split_once(char::is_whitespace)?;
    let number = number.trim_end_matches(['.', '、']);
    let components: Vec<&str> = number.split('.').collect();
    if components.is_empty()
        || components.len() > 4
        || components
            .iter()
            .any(|component| component.is_empty() || !component.chars().all(|c| c.is_ascii_digit()))
    {
        return None;
    }

    let mut heading = normalize_line(heading);
    if let Some((without_page, possible_page)) = heading.rsplit_once(' ') {
        if possible_page
            .chars()
            .all(|character| character.is_ascii_digit())
            && without_page.split_whitespace().count() >= 1
        {
            heading = without_page.to_owned();
        }
    }
    let letter_count = heading
        .chars()
        .filter(|character| character.is_alphabetic())
        .count();
    let looks_like_equation = ['=', '≤', '≥', '∑', '∈', '∩', '∪', '√', '�', '|', '#']
        .iter()
        .any(|symbol| heading.contains(*symbol));
    let looks_like_toc = heading.contains(". . .") || heading.contains("……");
    let looks_like_list_item = heading.to_ascii_lowercase().contains("(section ");
    if !(2..=100).contains(&heading.chars().count())
        || heading.split_whitespace().count() > 18
        || letter_count < 2
        || looks_like_equation
        || looks_like_toc
        || looks_like_list_item
    {
        return None;
    }

    Some(SectionSummary {
        level: u8::try_from(components.len()).unwrap_or(1),
        heading: format!("{number} {heading}"),
    })
}

fn normalize_line(line: &str) -> String {
    line.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn strip_tex_comments(source: &str) -> String {
    source
        .lines()
        .map(|line| {
            let mut escaped = false;
            for (index, character) in line.char_indices() {
                if character == '%' && !escaped {
                    return &line[..index];
                }
                escaped = character == '\\' && !escaped;
                if character != '\\' {
                    escaped = false;
                }
            }
            line
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn tex_command_argument(source: &str, command: &str) -> Option<String> {
    tex_command_arguments(source, command).into_iter().next()
}

fn tex_command_arguments(source: &str, command: &str) -> Vec<String> {
    let marker = format!("\\{command}");
    let mut values = Vec::new();
    let mut cursor = 0;
    while let Some(relative) = source[cursor..].find(&marker) {
        let command_start = cursor + relative + marker.len();
        let Some(open_relative) = source[command_start..].find('{') else {
            break;
        };
        let open = command_start + open_relative;
        if let Some((value, close)) = balanced_brace_content(source, open) {
            let cleaned = strip_tex_commands(value.trim());
            if !cleaned.is_empty() {
                values.push(cleaned);
            }
            cursor = close + 1;
        } else {
            break;
        }
    }
    values
}

fn tex_environment_text(source: &str, environment: &str) -> Option<String> {
    let start_marker = format!("\\begin{{{environment}}}");
    let end_marker = format!("\\end{{{environment}}}");
    let start = source
        .find(&start_marker)?
        .saturating_add(start_marker.len());
    let end = source[start..].find(&end_marker)?.saturating_add(start);
    let content = normalize_line(&strip_tex_commands(&source[start..end]));
    (!content.is_empty()).then_some(content)
}

fn balanced_brace_content(source: &str, open: usize) -> Option<(&str, usize)> {
    let mut depth = 0_u32;
    for (relative, character) in source[open..].char_indices() {
        match character {
            '{' => depth = depth.saturating_add(1),
            '}' => {
                depth = depth.checked_sub(1)?;
                if depth == 0 {
                    let close = open + relative;
                    return Some((&source[open + 1..close], close));
                }
            }
            _ => {}
        }
    }
    None
}

fn strip_tex_commands(text: &str) -> String {
    let mut output = String::with_capacity(text.len());
    let mut in_command = false;
    for character in text.chars() {
        if character == '\\' {
            in_command = true;
            output.push(' ');
        } else if in_command && character.is_alphabetic() {
            continue;
        } else {
            in_command = false;
            match character {
                '{' | '}' | '[' | ']' | '~' => output.push(' '),
                _ => output.push(character),
            }
        }
    }
    output.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn heading_level(style: &str) -> Option<u8> {
    let suffix = style.strip_prefix("heading")?;
    suffix
        .trim_start_matches([' ', '-'])
        .parse::<u8>()
        .ok()
        .filter(|level| (1..=9).contains(level))
}

fn attribute_value(
    event: &quick_xml::events::BytesStart<'_>,
    wanted_local_name: &[u8],
) -> Result<Option<String>, StructureError> {
    for attribute in event.attributes() {
        let attribute =
            attribute.map_err(|error| StructureError::InvalidDocx(error.to_string()))?;
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

fn is_reference_heading(text: &str) -> bool {
    matches!(
        text.trim().to_ascii_lowercase().as_str(),
        "references" | "reference" | "bibliography" | "参考文献"
    )
}

fn declaration_types<'a>(headings: impl IntoIterator<Item = &'a str>) -> Vec<String> {
    let mut declarations = Vec::new();
    for heading in headings {
        if let Some(declaration) = declaration_type(heading) {
            push_unique(&mut declarations, declaration);
        }
    }
    declarations
}

fn declaration_type(text: &str) -> Option<&'static str> {
    let normalized = text.trim().to_ascii_lowercase();
    if normalized.contains("conflict of interest")
        || normalized.contains("competing interest")
        || normalized.contains("利益冲突")
    {
        Some("conflict_of_interest")
    } else if normalized == "funding"
        || normalized.starts_with("funding ")
        || normalized.contains("funding statement")
        || normalized.contains("基金资助")
        || normalized.contains("资助声明")
    {
        Some("funding")
    } else if normalized.contains("consent for publication")
        || normalized.contains("publication consent")
        || normalized.contains("发表同意")
    {
        Some("consent_for_publication")
    } else if normalized.contains("ethics approval")
        || normalized.contains("ethical approval")
        || normalized.contains("institutional review board")
        || normalized.contains("伦理批准")
        || normalized.contains("伦理审批")
        || normalized.contains("伦理声明")
    {
        Some("ethics_approval")
    } else if normalized.contains("data availability") || normalized.contains("数据可用") {
        Some("data_availability")
    } else if normalized.contains("author contribution") || normalized.contains("作者贡献") {
        Some("author_contributions")
    } else if normalized.contains("trial registration")
        || normalized.contains("study registration")
        || normalized.contains("review registration")
        || normalized.contains("registration number")
        || normalized.contains("试验注册")
        || normalized.contains("研究注册")
        || normalized.contains("注册号")
    {
        Some("registration")
    } else if normalized.contains("generative ai")
        || normalized.contains("artificial intelligence use")
        || normalized.contains("declaration of ai")
        || normalized.contains("生成式人工智能")
        || normalized.contains("生成式 ai")
        || normalized.contains("人工智能使用")
    {
        Some("ai_use")
    } else if normalized.contains("acknowledg") || normalized.contains("致谢") {
        Some("acknowledgements")
    } else {
        None
    }
}

fn push_unique(values: &mut Vec<String>, value: &str) {
    if !values.iter().any(|existing| existing == value) {
        values.push(value.to_owned());
    }
}

fn count_occurrences(text: &str, marker: &str) -> u32 {
    u32::try_from(text.matches(marker).count()).unwrap_or(u32::MAX)
}

fn count_numbered_labels(text: &str, labels: &[&str]) -> u32 {
    let lower = text.to_ascii_lowercase();
    let mut count = 0_u32;
    for line in lower.lines() {
        if labels
            .iter()
            .any(|label| line.trim_start().starts_with(label))
        {
            count = count.saturating_add(1);
        }
    }
    count
}

fn count_words(text: &str) -> u64 {
    text.split_whitespace().count() as u64
}

#[cfg(test)]
mod tests {
    use super::{
        choose_pdf_title, decode_pdf_text_string, extract_tex, infer_from_plain_text,
        parse_docx_xml, pdf_metadata_authors, pdf_metadata_title, AnalysisQuality,
    };
    use lopdf::{dictionary, Document, Object};
    use std::{fs, path::PathBuf, time::SystemTime};

    #[test]
    fn extracts_latex_structure_deterministically() {
        let path = synthetic_tex_path();
        fs::write(
            &path,
            r"\title{Synthetic Evidence Study}
\author{Ada Author and Ben Researcher}
\begin{abstract}A compact abstract.\end{abstract}
\keywords{testing, evidence}
\section{Introduction}
\subsection{Method}
\section{Data Availability}
\begin{figure}\end{figure}
\begin{table}\end{table}
\bibliography{references}",
        )
        .unwrap();

        let extracted = extract_tex(&path).unwrap();
        let _ = fs::remove_file(path);

        assert_eq!(extracted.title.as_deref(), Some("Synthetic Evidence Study"));
        assert_eq!(extracted.authors, vec!["Ada Author", "Ben Researcher"]);
        assert!(extracted.abstract_present);
        assert_eq!(
            extracted.abstract_text.as_deref(),
            Some("A compact abstract.")
        );
        assert!(extracted.keywords_present);
        assert_eq!(extracted.sections.len(), 3);
        assert_eq!(extracted.figure_count, 1);
        assert_eq!(extracted.table_count, 1);
        assert!(extracted.references_present);
        assert_eq!(extracted.declarations, vec!["data_availability"]);
    }

    #[test]
    fn extracts_word_styles_and_counts_from_synthetic_xml() {
        let xml = r#"<w:document xmlns:w="urn:w"><w:body>
          <w:p><w:pPr><w:pStyle w:val="Title"/></w:pPr><w:r><w:t>Local Research</w:t></w:r></w:p>
          <w:p><w:pPr><w:pStyle w:val="Author"/></w:pPr><w:r><w:t>Ada Author; Ben Researcher</w:t></w:r></w:p>
          <w:p><w:pPr><w:pStyle w:val="Abstract"/></w:pPr><w:r><w:t>Abstract: evidence.</w:t></w:r></w:p>
          <w:p><w:pPr><w:pStyle w:val="Heading1"/></w:pPr><w:r><w:t>Methods</w:t></w:r></w:p>
          <w:p><w:r><w:drawing/></w:r></w:p><w:tbl></w:tbl>
          <w:p><w:pPr><w:pStyle w:val="Heading1"/></w:pPr><w:r><w:t>References</w:t></w:r></w:p>
          <w:p><w:pPr><w:pStyle w:val="Heading1"/></w:pPr><w:r><w:t>Conflict of Interest</w:t></w:r></w:p>
        </w:body></w:document>"#;

        let extracted = parse_docx_xml(xml).unwrap();

        assert_eq!(extracted.title.as_deref(), Some("Local Research"));
        assert_eq!(extracted.authors, vec!["Ada Author", "Ben Researcher"]);
        assert!(extracted.abstract_present);
        assert_eq!(extracted.abstract_text.as_deref(), Some("evidence."));
        assert_eq!(extracted.sections.len(), 3);
        assert_eq!(extracted.figure_count, 1);
        assert_eq!(extracted.table_count, 1);
        assert!(extracted.references_present);
        assert_eq!(extracted.declarations, vec!["conflict_of_interest"]);
    }

    #[test]
    fn marks_plain_text_inference_as_a_limited_pdf_layer() {
        let mut extracted = infer_from_plain_text(
            "Synthetic Study\nAbstract\n1 Introduction\n2 Methods\nReferences\nFigure 1 result",
        );
        extracted.quality = Some(AnalysisQuality::Limited);

        assert_eq!(extracted.title.as_deref(), Some("Synthetic Study"));
        assert!(extracted.abstract_present);
        assert!(extracted.references_present);
        assert_eq!(extracted.figure_count, 1);
        assert_eq!(extracted.quality, Some(AnalysisQuality::Limited));
    }

    #[test]
    fn recognizes_extended_submission_declarations_in_plain_text() {
        let extracted = infer_from_plain_text(
            "Synthetic Study\nAbstract\n1 Methods\nEthics approval\nTrial registration number\nAuthor contributions\nDeclaration of generative AI\nConsent for publication\nFunding statement\nReferences",
        );

        assert!(extracted
            .declarations
            .contains(&"ethics_approval".to_owned()));
        assert!(extracted.declarations.contains(&"registration".to_owned()));
        assert!(extracted
            .declarations
            .contains(&"author_contributions".to_owned()));
        assert!(extracted.declarations.contains(&"ai_use".to_owned()));
        assert!(extracted
            .declarations
            .contains(&"consent_for_publication".to_owned()));
        assert!(extracted.declarations.contains(&"funding".to_owned()));
    }

    #[test]
    fn recognizes_visible_authors_and_dash_or_bilingual_abstract_markers() {
        let english = infer_from_plain_text(
            "Memory in the Age of Agents\nAda Lovelace, Alan Turing and Grace Hopper\nDepartment of Computer Science\nABSTRACT—This survey maps durable agent memory.\nIt preserves a second abstract line.\nKeywords: agents, memory\n1 Introduction",
        );
        assert_eq!(
            english.authors,
            vec!["Ada Lovelace", "Alan Turing", "Grace Hopper"]
        );
        assert!(english.abstract_present);
        assert_eq!(
            english.abstract_text.as_deref(),
            Some("This survey maps durable agent memory. It preserves a second abstract line.")
        );

        let chinese = infer_from_plain_text(
            "可信学术工作台研究\n张三，李四\n某某大学计算机学院\n摘 要：本文提出一种本地优先方法。\n关键词：知识体；投稿\n1 引言",
        );
        assert_eq!(chinese.authors, vec!["张三", "李四"]);
        assert_eq!(
            chinese.abstract_text.as_deref(),
            Some("本文提出一种本地优先方法。")
        );
    }

    #[test]
    fn recognizes_unlabelled_front_matter_abstract_without_treating_subtitle_as_authors() {
        let extracted = infer_from_plain_text(
            "Memory in the Age of AI Agents: A Survey\nForms, Functions and Dynamics\nYuyang Hu\n† , Shichun Liu\n†, Yanwei Yue\nAffiliations: National University of Singapore\nMemory has emerged as a core capability of foundation model-based agents and now underpins long-horizon reasoning across complex environments.\nThis survey provides a comprehensive landscape of agent memory research and distinguishes its forms, functions, and dynamics for future systems.\nMain Contact: author@example.org\nContents\n1 Introduction 4",
        );

        assert_eq!(
            extracted.authors,
            vec!["Yuyang Hu", "Shichun Liu", "Yanwei Yue"]
        );
        assert!(extracted.abstract_present);
        assert!(extracted.abstract_inferred_from_front_matter);
        assert!(extracted
            .abstract_text
            .as_deref()
            .is_some_and(|text| text.starts_with("Memory has emerged")));
    }

    #[test]
    fn reads_pdf_metadata_title_and_utf16_text_strings() {
        let mut document = Document::with_version("1.7");
        let info_id = document.add_object(dictionary! {
            "Title" => Object::string_literal("Metadata Study Title"),
            "Author" => Object::string_literal("Ada Author; Ben Researcher")
        });
        document.trailer.set("Info", info_id);

        assert_eq!(
            pdf_metadata_title(&document).as_deref(),
            Some("Metadata Study Title")
        );
        assert_eq!(
            pdf_metadata_authors(&document),
            vec!["Ada Author", "Ben Researcher"]
        );
        assert_eq!(
            decode_pdf_text_string(&[0xfe, 0xff, 0x78, 0x6e, 0x5b, 0x9a]),
            "确定"
        );
        assert_eq!(
            choose_pdf_title(
                Some("Memory in the Age of AI Agents".to_owned()),
                Some("Memory in the Age of AI Agents: A Survey".to_owned()),
            )
            .as_deref(),
            Some("Memory in the Age of AI Agents: A Survey")
        );
    }

    #[test]
    fn keeps_numbered_headings_but_rejects_formulas_and_toc_leaders() {
        let extracted = infer_from_plain_text(
            "arXiv:synthetic\nSynthetic Memory Study\nAbstract\n1 Introduction\n2.1 Agent Systems\n2 (# U 0 ) = 4\n2.2 Related Work . . . . . . 8\nReferences",
        );

        assert_eq!(extracted.title.as_deref(), Some("Synthetic Memory Study"));
        assert!(extracted
            .sections
            .iter()
            .any(|section| section.heading == "1 Introduction"));
        assert!(extracted
            .sections
            .iter()
            .any(|section| section.heading == "2.1 Agent Systems"));
        assert!(!extracted
            .sections
            .iter()
            .any(|section| section.heading.contains("# U") || section.heading.contains(". . .")));
    }

    fn synthetic_tex_path() -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!(
            "manuscriptdock-structure-{}-{nonce}.tex",
            std::process::id()
        ))
    }
}
