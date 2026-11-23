// `#[serde(..)]` attributes as serde_derive reads them, at each place it
// takes them: a field's own writer and reader, the container's default, a
// skip predicate, their spans under `-C incremental`, and an internal tag
// beside rust-js's own, `#[rust_js::tag]`. Native serde_json
// is the oracle, the same source and dependency versions as the JavaScript
// (split from serde.test.ts by concept, #1334).
import { beforeAll, expect, test } from "bun:test";
import { writeFileSync } from "node:fs";
import { join } from "node:path";
import { compiled, native } from "./serde-support";
import { buildSerde } from "./serde-crate";
import { buildCompiler, compiler, fixture, run } from "./support";

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

// An enum serde reads by its internal tag that rust-js tags by a property
// of its own, `#[rust_js::tag = "type"]` (ADR 0284): each variant read is
// the object every other variant made is, `{ type: "circle", radius }`, its
// tag its own, a boolean one too, `{ done: true, .. }`, so a `match` on it
// takes its arm. Read as `{ TAG: "circle", .. }`, the reader's own key, a
// circle's area was `NaN`, its arm the square's (#2360).
test("serde: an enum read by its internal tag is the object rust-js tags it by", async () => {
  const dir = fixture("serde-rust-js-tag");
  writeFileSync(join(dir, "cases.rs"), `#[derive(serde::Serialize, serde::Deserialize, Debug)]
#[serde(tag = "type", rename_all = "lowercase")]
#[cfg_attr(rust_js, rust_js::tag = "type")]
pub enum Shape {
    #[cfg_attr(rust_js, rust_js::name = "circle")]
    Circle { radius: f64 },
    #[cfg_attr(rust_js, rust_js::name = "square")]
    Square { side: f64 },
    #[cfg_attr(rust_js, rust_js::name = "point")]
    Point,
}
#[derive(serde::Serialize, serde::Deserialize, Debug)]
#[serde(tag = "done")]
#[cfg_attr(rust_js, rust_js::tag = "done")]
pub enum Task {
    #[serde(rename = "yes")]
    #[cfg_attr(rust_js, rust_js::name = true)]
    Yes { at: u32 },
    #[serde(rename = "no")]
    #[cfg_attr(rust_js, rust_js::name = false)]
    No { reason: String },
}
fn area(shape: &Shape) -> f64 {
    match shape {
        Shape::Circle { radius } => 3.0 * radius * radius,
        Shape::Square { side } => side * side,
        Shape::Point => 0.0,
    }
}
fn when(task: &Task) -> String {
    match task {
        Task::Yes { at } => format!("done at {at}"),
        Task::No { reason } => format!("not done: {reason}"),
    }
}
pub fn report() -> String {
    let mut out = String::new();
    for text in [r#"{"type":"circle","radius":2}"#, r#"{"type":"square","side":3}"#, r#"{"type":"point"}"#] {
        match serde_json::from_str::<Shape>(text) {
            Ok(shape) => out.push_str(&format!("{} {:?} {}\\n", area(&shape), shape, serde_json::to_string(&shape).unwrap())),
            Err(e) => out.push_str(&format!("err {e}\\n")),
        }
    }
    for text in [r#"{"done":"yes","at":5}"#, r#"{"done":"no","reason":"later"}"#] {
        match serde_json::from_str::<Task>(text) {
            Ok(task) => out.push_str(&format!("{} {:?} {}\\n", when(&task), task, serde_json::to_string(&task).unwrap())),
            Err(e) => out.push_str(&format!("err {e}\\n")),
        }
    }
    out
}
`);
  const expected = JSON.parse(run([native(dir)]));
  expect((await compiled(dir)).report()).toBe(expected);
});

// A field `#[rust_js::flatten]` flattens is the struct's own keys (ADR
// 0204), so the struct serde reads is made as every other maker makes it,
// `{ ...base, name }`, not nested, `{ base, name }`, whose `user.base.id`,
// read as `user.id`, was `undefined` (#2364). And a keyed enum flattened
// into one is its variant's keys alone, no tag, as a literal of it is
// (ADR 1071, #2360).
test("serde: a struct with a flattened field is read flat, its keyed enum's variant without a tag", async () => {
  const dir = fixture("serde-rust-js-flatten");
  writeFileSync(join(dir, "cases.rs"), `#[derive(serde::Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Base {
    pub id: String,
    pub created_at: String,
}
#[derive(serde::Deserialize, Debug)]
pub struct User {
    #[serde(flatten)]
    #[cfg_attr(rust_js, rust_js::flatten)]
    pub base: Base,
    pub name: String,
}
#[derive(serde::Deserialize, Debug)]
pub enum Kind {
    Signal { current: Option<u32> },
    Computed { value: u32, dirty: bool },
}
#[derive(serde::Deserialize, Debug)]
pub struct Node {
    pub flags: u32,
    #[serde(flatten)]
    #[cfg_attr(rust_js, rust_js::flatten)]
    pub kind: Kind,
}
fn label(base: &Base) -> String {
    format!("{}, created {}", base.id, base.created_at)
}
fn value(node: &Node) -> String {
    match &node.kind {
        Kind::Signal { current } => format!("signal {current:?}"),
        Kind::Computed { value, dirty } => format!("computed {value} {dirty}"),
    }
}
pub fn report() -> String {
    let mut out = String::new();
    match serde_json::from_str::<User>(r#"{"id":"u1","createdAt":"2026-11-23","name":"Ada"}"#) {
        Ok(user) => out.push_str(&format!("{} ({}) {:?}\\n", user.name, label(&user.base), user)),
        Err(e) => out.push_str(&format!("err {e}\\n")),
    }
    for text in [r#"{"flags":1,"Signal":{"current":3}}"#, r#"{"flags":2,"Computed":{"value":4,"dirty":true}}"#] {
        match serde_json::from_str::<Node>(text) {
            Ok(node) => out.push_str(&format!("{} {} {:?}\\n", node.flags, value(&node), node)),
            Err(e) => out.push_str(&format!("err {e}\\n")),
        }
    }
    out
}
pub fn user(text: &str) -> Option<User> {
    serde_json::from_str(text).ok()
}
pub fn node(text: &str) -> Option<Node> {
    serde_json::from_str(text).ok()
}
`);
  const expected = JSON.parse(run([native(dir)]));
  const cases = await compiled(dir);
  expect(cases.report()).toBe(expected);
  // What JS sees of each: its keys, a spread's and `JSON.stringify`'s,
  // the flattened struct's own and the variant's, no tag beside them.
  expect(Object.keys(cases.user('{"id":"u1","createdAt":"2026-11-23","name":"Ada"}'))).toEqual(["id", "createdAt", "name"]);
  expect(Object.keys(cases.node('{"flags":1,"Signal":{"current":3}}'))).toEqual(["flags", "current"]);
  expect(Object.keys(cases.node('{"flags":2,"Computed":{"value":4,"dirty":true}}'))).toEqual(["flags", "value", "dirty"]);
});

// A field's own reader is never called for a field that isn't there:
// serde's derive says `missing field` itself, so a field read by
// `Option::deserialize`, serde's usual "there, but maybe null", is required
// (#2382); its default comes first. Its own reader or writer that is serde's
// impl of its type, `Option::deserialize` or `Option::serialize`, reads or
// writes it as its type (#2383). An adjacently tagged newtype variant's
// reader, unlike a field's, is given its missing content, as serde gives it.
test("serde: a field of its own reader that isn't there is missing, as serde's derive says", async () => {
  const dir = fixture("serde-field-codec-missing");
  writeFileSync(join(dir, "cases.rs"), String.raw`
use serde::{Deserialize, Deserializer, Serialize, Serializer};

fn nullable<'de, D: Deserializer<'de>>(d: D) -> Result<Option<String>, D::Error> {
    Option::deserialize(d)
}

fn doubled<'de, D: Deserializer<'de>>(d: D) -> Result<u32, D::Error> {
    Ok(u32::deserialize(d)? * 2)
}

fn shout<S: Serializer>(value: &Option<String>, s: S) -> Result<S::Ok, S::Error> {
    s.serialize_str(&value.as_deref().unwrap_or("").to_uppercase())
}

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

/// A field that must be there, but may be null: its own function's.
#[derive(Deserialize, Debug)]
pub struct Own {
    #[serde(deserialize_with = "nullable")]
    pub bio: Option<String>,
}

/// The same by serde's own Option::deserialize, its usual idiom.
#[derive(Deserialize, Debug)]
pub struct Idiom {
    #[serde(deserialize_with = "Option::deserialize")]
    pub bio: Option<String>,
    #[serde(deserialize_with = "Vec::deserialize")]
    pub tags: Vec<u32>,
}

/// A default beside its own reader: missing is the default.
#[derive(Deserialize, Debug)]
pub struct Defaulted {
    #[serde(default, deserialize_with = "doubled")]
    pub n: u32,
    #[serde(default, deserialize_with = "Option::deserialize")]
    pub bio: Option<String>,
}

/// A module's pair, with.
#[derive(Serialize, Deserialize, Debug)]
pub struct Price {
    #[serde(with = "cents")]
    pub amount: u64,
}

/// A writer of its own, skipped where there's nothing, and serde's own
/// Option::serialize.
#[derive(Serialize, Debug)]
pub struct Shown {
    #[serde(serialize_with = "shout", skip_serializing_if = "Option::is_none")]
    pub nick: Option<String>,
    #[serde(serialize_with = "Option::serialize")]
    pub bio: Option<String>,
}

/// An adjacently tagged newtype variant of its own reader, its content left out.
#[derive(Deserialize, Debug)]
#[serde(tag = "t", content = "c")]
pub enum Tagged {
    A(#[serde(deserialize_with = "nullable")] Option<String>),
}

fn read<T: for<'de> Deserialize<'de> + std::fmt::Debug>(text: &str) -> String {
    format!("{:?}", serde_json::from_str::<T>(text).map_err(|e| e.to_string()))
}

pub fn report() -> Vec<String> {
    let mut lines = Vec::new();
    for text in ["{}", r#"{"bio":null}"#, r#"{"bio":"x"}"#] {
        lines.push(read::<Own>(text));
    }
    for text in ["{}", r#"{"tags":[1]}"#, r#"{"bio":null}"#, r#"{"bio":null,"tags":[1]}"#, r#"{"bio":"x","tags":[]}"#, r#"{"bio":1,"tags":[]}"#] {
        lines.push(read::<Idiom>(text));
    }
    for text in ["{}", r#"{"n":2}"#, r#"{"bio":null}"#, r#"{"bio":"x"}"#] {
        lines.push(read::<Defaulted>(text));
    }
    for text in ["{}", r#"{"amount":"1.50"}"#, r#"{"amount":"x"}"#] {
        lines.push(read::<Price>(text));
    }
    lines.push(serde_json::to_string(&Price { amount: 150 }).unwrap());
    lines.push(serde_json::to_string(&Shown { nick: None, bio: None }).unwrap());
    lines.push(serde_json::to_string(&Shown { nick: Some("ada".into()), bio: Some("hi".into()) }).unwrap());
    for text in [r#"{"t":"A"}"#, r#"{"t":"A","c":null}"#, r#"{"t":"A","c":"x"}"#] {
        lines.push(read::<Tagged>(text));
    }
    lines
}
`);
  const expected = run([native(dir)]);
  const { report } = await compiled(dir);
  expect(JSON.stringify(report())).toBe(expected.trim());
}, 300_000);

// A `#[rust_js::untagged]` enum serde reads and writes, by each of serde's
// representations of it: its value is its payload, TypeScript's `number |
// string`, as every other maker of one makes it, read and written as
// native serde does (ADR 0214, #2388). zod's `z.union`, coming-from-zod's
// chapter 8, is `#[serde(untagged)]` beside it.
test("serde: an untagged enum of rust-js's is its payload, read and written by each of serde's representations", async () => {
  const dir = fixture("serde-rust-js-untagged");
  writeFileSync(join(dir, "cases.rs"), String.raw`
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, PartialEq)]
#[serde(untagged)]
#[cfg_attr(rust_js, rust_js::untagged)]
pub enum Id {
    Number(u32),
    Text(String),
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct Email {
    #[cfg_attr(rust_js, rust_js::test(key))]
    pub email: String,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct Phone {
    #[cfg_attr(rust_js, rust_js::test(key))]
    pub phone: String,
    pub ext: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
#[serde(untagged)]
#[cfg_attr(rust_js, rust_js::untagged)]
pub enum Contact {
    Email(Email),
    Phone(Phone),
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
#[serde(untagged)]
#[cfg_attr(rust_js, rust_js::untagged)]
pub enum Cell {
    Number(f64),
    Text(String),
    #[cfg_attr(rust_js, rust_js::null)]
    Empty,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
#[cfg_attr(rust_js, rust_js::untagged)]
pub enum External {
    Number(u32),
    Text(String),
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
#[serde(tag = "kind")]
#[cfg_attr(rust_js, rust_js::untagged)]
pub enum Internal {
    Email(Email),
    Phone(Phone),
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
#[serde(tag = "t", content = "c")]
#[cfg_attr(rust_js, rust_js::untagged)]
pub enum Adjacent {
    Number(u32),
    Text(String),
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct Row {
    pub id: Id,
    pub contact: Option<Contact>,
    pub cells: Vec<Cell>,
}

fn label(id: &Id) -> String {
    match id {
        Id::Number(n) => format!("#{n}"),
        Id::Text(t) => t.clone(),
    }
}

fn reach(contact: &Contact) -> String {
    match contact {
        Contact::Email(e) => format!("write {}", e.email),
        Contact::Phone(p) => format!("call {}", p.phone),
    }
}

fn round<T: Serialize + for<'de> Deserialize<'de> + std::fmt::Debug>(text: &str) -> String {
    match serde_json::from_str::<T>(text) {
        Ok(value) => format!("{value:?} -> {}", serde_json::to_string(&value).unwrap()),
        Err(error) => format!("Err({error})"),
    }
}

pub fn report() -> Vec<String> {
    let mut lines = Vec::new();
    for text in ["42", r#""a7""#, "-1", "1.5", "true", "null"] {
        lines.push(round::<Id>(text));
    }
    for text in [r#"{"email":"a@b.c"}"#, r#"{"phone":"555","ext":"9"}"#, r#"{"email":"a@b.c","phone":"555"}"#, r#"{"fax":"1"}"#, "[]"] {
        lines.push(round::<Contact>(text));
    }
    for text in ["1.5", r#""x""#, "null", "[1]"] {
        lines.push(round::<Cell>(text));
    }
    for text in [r#"{"Number":7}"#, r#"{"Text":"x"}"#, r#"{"Other":1}"#, "7"] {
        lines.push(round::<External>(text));
    }
    for text in [r#"{"kind":"Email","email":"a@b.c"}"#, r#"{"kind":"Phone","phone":"555"}"#, r#"{"kind":"Fax"}"#] {
        lines.push(round::<Internal>(text));
    }
    for text in [r#"{"t":"Number","c":7}"#, r#"{"t":"Text","c":"x"}"#, r#"{"t":"Number","c":"x"}"#] {
        lines.push(round::<Adjacent>(text));
    }
    for text in [r#"{"id":"r1","contact":{"phone":"555"},"cells":[1,"two",null]}"#, r#"{"id":3,"contact":null,"cells":[]}"#, r#"{"id":{},"contact":null,"cells":[]}"#] {
        lines.push(round::<Row>(text));
    }
    if let Ok(row) = serde_json::from_str::<Row>(r#"{"id":12,"contact":{"email":"a@b.c"},"cells":[]}"#) {
        lines.push(format!("{} / {}", label(&row.id), row.contact.as_ref().map_or("none".to_string(), reach)));
    }
    lines
}

pub fn parse_row(text: &str) -> Option<Row> {
    serde_json::from_str(text).ok()
}
`);
  const expected = run([native(dir)]);
  const { report, parseRow } = await compiled(dir);
  expect(JSON.stringify(report())).toBe(expected.trim());
  // Each value its payload: no `TAG`, a fieldless one `null`.
  expect(parseRow('{"id":"r1","contact":{"phone":"555"},"cells":[1,"two",null]}')).toEqual({
    id: "r1",
    contact: { phone: "555", ext: undefined },
    cells: [1, "two", null],
  });
  expect(parseRow('{"id":7,"contact":{"email":"a@b.c"},"cells":[]}')).toEqual({ id: 7, contact: { email: "a@b.c" }, cells: [] });
}, 300_000);
