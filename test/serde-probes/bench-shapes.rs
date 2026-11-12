// The shapes of JSON #1532's S0 measures beside #1542's document: each
// made at a size, written and read typed and as a `Value`.
//
// - `deep`: arrays of trees 30 levels deep, 60 of JSON's nesting as each
//   level is an object and an array, under serde_json's limit of 128;
// - `wide`: one object of `n` keys;
// - `long`: strings of 64 KiB, each with escapes and non-ASCII text;
// - `map`: `n` maps of eight keys each.
#![allow(unused, dead_code)]
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

/// A tree, nested as deep as it's made.
#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub enum Deep {
    Leaf(u32),
    Nest(Vec<Deep>),
}

fn deep(depth: u32, at: u32) -> Deep {
    match depth {
        0 => Deep::Leaf(at),
        _ => Deep::Nest(vec![deep(depth - 1, at), Deep::Leaf(depth)]),
    }
}

/// The text of `shape` at size `n`.
pub fn make(shape: &str, n: u32) -> String {
    match shape {
        "deep" => serde_json::to_string(&(0..n).map(|i| deep(30, i)).collect::<Vec<_>>()).unwrap(),
        "wide" => serde_json::to_string(&(0..n).map(|i| (format!("key{i:08}"), i)).collect::<BTreeMap<_, _>>()).unwrap(),
        "long" => {
            let piece = "a \"quoted\" line\\ caf\u{e9} \u{1F600} tab\t end\n";
            let text: String = piece.repeat(65_536 / piece.len());
            serde_json::to_string(&(0..n).map(|_| text.clone()).collect::<Vec<_>>()).unwrap()
        }
        "map" => serde_json::to_string(
            &(0..n)
                .map(|i| (0..8).map(|k| (format!("field{k}"), i + k)).collect::<BTreeMap<_, _>>())
                .collect::<Vec<_>>(),
        )
        .unwrap(),
        other => panic!("no shape {other}"),
    }
}

/// `text` read as `shape`'s type: how many values it holds.
pub fn read_typed(shape: &str, text: &str) -> usize {
    match shape {
        "deep" => serde_json::from_str::<Vec<Deep>>(text).unwrap().len(),
        "wide" => serde_json::from_str::<BTreeMap<String, u32>>(text).unwrap().len(),
        "long" => serde_json::from_str::<Vec<String>>(text).unwrap().len(),
        "map" => serde_json::from_str::<Vec<BTreeMap<String, u32>>>(text).unwrap().len(),
        other => panic!("no shape {other}"),
    }
}

/// `text` read as `shape`'s type, then written again: its length.
pub fn write_typed(shape: &str, text: &str) -> usize {
    match shape {
        "deep" => serde_json::to_string(&serde_json::from_str::<Vec<Deep>>(text).unwrap()).unwrap().len(),
        "wide" => serde_json::to_string(&serde_json::from_str::<BTreeMap<String, u32>>(text).unwrap()).unwrap().len(),
        "long" => serde_json::to_string(&serde_json::from_str::<Vec<String>>(text).unwrap()).unwrap().len(),
        "map" => serde_json::to_string(&serde_json::from_str::<Vec<BTreeMap<String, u32>>>(text).unwrap()).unwrap().len(),
        other => panic!("no shape {other}"),
    }
}

pub fn read_value(text: &str) -> Value {
    serde_json::from_str(text).unwrap()
}

pub fn write_value(value: &Value) -> String {
    serde_json::to_string(value).unwrap()
}

fn main() {
    for shape in ["deep", "wide", "long", "map"] {
        let text = make(shape, 2);
        println!("{shape} {} {} {}", text.len(), read_typed(shape, &text), write_value(&read_value(&text)).len());
    }
}
