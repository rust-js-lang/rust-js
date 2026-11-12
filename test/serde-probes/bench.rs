#![allow(unused, dead_code)]
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Kind {
    Simple,
    Sized { w: u32, h: u32 },
    Linked { url: String },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Item {
    pub item_id: u32,
    pub name: String,
    pub tags: Vec<String>,
    pub price: f64,
    pub active: bool,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub note: Option<String>,
    pub dims: (f64, f64, f64),
    pub kind: Kind,
    pub attrs: BTreeMap<String, String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Doc {
    pub items: Vec<Item>,
    pub total: u32,
}

pub fn make(n: u32) -> Doc {
    let mut items = Vec::with_capacity(n as usize);
    for i in 0..n {
        let kind = match i % 3 {
            0 => Kind::Simple,
            1 => Kind::Sized { w: i, h: i / 2 },
            _ => Kind::Linked { url: format!("https://example.com/items/{i}") },
        };
        let mut attrs = BTreeMap::new();
        attrs.insert("color".to_string(), format!("c{}", i % 17));
        attrs.insert("size".to_string(), format!("s{}", i % 5));
        items.push(Item {
            item_id: i,
            name: format!("Item number {i} \"quoted\""),
            tags: vec![format!("tag{}", i % 7), "common".to_string(), "caf\u{e9}".to_string()],
            price: (i as f64) * 1.25 + 0.1,
            active: i % 2 == 0,
            note: if i % 4 == 0 { Some(format!("note {i}")) } else { None },
            dims: (i as f64 / 3.0, 2.5, 1e-3),
            kind,
            attrs,
        });
    }
    Doc { items, total: n }
}

pub fn write(doc: &Doc) -> String {
    serde_json::to_string(doc).unwrap()
}

pub fn read(text: &str) -> Doc {
    serde_json::from_str(text).unwrap()
}

pub fn read_value(text: &str) -> Value {
    serde_json::from_str(text).unwrap()
}

pub fn write_value(value: &Value) -> String {
    serde_json::to_string(value).unwrap()
}

pub fn count(doc: &Doc) -> usize {
    doc.items.iter().map(|item| item.tags.len() + item.name.len()).sum()
}

fn main() {
    let doc = make(3);
    let text = write(&doc);
    let back = read(&text);
    let value = read_value(&text);
    println!("{} {} {} {}", text.len(), back == doc, write_value(&value).len(), count(&back));
}
