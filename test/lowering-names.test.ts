// What the compiler writes of a program's comments and blank lines, and of its
// names: a variable's, a shadowed one's, a held temporary's, `const` or `let`
// and where a `let` is declared. Each test is a program of its own, compiled
// and run.

import { beforeAll, expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { buildReact, compiler, fixture, run, target } from "./support";

beforeAll(buildReact, 600_000);

// ADR 0609: a \`let\` that shadows a parameter, the parameter read no more
// after it nor by a closure, is the parameter assigned, as React's
// scheduler has \`priorityLevel = NormalPriority\`, not \`priorityLevel$1\`.
test("a let that shadows a parameter read no more is the parameter assigned", async () => {
  const dir = fixture("shadowed-parameters");
  writeFileSync(join(dir, "lib.rs"), `pub fn clamped(level: u32) -> u32 {
    let level = if level > 5 { 3 } else { level };
    level * 2
}

pub fn kept(level: u32) -> Box<dyn Fn() -> u32> {
    let first = move || level;
    let level = level + 1;
    Box::new(move || first() + level)
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("export function clamped(level) {\n  level = level > 5 ? 3 : level;\n");
  // Read by a closure made before: a name of its own.
  expect(js).toContain("level$1");
  const lib = await import(join(dir, "lib.js"));
  expect([lib.clamped(9), lib.clamped(2), lib.kept(1)()]).toEqual([6, 4, 3]);
});

// ADR 0606: a blank line before an item's attributes is before the item,
// as React's SchedulerFeatureFlags groups its flags.
test("a blank line before an item's attributes is kept", () => {
  const dir = fixture("blank-before-attributes");
  writeFileSync(join(dir, "lib.rs"), `pub const A: u32 = 1;

#[cfg_attr(rust_js, rust_js::name = "dee")]
pub const D: u32 = 4;
#[cfg_attr(rust_js, rust_js::name = "eee")]
pub const E: u32 = 5;
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("export const A = 1;\n\nexport const dee = 4;\nexport const eee = 5;\n");
});

// ADR 0603: a comment before a `let` with no JS of its own, a variable
// read in place, is the next statement's: React's scheduler has
// `// Timer fired. Transfer to the task queue.` before `pop(timerQueue);`.
test("a comment before a let that has no JS is the next statement's", () => {
  const dir = fixture("comment-of-a-removed-let");
  writeFileSync(join(dir, "lib.rs"), `use std::rc::Rc;

pub fn moved(items: &mut Vec<Rc<String>>, into: &mut Vec<Rc<String>>) {
    if let Some(first) = items.first() {
        // Taken, then moved.
        let first = Rc::clone(first);
        items.remove(0);
        into.push(first);
    }
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("    // Taken, then moved.\n    $removeAt(items, 0);\n");
});

// ADR 0621: a comment before a macro's opening, `thread_local! {`, is its
// code's, its first item's, a blank line between them kept; one after a module's
// last code, before its file's end or an inline module's `}`, ends its JS.
test("a comment before a macro's opening is its code's, one at the end ends its module", () => {
  const dir = fixture("comments-around");
  writeFileSync(join(dir, "lib.rs"), `use std::cell::Cell;

pub mod outer;

// The counts.

thread_local! {
    // The first.
    static COUNT: Cell<u32> = const { Cell::new(1) };
}

pub fn count() -> u32 {
    COUNT.get()
}

pub mod inner {
    pub fn two() -> u32 {
        2
    }

    // Inner's end.
}
`);
  writeFileSync(join(dir, "outer.rs"), `pub fn three() -> u32 {
    3
}

// The file's end,
// in two lines.
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("// The counts.\n\n// The first.\nconst COUNT = 1;\n");
  expect(js).not.toContain("Inner's end");
  const inner = readFileSync(join(dir, "inner.js"), "utf8");
  expect(inner).toMatch(/\n}\n\n\/\/ Inner's end\.\n\/\/# sourceMappingURL/);
  const outer = readFileSync(join(dir, "outer.js"), "utf8");
  expect(outer).toMatch(/\n}\n\n\/\/ The file's end,\n\/\/ in two lines\.\n\/\/# sourceMappingURL/);
});

// ADR 0630: a block comment with code after it on its line, inside an
// expression, is after the JS of the code right before it, as React's
// normalizeConsoleFormat has its `case 79 /* "O" */:`.
test("a comment inside an expression is after the code before it", () => {
  const dir = fixture("comments-inline");
  writeFileSync(join(dir, "lib.rs"), `pub fn kind(code: u32) -> u32 {
    if code != 37 /* "%" */ {
        return 0;
    }
    match code {
        79 /* "O" */ | 99 /* "c" */ | 100 /* "d" */ => 1,
        _ => 2,
    }
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain('if (code !== 37 /* "%" */) {');
  expect(js).toContain('case 79 /* "O" */:\n    case 99 /* "c" */:\n    case 100 /* "d" */:\n');
});

// ADR 0444: a comment is its JS's where its code is, before the statement
// or item it's before, as written; a doc comment, `///`, is a JSDoc one; one
// before what has no JS, a struct's, is nobody's.
test("a comment is in the JS where it's in the Rust", () => {
  const dir = fixture("comments-kept");
  writeFileSync(join(dir, "lib.rs"), `// The file's own leading comment, its header's.

pub fn a() -> u32 {
    1
}

// Why b is two.
pub fn b() -> u32 {
    2
}

/// The sum, documented.
/// Of two lines.
pub fn sum(x: u32) -> u32 {
    // Add them.
    let y = x + a();
    /* Then b. */
    y + b()
}

/// No JS: a type's.
pub struct S {
    pub a: u32,
}

pub fn c(s: &S) -> u32 {
    s.a
}

/// One line.
pub fn later(v: &[u32]) -> u32 {
    let first = v.len() as u32; // Its length.

    // In a closure.
    let f = |x: u32| {
        // Inside.
        x + 1
    };
    // Before a match.
    let n = match v.first() {
        Some(&x) if x > 3 => x,
        _ => 0,
    };
    f(first) + n
}

/// Globs: \`src/**/*.rs\`.
pub fn glob() -> u32 {
    /* Outer /* nested */ still outer. */
    3
}

thread_local! {
    /// The count's start.
    static START: u32 = 5;
}

pub fn pick(flag: bool) -> u32 {
    let n = if flag {
        // Yes.
        1
    } else {
        2
    };
    let k = if flag {
        3
    } else {
        // Four.
        4
    };
    let m = match n {
        // One.
        1 => 10,
        _ => 20,
    };
    n + m + k
}

pub fn gated(n: u32, o: Option<u32>) -> u32 {
    let m = n + 1;

    let Some(p) = o else {
        return m;
    };
    p * 2
}

pub fn start() -> u32 {
    START.with(|s| *s)
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("// The file's own leading comment, its header's.\n");
  expect(js.match(/leading comment/g)?.length).toBe(1);
  expect(js).toContain("// Why b is two.\nexport function b() {");
  expect(js).toContain("/**\n * The sum, documented.\n * Of two lines.\n */\nexport function sum(x) {\n  // Add them.\n  const y");
  expect(js).toContain("  /* Then b. */\n  return (y + b()) >>> 0;");
  expect(js).not.toContain("No JS");
  expect(js).toContain("/** One line. */\nexport function later(v) {\n  const first = v.length; // Its length.\n\n  // In a closure.\n  const f = (x) => {\n    // Inside.\n    return (x + 1) >>> 0;\n  };\n  // Before a match.\n  const x = v[0];\n  let n;");
  expect(js).toContain("/// Globs: `src/**/*.rs`.\nexport function glob() {\n  return 3;");
  expect(js).not.toContain("nested");
  expect(js).toContain("}\n\n/** The count's start. */\nconst START = 5;\n");
  // A blank line before a statement whose span starts inside its line, a
  // \`let .. else\`'s at its pattern.
  expect(js).toContain("  const m = (n + 1) >>> 0;\n\n  if (o == null) {");
  // A branch's comment keeps its branch a statement, as an expression has
  // no line for it, an \`if\`'s and a \`match\`'s.
  expect(js).toContain("  let n;\n  if (flag) {\n    // Yes.\n    n = 1;\n  } else {\n    n = 2;\n  }\n  let k;\n  if (flag) {\n    k = 3;\n  } else {\n    // Four.\n    k = 4;\n  }\n  let m;\n  if (n === 1) {\n    // One.\n    m = 10;\n  } else {\n    m = 20;\n  }\n");
});

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
  expect(js).toContain("  let n = 0;\n  n += 1;");
  expect(js).toContain("  const n = flag ? 1 : 2;");
  expect(js).toContain("  let n = 0;\n  const n$1 = { value: n };");
  const lib = await import(join(dir, "lib.js"));
  expect([lib.pushed(), lib.counted(), lib.later(true), lib.later(false), lib.handled()]).toEqual([[1], 1, 1, 2, 1]);
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

// What \`if let Some(x) = e\` binds, from MIR (ADR 0364), is named as the
// program names it: the temporary MIR holds \`e\` in takes \`x\`'s name, and
// \`x\` of a temporary keeps it.
test("an if-let's binding from MIR keeps the name the program gives it", async () => {
  const dir = fixture("mir-named-bindings");
  writeFileSync(join(dir, "lib.rs"), `use std::cell::Cell;
pub struct Ref { pub current: Cell<Option<u32>> }
pub fn scrolled(selected: bool, r: &Ref, out: &mut Vec<u32>) {
    if selected && let Some(current) = r.current.get() {
        if current > 2 {
            out.push(current);
        }
    }
}
fn find(items: &[u32], k: u32) -> Option<u32> {
    items.iter().copied().find(|x| *x > k)
}
pub fn first_over(items: &[u32], k: u32, out: &mut Vec<u32>) {
    if let Some(found) = find(items, k) {
        out.push(found);
        out.push(found + 1);
    }
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain("  const found = find(items, k);\n  if (found != null) {\n    out.push(found);\n");
  expect(js).toContain("    const current = r.current;\n");
  expect(js).not.toMatch(/\b_\d+\b/);
  const lib = await import(join(dir, "mir.js"));
  const [a, b]: unknown[][] = [[], []];
  lib.scrolled(true, { current: 3 }, a);
  lib.firstOver([1, 5], 2, b);
  expect([a, b]).toEqual([[3], [5, 6]]);
});

// A block's `const` is named as Rust names it where every read is still of
// what it was, a JS scope's own shadowing another's, as a person writes an
// early return: `if (a) { const n = 1; return n + 1; } const n = 2;`. One
// whose block reads the outer `n` keeps its `$1`: JS's `n` there would be
// the block's, not yet made.
test("a block's const reuses a name where each read keeps its own", async () => {
  const dir = fixture("block-names");
  writeFileSync(join(dir, "lib.rs"), `pub fn twice(a: bool) -> u32 {
    if a {
        let n = 1;
        n + 1
    } else {
        let n = 2;
        n * 3
    }
}

pub fn both(a: bool) -> u32 {
    let n = 10;
    if a {
        let m = n;
        let n = 1;
        m + n
    } else {
        n
    }
}

// An else after a branch that returns is written after its if, in the
// function's block, beside its other n.
pub fn early(a: bool, k: u32) -> u32 {
    let n = k + 1;
    if a {
        n
    } else {
        let n = 2;
        n * 3
    }
}

// Two of one name in one block: JS refuses a second const n there.
#[allow(unused_variables)]
pub fn again(mut items: Vec<u32>) -> usize {
    let n = items.pop();
    let n = items.len();
    n
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--library", "--manifest", join(dir, "manifest.json")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("  if (a) {\n    const n = 1;\n    return n + 1;\n  }\n  const n = 2;\n  return n * 3;");
  expect(js).toContain("    const m = n;\n    const n$1 = 1;");
  expect(js).toContain("  items.pop();\n  const n = items.length;");
  expect(js).toContain("    return n;\n  }\n  const n$1 = 2;");
  const lib = await import(join(dir, "lib.js"));
  expect([lib.twice(true), lib.twice(false), lib.both(true), lib.both(false), lib.again([1, 2]), lib.early(true, 4), lib.early(false, 4)]).toEqual([2, 6, 11, 10, 1, 5, 6]);
});

// A guard's binding, which the guard reads in place, gets no `const` its arm
// never reads, as Next.js's getStaticProps has it: in a library, whose
// pattern's bindings are `const`s, as another crate may change what's
// matched (ADR 0100), and in a program.
test("a binding only a guard reads gets no const", async () => {
  const dir = fixture("guard-binding");
  writeFileSync(join(dir, "lib.rs"), `pub struct Params {
    pub code: String,
}

pub struct Props {
    pub code: String,
}

pub fn pick(params: Option<Params>) -> Option<Props> {
    match params {
        Some(Params { code }) if code == "3" => None,
        Some(Params { code }) if code != "0" => Some(Props { code }),
        _ => None,
    }
}

// One unread, of another only it reads.
pub fn cascade(n: u32) -> u32 {
    let next = n + 1;
    let _kept = next;
    1
}

// One unread whose making does something, kept.
pub fn popped(mut items: Vec<u32>) -> usize {
    let _last = items.pop();
    items.len()
}

// One read, kept.
pub fn doubled(n: u32) -> u32 {
    let next = n + 1;
    next * 2
}

pub struct Noisy(pub u32);

impl Drop for Noisy {
    fn drop(&mut self) {
        println!("dropped {}", self.0);
    }
}

// One a guard alone reads that owns what it binds, dropped as its arm ends.
pub fn guarded(noisy: Option<Noisy>, go: bool) -> u32 {
    match noisy {
        Some(owned) if go => {
            println!("arm");
            1
        }
        _ => 0,
    }
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--library", "--manifest", join(dir, "manifest.json")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain('  if (params) {\n    if (params.code === "3") {\n      return;\n    }');
  expect(js).toContain("export function cascade(n) {\n  return 1;\n}");
  const lib = await import(join(dir, "lib.js"));
  expect([lib.pick({ code: "3" }), lib.pick({ code: "1" }), lib.pick({ code: "0" }), lib.popped([1, 2]), lib.doubled(2)]).toEqual([undefined, { code: "1" }, undefined, 1, 6]);
  const logged: string[] = [];
  const log = console.log;
  console.log = (line: string) => logged.push(line);
  try {
    lib.guarded([7], true);
  } finally {
    console.log = log;
  }
  expect(logged).toEqual(["arm", "dropped 7"]);
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
  expect(js).toContain("  const kept = text;\n");
  expect(js).toContain("  return n + 1;\n");
  expect(js).toContain("    const n$1 = n;\n    fs.push(() => n$1);");
  // In a closure too, of its own parameter, as react.dev's createFileMap
  // casts each snippet it's given.
  expect(js).toContain("return items.reduce((total, item) => (total + $byteLen(item)) >>> 0, 0);");
  const lib = await import(join(dir, "lib.js"));
  expect([lib.captured("a")(), lib.kept("b"), lib.before(true), lib.before(false), lib.looped(), lib.summed(["ab", "c"])]).toEqual(["a", "b!", 3, 2, 1, 3]);
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
  expect([lib.cleared(failure), lib.cleared({ message: "kept" }), lib.readAfter(failure), lib.taken(failure), lib.looped(failure), lib.captured(failure), lib.bumped(1), lib.bumpedElse(1), lib.lent({ hits: 0 }, { hits: 0 })])
    .toEqual([true, false, 6, 6, 11, 6, 3, 3, 50]);
});

// ADR 0573, amended: a `thread_local!` is its module's variable (ADR 0270),
// named as JS names one, camelCase of its `SCREAMING_SNAKE` words: React's
// scheduler's `taskQueue`. A `const` is a constant, named as written; one
// that must be another says so, `#[rust_js::name]`.
test("a thread-local's SCREAMING_SNAKE name is camelCase, a constant's as written", async () => {
  const dir = fixture("screaming-names");
  writeFileSync(join(dir, "lib.rs"), `use std::cell::{Cell, RefCell};

pub const MAX_SIGNED_31_BIT_INT: u32 = 1073741823;

#[rust_js::name = "NormalPriority"]
pub const NORMAL_PRIORITY: u8 = 3;

thread_local! {
    static TASK_ID_COUNTER: Cell<u32> = const { Cell::new(1) };
    static SEEN_IDS: RefCell<Vec<u32>> = const { RefCell::new(Vec::new()) };
}

pub fn next_id() -> u32 {
    let id = TASK_ID_COUNTER.get();
    TASK_ID_COUNTER.set(id + 1);
    SEEN_IDS.with_borrow_mut(|seen| seen.push(id));
    id.min(MAX_SIGNED_31_BIT_INT) + u32::from(NORMAL_PRIORITY)
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("export const MAX_SIGNED_31_BIT_INT = 1073741823;");
  expect(js).toContain("export const NormalPriority = 3;");
  expect(js).toContain("let taskIdCounter = 1;");
  expect(js).toContain("const seenIds = [];");
  expect(js).not.toMatch(/TASK_ID|SEEN_IDS/);
  const lib = await import(join(dir, "lib.js"));
  expect([lib.nextId(), lib.nextId()]).toEqual([4, 5]);
});

// ADR 0598: of a variable the program names, its parts read in a row are
// one pattern only of the names the program gives them, not a temporary
// read once: React's scheduler has `const port = channel.port2;
// channel.port1.onmessage = ..`, not `const { port2: port, port1: _12 } =
// channel;`.
test("a temporary is no part of a named variable's pattern", () => {
  const dir = fixture("parts-of-a-name");
  writeFileSync(join(dir, "lib.rs"), `fn work() {}

pub fn make(immediate: bool) -> Box<dyn Fn()> {
    if immediate {
        Box::new(work)
    } else {
        let channel = webapi::message_channel::new();
        let port = channel.port2();
        channel.port1().set_onmessage(Some(Box::new(|_| work())));
        Box::new(move || port.post_message(None::<u8>, None))
    }
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "--extern", `webapi=${join(target, "libwebapi.rmeta")}`, "-L", target]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toMatch(/ const port = channel\.port2;\n +channel\.port1\.onmessage = \(\) => \{\n/);
});

// ADR 0619: a thread-local of a primitive, never written, keeps its name as a
// const does, \`export const REACT_ELEMENT_TYPE = Symbol.for(..)\`, React's
// symbols'; one of a function or an object, or a Cell, is a camelCase
// variable.
test("a thread-local of a primitive is named as a constant", async () => {
  const dir = fixture("constant-thread-locals");
  writeFileSync(join(dir, "lib.rs"), `use std::cell::Cell;

use js::Symbol;

thread_local! {
    pub static REACT_ELEMENT_TYPE: &'static Symbol = js::symbol::r#for("react.transitional.element");
    pub static FRAME_YIELD_MS: f64 = js::math::random() * 0.0 + 5.0;
    pub static LOCAL_NOW: fn() -> f64 = js::date::now;
    pub static CLICK_COUNT: Cell<u32> = const { Cell::new(0) };
}

pub fn is_element(s: &Symbol) -> bool {
    REACT_ELEMENT_TYPE.with(|t| std::ptr::eq(*t, s))
}

pub fn yield_ms() -> f64 {
    FRAME_YIELD_MS.with(|ms| *ms) + LOCAL_NOW.with(|now| now()) * 0.0 + f64::from(CLICK_COUNT.get())
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain('export const REACT_ELEMENT_TYPE = Symbol.for("react.transitional.element");');
  expect(js).toContain("export const FRAME_YIELD_MS = ");
  expect(js).toContain("export const localNow = Date.now;");
  expect(js).toContain("export const clickCount = { value: 0 };");
  const lib = await import(join(dir, "lib.js"));
  expect([lib.isElement(Symbol.for("react.transitional.element")), lib.yieldMs()]).toEqual([true, 5]);
});

// Two references to one field, `let a = &x.0; let b = &x.0;`: each its
// own name, not one taken apart, `const [b] = x`, which lost `a` (rustc's
// borrow-tuple-fields).
test("two names of one field are each kept", async () => {
  const dir = fixture("same-field-twice");
  writeFileSync(join(dir, "lib.rs"), `pub fn both(x: (i32, i32)) -> i32 {
    let a = &x.0;
    let b = &x.0;
    *a + *b
}

pub fn each(pairs: Vec<(i32, i32)>) -> i32 {
    let mut total = 0;
    for p in pairs {
        let a = p.0;
        let b = p.0;
        total += a * b;
    }
    total
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const lib = await import(join(dir, "mir.js"));
  expect([lib.both([3, 4]), lib.each([[2, 0], [3, 1]])]).toEqual([6, 13]);
});

// Two loops of ranges a call ends, each end held: two names, as two
// `const end`s of one block are no JS (the fuzz's seed 2).
test("two loops each hold their range's end by a name of their own", async () => {
  const dir = fixture("held-ends");
  writeFileSync(join(dir, "lib.rs"), `fn id<T>(x: T) -> T {
    x
}

pub fn counts() -> u32 {
    let mut n = 0;
    for k in 0..id(1u32) {
        n += k + 1;
    }
    for k in 0..id(2u32) {
        n += k + 10;
    }
    n
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js.match(/const end =/g)?.length).toBe(1);
  const lib = await import(join(dir, "lib.js"));
  expect(lib.counts()).toBe(22);
});

// A local named apart from a module's name its function never reads is
// that name, `{ label }`, as JS shadows it; and an import named apart from
// a global its module never reads is the global's name, `import Error`
// (ADR 0352). One whose function reads the outer name keeps its own.
test("a name nothing it shadows is read in is the name", async () => {
  const dir = fixture("reclaimed-names");
  writeFileSync(join(dir, "err.js"), "export default function Error() { return 40; }\nexport function tag() { return 7; }\n");
  writeFileSync(join(dir, "lib.rs"), `pub fn label(home: bool) -> &'static str {
    if home { "Home" } else { "Away" }
}

pub struct P<'a> {
    pub label: &'a str,
}

pub fn show(P { label }: P) -> usize {
    label.len()
}

pub fn make() -> impl Fn(P) -> usize {
    |P { label }| label.len()
}

pub fn shadowing(P { label: text }: P) -> usize {
    let label = text;
    label.len() + self::label(false).len()
}

#[cfg_attr(rust_js, rust_js::link_name = "./err.js#default")]
fn Error() -> u32 {
    unreachable!()
}

pub fn made() -> u32 {
    Error() + 2
}

pub fn kept(class: u32) -> u32 {
    class + 1
}

#[allow(non_snake_case)]
pub fn both(n: u32) -> (u32, String) {
    let String = n + 1;
    (String, n.to_string())
}

#[allow(non_snake_case)]
pub fn alone(n: u32) -> u32 {
    let String = n + 1;
    String
}

// An import of the name of a function of the module's own stays apart.
#[cfg_attr(rust_js, rust_js::link_name = "./err.js#tag")]
fn js_tag() -> u32 {
    unreachable!()
}

pub fn tag() -> u32 {
    js_tag() + 1
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("export function show({ label }) {\n  return $byteLen(label);\n}");
  expect(js).toContain("return ({ label }) => $byteLen(label);");
  expect(js).toContain("const label$1 = text;");
  expect(js).toContain('import Error, { tag as tag$1 } from "./err.js";');
  expect(js).toContain("return (Error() + 2) >>> 0;");
  // A word JS keeps stays apart; so does a global a conversion reads.
  expect(js).toContain("export function kept(class$) {");
  expect(js).toContain("return [String$, String(n)];");
  expect(js).toContain("const String = (n + 1) >>> 0;");
  const lib = await import(join(dir, "lib.js"));
  expect([lib.show({ label: "ab" }), lib.make()({ label: "abc" }), lib.shadowing({ label: "a" }), lib.made(), lib.kept(1), lib.both(1), lib.alone(1), lib.tag()]).toEqual([2, 3, 5, 42, 2, [2, "1"], 2, 8]);
});

// An import of JS's global's name, `String`, stays apart where the module
// converts with the global, `String(n)` (ADR 0352).
test("an import named as a global a conversion reads stays apart", async () => {
  const dir = fixture("reclaimed-global-import");
  writeFileSync(join(dir, "names.js"), "export function String() { return 9; }\n");
  writeFileSync(join(dir, "lib.rs"), `#[cfg_attr(rust_js, rust_js::link_name = "./names.js#String")] fn js_string() -> u32 { unreachable!() }

pub fn stringed(n: u32) -> String {
    js_string().to_string() + &n.to_string()
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain('import { String as String$ } from "./names.js";');
  const lib = await import(join(dir, "lib.js"));
  expect(lib.stringed(3)).toBe("93");
});

// A function of the module's own, not exported, named as a global its
// module never reads is the global's name, as react.dev's MDXComponents
// writes `function Math({children})` (ADR 0352). One in a module that
// reads the global stays apart, and so does one another module imports by
// its name.
test("a module's own function named as a global it never reads is the name", async () => {
  const dir = fixture("reclaimed-own-item");
  writeFileSync(join(dir, "lib.rs"), `#[allow(non_snake_case)]
fn Math(x: u32) -> u32 {
    x + 1
}

pub fn twice(x: u32) -> u32 {
    Math(Math(x))
}

pub fn shared(x: u32) -> u32 {
    exported::Math(x)
}

pub mod exported {
    #[allow(non_snake_case)]
    pub fn Math(x: u32) -> u32 {
        x + 2
    }
}

pub mod rounded {
    #[allow(non_snake_case)]
    fn Math(x: f64) -> f64 {
        x.floor()
    }

    pub fn floored(x: f64) -> f64 {
        Math(x)
    }
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  const rounded = readFileSync(join(dir, "rounded.js"), "utf8");
  expect([js.includes("function Math(x) {"), rounded.includes("function Math$(x) {")]).toEqual([true, true]);
  const lib = await import(join(dir, "lib.js"));
  expect([lib.twice(1), lib.shared(2)]).toEqual([3, 4]);
});

test("a `let` from MIR is declared where it's first named", async () => {
  const dir = fixture("mir-sunk-lets");
  writeFileSync(join(dir, "lib.rs"), `pub fn label(path: Option<&str>, crumbs: &[&str], force: bool) -> String {
    let selected = path == Some("x");
    let item;
    if path.is_none() {
        item = "none".to_string();
    } else if crumbs.len() > 1 {
        let is_crumb = crumbs.len() > 2 && crumbs[0] == "y";
        let is_expanded = force || is_crumb || selected;
        item = format!("{is_expanded}");
    } else {
        item = "leaf".to_string();
    }
    item
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain(`export function label(path, crumbs, force) {
  const selected = path === "x";
  let item;
  if (path == null) {
    item = "none";
  } else if (crumbs.length > 1) {
    const isCrumb = crumbs.length > 2 && crumbs[0] === "y";
    const isExpanded = force || isCrumb || selected;
    item = String(isExpanded);
  } else {
    item = "leaf";
  }
  return item;
}`);
});

// What \`?\` of an \`Option\` gives a \`let\`, from MIR, is held under the
// name the program gives it, \`f\`: not \`value\`.
test("a let of an Option's ? from MIR is named as the program names it", async () => {
  const dir = fixture("mir-tried-named");
  writeFileSync(join(dir, "lib.rs"), `pub fn first_even(xs: &[i32]) -> Option<i32> {
    let f = xs.iter().find(|x| *x % 2 == 0)?;
    Some(f * 10)
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain("  const f = xs.find((x) => x % 2 === 0);\n  if (f == null) {\n    return;\n  }\n  return (f * 10) | 0;\n");
  const lib = await import(join(dir, "mir.js"));
  expect([lib.firstEven([1, 4]), lib.firstEven([1])]).toEqual([40, undefined]);
});

test("a `let` a loop's body makes, from MIR, is declared in the body", async () => {
  const dir = fixture("mir-loop-lets");
  writeFileSync(join(dir, "lib.rs"), `pub fn starts(lines: &[&str], ends: &[usize]) -> Vec<usize> {
    let mut out = Vec::new();
    for end in ends {
        let mut start = 0;
        for i in 0..*end {
            start += lines[i].len() + 1;
        }
        out.push(start);
    }
    out
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain("  for (const end of ends) {\n    let start = 0;\n");
  const lib = await import(join(dir, "mir.js"));
  expect(lib.starts(["ab", "c"], [1, 2])).toEqual([3, 5]);
});

// A method's `self` is named after its type, `section` of a `Section`, as
// react.dev's TopNav names `Section.asStr`'s.
test("a method's self from MIR is named after its type", async () => {
  const dir = fixture("self-name");
  writeFileSync(join(dir, "lib.rs"), `#[derive(Clone, Copy)]
pub enum Section {
    #[cfg_attr(rust_js, rust_js::name = "learn")]
    Learn,
    #[cfg_attr(rust_js, rust_js::name = "home")]
    Home,
}
impl Section {
    pub fn as_str(self) -> &'static str {
        match self {
            Section::Learn => "learn",
            Section::Home => "home",
        }
    }
}
pub fn label(section: Section) -> String {
    format!("og-{}", section.as_str())
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain("  asStr(section) {\n    return section;\n  },\n");
  const lib = await import(join(dir, "mir.js"));
  expect(lib.label("home")).toBe("og-home");
});

// A pattern's binding of a variable, `if let Some(error) = raw`, a copy of
// it MIR makes where the variable is borrowed, `const error = raw;`: the
// variable itself where the block sets neither again, as react.dev's
// Preview renames a runtime error.
test("a copy binding of a variable is the variable from MIR", async () => {
  const dir = fixture("copy-binding");
  writeFileSync(join(dir, "lib.rs"), `use std::cell::Cell;
pub struct Error {
    pub title: Cell<Option<&'static str>>,
}
fn debounced(e: Option<&'static Error>) -> Option<&'static Error> {
    e
}
pub fn fixed(given: Option<&'static Error>, lint: Option<&'static Error>) -> Option<&'static str> {
    let mut raw = given;
    let first = &raw;
    if first.is_none_or(|e| e.title.get() == Some("Runtime Exception")) && lint.is_some() {
        raw = lint;
    }
    if let Some(error) = raw
        && error.title.get() == Some("Runtime Exception")
    {
        error.title.set(Some("Runtime Error"));
    }
    let error = debounced(raw);
    error.and_then(|e| e.title.get())
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain('  if (raw && raw.title === "Runtime Exception") {\n    raw.title = "Runtime Error";\n  }');
  const lib = await import(join(dir, "mir.js"));
  expect([lib.fixed({ title: "Runtime Exception" }, undefined), lib.fixed(undefined, { title: "x" })]).toEqual(["Runtime Error", "x"]);
});

// A temporary MIR holds of what a pattern takes apart, read more than once,
// is named after the call that made it, `find`, as rust-analyzer suggests
// (ADR 0569), not `_4`, a name no other in the function has, as react.dev's
// CodeBlock reads its regular expressions' matches.
test("a held temporary is named after the call that made it", async () => {
  const dir = fixture("temporary-name");
  writeFileSync(join(dir, "lib.rs"), `#[cfg_attr(rust_js, rust_js::link_name = "globalThis.find")] #[allow(unused_variables)] fn find(text: &str) -> Option<(String, String)> { unreachable!() }
pub fn lines(meta: &str) -> String {
    let Some((_, lines)) = find(meta) else {
        return String::new();
    };
    lines.to_uppercase()
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain("  const find = globalThis.find(meta);\n  if (!find) {");
  const lib = await import(join(dir, "mir.js"));
  (globalThis as any).find = (text: string) => (text ? [text, "ab"] : undefined);
  expect([lib.lines("x"), lib.lines("")]).toEqual(["AB", ""]);
});

// A `let` Rust declares in a loop's body is declared there,
// `const contentBoxSize = ..` where a branch of each turn makes it, not one
// `let` before the loop, as react.dev's NavigationBar measures its tabs.
test("a let a loop's branch makes is declared in the loop", async () => {
  const dir = fixture("loop-branch-let");
  writeFileSync(join(dir, "lib.rs"), `use js::Unknown;
pub struct BoxSize {
    pub inline_size: f64,
}
#[cfg_attr(rust_js, rust_js::untagged)]
pub enum ContentBoxSize {
    Array(&'static [&'static BoxSize]),
    #[cfg_attr(rust_js, rust_js::otherwise)]
    Single(&'static BoxSize),
}
#[cfg_attr(rust_js, rust_js::link_name = "get contentBoxSize")]
#[allow(unused_variables)]
fn contentBoxSize(this: &Unknown) -> Option<ContentBoxSize> {
    unreachable!()
}
pub fn widths(entries: &[&Unknown], log: impl Fn(f64)) {
    for entry in entries {
        if let Some(size) = contentBoxSize(entry) {
            let contentBoxSize = match size {
                ContentBoxSize::Array(sizes) => unsafe { *sizes.get_unchecked(0) },
                ContentBoxSize::Single(size) => size,
            };
            let width = contentBoxSize.inline_size;
            log(width);
        }
    }
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain("      const contentBoxSize = Array.isArray(size) ? size[0] : size;");
  const lib = await import(join(dir, "mir.js"));
  const widths: number[] = [];
  lib.widths([{ contentBoxSize: [{ inlineSize: 1 }] }, {}, { contentBoxSize: { inlineSize: 2 } }], (w: number) => widths.push(w));
  expect(widths).toEqual([1, 2]);
});

// What a `let`-`else` binds of a variable, `let Some(current) = &mut
// current else { continue; }`, is that variable, read as it is, not a
// copy, `const current$1 = current`, as react.dev's llmsTxt adds pages to
// its current section.
test("a let-else's binding of a variable is the variable", async () => {
  const dir = fixture("let-else-copy");
  writeFileSync(join(dir, "lib.rs"), `pub struct Section {
    pub pages: Vec<u32>,
}
pub fn grouped(items: &[u32]) -> Vec<Section> {
    let mut sections = Vec::new();
    let mut current: Option<Section> = None;
    for &item in items {
        if item == 0 {
            if let Some(section) = current {
                sections.push(section);
            }
            current = Some(Section { pages: Vec::new() });
            continue;
        }
        let Some(current) = &mut current else {
            continue;
        };
        current.pages.push(item);
    }
    if let Some(section) = current {
        sections.push(section);
    }
    sections
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain("    current.pages.push(item);");
  const lib = await import(join(dir, "mir.js"));
  expect(lib.grouped([1, 0, 2, 3, 0, 4])).toEqual([{ pages: [2, 3] }, { pages: [4] }]);
});

// A name given apart from one given apart already, `caption$1`'s, is the
// first's next, `caption$2`, which is the name itself where nothing it
// would shadow is read (ADR 0352), not `caption$1$1`, as react.dev's
// MDXComponents' IllustrationBlock names its captions from MIR.
test("a name given apart twice is reclaimed", async () => {
  const dir = fixture("fresh-twice");
  writeFileSync(join(dir, "lib.rs"), `pub struct Info {
    pub caption: Option<&'static str>,
}
pub fn shown(infos: &[Info]) -> Vec<String> {
    infos
        .iter()
        .map(|info| info.caption.filter(|caption| !caption.is_empty()).map(|caption| format!("<{caption}>")).unwrap_or_default())
        .collect()
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain("const caption = info.caption ? info.caption : undefined;");
  const lib = await import(join(dir, "mir.js"));
  expect(lib.shown([{ caption: "a" }, { caption: "" }, {}])).toEqual(["<a>", "", ""]);
});

// A variable a loop's turn declares, `let y = match x { 0 => continue, ..
// }`, is declared in the loop's body from MIR: each arm assigns it or
// leaves the turn before it's read.
test("a let each turn assigns from MIR is the loop body's", async () => {
  const dir = fixture("turn-let");
  writeFileSync(join(dir, "lib.rs"), `pub fn named(v: &[u32]) -> Vec<String> {
    let mut out = Vec::new();
    for &x in v {
        let y = match x {
            0 => continue,
            1 => "one".to_string(),
            _ => format!("{x}"),
        };
        out.push(y);
    }
    out
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  expect(readFileSync(join(dir, "mir.js"), "utf8")).toContain("  for (const x of v) {\n    let y;\n");
  expect((await import(join(dir, "mir.js"))).named([0, 1, 7])).toEqual(["one", "7"]);
});
