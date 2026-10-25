// What the compiler writes of a `match` and of patterns: arms in written
// order, guards, or-patterns, `matches!`, a `let`-`else` as a guard, and a
// pattern's bindings destructured. Each test is a program of its own, compiled
// and run.

import { beforeAll, expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { buildReact, compiler, fixture, root, run, target } from "./support";

beforeAll(buildReact, 600_000);

// A pattern's bindings from MIR (ADR 0364), each a part read on its own
// line, are destructured, as a person writes them: of a value made for
// them, or of a variable.
test("a pattern's bindings from MIR are destructured", async () => {
  const dir = fixture("mir-destructured");
  writeFileSync(join(dir, "lib.rs"), `pub struct P { pub a: u32, pub b: u32 }
fn pair(n: u32) -> (u32, u32) { (n, n + 1) }
pub fn split(n: u32) -> u32 {
    let (a, b) = pair(n);
    a * b
}
pub fn both(t: (u32, u32)) -> u32 {
    let (a, b) = t;
    a * b + a
}
pub fn fields(p: P) -> u32 {
    let P { a, b } = p;
    a + b
}
pub fn ends() -> u32 {
    let [first, .., last] = [1, 2, 3, 4];
    first * 10 + last
}
pub fn made(n: u32) -> u32 {
    let (x, y) = (pair(n).1, n + 1);
    x * y
}
pub struct Q { pub dir: Option<u32>, pub name: Option<String> }
fn join(a: Option<&str>, b: Option<&str>) -> String { format!("{a:?}{b:?}") }
pub fn label(q: &Q) -> String {
    let dir = q.dir;
    join(q.name.as_deref(), matches!(dir, Some(1)).then_some("one"))
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain("  const [a, b] = pair(n);\n");
  expect(js).toContain("  const [a, b] = t;\n");
  expect(js).toContain("  const { a, b } = p;\n");
  // A part read where it's used isn't one a pattern names.
  expect(js).toContain("  const dir = q.dir;\n  return join(q.name, ");
  // Of an array made for them, each part itself.
  expect(js).toContain("  const first = 1;\n  const last = 4;\n");
  expect(js).toContain("  const x = pair(n)[1];\n  const y = (n + 1) >>> 0;\n");
  const lib = await import(join(dir, "mir.js"));
  expect([lib.split(2), lib.both([2, 3]), lib.fields({ a: 1, b: 2 }), lib.ends(), lib.made(2)]).toEqual([6, 8, 3, 14, 9]);
});

// A \`let … else\` from MIR (ADR 0364), its tests each leaving to one panic,
// is a guard, \`if (a || b) { throw … }\`, then the rest; what it binds
// keeps its name; \`nth(n)\` of a string's parts is \`[n]\`.
test("a let-else from MIR is a guard, then the rest", async () => {
  const dir = fixture("mir-let-else");
  writeFileSync(join(dir, "lib.rs"), `pub struct Title {
    pub kind: &'static str,
    pub id: u32,
}
pub enum Node {
    Text(&'static str),
    Element(Title),
}
pub fn heading(children: &[Node], kind: &str, path: &str) -> u32 {
    let Some(Node::Element(first)) = children.first() else {
        panic!("{kind} is missing a title");
    };
    if first.kind != "h4" {
        panic!("{kind} is missing an h4");
    }
    let hash = path.split('#').nth(1).unwrap_or("");
    first.id * 2 + hash.len() as u32
}
pub fn sample(titled: bool) -> Vec<Node> {
    match titled {
        true => vec![Node::Element(Title { kind: "h4", id: 3 })],
        false => vec![Node::Text("t")],
    }
}
#[rust_js::untagged]
pub enum Part<'a> {
    Text(&'static str),
    Element(&'a Title),
}
pub fn element_id(parts: &[Part]) -> u32 {
    let Some(Part::Element(first)) = parts.first() else {
        return 0;
    };
    first.id + first.id
}
pub fn first_id(items: &[Title]) -> u32 {
    let Some(first) = items.first() else {
        return 0;
    };
    first.id + first.id
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).not.toMatch(/bb\d+:/);
  expect(js).toContain("    throw new Error(`${kind} is missing a title`);\n  }\n");
  expect(js).toContain('  const hash = path.split("#")[1] ?? "";\n');
  expect(js).toContain("  const first = items[0];\n");
  {
    expect(js).toContain('  const first = parts[0];\n  if (first == null || typeof first !== "object") {\n    return 0;\n  }\n');
  }
  // A \`vec!\` a branch gives back, its box made, then filled: the array.
  expect(js).toContain('  if (titled) {\n    return [{ TAG: "Element", _0: { kind: "h4", id: 3 } }];\n  }\n  return [{ TAG: "Text", _0: "t" }];\n');
  const lib = await import(join(dir, "mir.js"));
  expect([lib.heading(lib.sample(true), "k", "a#bc"), lib.firstId([{ kind: "p", id: 4 }]), lib.firstId([]), lib.elementId([{ kind: "p", id: 5 }]), lib.elementId(["t"])]).toEqual([8, 8, 0, 10, 0]);
  expect(() => lib.heading(lib.sample(false), "k", "")).toThrow("k is missing a title");
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
    matches!(classify(js::get(value, "mdxName")), Kind::String("inlineCode"))
}
pub fn is_text(value: Option<&Unknown>) -> bool {
    matches!(classify(value), Kind::String("inlineCode"))
}
pub fn kind_is(value: &Unknown) -> bool {
    matches!(classify(Some(value)), Kind::String("inlineCode"))
}
pub fn listed(value: Option<&Unknown>) -> u32 {
    if let Some(value) = value
        && matches!(classify(Some(value)), Kind::Array(_))
    {
        1
    } else {
        0
    }
}
pub fn is_string(value: &Unknown) -> bool {
    matches!(classify(js::get(value, "mdxName")), Kind::String(_))
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
  expect(js).toContain('export function isCode(value) {\n  return value.mdxName === "inlineCode";\n}');
  expect(js).toContain('export function isText(value) {\n  return value === "inlineCode";\n}');
  expect(js).toContain('export function kindIs(value) {\n  return value === "inlineCode";\n}');
  // An array is never `null`: `Array.isArray` alone says it.
  expect(js).toContain("export function listed(value) {\n  return Array.isArray(value) ? 1 : 0;\n}");
  expect(js).toContain('export function isString(value) {\n  return typeof value.mdxName === "string";\n}');
  const lib = await import(join(dir, "lib.js"));
  const values = [{ mdxName: "inlineCode" }, { mdxName: "pre" }, { mdxName: null }, {}, { mdxName: 5 }];
  expect(values.map(lib.isCode)).toEqual([true, false, false, false, false]);
  expect(values.map((v) => lib.isString(v))).toEqual([true, true, false, false, false]);
  expect(["inlineCode", "pre", 5, null, undefined].map((v) => lib.isText(v))).toEqual([true, false, false, false, false]);
  expect(["inlineCode", "pre", 5, {}].map((v) => lib.kindIs(v))).toEqual([true, false, false, false]);
  expect([lib.isThree(3), lib.isThree(4)]).toEqual([true, false]);
  expect([[1], undefined, null, "a"].map((v) => lib.listed(v))).toEqual([1, 0, 0, 0]);
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

// A parameter taken apart reads its \`Cell\` where it is, a field JS can set.
pub fn bumped(Counter { count, label }: Counter) -> u32 {
    count.set(count.get() + label.len() as u32);
    count.get()
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
  expect(lib.bumped({ count: 1, label: "ab" })).toBe(3);
});

// A `match` of a fieldless enum whose arms each give one table's field named
// as their variant is the table read by the value, as react.dev's
// ExpandableCallout reads `variantMap[type]`; an arm of another field is
// the `switch` of a `match` of constants (ADR 0629). The corpus's
// `match_index` runs it beside native Rust.
test("a match giving a table's field named as each variant reads the table by it", () => {
  const dir = fixture("match-index");
  writeFileSync(join(dir, "lib.rs"), readFileSync(join(root, "test/corpus/match_index.rs"), "utf8").replace("fn main()", "pub fn main()"));
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("function variant(kind) {\n  return VARIANTS[kind];\n}");
  expect(js).toContain("function picked(i) {\n  return VARIANTS[pick(i)];\n}");
  expect(js).toContain("const chosen = VARIANTS[kind];\n  return chosen.title;");
  expect(js).toContain('  switch (kind) {\n    case "note":\n      return VARIANTS.pitfall;\n');
  expect(js).toContain('    case "pitfall":\n      return OTHER.pitfall;\n');
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
  expect(js).toContain('return name ? name.toUpperCase() : "anon";');
  expect(js).toContain("items = items != null && items.length !== 0 ? items : undefined;\n  if (!items) {\n    return 0;\n  }\n  return $index(items, 0);");
  const lib = await import(join(dir, "lib.js"));
  expect([lib.link("/a"), lib.link(""), lib.link(undefined)]).toEqual(["/A", "none", "none"]);
  expect([lib.named("ann"), lib.named(""), lib.named(undefined)]).toEqual(["ANN", "anon", "anon"]);
  expect([lib.listed([7]), lib.listed([]), lib.listed(undefined)]).toEqual([7, 0, 0]);
  expect([lib.later("/a"), lib.later("")]).toEqual(["/a changed", "none"]);
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

// ADR 0590: a \`match\` of a number by the crate's constants tests each by
// its name, as React's scheduler's \`switch (priorityLevel)\` does, its own
// module's or another's, imported; a value written another way too is its
// number. A comment before an arm is before its body.
test("a match of constants cases each by its name, its comments in its arms", async () => {
  const dir = fixture("const-patterns");
  writeFileSync(join(dir, "lib.rs"), `pub mod levels {
    pub const IMMEDIATE: u8 = 1;
    pub const IDLE: u8 = 5;
}

use levels::{IDLE, IMMEDIATE};

const NORMAL: u8 = 3;

pub fn timeout(level: u8) -> f64 {
    match level {
        // Times out immediately
        IMMEDIATE => -1.0,
        // Normal
        NORMAL => 5000.0,
        // Never times out
        IDLE => 1073741823.0,
        // Eventually times out
        _ => 10000.0,
    }
}

pub fn both(a: u8, b: u8) -> bool {
    matches!(a, NORMAL) && matches!(b, 3 | 4)
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain('import { IDLE, IMMEDIATE } from "./levels.js";');
  // A switch of them (ADR 0629), each arm's comment before its statements.
  expect(js).toContain("  switch (level) {\n    case IMMEDIATE:\n      // Times out immediately\n      return -1;\n");
  expect(js).toContain("    case NORMAL:\n      // Normal\n      return 5000;\n");
  expect(js).toContain("    case IDLE:\n      // Never times out\n");
  expect(js).toContain("a === 3");
  const lib = await import(join(dir, "lib.js"));
  expect([1, 3, 5, 9].map(lib.timeout)).toEqual([-1, 5000, 1073741823, 10000]);
  expect([lib.both(3, 4), lib.both(3, 5), lib.both(1, 3)]).toEqual([true, false, false]);
});

// ADR 0614: a match arm's pattern of several parts, \`(Some(x), Some(y))\`,
// is one test of each, \`if (p != null && q != null)\`, as its arm is one
// pattern; a drop flag's test joined with it keeps the flag's name.
test("a match arm's pattern of parts is one test of each", async () => {
  const dir = fixture("pattern-tests");
  writeFileSync(join(dir, "lib.rs"), `pub fn both(p: Option<f64>, q: Option<f64>, other: f64) -> f64 {
    match (p, q) {
        (Some(x), Some(y)) => x + y,
        _ => other,
    }
}

pub fn numbers(x: Option<&js::Unknown>, y: Option<&js::Unknown>) -> f64 {
    match (js::classify(x), js::classify(y)) {
        (js::Kind::Number(x), js::Kind::Number(y)) => x + y,
        _ => 0.0,
    }
}

// A let chain's \`!= null\` of what's then a number, which that implies,
// past a binding between, one of Rust's data that a \`&mut\` changes.
pub struct Pair { pub left: Option<u32>, pub right: u32 }

pub fn weighed(x: Option<&js::Unknown>, pair: &mut Pair) -> f64 {
    pair.right += 1;
    if let Some(v) = x
        && let Some(left) = pair.left
        && let js::Kind::Number(n) = js::classify(Some(v))
    {
        return n + f64::from(left);
    }
    0.0
}

// \`typeof null\` is "object": its \`!= null\` stays.
pub fn object(x: Option<&js::Unknown>) -> bool {
    match x {
        Some(v) if js::type_of(Some(v)) == "object" => true,
        _ => false,
    }
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("  if (p != null && q != null) {\n");
  // A test a later one implies, \`x != null\` of \`typeof x === "number"\`'s, is gone.
  expect(js).toContain('  if (typeof x === "number" && typeof y === "number") {\n');
  expect(js).toContain('  if (pair.left != null && typeof x === "number") {\n');
  const lib = await import(join(dir, "lib.js"));
  expect([lib.both(1, 2, 0), lib.both(1, undefined, 9), lib.both(undefined, 2, 9)]).toEqual([3, 9, 9]);
  expect([lib.numbers(1, 2), lib.numbers(1, "2"), lib.numbers(null, 2)]).toEqual([3, 0, 0]);
  expect([lib.object(null), lib.object({}), lib.object(1)]).toEqual([false, true, false]);
  expect([lib.weighed(4, { left: 1, right: 0 }), lib.weighed("4", { left: 1, right: 0 }), lib.weighed(4, { right: 0 })]).toEqual([5, 0, 0]);
});

test("a `match` of an `Option`'s strings from MIR is the `switch` or `else if` chain a person writes", async () => {
  const dir = fixture("mir-option-string-match");
  writeFileSync(join(dir, "lib.rs"), `pub fn flagged(flag: Option<&str>, mut arg: String) -> String {
    match flag {
        Some("s") => arg += "",
        Some("d") | Some("i") => arg = arg.trim().to_string(),
        Some("f") => arg = arg.to_uppercase(),
        _ => {}
    }
    arg
}
pub fn named(flag: Option<&str>, v: &mut Vec<u32>) {
    match flag {
        Some("s") => v.push(1),
        Some("d") => v.push(2),
        _ => v.push(3),
    }
}
pub fn either(flag: Option<&str>, v: &mut Vec<u32>) {
    match flag {
        Some("d") | Some("i") => v.push(1),
        _ => v.push(2),
    }
}
pub fn nested(flag: Option<&str>, v: &mut Vec<u32>) {
    if flag.is_some() {
        if flag == Some("s") {
            v.push(1);
        } else {
            v.push(2);
        }
    }
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  // Of three cases or more, a `switch`, a `Some`'s constants its cases
  // (ADRs 0629, 0634).
  expect(js).toContain(`export function flagged(flag, arg) {
  switch (flag) {
    case "s":
      arg += "";
      break;
    case "d":
    case "i":
      arg = $trim(arg);
      break;
    case "f":
      arg = arg.toUpperCase();
  }
  return arg;
}`);
  expect(js).toContain(`export function named(flag, v) {
  switch (flag) {
    case "s":
      v.push(1);
      break;
    case "d":
      v.push(2);
      break;
    default:
      v.push(3);
  }
}`);
  // An or-pattern's cases, `x === "d" || x === "i"`, each say `x != null`,
  // so the `None` arm is the `default`, not a test of its own.
  expect(js).toContain(`export function either(flag, v) {
  switch (flag) {
    case "d":
    case "i":
      v.push(1);
      break;
    default:
      v.push(2);
  }
}`);
  expect(js).toContain(`export function nested(flag, v) {
  if (flag != null) {
    if (flag === "s") {
      v.push(1);
    } else {
      v.push(2);
    }
  }
}`);
  const lib = await import(join(dir, "mir.js"));
  expect([lib.flagged("d", " x "), lib.flagged("f", "x"), lib.flagged(undefined, "y"), lib.flagged("q", "z")]).toEqual(["x", "X", "y", "z"]);
  // A guard its chain's last `else` doesn't say stays.
  const pushed: number[] = [];
  lib.nested(undefined, pushed);
  lib.nested("q", pushed);
  lib.named(undefined, pushed);
  lib.named("d", pushed);
  expect(pushed).toEqual([2, 3, 2]);
});

test("a let-else of what `filter` keeps from MIR tests the value", async () => {
  const dir = fixture("mir-filtered-let-else");
  writeFileSync(join(dir, "lib.rs"), `pub fn filtered(href: Option<&str>) -> String {
    let Some(h) = href.filter(|h| !h.is_empty()) else {
        return "none".to_string();
    };
    if h.starts_with("x") { "a".to_string() } else { h.to_string() }
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain(`export function filtered(href) {
  if (!href) {
    return "none";
  }
  return href.startsWith("x") ? "a" : href;
}`);
  const lib = await import(join(dir, "mir.js"));
  expect([lib.filtered(undefined), lib.filtered(""), lib.filtered("xy"), lib.filtered("q")]).toEqual(["none", "none", "a", "q"]);
});

test("a `let` pattern of one field from MIR destructures", async () => {
  const dir = fixture("mir-one-field-pattern");
  writeFileSync(join(dir, "lib.rs"), `pub struct Collapse { pub get_props: u32, pub other: u32 }
fn use_collapse(n: u32) -> Collapse { Collapse { get_props: n, other: 2 } }
pub fn one(n: u32) -> u32 {
    let Collapse { get_props, .. } = use_collapse(n);
    get_props + 1
}
pub fn field(n: u32) -> u32 {
    let get_props = use_collapse(n).get_props;
    get_props + 1
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain("  const { getProps } = useCollapse(n);\n");
  expect(js).toContain("  const getProps = useCollapse(n).getProps;\n");
});

test("a let-else from MIR whose rest is long, and drops what it owns, is a guard first", async () => {
  const dir = fixture("mir-let-else-dropping");
  writeFileSync(join(dir, "lib.rs"), `pub fn named(href: Option<&str>, items: Vec<String>) -> String {
    let Some(href) = href.filter(|h| !h.is_empty()) else {
        return format!("none{}", items.len());
    };
    format!("[{}{}]", if href.starts_with("x") { "a" } else { href }, items.len())
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain(`export function named(href, items) {
  if (!href) {
    return \`none\${items.length}\`;
  }
  return \`[\${href.startsWith("x") ? "a" : href}\${items.length}]\`;
}`);
});

// A nested pattern of an item a closure reads of what it captured, from
// MIR, takes it apart once, as react.dev's Preview reads its first lint
// error: \`const { line, column, message } = errors[0];\`.
test("a nested pattern of a captured item from MIR takes it apart once", async () => {
  const dir = fixture("mir-captured-pattern");
  writeFileSync(join(dir, "lib.rs"), `pub struct Message { pub line: u32, pub column: u32, pub message: String, pub rule: String }
pub struct Lint {
    #[cfg_attr(rust_js, rust_js::flatten)]
    pub item: &'static Message,
    pub severity: &'static str,
}
pub fn first(errors: &'static Vec<Lint>) -> impl Fn() -> Option<String> {
    move || {
        if errors.is_empty() {
            None
        } else {
            let Lint { item: Message { line, column, message, .. }, .. } = &errors[0];
            Some(format!("{line}:{column} - {message}"))
        }
    }
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain("const { line, column, message } = errors[0]");
  const lib = await import(join(dir, "mir.js"));
  expect(lib.first([{ line: 1, column: 2, message: "m", rule: "r", severity: "error" }])()).toBe("1:2 - m");
});

// A string's or-pattern from MIR is one test of each, \`s === "a" || …\`,
// its arm first: not a test failing for each, one inside another, the arm
// after them all.
test("a string's or-pattern from MIR is an or of its tests", async () => {
  const dir = fixture("mir-string-or");
  writeFileSync(join(dir, "lib.rs"), `pub fn kind(s: &str) -> &'static str {
    match s {
        "a" | "e" | "i" => "vowel",
        "" => "empty",
        x if x.len() > 1 => "word",
        _ => "consonant",
    }
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain('  if (s === "a" || s === "e" || s === "i") {\n    return "vowel";\n  }\n');
  // What an arm binds of the value whole is the value.
  expect(js).toContain("  if ($byteLen(s) > 1) {\n");
  const lib = await import(join(dir, "mir.js"));
  expect(["a", "i", "", "z", "zz"].map(lib.kind)).toEqual(["vowel", "vowel", "empty", "consonant", "word"]);
});

// A range pattern's tests from MIR read the value first, \`k >= 2\`, and a
// bound the type has anyway, \`0 <= k\` of a \`u8\`, none; a comparison the
// program wrote stays as written.
test("a range pattern's tests from MIR read the value first", async () => {
  const dir = fixture("mir-range-tests");
  writeFileSync(join(dir, "lib.rs"), `pub fn big(k: u8) -> bool { matches!(k, 2..) }
pub fn mid(k: u8) -> bool { matches!(k, 2..=5) }
pub fn grade(k: u8) -> &'static str { match k { 0..=4 => "low", 5..10 => "mid", _ => "high" } }
pub fn written(k: u8) -> bool { 2 <= k }
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain("export function big(k) {\n  return k >= 2;\n}");
  expect(js).toContain("export function mid(k) {\n  return k >= 2 && k <= 5;\n}");
  expect(js).toContain('  if (k <= 4) {\n    return "low";\n  }\n');
  expect(js).toContain("export function written(k) {\n  return 2 <= k;\n}");
  const lib = await import(join(dir, "mir.js"));
  expect([lib.big(1), lib.big(2), lib.mid(5), lib.mid(6), lib.grade(0), lib.grade(7), lib.grade(10)]).toEqual([false, true, true, false, "low", "mid", "high"]);
});

// react.dev's Console clears its logs when Sandpack starts for the first
// time or refreshes: two `matches!`s joined by `||` are one `if (a || b)`,
// not the first held in a `const` and an empty `if` of the negated tests.
test("react.dev's Console clears its logs on an or of two matches", async () => {
  const dir = fixture("mir-console-or");
  writeFileSync(join(dir, "lib.rs"), `#![allow(non_snake_case)]
#[cfg_attr(rust_js, rust_js::tag = "type")]
pub enum SandpackMessage {
    #[cfg_attr(rust_js, rust_js::name = "start")]
    Start {
        first_load: Option<bool>,
    },
    #[cfg_attr(rust_js, rust_js::name = "refresh")]
    Refresh,
    #[cfg_attr(rust_js, rust_js::name = "console")]
    Console { log: Vec<String>, codesandbox: Option<bool> },
    #[cfg_attr(rust_js, rust_js::otherwise)]
    Other(&'static str),
}

pub fn listener(setLogs: &'static dyn Fn(Vec<String>)) -> Box<dyn Fn(&'static SandpackMessage)> {
    Box::new(move |message: &'static SandpackMessage| {
        if matches!(message, SandpackMessage::Start { first_load: Some(true) })
            || matches!(message, SandpackMessage::Refresh)
        {
            setLogs(Vec::new());
        }
        if let SandpackMessage::Console {
            log,
            codesandbox: Some(true),
        } = message
        {
            setLogs(log.clone());
        }
    })
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain('    if ((message.type === "start" && message.firstLoad) || message.type === "refresh") {\n      setLogs([]);\n    }\n');
  expect(js).not.toContain("const value");
  const lib = await import(join(dir, "mir.js"));
  const set: string[][] = [];
  const listen = lib.listener((logs: string[]) => set.push(logs));
  for (const message of [{ type: "start", firstLoad: true }, { type: "start" }, { type: "refresh" }, { type: "console", log: ["a"], codesandbox: true }, { type: "console", log: ["b"] }, { type: "done" }]) listen(message);
  expect(set).toEqual([[], [], ["a"]]);
});

// A \`match\`'s arms from MIR are tested in the order they're written, an
// arm that tests a field first, \`Done { ok: true }\`, where it's written,
// not where its test is spanned, the \`match\`.
test("a match's arms from MIR are in written order, of fields tested too", async () => {
  const dir = fixture("mir-arms-ordered");
  writeFileSync(join(dir, "lib.rs"), `#[derive(Clone, Copy)]
pub enum Status { Idle, Loading(u8), Done { ok: bool }, Failed(&'static str) }
pub fn view(s: Status) -> String {
    match s {
        Status::Idle => "idle".to_string(),
        Status::Loading(p) => format!("{p}%"),
        Status::Done { ok: true } => "ok".to_string(),
        Status::Done { ok: false } => "not ok".to_string(),
        Status::Failed(why) => why.to_string(),
    }
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  const at = (s: string) => js.indexOf(s);
  expect([at('"Idle"') < at('"Loading"'), at('"Loading"') < at('"Done"')]).toEqual([true, true]);
  const lib = await import(join(dir, "mir.js"));
  const all = ["Idle", { TAG: "Loading", _0: 5 }, { TAG: "Done", ok: true }, { TAG: "Done", ok: false }, { TAG: "Failed", _0: "x" }];
  expect(all.map(lib.view)).toEqual(["idle", "5%", "ok", "not ok", "x"]);
});

// Each \`?\` from MIR is a guard where it's written, \`if (result.TAG ===
// "Err") return result;\`, the rest after it: not the rest nested in a
// test that it's \`Ok\`.
test("a function's ?s from MIR are guards, the rest after them", async () => {
  const dir = fixture("mir-tried-guards");
  writeFileSync(join(dir, "lib.rs"), `pub fn parse2(a: &str, b: &str) -> Result<i32, std::num::ParseIntError> {
    let x = a.parse::<i32>()?;
    let y = b.parse::<i32>()?;
    Ok(x + y)
}
pub fn opt2(a: Option<i32>, b: Option<i32>) -> Option<i32> {
    let x = a?;
    let y = b?;
    Some(x + y)
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain(`  const result = $parseInt(a, -2147483648, 2147483647);
  if (result.TAG === "Err") {
    return result;
  }
  const x = result._0;
  const result$1 = $parseInt(b, -2147483648, 2147483647);
  if (result$1.TAG === "Err") {
    return result$1;
  }
  const y = result$1._0;
  return { TAG: "Ok", _0: (x + y) | 0 };`);
  expect(js).toContain("  if (a == null) {\n    return;\n  }\n  if (b == null) {\n    return;\n  }\n  return (a + b) | 0;\n");
  const lib = await import(join(dir, "mir.js"));
  expect([lib.parse2("1", "2"), lib.parse2("x", "2").TAG, lib.opt2(1, 2), lib.opt2(1, undefined)]).toEqual([{ TAG: "Ok", _0: 3 }, "Err", 3, undefined]);
});

test("a match guard's binding from MIR is what it binds", async () => {
  const dir = fixture("mir-guard-binding");
  writeFileSync(join(dir, "lib.rs"), `pub fn titled(error_code: Option<&str>) -> String {
    match error_code {
        Some(code) if !code.is_empty() => format!("#{code}"),
        _ => "none".to_string(),
    }
}
// A variant's field the guard reads, and the arms after: each read where
// it is, \`k.level\`, as nothing changes \`k\`.
#[derive(Clone, Copy)]
pub enum Kind { Note, Warn { level: u8 }, Tip(&'static str) }
pub fn icon(k: Kind) -> &'static str {
    match k {
        Kind::Note => "i",
        Kind::Warn { level } if level > 1 => "!!",
        Kind::Warn { .. } => "!",
        Kind::Tip(t) => t,
    }
}
pub fn doubled(k: Kind) -> u8 {
    match k {
        Kind::Warn { level } if level > 1 => level * 2,
        _ => 0,
    }
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain('export function titled(errorCode) {\n  if (errorCode) {\n    return `#${errorCode}`;\n  }\n  return "none";\n}');
  expect(js).toContain("  return k._0;\n");
  expect(js).toContain("    return (k.level * 2) & 255;\n");
  expect(js).not.toContain("const level");
  const lib = await import(join(dir, "mir.js"));
  expect([lib.icon({ TAG: "Warn", level: 2 }), lib.icon({ TAG: "Warn", level: 1 }), lib.icon({ TAG: "Tip", _0: "t" }), lib.doubled({ TAG: "Warn", level: 3 })]).toEqual(["!!", "!", "t", 6]);
});

test("a let-else from MIR inside a branch, its `else` a `return`, is a guard there", async () => {
  const dir = fixture("mir-inner-let-else");
  writeFileSync(join(dir, "lib.rs"), `pub struct Nav { pub offset: Option<u32> }
pub fn scroll(pos: usize, navs: &[Nav], out: &mut Vec<u32>) {
    if pos < navs.len() {
        let Some(offset) = navs[pos].offset else {
            return;
        };
        out.push(offset);
        out.push(offset + 1);
    }
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain(`  if (pos < navs.length) {
    const offset = navs[pos].offset;
    if (offset == null) {
      return;
    }
    out.push(offset);
    out.push((offset + 1) >>> 0);
  }`);
});

// An `async fn`'s parameter pattern, as react.dev's errors page takes
// `{ params }` of Next.js's context: taken apart where it's given, not
// `__arg0` and a `const params = __arg0.params;`.
test("an async fn's parameter pattern from MIR is taken apart where it's given", async () => {
  const dir = fixture("async-param-pattern");
  writeFileSync(join(dir, "lib.rs"), `pub struct Ctx {
    pub params: Option<String>,
    pub locale: Option<String>,
}
pub async fn props(Ctx { params, .. }: Ctx) -> String {
    params.unwrap_or_default()
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain('export async function props({ params }) {\n  return params ?? "";\n}');
  const lib = await import(join(dir, "mir.js"));
  expect([await lib.props({ params: "a" }), await lib.props({})]).toEqual(["a", ""]);
});

// A match arm's guard, `Some(path) if !skip`, is one test with its pattern,
// as react.dev's Breadcrumbs keeps a crumb: `c.path != null && !(..)`.
test("a match arm's guard from MIR is one test with its pattern", async () => {
  const dir = fixture("guarded-arm-test");
  writeFileSync(join(dir, "lib.rs"), `pub struct Crumb {
    pub path: Option<String>,
    pub skip: Option<bool>,
    pub title: String,
}
pub fn crumbs(cs: &[Crumb]) -> Vec<Option<String>> {
    cs.iter()
        .map(|c| match &c.path {
            Some(path) if !c.skip.unwrap_or_default() => Some(format!("{path}{}", c.title)),
            _ => None,
        })
        .collect()
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain("    if (c.path != null && !c.skip) {\n      return c.path + c.title;\n    }\n");
  const lib = await import(join(dir, "mir.js"));
  expect(lib.crumbs([{ path: "/a", title: "A" }, { path: "/b", skip: true, title: "B" }, { title: "C" }])).toEqual(["/aA", undefined, undefined]);
});

// A `let` pattern of a variable the program names takes it apart, one
// field too: `const { message } = error;`, as react.dev's ErrorMessage
// reads its error.
test("a let pattern of a named variable takes it apart, one field too", async () => {
  const dir = fixture("let-pattern-one");
  writeFileSync(join(dir, "lib.rs"), `pub struct E {
    pub message: String,
    pub title: Option<String>,
}
pub fn one(error: &E) -> String {
    let E { message, .. } = error;
    message.clone()
}
pub fn written(error: &E) -> String {
    let message = &error.message;
    message.clone()
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain("export function one(error) {\n  const { message } = error;\n");
  // One the program wrote as a field's, as it is.
  expect(js).toContain("export function written(error) {\n  const message = error.message;\n");
  const lib = await import(join(dir, "mir.js"));
  expect([lib.one({ message: "m" }), lib.written({ message: "w" })]).toEqual(["m", "w"]);
});

// A match of more than two arms as a value is a chain of its arms in the
// order written, each tested as it is, `variant === "foundation"`, not
// `!==` with the rest first, as react.dev's Tag picks its name and classes.
test("a match's arms as values chain in written order", async () => {
  const dir = fixture("match-chain");
  writeFileSync(join(dir, "lib.rs"), `#[derive(Clone, Copy)]
pub enum RouteTag {
    #[cfg_attr(rust_js, rust_js::name = "foundation")]
    Foundation,
    #[cfg_attr(rust_js, rust_js::name = "intermediate")]
    Intermediate,
    #[cfg_attr(rust_js, rust_js::name = "advanced")]
    Advanced,
    #[cfg_attr(rust_js, rust_js::name = "experimental")]
    Experimental,
    #[cfg_attr(rust_js, rust_js::name = "deprecated")]
    Deprecated,
}
pub fn tag(variant: RouteTag, text: Option<&'static str>) -> String {
    let (name, classes) = match variant {
        RouteTag::Foundation => ("Foundation", "bg-yellow-50 text-white"),
        RouteTag::Intermediate => ("Intermediate", "bg-purple-40 text-white"),
        RouteTag::Advanced => ("Advanced", "bg-green-40 text-white"),
        RouteTag::Experimental => ("Experimental", "bg-ui-orange text-white"),
        RouteTag::Deprecated => ("Deprecated", "bg-red-40 text-white"),
    };
    format!("{} {classes}", text.unwrap_or(name))
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain('variant === "foundation"');
  expect(js).not.toContain("!==");
  expect(js.indexOf('"Foundation"')).toBeLessThan(js.indexOf('"Deprecated"'));
  const lib = await import(join(dir, "mir.js"));
  expect([lib.tag("advanced", undefined), lib.tag("deprecated", "D")]).toEqual(["Advanced bg-green-40 text-white", "D bg-red-40 text-white"]);
});

// `matches!` of an or-pattern, `None | Some(Bottom)`, is its tests'
// `||`, no labeled block, as react.dev's Diagram places its caption.
test("an or-pattern's matches is an or of its tests", async () => {
  const dir = fixture("or-pattern-matches");
  writeFileSync(join(dir, "lib.rs"), `#[derive(Clone, Copy, PartialEq)]
pub enum CaptionPosition {
    #[cfg_attr(rust_js, rust_js::name = "top")]
    Top,
    #[cfg_attr(rust_js, rust_js::name = "bottom")]
    Bottom,
}
pub fn below(caption_position: Option<CaptionPosition>) -> bool {
    matches!(caption_position, None | Some(CaptionPosition::Bottom))
}
pub fn shown(caption_position: Option<CaptionPosition>, text: &'static str) -> Option<&'static str> {
    matches!(caption_position, None | Some(CaptionPosition::Bottom)).then_some(text)
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain('return captionPosition == null || captionPosition === "bottom";');
  expect(js).toContain('return captionPosition == null || captionPosition === "bottom" ? text : undefined;');
  const lib = await import(join(dir, "mir.js"));
  expect([lib.below(undefined), lib.below("top"), lib.shown("bottom", "t"), lib.shown("top", "t")]).toEqual([true, false, "t", undefined]);
});

// A match's arms as statements chain in the order written, `if (TAG ===
// "resize") { .. } else if (TAG === "start") ..`, not `!== "resize"` with
// the first arm last, as react.dev's Preview handles Sandpack's messages.
test("a match's arms as statements chain in written order", async () => {
  const dir = fixture("match-statements");
  writeFileSync(join(dir, "lib.rs"), `pub enum Message {
    #[cfg_attr(rust_js, rust_js::name = "resize")]
    Resize { height: f64 },
    #[cfg_attr(rust_js, rust_js::name = "start")]
    Start { first_load: Option<bool> },
    #[cfg_attr(rust_js, rust_js::name = "done")]
    Done,
    #[cfg_attr(rust_js, rust_js::name = "refresh")]
    Refresh,
    #[cfg_attr(rust_js, rust_js::name = "console")]
    Console,
}
pub fn handle(message: &Message, log: &mut Vec<String>) {
    match message {
        Message::Resize { height } => {
            log.push(format!("h{height}"));
        }
        Message::Start { first_load } => {
            if *first_load == Some(true) {
                log.push("first".to_string());
            }
            log.push("start".to_string());
        }
        Message::Done => {
            log.push("done".to_string());
        }
        Message::Refresh | Message::Console => {}
    }
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain('  if (message.TAG === "resize") {');
  expect(js).toContain('  } else if (message.TAG === "start") {');
  expect(js).not.toContain("!==");
  const lib = await import(join(dir, "mir.js"));
  const log: string[] = [];
  for (const m of [{ TAG: "resize", height: 2 }, { TAG: "start", firstLoad: true }, "done", "refresh"]) lib.handle(m, log);
  expect(log).toEqual(["h2", "first", "start", "done"]);
});

// A match's arm that does nothing, `Refresh | Console { .. } | Other(_) =>
// {}`, is no `else if (message !== "refresh") {}`, as react.dev's Preview
// ignores Sandpack's other messages.
test("a match arm that does nothing is no empty else if", async () => {
  const dir = fixture("empty-arm");
  writeFileSync(join(dir, "lib.rs"), `pub enum Message {
    #[cfg_attr(rust_js, rust_js::name = "resize")]
    Resize { height: f64 },
    #[cfg_attr(rust_js, rust_js::name = "done")]
    Done,
    #[cfg_attr(rust_js, rust_js::name = "refresh")]
    Refresh,
    #[cfg_attr(rust_js, rust_js::name = "console")]
    Console { log: Vec<String> },
    #[cfg_attr(rust_js, rust_js::untagged)]
    Other(&'static str),
}
pub fn handle(message: &Message, log: &mut Vec<String>) {
    match message {
        Message::Resize { height } => {
            log.push(format!("h{height}"));
        }
        Message::Done => {
            log.push("done".to_string());
        }
        Message::Refresh | Message::Console { .. } | Message::Other(_) => {}
    }
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain('  } else if (message === "done") {\n    log.push("done");\n  }\n}');
  const lib = await import(join(dir, "mir.js"));
  const log: string[] = [];
  for (const m of [{ TAG: "resize", height: 1 }, "done", "refresh", "x"]) lib.handle(m, log);
  expect(log).toEqual(["h1", "done"]);
});

// A closure's pattern of a pattern, `|(index, Challenge { name, .. })|`, is
// one JS pattern of another, `([n, { name }]) => ..`, and of an enumerated
// item, the item's pattern and its index, `({ name, id, order }, index) =>
// ..`, as react.dev's Challenges' Navigation lists its buttons.
test("a closure's nested patterns are nested JS patterns", async () => {
  const dir = fixture("nested-patterns");
  writeFileSync(join(dir, "lib.rs"), `pub struct Challenge {
    pub id: &'static str,
    pub name: &'static str,
    pub order: u32,
}
pub fn labels(challenges: &[Challenge]) -> Vec<String> {
    challenges
        .iter()
        .enumerate()
        .map(|(index, Challenge { name, id, order })| format!("{index}:{id}:{order}. {name}"))
        .collect()
}
pub fn named(challenges: &[(u32, Challenge)]) -> Vec<String> {
    challenges.iter().map(|(n, Challenge { name, .. })| format!("{n} {name}")).collect()
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain("challenges.map(({ name, id, order }, index) => `${index}:${id}:${order}. ${name}`)");
  expect(js).toContain("challenges.map(([n, { name }]) => `${n} ${name}`)");
  const lib = await import(join(dir, "mir.js"));
  expect(lib.labels([{ id: "a", name: "A", order: 1 }])).toEqual(["0:a:1. A"]);
  expect(lib.named([[7, { id: "a", name: "A", order: 1 }]])).toEqual(["7 A"]);
});

// An or-pattern's values that go to one block, `op @ ('+' | '-')`, are one
// test from MIR, `op === "+" || op === "-"`, and the block once, as the
// program wrote it: not a copy for each, which declared what the block
// binds, `const right`, in one copy alone.
test("an or-pattern's values from MIR are one test of its block", async () => {
  const dir = fixture("or-pattern-block");
  writeFileSync(join(dir, "lib.rs"), `#[derive(Clone)]
pub enum Token { Op(char), Num(f64) }
pub struct P { pub toks: Vec<Token>, pub at: usize }
impl P {
    fn peek(&self) -> Option<&Token> { self.toks.get(self.at) }
    fn next(&mut self) { self.at += 1; }
    fn term(&mut self) -> Result<f64, String> {
        match self.toks.get(self.at).cloned() {
            Some(Token::Num(n)) => { self.at += 1; Ok(n) }
            _ => Err("num".into()),
        }
    }
    pub fn expr(&mut self) -> Result<f64, String> {
        let mut left = self.term()?;
        while let Some(Token::Op(op @ ('+' | '-'))) = self.peek().cloned() {
            self.next();
            let right = self.term()?;
            left = if op == '+' { left + right } else { left - right };
        }
        Ok(left)
    }
}
pub fn run() -> f64 {
    let mut p = P { toks: vec![Token::Num(1.0), Token::Op('+'), Token::Num(2.0), Token::Op('-'), Token::Num(0.5)], at: 0 };
    p.expr().unwrap()
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js.split("P.next(p);").length - 1).toBe(1);
  expect((await import(join(dir, "mir.js"))).run()).toBe(2.5);
});

// A match MIR tests as a tree, an arm reached from more than one of its
// leaves, `'0'..='9' | '.'`: an `if` chain of the arms, each tested where
// it's written, not a labeled block each leaf breaks out of.
test("a match's arms of more than one pattern are an if chain", async () => {
  const dir = fixture("decision-chain");
  writeFileSync(join(dir, "lib.rs"), `pub enum Shape { Circle(f64), Sphere(f64), Square(f64), Rect { w: f64, h: f64 }, Line { w: f64 } }
pub fn size(s: &Shape) -> f64 {
    match s {
        Shape::Circle(r) | Shape::Sphere(r) => r * 2.0,
        Shape::Rect { w, .. } | Shape::Line { w } => *w,
        Shape::Square(x) => *x,
    }
}
pub fn shapes() -> Vec<f64> {
    [Shape::Circle(1.0), Shape::Sphere(2.0), Shape::Square(3.0), Shape::Rect { w: 4.0, h: 5.0 }, Shape::Line { w: 6.0 }]
        .iter()
        .map(size)
        .collect()
}
pub fn kind(c: char) -> u32 {
    match c {
        ' ' => 0,
        '0'..='9' | '.' => 1,
        'a'..='z' | 'A'..='Z' | '_' => 2,
        '+' | '-' => 3,
        _ => 4,
    }
}
pub fn log(c: char, out: &mut Vec<u32>) {
    match c {
        ' ' => out.push(0),
        '0'..='9' | '.' => out.push(1),
        'a'..='z' | 'A'..='Z' | '_' => {
            out.push(2);
            out.push(2)
        }
        '+' | '-' => out.push(3),
        _ => out.push(4),
    }
}
pub fn logged(s: &str) -> Vec<u32> {
    let mut out = Vec::new();
    for c in s.chars() {
        log(c, &mut out);
    }
    out
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).not.toMatch(/\bbb\d+:/);
  expect(js).toContain(`  if (s.TAG === "Circle" || s.TAG === "Sphere") {`);
  expect(js).toContain(`  if (s.TAG === "Rect" || s.TAG === "Line") {`);
  expect(js).toContain(`export function kind(c) {
  if (c === " ") {
    return 0;
  }
  if ((c >= "0" && c <= "9") || c === ".") {
    return 1;
  }
  if ((c >= "a" && c <= "z") || (c >= "A" && c <= "Z") || c === "_") {
    return 2;
  }
  if (c === "+" || c === "-") {
    return 3;
  }
  return 4;
}`);
  expect(js).toContain(`  } else if ((c >= "a" && c <= "z") || (c >= "A" && c <= "Z") || c === "_") {
    out.push(2);
    out.push(2);
  } else if (c === "+" || c === "-") {`);
  const lib = await import(join(dir, "lib.js"));
  expect(lib.shapes()).toEqual([2, 4, 3, 4, 6]);
  const chars = [" ", "0", "5", "9", ".", "/", ":", "a", "z", "A", "Z", "_", "`", "{", "@", "[", "+", "-", "*", "é", "😀"];
  expect(chars.map((c) => lib.kind(c))).toEqual([0, 1, 1, 1, 1, 4, 4, 2, 2, 2, 2, 2, 4, 4, 4, 4, 3, 3, 4, 4, 4]);
  expect(lib.logged(chars.join(""))).toEqual([0, 1, 1, 1, 1, 4, 4, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 4, 4, 4, 4, 3, 3, 4, 4, 4]);
});

// A value a block gives where nothing else does, `matches!(v, Some(x) if
// ..)`'s `false`, given first: `let matched = false;`, then what sets it.
test("a value a match gives where no arm sets it is given first", async () => {
  const dir = fixture("default-value");
  writeFileSync(join(dir, "lib.rs"), `fn half(n: i32) -> Option<i32> {
    if n % 2 == 0 { Some(n / 2) } else { None }
}
pub fn chained(n: i32, calls: &mut u32) -> i32 {
    let v = if let Some(h) = half(n) && h != 0 && let Some(q) = { *calls += 1; half(h) } && q > 1 { q } else { 0 };
    v * 10 + *calls as i32
}
pub fn run() -> Vec<i32> {
    let mut calls = 0;
    [12, 4, 3, 0, 8].iter().map(|&n| chained(n, &mut calls)).collect()
}
fn fallback(calls: &mut u32) -> i32 {
    *calls += 100;
    -1
}
pub fn given(n: i32) -> (i32, u32) {
    let mut calls = 0;
    let v = match half(n) {
        Some(h) if h > 1 => h,
        _ => fallback(&mut calls),
    };
    (v, calls)
}
pub fn bumped(n: i32) -> (i32, i32) {
    let mut v = 5;
    let w = if let Some(h) = half(n) && { v += 1; h > v } { h } else { v };
    (w, v)
}
pub fn set_first(n: i32) -> i32 {
    let mut w = 3;
    #[allow(unused_assignments)]
    if let Some(h) = half(n) && { w = 7; h > 2 } { w = h; } else { w = 0; }
    w
}
pub fn read_first(n: i32) -> (i32, Vec<i32>) {
    let mut w = 3;
    let mut seen = Vec::new();
    if let Some(h) = half(n) && { seen.push(w); h > 2 } { w = h; } else { w = 0; }
    (w, seen)
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  const chained = js.slice(js.indexOf("export function chained"), js.indexOf("export function run"));
  expect(chained).not.toMatch(/\bbb\d+:/);
  expect(chained).toContain("  let v = 0;\n");
  const lib = await import(join(dir, "lib.js"));
  expect(lib.run()).toEqual([31, 2, 2, 2, 23]);
  // A default of a call is made where no arm gives one, not first.
  expect([lib.given(8), lib.given(2), lib.given(3)]).toEqual([[4, 0], [-1, 100], [-1, 100]]);
  // Not of what isn't a constant, nor where the block reads or sets it first.
  expect([lib.bumped(4), lib.bumped(20)]).toEqual([[6, 6], [10, 6]]);
  expect([lib.setFirst(4), lib.setFirst(8), lib.setFirst(3)]).toEqual([0, 4, 0]);
  expect([lib.readFirst(4), lib.readFirst(8)]).toEqual([[0, [3]], [4, [3]]]);
});

// An or-pattern's arm that changes what it binds, `*r *= 10.0`, after the
// handle each alternative makes: written once, after the chain that makes
// it, not copied into each alternative's arm.
test("an or-pattern's arm that changes what it binds is written once", async () => {
  const dir = fixture("or-pattern-shared");
  writeFileSync(join(dir, "lib.rs"), `pub enum Shape { Circle(f64), Sphere(f64), Square(f64) }
pub fn grow(shapes: &mut [Shape]) {
    for s in shapes.iter_mut() {
        if let Shape::Circle(r) | Shape::Sphere(r) = s {
            *r *= 10.0;
        }
    }
}
pub fn grown() -> Vec<f64> {
    let mut shapes = vec![Shape::Circle(1.0), Shape::Square(2.0), Shape::Sphere(3.0)];
    grow(&mut shapes);
    shapes.iter().map(|s| match s { Shape::Circle(r) | Shape::Sphere(r) | Shape::Square(r) => *r }).collect()
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js.match(/r\.value \*= 10;/g)).toHaveLength(1);
  expect((await import(join(dir, "lib.js"))).grown()).toEqual([10, 2, 30]);
});

// Two matches whose arms bind the same names, `w` of each: each chain's
// variables its own, as JS declares a name once in a block.
test("matches binding the same names are chains of their own", async () => {
  const dir = fixture("chains-same-names");
  writeFileSync(join(dir, "lib.rs"), `pub enum Shape { Circle(f64), Sphere(f64), Square(f64), Rect { w: f64, h: f64 }, Line { w: f64 } }
pub fn both(a: &Shape, b: &Shape) -> f64 {
    let x = match a {
        Shape::Rect { w, .. } | Shape::Line { w } => *w,
        Shape::Circle(r) | Shape::Sphere(r) => r * 2.0,
        Shape::Square(s) => *s,
    };
    let y = match b {
        Shape::Rect { w, .. } | Shape::Line { w } => *w,
        Shape::Circle(r) | Shape::Sphere(r) => r * 2.0,
        Shape::Square(s) => *s,
    };
    x + y
}
pub fn sums() -> Vec<f64> {
    vec![
        both(&Shape::Rect { w: 1.0, h: 9.0 }, &Shape::Line { w: 2.0 }),
        both(&Shape::Circle(1.0), &Shape::Square(5.0)),
        both(&Shape::Sphere(2.0), &Shape::Rect { w: 3.0, h: 0.0 }),
    ]
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).not.toMatch(/\bbb\d+:/);
  expect((await import(join(dir, "lib.js"))).sums()).toEqual([3, 7, 7]);
});

// ADR 0629: a `match` the program wrote whose every arm tests constants is
// a `switch`, as React writes one: an or-pattern's constants stacked cases,
// a last `_` the `default`, left out where it does nothing; each case ends
// in `break;` where it'd go on, in a block where it declares; a `break` of
// the loop around it names the loop. Not one of two arms, an `if`'s two
// ways, nor one with a guard, nor one whose value is read in place, which
// JS has no `switch` expression for.
test("a match of constants is a switch", async () => {
  const dir = fixture("switches");
  writeFileSync(join(dir, "lib.rs"), `pub const UPPER: u8 = b'O';
const LOWER: u8 = b'c';
#[derive(Clone, Copy)]
pub enum Tag {
    Function,
    Class,
    Host,
}
pub fn kind(n: u32) -> &'static str {
    match n {
        0 => "zero",
        1 | 2 | 3 => "small",
        _ => "big",
    }
}
pub fn named(format: &str, i: usize) -> u32 {
    match format.as_bytes()[i] {
        UPPER | LOWER => 1,
        b'd' => 2,
        _ => 0,
    }
}
pub fn tag(t: Tag) -> u32 {
    match t {
        Tag::Function => 1,
        Tag::Class => 2,
        Tag::Host => 3,
    }
}
pub fn count(s: &str) -> u32 {
    let mut n = 0;
    for c in s.chars() {
        match c {
            'a' | 'e' => n += 1,
            'x' => break,
            'y' => continue,
            _ => {}
        }
        n += 10;
    }
    n
}
pub fn rows(grid: &[&str]) -> u32 {
    let mut n = 0;
    'rows: for row in grid {
        for c in row.chars() {
            match c {
                '#' => break 'rows,
                '.' => break,
                _ => n += 1,
            }
        }
    }
    n
}
pub fn declares(n: u32, log: &mut Vec<String>) {
    match n {
        1 => {
            let s = format!("one{n}");
            log.push(s);
        }
        2 => log.push("two".to_string()),
        _ => {}
    }
}
pub fn guarded(n: u32) -> u32 {
    match n {
        1 if n > 0 => 1,
        2 => 2,
        _ => 3,
    }
}
pub fn length(s: &str) -> u32 {
    let len = s.len();
    let label = match len {
        0 => 10,
        1 => 20,
        _ => 30,
    };
    label + match s.len() {
        0 => 1,
        1 => 2,
        _ => 3,
    }
}
pub fn spoken(n: u32) -> String {
    format!("{}!", match n { 1 => "one", 2 => "two", _ => "many" })
}
pub fn two(n: u32) -> &'static str {
    match n {
        0 => "zero",
        _ => "other",
    }
}
pub fn skips(xs: &[u32]) -> u32 {
    let mut n = 0;
    for x in xs {
        match x {
            1 => n += 1,
            2 => n += 2,
            _ => continue,
        }
    }
    n
}
pub fn sized(s: &str) -> u32 {
    match s.len() {
        0 => 1,
        1 | 2 => 2,
        _ => 3,
    }
}
pub fn slices(n: u32) -> usize {
    let slice: &[u8] = match n {
        0 => &[1],
        1 => &[2, 3],
        _ => &[],
    };
    slice.len()
}

pub fn specifier(code: u8, log: &mut Vec<u8>) {
    match code {
        b'O' | b'c' | b'd' => log.push(code),
        _ => {}
    }
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  // Of one arm of three constants and a `_`, a case of each.
  expect(js).toContain("  switch (code) {\n    case 79:\n    case 99:\n    case 100:\n      log.push(code);\n");
  expect(js).toContain(
    '  switch (n) {\n    case 0:\n      return "zero";\n    case 1:\n    case 2:\n    case 3:\n      return "small";\n    default:\n      return "big";\n  }',
  );
  // What's matched read in place, its temporary's value; a `const` by its name.
  expect(js).toContain("  switch (bytes[i]) {\n    case UPPER:\n    case LOWER:\n      return 1;\n    case 100:");
  // Each variant a case, the last too: Rust tested every one.
  expect(js).toContain('    case "Class":\n      return 2;\n    case "Host":\n      return 3;\n  }');
  expect(js).toContain(
    '  loop: for (const c of s) {\n    switch (c) {\n      case "a":\n      case "e":\n        n = (n + 1) >>> 0;\n        break;\n      case "x":\n        break loop;\n      case "y":\n        continue;\n    }\n',
  );
  // The program's label, and one of its own for the inner loop.
  expect(js).toContain('        case "#":\n          break rows;\n        case ".":\n          break loop;\n        default:\n          n = (n + 1) >>> 0;\n      }\n');
  expect(js).toContain("    loop: for (const c of row) {\n");
  expect(js).toContain("    case 1: {\n      const s = `one${n}`;\n      log.push(s);\n      break;\n    }\n");
  expect(js).not.toMatch(/default:\n *break;/);
  expect(js).toContain("  if (n === 1) {\n    if (n > 0) {");
  // A variable the program named, by its name; a value read in place, its `?:`.
  expect(js).toContain("  switch (len) {\n");
  expect(js).toContain("  switch ($byteLen(s)) {\n");
  expect(js).toContain("  let slice;\n  switch (n) {\n    case 0:\n      slice = [1];\n");
  expect(js).toContain("$byteLen(s) === 0 ? 1 : $byteLen(s) === 1 ? 2 : 3");
  expect(js).toContain('n === 1 ? "one" : n === 2 ? "two" : "many"');
  // Of two arms, the \`if\` of two ways.
  expect(js).toContain('  if (n === 0) {\n    return "zero";\n  }\n  return "other";\n');
  const lib = await import(join(dir, "lib.js"));
  expect([0, 2, 9].map(lib.kind)).toEqual(["zero", "small", "big"]);
  expect([0, 1, 2, 3].map((i) => lib.named("Ocdz", i))).toEqual([1, 1, 2, 0]);
  expect(["Function", "Class", "Host"].map(lib.tag)).toEqual([1, 2, 3]);
  expect([lib.count("abeyz"), lib.count("aaxa")]).toEqual([42, 22]);
  expect(lib.rows(["ab.c", "d#e", "fg"])).toEqual(3);
  const log: string[] = [];
  [1, 2, 3].forEach((n) => lib.declares(n, log));
  expect(log).toEqual(["one1", "two"]);
  expect([1, 2, 5].map(lib.guarded)).toEqual([1, 2, 3]);
  expect(["", "a", "abc"].map(lib.length)).toEqual([11, 22, 33]);
  expect([1, 2, 3].map(lib.spoken)).toEqual(["one!", "two!", "many!"]);
  expect([0, 1].map(lib.two)).toEqual(["zero", "other"]);
  expect(lib.skips([1, 2, 5, 2])).toEqual(5);
  expect(["", "ab", "abc"].map(lib.sized)).toEqual([1, 2, 3]);
  expect([0, 1, 2].map(lib.slices)).toEqual([1, 2, 0]);
  const pushed: number[] = [];
  [79, 1, 100].forEach((code) => lib.specifier(code, pushed));
  expect(pushed).toEqual([79, 100]);
});
