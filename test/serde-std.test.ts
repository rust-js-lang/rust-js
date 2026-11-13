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

// Time: a `Duration` as `{"secs","nanos"}`, its nanoseconds carried where
// they're a second or more, and serde's `overflow deserializing Duration`
// where they carry past `u64::MAX` seconds; a `SystemTime` as its time since
// the epoch, serde's error of one before it, and of an offset unix's
// `SystemTime` can't hold.
test("serde: a Duration and a SystemTime are written and read as serde's own impls do", async () => {
  const dir = fixture("serde-std-time");
  writeFileSync(join(dir, "cases.rs"), harness + String.raw`
use std::time::{Duration, SystemTime, UNIX_EPOCH};

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct Timing {
    pub took: Duration,
    pub at: SystemTime,
    pub every: Vec<Duration>,
}

fn write_result<T: serde::Serialize>(out: &mut String, value: &T) {
    match serde_json::to_string(value) {
        Ok(text) => out.push_str(&format!("{text}\n")),
        Err(e) => out.push_str(&format!("error: {e}\n")),
    }
    match serde_json::to_value(value) {
        Ok(v) => out.push_str(&format!("{v}\n")),
        Err(e) => out.push_str(&format!("error: {e}\n")),
    }
}

pub fn report() -> String {
    let mut out = String::new();
    write(&mut out, &Duration::new(5, 30));
    write(&mut out, &Duration::from_millis(1500));
    write(&mut out, &Duration::MAX);
    write(&mut out, &Duration::ZERO);
    let at = UNIX_EPOCH + Duration::new(1_700_000_000, 123_456_789);
    write(&mut out, &at);
    write(&mut out, &UNIX_EPOCH);
    let timing = Timing { took: Duration::from_micros(2500), at, every: vec![Duration::from_secs(1), Duration::new(0, 1)] };
    write(&mut out, &timing);
    write_result(&mut out, &(UNIX_EPOCH - Duration::from_secs(1)));
    write_result(&mut out, &vec![Some(UNIX_EPOCH - Duration::from_nanos(1))]);
    for text in [
        r#"{"secs":1,"nanos":2}"#,
        r#"{"nanos":2,"secs":1}"#,
        "[3,4]",
        r#"{"secs":1,"nanos":2000000000}"#,
        r#"{"secs":1,"nanos":4294967295}"#,
        r#"{"secs":18446744073709551615,"nanos":999999999}"#,
        r#"{"secs":18446744073709551615,"nanos":1000000000}"#,
        "[18446744073709551614,2000000000]",
        r#"{"secs":1}"#,
        r#"{"secs":1,"nanos":2,"millis":3}"#,
        r#"{"secs":1,"secs":2,"nanos":3}"#,
        r#"{"secs":-1,"nanos":0}"#,
        r#"{"secs":1,"nanos":4294967296}"#,
        "[1]",
        "1.5",
    ] {
        read::<Duration>(&mut out, text);
    }
    for text in [
        r#"{"secs_since_epoch":1700000000,"nanos_since_epoch":5}"#,
        "[0,0]",
        r#"{"secs_since_epoch":1,"nanos_since_epoch":1500000000}"#,
        r#"{"secs_since_epoch":9223372036854775807,"nanos_since_epoch":999999999}"#,
        r#"{"secs_since_epoch":9223372036854775807,"nanos_since_epoch":1000000000}"#,
        r#"{"secs_since_epoch":9223372036854775808,"nanos_since_epoch":0}"#,
        r#"{"secs_since_epoch":18446744073709551615,"nanos_since_epoch":1000000000}"#,
        r#"{"secs":1,"nanos":2}"#,
        r#"{"secs_since_epoch":1}"#,
    ] {
        match serde_json::from_str::<SystemTime>(text) {
            Ok(t) => out.push_str(&format!("{text} -> {:?}\n", t.duration_since(UNIX_EPOCH))),
            Err(e) => out.push_str(&format!("{text} -> error: {e}\n")),
        }
    }
    let back: Timing = serde_json::from_str(&serde_json::to_string(&timing).unwrap()).unwrap();
    out.push_str(&format!("{:?} {:?} {:?}\n", back.took, back.at.duration_since(UNIX_EPOCH), back.every));
    read_value::<Duration>(&mut out, serde_json::json!({"secs": 7, "nanos": 8}));
    read_value::<Duration>(&mut out, serde_json::json!([u64::MAX, 2_000_000_000u32]));
    read_value::<Duration>(&mut out, serde_json::json!({"secs": 7}));
    out
}
`);
  const expected = JSON.parse(run([native(dir)]));
  expect((await compiled(dir)).report()).toBe(expected);
});
