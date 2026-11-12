// `#[serde(..)]` attributes as serde_derive reads them, at each place it
// takes them: a field's own writer and reader, the container's default, a
// skip predicate, and their spans under `-C incremental`. Native serde_json
// is the oracle, the same source and dependency versions as the JavaScript
// (split from serde.test.ts by concept, #1334).
import { beforeAll, expect, test } from "bun:test";
import { writeFileSync } from "node:fs";
import { join } from "node:path";
import { compiled, native } from "./serde-support";
import { buildCompiler, buildSerde, compiler, fixture, run } from "./support";

beforeAll(() => {
  buildCompiler();
  buildSerde("rlib");
}, 600_000);

// A field's own writer and reader (ADR 0459): `#[serde(with = "module")]`,
// `serialize_with` and `deserialize_with`, each a hand-written codec.
test("serde: a field written and read by its own functions, as serde_json does", async () => {
  const dir = fixture("serde-field-with");
  writeFileSync(join(dir, "cases.rs"), String.raw`use serde::{Deserialize, Deserializer, Serialize, Serializer};

mod cents {
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S: Serializer>(value: &u64, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&format!("{}.{:02}", value / 100, value % 100))
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<u64, D::Error> {
        let text = String::deserialize(d)?;
        let (whole, part) = text.split_once('.').ok_or_else(|| serde::de::Error::custom("no point"))?;
        Ok(whole.parse::<u64>().map_err(serde::de::Error::custom)? * 100 + part.parse::<u64>().map_err(serde::de::Error::custom)?)
    }
}

fn upper<S: Serializer>(value: &str, s: S) -> Result<S::Ok, S::Error> {
    s.serialize_str(&value.to_uppercase())
}

fn lower<'de, D: Deserializer<'de>>(d: D) -> Result<String, D::Error> {
    Ok(String::deserialize(d)?.to_lowercase())
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Item {
    #[serde(with = "cents")]
    pub price: u64,
    #[serde(serialize_with = "upper", deserialize_with = "lower")]
    pub name: String,
    pub qty: u32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Pair(#[serde(with = "cents")] pub u64, pub bool);

pub fn report() -> String {
    let item = Item { price: 1250, name: "Pen".into(), qty: 2 };
    let read = |text: &str| match serde_json::from_str::<Item>(text) { Ok(i) => format!("{i:?}"), Err(e) => e.to_string() };
    let pair = |text: &str| match serde_json::from_str::<Pair>(text) { Ok(p) => format!("{p:?}"), Err(e) => e.to_string() };
    [
        serde_json::to_string(&item).unwrap(),
        serde_json::to_string(&Pair(5, true)).unwrap(),
        read("{\"price\": \"3.05\", \"name\": \"INK\", \"qty\": 1}"),
        read("{\"price\": \"305\", \"name\": \"x\", \"qty\": 1}"),
        read("{\"price\": 3, \"name\": \"x\", \"qty\": 1}"),
        pair("[\"0.07\", false]"),
    ].join("\n")
}
`);
  const expected = JSON.parse(run([native(dir)]));
  expect((await compiled(dir)).report()).toBe(expected);
});

// #450: `rust-js build` compiles with `-C incremental`, Cargo's default,
// under which rustc keeps each HIR span relative to its item: `#[serde(..)]`,
// read from the expanded crate, is found by each item's, variant's and
// field's `DefId`, not by the span of its name, so the JSON is serde's,
// `{"orderId":1}` and `{"kind":"waiting",..}`, with a cache cold and warm.
test("serde: #[serde(..)] holds under -C incremental, its cache cold and warm", async () => {
  const dir = fixture("serde-incremental");
  writeFileSync(join(dir, "cases.rs"), `#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Order {
    pub order_id: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}
#[derive(serde::Serialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum Status {
    Waiting { on: String },
}
#[derive(serde::Serialize)]
#[serde(tag = "t", content = "c")]
pub enum Shape {
    #[serde(rename = "circle")]
    Circle(f64),
}
pub fn report() -> String {
    let value = (Order { order_id: 1, note: None }, Status::Waiting { on: "ci".into() }, Shape::Circle(2.5));
    format!("{}\\n{}", serde_json::to_string(&value).unwrap(), serde_json::to_string_pretty(&value).unwrap())
}
`);
  const expected = JSON.parse(run([native(dir)]));
  expect(expected).toContain('"orderId":1');
  const cache = join(dir, "incremental");
  for (const build of ["cold", "warm"]) {
    const js = join(dir, `cases-${build}.js`);
    run([compiler, join(dir, "cases.rs"), "-o", js, "--", ...buildSerde(), "-C", `incremental=${cache}`]);
    expect((await import(js)).report(), build).toBe(expected);
  }
});

// A tuple struct with the container's `#[serde(default)]`, read from an
// array shorter than it: each missing element is the container default's,
// read as the representation reads a tuple struct's field, `defaults[1]`
// of the array it is, never `defaults._1`, which is `undefined` (#1537).
test("serde: a tuple struct's container default gives each missing element", async () => {
  const dir = fixture("serde-tuple-defaults");
  writeFileSync(join(dir, "cases.rs"), `#[derive(serde::Serialize, serde::Deserialize, Debug, Default)]
#[serde(default)]
pub struct T(pub i32, pub String);
#[derive(serde::Serialize, serde::Deserialize, Debug)]
#[serde(default)]
pub struct Given(pub u8, pub Vec<u32>, pub Option<String>, pub String);
impl Default for Given {
    fn default() -> Self {
        Given(7, vec![1, 2], Some("o".to_string()), "s".to_string())
    }
}
#[derive(serde::Serialize, serde::Deserialize, Debug, Default)]
#[serde(default)]
pub struct Named { pub a: u32, pub b: String }
macro_rules! read {
    ($out:ident, $ty:ty, $text:expr) => {
        match serde_json::from_str::<$ty>($text) {
            Ok(v) => $out.push_str(&format!("ok {:?} {}\\n", v, serde_json::to_string(&v).unwrap())),
            Err(e) => $out.push_str(&format!("err {}\\n", e)),
        }
    };
}
pub fn report() -> String {
    let mut out = String::new();
    for text in ["[4]", "[]", "[4, \\"x\\"]"] {
        read!(out, T, text);
    }
    for text in ["[]", "[1]", "[1, [5]]", "[1, [5], null]", "[1, [5], \\"p\\", \\"q\\"]"] {
        read!(out, Given, text);
    }
    for text in ["{}", "{\\"b\\": \\"x\\"}"] {
        read!(out, Named, text);
    }
    out
}
`);
  const expected = JSON.parse(run([native(dir)]));
  expect((await compiled(dir)).report()).toBe(expected);
});

// `#[serde(with = "..")]`, `serialize_with` and `deserialize_with` are
// honoured wherever a field is written or read on its own: a newtype
// struct's, a `transparent` one's, a newtype variant's in each tagging, a
// flattened one's, and a newtype variant's own. Each was ignored, the JSON
// its type's, with no error (#1536).
test("serde: a field's own writer and reader are used at each position", async () => {
  const dir = fixture("serde-with-positions");
  writeFileSync(join(dir, "cases.rs"), `use serde::{Deserialize, Deserializer, Serialize, Serializer};
pub mod txt {
    use serde::{Deserialize, Deserializer, Serializer};
    pub fn serialize<S: Serializer>(n: &i32, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&n.to_string())
    }
    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<i32, D::Error> {
        String::deserialize(d)?.parse().map_err(serde::de::Error::custom)
    }
}
pub fn doubled<S: Serializer>(n: &i32, s: S) -> Result<S::Ok, S::Error> {
    s.serialize_i32(n * 2)
}
pub fn halved<'de, D: Deserializer<'de>>(d: D) -> Result<i32, D::Error> {
    Ok(i32::deserialize(d)? / 2)
}
#[derive(Serialize, Deserialize, Debug)]
pub enum OnVariant { #[serde(with = "txt")] A(i32), B(i32) }
#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "t", content = "c")]
pub enum OnVariantAdjacent { #[serde(with = "txt")] A(i32), B(i32) }
#[derive(Serialize, Deserialize, Debug)]
#[serde(untagged)]
pub enum OnVariantUntagged { #[serde(with = "txt")] A(i32), B(bool) }
#[derive(Serialize, Deserialize, Debug)]
pub enum OnNewtypeVariantField { A(#[serde(with = "txt")] i32), B(#[serde(serialize_with = "doubled", deserialize_with = "halved")] i32) }
#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "t", content = "c")]
pub enum OnFieldAdjacent { A(#[serde(with = "txt")] i32) }
#[derive(Serialize, Deserialize, Debug)]
#[serde(untagged)]
pub enum OnFieldUntagged { A(#[serde(with = "txt")] i32) }
#[derive(Serialize, Deserialize, Debug)]
pub struct OnNewtypeStructField(#[serde(with = "txt")] pub i32);
#[derive(Serialize, Deserialize, Debug)]
pub struct OnNewtypeStructFieldApart(#[serde(serialize_with = "doubled", deserialize_with = "halved")] pub i32);
#[derive(Serialize, Deserialize, Debug)]
#[serde(transparent)]
pub struct OnTransparentField { #[serde(with = "txt")] pub v: i32 }
#[derive(Serialize, Debug)]
pub struct Payload { pub source: u32 }
pub fn wire<S: Serializer>(p: &Payload, s: S) -> Result<S::Ok, S::Error> {
    use serde::ser::SerializeStruct;
    let mut st = s.serialize_struct("Payload", 1)?;
    st.serialize_field("wire", &p.source)?;
    st.end()
}
#[derive(Serialize, Debug)]
pub struct Outer { pub n: u32, #[serde(flatten, serialize_with = "wire")] pub payload: Payload }
#[derive(Deserialize)]
struct Wire { wire: u32 }
pub fn unwire<'de, D: Deserializer<'de>>(d: D) -> Result<Payload, D::Error> {
    Ok(Payload { source: Wire::deserialize(d)?.wire })
}
#[derive(Deserialize, Debug)]
pub struct Inner { pub n: u32, #[serde(flatten, deserialize_with = "unwire")] pub payload: Payload }
macro_rules! read {
    ($out:ident, $ty:ty, $text:expr) => {
        $out.push_str(&format!("{:?}\\n", serde_json::from_str::<$ty>($text).map_err(|e| e.to_string())));
    };
}
macro_rules! write {
    ($out:ident, $value:expr) => {
        $out.push_str(&format!("{:?}\\n", serde_json::to_string(&$value).map_err(|e| e.to_string())));
    };
}
pub fn report() -> String {
    let mut out = String::new();
    write!(out, OnVariant::A(7));
    write!(out, OnVariant::B(7));
    read!(out, OnVariant, r#"{"A":"7"}"#);
    read!(out, OnVariant, r#"{"A":7}"#);
    write!(out, OnVariantAdjacent::A(7));
    read!(out, OnVariantAdjacent, r#"{"t":"A","c":"7"}"#);
    write!(out, OnVariantUntagged::A(7));
    read!(out, OnVariantUntagged, r#""7""#);
    read!(out, OnVariantUntagged, r#"true"#);
    write!(out, OnNewtypeVariantField::A(7));
    write!(out, OnNewtypeVariantField::B(7));
    read!(out, OnNewtypeVariantField, r#"{"A":"7"}"#);
    read!(out, OnNewtypeVariantField, r#"{"B":14}"#);
    write!(out, OnFieldAdjacent::A(7));
    read!(out, OnFieldAdjacent, r#"{"t":"A","c":"7"}"#);
    write!(out, OnFieldUntagged::A(7));
    read!(out, OnFieldUntagged, r#""7""#);
    write!(out, OnNewtypeStructField(3));
    read!(out, OnNewtypeStructField, r#""3""#);
    read!(out, OnNewtypeStructField, r#"3"#);
    write!(out, OnNewtypeStructFieldApart(3));
    read!(out, OnNewtypeStructFieldApart, r#"6"#);
    write!(out, OnTransparentField { v: 1 });
    read!(out, OnTransparentField, r#""1""#);
    write!(out, Outer { n: 1, payload: Payload { source: 7 } });
    read!(out, Inner, r#"{"n":1,"wire":7}"#);
    read!(out, Inner, r#"{"n":1,"source":7}"#);
    out
}
`);
  const expected = JSON.parse(run([native(dir)]));
  expect((await compiled(dir)).report()).toBe(expected);
});

// A crate's own `skip_serializing_if` predicate is called as serde_derive
// calls it: once for each field to count what's written,
// `serialize_struct(name, len)`, before any is, then again as each is; once
// only where a field is flattened, `serialize_map(None)`. A predicate that
// answers differently the second time changes the JSON, `{}` where it was
// `{"n":7}` (#1543). Each call is logged, so its order is held too.
test("serde: a skip predicate is called as serde_derive calls it", async () => {
  const dir = fixture("serde-skip-counts");
  writeFileSync(join(dir, "cases.rs"), `use serde::{Serialize, Serializer};
use std::cell::{Cell, RefCell};
thread_local! {
    static ASKED: Cell<bool> = const { Cell::new(false) };
    static LOG: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
}
fn note(s: String) {
    LOG.with(|l| l.borrow_mut().push(s));
}
pub fn once(_: &u32) -> bool {
    ASKED.with(|x| x.replace(true))
}
pub fn logged(n: &u32) -> bool {
    note(format!("skip {n}"));
    *n == 0
}
pub fn shown<S: Serializer>(n: &u32, s: S) -> Result<S::Ok, S::Error> {
    note(format!("write {n}"));
    s.serialize_u32(*n)
}
#[derive(Serialize)]
pub struct Data { #[serde(skip_serializing_if = "once")] pub n: u32 }
#[derive(Serialize)]
pub struct Two {
    #[serde(skip_serializing_if = "logged")] pub a: u32,
    #[serde(serialize_with = "shown")] pub w: u32,
    #[serde(skip_serializing_if = "logged", serialize_with = "shown")] pub b: u32,
    #[serde(skip_serializing_if = "Option::is_none")] pub o: Option<u32>,
}
#[derive(Serialize)]
pub struct Pair(#[serde(skip_serializing_if = "logged")] pub u32, #[serde(skip_serializing_if = "logged")] pub u32);
#[derive(Serialize)]
pub struct Rest { pub r: u32 }
#[derive(Serialize)]
pub struct Flat { #[serde(skip_serializing_if = "logged")] pub a: u32, #[serde(flatten)] pub rest: Rest }
#[derive(Serialize)]
pub struct Inner { #[serde(skip_serializing_if = "logged")] pub i: u32, #[serde(skip_serializing_if = "logged")] pub j: u32 }
#[derive(Serialize)]
pub enum External { S { #[serde(skip_serializing_if = "logged")] a: u32, #[serde(skip_serializing_if = "logged")] b: u32 }, T(#[serde(skip_serializing_if = "logged")] u32, u32) }
#[derive(Serialize)]
#[serde(tag = "t")]
pub enum Internal { S { #[serde(skip_serializing_if = "logged")] a: u32, #[serde(skip_serializing_if = "logged")] b: u32 }, N(Inner) }
#[derive(Serialize)]
#[serde(tag = "t", content = "c")]
pub enum Adjacent { S { #[serde(skip_serializing_if = "logged")] a: u32, #[serde(skip_serializing_if = "logged")] b: u32 }, T(#[serde(skip_serializing_if = "logged")] u32, u32) }
#[derive(Serialize)]
#[serde(untagged)]
pub enum Untagged { S { #[serde(skip_serializing_if = "logged")] a: u32, #[serde(skip_serializing_if = "logged")] b: u32 } }
macro_rules! write {
    ($out:ident, $value:expr) => {
        let json = serde_json::to_string(&$value).map_err(|e| e.to_string());
        let log = LOG.with(|l| l.borrow_mut().drain(..).collect::<Vec<_>>().join(", "));
        $out.push_str(&format!("{:?} [{}]\\n", json, log));
    };
}
pub fn report() -> String {
    let mut out = String::new();
    write!(out, Data { n: 7 });
    write!(out, Two { a: 1, w: 2, b: 0, o: None });
    write!(out, Two { a: 0, w: 2, b: 3, o: Some(4) });
    write!(out, Pair(0, 5));
    write!(out, Flat { a: 1, rest: Rest { r: 2 } });
    write!(out, External::S { a: 1, b: 0 });
    write!(out, External::T(0, 2));
    write!(out, Internal::S { a: 1, b: 2 });
    write!(out, Internal::N(Inner { i: 0, j: 3 }));
    write!(out, Adjacent::S { a: 0, b: 2 });
    write!(out, Adjacent::T(1, 2));
    write!(out, Untagged::S { a: 1, b: 0 });
    out
}
`);
  const expected = JSON.parse(run([native(dir)]));
  expect((await compiled(dir)).report()).toBe(expected);
});
