// What the compiler writes of a Rust program, each test a small program of
// its own, compiled and run: `const` of a `let mut` nothing sets again, a
// discriminated union, `ptr::eq`, `on_load!` bodies, and the rest. Each
// compiles only its own fixture, past a setup that builds the crates, so a
// mutation that breaks one is caught here, not in a shared fixture's setup
// (ADR 0093).

import { beforeAll, expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { buildReact, compiler, fixture, root, run, target } from "./support";

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
  expect(js).toContain("return o ? Node.add(o, x) : undefined;");
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
  expect(js).toContain("    return;");
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
  expect(js).toContain("<Choice key={item} value={item} as={Fragment} />");
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

// A dictionary of entries written out, each key a string, is an object
// literal of them, as react.dev's RSC template writes its files: the same
// properties, in the same order. Not of a `__proto__` key, which a literal
// makes the object's prototype, nor of a key given twice, nor of a value
// `undefined`, which an object's field leaves out (ADR 0280).
test("a dictionary of entries written out is an object literal", async () => {
  const withJs = ["--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];
  const dir = fixture("dict-literal");
  writeFileSync(join(dir, "lib.rs"), `use js::{Dict, dict};
pub fn files(code: &str) -> &'static Dict<String> {
    dict::from_entries(vec![
        ("/index.html".to_string(), code.to_string()),
        ("main".to_string(), "m".to_string()),
    ])
}
pub fn twice(code: &str) -> &'static Dict<String> {
    dict::from_entries(vec![("a".to_string(), code.to_string()), ("a".to_string(), "again".to_string())])
}
pub fn proto(code: &str) -> &'static Dict<String> {
    dict::from_entries(vec![("__proto__".to_string(), code.to_string())])
}
pub fn missing() -> &'static Dict<Option<String>> {
    dict::from_entries(vec![("a".to_string(), None)])
}
pub fn named(name: String, code: &str) -> &'static Dict<String> {
    dict::from_entries(vec![(name, code.to_string())])
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withJs]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain('return { "/index.html": code, main: "m" };');
  expect(js).toContain('return Object.fromEntries([\n    ["a", code],\n    ["a", "again"],\n  ]);');
  expect(js).toContain('return Object.fromEntries([["__proto__", code]]);');
  expect(js).toContain("return Object.fromEntries([[name, code]]);");
  expect(js).toContain('return Object.fromEntries([["a", undefined]]);');
  const lib = await import(join(dir, "lib.js"));
  expect(Object.entries(lib.files("c"))).toEqual([["/index.html", "c"], ["main", "m"]]);
  expect(Object.keys(lib.proto("c"))).toEqual(["__proto__"]);
  expect(Object.keys(lib.missing())).toEqual(["a"]);
});

// ADR 0275: a `#[rust_js::nullable]` field is TypeScript's `T | null`: its
// `None` is `null`, as Next.js's `getStaticProps` gives react.dev's errors
// page its `errorCode`, which JSON has no `undefined` for. Another such
// field's value is one already, passed on as it is.
test("a nullable field's None is null", async () => {
  const dir = fixture("nullable-null");
  writeFileSync(join(dir, "lib.rs"), `pub struct Props {
    #[cfg_attr(rust_js, rust_js::nullable)]
    pub code: Option<String>,
    #[cfg_attr(rust_js, rust_js::nullable)]
    pub message: Option<String>,
    pub title: Option<String>,
}

pub fn none() -> Props {
    Props { code: None, message: Some("m".to_string()), title: None }
}

pub fn given(code: Option<String>) -> Props {
    Props { code, message: None, title: None }
}

pub fn passed(Props { code, message, .. }: Props) -> Props {
    Props { code, message, title: None }
}

pub fn chosen(code: Option<&str>) -> Props {
    Props {
        code: match code {
            Some(code) if !code.is_empty() => Some(code.to_string()),
            _ => None,
        },
        message: None,
        title: None,
    }
}

pub fn moved(p: Props) -> Props {
    Props { code: p.code, message: Some(p.title.unwrap_or_default()), title: None }
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain('return { code: null, message: "m" };');
  expect(js).toContain("return { code: code ?? null, message: null };");
  expect(js).toContain("return { code, message };");
  // A conditional of `Some` or `None` is one of the value or `null`.
  expect(js).toContain("return { code: code || null, message: null };");
  expect(js).toContain('return { code: p.code, message: p.title ?? "" };');
  const lib = await import(join(dir, "lib.js"));
  expect([lib.given(undefined), lib.given("1")]).toEqual([{ code: null, message: null }, { code: "1", message: null }]);
  expect(JSON.stringify(lib.passed(lib.given(undefined)))).toBe('{"code":null,"message":null}');
  expect([lib.chosen("1").code, lib.chosen("").code, lib.chosen(undefined).code]).toEqual(["1", null, null]);
});

// ADR 0277: `let n = n;`, a variable shadowed by its own value, is `n`
// itself where nothing sets `n` while the new one is read: no `n$1`, as
// react.dev's errors page has `<Type />` of `let Type = from_unknown(Type)`.
// One of another name is the name written; one a loop sets again keeps its own.
test("a variable shadowed by its own value is the variable", async () => {
  const dir = fixture("shadows");
  writeFileSync(join(dir, "lib.rs"), `pub fn captured(text: String) -> Box<dyn Fn() -> String> {
    let text = text.clone();
    Box::new(move || text.clone())
}
pub fn kept(text: String) -> String {
    let kept = text;
    kept + "!"
}
pub fn before(flag: bool) -> u32 {
    let mut n = 1;
    if flag {
        n = 2;
    }
    let n = n;
    n + 1
}
pub fn summed(items: &[String]) -> usize {
    items.iter().fold(0, |total, item| {
        let item = item.as_str();
        total + item.len()
    })
}
pub fn looped() -> u32 {
    let mut fs: Vec<Box<dyn Fn() -> u32>> = Vec::new();
    let mut n = 0;
    while n < 2 {
        n += 1;
        let n = n;
        fs.push(Box::new(move || n));
    }
    fs[0]()
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("export function captured(text) {\n  return () => text;\n}");
  expect(js).toContain("  const kept$1 = text;\n");
  expect(js).toContain("  return (n + 1) >>> 0;\n");
  expect(js).toContain("    const n$1 = n;\n    fs.push(() => n$1);");
  // In a closure too, of its own parameter, as react.dev's createFileMap
  // casts each snippet it's given.
  expect(js).toContain("return items.reduce((total, item) => (total + $byteLen(item)) >>> 0, 0);");
  const lib = await import(join(dir, "lib.js"));
  expect([lib.captured("a")(), lib.kept("b"), lib.before(true), lib.before(false), lib.looped(), lib.summed(["ab", "c"])]).toEqual(["a", "b!", 3, 2, 1, 3]);
});

// ADR 0030: a getter's option mapped to a property of what's in it,
// `o.map(|e| e.id)`, is `o?.id`, which reads `o` once, as a `const` of it
// would, as react.dev's SocialBanner has `ref.current?.offsetHeight`.
test("a getter's option mapped to a property is an optional chain", async () => {
  const withWeb = ["--", "--extern", `webapi=${join(target, "libwebapi.rmeta")}`, "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];
  const dir = fixture("getter-chain");
  writeFileSync(join(dir, "lib.rs"), `use webapi::Element;

pub fn first_id(parent: &Element) -> Option<String> {
    parent.first_element_child().map(|child| child.id())
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withWeb]);
  expect(readFileSync(join(dir, "lib.js"), "utf8")).toContain("return parent.firstElementChild?.id;");
  const { first_id } = await import(join(dir, "lib.js"));
  expect([first_id({ firstElementChild: { id: "a" } }), first_id({ firstElementChild: null })]).toEqual(["a", undefined]);
});

// ADR 0280: a field made of a literal `None` is no key, `{ code }`, as
// hand-written JS leaves it out, and as react.dev's sandboxes give Sandpack
// `{ code, hidden, active }`. Rust can't tell: it reads a key that isn't
// there as `None`, compares the two alike, and hashes them alike.
test("a field of a literal None is left out", async () => {
  const dir = fixture("none-fields");
  writeFileSync(join(dir, "lib.rs"), `use std::collections::HashSet;

#[derive(PartialEq, Eq, Hash, Clone, Debug, Default)]
pub struct File {
    pub code: String,
    pub hidden: Option<bool>,
    pub read_only: Option<bool>,
}

pub fn literal(code: &str) -> File {
    File { code: code.to_string(), hidden: Some(true), read_only: None }
}

pub fn given(code: &str, read_only: Option<bool>) -> File {
    File { code: code.to_string(), hidden: Some(true), read_only }
}

pub fn defaulted(code: &str) -> File {
    File { code: code.to_string(), ..Default::default() }
}

#[derive(Clone, Copy)]
pub struct Flags {
    pub on: Option<bool>,
    pub off: Option<bool>,
}

pub fn reset(flags: &Flags) -> Flags {
    Flags { on: None, ..*flags }
}

pub fn alike(code: &str) -> (bool, usize, String) {
    let a = literal(code);
    let b = given(code, None);
    let set: HashSet<File> = [a.clone(), b.clone()].into_iter().collect();
    (a == b, set.len(), format!("{a:?}"))
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("return { code, hidden: true };");
  const lib = await import(join(dir, "lib.js"));
  expect(Object.keys(lib.literal("a"))).toEqual(["code", "hidden"]);
  expect(Object.keys(lib.defaulted("a"))).toEqual(["code"]);
  expect(lib.given("a", undefined).hidden).toBe(true);
  // Not after a spread, whose field it would be.
  expect(js).toContain("return { ...flags, on: undefined };");
  expect(lib.reset({ on: true, off: false })).toEqual({ on: undefined, off: false });
  expect(lib.alike("a")).toEqual([true, 1, 'File { code: "a", hidden: Some(true), read_only: None }']);
});

// ADR 0214: an untagged enum only made, never told apart, may have variants
// of one kind, as Next.js's `getStaticProps` gives `{ props }` or
// `{ notFound: true }`, react.dev's errors page's: each is its payload.
test("an untagged enum only made may hold objects in two variants", async () => {
  const dir = fixture("untagged-objects");
  writeFileSync(join(dir, "lib.rs"), `pub struct Found {
    pub props: u32,
}

pub struct Missing {
    #[cfg_attr(rust_js, rust_js::name = "notFound")]
    pub not_found: bool,
}

#[cfg_attr(rust_js, rust_js::untagged)]
pub enum Page {
    Found(Found),
    Missing(Missing),
}

pub fn page(code: u32) -> Page {
    if code == 0 { Page::Missing(Missing { not_found: true }) } else { Page::Found(Found { props: code }) }
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("{ notFound: true }");
  expect(js).toContain("{ props: code }");
  const { page } = await import(join(dir, "lib.js"));
  expect([page(0), page(3)]).toEqual([{ notFound: true }, { props: 3 }]);
});

// `matches!` of a kind's literal says the literal: `x === "a"` holds of no
// other kind, nor of `null`, so neither `typeof` nor `!= null` is said, and
// what's tested once is read where it's tested, as react.dev's Link tests
// `child.type.mdxName === "inlineCode"`. A temporary with a destructor
// stays in its `const`, which its drop names.
test("matches! of a kind's literal tests the value alone, where it's read", async () => {
  const withJs = ["--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];
  const dir = fixture("matches-literal");
  writeFileSync(join(dir, "lib.rs"), `use js::{Kind, Unknown, classify};
pub fn is_code(value: &Unknown) -> bool {
    matches!(js::get(value, "mdxName").map(classify), Some(Kind::String("inlineCode")))
}
pub fn is_text(value: Option<&Unknown>) -> bool {
    matches!(value.map(classify), Some(Kind::String("inlineCode")))
}
pub fn kind_is(value: &Unknown) -> bool {
    matches!(classify(value), Kind::String("inlineCode"))
}
pub fn listed(value: Option<&Unknown>) -> u32 {
    if let Some(value) = value
        && matches!(classify(value), Kind::Array(_))
    {
        1
    } else {
        0
    }
}
pub fn is_string(value: &Unknown) -> bool {
    matches!(js::get(value, "mdxName").map(classify), Some(Kind::String(_)))
}
pub struct Loud(pub u32);
impl Drop for Loud {
    fn drop(&mut self) {}
}
fn make(n: u32) -> Loud {
    Loud(n)
}
pub fn is_three(n: u32) -> bool {
    matches!(make(n).0, 3)
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withJs]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain('export function is_code(value) {\n  return value.mdxName === "inlineCode";\n}');
  expect(js).toContain('export function is_text(value) {\n  return value === "inlineCode";\n}');
  expect(js).toContain('export function kind_is(value) {\n  return value === "inlineCode";\n}');
  // An array is never `null`: `Array.isArray` alone says it.
  expect(js).toContain("export function listed(value) {\n  if (Array.isArray(value)) {");
  expect(js).toContain('export function is_string(value) {\n  return typeof value.mdxName === "string";\n}');
  const lib = await import(join(dir, "lib.js"));
  const values = [{ mdxName: "inlineCode" }, { mdxName: "pre" }, { mdxName: null }, {}, { mdxName: 5 }];
  expect(values.map(lib.is_code)).toEqual([true, false, false, false, false]);
  expect(values.map((v) => lib.is_string(v))).toEqual([true, true, false, false, false]);
  expect(["inlineCode", "pre", 5, null, undefined].map((v) => lib.is_text(v))).toEqual([true, false, false, false, false]);
  expect(["inlineCode", "pre", 5, {}].map((v) => lib.kind_is(v))).toEqual([true, false, false, false]);
  expect([lib.is_three(3), lib.is_three(4)]).toEqual([true, false]);
  expect([[1], undefined, null, "a"].map((v) => lib.listed(v))).toEqual([1, 0, 0, 0]);
});

// ADR 0020: only a mutated type is copied, and that's decided per type: a
// mutated `Pair<u32>` isn't a reason to copy a `Pair<bool>`. A generic
// function that mutates `Holder<T>` may mutate any `Holder<..>`, though.
test("copies are made for the mutated instantiations of a generic type only", async () => {
  const { fixture, compiler } = await import("./support");
  const { writeFileSync } = await import("node:fs");
  const dir = fixture("copies");
  writeFileSync(join(dir, "lib.rs"), `#[derive(Clone, Copy)]
pub struct Pair<T: Copy> {
    pub a: T,
    pub b: T,
}

#[derive(Clone, Copy)]
pub struct Holder<T: Copy> {
    pub value: T,
}

pub fn bumped(p: Pair<u32>) -> (Pair<u32>, Pair<u32>) {
    let mut q = p;
    q.a += 1;
    (p, q)
}

pub fn twice(p: Pair<bool>) -> (Pair<bool>, Pair<bool>) {
    let q = p;
    (p, q)
}

pub fn set<T: Copy>(holder: &mut Holder<T>, value: T) {
    holder.value = value;
}

pub fn both(h: Holder<bool>) -> (Holder<bool>, Holder<bool>) {
    let mut g = h;
    set(&mut g, !h.value);
    (h, g)
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = await Bun.file(join(dir, "lib.js")).text();
  expect(js).toContain("export function bumped(p) {\n  const q = { ...p };");
  expect(js).toContain("export function twice(p) {\n  const q = p;");
  expect(js).toContain("export function both(h) {\n  const g = { ...h };");
  const module = await import(join(dir, "lib.js"));
  expect(module.bumped({ a: 1, b: 2 })).toEqual([{ a: 1, b: 2 }, { a: 2, b: 2 }]);
  expect(module.twice({ a: true, b: false })).toEqual([{ a: true, b: false }, { a: true, b: false }]);
  expect(module.both({ value: true })).toEqual([{ value: true }, { value: false }]);
});

// A JS module's namespace, `#*.Root`, whose bindings a Rust module holds,
// is imported by that module's name, as react.dev's BrandMenu has
// `import * as ContextMenu` and `<ContextMenu.Root>`.
// A closure that only spawns an async block is an async arrow, as
// react.dev's BrandMenu writes `onSelect={async () => { await
// navigator.clipboard.writeText(..) }}`: it starts the same, and gives
// back a promise where nothing reads its `()` (ADR 0257).
test("a closure that only spawns an async block is an async arrow", async () => {
  const dir = fixture("spawning-closure");
  writeFileSync(join(dir, "copy.js"), "export const copied = [];\nexport async function copy(text) { copied.push(text); }\n");
  writeFileSync(join(dir, "lib.rs"), `unsafe extern "Rust" {
    #[link_name = "./copy.js#copy"]
    safe fn copy(text: &str) -> js::Promise<()>;
}

pub fn handler() -> Box<dyn Fn()> {
    Box::new(|| {
        js::spawn(Box::new(async {
            copy("#58C4DC").await;
        }))
    })
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain('return async () => {\n    await copy("#58C4DC");\n  };');
  const lib = await import(join(dir, "lib.js"));
  const { copied } = await import(join(dir, "copy.js"));
  lib.handler()();
  expect(copied).toEqual(["#58C4DC"]);
});

// ADR 0266: text is falsy in JS only where it's empty, so text an option
// keeps where it isn't, `filter(|s| !s.is_empty())`, then another's or a
// default, is JS's `||`: `meta.title || route?.title || ""`, as react.dev's
// Page writes it. An array, which is truthy empty, isn't.
test("text kept where it isn't empty, or another, is ||", async () => {
  const dir = fixture("text-or");
  writeFileSync(join(dir, "lib.rs"), `pub struct Meta<'a> {
    pub title: Option<&'a str>,
}

pub struct Route {
    pub title: String,
}

pub fn title<'a>(meta: &Meta<'a>, route: Option<&'a Route>) -> &'a str {
    (meta.title.filter(|title| !title.is_empty()))
        .or(route.map(|route| route.title.as_str()).filter(|title| !title.is_empty()))
        .unwrap_or("")
}

pub fn items(list: Option<Vec<u32>>) -> Vec<u32> {
    list.filter(|list| !list.is_empty()).unwrap_or(vec![1])
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain('return meta.title || route?.title || "";');
  expect(js).not.toContain("list ||");
  const { title, items } = await import(join(dir, "lib.js"));
  expect([title({ title: "" }, { title: "R" }), title({}, undefined), title({ title: "M" }, { title: "R" }), title({ title: "" }, { title: "" })]).toEqual(["R", "", "M", ""]);
  expect([items([]), items([2]), items(undefined)]).toEqual([[1], [2], [1]]);
});

// ADR 0264: a fieldless variant is its name (ADR 0013), so a `match` giving
// each variant its own name is what's matched, and a function that gives
// back what it's given, its argument: `/images/og-${section}.png`, as
// react.dev's Page writes it. One giving another name is a conditional.
test("an enum's own names are the enum", async () => {
  const dir = fixture("enum-names");
  writeFileSync(join(dir, "lib.rs"), `#[derive(Clone, Copy, PartialEq)]
pub enum Section {
    #[cfg_attr(rust_js, rust_js::name = "learn")]
    Learn,
    #[cfg_attr(rust_js, rust_js::name = "blog")]
    Blog,
}

impl Section {
    pub fn as_str(self) -> &'static str {
        match self {
            Section::Learn => "learn",
            Section::Blog => "blog",
        }
    }

    pub fn heading(self) -> &'static str {
        match self {
            Section::Learn => "learn",
            Section::Blog => "news",
        }
    }
}

pub fn image(section: Section) -> String {
    format!("/images/og-{}.png", section.as_str())
}

pub fn title(section: Section) -> String {
    format!("{} page", section.heading())
}

// Mapped by it, an option's is the option, as react.dev's Page keys its
// SidebarNav, \`key={section}\`.
pub fn key(section: Option<Section>) -> Option<&'static str> {
    section.map(Section::as_str)
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("as_str(section) {\n    return section;\n  }");
  expect(js).toContain("return `/images/og-${section}.png`;");
  expect(js).toContain("Section.heading(section)");
  expect(js).toContain("export function key(section) {\n  return section;\n}");
  const { image, title } = await import(join(dir, "lib.js"));
  expect([image("learn"), image("blog"), title("learn"), title("blog")]).toEqual(["/images/og-learn.png", "/images/og-blog.png", "learn page", "news page"]);
});

// ADR 0238: a `String` kept for good, `.leak()`, is the string itself, as
// react.dev's Page gives Seo the image it makes for a component's
// `'static` props: JS frees nothing itself.
test("a leaked String is the string", async () => {
  const dir = fixture("string-leak");
  writeFileSync(join(dir, "lib.rs"), `pub fn named(n: u32) -> &'static str {
    format!("item-{n}").leak()
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("return `item-${n}`;");
  const { named } = await import(join(dir, "lib.js"));
  expect(named(3)).toBe("item-3");
});

// A struct taken apart through a shared reference is JS's destructuring,
// `const { errorMessage, errorCode } = useErrorDecoderParams();`, as
// react.dev's ErrorDecoder has it: what's borrowed can't change while it
// is (ADR 0244). A `Cell`, which can, is the one JS object either way.
test("a struct taken apart through a reference is destructured", async () => {
  const dir = fixture("ref-destructure");
  writeFileSync(join(dir, "lib.rs"), `use std::cell::Cell;

pub struct Params {
    pub message: Option<String>,
    pub code: Option<String>,
}

pub struct Counter {
    pub count: Cell<u32>,
    pub label: String,
}

fn params() -> &'static Params {
    Box::leak(Box::new(Params { message: Some("m".to_string()), code: None }))
}

fn first(counter: &Counter) -> &Counter {
    counter
}

pub fn described() -> String {
    let Params { message, code } = params();
    format!("{message:?} {code:?}")
}

pub fn counted() -> u32 {
    let counter = Counter { count: Cell::new(1), label: "ab".to_string() };
    let Counter { count, label } = first(&counter);
    count.set(count.get() + label.len() as u32);
    counter.count.get()
}

// A closure taking a pair apart, inlined where it's called, is its part.
pub fn found(people: Vec<(String, u32)>) -> Option<usize> {
    people.binary_search_by_key(&57, |&(_, age)| age).ok()
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("const { message, code } = params();");
  const lib = await import(join(dir, "lib.js"));
  expect(js).toContain("$cmp(item[1], 57)");
  expect([lib.described(), lib.counted(), lib.found([["a", 50], ["b", 57]])]).toEqual(['Some("m") None', 3, 1]);
});

// An `if` whose branch leaves has no `else`: what follows it runs only when
// the branch doesn't, as JS writes it and react.dev's Link has it, `if (..)
// { return cloneElement(..); } return child;` (ADR 0237). A branch that
// doesn't leave keeps its `else`.
test("an if whose branch returns has no else", async () => {
  const dir = fixture("no-else-return");
  writeFileSync(join(dir, "lib.rs"), `pub fn classify(n: i32) -> &'static str {
    if n < 0 {
        println!("neg");
        "neg"
    } else if n == 0 {
        println!("zero");
        "zero"
    } else {
        println!("pos");
        "pos"
    }
}
// A branch that leaves by an inner \`if\` both of whose branches do.
pub fn nested(a: bool, b: bool) -> u32 {
    if a {
        if b {
            println!("ab");
            1
        } else {
            println!("a");
            2
        }
    } else {
        println!("none");
        3
    }
}
pub fn counted(n: i32) -> i32 {
    let mut total = 0;
    if n > 0 {
        total += n;
    } else {
        total -= n;
    }
    total
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain('  if (n < 0) {\n    console.log("neg");\n    return "neg";\n  }\n  if (n === 0) {\n    console.log("zero");\n    return "zero";\n  }\n  console.log("pos");\n  return "pos";\n}');
  expect(js).toContain("} else {\n    total = (total - n) | 0;");
  expect(js).toContain('    console.log("a");\n    return 2;\n  }\n  console.log("none");\n  return 3;\n}');
  const lib = await import(join(dir, "lib.js"));
  const log = console.log;
  console.log = () => {};
  try {
    expect([lib.classify(-1), lib.classify(0), lib.classify(3), lib.counted(2), lib.counted(-2)]).toEqual(["neg", "zero", "pos", 2, 2]);
    expect([lib.nested(true, true), lib.nested(true, false), lib.nested(false, true)]).toEqual([1, 2, 3]);
  } finally {
    console.log = log;
  }
});

// A `match` of a fieldless enum whose arms each give one table's field named
// as their variant is the table read by the value, as react.dev's
// ExpandableCallout reads `variantMap[type]`; an arm of another field keeps
// the conditional. The corpus's `match_index` runs it beside native Rust.
test("a match giving a table's field named as each variant reads the table by it", () => {
  const dir = fixture("match-index");
  writeFileSync(join(dir, "lib.rs"), readFileSync(join(root, "test/corpus/match_index.rs"), "utf8").replace("fn main()", "pub fn main()"));
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("function variant(kind) {\n  return VARIANTS[kind];\n}");
  expect(js).toContain("function picked(i) {\n  return VARIANTS[pick(i)];\n}");
  expect(js).toContain("const chosen = VARIANTS[kind];\n  return chosen.title;");
  expect(js).toContain('if (kind === "note") {\n    tmp = VARIANTS.pitfall;');
  expect(js).toContain('tmp = OTHER.pitfall;');
});

// `let Some(href) = href.filter(|href| !href.is_empty()) else { .. }` tests
// what the filter does and names `href`, with no `const` of the `Option`:
// of text, which is falsy only empty, `if (!href)`, as react.dev's Link
// has it. An array, which is truthy empty, keeps its `length` test, and
// what's bound of a variable that changes is a copy.
test("a let-else of an Option's filter tests the filter, and binds what it kept", async () => {
  const dir = fixture("let-else-filter");
  writeFileSync(join(dir, "lib.rs"), `pub fn link(href: Option<&str>) -> String {
    let Some(href) = href.filter(|href| !href.is_empty()) else {
        return "none".to_string();
    };
    href.to_uppercase()
}
pub fn named(name: Option<String>) -> String {
    if let Some(n) = name.filter(|n| !n.is_empty()) { n.to_uppercase() } else { "anon".to_string() }
}
pub fn listed(items: Option<Vec<u32>>) -> u32 {
    let Some(items) = items.filter(|items| !items.is_empty()) else {
        return 0;
    };
    items[0]
}
pub fn later(mut href: Option<&str>) -> String {
    let Some(h) = href.filter(|h| !h.is_empty()) else {
        return "none".to_string();
    };
    href = Some("changed");
    format!("{h} {}", href.unwrap_or_default())
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain('export function link(href) {\n  if (!href) {\n    return "none";\n  }\n  return href.toUpperCase();\n}');
  expect(js).toContain('if (name) {\n    return name.toUpperCase();');
  expect(js).toContain("if (!(items != null && items.length !== 0)) {\n    return 0;\n  }\n  return $index(items, 0);");
  const lib = await import(join(dir, "lib.js"));
  expect([lib.link("/a"), lib.link(""), lib.link(undefined)]).toEqual(["/A", "none", "none"]);
  expect([lib.named("ann"), lib.named(""), lib.named(undefined)]).toEqual(["ANN", "anon", "anon"]);
  expect([lib.listed([7]), lib.listed([]), lib.listed(undefined)]).toEqual([7, 0, 0]);
  expect([lib.later("/a"), lib.later("")]).toEqual(["/a changed", "none"]);
});

// `Some(Direction::Up)` of an `Option` of a unit variant is `d === "Up"`:
// `undefined === "Up"` is false too, so it's said without `d != null`, as
// a constant is. Where the variant is an object's, `d.TAG`, it's needed.
test("a Some of a unit variant is tested as the variant, without a null test", async () => {
  const dir = fixture("option-unit-variant");
  writeFileSync(join(dir, "lib.rs"), 'pub enum Direction {\n    Up,\n    Down,\n}\npub enum Shape {\n    Dot,\n    Line(u32),\n}\npub fn turn(d: Option<Direction>) -> u32 {\n    match d {\n        Some(Direction::Up) => 1,\n        Some(Direction::Down) => 2,\n        None => 0,\n    }\n}\npub fn size(s: Option<Shape>) -> u32 {\n    match s {\n        Some(Shape::Dot) => 1,\n        Some(Shape::Line(n)) => n,\n        None => 0,\n    }\n}\n');
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect([js.includes('if (d === "Up")'), js.includes("d != null"), js.includes('if (s === "Dot")'), js.includes("s != null && s.TAG")]).toEqual([true, false, true, true]);
  const lib = await import(join(dir, "lib.js"));
  expect([lib.turn("Up"), lib.turn("Down"), lib.turn(undefined), lib.size("Dot"), lib.size({ TAG: "Line", _0: 7 }), lib.size(undefined)]).toEqual([1, 2, 0, 1, 7, 0]);
});

// A value with a destructor moved before anything in its scope can leave,
// `hold(l)`'s into what it returns, is never the scope's to drop: no flag,
// no `try` (ADR 0197), as react.dev's ExternalLink moves its children. One
// moved after what may panic keeps them.
test("a value moved before anything can leave needs no drop of its scope", async () => {
  const dir = fixture("moved-first");
  writeFileSync(join(dir, "lib.rs"), 'pub struct Loud(pub u32);\nimpl Drop for Loud {\n    fn drop(&mut self) {}\n}\npub struct Holder {\n    pub l: Loud,\n}\npub fn hold(l: Loud) -> Holder {\n    Holder { l }\n}\npub fn checked(l: Loud, n: u32) -> Holder {\n    assert!(n > 0);\n    Holder { l }\n}\n');
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("export function hold(l) {\n  return { l };\n}");
  expect(js).toContain("l$live = false;");
  const lib = await import(join(dir, "lib.js"));
  expect([lib.hold([1]).l[0], lib.checked([2], 1).l[0]]).toEqual([1, 2]);
});

// An f64 known to be a whole number in an integer's range is that integer as
// it is (ADR 0302): where `indexOf` found it, as react.dev's CodeBlock writes
// `startColumn: index` once `index` isn't -1, and an integer's `as f64`. One
// that may be -1, or is set again after its test, is cast as `as` casts.
test("an f64 known to be a whole number in range is cast as it is", async () => {
  const dir = fixture("whole-casts");
  writeFileSync(join(dir, "lib.rs"), `unsafe extern "Rust" {
    #[link_name = "indexOf"]
    #[rust_js::position]
    safe fn index_of(this: &str, search: &str) -> f64;
}
pub fn found(line: &str, word: &str) -> u32 {
    let index = index_of(line, word);
    if index == -1.0 {
        panic!("Could not find {word}");
    }
    index as u32
}
pub fn unchecked(line: &str, word: &str) -> u32 {
    let index = index_of(line, word);
    index as u32
}
pub fn set_again(line: &str, word: &str) -> u32 {
    let mut index = index_of(line, word);
    if index == -1.0 {
        panic!("Could not find {word}");
    }
    index = -1.0;
    index as u32
}
pub fn byte(n: u8) -> u8 {
    let x = n as f64;
    x as u8
}
pub fn shifted(line: &str, word: &str) -> u32 {
    let mut index = index_of(line, word);
    index -= 1.0;
    if index == -1.0 {
        panic!("Could not find {word}");
    }
    index as u32
}
pub fn wide(n: u32) -> u64 {
    let x = n as f64;
    x as u64
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("  return index;\n}");
  expect(js).toContain("export function unchecked(line, word) {\n  const index = line.indexOf(word);\n  return $f64ToInt(index, 0, 4294967295);\n}");
  expect(js).toContain("  index = -1;\n  return $f64ToInt(index, 0, 4294967295);\n}");
  expect(js).toContain("export function byte(n) {\n  const x = n;\n  return x;\n}");
  const lib = await import(join(dir, "lib.js"));
  expect([lib.found("abc", "c"), lib.unchecked("abc", "z"), lib.set_again("abc", "a"), lib.byte(7), lib.shifted("abc", "z"), lib.wide(7)]).toEqual([2, 0, 0, 7, 0, 7n]);
});

// One whose arms bind what their subject holds, named where it is, is a
// conditional too (ADR 0209), as react.dev's DocsFooter picks a link or a
// `<div />`. What needs a `const` of its own is statements, as before.
test("a two-arm match whose arms bind places is a conditional expression", async () => {
  const dir = fixture("match-conditional-binds");
  writeFileSync(join(dir, "lib.rs"), `pub struct Item {
    pub title: String,
    pub path: Option<String>,
}
pub fn path(item: Option<&Item>) -> &str {
    let label = match item {
        Some(Item { path: Some(path), .. }) => path.as_str(),
        _ => "none",
    };
    label
}
// The second arm's too.
pub fn either(r: Result<u32, u32>) -> u32 {
    let v = match r {
        Ok(n) => n,
        Err(n) => n + 1,
    };
    v
}
// A guard reads what its arm binds.
pub fn titled(item: Option<&Item>) -> &str {
    let label = match item {
        Some(item) if item.path.is_some() => item.title.as_str(),
        _ => "untitled",
    };
    label
}
// What's owned has a \`const\` of its own, dropped where Rust drops it.
pub struct Loud(pub u32);
impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}
pub fn owned(some: bool) -> u32 {
    let l = if some { Some(Loud(7)) } else { None };
    let n = match l {
        Some(l) => l.0,
        None => 0,
    };
    println!("after");
    n
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain('const label = item && item.path != null ? item.path : "none";');
  expect(js).toContain('const v = r.TAG === "Ok" ? r._0 : (r._0 + 1) >>> 0;');
  expect(js).toContain('const label = item && item.path != null ? item.title : "untitled";');
  expect(js).not.toContain("let tmp");
  const lib = await import(join(dir, "lib.js"));
  const item = { title: "T", path: "/p" };
  expect([lib.path(item), lib.path({ title: "T" }), lib.path(undefined)]).toEqual(["/p", "none", "none"]);
  expect([lib.either({ TAG: "Ok", _0: 1 }), lib.either({ TAG: "Err", _0: 1 })]).toEqual([1, 2]);
  expect([lib.titled(item), lib.titled({ title: "T" })]).toEqual(["T", "untitled"]);
  const logs: string[] = [];
  const log = console.log;
  console.log = (line: string) => logs.push(line);
  try {
    expect([lib.owned(true), lib.owned(false)]).toEqual([7, 0]);
  } finally {
    console.log = log;
  }
  expect(logs).toEqual(["drop 7", "after", "after"]);
});

// A unit struct named `#[rust_js::name]` is that string, as a fieldless
// variant is (ADR 0013): `webapi`'s event names, `Click`, are types whose
// value is `"click"` (ADR 0223). One without a name holds nothing.
test("a named unit struct is its name", async () => {
  const dir = fixture("named-unit-struct");
  writeFileSync(join(dir, "lib.rs"), `#[rust_js::name = "click"]
pub struct Click;
pub struct Marker;
const CLICK: Click = Click;
fn pass<E>(event: E) -> E {
    event
}
pub fn click() -> Click {
    Click
}
pub fn passed() -> Click {
    pass(Click)
}
pub fn constant() -> Click {
    CLICK
}
pub fn marker() -> Marker {
    Marker
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain('return "click";');
  const lib = await import(join(dir, "lib.js"));
  expect([lib.click(), lib.passed(), lib.constant(), lib.marker()]).toEqual(["click", "click", "click", undefined]);
});

// A value shown is only read, through the reference `format_args!` takes,
// which nothing can change before it's shown: a `Copy` one changed elsewhere
// is shown in place, not copied first, as a read of it by value is (ADR 0020).
test("a formatted value is read in place, not copied", async () => {
  const dir = fixture("format-in-place");
  writeFileSync(join(dir, "lib.rs"), `#[derive(Clone, Copy, Debug)]
pub struct P {
    pub x: i32,
}
pub fn show(mut p: P) -> String {
    let q = p;
    p.x += 1;
    let all = [q, p];
    format!("{q:?} {p:?} {all:?}")
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("const q = { ...p };");
  expect(js).toContain("${pDebug_fmt(q)} ${pDebug_fmt(p)} [${all.map((item) => pDebug_fmt(item))");
  const lib = await import(join(dir, "lib.js"));
  expect(lib.show({ x: 1 })).toBe("P { x: 1 } P { x: 2 } [P { x: 1 }, P { x: 2 }]");
});

// A two-arm `match` as a value, its arms plain and binding nothing, is a
// conditional, as a person writes it (ADR 0209): its subject in place
// where the test reads it once, else in a `const` of its own.
test("a two-arm match that's a value is a conditional expression", async () => {
  const dir = fixture("match-conditional");
  writeFileSync(join(dir, "lib.rs"), `pub enum Kind {
    Primary,
    Secondary,
}
pub fn button_class(kind: Option<Kind>) -> &'static str {
    let class = match kind.unwrap_or(Kind::Primary) {
        Kind::Primary => "bg-link",
        Kind::Secondary => "text-primary",
    };
    class
}
pub fn low(x: u32) -> &'static str {
    let size = match x % 3 {
        0 | 1 => "low",
        _ => "high",
    };
    size
}
// A plain guard is the arm's test too.
pub fn guarded(n: u32, flag: bool) -> &'static str {
    let label = match n {
        0 if flag => "flagged zero",
        _ => "other",
    };
    label
}
// A &mut to a number, a cell, is tested by its value.
pub fn zero(n: &mut i32) -> &'static str {
    let label = match n {
        &mut 0 => "zero",
        _ => "other",
    };
    label
}
// A bool tested against true is the bool, as JS has it.
pub fn emptiness(xs: Vec<u32>) -> &'static str {
    let label = match xs.is_empty() {
        true => "empty",
        false => "some",
    };
    label
}
pub fn toggle(flag: bool) -> &'static str {
    let label = match flag {
        false => "off",
        true => "on",
    };
    label
}
// A first arm that takes everything is the value, its guard the test.
#[allow(unreachable_patterns)]
pub fn every(n: u32, flag: bool) -> &'static str {
    let all = match n {
        _ => "all",
        1 => "one",
    };
    let guarded = match n {
        _ if flag => "flag",
        _ => "no flag",
    };
    if all == "all" { guarded } else { "never" }
}
// A subject with a destructor is dropped where Rust drops it.
pub struct Loud(pub u32);
impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}
pub fn dropped() -> &'static str {
    let label = match Loud(1) {
        Loud(1) => "one",
        _ => "other",
    };
    println!("after");
    label
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain('(kind ?? "Primary") === "Primary" ? "bg-link" : "text-primary"');
  expect(js).toContain('match === 0 || match === 1 ? "low" : "high"');
  expect(js).toContain('const label = n === 0 && flag ? "flagged zero" : "other";');
  expect(js).toContain('const label = n.value === 0 ? "zero" : "other";');
  expect(js).toContain('const label = xs.length === 0 ? "empty" : "some";');
  expect(js).toContain('const label = !flag ? "off" : "on";');
  const lib = await import(join(dir, "lib.js"));
  expect([lib.button_class(undefined), lib.button_class("Secondary"), lib.low(4), lib.low(5)]).toEqual(["bg-link", "text-primary", "low", "high"]);
  expect([lib.guarded(0, true), lib.guarded(0, false), lib.guarded(1, true)]).toEqual(["flagged zero", "other", "other"]);
  expect([lib.zero({ value: 0 }), lib.zero({ value: 3 })]).toEqual(["zero", "other"]);
  expect([lib.every(1, true), lib.every(1, false)]).toEqual(["flag", "no flag"]);
  expect([lib.emptiness([]), lib.emptiness([1])]).toEqual(["empty", "some"]);
  expect([lib.toggle(false), lib.toggle(true)]).toEqual(["off", "on"]);
  const logged: string[] = [];
  const log = console.log;
  console.log = (line: string) => logged.push(line);
  try {
    expect(lib.dropped()).toBe("one");
  } finally {
    console.log = log;
  }
  expect(logged).toEqual(["drop 1", "after"]);
});

// `Option::as_deref` of a `String` or a `Vec` is the option itself: a
// `&str` is the string a `String` is, a slice the array (ADR 0211).
test("Option::as_deref of a String or a Vec is the option itself", async () => {
  const dir = fixture("option-as-deref");
  writeFileSync(join(dir, "lib.rs"), `pub fn path_is(path: Option<String>, want: &str) -> bool {
    path.as_deref() == Some(want)
}
pub fn first(items: Option<Vec<u32>>) -> Option<u32> {
    items.as_deref().and_then(|items| items.first().copied())
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).not.toContain("as_deref");
  const lib = await import(join(dir, "lib.js"));
  expect([lib.path_is("/learn", "/learn"), lib.path_is("/a", "/learn"), lib.path_is(undefined, "/learn")]).toEqual([true, false, false]);
  expect([lib.first([4, 5]), lib.first([]), lib.first(undefined)]).toEqual([4, undefined, undefined]);
});

// ADR 0284: a discriminated union a binding knows only some of, Sandpack's
// messages say, ends in an `otherwise` variant: any object whose tag names
// none of the others, the object itself. A match tests the others' tags,
// whichever arm comes first, and `Other(o)` is `o`.
test("a discriminated union's otherwise variant is any other object", async () => {
  const dir = fixture("open-unions");
  writeFileSync(join(dir, "lib.rs"), `#[cfg_attr(rust_js, rust_js::tag = "type")]
pub enum Message {
    #[cfg_attr(rust_js, rust_js::name = "resize")]
    Resize { height: f64 },
    #[cfg_attr(rust_js, rust_js::name = "start")]
    Start {
        #[cfg_attr(rust_js, rust_js::name = "firstLoad")]
        first_load: Option<bool>,
    },
    #[cfg_attr(rust_js, rust_js::otherwise)]
    Other(&'static js::JsObject),
}

pub fn described(message: &Message) -> String {
    match message {
        Message::Resize { height } => format!("resize {height}"),
        Message::Start { first_load } => format!("start {}", first_load.unwrap_or(false)),
        Message::Other(_) => "other".to_string(),
    }
}

pub fn other(message: &Message) -> bool {
    matches!(message, Message::Other(_))
}

pub fn made(object: &'static js::JsObject) -> Message {
    Message::Other(object)
}

pub fn all_made(objects: Vec<&'static js::JsObject>) -> Vec<Message> {
    objects.into_iter().map(Message::Other).collect()
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  const lib = await import(join(dir, "lib.js"));
  const done = { type: "done", compilatonError: false };
  expect([lib.described({ type: "resize", height: 3 }), lib.described({ type: "start", firstLoad: true }), lib.described(done)])
    .toEqual(["resize 3", "start true", "other"]);
  expect([lib.other(done), lib.other({ type: "resize", height: 1 }), lib.made(done) === done, lib.all_made([done])[0] === done]).toEqual([true, false, true, true]);
});

// ADR 0287: a `Cell`, or an `Rc` of one, that only its function and its
// closures read and set is a `let`, as react.dev's Preview keeps `let
// timeout` its listener sets and its cleanup clears: a clone of it is the
// same variable. One that leaves, returned or given whole, is `{ value }`.
test("a cell only its function reads and sets is a let", async () => {
  const dir = fixture("plain-cells");
  writeFileSync(join(dir, "lib.rs"), `use std::cell::Cell;
use std::rc::Rc;

pub fn local() -> u32 {
    let n = Cell::new(1);
    n.set(n.get() + 1);
    n.get()
}

pub fn counted(each: impl Fn(&dyn Fn())) -> u32 {
    let n = Cell::new(0);
    each(&|| n.set(n.get() + 1));
    n.get()
}

pub fn shared(listen: impl Fn(Box<dyn Fn(u32)>)) -> Box<dyn Fn() -> Option<u32>> {
    let timeout: Rc<Cell<Option<u32>>> = Rc::new(Cell::new(None));
    let started = timeout.clone();
    listen(Box::new(move |n| started.set(Some(n))));
    Box::new(move || timeout.get())
}

pub fn escaping() -> Rc<Cell<u32>> {
    let n = Rc::new(Cell::new(1));
    n.set(2);
    n
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("  let n = 1;\n  n = (n + 1) >>> 0;\n  return n;");
  expect(js).toContain("  let n = 0;\n  each(() => {\n    n = (n + 1) >>> 0;\n  });\n  return n;");
  expect(js).toContain("  let timeout;\n  listen((n) => {\n    timeout = n;\n  });\n  return () => timeout;");
  expect(js).toContain("  const n = { value: 1 };\n  n.value = 2;\n  return n;");
  const lib = await import(join(dir, "lib.js"));
  let listener: (n: number) => void = () => {};
  const read = lib.shared((f: (n: number) => void) => { listener = f; });
  const before = read();
  listener(7);
  expect([lib.local(), lib.counted((f: () => void) => { f(); f(); }), before, read(), lib.escaping().value]).toEqual([2, 2, undefined, 7, 2]);
});

// ADR 0102: `clearTimeout` and `clearInterval` take an id or `undefined`, as
// TypeScript types them, `number | undefined`: react.dev clears a timeout
// its listener may not have set, `clearTimeout(timeout)`.
test("a timer is cleared of an id or none", async () => {
  const dir = fixture("cleared-timers");
  writeFileSync(join(dir, "lib.rs"), `pub fn cleared(id: Option<&js::TimeoutId>) {
    js::clear_timeout(id);
}

pub fn stopped(id: Option<&js::IntervalId>) {
    js::clear_interval(id);
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("  clearTimeout(id);");
  expect(js).toContain("  clearInterval(id);");
  const lib = await import(join(dir, "lib.js"));
  let ran = false;
  const timer = setTimeout(() => { ran = true; }, 0);
  lib.cleared(undefined);
  lib.stopped(undefined);
  lib.cleared(timer);
  await new Promise((resolve) => setTimeout(resolve, 10));
  expect(ran).toBe(false);
});

// ADR 0288: a `Cell` in a struct's or a variant's field is the value it
// holds, the property set in place, as ReScript's `mutable` field is:
// `c.count = c.count + 1`, not `c.count.value`. A `&Cell` of one is a handle
// on the property (ADR 0099), so what's lent it sets the field.
test("a cell in a field is the property it holds", async () => {
  const dir = fixture("cell-fields");
  writeFileSync(join(dir, "lib.rs"), `use std::cell::Cell;

#[derive(Clone, Default, PartialEq, Debug)]
pub struct Counter {
    pub label: &'static str,
    pub count: Cell<u32>,
}

pub fn made() -> Counter {
    Counter { label: "a", count: Cell::new(1) }
}

pub fn counting(n: u32) -> Counter {
    Counter { label: "n", count: Cell::new(n) }
}

pub fn bumped(c: &Counter) -> u32 {
    c.count.set(c.count.get() + 1);
    c.count.get()
}

fn add(cell: &Cell<u32>, n: u32) {
    cell.set(cell.get() + n);
}

pub fn lent(c: &Counter) -> u32 {
    add(&c.count, 5);
    c.count.get()
}

pub fn cloned(c: &Counter) -> Counter {
    let copy = c.clone();
    c.count.set(9);
    copy
}

pub fn same(a: &Counter, b: &Counter) -> bool {
    a == b
}

pub fn shown(c: &Counter) -> String {
    format!("{c:?}")
}

pub fn fresh() -> Counter {
    Counter::default()
}

pub fn taken(c: Counter) -> u32 {
    let Counter { count, .. } = c;
    count.set(count.get() + 10);
    count.get()
}

pub fn matched(c: &Counter) -> u32 {
    match c {
        Counter { count, .. } => {
            count.set(count.get() * 3);
            count.get()
        }
    }
}

pub enum Slot {
    Empty,
    Held { n: Cell<u32> },
}

pub fn held(slot: &Slot) -> u32 {
    if let Slot::Held { n } = slot {
        n.set(n.get() * 2);
        n.get()
    } else {
        0
    }
}

pub fn empty() -> Slot {
    Slot::Empty
}

/// A name equal to another of any case: a hand-written eq.
pub struct Name(pub &'static str);

impl PartialEq for Name {
    fn eq(&self, other: &Name) -> bool {
        self.0.eq_ignore_ascii_case(other.0)
    }
}

#[derive(PartialEq)]
pub struct Named {
    pub name: Name,
    pub hits: Cell<u32>,
}

pub fn named_alike(a: &'static str, b: &'static str, hits: u32) -> bool {
    Named { name: Name(a), hits: Cell::new(1) } == Named { name: Name(b), hits: Cell::new(hits) }
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain('return { label: "a", count: 1 };');
  expect(js).toContain('return { label: "n", count: n };');
  expect(js).toContain("  c.count = (c.count + 1) >>> 0;\n  return c.count;");
  expect(js).toContain("  const copy = { ...c };");
  const lib = await import(join(dir, "lib.js"));
  expect(lib.made()).toEqual({ label: "a", count: 1 });
  const lentTo = lib.made();
  expect([lib.bumped(lib.made()), lib.lent(lentTo), lentTo.count]).toEqual([2, 6, 6]);
  const original = lib.made();
  expect([lib.cloned(original).count, original.count]).toEqual([1, 9]);
  expect([lib.same(lib.made(), lib.made()), lib.same(lib.made(), original)]).toEqual([true, false]);
  expect(lib.shown(lib.made())).toBe('Counter { label: "a", count: Cell { value: 1 } }');
  expect(lib.fresh()).toEqual({ label: "", count: 0 });
  const matched = lib.made();
  expect([lib.taken(lib.made()), lib.matched(matched), matched.count]).toEqual([11, 3, 3]);
  const slot = { TAG: "Held", n: 2 };
  expect([lib.held(slot), slot.n, lib.held(lib.empty())]).toEqual([4, 4, 0]);
  expect([lib.named_alike("Ann", "ANN", 1), lib.named_alike("Ann", "ANN", 2), lib.named_alike("Ann", "Bob", 1)]).toEqual([true, false, false]);
});

// ADR 0289: a `T: Copy` takes a copy function only where it may be given
// a value a copy isn't, one changed in place: react.dev's Preview debounces
// an `Option<&SandpackError>`, whose copy is itself, so `useDebounced(value)`
// is called as the original does. One a caller outside the crate may call,
// or given a struct that changes, still takes one.
test("a Copy bound takes a copy only where a copy isn't the value", async () => {
  const dir = fixture("copy-bounds");
  writeFileSync(join(dir, "lib.rs"), `#[derive(Clone, Copy)]
pub struct Point {
    pub x: u32,
}

fn twice<T: Copy>(value: T) -> (T, T) {
    (value, value)
}

fn kept<T: Copy>(value: T) -> T {
    let copy = value;
    copy
}

pub fn named(name: Option<&'static str>) -> (Option<&'static str>, Option<&'static str>) {
    twice(name)
}

pub fn moved(p: Point) -> u32 {
    let mut q = kept(p);
    q.x += 1;
    p.x + q.x
}

fn inner<T: Copy>(value: T) -> (T, T) {
    (value, value)
}

fn relay<T: Copy>(value: T) -> (T, T) {
    inner(value)
}

pub fn relayed(p: Point) -> u32 {
    let mut pair = relay(p);
    pair.0.x += 1;
    pair.0.x + pair.1.x
}

pub struct Holder<T> {
    value: T,
}

impl<T: Copy> Holder<T> {
    fn get(&self) -> T {
        self.value
    }
}

pub fn held(p: Point) -> u32 {
    let h = Holder { value: p };
    let mut q = h.get();
    q.x += 1;
    h.value.x + q.x
}

// Called through a dictionary, a trait's method takes one whatever its
// direct callers give.
trait Dup {
    fn dup<T: Copy>(&self, value: T) -> (T, T);
}

struct Twin;

impl Dup for Twin {
    fn dup<T: Copy>(&self, value: T) -> (T, T) {
        (value, value)
    }
}

fn through<S: Dup>(s: &S) -> (&'static str, &'static str) {
    s.dup("a")
}

pub fn duped() -> (&'static str, &'static str) {
    through(&Twin)
}

// A trait impl's own bound, given with its dictionary, as rustc's
// traits/conditional-dispatch.rs has it.
trait Again {
    fn again(&self) -> Self;
}

impl<T: Copy> Again for Option<T> {
    fn again(&self) -> Self {
        *self
    }
}

trait Get {
    fn get(&self) -> Self;
}

impl<T: Again> Get for T {
    fn get(&self) -> T {
        self.again()
    }
}

fn get_it<T: Get>(t: &T) -> T {
    t.get()
}

pub fn got() -> Option<u32> {
    get_it(&Some(4))
}

pub fn shared<T: Copy>(value: T) -> T {
    value
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("function twice(value) {");
  expect(js).toContain("return twice(name);");
  expect(js).toContain("function kept(value, TCopy) {");
  const lib = await import(join(dir, "lib.js"));
  expect([lib.named("a"), lib.moved({ x: 1 }), lib.relayed({ x: 1 }), lib.held({ x: 1 }), lib.duped(), lib.got()]).toEqual([["a", "a"], 3, 3, 3, ["a", "a"], 4]);
  // A library's public one may be given anything by its consumers.
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "library", "lib.js"), "--library", "--manifest", join(dir, "library", "lib.manifest.json"),
    "--", "--crate-name", "shared", `--emit=metadata=${join(dir, "library", "libshared.rmeta")}`]);
  expect(readFileSync(join(dir, "library", "lib.js"), "utf8")).toContain("export function shared(value, TCopy) {");
});

// ADR 0290: what a pattern binds of a variable that's set again later names
// it in place while nothing changes it: react.dev's Preview clears
// `rawError` after testing what it holds, `if (rawError &&
// rawError.message === ..) { rawError = null; }`. Changed while what's bound
// is still read, by a loop or a closure, what's bound is a `const`.
test("a binding names a variable set again only after its last use", async () => {
  const dir = fixture("steady-bindings");
  writeFileSync(join(dir, "lib.rs"), `use std::cell::Cell;

pub struct Failure {
    pub message: String,
}

pub struct Counter {
    pub hits: Cell<u32>,
}

pub fn cleared(error: Option<&Failure>) -> bool {
    let mut raw = error;
    if let Some(e) = raw
        && e.message == "noisy"
    {
        raw = None;
    }
    if let Some(e) = raw
        && e.message.contains("Example Error:")
    {
        raw = None;
    }
    raw.is_none()
}

pub fn read_after(error: Option<&Failure>) -> usize {
    let mut raw = error;
    if let Some(e) = raw {
        raw = None;
        return e.message.len() + raw.is_none() as usize;
    }
    0
}

pub fn taken(error: Option<&Failure>) -> usize {
    let mut raw = error;
    if let Some(e) = raw {
        raw.take();
        return e.message.len() + raw.is_none() as usize;
    }
    0
}

pub fn looped(error: Option<&Failure>) -> usize {
    let mut raw = error;
    let mut n = 0;
    if let Some(e) = raw {
        for _ in 0..2 {
            n += e.message.len();
            raw = None;
        }
    }
    n + raw.is_none() as usize
}

pub fn captured(error: Option<&Failure>) -> usize {
    let mut raw = error;
    if let Some(e) = raw {
        let length = || e.message.len();
        raw = None;
        return length() + raw.is_none() as usize;
    }
    0
}

pub fn bumped(start: Option<u32>) -> u32 {
    let mut raw = start;
    if let Some(n) = raw {
        if let Some(ref mut m) = raw {
            *m += 1;
        }
        return n + raw.unwrap_or(0);
    }
    0
}

pub fn bumped_else(start: Option<u32>) -> u32 {
    let mut raw = start;
    if let Some(n) = raw {
        let Some(ref mut m) = raw else { return 0 };
        *m += 1;
        return n + raw.unwrap_or(0);
    }
    0
}

pub fn lent(a: &Counter, b: &Counter) -> u32 {
    let mut raw = Some(a);
    if let Some(c) = raw {
        let hits = &c.hits;
        raw = Some(b);
        hits.set(5);
        return a.hits.get() * 10 + raw.map_or(0, |r| r.hits.get());
    }
    0
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain(`if (raw && raw.message === "noisy") {`);
  expect(js).toContain(`if (raw && raw.message.includes("Example Error:")) {`);
  expect(js.match(/const e\b/g)?.length).toBe(4);
  expect(js).toContain("const c = raw;");
  expect(js.match(/const n = raw;/g)?.length).toBe(2);
  const lib = await import(join(dir, "lib.js"));
  const failure = { message: "noisy" };
  expect([lib.cleared(failure), lib.cleared({ message: "kept" }), lib.read_after(failure), lib.taken(failure), lib.looped(failure), lib.captured(failure), lib.bumped(1), lib.bumped_else(1), lib.lent({ hits: 0 }, { hits: 0 })])
    .toEqual([true, false, 6, 6, 11, 6, 3, 3, 50]);
});

// ADR 0291: a `let` taking a variable apart is JS's destructuring, as one
// taking a value apart is: react.dev's Preview has `let {error: rawError,
// registerBundler} = sandpack;`, and calls `registerBundler(..)`, which a
// method call `sandpack.registerBundler(..)` wouldn't be: JS gives that one
// `sandpack` as its `this`.
test("a let taking a variable apart destructures it", async () => {
  const dir = fixture("place-destructure");
  writeFileSync(join(dir, "lib.rs"), `pub struct State {
    pub error: Option<&'static str>,
    pub status: u32,
    pub register: fn(u32) -> u32,
}

pub fn renamed(state: &State) -> u32 {
    let &State {
        error: mut raw,
        register: registered,
        ..
    } = state;
    if raw == Some("x") {
        raw = None;
    }
    registered(raw.map_or(0, |s| s.len() as u32))
}

pub fn plain(state: &State) -> u32 {
    let State { status, register, .. } = state;
    register(*status)
}

pub fn pair(values: (u32, u32)) -> u32 {
    let (a, b) = values;
    a * 10 + b
}

pub fn ignored(values: (u32, u32)) -> u32 {
    let (_, _) = values;
    1
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("let { error: raw, register: registered } = state;");
  expect(js).toContain("const { status, register } = state;");
  expect(js).toContain("const [a, b] = values;");
  expect(js).not.toContain("const [] = values;");
  const lib = await import(join(dir, "lib.js"));
  const state = { error: "abc", status: 4, register(n: number) { return this === undefined ? n : -1; } };
  expect([lib.renamed(state), lib.renamed({ ...state, error: "x" }), lib.plain(state), lib.pair([1, 2])]).toEqual([3, 0, 4, 12]);
});

// ADR 0292: an index a condition shows is in bounds is JS's own: react.dev's
// Preview reads `lintErrors[0]` where `lintErrors.length === 0` is false. In
// a branch, after an early return and in `for i in 0..xs.len()`, of a slice
// and an index the function never changes; anywhere else it's checked.
test("an index a condition shows in bounds is read as it is", async () => {
  const dir = fixture("known-bounds");
  writeFileSync(join(dir, "lib.rs"), `pub fn first(items: &[u32]) -> Option<u32> {
    if items.is_empty() { None } else { Some(items[0]) }
}

pub fn second(items: &Vec<u32>) -> u32 {
    if items.len() < 2 {
        return 0;
    }
    items[1]
}

pub fn total(items: &[u32]) -> u32 {
    let mut sum = 0;
    for i in 0..items.len() {
        sum += items[i];
    }
    sum
}

pub fn below(items: &[u32], i: usize) -> u32 {
    if i < items.len() && !items.is_empty() { items[i] + items[0] } else { 0 }
}

pub fn unchecked(items: &[u32]) -> u32 {
    if items.len() > 1 { items[2] } else { 0 }
}

pub fn changed(mut items: Vec<u32>) -> u32 {
    if items.is_empty() {
        return 0;
    }
    items.clear();
    items[0]
}

pub fn either(items: &[u32]) -> u32 {
    if items.len() > 3 || !items.is_empty() { items[3] } else { 0 }
}

pub fn third(items: &[u32]) -> u32 {
    if items.len() < 2 {
        return 0;
    }
    items[2]
}

pub fn after(items: &[u32]) -> u32 {
    let mut n = 0;
    if items.is_empty() {
        n = 1;
    }
    n + items[0]
}

pub fn wrong(items: &[u32]) -> u32 {
    if items.is_empty() { items[0] } else { 0 }
}

pub fn other(items: &[u32]) -> u32 {
    if items.len() != 5 { items[0] } else { 0 }
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  for (const read of ["items[0]", "items[1]", "items[i]", "items[i] + items[0]"]) expect(js).toContain(read);
  expect(js).toContain("$index(items, 2)");
  expect(js).toContain("$index(items, 0)");
  expect(js.match(/\$index/g)?.length).toBe(8);
  const lib = await import(join(dir, "lib.js"));
  expect([lib.first([]), lib.first([4]), lib.second([1, 2]), lib.second([1]), lib.total([1, 2, 3]), lib.below([5, 6], 1), lib.below([5], 3)]).toEqual([undefined, 4, 2, 0, 6, 11, 0]);
  expect(() => lib.unchecked([1, 2])).toThrow();
  expect(() => lib.changed([1])).toThrow();
  for (const unknown of [() => lib.either([1]), () => lib.third([1, 2]), () => lib.after([]), () => lib.wrong([]), () => lib.other([])]) {
    expect(unknown).toThrow();
  }
});

// ADR 0293: a `Cell` a `let` takes apart, read by `get()` before anything
// else runs, is its value in JS's destructuring: react.dev's ErrorMessage
// has `const {message, title} = error;`. Set before it's read, it's read
// where it is.
test("a Cell a let takes apart and reads at once is its value", async () => {
  const dir = fixture("early-gets");
  writeFileSync(join(dir, "lib.rs"), `use std::cell::Cell;

pub struct Failure {
    pub title: Cell<Option<&'static str>>,
    pub message: String,
}

pub fn shown(error: &Failure) -> String {
    let Failure { message, title } = error;
    format!("{}: {}", title.get().unwrap_or("Error"), message)
}

pub fn retitled(error: &Failure) -> Option<&'static str> {
    let Failure { title, .. } = error;
    error.title.set(Some("new"));
    title.get()
}

pub fn twice(error: &Failure) -> usize {
    let Failure { title, .. } = error;
    let mut n = 0;
    for _ in 0..2 {
        n += title.get().map_or(0, |t| t.len());
        error.title.set(None);
    }
    n
}

pub fn looped(error: &Failure) -> usize {
    let Failure { title, .. } = error;
    let mut n = 0;
    let mut turns = 0;
    loop {
        n += title.get().map_or(0, |t| t.len());
        turns += 1;
        if turns == 2 {
            break;
        }
        error.title.set(None);
    }
    n
}

pub fn aliased(error: &Failure) -> (Option<&'static str>, Option<&'static str>) {
    let Failure { title, .. } = error;
    let first = title.get();
    let other = title;
    other.set(Some("set"));
    (first, error.title.get())
}

struct Retitle<'a>(&'a Failure);

impl Drop for Retitle<'_> {
    fn drop(&mut self) {
        self.0.title.set(Some("dropped"));
    }
}

pub fn dropped(error: &Failure) -> Option<&'static str> {
    let Failure { title, .. } = error;
    {
        let _retitle = Retitle(error);
    }
    title.get()
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("const { message, title } = error;");
  expect(js.match(/const \{ title \} = /g)).toBeNull();
  const lib = await import(join(dir, "lib.js"));
  expect([lib.shown({ title: undefined, message: "m" }), lib.shown({ title: "T", message: "m" }), lib.retitled({ title: "old", message: "" }), lib.twice({ title: "ab", message: "" }), lib.looped({ title: "ab", message: "" }), lib.aliased({ title: "T", message: "" }), lib.dropped({ title: "T", message: "" })])
    .toEqual(["Error: m", "T: m", "new", 2, 2, ["T", "set"], "dropped"]);
});

// ADR 0294: `unwrap_unchecked()` is TypeScript's `x!`: the value, unchecked,
// as react.dev's Preview has `iframeRef.current!`. Rust leaves a `None`
// undefined behavior, so JS's `undefined` read on is the program's.
test("unwrap_unchecked is the value itself", async () => {
  const dir = fixture("unwrap-unchecked");
  writeFileSync(join(dir, "lib.rs"), `pub fn first(v: Option<&'static str>) -> &'static str {
    unsafe { v.unwrap_unchecked() }
}

fn nested(v: Option<Option<u32>>) -> Option<u32> {
    unsafe { v.unwrap_unchecked() }
}

pub fn inner() -> (Option<u32>, Option<u32>) {
    (nested(Some(None)), nested(Some(Some(3))))
}

thread_local! {
    static DROPS: std::cell::Cell<u32> = const { std::cell::Cell::new(0) };
}

struct Guard(u32);

impl Drop for Guard {
    fn drop(&mut self) {
        DROPS.with(|d| d.set(d.get() + self.0));
    }
}

pub fn guarded() -> u32 {
    {
        let _guard = unsafe { Some(Guard(5)).unwrap_unchecked() };
    }
    DROPS.with(|d| d.get())
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("return v;");
  expect(js).not.toContain("$unwrap");
  const lib = await import(join(dir, "lib.js"));
  expect([lib.first("a"), lib.inner(), lib.guarded()]).toEqual(["a", [undefined, 3], 5]);
});

// `get_unchecked(i)` of a slice is its item, `xs[i]`, unchecked, as JS reads
// one (ADR 0294): react.dev's NavigationBar reads `entry.contentBoxSize[0]`.
test("get_unchecked is the item itself", async () => {
  const dir = fixture("get-unchecked");
  writeFileSync(join(dir, "lib.rs"), `pub fn first(xs: &[u32]) -> u32 {
    unsafe { *xs.get_unchecked(0) }
}
pub fn second(names: &Vec<String>) -> &str {
    unsafe { names.get_unchecked(1) }
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("return xs[0];");
  expect(js).toContain("return names[1];");
  const lib = await import(join(dir, "lib.js"));
  expect([lib.first([4, 5]), lib.second(["a", "b"])]).toEqual([4, "b"]);
});

// ADR 0298: an Option whose value is never falsy, an object, an array or a
// function, is tested by its truth, as react.dev's Preview writes `{error &&
// ..}`, `!error` and `rawError && rawError.message`; and `o == Some(true)` of
// an `Option<bool>` is `o` in a test, `if (message.firstLoad)`. A number's
// or a string's, which can be falsy, is tested against null still.
test("an Option of an object is tested by its truth", async () => {
  const dir = fixture("truthy-options");
  writeFileSync(join(dir, "lib.rs"), `pub struct Failure {
    pub message: String,
}

pub fn shown(error: Option<&Failure>) -> bool {
    error.is_some()
}

pub fn hidden(error: Option<&Failure>) -> bool {
    error.is_none()
}

pub fn tested(error: Option<&Failure>) -> u32 {
    if error.is_some() { 1 } else { 0 }
}

pub fn chained(error: Option<&Failure>) -> bool {
    if let Some(e) = error
        && e.message == "x"
    {
        return true;
    }
    false
}

pub fn mapped(error: Option<&Failure>) -> Option<usize> {
    error.map(|e| e.message.len())
}

pub fn number(n: Option<u32>) -> bool {
    n.is_some()
}

pub fn text(s: Option<&str>) -> bool {
    s.is_none()
}

pub fn first(f: Option<bool>) -> u32 {
    if f == Some(true) { 1 } else { 0 }
}

pub fn first_value(f: Option<bool>) -> bool {
    f == Some(true)
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  for (const written of ["return !!error;", "return !error;", "if (error) {", "if (error && error.message === \"x\") {",
    "return error ? $byteLen(error.message) : undefined;", "return n != null;", "return s == null;", "if (f) {", "return !!f;"]) {
    expect(js).toContain(written);
  }
  const lib = await import(join(dir, "lib.js"));
  const failure = { message: "x" };
  expect([lib.shown(failure), lib.shown(undefined), lib.hidden(undefined), lib.tested(failure), lib.chained(failure), lib.chained(undefined),
    lib.mapped(failure), lib.number(0), lib.text(""), lib.first(true), lib.first(undefined), lib.first_value(false)])
    .toEqual([true, false, true, 1, true, false, 1, true, false, 1, 0, false]);
  // A JS value of unknown kind may be `""` or `0`: tested against null still.
  writeFileSync(join(dir, "unknown.rs"), `use js::Unknown;

pub fn has(value: Option<&Unknown>) -> bool {
    value.is_some()
}
`);
  run([compiler, join(dir, "unknown.rs"), "-o", join(dir, "unknown.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  const unknown = readFileSync(join(dir, "unknown.js"), "utf8");
  expect(unknown).toContain("return value != null;");
  expect((await import(join(dir, "unknown.js"))).has("")).toBe(true);
});

// ADR 0298: a `const` only tests read is a test itself, as react.dev's
// Preview has `const hideContent = error || !iframeComputedHeight ||
// !bundlerIsReady;`: what it's made of is read by its truth. One read as a
// value keeps its boolean.
test("a const only tests read is read by its truth", async () => {
  const dir = fixture("tested-consts");
  writeFileSync(join(dir, "lib.rs"), `pub struct Failure {
    pub message: String,
}

pub fn shown(error: Option<&Failure>, ready: bool) -> u32 {
    let hide = error.is_some() || !ready;
    let mut n = 0;
    if hide {
        n += 1;
    }
    if !hide && ready {
        n += 10;
    }
    n + if hide { 100 } else { 0 }
}

pub fn kept(error: Option<&Failure>, ready: bool) -> (bool, u32) {
    let has = error.is_some() || !ready;
    (has, if has { 1 } else { 0 })
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("const hide = error || !ready;");
  expect(js).toContain("const has = !!error || !ready;");
  const lib = await import(join(dir, "lib.js"));
  expect([lib.shown({ message: "" }, true), lib.shown(undefined, true), lib.shown(undefined, false), lib.kept({ message: "" }, true)])
    .toEqual([101, 10, 101, [true, 1]]);
});

// ADR 0299: a `return` of `undefined` is a bare `return;`, as react.dev's
// NavigationBar ends its effect, `} else { return; }`.
test("a return of undefined is a bare return", async () => {
  const dir = fixture("bare-return");
  writeFileSync(join(dir, "lib.rs"), `pub fn halved(n: u32) -> Option<u32> {
    if n % 2 == 1 {
        return None;
    }
    Some(n / 2)
}

pub fn cleanup(on: bool) -> Option<fn()> {
    fn noop() {}
    if on { Some(noop) } else { None }
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).not.toContain("return undefined;");
  expect(js.match(/return;/g)?.length).toBe(2);
  const lib = await import(join(dir, "lib.js"));
  expect([lib.halved(3), lib.halved(4), lib.cleanup(false)]).toEqual([undefined, 2, undefined]);
});
