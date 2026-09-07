use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

/// Evidence is separate from layout regions and from author corrections.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecognitionObject {
    pub id: String,
    pub kind: String,
    pub label: String,
    pub text: String,
    pub page: u32,
    pub status: String,
    pub parser_version: u32,
    pub source_version: u32,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StructureReviewInput {
    pub source_content_hash: String,
    pub base_review_id: Option<String>,
    pub authors: Vec<String>,
    pub objects: Vec<RecognitionObject>,
    pub reason: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct StructureReview {
    pub id: String,
    pub workspace_id: String,
    pub source_version: u32,
    pub parser_version: u32,
    pub created_unix_ms: u64,
    pub input: StructureReviewInput,
    pub hash: String,
}

fn cjk(c: char) -> bool {
    matches!(c as u32, 0x3400..=0x9fff)
}

pub(crate) fn chinese_names(text: &str) -> Vec<String> {
    let text = text.replace('　', "");
    let names: Vec<_> = text
        .split(|c: char| {
            c.is_whitespace()
                || c.is_numeric()
                || matches!(c, ',' | '，' | '、' | ';' | '；' | '*' | '†' | '‡')
        })
        .filter(|part| !part.is_empty())
        .collect();
    if names.is_empty()
        || !names
            .iter()
            .all(|name| (2..=4).contains(&name.chars().count()) && name.chars().all(cjk))
    {
        return Vec::new();
    }
    names.iter().map(|s| (*s).to_owned()).collect()
}

pub(crate) fn caption_label(text: &str) -> Option<(String, String, bool)> {
    let text = text.trim();
    let lower = text.to_lowercase();
    let (kind, rest) = [
        ("figure", "figure"),
        ("figure", "fig."),
        ("figure", "图"),
        ("table", "table"),
        ("table", "表"),
    ]
    .into_iter()
    .find_map(|(kind, prefix)| lower.strip_prefix(prefix).map(|rest| (kind, rest)))?;
    let rest = rest.trim_start();
    let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
    if digits.is_empty() {
        return None;
    }
    let remainder = &rest[digits.len()..];
    let separated = remainder.is_empty()
        || remainder.starts_with(|c: char| c.is_whitespace() || matches!(c, ':' | '：'));
    if !separated {
        return None;
    } // 正文“图1展示…”不是题注。
    let caption =
        remainder.trim_start_matches(|c: char| c.is_whitespace() || matches!(c, ':' | '：'));
    let reference = [
        "shows",
        "show ",
        "illustrates",
        "is ",
        "展示",
        "所示",
        "表明",
        "给出",
    ]
    .iter()
    .any(|v| caption.starts_with(v));
    Some((kind.into(), digits, !reference))
}

pub(crate) fn pdf_objects(
    items: &[pdf_inspector::TextItem],
    version: u32,
) -> Vec<RecognitionObject> {
    let mut objects = Vec::new();
    let mut captions: BTreeMap<(String, String), usize> = BTreeMap::new();
    // Individual caption anchors survive layout tables and two-column reading-order errors.
    for item in items {
        let Some((kind, label, confident)) = caption_label(&item.text) else {
            continue;
        };
        let mut same_line: Vec<_> = items
            .iter()
            .filter(|other| {
                other.page == item.page && (other.y - item.y).abs() < 2.0 && other.x >= item.x
            })
            .collect();
        same_line.sort_by(|a, b| a.x.total_cmp(&b.x));
        let text = same_line
            .iter()
            .map(|part| part.text.trim())
            .take(24)
            .collect::<Vec<_>>()
            .join(" ");
        let key = (kind.clone(), label.clone());
        if let Some(index) = captions.get(&key).copied() {
            let previous: &mut RecognitionObject = &mut objects[index];
            // Bilingual captions and continued tables share one identity; retain both quotes.
            if !previous.text.contains(&item.text) {
                previous
                    .text
                    .push_str(&format!("\n[p{}] {}", item.page, text));
            }
            if confident {
                previous.status = "detected".into();
            }
            continue;
        }
        captions.insert(key, objects.len());
        objects.push(RecognitionObject {
            id: format!("recognition:{version}:{kind}:{label}"),
            kind,
            label,
            text,
            page: item.page,
            status: if confident { "detected" } else { "candidate" }.into(),
            parser_version: 9,
            source_version: version,
        });
    }
    let mut front: Vec<_> = items.iter().filter(|item| item.page == 1).collect();
    front.sort_by(|a, b| b.y.total_cmp(&a.y).then(a.x.total_cmp(&b.x)));
    let mut title_seen = false;
    let mut author_row_y: Option<f32> = None;
    let mut author_font: Option<f32> = None;
    for item in front {
        if item.text.chars().filter(|c| cjk(*c)).count() >= 8 && item.font_size >= 12.0 {
            title_seen = true;
            continue;
        }
        if !title_seen {
            continue;
        }
        if author_row_y.is_some_and(|y| y - item.y > 32.0) {
            break;
        }
        if author_font.is_some_and(|size| item.font_size < size * 0.85) {
            continue;
        }
        let names = chinese_names(&item.text);
        if item.font_size < 8.0 || names.is_empty() {
            continue;
        }
        // Name rows use an author-sized face, not address fragments under the affiliation.
        if author_row_y.is_none() && item.font_size < 10.0 {
            continue;
        }
        author_row_y.get_or_insert(item.y);
        author_font.get_or_insert(item.font_size);
        for name in names {
            let id = format!(
                "recognition:{version}:author:{}",
                hex::encode(Sha256::digest(name.as_bytes()))[..12].to_owned()
            );
            if objects.iter().any(|object| object.id == id) {
                continue;
            }
            objects.push(RecognitionObject {
                id,
                kind: "author".into(),
                label: name.clone(),
                text: item.text.clone(),
                page: 1,
                status: "detected".into(),
                parser_version: 9,
                source_version: version,
            });
        }
    }
    objects
}

#[cfg(test)]
mod tests {
    use super::*;
    fn item(text: &str, x: f32, y: f32, size: f32, page: u32) -> pdf_inspector::TextItem {
        pdf_inspector::TextItem {
            text: text.into(),
            x,
            y,
            width: 30.0,
            height: size,
            font: "Synthetic".into(),
            font_tag: "F1".into(),
            font_size: size,
            page,
            is_bold: false,
            is_italic: false,
            is_underline: false,
            is_strikeout: false,
            item_type: pdf_inspector::types::ItemType::Text,
            mcid: None,
        }
    }
    #[test]
    fn positional_anchors_recover_nine_authors_fourteen_figures_two_tables() {
        let mut items = vec![item("合成网络论文的现状与趋势", 30.0, 700.0, 15.0, 1)];
        for (index, name) in [
            "张　甲",
            "李乙",
            "王丙",
            "陈丁",
            "孙戊",
            "周己",
            "吴庚",
            "郑辛",
            "冯壬",
        ]
        .iter()
        .enumerate()
        {
            items.push(item(name, 30.0 + index as f32 * 40.0, 670.0, 13.0, 1));
        }
        items.push(item("北京", 80.0, 650.0, 8.0, 1));
        items.push(item("示例大学", 30.0, 650.0, 8.0, 1));
        for number in 1..=14 {
            items.push(item(
                &format!("Fig. {number}"),
                30.0,
                120.0,
                8.0,
                number + 1,
            ));
            items.push(item(&format!("图 {number}"), 30.0, 100.0, 8.0, number + 1));
        }
        for number in 1..=2 {
            items.push(item(
                &format!("Table {number}"),
                30.0,
                90.0,
                8.0,
                number + 1,
            ));
        }
        items.push(item("图3展示了网络", 30.0, 60.0, 8.0, 8));
        items.push(item("表格布局区域", 30.0, 30.0, 8.0, 8));
        let result = pdf_objects(&items, 1);
        for (kind, count) in [("author", 9), ("figure", 14), ("table", 2)] {
            assert_eq!(
                result
                    .iter()
                    .filter(|object| object.kind == kind && object.status == "detected")
                    .count(),
                count
            );
        }
        assert_eq!(
            result.iter().find(|o| o.kind == "author").unwrap().label,
            "张甲"
        );
        assert_eq!(
            result
                .iter()
                .find(|o| o.kind == "figure" && o.label == "14")
                .unwrap()
                .page,
            15
        );
    }
    #[test]
    fn gaps_and_continued_captions_do_not_inflate_counts() {
        let result = pdf_objects(
            &[
                item("Table 2", 30.0, 300.0, 8.0, 1),
                item("Table 2 (continued)", 30.0, 300.0, 8.0, 2),
                item("Fig. 9", 30.0, 120.0, 8.0, 1),
            ],
            2,
        );
        assert_eq!(result.len(), 2);
        assert_eq!(result.iter().filter(|o| o.kind == "figure").count(), 1);
    }
    #[test]
    fn chinese_spacing_and_superscripts_are_not_extra_authors() {
        assert_eq!(
            chinese_names("张　甲¹ 李乙² 王丙³ 陈丁 孙戊 周己 吴庚 郑辛 冯壬"),
            vec![
                "张甲", "李乙", "王丙", "陈丁", "孙戊", "周己", "吴庚", "郑辛", "冯壬"
            ]
        );
        assert!(chinese_names("北京示例大学计算机学院").is_empty());
    }
    #[test]
    fn captions_require_a_number_and_do_not_count_body_references() {
        assert_eq!(
            caption_label("Fig.\u{a0}14"),
            Some(("figure".into(), "14".into(), true))
        );
        assert!(caption_label("图片技术发展").is_none());
        assert!(caption_label("图1展示了网络").is_none());
        assert_eq!(
            caption_label("Table 2: Synthetic results"),
            Some(("table".into(), "2".into(), true))
        );
        assert!(!caption_label("Figure 2 shows our result").unwrap().2);
    }
}
