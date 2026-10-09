// What the compiler writes of a Rust program, each test a small program of
// its own, compiled and run: `const` of a `let mut` nothing sets again, a
// discriminated union, `ptr::eq`, `on_load!` bodies, and the rest. Each
// compiles only its own fixture, past a setup that builds the crates, so a
// mutation that breaks one is caught here, not in a shared fixture's setup
// (ADR 0093).

import { beforeAll, expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { buildReact, compiler, fixture, run, target } from "./support";

beforeAll(buildReact, 600_000);

// ADR 0278: a `let mut` nothing sets again, only changed in place, is a
// `const`, as react.dev's toCommaSeparatedList has `const list = []`; one
// set again stays a `let`.
test("a variable nothing sets again is a const", async () => {
  const dir = fixture("consts-of-lets");
  writeFileSync(join(dir, "lib.rs"), `pub fn pushed() -> Vec<u32> {
    let mut list = Vec::new();
    list.push(1);
    list
}
pub fn counted() -> u32 {
    let mut n = 0;
    n += 1;
    n
}
pub fn later(flag: bool) -> u32 {
    let n;
    if flag { n = 1 } else { n = 2 }
    n
}
fn bump(n: &mut u32) {
    *n += 1;
}
pub fn handled() -> u32 {
    let mut n = 0;
    bump(&mut n);
    n
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("  const list = [];\n  list.push(1);");
  expect(js).toContain("  let n = 0;\n  n = (n + 1) >>> 0;");
  expect(js).toContain("  let n;\n  if (flag) {");
  expect(js).toContain("  let n = 0;\n  const n$1 = { value: n };");
  const lib = await import(join(dir, "lib.js"));
  expect([lib.pushed(), lib.counted(), lib.later(true), lib.later(false), lib.handled()]).toEqual([[1], 1, 1, 2, 1]);
});

// ADR 0279: a callback that only passes what it's given on, `(x) => double(x)`,
// is the function itself, `xs.map(double)`, as react.dev's
// toCommaSeparatedList has `array.map(renderCallback)`, where it's Rust's, which
// takes no more than it's given: a JS function's, `parseFloat`, may read
// the index too.
test("a callback that only passes its arguments on is the function", async () => {
  const dir = fixture("eta");
  writeFileSync(join(dir, "lib.rs"), `unsafe extern "Rust" {
    #[link_name = "parseFloat"]
    safe fn parse_float(text: &str) -> f64;
}

fn double(x: &u32) -> u32 {
    x * 2
}

pub fn doubled(xs: &[u32]) -> Vec<u32> {
    xs.iter().map(|x| double(x)).collect()
}

pub fn rendered(xs: &[u32], f: impl Fn(&u32, usize) -> u32) -> Vec<u32> {
    xs.iter().enumerate().map(|(i, x)| f(x, i)).collect()
}

pub fn parsed(xs: &[String]) -> Vec<f64> {
    xs.iter().map(|x| parse_float(x)).collect()
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("return xs.map(double);");
  expect(js).toContain("return xs.map(f);");
  expect(js).toContain("return xs.map((x) => parseFloat(x));");
  const lib = await import(join(dir, "lib.js"));
  expect([lib.doubled([1, 2]), lib.rendered([5, 6], (x: number, i: number) => x + i), lib.parsed(["1.5", "2"])]).toEqual([[2, 4], [5, 7], [1.5, 2]]);
});

// An `else` of nothing, of only comments in Rust, which rust-js doesn't
// carry, is no `else`, as react.dev's _app has one.
test("an else of nothing is left out", async () => {
  const dir = fixture("empty-else");
  writeFileSync(join(dir, "lib.rs"), `pub fn picked(flag: bool, out: &mut Vec<u32>) {
    if flag {
        out.push(1);
    } else {
        // Nothing to do.
    }
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("  if (flag) {\n    out.push(1);\n  }\n}");
  const out: number[] = [];
  (await import(join(dir, "lib.js"))).picked(true, out);
  expect(out).toEqual([1]);
});

// A loop over a string's characters is over the string, which JS steps
// through by them, `for (const char of text)`, as react.dev's createFileMap
// reads a meta string: no `Array.from` first, as nothing changes it.
test("a loop over a string's characters is over the string", async () => {
  const dir = fixture("chars-loop");
  writeFileSync(join(dir, "lib.rs"), `pub fn braces(text: &str) -> u32 {
    let mut n = 0;
    for char in text.chars() {
        if char == '{' {
            n += 1;
        }
    }
    n
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("  for (const char of text) {");
  expect((await import(join(dir, "lib.js"))).braces("{a}{😀}")).toBe(2);
});

// Whether any item is one value, `any(|t| t == "hidden")`, is whether the
// array includes it, as react.dev's createFileMap asks of a meta's tokens.
// Not of floats: `includes` finds NaN, which `==` never does.
test("whether any item equals a value is whether the array includes it", async () => {
  const dir = fixture("any-equal");
  writeFileSync(join(dir, "lib.rs"), `pub fn hidden(tokens: Vec<String>, name: &str) -> (bool, bool) {
    (tokens.iter().any(|token| token == "hidden"), tokens.iter().any(|token| token == name))
}

pub fn nan(xs: Vec<f64>) -> bool {
    xs.iter().any(|&x| x == f64::NAN)
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain(`[tokens.includes("hidden"), tokens.includes(name)]`);
  expect(js).toContain("xs.some((x) => x === NaN)");
  const lib = await import(join(dir, "lib.js"));
  expect(lib.hidden(["a", "hidden"], "b")).toEqual([true, false]);
  expect(lib.hidden([], "b")).toEqual([false, false]);
  expect(lib.nan([NaN])).toBe(false);
});

// A binding's parameter it names `#[rust_js::nullable(..)]` is `T | null`,
// as a field is (ADR 0275): its `None` is `null`, `JSON.stringify(value,
// null, 2)` as react.dev's Sandpack template writes it.
test("a binding's nullable parameter's None is null", async () => {
  const dir = fixture("nullable-param");
  writeFileSync(join(dir, "lib.rs"), `#[cfg_attr(rust_js, rust_js::link_name = "JSON.stringify")]
#[cfg_attr(rust_js, rust_js::nullable(replacer))]
#[allow(unused_variables)]
fn stringify(value: &[u32], replacer: Option<&[&str]>, space: u32) -> String {
    unreachable!()
}

pub fn plain(items: &[u32]) -> String {
    stringify(items, None, 2)
}

pub fn given(items: &[u32], keys: Option<&[&str]>) -> String {
    stringify(items, keys, 2)
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("return JSON.stringify(items, null, 2);");
  expect(js).toContain("return JSON.stringify(items, keys ?? null, 2);");
  const lib = await import(join(dir, "lib.js"));
  expect([lib.plain([1, 2]), lib.given([3], undefined)]).toEqual(["[\n  1,\n  2\n]", "[\n  3\n]"]);
});

// A string literal written across lines is a template literal with its
// line breaks, as react.dev's Sandpack template writes a file's code; one
// written with `\n` keeps it.
test("a string written across lines keeps its lines", async () => {
  const dir = fixture("multiline-string");
  writeFileSync(join(dir, "lib.rs"), [
    "pub fn page() -> &'static str {",
    "    r#\"<div>",
    "  `${x}` \\ \"q\"",
    "</div>\"#",
    "}",
    "",
    "pub fn cooked() -> &'static str {",
    "    \"a",
    "b\"",
    "}",
    "",
    "pub fn escaped() -> &'static str {",
    "    \"a\\nb\"",
    "}",
    "",
    "pub fn continued() -> &'static str {",
    "    \"a\\n\\",
    "     b\"",
    "}",
    "",
  ].join("\n"));
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("  return `<div>\n  \\`\\${x}\\` \\\\ \"q\"\n</div>`;");
  expect(js).toContain("  return `a\nb`;");
  expect(js).toContain('  return "a\\nb";');
  // A `\` ending a line leaves its break out: the one there is `\n`.
  expect(js).toMatch(/export function continued\(\) \{\n  return "a\\nb";/);
  const lib = await import(join(dir, "lib.js"));
  expect([lib.page(), lib.cooked(), lib.escaped(), lib.continued()]).toEqual(['<div>\n  `${x}` \\ "q"\n</div>', "a\nb", "a\nb", "a\nb"]);
});

// A field whose JS name isn't a name, `#[rust_js::name = "worker-bundle"]`,
// is read and written by its key, `files["worker-bundle"]`, as react.dev's
// RSC template reads it: `files.worker-bundle` would be a subtraction.
test("a field named what isn't a name is read by its key", async () => {
  const dir = fixture("keyed-field");
  writeFileSync(join(dir, "lib.rs"), `pub struct Files {
    #[cfg_attr(rust_js, rust_js::name = "worker-bundle")]
    pub worker_bundle: String,
    #[cfg_attr(rust_js, rust_js::name = "2d")]
    pub two_d: Option<u32>,
}

pub fn bundle(files: &Files) -> String {
    files.worker_bundle.clone()
}

pub fn set(files: &mut Files, to: u32) -> Option<u32> {
    files.two_d = Some(to);
    files.two_d
}

pub fn made() -> Files {
    Files { worker_bundle: "w".to_string(), two_d: None }
}

pub fn maybe(files: Option<&Files>) -> Option<String> {
    files.map(|files| files.worker_bundle.clone())
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain('return files["worker-bundle"];');
  expect(js).toContain('files["2d"] = to;');
  expect(js).toContain('return files?.["worker-bundle"];');
  const lib = await import(join(dir, "lib.js"));
  expect(lib.bundle({ "worker-bundle": "w" })).toBe("w");
  const files = lib.made();
  expect([lib.set(files, 3), files["2d"]]).toEqual([3, 3]);
  expect([lib.maybe(files), lib.maybe(undefined)]).toEqual(["w", undefined]);
});

// A struct updated from one it owns, `..file`, is that one spread, then the
// fields named, as a reference's is (ADR 0250): `{ ...code, hidden: true }`,
// as react.dev's RSC template hides its files.
test("a struct updated from one it owns is a spread", async () => {
  const dir = fixture("owned-spread");
  writeFileSync(join(dir, "lib.rs"), `pub struct File {
    pub code: String,
    pub hidden: Option<bool>,
    pub active: Option<bool>,
}

pub fn hide(file: File) -> File {
    File { hidden: Some(true), ..file }
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("return { ...file, hidden: true };");
  const lib = await import(join(dir, "lib.js"));
  expect(lib.hide({ code: "c", hidden: false, active: true })).toEqual({ code: "c", hidden: true, active: true });
});

// ADR 0284: an enum tagged by a property of its own, `#[rust_js::tag =
// "status"]`, is a discriminated union, as TypeScript's and ReScript's
// `@tag` are: each variant an object of its name and its fields,
// `{ status: "fulfilled", value }`, as `Promise.allSettled` gives them.
test("an enum tagged by a property of its own is a discriminated union", async () => {
  const dir = fixture("tagged-enum");
  writeFileSync(join(dir, "lib.rs"), `// A name equal to another of any case: its own \`==\`.
#[derive(Clone, Debug)]
pub struct Name(pub String);

impl PartialEq for Name {
    fn eq(&self, other: &Name) -> bool {
        self.0.to_lowercase() == other.0.to_lowercase()
    }
}

#[cfg_attr(rust_js, rust_js::tag = "status")]
#[derive(Clone, Debug, PartialEq)]
pub enum Settled {
    #[cfg_attr(rust_js, rust_js::name = "named")]
    Named { name: Name },
    #[cfg_attr(rust_js, rust_js::name = "fulfilled")]
    Fulfilled { value: u32 },
    #[cfg_attr(rust_js, rust_js::name = "rejected")]
    Rejected { reason: String },
    #[cfg_attr(rust_js, rust_js::name = "pending")]
    Pending,
    #[cfg_attr(rust_js, rust_js::name = "measured")]
    Measured { values: Vec<f64> },
}

pub fn made(n: u32) -> Vec<Settled> {
    vec![Settled::Fulfilled { value: n }, Settled::Rejected { reason: "no".to_string() }, Settled::Pending]
}

pub fn told(s: &Settled) -> String {
    match s {
        Settled::Fulfilled { value } => format!("got {value}"),
        Settled::Rejected { reason } => format!("failed: {reason}"),
        Settled::Pending => "waiting".to_string(),
        Settled::Named { name } => name.0.clone(),
        Settled::Measured { values } => format!("{} values", values.len()),
    }
}

pub fn same(a: &Settled, b: &Settled) -> bool {
    a.clone() == *b
}

const WAITING: Settled = Settled::Pending;
const DONE: Settled = Settled::Fulfilled { value: 7 };

pub fn constants() -> (Settled, Settled) {
    (WAITING, DONE)
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain('{ status: "fulfilled", value: n }');
  expect(js).toContain('{ status: "pending" }');
  expect(js).toContain('if (s.status === "fulfilled") {');
  const lib = await import(join(dir, "lib.js"));
  expect(lib.constants()).toEqual([{ status: "pending" }, { status: "fulfilled", value: 7 }]);
  expect(lib.made(1)).toEqual([{ status: "fulfilled", value: 1 }, { status: "rejected", reason: "no" }, { status: "pending" }]);
  // What JS made, as Promise.allSettled does, is read as Rust's.
  expect([lib.told({ status: "fulfilled", value: 2 }), lib.told({ status: "rejected", reason: "x" }), lib.told({ status: "pending" })])
    .toEqual(["got 2", "failed: x", "waiting"]);
  expect([lib.same({ status: "fulfilled", value: 2 }, { status: "fulfilled", value: 2 }), lib.same({ status: "pending" }, { status: "rejected", reason: "x" })])
    .toEqual([true, false]);
  expect(lib.same({ status: "named", name: ["Ada"] }, { status: "named", name: ["ADA"] })).toBe(true);
  // Equality compares fields by the tag too, NaN unequal as Rust has it.
  expect([lib.same({ status: "measured", values: [1] }, { status: "measured", values: [1] }), lib.same({ status: "measured", values: [NaN] }, { status: "measured", values: [NaN] })])
    .toEqual([true, false]);
});

// ADR 0029: an awaited reference given to a generic `&T` is awaited, though
// rustc reborrows it inside the `.await`.
test("an awaited reference given to a generic function is awaited", async () => {
  const dir = fixture("await-generic");
  writeFileSync(join(dir, "lib.rs"), `use js::Promise;

#[cfg_attr(rust_js, rust_js::link_name = "node:timers/promises#setTimeout")]
#[allow(unused_variables)]
fn later(ms: u32, value: &str) -> Promise<&'static str> {
    unreachable!()
}

fn same<T: ?Sized>(x: &T) -> &T {
    x
}

pub async fn passed(text: &str) -> String {
    same(later(0, text).await).to_string()
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  expect(readFileSync(join(dir, "lib.js"), "utf8")).toContain("return same(await setTimeout(0, text));");
  expect(await (await import(join(dir, "lib.js"))).passed("text")).toBe("text");
});

// ADR 0060: a fieldless enum's derived `{:?}` is its Rust name, though a
// variant's JS is another, `rust_js::name`'s, of the crate's or another's.
test("a renamed variant is shown by its Rust name", async () => {
  const dir = fixture("debug-renamed");
  writeFileSync(join(dir, "lib.rs"), `use js::intl::LocaleMatcher;

#[derive(Clone, Copy, Debug)]
pub enum Fit {
    #[cfg_attr(rust_js, rust_js::name = "best fit")]
    BestFit,
    Lookup,
}

#[cfg_attr(rust_js, rust_js::tag = "kind")]
#[derive(Clone, Copy, Debug)]
pub enum Light {
    #[cfg_attr(rust_js, rust_js::name = "on")]
    On,
    Off,
}

pub fn shown(best: bool) -> (String, String, String) {
    let fit = if best { Fit::BestFit } else { Fit::Lookup };
    let matcher = if best { LocaleMatcher::BestFit } else { LocaleMatcher::Lookup };
    let light = if best { Light::On } else { Light::Off };
    (format!("{fit:?}"), format!("{matcher:?}"), format!("{light:?}"))
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  const lib = await import(join(dir, "lib.js"));
  expect([lib.shown(true), lib.shown(false)]).toEqual([["BestFit", "BestFit", "On"], ["Lookup", "Lookup", "Off"]]);
});

// ADR 0024: `get X.y` of a function without a receiver reads a class's static
// property each time it's called, `Notification.permission`.
test("a binding reads a static property", async () => {
  const dir = fixture("static-getter");
  writeFileSync(join(dir, "lib.rs"), `#[cfg_attr(rust_js, rust_js::link_name = "get Math.PI")]
fn pi() -> f64 {
    unreachable!()
}

pub fn read() -> f64 {
    pi() * 2.0
}

pub fn reader() -> fn() -> f64 {
    pi
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  expect(readFileSync(join(dir, "lib.js"), "utf8")).toContain("return Math.PI * 2;");
  const lib = await import(join(dir, "lib.js"));
  expect([lib.read(), lib.reader()()]).toEqual([Math.PI * 2, Math.PI]);
});

// ADR 0285: `std::ptr::eq` of JS objects is whether they're one, `===`.
test("ptr::eq of JS objects is whether they're one", async () => {
  const dir = fixture("ptr-eq");
  writeFileSync(join(dir, "lib.rs"), `use js::{Date, date};

pub fn same(a: &Date, b: &Date) -> bool {
    std::ptr::eq(a, b)
}

pub fn made() -> (bool, bool) {
    let a = date::new_with_time(0.0);
    let b = date::new_with_time(0.0);
    (core::ptr::eq(a, a), std::ptr::eq(a as *const Date, b))
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  expect(readFileSync(join(dir, "lib.js"), "utf8")).toContain("return a === b;");
  const lib = await import(join(dir, "lib.js"));
  const d = new Date(0);
  expect([lib.same(d, d), lib.same(d, new Date(0)), ...lib.made()]).toEqual([true, false, true, false]);
});

// ADR 0225: a JS value of unknown shape is a `js::Unknown`, as TypeScript's
// `unknown` and ReScript's are, which `classify` tells by `typeof`, and
// whose properties are read and set by name, as `obj[key]` is.
test("an unknown JS value is classified, and its properties read by name", async () => {
  const withWeb = ["--", "--extern", `webapi=${join(target, "libwebapi.rmeta")}`, "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];
  const dir = fixture("unknown");
  writeFileSync(join(dir, "lib.rs"), `use js::{Kind, Promise, Unknown, classify, json, object};
fn show(value: Option<&Unknown>) -> String {
    match value {
        None => "null".to_string(),
        Some(value) => match classify(value) {
            Kind::String(text) => format!("'{text}'"),
            Kind::Number(n) => n.to_string(),
            Kind::BigInt(n) => format!("{n}n"),
            Kind::Bool(b) => b.to_string(),
            Kind::Function(_) => "fn".to_string(),
            Kind::Array(items) => format!("[{}]", items.iter().map(|item| show(*item)).collect::<Vec<_>>().join(",")),
            Kind::Object(fields) => format!(
                "{{{}}}",
                object::keys(fields).iter().map(|key| format!("{key}:{}", show(js::get(fields, key)))).collect::<Vec<_>>().join(",")
            ),
        },
    }
}
pub fn parsed(text: &str) -> String {
    match json::parse(text) {
        Ok(value) => show(value),
        Err(_) => "invalid".to_string(),
    }
}
pub fn renamed(text: &str) -> String {
    match json::parse(text) {
        Ok(Some(value)) => {
            js::set(value, "name", "new");
            show(Some(value))
        }
        _ => "invalid".to_string(),
    }
}
pub fn shown(value: &Unknown) -> String {
    show(Some(value))
}
pub fn body(response: &webapi::Response) -> Promise<Option<&'static Unknown>> {
    response.json()
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withWeb]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  // `classify` is the value, each variant told by `typeof`; a key that's a
  // variable is `[key]`, one written that's a name `.name`.
  expect(js).toContain('}\n  if (typeof value === "string") {');
  expect(js).toContain("}\n  if (Array.isArray(value)) {");
  expect(js).toContain('if (typeof value === "function") {');
  expect(js).toContain("Object.keys(value)");
  expect(js).toContain("show(value[key])");
  expect(js).toContain('match._0.name = "new";');
  expect(js).toContain("const match = $try(() => JSON.parse(text));");
  expect(js).toContain("return response.json();");
  const lib = await import(join(dir, "lib.js"));
  expect([lib.parsed('{"a":[1,"x",null,true],"b":{}}'), lib.parsed("12"), lib.parsed("null"), lib.parsed("nope")])
    .toEqual(["{a:[1,'x',null,true],b:{}}", "12", "null", "invalid"]);
  expect(lib.renamed('{"name":"old","n":2}')).toBe("{name:'new',n:2}");
  expect([lib.shown(() => 1), lib.shown({ f: Math.max })]).toEqual(["fn", "{f:fn}"]);
});

// ADR 0267: each `on_load!` body is a function's, its locals its own: two
// that bind one name are two variables at the module's top.
test("on_load bodies that bind one name each keep their own", async () => {
  const dir = fixture("on-load-scopes");
  writeFileSync(join(dir, "lib.rs"), `unsafe extern "Rust" {
    #[link_name = "globalThis.onLoadSeen"]
    safe fn seen(n: u32);
}
fn pair() -> (u32, u32) {
    (1, 0)
}
js::on_load! {
    let (x, y) = pair();
    seen(x + y);
}
js::on_load! {
    let x = 2;
    seen(x);
}
pub fn ready() -> u32 {
    other::three()
}
pub mod other {
    js::on_load! {
        let x = 4;
        super::seen(x);
    }
    pub fn three() -> u32 {
        3
    }
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  expect(readFileSync(join(dir, "lib.js"), "utf8")).toContain("const [x, y] = pair();\nglobalThis.onLoadSeen((x + y) >>> 0);\nconst x$1 = 2;\nglobalThis.onLoadSeen(x$1);");
  // Another module's top is its own.
  expect(readFileSync(join(dir, "other.js"), "utf8")).toContain("const x = 4;");
  const seen: number[] = [];
  const globals = globalThis as typeof globalThis & { onLoadSeen?: (n: number) => void };
  globals.onLoadSeen = (n) => seen.push(n);
  try {
    expect((await import(join(dir, "lib.js"))).ready()).toBe(3);
    expect(seen).toEqual([4, 1, 2]);
  } finally {
    delete globals.onLoadSeen;
  }
});

// ADR 0034: a replacement is the text it is, as Rust's is: JS reads `$&` and
// `$1` in a string as what's matched, so one with a `$` doubles it, and one
// not written out is a function's, `() => t`. `replacen(.., 1)` is JS's
// `replace`, of the first.
test("a replacement is the text it is, and replacen's one is the first", async () => {
  const dir = fixture("replacements");
  writeFileSync(join(dir, "lib.rs"), `pub fn all(s: &str, t: &str) -> String {
    s.replace("a", t)
}

pub fn dollar(s: &str) -> String {
    s.replace("a", "$&!")
}

pub fn plain(s: &str) -> String {
    s.replace("a", "b")
}

pub fn first(s: &str) -> String {
    s.replacen("export default ", "let App = ", 1)
}

pub fn first_of(s: &str, t: &str) -> String {
    s.replacen("a", t, 1)
}

pub fn by(s: &str, p: &str, t: &str) -> String {
    s.replace(p, t)
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain('return s.replaceAll("a", () => t);');
  expect(js).toContain('return s.replaceAll("a", "$$&!");');
  expect(js).toContain('return s.replaceAll("a", "b");');
  expect(js).toContain('return s.replace("export default ", "let App = ");');
  expect(js).toContain('return s.replace("a", () => t);');
  const lib = await import(join(dir, "lib.js"));
  expect([lib.all("xa", "$&$&"), lib.dollar("ba"), lib.plain("aa"), lib.first("export default A; export default B"), lib.first_of("aa", "$1")])
    .toEqual(["x$&$&", "b$&!", "bb", "let App = A; export default B", "$1a"]);
  // A pattern that may be empty is the runtime's (ADR 0063), its replacement as it is too.
  expect([lib.by("xa", "a", "$&"), lib.by("ab", "", "$")]).toEqual(["x$&", "$a$b$"]);
});

// ADR 0066: a format string written across lines keeps them, as a string
// literal does, a template literal of the page react.dev's DownloadButton
// writes; one written with `\n` keeps those.
test("a format string written across lines keeps its lines", async () => {
  const dir = fixture("format-lines");
  writeFileSync(join(dir, "lib.rs"), `pub fn page(code: &str) -> String {
    format!(
        "<p>
{code}
</p>"
    )
}

pub fn escaped(code: &str) -> String {
    format!("<p>\\n{code}\\n</p>")
}

pub fn raw(code: &str) -> String {
    format!(
        r#"<a href="x">
{}
</a>"#,
        code
    )
}

pub fn continued(code: &str) -> String {
    format!("<p>\\n{code}\\
        </p>")
}

pub fn logged(code: &str) {
    println!(
        "<p>
{code}
</p>"
    );
}

pub fn plain() -> String {
    format!(
        "<p>
</p>"
    )
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("return `<p>\n${code}\n</p>`;");
  expect(js).toContain("return `<p>\\n${code}\\n</p>`;");
  expect(js).toContain('return `<a href="x">\n${code}\n</a>`;');
  // A line a `\` continues isn't one: its `\n` is written so.
  expect(js).toContain("return `<p>\\n${code}</p>`;");
  expect(js).toContain("console.log(`<p>\n${code}\n</p>`);");
  expect(js).toContain("return `<p>\n</p>`;");
  const lib = await import(join(dir, "lib.js"));
  expect([lib.page("x"), lib.escaped("x"), lib.raw("x")]).toEqual(["<p>\nx\n</p>", "<p>\nx\n</p>", '<a href="x">\nx\n</a>']);
});

// ADR 0031: a constant borrowed, `&XS`, is the one there is, as nothing can
// change it through a shared reference, a library's too (ADR 0100); one
// used by value, which may be, is copied.
test("a constant borrowed is the one there is, a library's too", async () => {
  const dir = fixture("borrowed-const");
  writeFileSync(join(dir, "lib.rs"), `pub mod files {
    pub const XS: [&str; 2] = ["a", "b"];
}

use files::XS;

pub fn has(x: &str) -> bool {
    XS.contains(&x)
}

pub fn changed() -> (&'static str, &'static str) {
    let mut copy = XS;
    copy[0] = "z";
    (copy[0], XS[0])
}

pub fn most(x: u32) -> bool {
    [x].contains(&u32::MAX)
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--library", "--manifest", join(dir, "manifest.json")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("return XS.includes(x);");
  expect(js).toContain("XS.slice()");
  const lib = await import(join(dir, "lib.js"));
  expect([lib.has("b"), lib.has("c"), lib.changed(), lib.most(4294967295)]).toEqual([true, false, ["z", "a"], true]);
});
