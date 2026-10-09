//! In-memory representation ("IR") of a PDF document.
//!
//! The IR is the single data structure passed between extraction and diffing.
//! Its serde attributes are load-bearing: they define the on-disk JSON shape,
//! which is a stable compatibility contract for downstream consumers.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// The kind of an [`Element`].
///
/// Using an enum rather than a string means a `match` over it is exhaustive: a
/// new variant becomes a compile error at every site that must handle it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ElementType {
    TextBlock,
    Image,
}

impl ElementType {
    /// The wire/JSON name (`"text_block"` / `"image"`).
    pub fn as_str(self) -> &'static str {
        match self {
            ElementType::TextBlock => "text_block",
            ElementType::Image => "image",
        }
    }

    /// True for `text_block`; convenience for callers that branch on type.
    pub fn is_text(self) -> bool {
        matches!(self, ElementType::TextBlock)
    }
}

/// Text alignment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TextAlignment {
    Left,
    Right,
    Center,
    Justify,
}

/// Where an element sits on a page.
///
/// `bbox` is `[x0, y0, x1, y1]` in PDF user space: origin bottom-left, y
/// increasing upward.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ElementLayout {
    pub bbox: [f64; 4],
    pub page: i32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font_size_mean: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub alignment: Option<TextAlignment>,
}

/// Content of a text block element.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TextBlockContent {
    pub raw_text: String,
    pub normalized_text: String,
    pub tokens: Vec<String>,
}

/// Content of an image element.
///
/// The raw image bytes are not part of the IR; only the hash and dimensions are
/// retained. A perceptual hash can be computed later from the source PDF.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ImageContent {
    pub bytes_hash: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub width: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub height: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pdf_width: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pdf_height: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub colorspace: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bits_per_component: Option<i32>,
}

/// A single cell within a table.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TableCell {
    pub row: i32,
    pub col: i32,
    #[serde(default = "default_span")]
    pub rowspan: i32,
    #[serde(default = "default_span")]
    pub colspan: i32,
    #[serde(default)]
    pub text_norm: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hash: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bbox: Option<[f64; 4]>,
}

fn default_span() -> i32 {
    1
}

/// Table content. Kept for schema completeness even though the current
/// extractor does not yet emit tables.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TableContent {
    pub rows: i32,
    pub cols: i32,
    #[serde(default)]
    pub cells: Vec<TableCell>,
}

/// The typed `content` of an [`Element`].
///
/// Serialized without a discriminator: it writes the inner struct directly and,
/// on the way in, tries each variant in order. The variants have disjoint
/// required fields (`raw_text` vs `bytes_hash` vs `rows`), so the choice is
/// unambiguous.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ElementContent {
    TextBlock(TextBlockContent),
    Image(ImageContent),
    Table(TableContent),
}

/// A single extracted element: either a text block or an image.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Element {
    pub id: String,
    #[serde(rename = "type")]
    pub element_type: ElementType,
    pub layout: ElementLayout,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content: Option<ElementContent>,
    // These are serialized even when empty, and the map types give a stable key
    // order so output is deterministic.
    #[serde(default)]
    pub hashes: std::collections::BTreeMap<String, String>,
    #[serde(default)]
    pub source: serde_json::Map<String, serde_json::Value>,
    #[serde(default)]
    pub is_recurring: bool,
}

/// Page geometry.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PageMeta {
    pub index: i32,
    pub width: f64,
    pub height: f64,
}

fn default_version() -> String {
    "1.0".to_owned()
}

/// The root document IR.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DocumentIr {
    #[serde(default = "default_version")]
    pub version: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_pdf_sha256: Option<String>,
    #[serde(default)]
    pub meta: serde_json::Map<String, serde_json::Value>,
    pub pages: Vec<PageMeta>,
    pub elements: Vec<Element>,
}

impl DocumentIr {
    /// Index elements by id for O(1) lookup during diffing.
    ///
    /// The returned map borrows from `self` and is valid only while `self` is.
    pub fn index_by_id(&self) -> HashMap<&str, &Element> {
        self.elements
            .iter()
            .map(|element| (element.id.as_str(), element))
            .collect()
    }
}

/// Return elements in deterministic reading order: page ascending, then `y0`
/// descending (top to bottom for a bottom-left origin), then `x0` ascending,
/// then id ascending.
///
/// `f64` is only `PartialOrd` because of `NaN`, so coordinates are compared with
/// `partial_cmp` and a stable fallback.
pub fn sorted_elements(mut elements: Vec<Element>) -> Vec<Element> {
    use std::cmp::Ordering;

    elements.sort_by(|a, b| {
        a.layout
            .page
            .cmp(&b.layout.page)
            .then_with(|| {
                b.layout.bbox[1]
                    .partial_cmp(&a.layout.bbox[1])
                    .unwrap_or(Ordering::Equal)
            })
            .then_with(|| {
                a.layout.bbox[0]
                    .partial_cmp(&b.layout.bbox[0])
                    .unwrap_or(Ordering::Equal)
            })
            .then_with(|| a.id.cmp(&b.id))
    });
    elements
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn text_element(id: &str, page: i32, y0: f64, x0: f64) -> Element {
        Element {
            id: id.to_owned(),
            element_type: ElementType::TextBlock,
            layout: ElementLayout {
                bbox: [x0, y0, x0 + 90.0, y0 + 30.0],
                page,
                font_size_mean: None,
                alignment: None,
            },
            content: Some(ElementContent::TextBlock(TextBlockContent {
                raw_text: "Hello\n".to_owned(),
                normalized_text: "Hello".to_owned(),
                tokens: vec!["Hello".to_owned()],
            })),
            hashes: Default::default(),
            source: Default::default(),
            is_recurring: false,
        }
    }

    #[test]
    fn serializes_expected_json() {
        let doc = DocumentIr {
            version: "1.0".to_owned(),
            source_pdf_sha256: None,
            meta: Default::default(),
            pages: vec![PageMeta {
                index: 0,
                width: 200.0,
                height: 200.0,
            }],
            elements: vec![text_element("t1", 0, 10.0, 10.0)],
        };

        let value = serde_json::to_value(&doc).unwrap();
        assert_eq!(
            value,
            json!({
                "version": "1.0",
                "meta": {},
                "pages": [{ "index": 0, "width": 200.0, "height": 200.0 }],
                "elements": [{
                    "id": "t1",
                    "type": "text_block",
                    "layout": { "bbox": [10.0, 10.0, 100.0, 40.0], "page": 0 },
                    "content": {
                        "raw_text": "Hello\n",
                        "normalized_text": "Hello",
                        "tokens": ["Hello"]
                    },
                    "hashes": {},
                    "source": {},
                    "is_recurring": false
                }]
            })
        );
    }

    #[test]
    fn round_trips_through_json() {
        let doc = DocumentIr {
            version: "1.0".to_owned(),
            source_pdf_sha256: Some("abc".to_owned()),
            meta: Default::default(),
            pages: vec![PageMeta {
                index: 0,
                width: 200.0,
                height: 200.0,
            }],
            elements: vec![
                text_element("t1", 0, 10.0, 10.0),
                Element {
                    id: "i1".to_owned(),
                    element_type: ElementType::Image,
                    layout: ElementLayout {
                        bbox: [20.0, 20.0, 120.0, 60.0],
                        page: 0,
                        font_size_mean: None,
                        alignment: None,
                    },
                    content: Some(ElementContent::Image(ImageContent {
                        bytes_hash: "ABCDEF".to_owned(),
                        width: None,
                        height: None,
                        pdf_width: None,
                        pdf_height: None,
                        colorspace: None,
                        bits_per_component: None,
                    })),
                    hashes: [("bytes_hash".to_owned(), "ABCDEF".to_owned())]
                        .into_iter()
                        .collect(),
                    source: Default::default(),
                    is_recurring: false,
                },
            ],
        };

        let json = serde_json::to_string(&doc).unwrap();
        let restored: DocumentIr = serde_json::from_str(&json).unwrap();
        assert_eq!(doc, restored);
        assert!(matches!(
            restored.elements[0].content,
            Some(ElementContent::TextBlock(_))
        ));
        assert!(matches!(
            restored.elements[1].content,
            Some(ElementContent::Image(_))
        ));
    }

    #[test]
    fn sorted_elements_uses_reading_order() {
        let elements = vec![
            text_element("low", 0, 10.0, 5.0),
            text_element("high", 0, 90.0, 5.0),
            text_element("page1", 1, 90.0, 5.0),
        ];
        let sorted = sorted_elements(elements);
        let ids: Vec<&str> = sorted.iter().map(|e| e.id.as_str()).collect();
        assert_eq!(ids, vec!["high", "low", "page1"]);
    }

    #[test]
    fn element_type_helpers() {
        assert!(ElementType::TextBlock.is_text());
        assert!(!ElementType::Image.is_text());
        assert_eq!(ElementType::TextBlock.as_str(), "text_block");
        assert_eq!(ElementType::Image.as_str(), "image");
    }

    #[test]
    fn index_by_id_finds_elements() {
        let doc = DocumentIr {
            version: "1.0".to_owned(),
            source_pdf_sha256: None,
            meta: Default::default(),
            pages: vec![],
            elements: vec![
                text_element("a", 0, 1.0, 1.0),
                text_element("b", 0, 2.0, 2.0),
            ],
        };
        let index = doc.index_by_id();
        assert_eq!(index.len(), 2);
        assert_eq!(index["a"].id, "a");
        assert_eq!(index["b"].id, "b");
    }
}
