use crate::{ManuscriptKind, ManuscriptSummary};
use lopdf::{Document, Object};
use quick_xml::{escape::unescape, events::Event, Reader};
use serde::{Deserialize, Serialize};
use std::{error::Error, fmt, fs::File, io::Read, path::Path};
use zip::ZipArchive;

pub const STRUCTURE_ANALYSIS_VERSION: u32 = 6;
pub const DECOMPOSITION_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Deserialize, Serialize)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SemanticElementKind {
    Claim,
    Scope,
    Method,
    Result,
    Evidence,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceModality {
    Text,
    Table,
    Figure,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SemanticCandidate {
    pub element: SemanticElementKind,
    pub text: String,
    pub source_label: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_fragment_id: Option<String>,
    pub modality: SourceModality,
    pub confidence_percent: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtractedSourceFragment {
    pub fragment_id: String,
    pub text: String,
    pub source_label: String,
    pub modality: SourceModality,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ExtractionCoverage {
    pub text_fragments: u32,
    pub table_fragments: u32,
    pub figure_fragments: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PdfProcessingSummary {
    pub classification: String,
    pub confidence_percent: u8,
    pub native_extraction: String,
    pub pages_needing_recognition: Vec<u32>,
    pub pages_with_tables: Vec<u32>,
    pub pages_with_columns: Vec<u32>,
    pub has_encoding_issues: bool,
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
    #[serde(default)]
    pub semantic_candidates: Vec<SemanticCandidate>,
    #[serde(default)]
    pub source_fragments: Vec<ExtractedSourceFragment>,
    #[serde(default)]
    pub extraction_coverage: ExtractionCoverage,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pdf_processing: Option<PdfProcessingSummary>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DecompositionManifest {
    pub schema_version: u32,
    pub decomposition_id: String,
    pub workspace_id: String,
    pub source_content_hash: String,
    pub source_snapshot_version: u32,
    pub created_unix_ms: u64,
    pub structure: StructureReport,
    pub declared_outputs: Vec<String>,
    pub manifest_hash: String,
    pub external_transmission: String,
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
    content_fragments: Vec<ContentFragment>,
    pdf_processing: Option<PdfProcessingSummary>,
    warnings: Vec<String>,
}

#[derive(Debug, Clone)]
struct ContentFragment {
    text: String,
    source_label: String,
    modality: SourceModality,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PdfTextSource {
    LayoutAwareInspector,
    EnhancedFontMapping,
    BasicContentStream,
    None,
}

impl PdfTextSource {
    fn label(self) -> &'static str {
        match self {
            Self::LayoutAwareInspector => "layout_aware_native",
            Self::EnhancedFontMapping => "enhanced_font_mapping",
            Self::BasicContentStream => "basic_content_stream",
            Self::None => "none",
        }
    }
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

    let (mut semantic_candidates, extraction_coverage) = derive_semantic_candidates(&extracted);
    let mut source_fragments = Vec::new();
    if let Some(abstract_text) = extracted.abstract_text.as_deref() {
        source_fragments.push(ExtractedSourceFragment {
            fragment_id: format!("fragment:{snapshot_version}:abstract"),
            text: bounded_text(abstract_text, 2_400),
            source_label: "摘要 / Abstract".to_owned(),
            modality: SourceModality::Text,
        });
    }
    source_fragments.extend(
        extracted
            .content_fragments
            .iter()
            .take(399)
            .enumerate()
            .map(|(index, fragment)| ExtractedSourceFragment {
                fragment_id: format!("fragment:{}:{}", snapshot_version, index + 1),
                text: fragment.text.clone(),
                source_label: fragment.source_label.clone(),
                modality: fragment.modality,
            })
            .collect::<Vec<_>>(),
    );
    for candidate in &mut semantic_candidates {
        candidate.source_fragment_id = source_fragments
            .iter()
            .find(|fragment| {
                fragment.source_label == candidate.source_label
                    && fragment
                        .text
                        .contains(candidate.text.trim_end_matches(['.', '。']))
            })
            .map(|fragment| fragment.fragment_id.clone());
    }

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
        semantic_candidates,
        source_fragments,
        extraction_coverage,
        pdf_processing: extracted.pdf_processing,
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
        content_fragments: text_fragments(&strip_tex_commands(&text), "LaTeX 正文"),
        ..ExtractedStructure::default()
    };
    if let Some(table_text) = tex_environment_text(&text, "table") {
        extracted.content_fragments.push(ContentFragment {
            text: bounded_text(&strip_tex_commands(&table_text), 1_200),
            source_label: "LaTeX 表格环境".to_owned(),
            modality: SourceModality::Table,
        });
    }
    if let Some(figure_text) = tex_environment_text(&text, "figure") {
        let figure_text = bounded_text(&strip_tex_commands(&figure_text), 1_200);
        if !figure_text.is_empty() {
            extracted.content_fragments.push(ContentFragment {
                text: figure_text,
                source_label: "LaTeX 图片环境".to_owned(),
                modality: SourceModality::Figure,
            });
        }
    }

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
    let mut table_depth = 0_u32;

    loop {
        match reader.read_event() {
            Ok(Event::Start(event)) => match local_name(event.name().as_ref()) {
                b"p" => {
                    in_paragraph = true;
                    paragraph_text.clear();
                    paragraph_style = None;
                }
                b"t" if in_paragraph => in_text = true,
                b"tbl" => {
                    table_depth = table_depth.saturating_add(1);
                    extracted.table_count = extracted.table_count.saturating_add(1);
                }
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
                        table_depth > 0,
                    );
                    in_paragraph = false;
                    in_text = false;
                }
                b"tbl" => table_depth = table_depth.saturating_sub(1),
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

fn consume_docx_paragraph(
    extracted: &mut ExtractedStructure,
    style: Option<&str>,
    text: &str,
    in_table: bool,
) {
    if text.is_empty() {
        return;
    }
    extracted.word_count = extracted.word_count.saturating_add(count_words(text));
    if extracted.content_fragments.len() < 400 {
        extracted.content_fragments.push(ContentFragment {
            text: bounded_text(text, 1_200),
            source_label: if style.unwrap_or_default().is_empty() {
                format!("Word 段落 {}", extracted.content_fragments.len() + 1)
            } else {
                format!(
                    "Word {} · 段落 {}",
                    style.unwrap_or_default(),
                    extracted.content_fragments.len() + 1
                )
            },
            modality: if in_table {
                SourceModality::Table
            } else {
                infer_source_modality(text)
            },
        });
    }
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
    let pdf_text = extract_pdf_text(path, &document, &page_numbers);
    let mut extracted = match pdf_text.source {
        PdfTextSource::LayoutAwareInspector => infer_from_inspector_markdown(&pdf_text.text),
        _ => infer_from_plain_text(&pdf_text.text),
    };
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
    if let Some(classification) = &pdf_text.classification {
        let confidence = pdf_text
            .classification_confidence
            .map(|value| format!("（置信度 {:.0}%）", value.clamp(0.0, 1.0) * 100.0))
            .unwrap_or_default();
        extracted.warnings.push(format!(
            "PDF 文档分类：{classification}{confidence}；已优先执行原生结构提取"
        ));
    }
    extracted.pdf_processing =
        pdf_text
            .classification
            .as_ref()
            .map(|classification| PdfProcessingSummary {
                classification: classification.clone(),
                confidence_percent: pdf_text
                    .classification_confidence
                    .map(|value| (value.clamp(0.0, 1.0) * 100.0).round() as u8)
                    .unwrap_or_default(),
                native_extraction: pdf_text.source.label().to_owned(),
                pages_needing_recognition: pdf_text.pages_needing_ocr.clone(),
                pages_with_tables: pdf_text.pages_with_tables.clone(),
                pages_with_columns: pdf_text.pages_with_columns.clone(),
                has_encoding_issues: pdf_text.has_encoding_issues,
            });
    match pdf_text.source {
        PdfTextSource::LayoutAwareInspector => extracted.warnings.push(
            "已使用布局感知 PDF 解析：按字体、坐标和分栏顺序规整文本；公式与复杂跨页表格仍需人工确认".to_owned(),
        ),
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
    if !pdf_text.pages_needing_ocr.is_empty() {
        extracted.warnings.push(format!(
            "检测到 {} 个页面需要 OCR 或字体解码复核：{}；当前版本未执行 OCR",
            pdf_text.pages_needing_ocr.len(),
            compact_page_ranges(&pdf_text.pages_needing_ocr)
        ));
    }
    if pdf_text.has_encoding_issues {
        extracted
            .warnings
            .push("检测到 PDF 字体编码异常；已保留可靠页面，异常页面需 OCR 或作者确认".to_owned());
    }
    if !pdf_text.pages_with_tables.is_empty() {
        extracted.warnings.push(format!(
            "原生表格候选页：{}；已优先保留版面结构，不使用文本 OCR 覆盖",
            compact_page_ranges(&pdf_text.pages_with_tables)
        ));
    }
    if !pdf_text.pages_with_columns.is_empty() {
        extracted.warnings.push(format!(
            "多栏版面候选页：{}；已按坐标重排阅读顺序",
            compact_page_ranges(&pdf_text.pages_with_columns)
        ));
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

struct PdfTextExtraction {
    text: String,
    source: PdfTextSource,
    classification: Option<String>,
    classification_confidence: Option<f32>,
    pages_needing_ocr: Vec<u32>,
    pages_with_tables: Vec<u32>,
    pages_with_columns: Vec<u32>,
    has_encoding_issues: bool,
}

fn extract_pdf_text(path: &Path, document: &Document, page_numbers: &[u32]) -> PdfTextExtraction {
    let inspection = std::panic::catch_unwind(|| pdf_inspector::process_pdf(path))
        .ok()
        .and_then(Result::ok);
    let classification = inspection
        .as_ref()
        .map(|result| format!("{:?}", result.pdf_type));
    let classification_confidence = inspection.as_ref().map(|result| result.confidence);
    let pages_needing_ocr = inspection
        .as_ref()
        .map(|result| result.pages_needing_ocr.clone())
        .unwrap_or_default();
    let pages_with_tables = inspection
        .as_ref()
        .map(|result| result.layout.pages_with_tables.clone())
        .unwrap_or_default();
    let pages_with_columns = inspection
        .as_ref()
        .map(|result| result.layout.pages_with_columns.clone())
        .unwrap_or_default();
    let has_encoding_issues = inspection
        .as_ref()
        .is_some_and(|result| result.has_encoding_issues);

    if let Some(result) = &inspection {
        let markdown = result.markdown.as_deref().unwrap_or_default();
        if !markdown.trim().is_empty() {
            return PdfTextExtraction {
                text: markdown.to_owned(),
                source: PdfTextSource::LayoutAwareInspector,
                classification,
                classification_confidence,
                pages_needing_ocr,
                pages_with_tables,
                pages_with_columns,
                has_encoding_issues,
            };
        }
    }
    let enhanced = std::panic::catch_unwind(|| pdf_extract::extract_text(path))
        .ok()
        .and_then(Result::ok)
        .unwrap_or_default();
    if !enhanced.trim().is_empty() {
        return PdfTextExtraction {
            text: enhanced,
            source: PdfTextSource::EnhancedFontMapping,
            classification,
            classification_confidence,
            pages_needing_ocr,
            pages_with_tables,
            pages_with_columns,
            has_encoding_issues,
        };
    }

    let basic = document.extract_text(page_numbers).unwrap_or_default();
    if basic.trim().is_empty() {
        PdfTextExtraction {
            text: String::new(),
            source: PdfTextSource::None,
            classification,
            classification_confidence,
            pages_needing_ocr: if pages_needing_ocr.is_empty() {
                page_numbers.to_vec()
            } else {
                pages_needing_ocr
            },
            pages_with_tables,
            pages_with_columns,
            has_encoding_issues,
        }
    } else {
        PdfTextExtraction {
            text: basic,
            source: PdfTextSource::BasicContentStream,
            classification,
            classification_confidence,
            pages_needing_ocr,
            pages_with_tables,
            pages_with_columns,
            has_encoding_issues,
        }
    }
}

fn infer_from_inspector_markdown(markdown: &str) -> ExtractedStructure {
    let plain_text = markdown_plain_text(markdown);
    let mut extracted = infer_from_plain_text(&plain_text);
    let headings = markdown
        .lines()
        .filter_map(|line| {
            let trimmed = line.trim();
            let marker_count = trimmed
                .chars()
                .take_while(|character| *character == '#')
                .count();
            if !(1..=4).contains(&marker_count)
                || !trimmed
                    .chars()
                    .nth(marker_count)
                    .is_some_and(char::is_whitespace)
            {
                return None;
            }
            let heading = normalize_line(trimmed[marker_count..].trim());
            (!heading.is_empty() && heading.chars().count() <= 300).then_some(SectionSummary {
                level: u8::try_from(marker_count).unwrap_or(1),
                heading,
            })
        })
        .collect::<Vec<_>>();
    if let Some(first) = headings.first() {
        if first.level == 1 {
            extracted.title = Some(first.heading.clone());
        }
    }
    let structural_headings = headings
        .into_iter()
        .skip_while(|heading| extracted.title.as_deref() == Some(heading.heading.as_str()))
        .collect::<Vec<_>>();
    if !structural_headings.is_empty() {
        extracted.sections = structural_headings;
    }
    extracted.table_count = extracted.table_count.max(count_markdown_tables(markdown));
    for (index, line) in markdown.lines().enumerate() {
        let trimmed = line.trim();
        let is_figure = trimmed.starts_with("![") || trimmed.starts_with("<figure");
        let cells = trimmed
            .trim_matches('|')
            .split('|')
            .map(str::trim)
            .collect::<Vec<_>>();
        let is_separator = cells.len() >= 2
            && cells.iter().all(|cell| {
                let marker = cell.trim_matches(':').trim();
                marker.len() >= 3 && marker.chars().all(|character| character == '-')
            });
        let is_table_row = trimmed.contains('|') && cells.len() >= 2 && !is_separator;
        if is_figure || is_table_row {
            extracted.content_fragments.push(ContentFragment {
                text: bounded_text(trimmed, 1_200),
                source_label: format!("PDF Markdown · 行 {}", index + 1),
                modality: if is_figure {
                    SourceModality::Figure
                } else {
                    SourceModality::Table
                },
            });
        }
        if extracted.content_fragments.len() >= 400 {
            break;
        }
    }
    extracted
}

fn count_markdown_tables(markdown: &str) -> u32 {
    markdown
        .lines()
        .filter(|line| {
            let trimmed = line.trim().trim_matches('|').trim();
            let cells = trimmed.split('|').map(str::trim).collect::<Vec<_>>();
            cells.len() >= 2
                && cells.iter().all(|cell| {
                    let marker = cell.trim_matches(':').trim();
                    marker.len() >= 3 && marker.chars().all(|character| character == '-')
                })
        })
        .count()
        .try_into()
        .unwrap_or(u32::MAX)
}

fn markdown_plain_text(markdown: &str) -> String {
    markdown
        .lines()
        .filter_map(|line| {
            let trimmed = line.trim();
            if trimmed.starts_with("<!--") && trimmed.ends_with("-->") {
                return None;
            }
            let without_heading = trimmed.trim_start_matches('#').trim_start();
            let without_quote = without_heading.trim_start_matches('>').trim_start();
            let without_list = without_quote
                .strip_prefix("- ")
                .or_else(|| without_quote.strip_prefix("* "))
                .unwrap_or(without_quote);
            let cleaned = without_list
                .replace("**", "")
                .replace("__", "")
                .replace('`', "")
                .replace('|', " ");
            (!cleaned.trim().is_empty()).then_some(cleaned)
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn compact_page_ranges(pages: &[u32]) -> String {
    let mut pages = pages.to_vec();
    pages.sort_unstable();
    pages.dedup();
    let mut ranges = Vec::new();
    let mut index = 0;
    while index < pages.len() {
        let start = pages[index];
        let mut end = start;
        while index + 1 < pages.len() && pages[index + 1] == end.saturating_add(1) {
            index += 1;
            end = pages[index];
        }
        ranges.push(if start == end {
            start.to_string()
        } else {
            format!("{start}-{end}")
        });
        index += 1;
    }
    ranges.join(", ")
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
        content_fragments: text_fragments(text, "提取文本"),
        ..ExtractedStructure::default()
    }
}

fn text_fragments(text: &str, source_prefix: &str) -> Vec<ContentFragment> {
    text.lines()
        .enumerate()
        .flat_map(|(line_index, line)| {
            let sentences = semantic_sentences(line);
            if sentences.is_empty() {
                vec![(line_index, normalize_line(line))]
            } else {
                sentences
                    .into_iter()
                    .map(|sentence| (line_index, sentence))
                    .collect()
            }
        })
        .filter(|(line_index, line)| line.chars().count() >= if *line_index < 80 { 4 } else { 16 })
        .take(400)
        .enumerate()
        .map(|(index, (_, line))| ContentFragment {
            modality: infer_source_modality(&line),
            text: bounded_text(&line, 1_200),
            source_label: format!("{source_prefix} · 片段 {}", index + 1),
        })
        .collect()
}

fn infer_source_modality(text: &str) -> SourceModality {
    let normalized = text.trim().to_ascii_lowercase();
    let numbered_chinese_label = |prefix: char| {
        normalized
            .strip_prefix(prefix)
            .and_then(|remainder| remainder.chars().next())
            .is_some_and(|character| {
                character.is_ascii_digit()
                    || character.is_whitespace()
                    || matches!(character, ':' | '：')
            })
    };
    if normalized.starts_with("table ")
        || normalized.starts_with("表 ")
        || normalized.starts_with("表：")
        || numbered_chinese_label('表')
    {
        SourceModality::Table
    } else if normalized.starts_with("figure ")
        || normalized.starts_with("fig. ")
        || normalized.starts_with("图 ")
        || normalized.starts_with("图：")
        || numbered_chinese_label('图')
    {
        SourceModality::Figure
    } else {
        SourceModality::Text
    }
}

fn bounded_text(text: &str, maximum_chars: usize) -> String {
    let normalized = normalize_line(text);
    if normalized.chars().count() <= maximum_chars {
        return normalized;
    }
    normalized.chars().take(maximum_chars).collect::<String>() + "…"
}

fn derive_semantic_candidates(
    extracted: &ExtractedStructure,
) -> (Vec<SemanticCandidate>, ExtractionCoverage) {
    let mut fragments = extracted.content_fragments.clone();
    if let Some(abstract_text) = extracted.abstract_text.as_deref() {
        fragments.insert(
            0,
            ContentFragment {
                text: bounded_text(abstract_text, 2_400),
                source_label: "摘要 / Abstract".to_owned(),
                modality: SourceModality::Text,
            },
        );
    }
    let extraction_coverage = ExtractionCoverage {
        text_fragments: fragments
            .iter()
            .filter(|fragment| fragment.modality == SourceModality::Text)
            .count()
            .try_into()
            .unwrap_or(u32::MAX),
        table_fragments: fragments
            .iter()
            .filter(|fragment| fragment.modality == SourceModality::Table)
            .count()
            .try_into()
            .unwrap_or(u32::MAX),
        figure_fragments: fragments
            .iter()
            .filter(|fragment| fragment.modality == SourceModality::Figure)
            .count()
            .try_into()
            .unwrap_or(u32::MAX),
    };

    let mut scored = Vec::new();
    for fragment in &fragments {
        for sentence in semantic_sentences(&fragment.text) {
            for element in [
                SemanticElementKind::Claim,
                SemanticElementKind::Scope,
                SemanticElementKind::Method,
                SemanticElementKind::Result,
                SemanticElementKind::Evidence,
            ] {
                if let Some(confidence_percent) = semantic_score(
                    element,
                    &sentence,
                    &fragment.source_label,
                    fragment.modality,
                ) {
                    scored.push(SemanticCandidate {
                        element,
                        text: sentence.clone(),
                        source_label: fragment.source_label.clone(),
                        source_fragment_id: None,
                        modality: fragment.modality,
                        confidence_percent,
                    });
                }
            }
        }
    }

    if !scored
        .iter()
        .any(|candidate| candidate.element == SemanticElementKind::Claim)
    {
        if let Some(abstract_text) = extracted.abstract_text.as_deref() {
            if let Some(sentence) = semantic_sentences(abstract_text)
                .into_iter()
                .filter(|sentence| sentence.chars().count() >= 24)
                .max_by_key(|sentence| sentence.chars().count())
            {
                scored.push(SemanticCandidate {
                    element: SemanticElementKind::Claim,
                    text: sentence,
                    source_label: "摘要 / Abstract".to_owned(),
                    source_fragment_id: None,
                    modality: SourceModality::Text,
                    confidence_percent: 58,
                });
            }
        }
    }

    scored.sort_by(|left, right| {
        left.element
            .cmp(&right.element)
            .then_with(|| right.confidence_percent.cmp(&left.confidence_percent))
            .then_with(|| left.source_label.cmp(&right.source_label))
    });
    let mut candidates = Vec::new();
    for candidate in scored {
        let same_kind_count = candidates
            .iter()
            .filter(|existing: &&SemanticCandidate| existing.element == candidate.element)
            .count();
        if same_kind_count >= 3
            || candidates.iter().any(|existing: &SemanticCandidate| {
                existing.element == candidate.element
                    && existing.text.eq_ignore_ascii_case(&candidate.text)
            })
        {
            continue;
        }
        candidates.push(candidate);
    }
    (candidates, extraction_coverage)
}

fn semantic_sentences(text: &str) -> Vec<String> {
    text.split_inclusive(['。', '！', '？', '.', '!', '?'])
        .flat_map(|part| part.split(['\n', '\r']))
        .map(normalize_line)
        .filter(|sentence| {
            let length = sentence.chars().count();
            (16..=900).contains(&length)
        })
        .collect()
}

fn semantic_score(
    element: SemanticElementKind,
    sentence: &str,
    source_label: &str,
    modality: SourceModality,
) -> Option<u8> {
    let lower = sentence.to_ascii_lowercase();
    let contains_any = |markers: &[&str]| markers.iter().any(|marker| lower.contains(marker));
    let in_abstract = source_label.contains("摘要") || source_label.contains("Abstract");
    let numeric = sentence.chars().any(|character| character.is_ascii_digit());
    let score = match element {
        SemanticElementKind::Claim
            if contains_any(&[
                "we demonstrate",
                "we show",
                "we find",
                "we conclude",
                "this study demonstrates",
                "our contribution",
                "we propose",
                "本文提出",
                "本研究表明",
                "研究发现",
                "结果表明",
                "本文证明",
                "主要贡献",
                "我们提出",
                "我们发现",
            ]) =>
        {
            82 + u8::from(in_abstract) * 8
        }
        SemanticElementKind::Scope
            if contains_any(&[
                "under ",
                "within ",
                "for patients",
                "participants",
                "dataset",
                "population",
                "sample",
                "assuming",
                "condition",
                "适用于",
                "在…条件",
                "在该条件",
                "研究对象",
                "样本",
                "数据集",
                "人群",
                "前提",
                "假设",
                "范围",
            ]) =>
        {
            70 + u8::from(in_abstract) * 8
        }
        SemanticElementKind::Method
            if contains_any(&[
                "we use",
                "we used",
                "we develop",
                "we developed",
                "we propose",
                "method",
                "algorithm",
                "experiment",
                "randomized",
                "regression",
                "simulation",
                "采用",
                "使用",
                "提出一种",
                "研究方法",
                "算法",
                "实验设计",
                "回归",
                "仿真",
            ]) =>
        {
            74 + u8::from(in_abstract) * 8
        }
        SemanticElementKind::Result
            if contains_any(&[
                "results show",
                "result shows",
                "we find",
                "we found",
                "significant",
                "increased",
                "decreased",
                "outperform",
                "accuracy",
                "results indicate",
                "结果表明",
                "研究发现",
                "显著",
                "提高了",
                "降低了",
                "优于",
                "准确率",
                "实验结果",
                "结果显示",
            ]) =>
        {
            78 + u8::from(numeric) * 7 + u8::from(in_abstract) * 5
        }
        SemanticElementKind::Evidence
            if modality != SourceModality::Text
                || (numeric
                    && contains_any(&[
                        "result",
                        "accuracy",
                        "significant",
                        "confidence",
                        "p=",
                        "p <",
                        "结果",
                        "准确率",
                        "显著",
                        "置信区间",
                        "图",
                        "表",
                    ])) =>
        {
            76 + u8::from(modality != SourceModality::Text) * 10
        }
        _ => return None,
    };
    Some(score.min(96))
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
        choose_pdf_title, compact_page_ranges, decode_pdf_text_string, derive_semantic_candidates,
        extract_pdf, extract_tex, infer_from_inspector_markdown, infer_from_plain_text,
        markdown_plain_text, parse_docx_xml, pdf_metadata_authors, pdf_metadata_title,
        AnalysisQuality, SemanticElementKind, SourceModality,
    };
    use lopdf::{
        content::{Content, Operation},
        dictionary, Document, Object, Stream,
    };
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
    fn derives_source_backed_knowledge_candidates_from_one_decomposition() {
        let extracted = infer_from_plain_text(
            "Synthetic Study\nAbstract\nWe propose a local method for multilingual manuscript analysis. Results show that the method improves extraction accuracy by 18 percent on the synthetic dataset.\nTable 1 Results show accuracy for every tested language.\nFigure 1 The workflow connects extracted evidence to the reported claim.",
        );

        let (candidates, coverage) = derive_semantic_candidates(&extracted);

        assert!(candidates
            .iter()
            .any(|candidate| candidate.element == SemanticElementKind::Claim));
        assert!(candidates
            .iter()
            .any(|candidate| candidate.element == SemanticElementKind::Method));
        assert!(candidates
            .iter()
            .any(|candidate| candidate.element == SemanticElementKind::Result));
        assert!(candidates
            .iter()
            .any(|candidate| candidate.element == SemanticElementKind::Evidence));
        assert!(candidates
            .iter()
            .any(|candidate| candidate.modality == SourceModality::Table));
        assert!(candidates
            .iter()
            .any(|candidate| candidate.modality == SourceModality::Figure));
        assert!(coverage.text_fragments > 0);
        assert_eq!(coverage.table_fragments, 1);
        assert_eq!(coverage.figure_fragments, 1);
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
    fn normalizes_layout_aware_markdown_into_academic_structure() {
        let markdown = "# Layout-Aware Research\n\nAda Author, Ben Researcher\n\n## Abstract\n\nThis paper evaluates a deterministic multi-column PDF extraction pipeline with enough words for a reliable abstract candidate.\n\n**Keywords:** PDF, layout\n\n## 1 Introduction\n\nBody text.\n\n### 1.1 Method\n\n| Metric | Value |\n| --- | --- |\n| Recall | 0.91 |\n\n## References";
        let extracted = infer_from_inspector_markdown(markdown);

        assert_eq!(extracted.title.as_deref(), Some("Layout-Aware Research"));
        assert_eq!(extracted.authors, vec!["Ada Author", "Ben Researcher"]);
        assert!(extracted.abstract_present);
        assert_eq!(
            extracted
                .sections
                .iter()
                .map(|section| (section.level, section.heading.as_str()))
                .collect::<Vec<_>>(),
            vec![
                (2, "Abstract"),
                (2, "1 Introduction"),
                (3, "1.1 Method"),
                (2, "References")
            ]
        );
        assert!(extracted.references_present);
        assert_eq!(extracted.table_count, 1);
        assert!(!markdown_plain_text(markdown).contains('|'));
    }

    #[test]
    fn compacts_page_level_ocr_candidates() {
        assert_eq!(compact_page_ranges(&[9, 3, 2, 4, 9, 12]), "2-4, 9, 12");
        assert_eq!(compact_page_ranges(&[]), "");
    }

    #[test]
    fn uses_layout_aware_extraction_for_a_synthetic_text_pdf() {
        let path = synthetic_pdf_path();
        let mut document = Document::with_version("1.7");
        let pages_id = document.new_object_id();
        let font_id = document.add_object(
            dictionary! { "Type" => "Font", "Subtype" => "Type1", "BaseFont" => "Helvetica" },
        );
        let resources_id =
            document.add_object(dictionary! { "Font" => dictionary! { "F1" => font_id } });
        let content = Content { operations: vec![
            Operation::new("BT", vec![]),
            Operation::new("Tf", vec![Object::Name(b"F1".to_vec()), 20.into()]),
            Operation::new("Td", vec![60.into(), 780.into()]),
            Operation::new("Tj", vec![Object::string_literal("Layout Aware Study")]),
            Operation::new("Tf", vec![Object::Name(b"F1".to_vec()), 11.into()]),
            Operation::new("Td", vec![0.into(), (-30).into()]),
            Operation::new("Tj", vec![Object::string_literal("Ada Author, Ben Researcher")]),
            Operation::new("Tf", vec![Object::Name(b"F1".to_vec()), 14.into()]),
            Operation::new("Td", vec![0.into(), (-35).into()]),
            Operation::new("Tj", vec![Object::string_literal("Abstract")]),
            Operation::new("Tf", vec![Object::Name(b"F1".to_vec()), 10.into()]),
            Operation::new("Td", vec![0.into(), (-22).into()]),
            Operation::new("Tj", vec![Object::string_literal("This paper evaluates reliable local PDF extraction for academic manuscripts.")]),
            Operation::new("Tf", vec![Object::Name(b"F1".to_vec()), 14.into()]),
            Operation::new("Td", vec![0.into(), (-36).into()]),
            Operation::new("Tj", vec![Object::string_literal("1 Introduction")]),
            Operation::new("ET", vec![]),
        ]}.encode().unwrap();
        let content_id = document.add_object(Stream::new(dictionary! {}, content));
        let page_id = document.add_object(dictionary! { "Type" => "Page", "Parent" => pages_id, "MediaBox" => vec![0.into(), 0.into(), 595.into(), 842.into()], "Resources" => resources_id, "Contents" => content_id });
        document.objects.insert(
            pages_id,
            Object::Dictionary(
                dictionary! { "Type" => "Pages", "Kids" => vec![page_id.into()], "Count" => 1 },
            ),
        );
        let catalog_id =
            document.add_object(dictionary! { "Type" => "Catalog", "Pages" => pages_id });
        document.trailer.set("Root", catalog_id);
        document.save(&path).unwrap();

        let extracted = extract_pdf(&path).unwrap();
        let _ = fs::remove_file(path);

        assert!(extracted.title.is_some());
        assert!(extracted.abstract_present);
        assert!(extracted
            .warnings
            .iter()
            .any(|warning| warning.starts_with("已使用布局感知 PDF 解析")));
        assert!(extracted
            .warnings
            .iter()
            .any(|warning| warning.starts_with("PDF 文档分类：TextBased")));
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

    fn synthetic_pdf_path() -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("manuscriptdock-structure-{nonce}.pdf"))
    }
}
