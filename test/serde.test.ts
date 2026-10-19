// Differential regressions: native serde_json is the oracle, using exactly
// the same Rust source and dependency versions as the generated JavaScript.
import { beforeAll, expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { buildCompiler, buildSerde, compiler, contentDirectory, fixture, nativeBinary, run, writeWhole } from "./support";

beforeAll(() => {
  buildCompiler();
  buildSerde("rlib");
}, 600_000);

/** `dir/cases.rs` with serde_json natively, the oracle: printing its
 * `report()`. Built beside a copy named by what it says, so the same cases
 * are one kept binary (`nativeBinary`). */
function native(dir: string): string {
  const cases = readFileSync(join(dir, "cases.rs"), "utf8");
  const beside = contentDirectory(cases);
  writeWhole(join(beside, "cases.rs"), cases);
  const built = nativeBinary(`include!("cases.rs");
fn main() { println!("{}", serde_json::to_string(&report()).unwrap()); }
`, beside, ["--edition=2024", "-Awarnings", ...buildSerde("rlib")]);
  if ("error" in built) throw new Error(built.error);
  return built.binary;
}

/** The case's JS, built and loaded. */
async function compiled(dir: string) {
  run([compiler, join(dir, "cases.rs"), "-o", join(dir, "cases.js"), "--", ...buildSerde()]);
  return await import(join(dir, "cases.js"));
}

const cases = [
  {
    // Each variant tested as a pattern tests it (ADR 0284): one of an enum
    // rust-js tags is an object, `{ kind: "On" }`, not its name.
    name: "an enum rust-js tags as a discriminated union serializes as serde has it",
    definitions: `#[cfg_attr(rust_js, rust_js::tag = "kind")]
#[derive(serde::Serialize)]
pub enum Light { On, Off, Dim { level: u8 } }`,
    value: '(Light::On, Light::Off, Light::Dim { level: 3 })',
  },
  {
    name: "a serde_json error given by ? to a Box<dyn Error> shows as serde_json's",
    definitions: `fn load(text: &str) -> Result<u64, Box<dyn std::error::Error>> {
    let value: serde_json::Value = serde_json::from_str(text)?;
    Ok(value["n"].as_u64().unwrap_or(0))
}
pub fn show(text: &str) -> String {
    match load(text) {
        Ok(n) => format!("ok {n}"),
        Err(e) => format!("err {e} / {e:?}"),
    }
}`,
    value: '(show("{\\"n\\": 3}"), show("{"), show("[1,]"), show("tru"))',
  },
  {
    name: "standard skip predicates recognize optional and empty fields",
    definitions: `#[derive(serde::Serialize)]
pub struct Skips {
    #[serde(skip_serializing_if = "Option::is_some")] pub present: Option<u32>,
    #[serde(skip_serializing_if = "String::is_empty")] pub text: String,
    #[serde(skip_serializing_if = "Vec::is_empty")] pub items: Vec<u32>,
}`,
    value: '(Skips { present: Some(7), text: String::new(), items: Vec::new() }, Skips { present: None, text: "kept".into(), items: vec![2] })',
  },
  {
    name: "nested Value conversions preserve vectors, optional values and nulls",
    definitions: "",
    value: `serde_json::Value::from(vec![Some(vec![1_i32, -2]), None, Some(Vec::<i32>::new())])`,
  },
  {
    name: "Value comparisons preserve operand order in both directions",
    definitions: `pub fn json(log: &mut String) -> serde_json::Value {
    log.push_str("v");
    serde_json::Value::from("hello")
}
pub fn text(log: &mut String) -> &'static str {
    log.push_str("s");
    "hello"
}`,
    value: `{
        let mut log = String::new();
        let a = json(&mut log) == text(&mut log);
        let b = text(&mut log) == json(&mut log);
        let c = json(&mut log) != text(&mut log);
        let d = text(&mut log) != json(&mut log);
        (a, b, c, d, log)
    }`,
  },
  {
    name: "renaming rules of variants, and a struct's tag, agree",
    definitions: `#[derive(serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Event { TextMessage, ImageSent }
#[derive(serde::Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum Level { LowPriority }
#[derive(serde::Serialize)]
#[serde(tag = "kind")]
pub struct Tagged { pub n: u32 }`,
    value: "(Event::TextMessage, Event::ImageSent, Level::LowPriority, Tagged { n: 1 })",
  },
  {
    name: "ordinary renamed structs and optional fields agree (control)",
    definitions: `#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Record {
    pub record_id: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}`,
    value: "Record { record_id: 7, note: None }",
  },
  {
    name: "tuple struct skips do not disclose omitted fields",
    definitions: `#[derive(serde::Serialize)]
pub struct Credentials(pub u32, #[serde(skip_serializing)] pub String, pub u32);`,
    value: 'Credentials(1, "secret".into(), 2)',
  },
  {
    name: "tuple variant skips do not disclose omitted fields",
    definitions: `#[derive(serde::Serialize)]
pub enum Message { Credentials(u32, #[serde(skip)] String, u32) }`,
    value: 'Message::Credentials(1, "secret".into(), 2)',
  },
  {
    name: "conditional skips apply to tuple fields",
    definitions: `#[derive(serde::Serialize)]
pub struct Values(#[serde(skip_serializing_if = "Option::is_none")] pub Option<u32>, pub u32);`,
    value: 'Values(None, 2)',
  },
  {
    name: "an untagged variant overrides its enum representation",
    definitions: `#[derive(serde::Serialize)]
pub enum Message { Named(u32), #[serde(untagged)] Other(u32) }`,
    value: 'Message::Other(7)',
  },
  {
    name: "a tagged struct includes its renamed type tag",
    definitions: `#[derive(serde::Serialize)]
#[serde(tag = "kind", rename = "record")]
pub struct Record { pub id: u32 }`,
    value: 'Record { id: 7 }',
  },
  {
    name: "internal tags respect a transparent newtype payload",
    definitions: `#[derive(serde::Serialize)]
pub struct Details { pub id: u32 }
#[derive(serde::Serialize)]
#[serde(transparent)]
pub struct Wrapper { pub value: Details }
#[derive(serde::Serialize)]
#[serde(tag = "kind")]
pub enum Message { Data(Wrapper) }`,
    value: 'Message::Data(Wrapper { value: Details { id: 7 } })',
  },
  {
    name: "skip predicates resolve in their declaring module",
    definitions: `pub mod first {
    pub fn skip(_: &u32) -> bool { true }
    #[derive(serde::Serialize)]
    pub struct Record { #[serde(skip_serializing_if = "skip")] pub id: u32 }
}
pub mod second {
    pub fn skip(_: &u32) -> bool { false }
    #[derive(serde::Serialize)]
    pub struct Record { #[serde(skip_serializing_if = "skip")] pub id: u32 }
}`,
    value: '(first::Record { id: 1 }, second::Record { id: 2 })',
  },
  {
    name: "tuple skips preserve included values and empty arrays",
    definitions: `#[derive(serde::Serialize)]
pub struct Hidden(#[serde(skip)] pub String, #[serde(skip)] pub u32);
#[derive(serde::Serialize)]
pub enum Message {
    Values(#[serde(skip_serializing_if = "Option::is_none")] Option<u32>, u32),
}`,
    value: '(Hidden("secret".into(), 1), Message::Values(None, 2), Message::Values(Some(3), 4))',
  },
  {
    name: "skip predicates honor imports and fully qualified paths",
    definitions: `pub mod helpers { pub fn skip(_: &u32) -> bool { true } }
use helpers::skip as omit;
#[derive(serde::Serialize)]
pub struct Record {
    #[serde(skip_serializing_if = "omit")] pub hidden: u32,
    #[serde(skip_serializing_if = "crate::helpers::skip")] pub qualified: u32,
    #[serde(skip_serializing_if = "std::option::Option::is_none")] pub absent: Option<u32>,
}`,
    value: 'Record { hidden: 1, qualified: 2, absent: None }',
  },
  {
    name: "a user predicate named Option::is_none is not a standard-library intrinsic",
    definitions: `pub struct Option;
impl Option { pub fn is_none(value: &std::option::Option<u32>) -> bool { value.is_some() } }
#[derive(serde::Serialize)]
pub struct Record { #[serde(skip_serializing_if = "Option::is_none")] pub hidden: std::option::Option<u32> }`,
    value: '(Record { hidden: None }, Record { hidden: Some(1) })',
  },
  {
    name: "internal tags unwrap nested transparent and newtype payloads",
    definitions: `#[derive(serde::Serialize)]
#[serde(tag = "type", rename = "details")]
pub struct Details { pub id: u32 }
#[derive(serde::Serialize)]
pub struct Newtype(pub Details);
#[derive(serde::Serialize)]
#[serde(transparent)]
pub struct Wrapper { #[serde(skip)] pub hidden: u32, pub value: Newtype }
#[derive(serde::Serialize)]
#[serde(tag = "kind")]
pub enum Message { Data(Wrapper) }`,
    value: 'Message::Data(Wrapper { hidden: 9, value: Newtype(Details { id: 7 }) })',
  },
] as const;

for (const { name, definitions, value } of cases) {
  test(`serde: ${name}`, async () => {
    const dir = fixture("serde-regression");
    writeFileSync(join(dir, "cases.rs"), `${definitions}
pub fn report() -> String {
    let value = ${value};
    format!("{}\\n{}", serde_json::to_string(&value).unwrap(), serde_json::to_string_pretty(&value).unwrap())
}
`);
    const expected = JSON.parse(run([native(dir)]));
    expect((await compiled(dir)).report()).toBe(expected);
  });
}

// An `f32` in JSON (ADR 0122): written with its own shortest digits, fixed
// from 1e-6 to 1e12, as serde_json's zmij writes them; read as serde reads
// one, any number `as f32`; and a `Value` of one is its `f64`.
test("serde: an f32 is written and read as serde_json does", async () => {
  const dir = fixture("serde-f32");
  writeFileSync(join(dir, "cases.rs"), `#[derive(Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Sample { pub x: f32, pub ys: Vec<f32>, pub z: Option<f32> }
pub fn report() -> String {
    let mut out = String::new();
    for v in [0.1f32, 1.5, -2.75, 1e-6, 9.9e-7, 1e-7, 123456.7, 1e12, 9.99e12, 1e13, 3.4028235e38, f32::MIN_POSITIVE, 1e-45, 16777217.0, f32::NAN, f32::INFINITY, -0.0, 0.0] {
        out.push_str(&serde_json::to_string(&v).unwrap());
        out.push('\\n');
    }
    let s = Sample { x: 0.1, ys: vec![1e-7, 2.5e13], z: Some(-3.2) };
    out.push_str(&serde_json::to_string(&s).unwrap());
    out.push('\\n');
    out.push_str(&serde_json::to_string_pretty(&s).unwrap());
    out.push('\\n');
    for text in ["0.1", "16777217", "18446744073709551615", "-9223372036854775808", "1152921573326323713", "1e39", "3.4028235e38", "\\"x\\"", "true", "null"] {
        match serde_json::from_str::<f32>(text) {
            Ok(v) => out.push_str(&format!("{:?}\\n", v)),
            Err(e) => out.push_str(&format!("{}\\n", e)),
        }
    }
    let back: Sample = serde_json::from_str(&serde_json::to_string(&s).unwrap()).unwrap();
    out.push_str(&format!("{:?} {}\\n", back, back == s));
    let v = serde_json::to_value(&0.1f32).unwrap();
    out.push_str(&format!("{} {} {} {}\\n", v, serde_json::json!(0.1f32), v == 0.1f32, serde_json::json!(16777217) == 16777216.0f32));
    out
}
`);
  const expected = JSON.parse(run([native(dir)]));
  expect((await compiled(dir)).report()).toBe(expected);
});

// A string whose text begins with U+FEFF keeps it, as serde_json reads
// every byte: `TextDecoder` drops one by default, as a byte order mark
// (found by the boundary matrices, ADR 0182).
test("serde: a string beginning with U+FEFF keeps it", async () => {
  const dir = fixture("serde-bom");
  writeFileSync(join(dir, "cases.rs"), `pub fn report() -> String {
    let mut out = String::new();
    for text in ["\\"\\u{feff}x\\"", "\\"\\u{feff}\\"", "[\\"a\\", \\"\\u{feff}b\\u{feff}\\"]"] {
        out.push_str(&format!("{:?} {:?}\\n", serde_json::from_str::<String>(text).ok(), serde_json::from_str::<Vec<String>>(text).ok()));
    }
    out
}
`);
  const expected = JSON.parse(run([native(dir)]));
  expect((await compiled(dir)).report()).toBe(expected);
});

// A `Value`'s reads, which only read, written where they're used, and an
// `Option` of one shown held first, `const value = ..`.
test("serde: a Value's reads are written in place", async () => {
  const dir = fixture("serde-reads");
  writeFileSync(join(dir, "cases.rs"), `use serde_json::json;
pub fn report() -> String {
    let v = json!({"name": "a", "list": [1, 2]});
    format!("{} {} {:?}", v["name"], v["list"][1], v["list"].get(0))
}
`);
  const expected = JSON.parse(run([native(dir)]));
  expect((await compiled(dir)).report()).toBe(expected);
  for (const out of ["cases.js"]) {
    const js = readFileSync(join(dir, out), "utf8");
    expect(js).toContain('${$jsonValueText($jsonIndex(v, "name"), false)} ${$jsonValueText($jsonIndex($jsonIndex(v, "list"), 1), false)}');
    expect(js).not.toContain("))($jsonGet(");
  }
});

// A set of structs, an array in JSON, found by value as it's read (ADR
// 0121): two equal items are one, as serde_json's `HashSet` has them.
test("serde: a set of structs read from JSON has each value once", async () => {
  const dir = fixture("serde-value-set");
  writeFileSync(join(dir, "cases.rs"), `#[derive(Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct Point { pub x: i32, pub y: i32 }
pub fn report() -> String {
    let set: std::collections::HashSet<Point> =
        serde_json::from_str(r#"[{"x":1,"y":2},{"y":2,"x":1},{"x":3,"y":4}]"#).unwrap();
    let one: std::collections::HashSet<Point> = serde_json::from_str(r#"[{"x":5,"y":6},{"x":5,"y":6}]"#).unwrap();
    format!(
        "{} {} {} {}",
        set.len(),
        set.contains(&Point { x: 1, y: 2 }),
        set.contains(&Point { x: 2, y: 1 }),
        serde_json::to_string(&one).unwrap()
    )
}
`);
  const expected = JSON.parse(run([native(dir)]));
  expect((await compiled(dir)).report()).toBe(expected);
});

// A generic type's derived codec, of a value with a destructor: rust-js's,
// as serde's derive is (ADR 0077), though another crate's derive writes code
// of the crate's own (ADR 0186).
test("serde: a generic type's derived codec of a value with a destructor", async () => {
  const dir = fixture("serde-generic-drop");
  writeFileSync(join(dir, "cases.rs"), `#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct Page<T> { pub items: Vec<T>, pub total: u32 }
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct Item { pub id: u32 }
impl Drop for Item {
    fn drop(&mut self) {}
}
pub fn report() -> String {
    let page: Page<Item> = serde_json::from_str(r#"{"items":[{"id":1},{"id":2}],"total":2}"#).unwrap();
    format!("{} {}", serde_json::to_string(&page).unwrap(), page.items.len())
}
`);
  const expected = JSON.parse(run([native(dir)]));
  expect((await compiled(dir)).report()).toBe(expected);
});

// Numbers chosen at random, the same ones each run: an f64 from any 64 bits,
// written with `to_string`, and number texts of every form, read with
// `from_str` as an `f64` and an `i32`. serde_json's float parsing isn't
// correctly rounded (ADR 0078), and its digits are its own (ADR 0077).
test("serde: numbers are written and read as serde_json does", async () => {
  let seed = 20260927n;
  const next = () => (seed = (seed * 6364136223846793005n + 1442695040888963407n) & 0xffffffffffffffffn);
  const below = (n: number) => Number(next() >> 33n) % n;
  const digits = (n: number) => String(1 + below(9)) + Array.from({ length: n - 1 }, () => below(10)).join("");
  const bits = new DataView(new ArrayBuffer(8));
  const floats: string[] = [];
  while (floats.length < 2000) {
    bits.setBigUint64(0, next());
    const x = bits.getFloat64(0);
    if (Number.isFinite(x)) floats.push(x.toPrecision(17).replace("e+", "e"));
  }
  const texts = Array.from({ length: 2000 }, () => {
    const sign = below(3) === 0 ? "-" : "";
    const forms = [
      () => digits(1 + below(25)),
      () => `${digits(1 + below(12))}.${digits(1 + below(20))}`,
      () => `${digits(1 + below(18))}e${below(2) ? "-" : ""}${below(330)}`,
      () => `0.0000${digits(1 + below(20))}e-${below(300)}`,
      () => `${digits(15 + below(10))}.${digits(10)}E+${below(290)}`,
    ];
    return sign + forms[below(forms.length)]();
  });
  const dir = fixture("serde-numbers");
  writeFileSync(join(dir, "cases.rs"), `pub fn report() -> String {
    let mut out = String::new();
    for x in [${floats.map((x) => `${/[.e]/.test(x) ? x : `${x}.0`}_f64`).join(", ")}] {
        out.push_str(&serde_json::to_string(&x).unwrap());
        out.push('\\n');
    }
    for text in [${texts.map((t) => JSON.stringify(t)).join(", ")}] {
        match serde_json::from_str::<f64>(text) {
            Ok(v) => out.push_str(&format!("{:?}\\n", v)),
            Err(e) => out.push_str(&format!("{}\\n", e)),
        }
        match serde_json::from_str::<i32>(text) {
            Ok(v) => out.push_str(&format!("{}\\n", v)),
            Err(e) => out.push_str(&format!("{}\\n", e)),
        }
    }
    out
}
`);
  const expected = JSON.parse(run([native(dir)]));
  expect((await compiled(dir)).report()).toBe(expected);
}, 120_000);

// A hand-written `Serialize` (ADR 0459): its serializer is the JSON writer,
// a struct, a sequence or a map it begins the writer too, a value it hands
// on written by its own writer, and an error it makes the codec's.
test("serde: a hand-written Serialize writes as serde_json does", async () => {
  const dir = fixture("serde-hand-written-serialize");
  writeFileSync(join(dir, "cases.rs"), String.raw`
use serde::ser::{SerializeMap, SerializeSeq, SerializeStruct};
use serde::{Serialize, Serializer};
use std::fmt;

pub struct Cents(pub u64);

impl Serialize for Cents {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&format!("{}.{:02}", self.0 / 100, self.0 % 100))
    }
}

pub struct Point {
    pub x: i32,
    pub y: f64,
    pub tag: Option<String>,
}

impl Serialize for Point {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let mut state = s.serialize_struct("Point", 3)?;
        state.serialize_field("x", &self.x)?;
        state.serialize_field("y", &self.y)?;
        if let Some(tag) = &self.tag {
            state.serialize_field("tag", tag)?;
        } else {
            state.skip_field("tag")?;
        }
        state.end()
    }
}

pub struct Bag(pub Vec<Cents>);

impl Serialize for Bag {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let mut seq = s.serialize_seq(Some(self.0.len()))?;
        for c in &self.0 {
            seq.serialize_element(c)?;
        }
        seq.end()
    }
}

pub struct Counts(pub Vec<(u8, bool)>);

impl Serialize for Counts {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let mut map = s.serialize_map(None)?;
        for (k, v) in &self.0 {
            map.serialize_entry(k, v)?;
        }
        map.end()
    }
}

pub enum Shape {
    Dot,
    Circle(f64),
}

impl Serialize for Shape {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        match self {
            Shape::Dot => s.serialize_unit_variant("Shape", 0, "Dot"),
            Shape::Circle(r) => s.serialize_newtype_variant("Shape", 1, "Circle", r),
        }
    }
}

pub struct Shown(pub u32);

impl fmt::Display for Shown {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "#{}", self.0)
    }
}

impl Serialize for Shown {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.collect_str(self)
    }
}

pub struct Ratio(pub f32);

impl Serialize for Ratio {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_f32(self.0)
    }
}

pub struct Labeled<T>(pub &'static str, pub T);

impl<T: Serialize> Serialize for Labeled<T> {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        if self.0.is_empty() {
            return Err(serde::ser::Error::custom("no label"));
        }
        self.1.serialize(s)
    }
}

pub fn report() -> String {
    let point = Point { x: -3, y: 1.5, tag: None };
    let tagged = Point { x: 1, y: 2.0, tag: Some("t".into()) };
    format!(
        "{} {} {} {} {} {} {} {} {} {} {}",
        serde_json::to_string(&Cents(1234)).unwrap(),
        serde_json::to_string(&point).unwrap(),
        serde_json::to_string_pretty(&tagged).unwrap(),
        serde_json::to_string(&Bag(vec![Cents(5), Cents(250)])).unwrap(),
        serde_json::to_string(&Counts(vec![(1, true), (2, false)])).unwrap(),
        serde_json::to_string(&vec![Shape::Dot, Shape::Circle(0.5)]).unwrap(),
        serde_json::to_string(&Shown(7)).unwrap(),
        serde_json::to_string(&Labeled("n", 9u8)).unwrap(),
        serde_json::to_string(&Labeled("c", Cents(1))).unwrap(),
        serde_json::to_string(&Labeled("", 1u8)).unwrap_err(),
        serde_json::to_string(&Ratio(0.1)).unwrap(),
    )
}
`);
  const expected = JSON.parse(run([native(dir)]));
  expect((await compiled(dir)).report()).toBe(expected);
});

// A hand-written `Deserialize` (ADR 0459): what it reads through another's,
// a `Result`, and an error it makes, at the place serde_json gives it.
test("serde: a hand-written Deserialize reads as serde_json does", async () => {
  const dir = fixture("serde-hand-written-deserialize");
  writeFileSync(join(dir, "cases.rs"), String.raw`
use serde::{Deserialize, Deserializer};

#[derive(Debug, PartialEq)]
pub struct Cents(pub u64);

impl<'de> Deserialize<'de> for Cents {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let text = String::deserialize(d)?;
        let (whole, part) = text.split_once('.').ok_or_else(|| serde::de::Error::custom("no point"))?;
        let whole: u64 = whole.parse().map_err(serde::de::Error::custom)?;
        let part: u64 = part.parse().map_err(serde::de::Error::custom)?;
        Ok(Cents(whole * 100 + part))
    }
}

#[derive(Debug, serde::Deserialize)]
pub struct Order {
    pub price: Cents,
    pub qty: u32,
}

pub fn report() -> String {
    let show = |text: &str| match serde_json::from_str::<Cents>(text) { Ok(l) => format!("{l:?}"), Err(e) => e.to_string() };
    let order = |text: &str| match serde_json::from_str::<Order>(text) { Ok(o) => format!("{o:?}"), Err(e) => e.to_string() };
    format!("{} {} {} {} {} {} {}", show("\"12.34\""), show("3"), show("\"1.x\""), show("\"12\""), show("true"),
        order("{\"price\": \"2.50\", \"qty\": 3}"), order("{\"price\": \"2\", \"qty\": 3}"))
}
`);
  const expected = JSON.parse(run([native(dir)]));
  expect((await compiled(dir)).report()).toBe(expected);
  // Its impl's function, once: not a derived codec of it beside.
  expect(readFileSync(join(dir, "cases.js"), "utf8").match(/function centsDeserialize_deserialize\b/g)).toHaveLength(1);
});

// A hand-written `Visitor` (ADR 0459): what's there given to its `visit_*` of
// that kind, a number by its kind, a borrowed string or a copied one, and
// serde's errors of `Unexpected` and what's expected, `&self` its
// `expecting`, at the place serde_json gives them.
test("serde: a hand-written Visitor is given what serde_json gives it", async () => {
  const dir = fixture("serde-hand-written-visitor");
  writeFileSync(join(dir, "cases.rs"), String.raw`use serde::de::{self, Unexpected, Visitor};
use serde::{Deserialize, Deserializer};
use std::fmt;

#[derive(Debug, PartialEq)]
pub struct Level(pub u8);

struct LevelVisitor;

impl<'de> Visitor<'de> for LevelVisitor {
    type Value = Level;
    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("a level from 0 to 9, or its name")
    }
    fn visit_u64<E: de::Error>(self, v: u64) -> Result<Level, E> {
        if v <= 9 { Ok(Level(v as u8)) } else { Err(E::invalid_value(Unexpected::Unsigned(v), &self)) }
    }
    fn visit_i64<E: de::Error>(self, v: i64) -> Result<Level, E> {
        Err(E::invalid_value(Unexpected::Signed(v), &"a level of no sign"))
    }
    fn visit_f64<E: de::Error>(self, v: f64) -> Result<Level, E> {
        Err(E::invalid_type(Unexpected::Float(v), &self))
    }
    fn visit_bool<E: de::Error>(self, v: bool) -> Result<Level, E> {
        Err(E::invalid_type(Unexpected::Bool(v), &self))
    }
    fn visit_unit<E: de::Error>(self) -> Result<Level, E> {
        Err(E::invalid_length(0, &self))
    }
    fn visit_str<E: de::Error>(self, v: &str) -> Result<Level, E> {
        match v {
            "low" => Ok(Level(1)),
            "high" => Ok(Level(9)),
            "" => Err(E::missing_field("name")),
            "twice" => Err(E::duplicate_field("name")),
            "field" => Err(E::unknown_field(v, &[])),
            _ => Err(E::unknown_variant(v, &["low", "high", "mid"])),
        }
    }
}

impl<'de> Deserialize<'de> for Level {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        d.deserialize_any(LevelVisitor)
    }
}

#[derive(Debug, PartialEq)]
pub struct Name(pub String);

struct NameVisitor;

impl<'de> Visitor<'de> for NameVisitor {
    type Value = Name;
    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "a name")
    }
    fn visit_borrowed_str<E: de::Error>(self, v: &'de str) -> Result<Name, E> {
        Ok(Name(format!("borrowed {v}")))
    }
    fn visit_str<E: de::Error>(self, v: &str) -> Result<Name, E> {
        Ok(Name(format!("copied {v}")))
    }
}

impl<'de> Deserialize<'de> for Name {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        d.deserialize_str(NameVisitor)
    }
}

pub struct Count(pub u64);

struct CountVisitor;

impl<'de> Visitor<'de> for CountVisitor {
    type Value = Count;
    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("a count")
    }
    fn visit_u64<E: de::Error>(self, v: u64) -> Result<Count, E> {
        Ok(Count(v))
    }
    // Never given one by deserialize_u64.
    fn visit_str<E: de::Error>(self, v: &str) -> Result<Count, E> {
        Ok(Count(v.len() as u64))
    }
}

impl<'de> Deserialize<'de> for Count {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        d.deserialize_u64(CountVisitor)
    }
}

pub fn report() -> String {
    let level = |text: &str| match serde_json::from_str::<Level>(text) { Ok(l) => format!("{l:?}"), Err(e) => e.to_string() };
    let name = |text: &str| match serde_json::from_str::<Name>(text) { Ok(l) => format!("{l:?}"), Err(e) => e.to_string() };
    let count = |text: &str| match serde_json::from_str::<Count>(text) { Ok(c) => c.0.to_string(), Err(e) => e.to_string() };
    [
        level("3"), level("12"), level("-2"), level("2.5"), level("1e3"), level("true"), level("null"),
        level("\"low\""), level("\"\""), level("\"twice\""), level("\"field\""), level("\"wat\""), level("[1]"),
        name("\"ada\""), name("\"a\\u0064a\""), name("7"),
        count("18446744073709551615"), count("\"9\""), count("-1"),
    ].join("\n")
}
`);
  const expected = JSON.parse(run([native(dir)]));
  expect((await compiled(dir)).report()).toBe(expected);
});

// A hand-written visitor's `visit_seq` and `visit_map` (ADR 0459): what
// it reads next of a sequence or a map, `deserialize_struct` given either,
// and a hint's error of what it peeks at, where serde_json gives it.
test("serde: a hand-written visitor reads a sequence and a map as serde_json does", async () => {
  const dir = fixture("serde-hand-written-seq-map");
  writeFileSync(join(dir, "cases.rs"), String.raw`use serde::de::{self, MapAccess, SeqAccess, Visitor};
use serde::{Deserialize, Deserializer};
use std::fmt;

#[derive(Debug)]
pub struct Point {
    pub x: i32,
    pub y: i32,
}

struct PointVisitor;

impl<'de> Visitor<'de> for PointVisitor {
    type Value = Point;
    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("a point")
    }
    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Point, A::Error> {
        let x = seq.next_element()?.ok_or_else(|| de::Error::invalid_length(0, &self))?;
        let y = seq.next_element()?.ok_or_else(|| de::Error::invalid_length(1, &self))?;
        Ok(Point { x, y })
    }
    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Point, A::Error> {
        let (mut x, mut y) = (None, None);
        while let Some(key) = map.next_key::<String>()? {
            match key.as_str() {
                "x" if x.is_some() => return Err(de::Error::duplicate_field("x")),
                "x" => x = Some(map.next_value()?),
                "y" => y = Some(map.next_value()?),
                other => return Err(de::Error::unknown_field(other, &["x", "y"])),
            }
        }
        Ok(Point {
            x: x.ok_or_else(|| de::Error::missing_field("x"))?,
            y: y.ok_or_else(|| de::Error::missing_field("y"))?,
        })
    }
}

impl<'de> Deserialize<'de> for Point {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        d.deserialize_struct("Point", &["x", "y"], PointVisitor)
    }
}

#[derive(Debug)]
pub struct Totals(pub Vec<(String, u32)>);

struct TotalsVisitor;

impl<'de> Visitor<'de> for TotalsVisitor {
    type Value = Totals;
    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("totals")
    }
    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Totals, A::Error> {
        let mut all = Vec::new();
        while let Some((name, n)) = map.next_entry::<String, u32>()? {
            all.push((name, n));
        }
        Ok(Totals(all))
    }
}

impl<'de> Deserialize<'de> for Totals {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        d.deserialize_map(TotalsVisitor)
    }
}

pub fn report() -> String {
    let point = |text: &str| match serde_json::from_str::<Point>(text) { Ok(p) => format!("{p:?}"), Err(e) => e.to_string() };
    let totals = |text: &str| match serde_json::from_str::<Totals>(text) { Ok(t) => format!("{t:?}"), Err(e) => e.to_string() };
    [
        point("{\"x\": 1, \"y\": 2}"), point("[3, 4]"), point("[5]"), point("{\"y\": 1}"), point("{\"x\": 1, \"x\": 2}"),
        point("{\"z\": 1}"), point("{\"x\": \"a\"}"), point("7"),
        totals("{\"a\": 1, \"b\": 2}"), totals("{}"), totals("[1]"), totals("{\"a\": -1}"),
    ].join("\n")
}
`);
  const expected = JSON.parse(run([native(dir)]));
  expect((await compiled(dir)).report()).toBe(expected);
});

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

// A hand-written `Serialize`'s `collect_seq` and `collect_map`, a tuple
// struct's fields, and a variant's tuple or struct (ADR 0459).
test("serde: a hand-written Serialize collects and writes variants as serde_json does", async () => {
  const dir = fixture("serde-hand-written-collect");
  writeFileSync(join(dir, "cases.rs"), String.raw`use serde::ser::{SerializeStructVariant, SerializeTupleStruct, SerializeTupleVariant};
use serde::{Serialize, Serializer};
use std::collections::BTreeMap;

pub struct Tags(pub Vec<String>);

impl Serialize for Tags {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.collect_seq(self.0.iter())
    }
}

pub struct Lengths(pub Vec<String>);

impl Serialize for Lengths {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.collect_seq(self.0.iter().map(|t| t.len()))
    }
}

pub struct Scores(pub BTreeMap<u32, f64>);

impl Serialize for Scores {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.collect_map(self.0.iter())
    }
}

pub struct Rgb(pub u8, pub u8, pub u8);

impl Serialize for Rgb {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let mut t = s.serialize_tuple_struct("Rgb", 3)?;
        t.serialize_field(&self.0)?;
        t.serialize_field(&self.1)?;
        t.serialize_field(&self.2)?;
        t.end()
    }
}

pub enum Shape {
    Line(i32, i32),
    Box { w: u32, h: u32 },
}

impl Serialize for Shape {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        match self {
            Shape::Line(a, b) => {
                let mut v = s.serialize_tuple_variant("Shape", 0, "Line", 2)?;
                v.serialize_field(a)?;
                v.serialize_field(b)?;
                v.end()
            }
            Shape::Box { w, h } => {
                let mut v = s.serialize_struct_variant("Shape", 1, "Box", 2)?;
                v.serialize_field("w", w)?;
                v.serialize_field("h", h)?;
                v.end()
            }
        }
    }
}

pub fn report() -> String {
    let tags = Tags(vec!["a".into(), "bc".into()]);
    [
        serde_json::to_string(&tags).unwrap(),
        serde_json::to_string_pretty(&tags).unwrap(),
        serde_json::to_string(&Lengths(vec!["abc".into(), "".into()])).unwrap(),
        serde_json::to_string(&Scores(BTreeMap::from([(2, 1.5), (1, 0.0)]))).unwrap(),
        serde_json::to_string(&Rgb(1, 2, 3)).unwrap(),
        serde_json::to_string(&vec![Shape::Line(-1, 4), Shape::Box { w: 2, h: 3 }]).unwrap(),
        serde_json::to_string_pretty(&Shape::Box { w: 2, h: 3 }).unwrap(),
    ]
    .join("\n")
}
`);
  const expected = JSON.parse(run([native(dir)]));
  expect((await compiled(dir)).report()).toBe(expected);
});

// A hand-written visitor's `visit_none` and `visit_some`, of
// `deserialize_option`, and serde's error of one it hasn't (ADR 0459).
test("serde: a hand-written visitor reads an option as serde_json does", async () => {
  const dir = fixture("serde-hand-written-option");
  writeFileSync(join(dir, "cases.rs"), String.raw`use serde::de::{self, Visitor};
use serde::{Deserialize, Deserializer};
use std::fmt;

#[derive(Debug)]
pub struct Maybe(pub Option<u8>);

struct MaybeVisitor;

impl<'de> Visitor<'de> for MaybeVisitor {
    type Value = Maybe;
    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("a level or nothing")
    }
    fn visit_none<E: de::Error>(self) -> Result<Maybe, E> {
        Ok(Maybe(None))
    }
    fn visit_some<D: Deserializer<'de>>(self, d: D) -> Result<Maybe, D::Error> {
        u8::deserialize(d).map(|n| Maybe(Some(n)))
    }
}

impl<'de> Deserialize<'de> for Maybe {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        d.deserialize_option(MaybeVisitor)
    }
}

#[derive(Debug)]
pub struct Required(pub u8);

struct RequiredVisitor;

impl<'de> Visitor<'de> for RequiredVisitor {
    type Value = Required;
    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("a level")
    }
    fn visit_some<D: Deserializer<'de>>(self, d: D) -> Result<Required, D::Error> {
        u8::deserialize(d).map(Required)
    }
}

impl<'de> Deserialize<'de> for Required {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        d.deserialize_option(RequiredVisitor)
    }
}

#[derive(Debug, Deserialize)]
pub struct Row {
    pub a: Maybe,
    pub b: Required,
}

pub fn report() -> String {
    let maybe = |text: &str| match serde_json::from_str::<Maybe>(text) { Ok(m) => format!("{m:?}"), Err(e) => e.to_string() };
    let required = |text: &str| match serde_json::from_str::<Required>(text) { Ok(m) => format!("{m:?}"), Err(e) => e.to_string() };
    let row = |text: &str| match serde_json::from_str::<Row>(text) { Ok(r) => format!("{r:?}"), Err(e) => e.to_string() };
    [
        maybe("null"), maybe("7"), maybe("300"), maybe("\"x\""), required("4"), required("null"),
        row("{\"a\": null, \"b\": 2}"), row("{\"a\": 1, \"b\": null}"),
    ]
    .join("\n")
}
`);
  const expected = JSON.parse(run([native(dir)]));
  expect((await compiled(dir)).report()).toBe(expected);
  // A `null`'s `visit_none` is given nothing.
  expect(readFileSync(join(dir, "cases.js"), "utf8")).toContain("none: () => maybeVisitorVisitor_visit_none(");
});
