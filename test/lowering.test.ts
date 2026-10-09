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

// ADR 0030: a property chain of an option's value ends where it's `None`,
// `o?.inner.v`, one chain, as `map` does: `(o?.inner).v` would read `.v` of
// `undefined`.
test("an option's property chain ends where it's None", async () => {
  const dir = fixture("optional-chains");
  writeFileSync(join(dir, "lib.rs"), `pub struct Inner {
    pub v: u32,
}

pub struct Outer {
    pub inner: Inner,
}

pub fn deep(o: Option<&Outer>) -> Option<u32> {
    o.map(|x| x.inner.v)
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  expect(readFileSync(join(dir, "lib.js"), "utf8")).toContain("return o?.inner.v;");
  const lib = await import(join(dir, "lib.js"));
  expect([lib.deep({ inner: { v: 3 } }), lib.deep(undefined)]).toEqual([3, undefined]);
});

// ADR 0030: a method of what's in an option, of a JS object, is `o?.m(x)`,
// which calls it, its arguments too, only where `o` is: as a person writes
// `document.body?.appendChild(a)`. A Rust type's method takes its value as
// an argument, `Node.add(o, x)`, which `?.` can't skip.
test("an option's method call ends where it's None", async () => {
  const dir = fixture("optional-calls");
  writeFileSync(join(dir, "lib.rs"), `pub struct Node {
    pub n: u32,
}

impl Node {
    pub fn add(&self, x: u32) -> u32 {
        self.n + x
    }
}

pub fn tested(o: Option<&js::RegExp>, x: &str) -> Option<bool> {
    o.map(|r| r.test(x))
}

pub fn ran(o: Option<&js::RegExp>, x: &str) {
    o.map(|r| r.test(x));
}

pub fn added(o: Option<&Node>, x: u32) -> Option<u32> {
    o.map(|node| node.add(x))
}

pub fn or_not(o: Option<&js::RegExp>, x: &str) -> bool {
    o.map_or(false, |r| r.test(x))
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("return o?.test(x);");
  expect(js).toContain("  o?.test(x);");
  expect(js).toContain("return o != null ? Node.add(o, x) : undefined;");
  const lib = await import(join(dir, "lib.js"));
  expect([lib.tested(/a/, "a"), lib.tested(undefined, "a"), lib.ran(undefined, "a"), lib.added({ n: 1 }, 2), lib.added(undefined, 2), lib.or_not(undefined, "a")])
    .toEqual([true, undefined, undefined, 3, undefined, false]);
});

// ADR 0286: a component that renders nothing sometimes returns an
// `Option<JSX::Element>`, `undefined` where it's None, as react.dev's
// DownloadButton has `return null`, and `jsx!` takes it as it takes one
// that always renders, memo's too. React gives it no dictionary, a
// generic one's, as it gives none of a component (ADR 0201).
test("a component of an optional element is a component", async () => {
  const dir = fixture("optional-components");
  writeFileSync(join(dir, "lib.rs"), `use react::{JSX, MemoExoticComponent, ReactNode, jsx, memo};

pub struct LabelProps<'a> {
    pub text: &'a str,
    pub shown: bool,
}

pub fn Label(LabelProps { text, shown }: LabelProps) -> Option<JSX::Element> {
    if !shown {
        return None;
    }
    Some(jsx! { <b>{text}</b> })
}

thread_local! {
    pub static Memoized: MemoExoticComponent<LabelProps<'static>> = memo(Label);
}

pub struct ShownProps<C> {
    pub children: C,
}

pub fn Shown<C: ReactNode + Default>(ShownProps { children }: ShownProps<C>) -> Option<JSX::Element> {
    Some(jsx! { <i>{children}</i> })
}

pub fn Page() -> JSX::Element {
    jsx! {
        <main>
            <Label text="hi" shown={true} />
            <Label text="no" shown={false} />
            <Memoized text="memo" shown={true} />
            <Shown>{"kid"}</Shown>
        </main>
    }
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.jsx"), "--", "--extern", `react=${join(target, "libreact.rmeta")}`, "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  const js = readFileSync(join(dir, "lib.jsx"), "utf8");
  expect(js).toContain('<Label text="hi" shown />');
  expect(js).toContain("    return undefined;");
  expect(js).toContain("export function Shown({ children }) {");
  const { renderToStaticMarkup } = await import("react-dom/server");
  const lib = await import(join(dir, "lib.jsx"));
  expect(renderToStaticMarkup(lib.Page())).toBe("<main><b>hi</b><b>memo</b><i>kid</i></main>");
});

// ADR 0254: what jsx! captures for a key that might do something, an
// import it reads, react's `Fragment`, reads alike wherever it's read, as
// a binding of a module never changes: it's written in place, not
// `const match = Fragment`, as react.dev's NavigationBar gives Headless
// UI's `Listbox.Option` `as={Fragment}`.
test("an import jsx! captures is read in place", () => {
  const dir = fixture("captured-imports");
  writeFileSync(join(dir, "lib.rs"), `use react::{ElementType, FRAGMENT, JSX, jsx};

pub struct OptionProps {
    pub value: String,
    #[cfg_attr(rust_js, rust_js::name = "as")]
    pub r#as: Option<ElementType>,
}

pub fn Choice(OptionProps { value, .. }: OptionProps) -> JSX::Element {
    jsx! { <li>{value}</li> }
}

pub fn Choices(items: Vec<String>) -> JSX::Element {
    jsx! {
        <ul>
            {items.iter().map(|item| jsx! { <Choice key={item.as_str()} value={item.clone()} r#as={Some(FRAGMENT)} /> }).collect::<Vec<_>>()}
        </ul>
    }
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.jsx"), "--", "--extern", `react=${join(target, "libreact.rmeta")}`, "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  const js = readFileSync(join(dir, "lib.jsx"), "utf8");
  expect(js).toContain("<Choice value={item} as={Fragment} key={item} />");
  expect(js).not.toContain("const match");
});

// ADR 0280: a field that's `undefined` is no key wherever its object is
// written: returned, in a `format!`'s template, or in an optional call,
// as a JS callback that asks `"value" in options` is answered the same.
test("an undefined field is no key in a template or an optional call", async () => {
  const dir = fixture("undefined-fields-anywhere");
  writeFileSync(join(dir, "lib.rs"), `pub struct Options {
    pub value: Option<u32>,
}

pub fn direct(f: impl Fn(Options) -> u32) -> u32 {
    f(Options { value: None })
}

pub fn formatted(f: impl Fn(Options) -> u32) -> String {
    format!("value {}", f(Options { value: None }))
}

unsafe extern "Rust" {
    #[link_name = "keyed"]
    safe fn keyed(this: &js::JsObject, options: Options) -> u32;
}

pub fn optional(o: Option<&js::JsObject>) -> Option<u32> {
    o.map(|o| keyed(o, Options { value: None }))
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).not.toContain("value: undefined");
  const lib = await import(join(dir, "lib.js"));
  const keyed = (options: object) => ("value" in options ? 1 : 0);
  expect(js).toContain("return o?.keyed({});");
  expect([lib.direct(keyed), lib.formatted(keyed), lib.optional({ keyed })]).toEqual([0, "value 0", 0]);
});

// ADR 0040: React ignores what a DOM element's handler returns, so one of a
// single call is `() => f()`; a component's callback may read it, so a
// callback that drops its value, `on_check`'s, keeps the statement there.
test("only a DOM element's handler returns its one call", async () => {
  const dir = fixture("handler-returns");
  writeFileSync(join(dir, "lib.rs"), `#![allow(non_snake_case)]
use react::{JSX, jsx};

unsafe extern "Rust" {
    #[link_name = "parseInt"]
    safe fn parse_int(s: &str) -> f64;
}

pub struct CustomProps {
    #[cfg_attr(rust_js, rust_js::name = "onCheck")]
    pub on_check: Box<dyn Fn()>,
}

pub fn Custom(CustomProps { on_check }: CustomProps) -> JSX::Element {
    jsx! { <b>{"x"}</b> }
}

pub fn Page() -> JSX::Element {
    jsx! {
        <div onClick={|_| { parse_int("7"); }}>
            <Custom onCheck={Box::new(|| { parse_int("7"); })} />
        </div>
    }
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.jsx"), "--", "--extern", `react=${join(target, "libreact.rmeta")}`, "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  const js = readFileSync(join(dir, "lib.jsx"), "utf8");
  expect(js).toContain('<div onClick={() => parseInt("7")}>');
  const lib = await import(join(dir, "lib.jsx"));
  const custom = lib.Page().props.children;
  expect(custom.props.onCheck()).toBeUndefined();
});

// ADR 0040: a component's callback of one call whose JS gives `undefined`
// anyway returns it, `onSubmit={() => submit(false)}`, as react.dev writes
// it: a function of the crate's that gives `()`, a closure of its own, or a
// binding that says so, `#[rust_js::returns_undefined]`, React's setter's.
// A binding that doesn't, `parseInt`, keeps its value dropped.
test("a component's callback returns a call that gives undefined", async () => {
  const dir = fixture("undefined-callbacks");
  writeFileSync(join(dir, "lib.rs"), `#![allow(non_snake_case)]
use std::rc::Rc;
use react::{JSX, jsx};

unsafe extern "Rust" {
    #[link_name = "parseInt"]
    safe fn parse_int(s: &str) -> f64;
    #[link_name = "globalThis.record"]
    safe fn record(n: i32);
    #[link_name = "globalThis.record"]
    #[cfg_attr(rust_js, rust_js::returns_undefined)]
    safe fn noted(n: i32);
}

pub struct CardProps {
    #[cfg_attr(rust_js, rust_js::name = "onSubmit")]
    pub on_submit: Box<dyn Fn()>,
    #[cfg_attr(rust_js, rust_js::name = "onNext")]
    pub on_next: Rc<dyn Fn()>,
    #[cfg_attr(rust_js, rust_js::name = "onNote")]
    pub on_note: Option<Box<dyn Fn()>>,
    #[cfg_attr(rust_js, rust_js::name = "onParse")]
    pub on_parse: Box<dyn Fn()>,
    #[cfg_attr(rust_js, rust_js::name = "onRecord")]
    pub on_record: Box<dyn Fn()>,
}

pub fn Card(CardProps { .. }: CardProps) -> JSX::Element {
    jsx! { <b>{"card"}</b> }
}

fn submit(flag: bool) {
    record(if flag { 1 } else { 2 })
}

pub fn Page() -> JSX::Element {
    let next = move || {
        record(3);
    };
    jsx! {
        <Card
            onSubmit={Box::new(|| submit(false))}
            onNext={Rc::new(move || next())}
            onNote={Some(Box::new(|| noted(4)))}
            onParse={Box::new(|| { parse_int("7"); })}
            onRecord={Box::new(|| record(5))}
        />
    }
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.jsx"), "--", "--extern", `react=${join(target, "libreact.rmeta")}`, "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  const js = readFileSync(join(dir, "lib.jsx"), "utf8");
  expect(js).toContain("onSubmit={() => submit(false)}");
  expect(js).toContain("onNext={() => next()}");
  expect(js).toContain("onNote={() => globalThis.record(4)}");
  expect(js).toContain('onParse={() => {\n        parseInt("7");\n      }}');
  expect(js).toContain("onRecord={() => {\n        globalThis.record(5);\n      }}");
  const previous = globalThis.record;
  globalThis.record = () => 9;
  try {
    const { props } = (await import(join(dir, "lib.jsx"))).Page();
    expect([props.onSubmit(), props.onNext(), props.onParse(), props.onRecord()]).toEqual([undefined, undefined, undefined, undefined]);
  } finally { globalThis.record = previous; }
});

// A RegExp of a pattern and flags written as they are is a literal, as JS
// writes one, `/%s/g` (ADR 0243); one JS can't parse is made as it was,
// to throw when it runs.
test("a RegExp of a pattern as it is written is a literal", async () => {
  const dir = fixture("regex-literals");
  writeFileSync(join(dir, "lib.rs"), `use js::reg_exp;

pub fn placeholders(text: &str) -> String {
    reg_exp::replace(text, reg_exp::new("%s", "g"), "_")
}

// A literal's \`/\` is escaped, but in a class, where it needn't be.
pub fn slashes(text: &str) -> String {
    reg_exp::replace(text, reg_exp::new("a/b|[/]", "g"), "-")
}

// An empty pattern's literal is \`/(?:)/\`: \`//\` is a comment.
pub fn emptied(text: &str) -> String {
    reg_exp::replace(text, reg_exp::new("", ""), "^")
}

// A line break can't be in a literal.
pub fn joined(text: &str) -> String {
    reg_exp::replace(text, reg_exp::new("\\n", "g"), " ")
}

// One JS can't parse throws as it's made, not as the module is read.
pub fn unparsed() -> bool {
    reg_exp::new("(", "").test("(")
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain('text.replace(/%s/g, "_")');
  expect(js).toContain('text.replace(/a\\/b|[/]/g, "-")');
  expect(js).toContain('text.replace(/(?:)/, "^")');
  expect(js).toContain('new RegExp("\\n", "g")');
  expect(js).toContain('new RegExp("(", "")');
  const lib = await import(join(dir, "lib.js"));
  expect(lib.placeholders("%s and %s")).toBe("_ and _");
  expect(lib.slashes("a/b /")).toBe("- -");
  expect(lib.emptied("x")).toBe("^x");
  expect(lib.joined("a\nb")).toBe("a b");
  expect(() => lib.unparsed()).toThrow(SyntaxError);
});
