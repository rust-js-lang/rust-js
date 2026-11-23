// serde_path_to_error, a crate rustc builds whose read rust-js writes (ADR
// 1188): `deserialize(&mut serde_json::Deserializer::from_str(text))`, its
// error where in the value it was made, `items[1].qty`, by the crate's
// rules, at each place a path has one: a struct's field, a map's key of text,
// a number or a `bool`, an item's index, a variant's name, a key not yet
// read, and none for an `Option`, a newtype, a flattened field or an
// internally tagged variant's content. Native serde_path_to_error is the
// oracle, the same source and dependency versions as the JavaScript.
import { beforeAll, expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { compiled, native } from "./serde-support";
import { buildSerde } from "./serde-crate";
import { buildCompiler, compiler, fixture, run } from "./support";
import { es2022, repoTypes, type Setting, typeChecked } from "./type-checked";

const checking: Setting = { ...es2022, ...repoTypes };

beforeAll(() => {
  buildCompiler();
  buildSerde("rlib");
}, 600_000);

test("serde_path_to_error's read says where in the value it failed, as native serde_path_to_error does", async () => {
  const dir = fixture("serde-path-to-error");
  writeFileSync(join(dir, "cases.rs"), String.raw`
use serde::Deserialize;
use std::collections::{BTreeMap, HashMap};

#[derive(Deserialize, Debug)]
pub struct Item {
    pub sku: Sku,
    pub qty: u32,
}

#[derive(Deserialize, Debug)]
pub struct Sku(pub String);

#[derive(Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct Customer {
    pub name: String,
    pub email: String,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "lowercase")]
pub enum Shape {
    Circle { radius: f64 },
    Square(f64),
    Point,
}

#[derive(Deserialize, Debug)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum Tagged {
    Circle { radius: f64 },
}

#[derive(Deserialize, Debug)]
pub struct Base {
    pub id: u32,
}

#[derive(Deserialize, Debug)]
pub struct Order {
    pub customer: Customer,
    pub items: Vec<Item>,
    #[serde(default)]
    pub stock: BTreeMap<String, Item>,
    #[serde(default)]
    pub counts: HashMap<u32, u32>,
    #[serde(default)]
    pub flags: BTreeMap<bool, u32>,
    #[serde(default)]
    pub shape: Option<Shape>,
    #[serde(default)]
    pub tagged: Option<Tagged>,
    #[serde(default)]
    pub meta: Option<serde_json::Value>,
    #[serde(default)]
    pub point: Option<(u32, u32)>,
    #[serde(default)]
    pub orders: Vec<Order>,
}

#[derive(Deserialize, Debug)]
pub struct Flat {
    #[serde(flatten)]
    pub base: Base,
    pub name: String,
}

fn read<T: for<'de> Deserialize<'de> + std::fmt::Debug>(text: &str) -> String {
    let reader = &mut serde_json::Deserializer::from_str(text);
    match serde_path_to_error::deserialize::<_, T>(reader) {
        Ok(value) => format!("ok {value:?}"),
        Err(error) => format!(
            "{error} | path {} | inner {} | {:?}",
            error.path(),
            error.inner(),
            error,
        ),
    }
}

const ORDER: &str = r#"{"customer":{"name":"Ada","email":"a@b.c"},"items":[{"sku":"A1","qty":2}]"#;

pub fn report() -> Vec<String> {
    let with = |rest: &str| format!("{ORDER}{rest}");
    let mut lines = vec![
        read::<Order>(&with("}")),
        read::<Order>(r#"{"customer":{"name":"Ada","email":"a@b.c"},"items":[{"sku":"A1","qty":2},{"sku":"B2","qty":-1}]}"#),
        read::<Order>(r#"{"customer":{"name":"Ada"},"items":[]}"#),
        read::<Order>(r#"{"customer":{"name":"Ada","email":7},"items":[]}"#),
        read::<Order>(r#"{"customer":{"name":"Ada","email":"a","extra":1},"items":[]}"#),
        read::<Order>(r#"{"customer":{"name":"Ada","name":"B","email":"a"},"items":[]}"#),
        read::<Order>(r#"{"customer":{"name":"Ada","email":"a"},"items":{}}"#),
        read::<Order>(r#"{"customer":{"name":"Ada","email":"a"},"items":[{"sku":"A1","qty":1}]"#),
        read::<Order>(r#"{"customer":{"name":"Ada", 5:1},"items":[]}"#),
        read::<Order>(r#"{"customer" 1}"#),
        read::<Order>("[]"),
        read::<Order>(&with(r#","stock":{"A1":{"sku":"A1","qty":"x"}}}"#)),
        read::<Order>(&with(r#","counts":{"7":-1}}"#)),
        read::<Order>(&with(r#","counts":{"5000000000":1}}"#)),
        read::<Order>(&with(r#","counts":{"x":1}}"#)),
        read::<Order>(&with(r#","counts":{"1.5":1}}"#)),
        read::<Order>(&with(r#","flags":{"x":1}}"#)),
        read::<Order>(&with(r#","flags":{"true":"x"}}"#)),
        read::<Order>(&with(r#","shape":{"circle":{"radius":"x"}}}"#)),
        read::<Order>(&with(r#","shape":{"square":"x"}}"#)),
        read::<Order>(&with(r#","shape":"circle"}"#)),
        read::<Order>(&with(r#","shape":{"point":1}}"#)),
        read::<Order>(&with(r#","shape":{"oval":1}}"#)),
        read::<Order>(&with(r#","tagged":{"type":"circle","radius":"x"}}"#)),
        read::<Order>(&with(r#","tagged":{"type":"oval"}}"#)),
        read::<Order>(&with(r#","tagged":{"type":"circle","radius":[1,}}"#)),
        read::<Order>(&with(r#","meta":{"a":[1,{"b":tru}]}}"#)),
        read::<Order>(&with(r#","point":[1,"x"]}"#)),
        read::<Order>(&with(r#","point":[1]}"#)),
        read::<Order>(&with(r#","extra":[1,}"#)),
        read::<Order>(&with(&format!(r#","orders":[{ORDER}}},{ORDER},"orders":[{{"customer":{{"name":"Bo","email":"b"}},"items":[{{"sku":"C","qty":true}}]}}]}}]}}"#))),
        read::<Order>(&with("} trailing")),
        read::<Vec<Item>>(r#"[{"sku":"A1","qty":1},{"sku":2,"qty":1}]"#),
        read::<Vec<Item>>(r#"[{"sku":"A1","qty":1}"#),
        read::<Flat>(r#"{"id":"x","name":"a"}"#),
        read::<Flat>(r#"{"id":1}"#),
        read::<HashMap<String, Vec<u8>>>(r#"{"a.b":[1,300]}"#),
    ];
    // The reader keeps its place: a second value is the text after the first.
    let reader = &mut serde_json::Deserializer::from_str(r#"{"id":1} {"id":"x"}"#);
    let first: Result<Base, _> = serde_path_to_error::deserialize(&mut *reader);
    let second: Result<Base, _> = serde_path_to_error::deserialize(reader);
    lines.push(format!("{first:?} {second:?}"));
    // into_inner(), the serde_json error itself, and {:#?}.
    let error = serde_path_to_error::deserialize::<_, Order>(&mut serde_json::Deserializer::from_str(r#"{"customer":{"name":"Ada","email":7},"items":[]}"#)).unwrap_err();
    lines.push(format!("{error:#?}"));
    let path = error.path().to_string();
    let inner: serde_json::Error = error.into_inner();
    lines.push(format!("{path} / {inner} / {:?} / {}", inner, inner.to_string().len()));
    lines
}
`);
  const expected = run([native(dir)]);
  const { report } = await compiled(dir);
  expect(JSON.stringify(report())).toBe(expected.trim());
}, 300_000);

// Its `Error` as the crate shows it everywhere a program shows one: given a
// `Formatter`'s options, which its `Display` doesn't read and a path's index
// does; written into a writer, a piece at a time; boxed as a `dyn Error`; a
// root's path none. And the reader `from_str` makes read by a type's own
// `deserialize`, value after value, as serde_json reads it.
test("serde_path_to_error's error is shown as the crate shows it, and its reader read as serde_json's", async () => {
  const dir = fixture("serde-path-to-error-shown");
  writeFileSync(join(dir, "cases.rs"), String.raw`use serde::Deserialize;
use std::fmt;

#[derive(Deserialize, Debug)]
pub struct Item {
    pub qty: u32,
}

pub fn parse(text: &str) -> Result<Item, serde_path_to_error::Error<serde_json::Error>> {
    serde_path_to_error::deserialize(&mut serde_json::Deserializer::from_str(text))
}

fn boxed(text: &str) -> Result<Item, Box<dyn std::error::Error>> {
    Ok(parse(text)?)
}

struct Shown<'a>(&'a serde_path_to_error::Error<serde_json::Error>);
impl fmt::Display for Shown<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "<{}|{:?}|{}>", self.0, self.0.path(), self.0.path())
    }
}

fn direct(text: &str) -> String {
    let mut reader = serde_json::Deserializer::from_str(text);
    let first = Item::deserialize(&mut reader).map_err(|e| e.to_string());
    let second = Item::deserialize(&mut reader).map_err(|e| e.to_string());
    format!("{first:?} {second:?}")
}

pub fn report() -> Vec<String> {
    let bad = r#"{"qty":[1]}"#;
    let e = parse(bad).unwrap_err();
    let b = boxed(bad).unwrap_err();
    vec![
        format!("[{:>60}] [{:x?}] [{:#x?}]", e, e.path(), e.path()),
        format!("{b} / {b:?} / {}", std::error::Error::source(&*b).is_none()),
        Shown(&e).to_string(),
        format!("{:?}", parse("[").unwrap_err()),
        {
            let reader = &mut serde_json::Deserializer::from_str(r#"[0,1,2,3,4,5,6,7,8,9,10,"x"]"#);
            let e = serde_path_to_error::deserialize::<_, Vec<u32>>(reader).unwrap_err();
            format!("{} {:?} {:x?} {:#X?}", e.path(), e.path(), e.path(), e.path())
        },
        direct(r#"{"qty":1} {"qty":2} x"#),
        direct(r#"{"qty":"a"}"#),
        direct("[1]"),
    ]
}
`);
  const expected = run([native(dir)]);
  const { report } = await compiled(dir);
  expect(JSON.stringify(report())).toBe(expected.trim());
}, 300_000);

// What rust-js doesn't write of either is refused where it's called:
// serde_json's reader's other methods (#1626), serde_path_to_error's own
// tracking and its path's segments, and its read of anything but serde_json's
// reader of text.
test("serde_path_to_error and serde_json's reader, where rust-js doesn't write them, are refused", () => {
  const dir = fixture("serde-path-to-error-refused");
  const refusal = (body: string) => {
    writeFileSync(join(dir, "lib.rs"), `use serde::Deserialize;\n#[derive(Deserialize)]\npub struct Item {\n    pub qty: u32,\n}\npub fn f(t: &str) -> String {\n    ${body}\n}\n`);
    const p = Bun.spawnSync([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--edition=2024", "--crate-type=lib", ...buildSerde()]);
    expect(p.exitCode).not.toBe(0);
    return p.stderr.toString();
  };
  expect(refusal("serde_json::Deserializer::from_str(t).end().is_ok().to_string()")).toContain(
    "rust-js does not support calling `serde_json::Deserializer::<R>::end` (#1626)",
  );
  expect(refusal("let mut track = serde_path_to_error::Track::new();\n    let mut de = serde_json::Deserializer::from_str(t);\n    let _ = Item::deserialize(serde_path_to_error::Deserializer::new(&mut de, &mut track));\n    track.path().to_string()")).toContain(
    "rust-js does not support calling `serde_path_to_error::Track::new`",
  );
  expect(refusal("let e = serde_path_to_error::deserialize::<_, Item>(&mut serde_json::Deserializer::from_str(t)).err().unwrap();\n    e.path().iter().count().to_string()")).toContain(
    "rust-js does not support calling `serde_path_to_error::Path::iter`",
  );
  expect(refusal("serde_path_to_error::deserialize::<_, Item>(serde_json::json!({ \"qty\": 1 })).is_ok().to_string()")).toContain(
    "rust-js does not support calling `serde_path_to_error::deserialize`",
  );
}, 300_000);

// A crate's declarations of its `Error` and `Path`: `@rust-js/std`'s opaque
// `JsonPathError` and `JsonPath`, not `any`, which TypeScript takes only of
// what rust-js gave (ADR 1149), and gives back for Rust to read.
test("serde_path_to_error's Error and Path are declared std's opaque types", async () => {
  const dir = fixture("serde-path-to-error-declared");
  writeFileSync(join(dir, "lib.rs"), `use serde::Deserialize;

#[derive(Deserialize)]
pub struct Point {
    pub x: f64,
}

pub fn parse(text: &str) -> Result<Point, serde_path_to_error::Error<serde_json::Error>> {
    serde_path_to_error::deserialize(&mut serde_json::Deserializer::from_str(text))
}

pub fn path(error: &serde_path_to_error::Error<serde_json::Error>) -> &serde_path_to_error::Path {
    error.path()
}

pub fn shown(path: &serde_path_to_error::Path) -> String {
    path.to_string()
}
`);
  run([compiler, "--declarations", join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", ...buildSerde()]);
  const declarations = readFileSync(join(dir, "lib.d.ts"), "utf8");
  expect(declarations).toContain('import type { JsonPath, JsonPathError } from "@rust-js/std";');
  expect(declarations).toContain("_0: JsonPathError;");
  expect(declarations).toContain("export function path(error: JsonPathError): JsonPath;");
  expect(declarations).not.toContain("any");
  writeFileSync(join(dir, "use.ts"), `import { parse, path, shown } from "./lib.js";
const parsed = parse('{"x":"a"}');
export const where: string = parsed.TAG === "Err" ? shown(path(parsed._0)) : "";
`);
  expect(typeChecked(dir, "use.ts", checking)).toEqual([]);
  writeFileSync(join(dir, "wrong.ts"), `import { shown } from "./lib.js";
shown(["x"]);
`);
  expect(typeChecked(dir, "wrong.ts", checking)).toEqual([
    expect.stringContaining("error TS2741: Property '[jsonPath]' is missing in type 'string[]' but required in type 'JsonPath'"),
  ]);
  const lib = await import(join(dir, "lib.js"));
  const failed = lib.parse('{"x":"a"}');
  expect(lib.shown(lib.path(failed._0))).toBe("x");
});
