use crate::{ManuscriptKind, ManuscriptSummary, ManuscriptVersionSummary, WorkspaceSummary};
use quick_xml::{escape::unescape, events::Event, Reader};
use serde::{Deserialize, Serialize};
use std::{
    error::Error,
    fmt,
    fs::File,
    io::{Read, Write},
    path::Path,
};
use zip::{write::SimpleFileOptions, ZipArchive, ZipWriter};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RevisionFieldKind {
    Title,
    Abstract,
    Keywords,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RevisionField {
    pub field: RevisionFieldKind,
    pub label: String,
    pub label_en: String,
    pub value: String,
    pub editable: bool,
    pub limitation: Option<String>,
    pub limitation_en: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RevisionDraft {
    pub workspace_id: String,
    pub base_version: u32,
    pub format: String,
    pub fields: Vec<RevisionField>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RevisionChangeInput {
    pub field: RevisionFieldKind,
    pub after: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RevisionChange {
    pub field: RevisionFieldKind,
    pub before: String,
    pub after: String,
    pub basis: String,
    pub status: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RevisionSet {
    pub revision_id: String,
    pub workspace_id: String,
    pub base_version: u32,
    pub output_version: u32,
    pub created_unix_ms: u64,
    pub changes: Vec<RevisionChange>,
    pub external_transmission: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum RevisionApplication {
    Created {
        workspace: Box<WorkspaceSummary>,
        version: Box<ManuscriptVersionSummary>,
        revision_set: RevisionSet,
    },
    Unchanged {
        version: u32,
        message: String,
    },
}

#[derive(Debug)]
pub enum RevisionError {
    Io(std::io::Error),
    UnsupportedFormat,
    FieldUnavailable(RevisionFieldKind),
    InvalidValue(RevisionFieldKind),
    InvalidDocx(String),
    InvalidTextEncoding,
}

impl fmt::Display for RevisionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(f, "无法生成本地修订稿：{error}"),
            Self::UnsupportedFormat => write!(
                f,
                "PDF 仅提供只读证据；请使用 DOCX 或 TEX 源稿进行结构化修订"
            ),
            Self::FieldUnavailable(field) => write!(f, "当前稿件中无法安全定位修订字段 {field:?}"),
            Self::InvalidValue(field) => {
                write!(f, "修订字段 {field:?} 不能为空或超过 20000 个字符")
            }
            Self::InvalidDocx(message) => write!(f, "DOCX 无法安全修订：{message}"),
            Self::InvalidTextEncoding => write!(f, "TEX 文件不是有效的 UTF-8 文本"),
        }
    }
}
impl Error for RevisionError {}
impl From<std::io::Error> for RevisionError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

pub(crate) fn extract_revision_fields(
    path: &Path,
    manuscript: &ManuscriptSummary,
) -> Result<(Vec<RevisionField>, Vec<String>), RevisionError> {
    match manuscript.kind {
        ManuscriptKind::Latex => extract_tex_fields(path),
        ManuscriptKind::Word => extract_docx_fields(path),
        ManuscriptKind::Pdf => Ok((
            Vec::new(),
            vec!["PDF 保持只读；请提供 DOCX 或 TEX 源稿进行结构化修订".to_owned()],
        )),
    }
}

pub(crate) fn apply_revision(
    path: &Path,
    output: &Path,
    manuscript: &ManuscriptSummary,
    inputs: &[RevisionChangeInput],
) -> Result<Vec<RevisionChange>, RevisionError> {
    let (fields, _) = extract_revision_fields(path, manuscript)?;
    let mut changes = Vec::new();
    for input in inputs {
        let after = input.after.trim().to_owned();
        if after.is_empty() || after.chars().count() > 20_000 {
            return Err(RevisionError::InvalidValue(input.field));
        }
        let field = fields
            .iter()
            .find(|field| field.field == input.field && field.editable)
            .ok_or(RevisionError::FieldUnavailable(input.field))?;
        if field.value != after {
            changes.push(RevisionChange {
                field: input.field,
                before: field.value.clone(),
                after,
                basis: "author_edit".to_owned(),
                status: "accepted".to_owned(),
            });
        }
    }
    if changes.is_empty() {
        std::fs::copy(path, output)?;
        return Ok(changes);
    }
    match manuscript.kind {
        ManuscriptKind::Latex => rewrite_tex(path, output, &changes),
        ManuscriptKind::Word => rewrite_docx(path, output, &changes),
        ManuscriptKind::Pdf => Err(RevisionError::UnsupportedFormat),
    }?;
    Ok(changes)
}

fn labels(field: RevisionFieldKind) -> (&'static str, &'static str) {
    match field {
        RevisionFieldKind::Title => ("论文标题", "Manuscript title"),
        RevisionFieldKind::Abstract => ("摘要", "Abstract"),
        RevisionFieldKind::Keywords => ("关键词", "Keywords"),
    }
}
fn field(
    field: RevisionFieldKind,
    value: String,
    editable: bool,
    limitation: Option<(&str, &str)>,
) -> RevisionField {
    let (label, label_en) = labels(field);
    RevisionField {
        field,
        label: label.to_owned(),
        label_en: label_en.to_owned(),
        value,
        editable,
        limitation: limitation.map(|v| v.0.to_owned()),
        limitation_en: limitation.map(|v| v.1.to_owned()),
    }
}

fn extract_tex_fields(path: &Path) -> Result<(Vec<RevisionField>, Vec<String>), RevisionError> {
    let source =
        String::from_utf8(std::fs::read(path)?).map_err(|_| RevisionError::InvalidTextEncoding)?;
    let mut fields = Vec::new();
    if let Some(value) = tex_command(&source, "title") {
        fields.push(field(RevisionFieldKind::Title, value, true, None));
    }
    if let Some(value) = tex_environment(&source, "abstract") {
        fields.push(field(RevisionFieldKind::Abstract, value, true, None));
    }
    if let Some(value) = tex_command(&source, "keywords") {
        fields.push(field(RevisionFieldKind::Keywords, value, true, None));
    }
    Ok((fields, Vec::new()))
}

fn extract_docx_fields(path: &Path) -> Result<(Vec<RevisionField>, Vec<String>), RevisionError> {
    let file = File::open(path)?;
    let mut archive =
        ZipArchive::new(file).map_err(|e| RevisionError::InvalidDocx(e.to_string()))?;
    let mut xml = String::new();
    archive
        .by_name("word/document.xml")
        .map_err(|e| RevisionError::InvalidDocx(e.to_string()))?
        .read_to_string(&mut xml)?;
    let title = docx_title_paragraph(&xml).and_then(|(_, _, fragment)| docx_text(fragment));
    let fields = title
        .into_iter()
        .map(|value| field(RevisionFieldKind::Title, value, true, None))
        .collect();
    Ok((
        fields,
        vec!["DOCX 首轮仅安全回写使用 Title 样式的标题；摘要和关键词继续保留为只读证据".to_owned()],
    ))
}

fn rewrite_tex(
    path: &Path,
    output: &Path,
    changes: &[RevisionChange],
) -> Result<(), RevisionError> {
    let mut source =
        String::from_utf8(std::fs::read(path)?).map_err(|_| RevisionError::InvalidTextEncoding)?;
    for change in changes {
        source = match change.field {
            RevisionFieldKind::Title => replace_tex_command(&source, "title", &change.after),
            RevisionFieldKind::Abstract => {
                replace_tex_environment(&source, "abstract", &change.after)
            }
            RevisionFieldKind::Keywords => replace_tex_command(&source, "keywords", &change.after),
        }
        .ok_or(RevisionError::FieldUnavailable(change.field))?;
    }
    std::fs::write(output, source)?;
    Ok(())
}

fn rewrite_docx(
    path: &Path,
    output: &Path,
    changes: &[RevisionChange],
) -> Result<(), RevisionError> {
    let input = File::open(path)?;
    let mut archive =
        ZipArchive::new(input).map_err(|e| RevisionError::InvalidDocx(e.to_string()))?;
    let mut writer = ZipWriter::new(File::create(output)?);
    for index in 0..archive.len() {
        let mut entry = archive
            .by_index(index)
            .map_err(|e| RevisionError::InvalidDocx(e.to_string()))?;
        let name = entry.name().to_owned();
        let options = SimpleFileOptions::default()
            .compression_method(entry.compression())
            .unix_permissions(entry.unix_mode().unwrap_or(0o644));
        if entry.is_dir() {
            writer
                .add_directory(name, options)
                .map_err(|e| RevisionError::InvalidDocx(e.to_string()))?;
            continue;
        }
        writer
            .start_file(name.clone(), options)
            .map_err(|e| RevisionError::InvalidDocx(e.to_string()))?;
        if name == "word/document.xml" {
            let mut xml = String::new();
            entry.read_to_string(&mut xml)?;
            for change in changes {
                if change.field != RevisionFieldKind::Title {
                    return Err(RevisionError::FieldUnavailable(change.field));
                }
                xml = replace_docx_title(&xml, &change.after)?;
            }
            writer.write_all(xml.as_bytes())?;
        } else {
            std::io::copy(&mut entry, &mut writer)?;
        }
    }
    writer
        .finish()
        .map_err(|e| RevisionError::InvalidDocx(e.to_string()))?;
    Ok(())
}

fn tex_command(source: &str, command: &str) -> Option<String> {
    let marker = format!("\\{command}");
    let start = source.find(&marker)? + marker.len();
    let open = start + source[start..].find('{')?;
    let (value, _) = balanced(source, open)?;
    Some(value.trim().to_owned())
}
fn balanced(source: &str, open: usize) -> Option<(&str, usize)> {
    let mut depth = 0;
    for (offset, ch) in source[open..].char_indices() {
        match ch {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    let close = open + offset;
                    return Some((&source[open + 1..close], close));
                }
            }
            _ => {}
        }
    }
    None
}
fn tex_environment(source: &str, name: &str) -> Option<String> {
    let begin = format!("\\begin{{{name}}}");
    let end = format!("\\end{{{name}}}");
    let start = source.find(&begin)? + begin.len();
    let close = start + source[start..].find(&end)?;
    Some(source[start..close].trim().to_owned())
}
fn replace_tex_command(source: &str, command: &str, value: &str) -> Option<String> {
    let marker = format!("\\{command}");
    let start = source.find(&marker)? + marker.len();
    let open = start + source[start..].find('{')?;
    let (_, close) = balanced(source, open)?;
    Some(format!(
        "{}{}{}",
        &source[..open + 1],
        value,
        &source[close..]
    ))
}
fn replace_tex_environment(source: &str, name: &str, value: &str) -> Option<String> {
    let begin = format!("\\begin{{{name}}}");
    let end = format!("\\end{{{name}}}");
    let start = source.find(&begin)? + begin.len();
    let close = start + source[start..].find(&end)?;
    Some(format!(
        "{}\n{}\n{}",
        &source[..start],
        value,
        &source[close..]
    ))
}

fn docx_title_paragraph(xml: &str) -> Option<(usize, usize, &str)> {
    let mut cursor = 0;
    while let Some(relative) = xml[cursor..].find("<w:p") {
        let start = cursor + relative;
        let boundary = xml.as_bytes().get(start + 4).copied()?;
        if boundary != b'>' && !boundary.is_ascii_whitespace() {
            cursor = start + 4;
            continue;
        }
        let end = start + xml[start..].find("</w:p>")? + 6;
        let fragment = &xml[start..end];
        if fragment.contains("<w:pStyle")
            && (fragment.contains("w:val=\"Title\"") || fragment.contains("w:val=\"title\""))
        {
            return Some((start, end, fragment));
        }
        cursor = end;
    }
    None
}
fn docx_text(fragment: &str) -> Option<String> {
    let mut reader = Reader::from_str(fragment);
    let mut value = String::new();
    let mut in_text = false;
    loop {
        match reader.read_event().ok()? {
            Event::Start(e) if e.name().as_ref().ends_with(b"t") => in_text = true,
            Event::Text(text) if in_text => {
                let decoded = reader.decoder().decode(text.as_ref()).ok()?;
                value.push_str(&unescape(&decoded).ok()?);
            }
            Event::End(e) if e.name().as_ref().ends_with(b"t") => in_text = false,
            Event::Eof => break,
            _ => {}
        }
    }
    (!value.trim().is_empty()).then(|| value.trim().to_owned())
}
fn replace_docx_title(xml: &str, value: &str) -> Result<String, RevisionError> {
    let (start, end, fragment) = docx_title_paragraph(xml)
        .ok_or(RevisionError::FieldUnavailable(RevisionFieldKind::Title))?;
    let escaped = quick_xml::escape::escape(value);
    let mut revised = String::with_capacity(fragment.len() + escaped.len());
    let mut cursor = 0;
    let mut first = true;
    while let Some(relative) = fragment[cursor..].find("<w:t") {
        let text_start = cursor + relative;
        let content_start = text_start
            + fragment[text_start..]
                .find('>')
                .ok_or_else(|| RevisionError::InvalidDocx("标题文本节点不完整".to_owned()))?
            + 1;
        let content_end = content_start
            + fragment[content_start..]
                .find("</w:t>")
                .ok_or_else(|| RevisionError::InvalidDocx("标题文本节点不完整".to_owned()))?;
        revised.push_str(&fragment[cursor..content_start]);
        if first {
            revised.push_str(&escaped);
            first = false;
        }
        cursor = content_end;
    }
    if first {
        return Err(RevisionError::InvalidDocx(
            "标题段落没有文本节点".to_owned(),
        ));
    }
    revised.push_str(&fragment[cursor..]);
    Ok(format!("{}{}{}", &xml[..start], revised, &xml[end..]))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        fs,
        time::{SystemTime, UNIX_EPOCH},
    };
    fn path(ext: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!(
            "manuscriptdock-revision-{}-{}.{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
            ext
        ))
    }
    #[test]
    fn revises_tex_fields_without_touching_other_content() {
        let input = path("tex");
        let output = path("tex");
        fs::write(&input, "\\title{Old}\n\\begin{abstract}Old abstract\\end{abstract}\n\\keywords{one, two}\n\\section{Methods}Keep").unwrap();
        let manuscript = ManuscriptSummary {
            name: "study.tex".to_owned(),
            extension: "tex".to_owned(),
            kind: ManuscriptKind::Latex,
            size_bytes: 0,
            modified_unix_ms: None,
        };
        let changes = apply_revision(
            &input,
            &output,
            &manuscript,
            &[
                RevisionChangeInput {
                    field: RevisionFieldKind::Title,
                    after: "New title".to_owned(),
                },
                RevisionChangeInput {
                    field: RevisionFieldKind::Abstract,
                    after: "New abstract".to_owned(),
                },
            ],
        )
        .unwrap();
        let revised = fs::read_to_string(&output).unwrap();
        assert_eq!(changes.len(), 2);
        assert!(revised.contains("\\title{New title}"));
        assert!(revised.contains("New abstract"));
        assert!(revised.contains("\\section{Methods}Keep"));
        let _ = fs::remove_file(input);
        let _ = fs::remove_file(output);
    }

    #[test]
    fn revises_a_docx_title_paragraph_and_preserves_other_entries() {
        let input = path("docx");
        let output = path("docx");
        let file = File::create(&input).unwrap();
        let mut archive = ZipWriter::new(file);
        archive
            .start_file("word/document.xml", SimpleFileOptions::default())
            .unwrap();
        archive.write_all(br#"<w:document xmlns:w="x"><w:body><w:p><w:pPr><w:pStyle w:val="Title"/></w:pPr><w:r><w:t>Old </w:t></w:r><w:r><w:t>title</w:t></w:r></w:p><w:p><w:r><w:t>Body stays</w:t></w:r></w:p></w:body></w:document>"#).unwrap();
        archive
            .start_file("custom/preserved.txt", SimpleFileOptions::default())
            .unwrap();
        archive.write_all(b"preserved").unwrap();
        archive.finish().unwrap();
        let manuscript = ManuscriptSummary {
            name: "study.docx".to_owned(),
            extension: "docx".to_owned(),
            kind: ManuscriptKind::Word,
            size_bytes: 0,
            modified_unix_ms: None,
        };
        let changes = apply_revision(
            &input,
            &output,
            &manuscript,
            &[RevisionChangeInput {
                field: RevisionFieldKind::Title,
                after: "New & safe title".to_owned(),
            }],
        )
        .unwrap();
        let mut revised = ZipArchive::new(File::open(&output).unwrap()).unwrap();
        let mut xml = String::new();
        revised
            .by_name("word/document.xml")
            .unwrap()
            .read_to_string(&mut xml)
            .unwrap();
        let mut preserved = String::new();
        revised
            .by_name("custom/preserved.txt")
            .unwrap()
            .read_to_string(&mut preserved)
            .unwrap();
        assert_eq!(changes.len(), 1);
        assert!(xml.contains("New &amp; safe title"));
        assert!(!xml.contains("Old "));
        assert!(xml.contains("Body stays"));
        assert_eq!(preserved, "preserved");
        let _ = fs::remove_file(input);
        let _ = fs::remove_file(output);
    }
}
