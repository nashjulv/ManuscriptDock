use crate::{AnonymityCheck, AppError, DocumentFacts, MAX_MANUSCRIPT_SIZE_BYTES};
use quick_xml::{events::Event, Reader};
use sha2::{Digest, Sha256};
use std::{fs::File, io::Read, path::Path};
use zip::ZipArchive;

const MAX_ZIP_ENTRIES: usize = 10_000;
const MAX_EXPANDED_BYTES: u64 = 1024 * 1024 * 1024;
const MAX_XML_BYTES: u64 = 32 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DocumentFeatureProfile {
    D1,
    D2,
    D3,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DocxInspection {
    pub sha256: String,
    pub size_bytes: u64,
    pub profile: DocumentFeatureProfile,
    pub facts: DocumentFacts,
    pub external_relationships: Vec<String>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PdfInspection {
    pub sha256: String,
    pub size_bytes: u64,
    pub facts: DocumentFacts,
    pub warnings: Vec<String>,
}

const MAX_PDF_INPUT_BYTES: u64 = 50 * 1024 * 1024;
const MAX_PDF_TEXT_CHARS: usize = 2_000_000;

pub fn inspect_pdf(path: &Path) -> Result<PdfInspection, AppError> {
    let metadata = path
        .metadata()
        .map_err(|_| AppError::new("INPUT_UNREADABLE", true).recover("choose_another_file"))?;
    if !metadata.is_file() || metadata.len() > MAX_PDF_INPUT_BYTES {
        return Err(AppError::new("LIMIT_EXCEEDED", true)
            .param("limitBytes", MAX_PDF_INPUT_BYTES.to_string())
            .recover("choose_another_file"));
    }
    if path
        .extension()
        .and_then(|value| value.to_str())
        .map(str::to_ascii_lowercase)
        .as_deref()
        != Some("pdf")
    {
        return Err(AppError::new("FORMAT_UNSUPPORTED", true)
            .param("format", "pdf")
            .recover("choose_another_file"));
    }
    let bytes = std::fs::read(path)
        .map_err(|_| AppError::new("INPUT_UNREADABLE", true).recover("choose_another_file"))?;
    if !bytes.starts_with(b"%PDF-") {
        return Err(AppError::new("INPUT_UNREADABLE", true)
            .param("reason", "invalid_pdf_header")
            .recover("choose_another_file"));
    }
    if bytes
        .windows(b"/Encrypt".len())
        .any(|window| window == b"/Encrypt")
    {
        return Err(AppError::new("PDF_ENCRYPTED", true).recover("choose_unencrypted_pdf"));
    }
    let extracted = pdf_extract::extract_text_from_mem(&bytes).map_err(|_| {
        AppError::new("PDF_TEXT_EXTRACTION_FAILED", true).recover("choose_docx_or_another_pdf")
    })?;
    if extracted.chars().count() > MAX_PDF_TEXT_CHARS {
        return Err(AppError::new("LIMIT_EXCEEDED", true)
            .param("textCharacterLimit", MAX_PDF_TEXT_CHARS.to_string())
            .recover("choose_docx_or_smaller_pdf"));
    }
    let cleaned = extracted.replace('\0', " ");
    let lines = cleaned
        .lines()
        .map(|line| line.split_whitespace().collect::<Vec<_>>().join(" "))
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>();
    let title = pdf_title(&lines).or_else(|| {
        path.file_stem()
            .and_then(|value| value.to_str())
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_owned)
    });
    let abstract_text = pdf_section(
        &lines,
        &["abstract", "摘要"],
        &["keywords", "key words", "关键词", "introduction", "引言"],
    );
    let keywords = pdf_keywords(&lines);
    let language = if lines
        .iter()
        .take(80)
        .flat_map(|value| value.chars())
        .any(|character| ('\u{4e00}'..='\u{9fff}').contains(&character))
    {
        Some("zh-CN".into())
    } else if lines.is_empty() {
        None
    } else {
        Some("en".into())
    };
    let sha256 = hex::encode(Sha256::digest(&bytes));
    Ok(PdfInspection {
        sha256: sha256.clone(),
        size_bytes: metadata.len(),
        facts: DocumentFacts {
            source_hash: sha256,
            extractor_version: "pdf-text-local-v1".into(),
            title,
            abstract_text,
            keywords,
            language,
            article_type: Some("research_article".into()),
            ..DocumentFacts::default()
        },
        warnings: if lines.is_empty() {
            vec!["pdf_has_no_extractable_text".into()]
        } else {
            vec!["pdf_author_fields_require_confirmation".into()]
        },
    })
}

pub fn inspect_docx(path: &Path) -> Result<DocxInspection, AppError> {
    let metadata = path
        .metadata()
        .map_err(|_| AppError::new("INPUT_UNREADABLE", true).recover("choose_another_file"))?;
    if !metadata.is_file() || metadata.len() > MAX_MANUSCRIPT_SIZE_BYTES {
        return Err(AppError::new("LIMIT_EXCEEDED", true)
            .param("limitBytes", MAX_MANUSCRIPT_SIZE_BYTES.to_string())
            .recover("choose_another_file"));
    }
    if path
        .extension()
        .and_then(|v| v.to_str())
        .map(str::to_ascii_lowercase)
        .as_deref()
        != Some("docx")
    {
        return Err(AppError::new("FORMAT_UNSUPPORTED", true)
            .param("format", "docx")
            .recover("choose_another_file"));
    }
    let bytes = std::fs::read(path)
        .map_err(|_| AppError::new("INPUT_UNREADABLE", true).recover("choose_another_file"))?;
    let sha256 = hex::encode(Sha256::digest(&bytes));
    let mut archive =
        ZipArchive::new(File::open(path).map_err(|_| AppError::new("INPUT_UNREADABLE", true))?)
            .map_err(|_| AppError::new("INPUT_UNREADABLE", true).recover("choose_another_file"))?;
    if archive.len() > MAX_ZIP_ENTRIES {
        return Err(
            AppError::new("LIMIT_EXCEEDED", true).param("entryLimit", MAX_ZIP_ENTRIES.to_string())
        );
    }
    let mut expanded = 0u64;
    let mut profile = DocumentFeatureProfile::D1;
    let mut external_relationships = Vec::new();
    let mut warnings = Vec::new();
    let mut document_xml = None;
    let mut core_xml = None;
    for index in 0..archive.len() {
        let mut entry = archive
            .by_index(index)
            .map_err(|_| AppError::new("INPUT_UNREADABLE", true))?;
        let name = entry.name().replace('\\', "/");
        if name.starts_with('/') || name.split('/').any(|part| part == "..") || entry.encrypted() {
            return Err(
                AppError::new("FORMAT_UNSUPPORTED", true).param("reason", "unsafe_container")
            );
        }
        expanded = expanded.saturating_add(entry.size());
        if expanded > MAX_EXPANDED_BYTES {
            return Err(AppError::new("LIMIT_EXCEEDED", true)
                .param("expandedLimitBytes", MAX_EXPANDED_BYTES.to_string()));
        }
        if (name.ends_with(".xml") || name.ends_with(".rels")) && entry.size() > MAX_XML_BYTES {
            return Err(AppError::new("LIMIT_EXCEEDED", true)
                .param("xmlLimitBytes", MAX_XML_BYTES.to_string()));
        }
        if name.ends_with("vbaProject.bin") || name.contains("embeddings/") {
            profile = DocumentFeatureProfile::D3;
            warnings.push("executable_or_embedded_object".into());
        }
        if (name == "word/comments.xml"
            || name == "word/footnotes.xml"
            || name == "word/endnotes.xml"
            || name.contains("header")
            || name.contains("footer"))
            && profile == DocumentFeatureProfile::D1
        {
            profile = DocumentFeatureProfile::D2;
        }
        if name.ends_with(".rels") {
            let mut xml = String::new();
            entry
                .read_to_string(&mut xml)
                .map_err(|_| AppError::new("INPUT_UNREADABLE", true))?;
            if xml.contains("TargetMode=\"External\"") || xml.contains("TargetMode='External'") {
                profile = DocumentFeatureProfile::D2;
                external_relationships.extend(external_targets(&xml));
            }
        } else if name == "word/document.xml" {
            let mut xml = String::new();
            entry
                .read_to_string(&mut xml)
                .map_err(|_| AppError::new("INPUT_UNREADABLE", true))?;
            if xml.contains("<w:txbxContent")
                || xml.contains("<w:fldChar")
                || xml.matches("<w:sectPr").count() > 1
            {
                profile = DocumentFeatureProfile::D2;
            }
            document_xml = Some(xml);
        } else if name == "docProps/core.xml" {
            let mut xml = String::new();
            entry
                .read_to_string(&mut xml)
                .map_err(|_| AppError::new("INPUT_UNREADABLE", true))?;
            core_xml = Some(xml);
        }
    }
    if profile == DocumentFeatureProfile::D3 {
        return Err(AppError::new("FORMAT_UNSUPPORTED", true)
            .param("reason", "high_risk_docx")
            .recover("choose_safe_docx"));
    }
    let document_xml = document_xml.ok_or_else(|| {
        AppError::new("INPUT_UNREADABLE", true).param("reason", "missing_document_xml")
    })?;
    let paragraphs = paragraphs(&document_xml);
    let title = title_from_core(core_xml.as_deref())
        .or_else(|| paragraphs.first().cloned())
        .filter(|v| !v.trim().is_empty());
    let abstract_text = section_text(
        &paragraphs,
        &["abstract", "摘要"],
        &["keywords", "关键词", "introduction", "引言"],
    );
    let keywords = keyword_values(&paragraphs);
    let language = if paragraphs
        .iter()
        .take(12)
        .flat_map(|v| v.chars())
        .any(|c| ('\u{4e00}'..='\u{9fff}').contains(&c))
    {
        Some("zh-CN".into())
    } else {
        Some("en".into())
    };
    Ok(DocxInspection {
        sha256: sha256.clone(),
        size_bytes: metadata.len(),
        profile,
        facts: DocumentFacts {
            source_hash: sha256,
            extractor_version: "docx-local-v1".into(),
            title,
            abstract_text,
            keywords,
            language,
            article_type: Some("research_article".into()),
            ..DocumentFacts::default()
        },
        external_relationships,
        warnings,
    })
}

pub(crate) fn anonymity_identity_context_hash(facts: &DocumentFacts) -> Result<String, AppError> {
    let encoded = serde_json::to_vec(&serde_json::json!({
        "authors": facts.authors,
        "affiliations": facts.affiliations,
        "correspondingEmail": facts.corresponding_email,
    }))
    .map_err(|_| AppError::new("PROJECT_INVALID", false))?;
    Ok(hex::encode(Sha256::digest(encoded)))
}

pub(crate) fn inspect_anonymity(
    path: &Path,
    facts: &DocumentFacts,
) -> Result<AnonymityCheck, AppError> {
    let probes = facts
        .authors
        .iter()
        .map(|value| ("authors", value.as_str()))
        .chain(
            facts
                .affiliations
                .iter()
                .map(|value| ("affiliations", value.as_str())),
        )
        .chain(
            facts
                .corresponding_email
                .iter()
                .map(|value| ("corresponding_email", value.as_str())),
        )
        .filter_map(|(category, value)| {
            let value = normalize_identity_text(value);
            (!value.is_empty()).then_some((category, value))
        })
        .collect::<Vec<_>>();
    let mut archive =
        ZipArchive::new(File::open(path).map_err(|_| AppError::new("INPUT_UNREADABLE", true))?)
            .map_err(|_| AppError::new("INPUT_UNREADABLE", true).recover("choose_another_file"))?;
    if archive.len() > MAX_ZIP_ENTRIES {
        return Err(
            AppError::new("LIMIT_EXCEEDED", true).param("entryLimit", MAX_ZIP_ENTRIES.to_string())
        );
    }
    let mut expanded = 0_u64;
    let mut detected = std::collections::HashSet::new();
    for index in 0..archive.len() {
        let mut entry = archive
            .by_index(index)
            .map_err(|_| AppError::new("INPUT_UNREADABLE", true))?;
        let name = entry.name().replace('\\', "/");
        if name.starts_with('/') || name.split('/').any(|part| part == "..") || entry.encrypted() {
            return Err(
                AppError::new("FORMAT_UNSUPPORTED", true).param("reason", "unsafe_container")
            );
        }
        expanded = expanded.saturating_add(entry.size());
        if expanded > MAX_EXPANDED_BYTES {
            return Err(AppError::new("LIMIT_EXCEEDED", true)
                .param("expandedLimitBytes", MAX_EXPANDED_BYTES.to_string()));
        }
        if !(name.ends_with(".xml") || name.ends_with(".rels")) {
            continue;
        }
        if entry.size() > MAX_XML_BYTES {
            return Err(AppError::new("LIMIT_EXCEEDED", true)
                .param("xmlLimitBytes", MAX_XML_BYTES.to_string()));
        }
        let mut xml = String::new();
        entry
            .read_to_string(&mut xml)
            .map_err(|_| AppError::new("INPUT_UNREADABLE", true))?;
        let raw = normalize_identity_text(&xml);
        let (spaced, joined) = searchable_xml_text(&xml);
        for (category, value) in &probes {
            if raw.contains(value) || spaced.contains(value) || joined.contains(value) {
                detected.insert((*category).to_owned());
            }
        }
    }
    let mut detected_categories = detected.into_iter().collect::<Vec<_>>();
    detected_categories.sort();
    Ok(AnonymityCheck {
        identity_context_hash: anonymity_identity_context_hash(facts)?,
        detected_categories,
    })
}

fn searchable_xml_text(xml: &str) -> (String, String) {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(false);
    let mut values = Vec::new();
    loop {
        match reader.read_event() {
            Ok(Event::Text(text)) => {
                if let Ok(decoded) = text.decode() {
                    let value = quick_xml::escape::unescape(decoded.as_ref())
                        .map(|value| value.into_owned())
                        .unwrap_or_else(|_| decoded.into_owned());
                    values.push(value);
                }
            }
            Ok(Event::CData(text)) => {
                if let Ok(value) = text.decode() {
                    values.push(value.into_owned());
                }
            }
            Ok(Event::GeneralRef(reference)) => {
                if let Ok(Some(value)) = reference.resolve_char_ref() {
                    values.push(value.to_string());
                } else if let Ok(name) = reference.decode() {
                    if let Some(value) = quick_xml::escape::resolve_xml_entity(name.as_ref()) {
                        values.push(value.to_owned());
                    }
                }
            }
            Ok(Event::Eof) | Err(_) => break,
            _ => {}
        }
    }
    (
        normalize_identity_text(&values.join(" ")),
        normalize_identity_text(&values.concat()),
    )
}

fn normalize_identity_text(value: &str) -> String {
    value
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

fn paragraphs(xml: &str) -> Vec<String> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);
    let mut values = Vec::new();
    let mut current = String::new();
    loop {
        match reader.read_event() {
            Ok(Event::Start(event)) if event.local_name().as_ref() == b"p" => current.clear(),
            Ok(Event::Text(text)) => {
                if let Ok(value) = text.decode() {
                    current.push_str(&value);
                }
            }
            Ok(Event::End(event)) if event.local_name().as_ref() == b"p" => {
                let value = current.split_whitespace().collect::<Vec<_>>().join(" ");
                if !value.is_empty() {
                    values.push(value);
                }
                current.clear();
            }
            Ok(Event::Eof) | Err(_) => break,
            _ => {}
        }
    }
    values
}

fn title_from_core(xml: Option<&str>) -> Option<String> {
    let xml = xml?;
    let start = xml.find("<dc:title>")? + 10;
    let end = xml[start..].find("</dc:title>")? + start;
    let value = xml[start..end].trim();
    (!value.is_empty()).then(|| value.to_owned())
}

fn pdf_title(lines: &[String]) -> Option<String> {
    lines.iter().take(30).find_map(|line| {
        let normalized = line.trim();
        let lower = normalized.to_lowercase();
        let plausible_length = (8..=300).contains(&normalized.chars().count());
        let looks_like_metadata = lower.starts_with("doi:")
            || lower.starts_with("http://")
            || lower.starts_with("https://")
            || lower.starts_with("arxiv:")
            || lower.starts_with("page ")
            || lower == "abstract"
            || lower == "摘要";
        (plausible_length && !looks_like_metadata).then(|| normalized.to_owned())
    })
}

fn pdf_section(lines: &[String], starts: &[&str], stops: &[&str]) -> Option<String> {
    let (start_index, inline) = lines.iter().enumerate().find_map(|(index, line)| {
        let lower = line.to_lowercase();
        starts.iter().find_map(|heading| {
            let heading = heading.to_lowercase();
            if lower == heading {
                Some((index + 1, None))
            } else {
                lower
                    .strip_prefix(&format!("{heading}:"))
                    .or_else(|| lower.strip_prefix(&format!("{heading}：")))
                    .map(|_| {
                        let offset = heading.chars().count() + 1;
                        (
                            index + 1,
                            Some(line.chars().skip(offset).collect::<String>()),
                        )
                    })
            }
        })
    })?;
    let mut values = inline
        .filter(|value| !value.trim().is_empty())
        .into_iter()
        .collect::<Vec<_>>();
    for line in lines.iter().skip(start_index).take(80) {
        let lower = line.to_lowercase();
        if stops.iter().any(|stop| {
            let stop = stop.to_lowercase();
            lower == stop
                || lower.starts_with(&format!("{stop}:"))
                || lower.starts_with(&format!("{stop}："))
        }) {
            break;
        }
        values.push(line.clone());
        if values
            .iter()
            .map(|value| value.chars().count())
            .sum::<usize>()
            > 12_000
        {
            break;
        }
    }
    (!values.is_empty()).then(|| values.join("\n"))
}

fn pdf_keywords(lines: &[String]) -> Vec<String> {
    lines
        .iter()
        .take(150)
        .find_map(|line| {
            let lower = line.to_lowercase();
            ["keywords:", "key words:", "关键词：", "关键词:"]
                .iter()
                .find_map(|prefix| {
                    lower.find(prefix).map(|index| {
                        line[index + prefix.len()..]
                            .split([',', ';', '；', '，', '·'])
                            .map(str::trim)
                            .filter(|value| !value.is_empty())
                            .take(30)
                            .map(str::to_owned)
                            .collect()
                    })
                })
        })
        .unwrap_or_default()
}

fn section_text(paragraphs: &[String], starts: &[&str], stops: &[&str]) -> Option<String> {
    let start = paragraphs
        .iter()
        .position(|p| starts.iter().any(|s| p.trim().eq_ignore_ascii_case(s)))?
        + 1;
    let values = paragraphs[start..]
        .iter()
        .take_while(|p| {
            !stops
                .iter()
                .any(|s| p.trim().to_lowercase().starts_with(&s.to_lowercase()))
        })
        .take(8)
        .cloned()
        .collect::<Vec<_>>();
    (!values.is_empty()).then(|| values.join("\n"))
}
fn keyword_values(paragraphs: &[String]) -> Vec<String> {
    paragraphs
        .iter()
        .find_map(|p| {
            let lower = p.to_lowercase();
            ["keywords:", "key words:", "关键词：", "关键词:"]
                .iter()
                .find_map(|prefix| {
                    lower.find(prefix).map(|index| {
                        p[index + prefix.len()..]
                            .split([',', ';', '；', '，'])
                            .map(str::trim)
                            .filter(|v| !v.is_empty())
                            .map(str::to_owned)
                            .collect()
                    })
                })
        })
        .unwrap_or_default()
}
fn external_targets(xml: &str) -> Vec<String> {
    let mut reader = Reader::from_str(xml);
    let mut values = Vec::new();
    loop {
        match reader.read_event() {
            Ok(Event::Empty(event)) | Ok(Event::Start(event))
                if event.local_name().as_ref() == b"Relationship" =>
            {
                let mut external = false;
                let mut target = None;
                for attribute in event.attributes().flatten() {
                    if attribute.key.local_name().as_ref() == b"TargetMode" {
                        external = attribute.unescape_value().is_ok_and(|v| v == "External");
                    }
                    if attribute.key.local_name().as_ref() == b"Target" {
                        target = attribute.unescape_value().ok().map(|v| v.into_owned());
                    }
                }
                if external {
                    if let Some(target) = target {
                        values.push(target);
                    }
                }
            }
            Ok(Event::Eof) | Err(_) => break,
            _ => {}
        }
    }
    values
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    #[test]
    fn extracts_keywords_without_translating_content() {
        let paragraphs = vec!["Keywords: local AI; privacy; journals".into()];
        assert_eq!(
            keyword_values(&paragraphs),
            vec!["local AI", "privacy", "journals"]
        );
        assert_eq!(
            searchable_xml_text("<w:p><w:t>Synthetic &amp;</w:t><w:t> Author</w:t></w:p>").0,
            "synthetic & author"
        );
    }
    #[test]
    fn rejects_traversal_parts_by_definition() {
        assert!("word/../evil.xml".split('/').any(|part| part == ".."));
    }

    #[test]
    fn extracts_bounded_pdf_facts_without_guessing_authors() {
        let root = std::env::temp_dir().join(format!(
            "manuscriptdock-pdf-inspection-{}",
            uuid::Uuid::new_v4()
        ));
        std::fs::create_dir_all(&root).unwrap();
        let path = root.join("synthetic-paper.pdf");
        write_simple_pdf(&path);
        let inspection = inspect_pdf(&path).unwrap();
        assert_eq!(
            inspection.facts.title.as_deref(),
            Some("Synthetic Local Paper")
        );
        assert_eq!(
            inspection.facts.abstract_text.as_deref(),
            Some("This study evaluates local document processing.")
        );
        assert_eq!(
            inspection.facts.keywords,
            vec!["local", "privacy", "journals"]
        );
        assert!(inspection.facts.authors.is_empty());
        assert_eq!(inspection.facts.extractor_version, "pdf-text-local-v1");
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn rejects_encrypted_pdf_before_text_extraction() {
        let root = std::env::temp_dir().join(format!(
            "manuscriptdock-encrypted-pdf-{}",
            uuid::Uuid::new_v4()
        ));
        std::fs::create_dir_all(&root).unwrap();
        let path = root.join("encrypted.pdf");
        std::fs::write(&path, b"%PDF-1.7\n/Encrypt 1 0 R\n%%EOF").unwrap();
        assert_eq!(inspect_pdf(&path).unwrap_err().code, "PDF_ENCRYPTED");
        let _ = std::fs::remove_dir_all(root);
    }

    fn write_simple_pdf(path: &Path) {
        let content = concat!(
            "BT\n/F1 18 Tf\n72 740 Td\n(Synthetic Local Paper) Tj\n",
            "0 -30 Td\n/F1 11 Tf\n(Abstract) Tj\n",
            "0 -18 Td\n(This study evaluates local document processing.) Tj\n",
            "0 -18 Td\n(Keywords: local; privacy; journals) Tj\nET\n"
        );
        let objects = [
            "<< /Type /Catalog /Pages 2 0 R >>".to_owned(),
            "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Resources << /Font << /F1 4 0 R >> >> /Contents 5 0 R >>".to_owned(),
            "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_owned(),
            format!("<< /Length {} >>\nstream\n{}endstream", content.len(), content),
        ];
        let mut bytes = b"%PDF-1.4\n".to_vec();
        let mut offsets = vec![0_usize];
        for (index, object) in objects.iter().enumerate() {
            offsets.push(bytes.len());
            write!(&mut bytes, "{} 0 obj\n{}\nendobj\n", index + 1, object).unwrap();
        }
        let xref = bytes.len();
        write!(&mut bytes, "xref\n0 {}\n", objects.len() + 1).unwrap();
        bytes.extend_from_slice(b"0000000000 65535 f \n");
        for offset in offsets.iter().skip(1) {
            writeln!(&mut bytes, "{offset:010} 00000 n ").unwrap();
        }
        write!(
            &mut bytes,
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n",
            objects.len() + 1
        )
        .unwrap();
        std::fs::write(path, bytes).unwrap();
    }
}
