// Differential regressions of values: each case's value written by
// serde_json natively, the oracle, and by its JS, from the same Rust and
// dependency versions (split from serde.test.ts by concept, #1334).
import { beforeAll, expect, test } from "bun:test";
import { writeFileSync } from "node:fs";
import { join } from "node:path";
import { compiled, native } from "./serde-support";
import { buildCompiler, buildSerde, fixture, run } from "./support";

beforeAll(() => {
  buildCompiler();
  buildSerde("rlib");
}, 600_000);

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
    // A numbered enum is its number in JS (ADR 0722), and serde's name in
    // JSON, written and read.
    name: "a repr enum serializes and deserializes by its variants' names",
    definitions: `#[derive(serde::Serialize, serde::Deserialize, Debug, PartialEq, Clone, Copy)]
#[repr(u8)]
pub enum Kind { Empty = 0, Entries = 3 }
pub fn read(text: &str) -> String {
    let kinds: Vec<Kind> = serde_json::from_str(text).unwrap();
    format!("{kinds:?} {}", kinds[1] == Kind::Entries)
}`,
    value: '(vec![Kind::Entries, Kind::Empty], read("[\\"Empty\\", \\"Entries\\"]"))',
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
