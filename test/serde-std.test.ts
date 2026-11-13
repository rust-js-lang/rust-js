// std's types whose serde impl is serde's own (#1623): each row of the
// catalog, `src/lower/serde/std_forms.rs`, written and read against native
// serde_json, the oracle, its errors among them, by family.
import { beforeAll, expect, test } from "bun:test";
import { writeFileSync } from "node:fs";
import { join } from "node:path";
import { compiled, native } from "./serde-support";
import { buildCompiler, buildSerde, fixture, run } from "./support";

beforeAll(() => {
  buildCompiler();
  buildSerde("rlib");
}, 600_000);

/** `report()`'s lines, each a value written, or a text read as `T`: what it
 * gives, `Debug`, or its error. */
const harness = String.raw`
fn read<T: serde::de::DeserializeOwned + std::fmt::Debug>(out: &mut String, text: &str) {
    match serde_json::from_str::<T>(text) {
        Ok(v) => out.push_str(&format!("{text} -> {v:?}\n")),
        Err(e) => out.push_str(&format!("{text} -> error: {e}\n")),
    }
}
fn read_value<T: serde::de::DeserializeOwned + std::fmt::Debug>(out: &mut String, value: serde_json::Value) {
    match serde_json::from_value::<T>(value.clone()) {
        Ok(v) => out.push_str(&format!("{value} as value -> {v:?}\n")),
        Err(e) => out.push_str(&format!("{value} as value -> error: {e}\n")),
    }
}
fn write<T: serde::Serialize>(out: &mut String, value: &T) {
    out.push_str(&serde_json::to_string(value).unwrap());
    out.push(' ');
    out.push_str(&serde_json::to_string_pretty(value).unwrap().replace('\n', "|"));
    out.push(' ');
    out.push_str(&serde_json::to_value(value).unwrap().to_string());
    out.push('\n');
}
`;

// Ranges and bounds: `Range` and `RangeInclusive` as `{"start","end"}`,
// read from a map or a sequence, an unknown, missing or duplicate field
// serde's error; `Bound` externally tagged.
test("serde: a range and a bound are written and read as serde's own impls do", async () => {
  const dir = fixture("serde-std-ranges");
  writeFileSync(join(dir, "cases.rs"), harness + String.raw`
use std::ops::{Bound, Range, RangeInclusive};

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct Window {
    pub rows: Range<u32>,
    pub cols: RangeInclusive<i64>,
    pub low: Bound<String>,
    pub high: Bound<f64>,
}

pub fn report() -> String {
    let mut out = String::new();
    write(&mut out, &(1u32..5));
    write(&mut out, &(-3i64..=3));
    write(&mut out, &(5u8..1));
    write(&mut out, &vec![Bound::Unbounded, Bound::Included(3u16), Bound::Excluded(4)]);
    let window = Window { rows: 0..10, cols: -1..=1, low: Bound::Included("a".into()), high: Bound::Unbounded };
    write(&mut out, &window);
    let mut stepped = 1u32..=2;
    stepped.next();
    stepped.next();
    write(&mut out, &stepped);
    for text in [
        r#"{"start":1,"end":5}"#,
        r#"{"end":5,"start":1}"#,
        "[2,7]",
        "[2]",
        "[2,7,9]",
        r#"{"start":1}"#,
        r#"{"start":1,"start":2,"end":3}"#,
        r#"{"start":1,"end":2,"step":3}"#,
        r#"{"start":-1,"end":2}"#,
        "null",
        "3",
    ] {
        read::<Range<u32>>(&mut out, text);
        read::<RangeInclusive<u32>>(&mut out, text);
    }
    for text in [
        r#""Unbounded""#,
        r#"{"Included":3}"#,
        r#"{"Excluded":-2}"#,
        r#""Included""#,
        r#"{"Unbounded":null}"#,
        r#"{"Sideways":1}"#,
        r#"{"Included":3,"Excluded":4}"#,
        "[]",
        "0",
    ] {
        read::<Bound<i32>>(&mut out, text);
    }
    read::<Window>(&mut out, r#"{"rows":{"start":0,"end":2},"cols":[1,1],"low":"Unbounded","high":{"Excluded":2.5}}"#);
    let back: Window = serde_json::from_str(&serde_json::to_string(&window).unwrap()).unwrap();
    out.push_str(&format!("{back:?}\n"));
    read_value::<Range<u8>>(&mut out, serde_json::json!({"start": 1, "end": 2}));
    read_value::<Range<u8>>(&mut out, serde_json::json!([3, 4]));
    read_value::<Range<u8>>(&mut out, serde_json::json!({"start": 1, "stop": 2}));
    read_value::<Bound<u8>>(&mut out, serde_json::json!({"Included": 7}));
    read_value::<Bound<u8>>(&mut out, serde_json::json!("Nowhere"));
    out
}
`);
  const expected = JSON.parse(run([native(dir)]));
  expect((await compiled(dir)).report()).toBe(expected);
});
