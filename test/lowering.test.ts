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
// ADR 0320: a `dyn` of the crate's trait drops what one of its implementors
// does, so where none drops anything, an `Rc<RefCell<dyn Listener>>` isn't
// counted though a `Weak` elsewhere counts others: it's the value it points
// at, `{ value: .. }`, as its owner's `Rc` is.
test("an Rc of a dyn none of whose implementors drops is its value", () => {
  const dir = fixture("uncounted-dyn-rc");
  writeFileSync(join(dir, "lib.rs"), readFileSync(join(root, "test/corpus/rc_dyn_listeners.rs"), "utf8").replace("fn main()", "pub fn main()"));
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("const logger = { value: { seen: [] } };");
  expect(js).not.toContain("$rcClone(logger)");
});

// ADR 0512: a list MIR held while it took its other items out, `[undefined,
// ids(v)]` of `println!`'s arguments, is its one item left, read as itself.
test("a list of one item left is the item", () => {
  const dir = fixture("holed-lists");
  writeFileSync(join(dir, "lib.rs"), readFileSync(join(root, "test/corpus/extract_if_drops.rs"), "utf8").replace("fn main()", "pub fn main()"));
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("console.log(`odd [${tmp.join(\", \")}] kept [${tmp$2.join(\", \")}]`);");
  expect(js).not.toContain("[undefined, ids(");
});

// ADR 0508: no `.map((item) => item)`: a list of unit enums, or of numbers,
// is shown by `join` alone, and `cloned()` of strings is the strings.
test("no item is mapped to itself", () => {
  const dir = fixture("no-identity-maps");
  writeFileSync(join(dir, "enums.rs"), readFileSync(join(root, "test/corpus/enum_order.rs"), "utf8").replace("fn main()", "pub fn main()"));
  writeFileSync(join(dir, "calc.rs"), readFileSync(join(root, "test/corpus/calculator.rs"), "utf8").replace("fn main()", "pub fn main()"));
  run([compiler, join(dir, "enums.rs"), "-o", join(dir, "enums.js")]);
  run([compiler, join(dir, "calc.rs"), "-o", join(dir, "calc.js")]);
  const enums = readFileSync(join(dir, "enums.js"), "utf8");
  const calc = readFileSync(join(dir, "calc.js"), "utf8");
  expect(enums).toContain("`[${levels.join(\", \")}] ${max == null");
  expect(calc).toContain("const names = Array.from(calc.vars.keys());");
  expect(enums + calc).not.toContain(".map((item) => item)");
  writeFileSync(join(dir, "edits.rs"), readFileSync(join(root, "test/corpus/vec_edits.rs"), "utf8").replace("fn main()", "pub fn main()"));
  run([compiler, join(dir, "edits.rs"), "-o", join(dir, "edits.js")]);
  const edits = readFileSync(join(dir, "edits.js"), "utf8");
  expect(edits).toContain("console.log(`[${v.join(\", \")}] [${w.join(\", \")}] ${asked}`);");
});

// ADR 0489: an iterator's items no caller gives a drop for drop nothing,
// as a type parameter's none's given for: `counted(0..3)`'s `count()` is
// `(n) => n + 1`, not a hook for a drop no dictionary has.
test("a generic count of items no caller gives a drop for drops none", () => {
  const dir = fixture("ungiven-item-slot");
  writeFileSync(join(dir, "lib.rs"), readFileSync(join(root, "test/corpus/yielded_item_drops.rs"), "utf8").replace("fn main()", "pub fn main()"));
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("return $iterator(IClone.clone(iter), IIterator.next, true).reduce((n) => n + 1, 0);");
  expect(js).not.toContain("IIterator.$dropItem");
});

// ADR 0345: an entry's `and_modify(f)` is a statement, and `or_insert`
// after it is of the map and the key, as of an entry alone; a `&mut` it
// gives that nothing reads isn't made.
test("an entry's and_modify then or_insert reads its map and key", () => {
  const dir = fixture("entry-and-modify");
  writeFileSync(join(dir, "lib.rs"), readFileSync(join(root, "test/corpus/entry_and_modify.rs"), "utf8").replace("fn main()", "pub fn main()"));
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain('$andModify($entry(inventory, "apple"), (e) => {');
  expect(js).toContain('$orInsert(inventory, "apple", [0, 0]);');
  expect(js).not.toContain("entry._0");
  expect(js).not.toContain("$mutGet(counts, c);");
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
  expect(js).toContain("  let n;\n  if (flag) {\n    // Yes.\n    n = 1;\n  } else {\n    n = 2;\n  }\n  let k;\n  if (flag) {\n    k = 3;\n  } else {\n    // Four.\n    k = 4;\n  }\n  // One.\n  let m;\n  if (n === 1) {\n    m = 10;\n  } else {\n    m = 20;\n  }\n");
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

// ADR 0279: a callback that only passes what it's given on, `(x) => double(x)`,
// is the function itself, `xs.map(double)`, as react.dev's
// toCommaSeparatedList has `array.map(renderCallback)`, where it's Rust's, which
// takes no more than it's given: a JS function's, `parseFloat`, may read
// the index too.
test("a callback that only passes its arguments on is the function", async () => {
  const dir = fixture("eta");
  writeFileSync(join(dir, "lib.rs"), `#[cfg_attr(rust_js, rust_js::link_name = "parseFloat")] #[allow(unused_variables)] fn parse_float(text: &str) -> f64 { unreachable!() }

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
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain("return xs.map(double);");
  expect(js).toContain("return xs.map(f);");
  expect(js).toContain("return xs.map((x) => parseFloat(x));");
  const lib = await import(join(dir, "mir.js"));
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
  expect(js).toContain(`[tokens.includes("hidden"), tokens.some((token) => token === name)]`);
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
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain("  return `<div>\n  \\`\\${x}\\` \\\\ \"q\"\n</div>`;");
  expect(js).toContain("  return `a\nb`;");
  expect(js).toContain('  return "a\\nb";');
  // A `\` ending a line leaves its break out: the one there is `\n`.
  expect(js).toMatch(/export function continued\(\) \{\n  return "a\\nb";/);
  const lib = await import(join(dir, "mir.js"));
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

// A `const` of a string written across lines, a raw string's, is a template
// literal of them, as an inline one is and as react.dev's
// SandpackWithHTMLOutput writes its sandbox's files; one of `\n`s stays a
// string.
test("a const of a string written across lines is a template literal", async () => {
  const dir = fixture("const-lines");
  writeFileSync(join(dir, "lib.rs"), `const PAGE: &str = r#"<main>
  <h1>Hi</h1>
</main>"#;

const PAIR: &str = "a\\nb";

pub fn page() -> String {
    format!("{PAGE}|{PAIR}")
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("const PAGE = `<main>\n  <h1>Hi</h1>\n</main>`;");
  expect(js).toContain('const PAIR = "a\\nb";');
  const { page } = await import(join(dir, "lib.js"));
  expect(page()).toBe("<main>\n  <h1>Hi</h1>\n</main>|a\nb");
});

// std's `From<bool>` of each number and `From<char>` of `u32`, `u64` and
// `u128` are what `as` makes of them: `flag ? 1 : 0`, `c.codePointAt(0)`,
// a BigInt of 64 bits and more (ADR 0086), as a function too.
test("a number from a bool or a char is its cast", async () => {
  const dir = fixture("from-bool-char");
  writeFileSync(join(dir, "lib.rs"), `pub fn counted(flag: bool) -> usize {
    usize::from(flag)
}

pub fn wide(flag: bool) -> u64 {
    flag.into()
}

pub fn half(flag: bool) -> f64 {
    f64::from(flag)
}

pub fn code(c: char) -> u32 {
    u32::from(c)
}

pub fn big_code(c: char) -> u128 {
    c.into()
}

pub fn all(flags: &[bool]) -> Vec<i8> {
    flags.iter().copied().map(i8::from).collect()
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("return flag ? 1 : 0;");
  expect(js).toContain("return flag ? 1n : 0n;");
  expect(js).toContain("return c.codePointAt(0);");
  const lib = await import(join(dir, "lib.js"));
  expect([lib.counted(true), lib.counted(false), lib.wide(true), lib.half(true), lib.code("é"), lib.bigCode("😀"), lib.all([true, false])])
    .toEqual([1, 0, 1n, 1, 233, 128512n, [1, 0]]);
});

// A trait's default constant of `Self`, in a generic impl's dictionary, is
// its initializer copied into the impl, as a default method is (ADR 0176):
// read on each use, `Self` the impl's type. In a library, every one is,
// never computed where no consumer reads it, as rustc computes one only
// where it's read: `Wrap`'s would divide by zero.
test("a trait's default constant of Self is its initializer, copied into the impl", async () => {
  const dir = fixture("default-const-of-self");
  const source = `pub trait K {
    const A: u32;
    const B: u32 = Self::A + 1;
    const PER: u32 = 360 / Self::A;
    const NAME: &'static str = "k";
}

pub struct W<T>(pub T);

impl<T: K> K for W<T> {
    const A: u32 = T::A;
}

impl K for u8 {
    const A: u32 = 1;
}

pub struct Zero;

impl K for Zero {
    const A: u32 = 0;
}

fn b<S: K>() -> u32 {
    S::B
}

pub fn f() -> u32 {
    b::<W<u8>>()
}
`;
  writeFileSync(join(dir, "lib.rs"), source);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  expect((await import(join(dir, "lib.js"))).f()).toBe(2);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "library", "lib.js"), "--library", "--manifest", join(dir, "library", "manifest.json")]);
  const library = readFileSync(join(dir, "library", "lib.js"), "utf8");
  expect(library).toContain("get PER() {");
  // A literal is the value itself.
  expect(library).toContain('NAME: "k",');
  expect((await import(join(dir, "library", "lib.js"))).f()).toBe(2);
});

// Trait defaults of each other are an error only where they're read, as
// rustc has it (its defaults-cyclic-pass): a default's own `Self::B` isn't
// generic code reading `B`, which would compute `()`'s, a cycle.
test("trait constants defaulting to each other compile where none is read", async () => {
  const dir = fixture("default-const-cycle");
  writeFileSync(join(dir, "lib.rs"), `pub trait Tr {
    const A: u8 = Self::B;
    const B: u8 = Self::A;
}

impl Tr for () {}

impl Tr for u8 {
    const A: u8 = 42;
}

pub fn read() -> (u8, u8) {
    (<u8 as Tr>::A, <u8 as Tr>::B)
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  expect((await import(join(dir, "lib.js"))).read()).toEqual([42, 42]);
});

// A struct updated from one it owns, `..file`, is that one spread, then the
// fields named, as a reference's is (ADR 0250): `{ ...code, hidden: true }`,
// as react.dev's RSC template hides its files. One of a type changed in
// place elsewhere too, moved into the update, as react.dev's
// SandpackWithHTMLOutput passes its props on; a `Copy` one, still there,
// keeps its fields read.
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

// Changed in place elsewhere: moved into the update, nothing changes it.
pub fn show(file: &mut File) {
    file.hidden = None;
}

#[derive(Clone, Copy)]
pub struct Pos {
    pub x: u32,
    pub y: u32,
}

pub fn nudge(pos: &mut Pos) {
    pos.y += 1;
}

pub fn right(pos: Pos) -> Pos {
    Pos { x: 9, ..pos }
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("return { ...file, hidden: true };");
  expect(js).toContain("return { x: 9, y: pos.y };");
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

// ADR 0362: a `RefCell` in a field that's only ever borrowed for a moment,
// where nothing can ask, is never counted, so it's the value it holds, as
// react.dev's nested table of contents pushes onto a node's `children`.
test("a field's RefCell only borrowed for a moment is its value", async () => {
  const dir = fixture("plain-refcell-field");
  writeFileSync(join(dir, "lib.rs"), `use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;

pub struct Item {
    pub depth: u32,
    pub url: String,
}

pub struct Node {
    pub item: Option<Rc<Item>>,
    pub children: RefCell<Vec<Rc<Node>>>,
}

pub fn nested(toc: Vec<Rc<Item>>) -> Rc<Node> {
    let mut ancestors: HashMap<u32, Rc<Node>> = HashMap::new();
    let root = Rc::new(Node { item: None, children: RefCell::new(vec![]) });
    for item in toc {
        let parent = ancestors.get(&(item.depth - 1)).cloned().unwrap_or_else(|| root.clone());
        let node = Rc::new(Node { item: Some(item.clone()), children: RefCell::new(vec![]) });
        parent.children.borrow_mut().push(node.clone());
        ancestors.insert(item.depth, node);
    }
    root
}

pub fn width(node: &Node) -> usize {
    node.children.borrow().len()
}

// A local of the same type is a cell still, its borrows checked.
pub fn both(node: Rc<Node>) -> usize {
    let spare: RefCell<Vec<Rc<Node>>> = RefCell::new(vec![]);
    let reading = spare.borrow();
    spare.borrow_mut().push(node);
    reading.len()
}

// One lent whole stays a cell: what it's lent counts its borrows.
pub struct Held {
    pub values: RefCell<Vec<u32>>,
}

fn first(values: &RefCell<Vec<u32>>) -> Option<u32> {
    values.borrow().first().copied()
}

pub fn held(n: u32) -> Option<u32> {
    let held = Held { values: RefCell::new(vec![n]) };
    first(&held.values)
}

// One bound by a pattern is a cell of its own.
pub struct Bound {
    pub values: RefCell<Vec<u16>>,
}

pub fn bound(n: u16) -> usize {
    let Bound { values } = Bound { values: RefCell::new(vec![n]) };
    values.borrow().len()
}

pub fn peek(bound: &Bound) -> usize {
    bound.values.borrow().len()
}

// One held while something may ask is counted.
pub struct Asked {
    pub values: RefCell<Vec<u8>>,
}

fn ask(asked: &Asked) -> usize {
    asked.values.borrow().len()
}

pub fn asked(n: u8) -> usize {
    let asked = Asked { values: RefCell::new(vec![n]) };
    let guard = asked.values.borrow();
    let len = guard.len() + ask(&asked);
    drop(guard);
    len
}

// One held while its statement calls what may ask is counted.
pub struct Pushed {
    pub values: RefCell<Vec<i16>>,
}

fn count(pushed: &Pushed) -> i16 {
    pushed.values.borrow().len() as i16
}

pub fn pushed(n: i16) {
    let pushed = Pushed { values: RefCell::new(vec![n]) };
    pushed.values.borrow_mut().push(count(&pushed));
}

// One held while its statement drops what may ask is counted.
pub struct Ticked {
    pub values: RefCell<Vec<i8>>,
}

pub struct Tick<'a>(&'a Cell<u32>, i8);

impl Drop for Tick<'_> {
    fn drop(&mut self) {
        self.0.set(self.0.get() + 1);
    }
}

pub fn ticked(n: i8) -> u32 {
    let ticks = Cell::new(0);
    let ticked = Ticked { values: RefCell::new(vec![n]) };
    ticked.values.borrow_mut().push(Tick(&ticks, n).1);
    ticks.get() + ticked.values.borrow().len() as u32
}

// Two shared borrows in one statement ask nothing of each other.
pub struct Shared {
    pub values: RefCell<Vec<f64>>,
}

pub fn shared(n: f64) -> usize {
    let shared = Shared { values: RefCell::new(vec![n]) };
    let len = shared.values.borrow().len() + shared.values.borrow().len();
    len
}

// A derive's reads its fields itself.
#[derive(Clone)]
pub struct Derived {
    pub values: RefCell<Vec<i32>>,
}

pub fn derived(n: i32) -> usize {
    let derived = Derived { values: RefCell::new(vec![n]) };
    let copy = derived.clone();
    copy.values.borrow_mut().push(1);
    copy.values.borrow().len() + derived.values.borrow().len()
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("{ children: [] }");
  expect(js).toContain("parent.children.push(node);");
  expect(js.slice(js.indexOf("export function nested"), js.indexOf("export function both"))).not.toContain("$borrow");
  expect(js).toContain("const shared = { values: [n] };");
  // Each of the others is a cell: lent, bound, counted, held while asked, derived.
  expect(js.match(/\{ values: \{ value: \[n\] \} \}/g)?.length).toBe(6);
  const lib = await import(join(dir, "lib.js"));
  const items = [1, 2, 2, 3, 2].map((depth, i) => ({ depth, url: `#${i}` }));
  const root = lib.nested(items);
  expect([lib.width(root), root.children[0].children.length, root.children[0].children[1].children[0].item.url]).toEqual([1, 3, "#3"]);
  expect([lib.held(4), lib.bound(1), lib.asked(1), lib.derived(1), lib.shared(1), lib.ticked(1)]).toEqual([4, 1, 2, 3, 2, 3]);
  expect(() => lib.both(root)).toThrow("already borrowed");
  expect(() => lib.pushed(1)).toThrow("already mutably borrowed");
});

// ADR 0214: an untagged enum's `Dict` is a plain object, of no class: told
// apart as an object, as a parsed URL's query, text or parsed, is.
test("an untagged enum's dictionary is told apart as an object", async () => {
  const dir = fixture("untagged-dict");
  writeFileSync(join(dir, "lib.rs"), `use js::{Dict, dict};

#[cfg_attr(rust_js, rust_js::untagged)]
pub enum Query {
    Str(String),
    Parsed(&'static Dict<u32>),
}

pub fn told(query: Option<Query>) -> String {
    match query {
        Some(Query::Parsed(parsed)) => format!("parsed {}", dict::get(parsed, "a").copied().unwrap_or(0)),
        Some(Query::Str(text)) => format!("text {text}"),
        None => "none".to_string(),
    }
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).not.toContain("instanceof Dict");
  const lib = await import(join(dir, "lib.js"));
  expect([lib.told("a=1"), lib.told({ a: 2 }), lib.told(undefined)]).toEqual(["text a=1", "parsed 2", "none"]);
});

// An array or an object made only to be dropped is what making it does,
// `loud(1);`, as a person writes it, not `[loud(1)];`.
test("a value made only to be dropped is what making it does", async () => {
  const dir = fixture("dropped-made");
  writeFileSync(join(dir, "lib.rs"), `fn loud(n: u32) -> u32 {
    println!("made {n}");
    n
}

pub fn made() {
    drop(vec![loud(1), 2, loud(3)]);
    drop((loud(4), 5));
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("export function made() {\n  loud(1);\n  loud(3);\n  loud(4);\n}");
});

// ADR 0284: a tag may be `true` or `false`, react-dom's `FormStatus`
// `{ pending: true, data, .. } | { pending: false, .. }`, as ReScript's
// `@as(true)` is.
test("a discriminated union's tag may be a boolean", async () => {
  const dir = fixture("tagged-bool");
  writeFileSync(join(dir, "lib.rs"), `#[cfg_attr(rust_js, rust_js::tag = "pending")]
#[derive(Clone, Debug, PartialEq)]
pub enum Status {
    #[cfg_attr(rust_js, rust_js::name = true)]
    Pending { data: String },
    #[cfg_attr(rust_js, rust_js::name = false)]
    Idle,
}

pub fn made(data: &str) -> Vec<Status> {
    vec![Status::Pending { data: data.to_string() }, Status::Idle]
}

pub fn told(s: &Status) -> String {
    match s {
        Status::Pending { data } => format!("sending {data}"),
        Status::Idle => "idle".to_string(),
    }
}

pub fn same(a: &Status, b: &Status) -> bool {
    a.clone() == *b
}

pub fn shown(s: &Status) -> String {
    format!("{s:?}")
}

const IDLE: Status = Status::Idle;

pub fn constant() -> Status {
    IDLE
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("{ pending: true, data }");
  expect(js).toContain("if (s.pending === true) {");
  const lib = await import(join(dir, "lib.js"));
  expect(lib.made("a")).toEqual([{ pending: true, data: "a" }, { pending: false }]);
  expect(lib.constant()).toEqual({ pending: false });
  // What JS made, as useFormStatus does, is read as Rust's.
  expect([lib.told({ pending: true, data: "b" }), lib.told({ pending: false, data: null })]).toEqual(["sending b", "idle"]);
  expect([lib.same({ pending: true, data: "b" }, { pending: true, data: "b" }), lib.same({ pending: false }, { pending: true, data: "b" })])
    .toEqual([true, false]);
  expect(lib.shown({ pending: false })).toBe("Idle");
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

// ADR 0035: a JS call whose throw is an `Err`, its result ignored, is
// `try { call; } catch {}`, as a person writes it, not `$try(() => call)`.
test("an ignored result of a call that throws is try and an empty catch", async () => {
  const dir = fixture("ignored-try");
  writeFileSync(join(dir, "lib.rs"), `use js::JsError;

#[cfg_attr(rust_js, rust_js::link_name = "globalThis.risky")]
#[allow(unused_variables)]
fn risky(n: f64) -> Result<f64, &'static JsError> {
    unreachable!()
}

pub fn ignored(n: f64) {
    let _ = risky(n);
}

pub fn twice(n: f64) {
    let _ = risky(n);
    _ = risky(n + 1.0);
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).not.toContain("$try");
  expect(js).toContain("export function ignored(n) {\n  try {\n    globalThis.risky(n);\n  } catch {}\n}");
  const lib = await import(join(dir, "lib.js"));
  const calls: number[] = [];
  (globalThis as any).risky = (n: number) => {
    calls.push(n);
    if (n < 0) throw new Error("negative");
    return n;
  };
  lib.ignored(-1);
  lib.twice(-2);
  expect(calls).toEqual([-1, -2, -1]);
});

// `set X.y` of a function without a receiver writes a global's property,
// `process.exitCode = 1`, as `get X.y` reads one.
test("a binding sets a static property", async () => {
  const dir = fixture("static-setter");
  writeFileSync(join(dir, "lib.rs"), `#[cfg_attr(rust_js, rust_js::link_name = "get globalThis.counter")]
fn counter() -> f64 {
    unreachable!()
}

#[cfg_attr(rust_js, rust_js::link_name = "set globalThis.counter")]
#[allow(unused_variables)]
fn set_counter(value: f64) {
    unreachable!()
}

pub fn bump() -> f64 {
    set_counter(counter() + 1.0);
    counter()
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  expect(readFileSync(join(dir, "lib.js"), "utf8")).toContain("globalThis.counter += 1;");
  const lib = await import(join(dir, "lib.js"));
  (globalThis as any).counter = 1;
  expect(lib.bump()).toBe(2);
});

// ADR 0285: `std::ptr::eq` of JS objects is whether they're one, `===`.
test("ptr::eq of JS objects is whether they're one", async () => {
  const dir = fixture("ptr-eq");
  writeFileSync(join(dir, "lib.rs"), `use js::{Date, date};

pub fn same(a: &Date, b: &Date) -> bool {
    std::ptr::eq(a, b)
}

pub fn made() -> (bool, bool) {
    let a = date::new(Some(&0.0));
    let b = date::new(Some(&0.0));
    (core::ptr::eq(a, a), std::ptr::eq(a as *const Date, b))
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  expect(readFileSync(join(dir, "mir.js"), "utf8")).toContain("return a === b;");
  const lib = await import(join(dir, "mir.js"));
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
  expect(js).toContain('      value.name = "new";\n      return show(value);');
  expect(js).toContain("const value$1 = $try(() => JSON.parse(text));");
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
  writeFileSync(join(dir, "lib.rs"), `#[cfg_attr(rust_js, rust_js::link_name = "globalThis.onLoadSeen")] #[allow(unused_variables)] fn seen(n: u32) { unreachable!() }
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

// ADR 0364: what MIR makes of a branch reads as a person writes it:
// `a && b`, `a || b`, `matches!`'s `n === "x"`, `c ? a : b` of two values,
// and no check of a constant divisor.
test("branches from MIR read as a person writes them", async () => {
  const dir = fixture("mir-branches");
  writeFileSync(join(dir, "lib.rs"), `pub fn both(a: u32, b: u32) -> bool {
    a > 1 && b > 2
}
pub fn either(a: u32, b: u32) -> bool {
    a > 1 || b > 2
}
pub fn named(n: Option<&str>) -> bool {
    matches!(n, Some("x"))
}
pub fn sign(n: i32) -> &'static str {
    let s = if n < 0 { "-" } else { "+" };
    s
}
pub fn odd(i: u32) -> bool {
    i % 2 == 1
}
pub fn flags(home: bool, index: bool, section: &str) -> (bool, bool) {
    let mut toc = true;
    let mut cols = true;
    if home || index {
        toc = false;
    } else if section == "blog" {
        toc = false;
        cols = false;
    }
    (toc, cols)
}
pub fn guard(a: u32, s: &str) -> bool {
    if a > 1 { s == "x" } else { false }
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain("return a > 1 && b > 2;");
  expect(js).toContain("return a > 1 || b > 2;");
  expect(js).toContain('return n === "x";');
  expect(js).toContain('n < 0 ? "-" : "+"');
  expect(js).not.toContain("2 === 0");
  expect(js).toContain('  if (home || index) {\n    toc = false;\n  } else if (section === "blog") {\n');
  // A branch of `false`, `&&`: as MIR has it, tighter than an `if`.
  expect(js).toContain('return a > 1 && s === "x";');
  const lib = await import(join(dir, "mir.js"));
  expect([lib.both(2, 3), lib.either(0, 1), lib.named("x"), lib.named(undefined), lib.sign(-1), lib.odd(3), lib.flags(false, false, "blog")]).toEqual([true, false, true, false, "-", true, [false, false]]);
});

// A `for` loop from MIR (ADR 0364) is the `for…of` or counted `for` a
// person writes, not the `while (true)` stepping an iterator MIR has.
test("loops from MIR read as a person writes them", async () => {
  const dir = fixture("mir-loops");
  writeFileSync(join(dir, "lib.rs"), `pub fn counted(items: &[u32]) -> u32 {
    let mut n = 0;
    for x in items {
        if *x > 2 {
            n += 1;
        }
    }
    n
}
pub fn found(items: &[u32]) -> Option<u32> {
    for &x in items {
        if x > 2 {
            return Some(x);
        }
    }
    None
}
pub fn pairs(items: &[(u32, u32)]) -> u32 {
    let mut n = 0;
    for (a, b) in items {
        n += a * b;
    }
    n
}
pub fn upto(k: u32) -> u32 {
    let mut n = 0;
    for i in 0..k {
        n += i;
    }
    n
}
pub fn stops(items: &[u32]) -> u32 {
    let mut n = 0;
    for x in items {
        if *x == 0 {
            break;
        }
        n += x;
    }
    n
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain("  for (const x of items) {\n    if (x > 2) {\n      n = ");
  expect(js).toContain("  for (const x of items) {\n    if (x > 2) {\n      return x;\n");
  expect(js).toContain("  for (const [a, b] of items) {\n");
  expect(js).toContain("  for (let i = 0; i < k; i++) {\n");
  expect(js).toContain("    if (x === 0) {\n      break;\n    }\n");
  expect(js).not.toContain("while (true)");
  expect(js).not.toContain("$iter");
  const lib = await import(join(dir, "mir.js"));
  expect([lib.counted([1, 3, 4]), lib.found([1, 3]), lib.found([1]), lib.pairs([[1, 2], [3, 4]]), lib.upto(4), lib.stops([1, 2, 0, 5])]).toEqual([2, 3, undefined, 14, 6, 3]);
});

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

// A temporary MIR (ADR 0364) makes of a variable nothing changes, or of a
// constant, is what it holds where it's read: of `assert_eq!`'s pair, each
// value compared. Its check, a branch that panics, comes first, as a person
// writes it.
test("a temporary from MIR of what reads the same anywhere is what it holds", async () => {
  const dir = fixture("mir-aliases");
  writeFileSync(join(dir, "lib.rs"), `use std::any::type_name;
pub fn describe<T>(_: &T) -> String {
    format!("{} {}", type_name::<T>(), size_of::<T>())
}
pub fn checked(n: u32) -> u32 {
    assert_eq!(n, 6, "not six");
    n
}
pub fn small(n: u32) -> u32 {
    if n > 9 {
        panic!("big");
    }
    n
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain("  return `${TName} ${TSize}`;\n");
  expect(js).toContain("  if (n !== 6) {\n    ");
  expect(js).toContain("  }\n  return n;\n");
  expect(js).toContain("  if (n > 9) {\n    ");
  const lib = await import(join(dir, "mir.js"));
  expect(lib.checked(6)).toBe(6);
  expect(() => lib.checked(7)).toThrow();
  expect(lib.small(9)).toBe(9);
});

// An iterator chain of an array MIR (ADR 0364) makes as Rust runs it, item
// by item, is the array's methods (ADR 0055) where the order its closures
// run in can't be told apart, a closure's pattern taken apart where it's
// given; where two of them do something, it stays a JS iterator's.
test("iterator chains from MIR are array methods where order can't be told", async () => {
  const dir = fixture("mir-chains");
  writeFileSync(join(dir, "lib.rs"), `use std::cell::RefCell;
pub struct Crumb { pub title: String, pub hide: bool }
pub fn titles(items: &[Crumb]) -> Vec<String> {
    items.iter().enumerate().map(|(i, c)| format!("{i}:{}", c.title)).collect()
}
pub fn shown(items: &[Crumb]) -> Vec<&Crumb> {
    items.iter().filter(|c| !c.hide).collect()
}
pub fn total(items: &[u32]) -> u32 {
    items.iter().fold(0, |a, b| a + b)
}
pub fn any_big(items: &[u32]) -> bool {
    items.iter().any(|x| *x > 9)
}
// Read alone, not copied.
pub fn top(items: &[u32]) -> u32 {
    items.iter().copied().max().unwrap_or(0)
}
// Its last items, the array reversed, not copied first.
pub fn last_two(items: &[u32]) -> Vec<u32> {
    items.iter().rev().take(2).copied().collect()
}
// Each item's items, and the words of text, as arrays.
pub fn flat(groups: &[Vec<u32>]) -> Vec<u32> {
    groups.iter().flat_map(|g| g.iter().map(|x| x * 10)).collect()
}
pub fn words(s: &str) -> String {
    s.split_whitespace().map(|w| w.to_uppercase()).collect::<Vec<_>>().join(" ")
}
// Some in the middle, and a char's capital, a string already.
pub fn mid(v: &[u32]) -> Vec<u32> {
    v.iter().skip(2).take(3).copied().collect()
}
pub fn up(c: char) -> String {
    c.to_uppercase().collect()
}
// A function given as a value, its parameter named by its type.
pub fn lens(groups: &[Vec<u32>]) -> Vec<usize> {
    groups.iter().map(Vec::len).collect()
}
pub fn sizes(words: &[&str]) -> Vec<usize> {
    words.iter().copied().map(str::len).collect()
}
// What a helper makes an array of, each window, mapped as one.
pub fn diffs(v: &[i32]) -> Vec<i32> {
    v.windows(2).map(|w| w[1] - w[0]).collect()
}
// Two arrays' items side by side.
pub fn pairs(a: &[&str], b: &[u32]) -> Vec<String> {
    a.iter().zip(b.iter()).map(|(s, n)| format!("{s}{n}")).collect()
}
// Closures that only read, a string's \`find\`, or set their own
// variables, beside one that calls out, as react.dev's console filters its
// logs: arrays' methods still.
fn marked(s: &str) -> String {
    format!("<{s}>")
}
pub fn logs(lines: &[&str]) -> Vec<String> {
    lines.iter().filter(|l| l.find("error").is_none()).map(|l| marked(l)).collect()
}
pub fn signs(xs: &[i32]) -> Vec<String> {
    xs.iter()
        .map(|&x| {
            let sign;
            if x < 0 {
                sign = "-";
            } else {
                sign = "+";
            }
            sign
        })
        .map(|s| marked(s))
        .collect()
}
pub fn order(items: &[u32]) -> Vec<String> {
    let log = RefCell::new(Vec::new());
    let _ = items
        .iter()
        .map(|x| {
            log.borrow_mut().push(format!("a{x}"));
            x
        })
        .filter(|x| {
            log.borrow_mut().push(format!("b{x}"));
            true
        })
        .count();
    log.into_inner()
}
pub fn stops(items: &[u32]) -> Vec<String> {
    let log = RefCell::new(Vec::new());
    let _ = items
        .iter()
        .map(|x| {
            log.borrow_mut().push(format!("a{x}"));
            x
        })
        .any(|x| *x > 1);
    log.into_inner()
}
pub fn taken(items: &[u32]) -> Vec<String> {
    let log = RefCell::new(Vec::new());
    let _ = items
        .iter()
        .map(|x| {
            log.borrow_mut().push(format!("a{x}"));
            x
        })
        .take(1)
        .count();
    log.into_inner()
}
pub fn copied(mut items: Vec<u32>) -> (Vec<u32>, Vec<u32>) {
    let kept: Vec<u32> = items.iter().copied().collect();
    items.push(9);
    (kept, items)
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain("  return items.map((c, i) => `${i}:${c.title}`);\n");
  expect(js).toContain("  return items.filter((c) => !c.hide);\n");
  expect(js).toContain("  return items.reduce((a, b) => (a + b) >>> 0, 0);\n");
  expect(js).toContain("  return items.some((x) => x > 9);\n");
  expect(js).toContain("  return $max(items) ?? 0;\n");
  expect(js).toContain("  return items.toReversed().slice(0, 2);\n");
  expect(js).toContain("  return groups.flatMap((g) => g.map((x) => Math.imul(x, 10) >>> 0));\n");
  expect(js).toContain(".filter((word) => word !== \"\")\n    .map((w) => w.toUpperCase())\n    .join(\" \");");
  expect(js).toContain("  return v.slice(2).slice(0, 3);\n");
  expect(js).toContain("  return c.toUpperCase();\n");
  expect(js).toContain("  return groups.map((x) => x.length);\n");
  expect(js).toContain("words.map((s) => $byteLen(s))");
  expect(js).toContain("  return $windows(v, 2).map(");
  expect(js).toContain("  return $zip(a, b).map(");
  expect(js).toMatch(/return lines\s*\.filter\(/);
  expect(js).toMatch(/return xs\s*\.map\(/);
  const lib = await import(join(dir, "mir.js"));
  expect([lib.top([2, 7, 3]), lib.top([])]).toEqual([7, 0]);
  expect([lib.lastTwo([1, 2, 3]), lib.lastTwo([1])]).toEqual([[3, 2], [1]]);
  expect([lib.flat([[1, 2], [], [3]]), lib.words(" a  b ")]).toEqual([[10, 20, 30], "A B"]);
  expect([lib.mid([1, 2, 3, 4, 5, 6]), lib.mid([1]), lib.up("ß")]).toEqual([[3, 4, 5], [], "SS"]);
  expect(lib.diffs([1, 4, 9])).toEqual([3, 5]);
  expect(lib.pairs(["x", "y", "z"], [1, 2])).toEqual(["x1", "y2"]);
  expect([lib.logs(["ok", "an error", "fine"]), lib.signs([1, -2])]).toEqual([["<ok>", "<fine>"], ["<+>", "<->"]]);
  const crumbs = [{ title: "a", hide: false }, { title: "b", hide: true }];
  expect([lib.titles(crumbs), lib.shown(crumbs), lib.total([1, 2]), lib.anyBig([3, 10])]).toEqual([["0:a", "1:b"], [crumbs[0]], 3, true]);
  expect(lib.order([1, 2])).toEqual(["a1", "b1", "a2", "b2"]);
  expect(lib.stops([1, 2, 3])).toEqual(["a1", "a2"]);
  expect(lib.taken([1, 2, 3])).toEqual(["a1"]);
  expect(lib.copied([1])).toEqual([[1], [1, 9]]);
});

// `if let`s chained with `&&`, and loops, from MIR (ADR 0364), as written:
// one condition's tests one `&&`, a variant's binding the place it reads
// where that reads the same, a tuple's parts in place, a loop's labeled
// block its `continue`; and `if`s the program nests, nested.
test("if-let chains and loops from MIR read as written", async () => {
  const dir = fixture("mir-let-chains");
  writeFileSync(join(dir, "lib.rs"), `use std::cell::Cell;
pub struct Route { pub title: Option<String>, pub path: Option<String>, pub routes: Option<Vec<Route>> }
pub fn pages(routes: &[Route]) -> Vec<String> {
    let mut out = Vec::new();
    for route in routes {
        if let Some(title) = &route.title
            && let Some(children) = &route.routes
            && !children.is_empty()
        {
            out.push(format!("{title}:{}", children.len()));
        } else if let (Some(title), Some(path)) = (&route.title, &route.path) {
            out.push(format!("{title}={path}"));
        }
    }
    out
}
pub fn sections(routes: &[Route]) -> Vec<String> {
    let mut out = Vec::new();
    for route in routes {
        if route.path.as_deref().is_some_and(|p| p.starts_with("http")) {
            continue;
        }
        if let Some(title) = &route.title
            && let Some(children) = &route.routes
            && !children.is_empty()
        {
            for child in children {
                if let Some(t) = &child.title {
                    out.push(format!("{title}/{t}"));
                }
            }
            continue;
        }
        if let Some(path) = &route.path {
            out.push(path.clone());
        }
    }
    out
}
pub fn nested(a: Option<u32>, b: bool) -> u32 {
    let mut n = 0;
    if let Some(x) = a {
        if b {
            n = x;
        }
    }
    n
}
// A closure's, by its own THIR.
pub fn joined(routes: &[Route]) -> Vec<String> {
    routes
        .iter()
        .map(|r| {
            if let Some(t) = &r.title
                && let Some(p) = &r.path
            {
                format!("{t}{p}")
            } else {
                String::new()
            }
        })
        .collect()
}
pub struct Item { pub kind: &'static str, pub valid: bool }
fn valid(i: &Item) -> Option<&Item> {
    if i.valid { Some(i) } else { None }
}
pub fn kept(items: &[Item]) -> usize {
    items.iter().filter(|c| !matches!(valid(c), Some(e) if e.kind == "pre")).count()
}
// Data whose fields may change, which a value is read out of first, or a
// reference to a field of, which reads the field where it's used.
pub struct Named { pub title: Option<String>, pub seen: Cell<bool> }
pub fn titled(nodes: &[Named]) -> Vec<String> {
    let mut out = Vec::new();
    for node in nodes {
        node.seen.set(true);
        if let Some(title) = &node.title
            && title.len() > 1
        {
            out.push(title.clone());
        }
    }
    out
}
pub struct Node { pub count: Option<u32>, pub items: Vec<u32>, pub seen: Cell<bool> }
pub fn counts(nodes: &[Node]) -> u32 {
    let mut n = 0;
    for node in nodes {
        if node.seen.get()
            && let Some(c) = node.count
            && c > 1
        {
            n += c;
        } else {
            n += 1;
        }
    }
    n
}
pub fn totals(nodes: &[Node]) -> u32 {
    let mut n = 0;
    for node in nodes {
        node.seen.set(true);
        for item in &node.items {
            n += item;
        }
    }
    n
}
pub fn chained(nodes: &[Node]) -> u32 {
    let mut n = 0;
    for node in nodes {
        if node.seen.get()
            && let Some(c) = node.count
        {
            for item in &node.items {
                n += item * c;
            }
            continue;
        }
        n += 1;
    }
    n
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain(`  for (const route of routes) {
    if (route.title != null && route.routes && route.routes.length !== 0) {
      out.push(\`\${route.title}:\${route.routes.length}\`);
    } else if (route.title != null && route.path != null) {
      out.push(\`\${route.title}=\${route.path}\`);
    }
  }
`);
  expect(js).toContain(`  for (const route of routes) {
    if (route.path != null && route.path.startsWith("http")) {
      continue;
    }
    if (route.title != null && route.routes && route.routes.length !== 0) {
      for (const child of route.routes) {
        if (child.title != null) {
          out.push(\`\${route.title}/\${child.title}\`);
        }
      }
      continue;
    }
    if (route.path != null) {
      out.push(route.path);
    }
  }
`);
  expect(js).toContain("  if (a != null) {\n    if (b) {\n");
  expect(js).toContain("    if (node.seen && node.count != null && node.count > 1) {\n");
  expect(js).not.toContain("let iter");
  expect(js).not.toMatch(/bb\d+:/);
  expect(js).toMatch(/r\.title != null && r\.path != null/);
  expect(js).toContain("    if (node.title != null && $byteLen(node.title) > 1) {\n      out.push(node.title);\n");
  const lib = await import(join(dir, "mir.js"));
  const routes = [
    { title: "a", path: "x", routes: [{ title: "c" }] },
    { title: "b", path: "y", routes: [] },
    { path: "http://z" },
  ];
  expect([lib.pages(routes), lib.sections(routes), lib.nested(3, true), lib.nested(3, false)]).toEqual([["a:1", "b=y"], ["a/c", "y"], 3, 0]);
  const nodes = () => [{ count: 3, items: [1, 2], seen: true }, { count: undefined, items: [5], seen: false }];
  expect([lib.counts(nodes()), lib.totals(nodes()), lib.chained(nodes())]).toEqual([4, 8, 10]);
  expect(lib.joined(routes)).toEqual(["ax", "by", ""]);
  expect(lib.titled([{ title: "ab", seen: false }, { title: "a", seen: false }])).toEqual(["ab"]);
  expect(lib.kept([{ kind: "pre", valid: true }, { kind: "pre", valid: false }, { kind: "p", valid: true }])).toBe(2);
});

// What a loop's recovery (ADR 0364) no longer shows: an iterator stepped by
// hand, a std one and a generic one given an array, each a JS iterator; a
// \`None\` of a \`Duration\` tested \`== null\`, as \`ZERO\` is falsy; and a
// \`const\` another takes is that one, from MIR.
test("iterators stepped by hand, a Duration's None and a const another takes", async () => {
  const dir = fixture("mir-by-hand");
  writeFileSync(join(dir, "lib.rs"), `use std::time::Duration;
pub struct B { pub a: u32, pub c: u32 }
pub fn moved() -> u32 {
    let whole = B { a: 5, c: 6 };
    let mut moved = whole;
    moved.c = 7;
    moved.a + moved.c
}
pub fn second(s: &str) -> Option<u8> {
    let mut b = s.bytes();
    b.next();
    b.next()
}
fn first<I: IntoIterator<Item = u32>>(items: I) -> Option<u32> {
    items.into_iter().next()
}
pub fn first_of(v: Vec<u32>) -> Option<u32> {
    first(v)
}
pub fn zero(a: u64, b: u64) -> bool {
    let d = Duration::from_secs(a).checked_sub(Duration::from_secs(b));
    if let Some(x) = d { x.as_secs() == 0 } else { false }
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain("  const moved = { a: 5, c: 6 };\n");
  const lib = await import(join(dir, "mir.js"));
  expect([lib.moved(), lib.second("ab"), lib.firstOf([4, 5]), lib.zero(5n, 5n), lib.zero(4n, 5n)]).toEqual([12, 98, 4, true, false]);
});

// A function that returns nothing, from MIR (ADR 0364), returns where the
// program wrote \`return\` alone: what falls off its end needs none, and an
// \`if\` MIR made a guard of is the \`if\` the program wrote.
test("a function returning nothing returns where the program wrote it", async () => {
  const dir = fixture("mir-unit-returns");
  writeFileSync(join(dir, "lib.rs"), `pub fn explicit(x: bool, v: &mut Vec<u32>) {
    if x {
        return;
    }
    v.push(1);
}
pub fn nested(a: Option<u32>, b: bool, out: &mut Vec<u32>) {
    if let Some(x) = a {
        if b {
            out.push(x);
        }
    }
}
pub fn guarded(flag: bool, cur: Option<&str>, out: &mut Vec<String>) {
    if flag && let Some(c) = cur {
        if c.len() > 2 {
            out.push(c.to_string());
        }
    }
}
pub fn one(n: u32, v: &mut Vec<u32>) {
    match n {
        1 => v.push(1),
        _ => {}
    }
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain(`export function explicit(x, v) {
  if (x) {
    return;
  }
  v.push(1);
}

export function nested(a, b, out) {
  if (a != null) {
    if (b) {
      out.push(a);
    }
  }
}

export function guarded(flag, cur, out) {
  if (flag && cur != null) {
    if ($byteLen(cur) > 2) {
      out.push(cur);
    }
  }
}

export function one(n, v) {
  if (n === 1) {
    v.push(1);
  }
}
`);
  const lib = await import(join(dir, "mir.js"));
  const [v, w, x]: unknown[][] = [[], [], []];
  lib.explicit(false, v);
  lib.nested(3, true, w);
  lib.guarded(true, "abc", x);
  expect([v, w, x]).toEqual([[1], [3], ["abc"]]);
});

// A string's parts from MIR (ADR 0364) are an array: its adapters, \`take\`
// and \`rev\` an array's; a guard returns first, the smaller branch; an
// \`if\`/\`else\` of a \`return\` in each branch is one conditional, as its
// value is; an element THIR's facts know is there is read unchecked, in a
// closure too.
test("a string's parts from MIR are an array", async () => {
  const dir = fixture("mir-split");
  writeFileSync(join(dir, "lib.rs"), `pub fn groups(group: &str) -> Vec<&str> {
    group.split(',').map(|g| g.trim()).filter(|g| !g.is_empty()).collect()
}
pub fn second(s: &str) -> Option<&str> {
    let mut parts = s.split(',');
    parts.next();
    parts.next()
}
pub fn badge(group: &str) -> Option<String> {
    let parts: Vec<&str> = group.split(',').collect();
    if parts.is_empty() {
        return None;
    }
    Some(format!("{} and {} more of {}", parts[0], parts.len() - 1, group.to_uppercase()))
}
pub fn last_two(s: &str) -> Vec<&str> {
    s.split(',').rev().take(2).collect()
}
pub fn heads(vs: &[Vec<u32>]) -> Vec<u32> {
    vs.iter().map(|v| if v.is_empty() { 0 } else { v[0] }).collect()
}
pub fn shown(group: &str) -> Option<String> {
    if group.is_empty() {
        return None;
    }
    Some(group.repeat(3).to_uppercase().trim().to_lowercase())
}
pub fn pick(c: bool, n: u32) -> u32 {
    if c {
        return n + 1;
    } else {
        return n * 2;
    }
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain('  return group\n    .split(",")\n    .map((g) => $trim(g))\n    .filter(Boolean);\n');
  expect(js).toContain('  const parts = group.split(",");\n  if (parts.length === 0) {\n    return;\n  }\n  return \`\${parts[0]} and ');
  expect(js).toContain('  return s.split(",").toReversed().slice(0, 2);\n');
  expect(js).toContain("v[0]");
  // A \`return\` the program wrote stays one, the smaller branch first.
  expect(js).toContain("  if (!group) {\n    return;\n  }\n  return $trim(");
  expect(js).toContain("  if (c) {\n    return (n + 1) >>> 0;\n  }\n  return Math.imul(n, 2) >>> 0;\n");
  expect(js).not.toContain("$index");
  const lib = await import(join(dir, "mir.js"));
  expect([lib.groups("a, b,"), lib.second("x,y"), lib.badge("p,q"), lib.lastTwo("1,2,3"), lib.heads([[], [7]]), lib.shown("ab"), lib.pick(false, 3)]).toEqual([["a", "b"], "y", "p and 1 more of P,Q", ["3", "2"], [0, 7], "ababab", 6]);
});

// ADR 0212: a props field's default, \`#[rust_js::default]\`, is where JS
// takes the props apart, \`{ className = "language-js" }\`: a
// field left out reads its default.
test("a props field's default is read where the props are taken apart", async () => {
  const dir = fixture("mir-prop-defaults");
  writeFileSync(join(dir, "lib.rs"), `pub struct CodeProps<'a> {
    #[rust_js::default = "language-js"]
    pub class_name: &'a str,
    #[rust_js::default = ""]
    pub children: &'a str,
    pub meta: Option<&'a str>,
}
pub fn describe(props: &CodeProps) -> String {
    let CodeProps { class_name, children: code, meta } = props;
    format!("{class_name}:{}:{}", code.len(), meta.unwrap_or("-"))
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain('  const { className = "language-js", children: code = "", meta } = props;\n');
  const lib = await import(join(dir, "mir.js"));
  expect([lib.describe({}), lib.describe({ className: "css", children: "ab", meta: "m" })]).toEqual(["language-js:0:-", "css:2:m"]);
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

// Loops one after another from MIR (ADR 0364), the second in what the
// first leaves to, each item named as the program names it: each a
// \`for…of\`, the innermost first; a map's \`insert\` nothing reads, \`set\`;
// and a \`let\` the loops held, a \`const\`.
test("loops one after another from MIR are for…of loops", async () => {
  const dir = fixture("mir-loops-in-turn");
  writeFileSync(join(dir, "lib.rs"), `use std::collections::HashMap;
pub struct D { pub line: u32, pub class_name: &'static str }
pub fn decorate(lines: &[D], inline: &[D]) -> u32 {
    let mut highlighted = HashMap::new();
    for decorator in lines {
        highlighted.insert(decorator.line, decorator.class_name);
    }
    let mut starts = HashMap::new();
    for decorator in inline {
        if starts.contains_key(&decorator.line) {
            panic!("already at {}", decorator.line);
        }
        starts.insert(decorator.line, decorator.class_name);
    }
    (highlighted.len() + starts.len()) as u32
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain(`export function decorate(lines, inline) {
  const highlighted = new Map();
  for (const decorator of lines) {
    highlighted.set(decorator.line, decorator.className);
  }
  const starts = new Map();
  for (const decorator of inline) {
    if (starts.has(decorator.line)) {
      throw new Error(\`already at \${decorator.line}\`);
    }
    starts.set(decorator.line, decorator.className);
  }
  return (highlighted.size + starts.size) >>> 0;
}`);
  const lib = await import(join(dir, "mir.js"));
  expect(lib.decorate([{ line: 1, className: "a" }], [{ line: 2, className: "b" }])).toBe(2);
});

// From MIR (ADR 0364): a \`return\` the program wrote first, as written; a
// range's loop over an array's indices, whose length it only reads, a
// counted \`for\`; its index, checked by THIR's facts, read unchecked.
test("early returns and index loops from MIR read as written", async () => {
  const dir = fixture("mir-early-returns");
  writeFileSync(join(dir, "lib.rs"), `pub fn get_type(message: &str) -> &'static str {
    if message == "log" || message == "info" {
        return "info";
    }
    if message == "warn" {
        return "warning";
    }
    "error"
}
pub fn joined(args: &[String], mut formatted: String) -> String {
    if !args.is_empty() {
        for i in 0..args.len() {
            formatted += &format!(" {}", args[i]);
        }
    }
    formatted
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain('  if (message === "warn") {\n    return "warning";\n  }\n  return "error";\n');
  expect(js).toContain("    for (let i = 0; i < args.length; i++) {\n      formatted += ` ${args[i]}`;\n");
  const lib = await import(join(dir, "mir.js"));
  expect([lib.getType("warn"), lib.getType("x"), lib.joined(["a", "b"], "f")]).toEqual(["warning", "error", "f a b"]);
});

// ADR 0250: a struct update of a base nothing changes is that base spread,
// `{ ...attrs, title }`: what the base doesn't hold stays out, where a
// field read of it each would be there.
test("a struct update of a base nothing changes is the base spread", async () => {
  const dir = fixture("struct-update-spread");
  writeFileSync(join(dir, "lib.rs"), `#[derive(Clone, Copy)]
pub struct Attrs<'a> {
    pub title: Option<&'a str>,
    pub rel: Option<&'a str>,
    pub id: Option<&'a str>,
}
pub struct Link<'a> {
    pub attrs: Attrs<'a>,
}
pub fn retitled<'a>(attrs: Attrs<'a>, title: &'a str) -> Attrs<'a> {
    Attrs { title: Some(title), ..attrs }
}
pub fn reread<'a>(link: &Link<'a>, title: &'a str) -> Attrs<'a> {
    Attrs { title: Some(title), ..link.attrs }
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain("return { ...attrs, title };");
  const lib = await import(join(dir, "mir.js"));
  expect(Object.keys(lib.retitled({ id: "a" }, "t"))).toEqual(["id", "title"]);
  expect(lib.reread({ attrs: { rel: "r" } }, "t")).toEqual({ rel: "r", title: "t" });
});

// ADR 0308: a function written in a body is declared there, where what
// uses it reads it.
test("a function written in a body is declared in it", async () => {
  const dir = fixture("local-functions");
  writeFileSync(join(dir, "lib.rs"), `pub fn doubled(n: u32) -> u32 {
    fn twice(n: u32) -> u32 {
        n * 2
    }
    let apply = |f: fn(u32) -> u32| f(n);
    if n > 10 {
        fn big() -> u32 {
            1000
        }
        return big();
    }
    apply(twice)
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const lib = await import(join(dir, "mir.js"));
  expect([lib.doubled(3), lib.doubled(11)]).toEqual([6, 1000]);
});

// ADR 0204: a struct with flattened fields made outside JSX is one object of
// their fields, `{ ...template, ...files }`.
test("a struct of flattened fields made outside JSX is one object", async () => {
  const dir = fixture("flattened-made");
  writeFileSync(join(dir, "lib.rs"), `pub struct Inner {
    pub a: u32,
}
pub struct Files<'a> {
    #[cfg_attr(rust_js, rust_js::flatten)]
    pub template: &'a Inner,
    #[cfg_attr(rust_js, rust_js::flatten)]
    pub files: &'a js::Unknown,
}
pub fn made<'a>(template: &'a Inner, files: &'a js::Unknown) -> Files<'a> {
    Files { template, files }
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  const lib = await import(join(dir, "mir.js"));
  expect(lib.made({ a: 1 }, { b: 2 })).toEqual({ a: 1, b: 2 });
});

// An insert at the front, which no length refuses, is `unshift`, as
// react.dev's prepareMDX puts its Overview first.
test("an insert at the front is unshift", async () => {
  const dir = fixture("insert-front");
  writeFileSync(join(dir, "lib.rs"), `pub fn first(mut anchors: Vec<String>) -> Vec<String> {
    if !anchors.is_empty() {
        anchors.insert(0, "Overview".to_string());
    }
    anchors
}
pub fn at(mut v: Vec<u32>, i: usize) -> Vec<u32> {
    v.insert(i, 9);
    v
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain('anchors.unshift("Overview");');
  expect(js).toContain("$insertAt(v, i, 9);");
  const lib = await import(join(dir, "mir.js"));
  expect([lib.first(["a"]), lib.first([]), lib.at([1, 2], 1)]).toEqual([["Overview", "a"], [], [1, 9, 2]]);
  expect(() => lib.at([1], 3)).toThrow();
});

// Arrays of arrays flattened are `flat()`, as react.dev's page route gathers
// each directory's Markdown, `files.flat().filter(..)`; of sets, whose items
// `flat()` wouldn't take apart, or in Rust's order, each item stepped.
test("arrays of arrays flattened are flat()", async () => {
  const dir = fixture("flatten-arrays");
  writeFileSync(join(dir, "lib.rs"), `use std::cell::RefCell;
use std::collections::BTreeSet;
pub fn markdown(groups: Vec<Vec<String>>) -> Vec<String> {
    groups.into_iter().flatten().filter(|s| s.ends_with(".md")).collect()
}
pub fn all(groups: Vec<Vec<u32>>) -> Vec<u32> {
    groups.into_iter().flatten().collect()
}
pub fn of_sets(groups: Vec<BTreeSet<u32>>) -> Vec<u32> {
    groups.into_iter().flatten().collect()
}
pub fn logged(groups: Vec<Vec<u32>>) -> (Vec<u32>, Vec<String>) {
    let log = RefCell::new(Vec::new());
    let kept = groups
        .into_iter()
        .flatten()
        .map(|n| {
            log.borrow_mut().push(format!("m{n}"));
            n
        })
        .filter(|n| {
            log.borrow_mut().push(format!("f{n}"));
            *n > 1
        })
        .collect();
    (kept, log.into_inner())
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain('return groups.flat().filter((s) => s.endsWith(".md"));');
  expect(js).toContain("return groups.flat();");
  expect(js).not.toContain("$iter");
  const lib = await import(join(dir, "mir.js"));
  expect([lib.markdown([["a.md", "b"], ["c.md"]]), lib.all([[1], [2, 3]])]).toEqual([["a.md", "c.md"], [1, 2, 3]]);
  expect(lib.logged([[1, 2], [3]])).toEqual([[2, 3], ["m1", "f1", "m2", "f2", "m3", "f3"]]);
  expect(lib.ofSets([new Set([1, 2]), new Set([3])])).toEqual([1, 2, 3]);
});

// A map by what gives its value as it is, an untagged variant's constructor
// or `String::as_str`, is no map: react.dev's page route gives Next.js its
// paths, `.map(StaticPath::Params)`, and reads `NODE_ENV`.
test("a map by what gives the value itself is none", async () => {
  const dir = fixture("identity-map");
  writeFileSync(join(dir, "lib.rs"), `#[cfg_attr(rust_js, rust_js::untagged)]
pub enum Entry {
    Name(String),
    Pair(Pair),
}
pub struct Pair {
    pub key: String,
}
pub fn entries(keys: Vec<String>) -> Vec<Entry> {
    keys.into_iter().filter(|key| !key.is_empty()).map(|key| Pair { key }).map(Entry::Pair).collect()
}
pub fn production(env: Option<&String>) -> bool {
    env.map(String::as_str) == Some("production")
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).not.toContain("(value) => value");
  expect(js).toContain('return env === "production";');
  const lib = await import(join(dir, "mir.js"));
  expect(lib.entries(["a", "", "b"])).toEqual([{ key: "a" }, { key: "b" }]);
  expect([lib.production("production"), lib.production(undefined), lib.production("dev")]).toEqual([true, false, false]);
});

// An Option compared to a literal is only that comparison, which none fails
// anyway: `segments.at(-1) === "index"`, as react.dev's page route drops a
// path's last `index`.
test("an Option compared to a literal needs no test that it's there", async () => {
  const dir = fixture("option-literal");
  writeFileSync(join(dir, "lib.rs"), `pub fn trimmed(mut segments: Vec<String>) -> Vec<String> {
    if segments.last().is_some_and(|last| last == "index") {
        segments.pop();
    }
    segments
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain('const option = segments.at(-1);\n  if (option === "index") {');
  const lib = await import(join(dir, "mir.js"));
  expect([lib.trimmed(["a", "index"]), lib.trimmed(["a"]), lib.trimmed([])]).toEqual([["a"], ["a"], []]);
});

// An insert whose answer goes unused is JS's own, `set.add(x)`, in a
// thread-local's closure too, as react.dev's home page keeps its liked
// videos and its caches.
test("an insert in a thread-local's closure, its answer unused, is JS's own", async () => {
  const dir = fixture("local-insert");
  writeFileSync(join(dir, "lib.rs"), `use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
thread_local! {
    static LIKED: RefCell<HashSet<&'static str>> = RefCell::new(HashSet::new());
    static CACHE: RefCell<HashMap<String, u32>> = RefCell::new(HashMap::new());
}
pub fn like(id: &'static str) -> bool {
    LIKED.with_borrow_mut(|liked| liked.insert(id));
    LIKED.with_borrow(|liked| liked.contains(id))
}
pub fn cache(key: &str, n: u32) -> Option<u32> {
    CACHE.with_borrow_mut(|cache| cache.insert(key.to_string(), n));
    CACHE.with_borrow(|cache| cache.get(key).copied())
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain("  LIKED.add(id);\n");
  expect(js).toContain("  CACHE.set(key, n);\n");
  expect(js).not.toContain("@rust-js/runtime");
  const lib = await import(join(dir, "mir.js"));
  expect([lib.like("a"), lib.cache("k", 2)]).toEqual([true, 2]);
});

// ADR 0140: a generic iterator collected is `Iterator.from(it).toArray()`,
// never `Array.from(it)`: it may be a binding's with only a `next`, of
// which `Array.from` makes an empty array.
test("a generic iterator collected keeps an iterator with only a next", async () => {
  const dir = fixture("generic-next-only");
  writeFileSync(join(dir, "lib.rs"), `#[cfg_attr(rust_js, rust_js::link_name = "globalThis.counted")] fn counted() -> Box<dyn Iterator<Item = u32>> { unreachable!() }
fn plus_one<I: Iterator<Item = u32>>(items: I) -> Vec<u32> {
    items.map(|x| x + 1).collect()
}
pub fn bumped() -> Vec<u32> {
    plus_one(counted())
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const lib = await import(join(dir, "mir.js"));
  let n = 0;
  (globalThis as any).counted = () => ({ next: () => (n < 2 ? { value: ++n, done: false } : { value: undefined, done: true }) });
  expect(lib.bumped()).toEqual([2, 3]);
  delete (globalThis as any).counted;
});

// ADR 0364: `for` over what `Object.keys` gives, a `Vec`, steps through
// its array: `Object.keys(o)` is no JS iterator.
test("a for over Object.keys steps through its array", async () => {
  const dir = fixture("object-keys-loop");
  writeFileSync(join(dir, "lib.rs"), `pub fn listed(o: &js::Unknown) -> u32 {
    let mut n = 0;
    for key in js::object::keys(o) {
        if !key.is_empty() {
            n += 1;
        }
    }
    n
}
pub fn first(o: &js::Unknown) -> Option<String> {
    let mut keys = js::object::keys(o).into_iter();
    keys.next()
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  const lib = await import(join(dir, "mir.js"));
  expect([lib.listed({ a: 1, b: 2 }), lib.listed({})]).toEqual([2, 0]);
  expect([lib.first({ a: 1, b: 2 }), lib.first({})]).toEqual(["a", undefined]);
});

// ADR 0364: an `on_load!` body from MIR, which ends a loop by its end, is
// its module's top all the same, where JS has no `return`.
test("an on_load body's loop ends at the module's top", async () => {
  const dir = fixture("on-load-loop");
  writeFileSync(join(dir, "lib.rs"), `#[cfg_attr(rust_js, rust_js::link_name = "globalThis.onLoadLooped")] #[allow(unused_variables)] fn seen(n: u32) { unreachable!() }
js::on_load! {
    let mut it = [1u32, 2, 3].iter();
    while let Some(&n) = it.next() {
        if n == 3 {
            break;
        }
        seen(n);
    }
}
pub fn ready() -> u32 {
    1
}
`);
  const seen: number[] = [];
  const globals = globalThis as typeof globalThis & { onLoadLooped?: (n: number) => void };
  globals.onLoadLooped = (n) => seen.push(n);
  try {
    run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
    seen.length = 0;
    expect((await import(join(dir, "mir.js"))).ready()).toBe(1);
    expect(seen).toEqual([1, 2]);
  } finally {
    delete globals.onLoadLooped;
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
  expect([lib.all("xa", "$&$&"), lib.dollar("ba"), lib.plain("aa"), lib.first("export default A; export default B"), lib.firstOf("aa", "$1")])
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
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain("return `<p>\n${code}\n</p>`;");
  expect(js).toContain("return `<p>\\n${code}\\n</p>`;");
  expect(js).toContain('return `<a href="x">\n${code}\n</a>`;');
  // A line a `\` continues isn't one: its `\n` is written so.
  expect(js).toContain("return `<p>\\n${code}</p>`;");
  expect(js).toContain("console.log(`<p>\n${code}\n</p>`);");
  expect(js).toContain("return `<p>\n</p>`;");
  const lib = await import(join(dir, "mir.js"));
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

// An unread getter's read stays: a binding's getter may do something, as
// `el.offsetWidth` lays the page out again, which restarts an animation.
test("an unread getter of a binding is still read", async () => {
  const dir = fixture("unread-getter");
  writeFileSync(join(dir, "lib.rs"), `#[cfg_attr(rust_js, rust_js::link_name = "get offsetWidth")] #[allow(unused_variables)] fn offset_width(this: &js::JsObject) -> f64 { unreachable!() }

pub fn reflow(el: &js::JsObject) {
    let _width = offset_width(el);
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--library", "--manifest", join(dir, "manifest.json"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("el.offsetWidth");
  const lib = await import(join(dir, "lib.js"));
  let laidOut = 0;
  lib.reflow({ get offsetWidth() { laidOut += 1; return 10; } });
  expect(laidOut).toBe(1);
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
  expect([lib.tested(/a/, "a"), lib.tested(undefined, "a"), lib.ran(undefined, "a"), lib.added({ n: 1 }, 2), lib.added(undefined, 2), lib.orNot(undefined, "a")])
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
    pub static Memoized: MemoExoticComponent<LabelProps<'static>> = memo(Label, None);
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

#[cfg_attr(rust_js, rust_js::link_name = "keyed")]
#[allow(unused_variables)]
fn keyed(this: &js::JsObject, options: Options) -> u32 {
    unreachable!()
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

#[cfg_attr(rust_js, rust_js::link_name = "parseInt")]
#[allow(unused_variables)]
fn parse_int(s: &str) -> f64 {
    unreachable!()
}

pub struct CustomProps {
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

#[cfg_attr(rust_js, rust_js::link_name = "parseInt")]
#[allow(unused_variables)]
fn parse_int(s: &str) -> f64 {
    unreachable!()
}

#[cfg_attr(rust_js, rust_js::link_name = "globalThis.record")]
#[allow(unused_variables)]
fn record(n: i32) {
    unreachable!()
}

#[cfg_attr(rust_js, rust_js::link_name = "globalThis.record")]
#[cfg_attr(rust_js, rust_js::returns_undefined)]
#[allow(unused_variables)]
fn noted(n: i32) {
    unreachable!()
}

pub struct CardProps {
    pub on_submit: Box<dyn Fn()>,
    pub on_next: Rc<dyn Fn()>,
    pub on_note: Option<Box<dyn Fn()>>,
    pub on_parse: Box<dyn Fn()>,
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
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain('return { code: null, message: "m" };');
  expect(js).toContain("return { code: code ?? null, message: null };");
  expect(js).toContain("return { code, message };");
  // A conditional of `Some` or `None` is one of the value or `null`.
  expect(js).toContain("return { code: code || null, message: null };");
  expect(js).toContain('return { code: p.code, message: p.title ?? "" };');
  const lib = await import(join(dir, "mir.js"));
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
  expect(js).toContain("  const kept = text;\n");
  expect(js).toContain("  return n + 1;\n");
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
  const { firstId } = await import(join(dir, "lib.js"));
  expect([firstId({ firstElementChild: { id: "a" } }), firstId({ firstElementChild: null })]).toEqual(["a", undefined]);
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

// A property of what may be `None`, `file.and_then(|f| js::get(f, "visible"))`,
// is `file?.visible`, and of a dictionary's, `files["/styles.css"]?.visible`,
// as react.dev's SandpackRoot reads it, and one of a variable's key,
// `files[name]?.active`: a key that's a name. Set where it's none, it's
// `counts[name] ??= ..`, as react.dev's CustomPreset counts lines. Another stays a test, `file != null ? file["a-b"] : undefined`.
test("a property read of what may be None is an optional chain", async () => {
  const withJs = ["--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];
  const dir = fixture("optional-property");
  writeFileSync(join(dir, "lib.rs"), `use js::{Dict, Unknown, dict};
pub struct File {
    pub active: Option<bool>,
}
pub fn active(files: &Dict<File>, name: &str) -> bool {
    dict::get(files, name).and_then(|f| f.active) == Some(true)
}
pub fn counted(counts: &Dict<f64>, name: &str, code: &str) -> f64 {
    if dict::get(counts, name).is_none() {
        dict::set(counts, name, code.split('\\n').count() as f64);
    }
    *dict::get(counts, name).unwrap()
}
pub fn styled(files: &Dict<&Unknown>) -> bool {
    !js::truthy(dict::get(files, "/styles.css").and_then(|f| js::get(*f, "visible")))
}
pub fn visible(file: Option<&Unknown>) -> bool {
    !js::truthy(file.and_then(|f| js::get(f, "visible")))
}
pub fn dashed(file: Option<&Unknown>) -> bool {
    js::truthy(file.and_then(|f| js::get(f, "a-b")))
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withJs]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("return !file?.visible;");
  expect(js).toContain('return !files["/styles.css"]?.visible;');
  expect(js).toContain("return !!files[name]?.active;");
  expect(js).toContain('counts[name] ??= code.split("\\n").length;');
  expect(js).toContain('file != null ? file["a-b"] : undefined');
  const { visible, dashed, styled, active, counted } = await import(join(dir, "lib.js"));
  expect([counted({}, "a", "x\ny"), counted({ a: 7 }, "a", "x"), active({ a: { active: true } }, "a"), active({}, "a"), styled({}), styled({ "/styles.css": { visible: true } }), visible(undefined), visible({}), visible({ visible: 1 }), dashed({ "a-b": 1 }), dashed(undefined)]).toEqual([2, 7, true, false, true, false, true, true, false, true, false]);
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
  expect(js).toContain('export function isCode(value) {\n  return value.mdxName === "inlineCode";\n}');
  expect(js).toContain('export function isText(value) {\n  return value === "inlineCode";\n}');
  expect(js).toContain('export function kindIs(value) {\n  return value === "inlineCode";\n}');
  // An array is never `null`: `Array.isArray` alone says it.
  expect(js).toContain("export function listed(value) {\n  return Array.isArray(value) ? 1 : 0;\n}");
  expect(js).toContain('export function isString(value) {\n  const mdxName = value.mdxName;\n  return typeof mdxName === "string";\n}');
  const lib = await import(join(dir, "lib.js"));
  const values = [{ mdxName: "inlineCode" }, { mdxName: "pre" }, { mdxName: null }, {}, { mdxName: 5 }];
  expect(values.map(lib.isCode)).toEqual([true, false, false, false, false]);
  expect(values.map((v) => lib.isString(v))).toEqual([true, true, false, false, false]);
  expect(["inlineCode", "pre", 5, null, undefined].map((v) => lib.isText(v))).toEqual([true, false, false, false, false]);
  expect(["inlineCode", "pre", 5, {}].map((v) => lib.kindIs(v))).toEqual([true, false, false, false]);
  expect([lib.isThree(3), lib.isThree(4)]).toEqual([true, false]);
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
// back a promise where nothing reads its `()` (ADR 0257). From MIR too
// (ADR 0364).
test("a closure that only spawns an async block is an async arrow", async () => {
  const dir = fixture("spawning-closure");
  writeFileSync(join(dir, "copy.js"), "export const copied = [];\nexport async function copy(text) { copied.push(text); }\n");
  writeFileSync(join(dir, "lib.rs"), `#[cfg_attr(rust_js, rust_js::link_name = "./copy.js#copy")] #[allow(unused_variables)] fn copy(text: &str) -> js::Promise<()> { unreachable!() }

pub fn handler() -> Box<dyn Fn()> {
    Box::new(|| {
        js::spawn(Box::new(async {
            copy("#58C4DC").await;
        }))
    })
}
`);
  const { copied } = await import(join(dir, "copy.js"));
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain('return async () => {\n    await copy("#58C4DC");\n  };');
  const lib = await import(join(dir, "mir.js"));
  copied.length = 0;
  await lib.handler()();
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
  expect(js).toContain("asStr(section) {\n    return section;\n  }");
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

// ADR 0331: a binding's `&dyn Any` is any JS value, given the value, not
// the pair that downcasts it: of each in a `Vec` and a slice too, as
// react.dev's CodeBlock gives CodeMirror's `HighlightStyle.define` its specs.
test("a binding is given each dyn Any's value", async () => {
  const dir = fixture("any-to-js");
  writeFileSync(join(dir, "lib.rs"), `use std::any::Any;

#[cfg_attr(rust_js, rust_js::link_name = "JSON.stringify")]
#[allow(unused_variables)]
fn stringify_all(values: Vec<&dyn Any>) -> String {
    unreachable!()
}

#[cfg_attr(rust_js, rust_js::link_name = "JSON.stringify")]
#[allow(unused_variables)]
fn stringify_slice(values: &[&dyn Any]) -> String {
    unreachable!()
}

struct Spec {
    tag: &'static str,
}

pub fn all() -> String {
    stringify_all(vec![&Spec { tag: "link" }, &3])
}

pub fn slice() -> String {
    let specs: [&dyn Any; 2] = [&Spec { tag: "strong" }, &"bold"];
    stringify_slice(&specs)
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const { all, slice } = await import(join(dir, "lib.js"));
  expect(all()).toBe('[{"tag":"link"},3]');
  expect(slice()).toBe('[{"tag":"strong"},"bold"]');
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
  expect(js).toContain("total = n > 0 ? total + n : (total - n) | 0;");
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
  expect(js).toContain('return kind === "note" ? VARIANTS.pitfall : kind === "pitfall" ? VARIANTS.note : VARIANTS.rsc;');
  expect(js).toContain('kind === "pitfall" ? OTHER.pitfall : VARIANTS.rsc');
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
  expect(js).toContain("const items$1 = items != null && items.length !== 0 ? items : undefined;\n  if (!items$1) {\n    return 0;\n  }\n  return $index(items$1, 0);");
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
  expect([js.includes('if (d === "Up")'), js.includes("d != null"), js.includes('if (s === "Dot")'), js.includes("s != null && s.TAG")]).toEqual([true, true, true, false]);
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
  // Dropped where the assertion fails, then the panic: no flag, as it's
  // known there, and no \`try\` of a \`throw\` (ADR 0372).
  expect(js).toContain('  if (n <= 0) {\n    loudDrop_drop(l);\n    throw new Error("assertion failed: n > 0");\n  }\n  return { l };');
  const lib = await import(join(dir, "lib.js"));
  expect([lib.hold([1]).l[0], lib.checked([2], 1).l[0]]).toEqual([1, 2]);
});

// An f64 known to be a whole number in an integer's range is that integer as
// it is (ADR 0302): where `indexOf` found it, as react.dev's CodeBlock writes
// `startColumn: index` once `index` isn't -1, and an integer's `as f64`. One
// that may be -1, or is set again after its test, is cast as `as` casts.
test("an f64 known to be a whole number in range is cast as it is", async () => {
  const dir = fixture("whole-casts");
  writeFileSync(join(dir, "lib.rs"), `#[cfg_attr(rust_js, rust_js::link_name = "indexOf")] #[rust_js::position] #[allow(unused_variables)] fn index_of(this: &str, search: &str) -> f64 { unreachable!() }
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
  expect([lib.found("abc", "c"), lib.unchecked("abc", "z"), lib.setAgain("abc", "a"), lib.byte(7), lib.shifted("abc", "z"), lib.wide(7)]).toEqual([2, 0, 0, 7, 0, 7n]);
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
  expect(js).toContain('x % 3 === 0 || x % 3 === 1 ? "low" : "high"');
  expect(js).toContain('const label = n === 0 && flag ? "flagged zero" : "other";');
  expect(js).toContain('const label = n.value === 0 ? "zero" : "other";');
  expect(js).toContain('const label = xs.length === 0 ? "empty" : "some";');
  expect(js).toContain('const label = !flag ? "off" : "on";');
  const lib = await import(join(dir, "lib.js"));
  expect([lib.buttonClass(undefined), lib.buttonClass("Secondary"), lib.low(4), lib.low(5)]).toEqual(["bg-link", "text-primary", "low", "high"]);
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
  expect(js).not.toMatch(/as_?deref/i);
  const lib = await import(join(dir, "lib.js"));
  expect([lib.pathIs("/learn", "/learn"), lib.pathIs("/a", "/learn"), lib.pathIs(undefined, "/learn")]).toEqual([true, false, false]);
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
  // From MIR too (ADR 0364).
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  const lib = await import(join(dir, "mir.js"));
  const done = { type: "done", compilatonError: false };
  expect([lib.described({ type: "resize", height: 3 }), lib.described({ type: "start", firstLoad: true }), lib.described(done)])
    .toEqual(["resize 3", "start true", "other"]);
  expect([lib.other(done), lib.other({ type: "resize", height: 1 }), lib.made(done) === done, lib.allMade([done])[0] === done]).toEqual([true, false, true, true]);
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
  expect([lib.namedAlike("Ann", "ANN", 1), lib.namedAlike("Ann", "ANN", 2), lib.namedAlike("Ann", "Bob", 1)]).toEqual([true, false, false]);
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
  expect([lib.cleared(failure), lib.cleared({ message: "kept" }), lib.readAfter(failure), lib.taken(failure), lib.looped(failure), lib.captured(failure), lib.bumped(1), lib.bumpedElse(1), lib.lent({ hits: 0 }, { hits: 0 })])
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

// ADR 0576: what a body's integers can be, walked as it runs, shows an index
// in bounds and an operator that can't wrap: React's scheduler heap reads
// `heap[leftIndex]`, `(index + 1) * 2 - 1` and a swap of two items, as
// upstream's does. What could leave its range, or an index the walk can't
// see stays below its slice's length, keeps its wrap or its check.
test("a body's integer ranges show indexes in bounds and operators that can't wrap", async () => {
  const dir = fixture("integer-ranges");
  writeFileSync(join(dir, "lib.rs"), `pub fn sift_down(heap: &mut Vec<u32>, i: usize) {
    let mut index = i;
    let length = heap.len();
    let half_length = length >> 1;
    while index < half_length {
        let left_index = (index + 1) * 2 - 1;
        let right_index = left_index + 1;
        if heap[left_index] < heap[index] {
            if right_index < length && heap[right_index] < heap[left_index] {
                heap.swap(index, right_index);
                index = right_index;
            } else {
                heap.swap(index, left_index);
                index = left_index;
            }
        } else if right_index < length && heap[right_index] < heap[index] {
            heap.swap(index, right_index);
            index = right_index;
        } else {
            return;
        }
    }
}

pub fn pop(heap: &mut Vec<u32>) -> Option<u32> {
    if heap.is_empty() {
        return None;
    }
    let first = heap.swap_remove(0);
    if !heap.is_empty() {
        sift_down(heap, 0);
    }
    Some(first)
}

pub fn last(items: &[u32]) -> u32 {
    if items.is_empty() { 0 } else { items[items.len() - 1] }
}

pub fn head(items: &[u32]) -> u32 {
    if !items.is_empty() { items[0] } else { 0 }
}

pub fn wrapped(n: u32, k: i32, m: u8) -> (u32, i32, u8) {
    (n + 1, k * 2, m + 1)
}

pub fn small(n: u32, k: i32, m: u8) -> (u32, i32, u8) {
    let a = if n < 10 { n * 3 + 1 } else { 0 };
    let b = if k > 0 { 0 - k } else { 1 };
    let c = if m < 255 { m + 1 } else { 0 };
    (a, b, c)
}

pub fn counted(n: u32) -> u32 {
    let mut i = 0;
    while i < n {
        i += 1;
    }
    i
}

pub fn stepped(n: u32) -> u32 {
    let mut steps = 0;
    loop {
        steps += 1;
        if steps == n {
            return steps;
        }
    }
}

pub fn skipped(n: u32) -> u32 {
    let mut i = 0;
    let mut k = 0;
    while i < 10 {
        i += 1;
        if i == 5 {
            k = n;
            continue;
        }
        k = 0;
    }
    k + 1
}

pub fn at_end(items: &[u32], i: usize) -> u32 {
    if i <= items.len() { items[i] } else { 0 }
}

pub fn popped(items: &mut Vec<u32>) -> u32 {
    let mut sum = 0;
    for i in 0..items.len() {
        sum ^= items[i];
        items.pop();
    }
    sum
}

pub fn swapped(items: &mut [u32], i: usize) {
    items.swap(0, i);
}

pub fn later(items: &mut Vec<u32>) -> u32 {
    if items.is_empty() {
        return 0;
    }
    items.clear();
    items[0]
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("    const leftIndex = (index + 1) * 2 - 1;\n    const rightIndex = leftIndex + 1;\n    if (heap[leftIndex] < heap[index]) {\n      if (rightIndex < length && heap[rightIndex] < heap[leftIndex]) {\n        [heap[index], heap[rightIndex]] = [heap[rightIndex], heap[index]];\n");
  expect(js).toContain("  const halfLength = length >>> 1;\n");
  expect(js).toContain("  [heap[0], heap[heap.length - 1]] = [heap[heap.length - 1], heap[0]];\n  const first = heap.pop();\n");
  expect(js).toContain("items[items.length - 1]");
  expect(js).toContain("items.length !== 0 ? items[0] : 0");
  // What could leave its range wraps; what can't is JS's own.
  expect(js).toContain("[(n + 1) >>> 0, Math.imul(k, 2), (m + 1) & 255]");
  expect(js).toContain("n < 10 ? n * 3 + 1 : 0");
  expect(js).toContain("k > 0 ? 0 - k : 1");
  expect(js).toContain("m < 255 ? m + 1 : 0");
  expect(js).toContain("  while (i < n) {\n    i += 1;\n  }\n");
  expect(js).toContain("steps = (steps + 1) >>> 0;");
  expect(js).toContain("return (k + 1) >>> 0;");
  expect(js).toContain("i <= items.length ? $index(items, i) : 0");
  // A slice changed in a loop, an index nothing bounds, a length changed.
  expect(js).toContain("$index(items, i)");
  expect(js).toContain("$swap(items, 0, i);");
  expect(js).toContain("$index(items, 0)");
  const lib = await import(join(dir, "lib.js"));
  const heap = [1, 3, 2, 7, 4, 6, 5];
  const out = [];
  for (let next = lib.pop(heap); next !== undefined; next = lib.pop(heap)) out.push(next);
  expect(out).toEqual([1, 2, 3, 4, 5, 6, 7]);
  expect([lib.last([]), lib.last([1, 7])]).toEqual([0, 7]);
  expect(lib.wrapped(4294967295, 1 << 30, 255)).toEqual([0, -2147483648, 0]);
  expect([lib.small(9, 5, 254), lib.counted(3), lib.stepped(3), lib.popped([5])]).toEqual([[28, -5, 255], 3, 3, 5]);
  expect(() => lib.popped([1, 2, 4])).toThrow("index out of bounds");
  expect(() => lib.swapped([1], 3)).toThrow("index out of bounds");
  expect(() => lib.atEnd([1], 1)).toThrow("index out of bounds");
  expect(lib.skipped(4294967295)).toBe(1);
  expect(() => lib.later([1])).toThrow("index out of bounds");
});

// ADR 0577: a function no JS caller sees, called and never named as a value,
// is given what its calls give it: React's heap's `siftUp` reads
// `heap[index]` of the index `push` gave it, the heap's length before its
// `push`. A `push` makes a length one more, a `pop` one fewer.
test("a function only calls reach is what its calls give it", async () => {
  const dir = fixture("only-called");
  writeFileSync(join(dir, "lib.rs"), `pub fn push(heap: &mut Vec<u32>, n: u32) {
    let index = heap.len();
    heap.push(n);
    sift_up(heap, index);
}

fn sift_up(heap: &mut [u32], i: usize) {
    let mut index = i;
    while index > 0 {
        let parent = (index - 1) >> 1;
        if heap[parent] > heap[index] {
            heap.swap(parent, index);
            index = parent;
        } else {
            return;
        }
    }
}

pub fn guarded(items: &[u32], i: usize) -> u32 {
    if i < items.len() { item(items, i) } else { 0 }
}

fn item(items: &[u32], i: usize) -> u32 {
    items[i]
}

pub fn mixed(items: &[u32], i: usize) -> u32 {
    let first = if i < items.len() { other(items, i) } else { 0 };
    first ^ other(items, i)
}

fn other(items: &[u32], i: usize) -> u32 {
    items[i]
}

pub fn doubled(items: &[u32]) -> (u32, Vec<u32>) {
    (double(3), items.iter().copied().map(double).collect())
}

fn double(n: u32) -> u32 {
    n * 2
}

pub fn half(n: u32) -> u32 {
    n - 1
}

pub fn halved() -> u32 {
    half(5)
}

pub fn counted() -> u32 {
    count(10, 0)
}

fn count(n: u32, total: u32) -> u32 {
    if n == 0 { total } else { count(n - 1, total + 1) }
}

pub fn pushed(v: &mut Vec<u32>) -> u32 {
    let n = v.len();
    v.push(7);
    v[n]
}

pub fn popped(v: &mut Vec<u32>) -> u32 {
    let n = v.len();
    if n == 0 {
        return 0;
    }
    v.pop();
    v[n - 1]
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("    const parent = (index - 1) >>> 1;\n    if (heap[parent] > heap[index]) {\n      [heap[parent], heap[index]] = [heap[index], heap[parent]];\n");
  expect(js).toContain("function item(items, i) {\n  return items[i];\n}");
  // Called where nothing bounds it too, named as a value, or a JS caller's.
  expect(js).toContain("function other(items, i) {\n  return $index(items, i);\n}");
  expect(js).toContain("function double(n) {\n  return Math.imul(n, 2) >>> 0;\n}");
  expect(js).toContain("export function half(n) {\n  return (n - 1) >>> 0;\n}");
  // Its own calls' too, until what it's given holds.
  expect(js).toContain("count(n - 1, (total + 1) >>> 0)");
  expect(js).toContain("  v.push(7);\n  return v[n];\n");
  expect(js).toContain("return $index(v, n - 1);");
  const lib = await import(join(dir, "lib.js"));
  const heap: number[] = [];
  for (const n of [5, 3, 8, 1]) lib.push(heap, n);
  expect(heap[0]).toBe(1);
  expect([lib.guarded([4, 5], 1), lib.guarded([4], 3), lib.counted(), lib.halved(), lib.pushed([1, 2])]).toEqual([5, 0, 10, 4, 7]);
  expect(lib.doubled([1, 2])).toEqual([6, [2, 4]]);
  expect(lib.half(0)).toBe(4294967295);
  expect(() => lib.mixed([1], 4)).toThrow("index out of bounds");
  expect(() => lib.popped([1])).toThrow("index out of bounds");
});

// ADR 0578: a `#[rust_js::nullable]` function's result is `null` where it's
// written, so each `return` gives it as written, `return last;`, not a
// variable set in each branch and `return result ?? null;`. And an item's
// place through `index_mut` the body's ranges show in bounds is unchecked:
// React's heap's `pop`, `heap[0] = last`.
test("a nullable result is written where it's made, and an index_mut known in bounds is unchecked", async () => {
  const dir = fixture("nullable-written");
  writeFileSync(join(dir, "lib.rs"), `#[rust_js::nullable]
pub fn pop(heap: &mut Vec<u32>) -> Option<u32> {
    let last = heap.pop()?;
    if heap.is_empty() {
        return Some(last);
    }
    let first = std::mem::replace(&mut heap[0], last);
    Some(first)
}

#[rust_js::nullable]
pub fn first(items: &[u32]) -> Option<&u32> {
    items.first()
}

#[rust_js::nullable]
pub fn found(items: &[u32], flag: bool) -> Option<&u32> {
    let first = items.first();
    if flag && first.is_some() {
        return first;
    }
    items.last()
}

pub fn replaced(items: &mut Vec<u32>, i: usize, x: u32) -> u32 {
    if i < items.len() { std::mem::replace(&mut items[i], x) } else { x }
}

pub fn replaced_any(items: &mut Vec<u32>, i: usize, x: u32) -> u32 {
    std::mem::replace(&mut items[i], x)
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("export function pop(heap) {\n  const last = heap.pop();\n  if (last == null) {\n    return null;\n  }\n  if (heap.length === 0) {\n    return last;\n  }\n  const first = heap[0];\n  heap[0] = last;\n  return first;\n}");
  expect(js).toContain("  return items[0] ?? null;\n");
  expect(js).toContain("    return first ?? null;\n");
  expect(js).toContain("export function replaced(items, i, x) {\n  if (i < items.length) {\n    const old = items[i];\n    items[i] = x;\n");
  expect(js).toContain("export function replacedAny(items, i, x) {\n  $index(items, i);\n");
  const lib = await import(join(dir, "lib.js"));
  const heap = [1, 2, 3];
  expect([lib.pop(heap), heap, lib.pop([4]), lib.pop([]), lib.first([]), lib.first([5])]).toEqual([1, [3, 2], 4, null, null, 5]);
  const items = [1, 2];
  expect([lib.replaced(items, 1, 7), lib.replaced(items, 5, 8), items]).toEqual([2, 8, [1, 7]]);
  expect(() => lib.replacedAny([1], 2, 0)).toThrow("index out of bounds");
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

pub fn rebound(error: &Failure, other: &Failure) -> Option<&'static str> {
    let mut error = error;
    let Failure { title, .. } = error;
    error = other;
    error.title.set(Some("other"));
    title.get()
}

pub fn rebound_set(error: &Failure, other: &Failure) -> Option<&'static str> {
    let mut error = error;
    let Failure { title, .. } = error;
    error = other;
    title.set(Some("first"));
    error.title.get()
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("const { message, title } = error;");
  expect(js.match(/const \{ title \} = /g)).toBeNull();
  const lib = await import(join(dir, "lib.js"));
  expect([lib.shown({ title: undefined, message: "m" }), lib.shown({ title: "T", message: "m" }), lib.retitled({ title: "old", message: "" }), lib.twice({ title: "ab", message: "" }), lib.looped({ title: "ab", message: "" }), lib.aliased({ title: "T", message: "" }), lib.dropped({ title: "T", message: "" })])
    .toEqual(["Error: m", "T: m", "new", 2, 2, ["T", "set"], "dropped"]);
  const [first, other] = [{ title: "a", message: "" }, { title: "b", message: "" }];
  expect([lib.rebound({ title: "a", message: "" }, { title: "b", message: "" }), lib.reboundSet(first, other), first.title]).toEqual(["a", "b", "first"]);
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

// A closure copies a variable set again only where it may be set after the
// closure is made: one set before it is read as it is, as react.dev's
// LoadingOverlay reads `fadeTimeout` in its cleanup. One set after, or in
// a loop around it, is copied where it's made, and so is one the closure changes.
test("a closure copies only what may be set after it's made", async () => {
  const dir = fixture("captured-after");
  writeFileSync(join(dir, "lib.rs"), `pub fn before(start: u32) -> impl Fn() -> u32 {
    let mut n = start;
    if n > 3 {
        n = 3;
    }
    move || n + 1
}
pub fn after(start: u32) -> u32 {
    let mut n = start;
    let f = move || n + 1;
    n = 10;
    f() + n
}
pub fn looped(xs: &[u32]) -> u32 {
    let mut n = 0;
    let mut fs = Vec::new();
    for x in xs {
        n = *x;
        let f = move || n;
        fs.push(f);
    }
    fs.iter().map(|f| f()).sum()
}
pub fn counted(start: u32) -> u32 {
    let mut n = start;
    let mut bump = move || {
        n += 1;
        n
    };
    bump();
    bump();
    n
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  const body = (name: string) => js.slice(js.indexOf(`function ${name}(`), js.indexOf("\n}\n", js.indexOf(`function ${name}(`)));
  expect(body("before")).not.toContain("n$1");
  expect(body("after")).toContain("const captured = n;");
  expect(body("looped")).toContain("const captured = n;");
  const lib = await import(join(dir, "lib.js"));
  expect([lib.before(7)(), lib.after(1), lib.looped([4, 5]), lib.counted(5)]).toEqual([4, 12, 9, 5]);
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
  for (const written of ["return !!error;", "return !error;", "return error ? 1 : 0;", "if (error && error.message === \"x\") {",
    "return error ? $byteLen(error.message) : undefined;", "return n != null;", "return s == null;", "return f ? 1 : 0;", "return !!f;"]) {
    expect(js).toContain(written);
  }
  const lib = await import(join(dir, "lib.js"));
  const failure = { message: "x" };
  expect([lib.shown(failure), lib.shown(undefined), lib.hidden(undefined), lib.tested(failure), lib.chained(failure), lib.chained(undefined),
    lib.mapped(failure), lib.number(0), lib.text(""), lib.first(true), lib.first(undefined), lib.firstValue(false)])
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

// An `Option` compared with a `Some(..)`, of what's never `undefined` or
// `null`, is `===`, as react.dev's getRouteMeta writes `currentRoute.path
// === searchPath`: either answers the same (ADR 0030). Two `Option`s keep
// `==`, as `null` and `undefined` are both `None`.
test("an Option compared with a Some is ===", async () => {
  const dir = fixture("option-strict-eq");
  writeFileSync(join(dir, "lib.rs"), `pub fn same(p: &Option<String>, s: &str) -> bool {
    p.as_deref() == Some(s)
}

pub fn other(p: Option<u32>, n: u32) -> bool {
    p != Some(n)
}

pub fn both(p: Option<u32>, q: Option<u32>) -> bool {
    p == q
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("  return p === s;\n");
  expect(js).toContain("  return p !== n;\n");
  expect(js).toContain("  return p == q;\n");
  const lib = await import(join(dir, "lib.js"));
  expect([lib.same("a", "a"), lib.same(null, "a"), lib.other(undefined, 1), lib.other(1, 1), lib.both(null, undefined)]).toEqual([true, false, true, false, true]);
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

pub fn effect(on: bool) -> impl Fn() -> Option<u32> {
    move || {
        if on {
            println!("on");
            return Some(1);
        }
        None
    }
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).not.toContain("return undefined;");
  // One at a function's end says nothing: it falls off its end there, a
  // closure's too, as a person ends one.
  expect(js.match(/return;/g)?.length).toBe(1);
  expect(js).toContain("  if (on) {\n    return noop;\n  }\n}\n");
  expect(js).toContain("      return 1;\n    }\n  };\n}\n");
  const lib = await import(join(dir, "lib.js"));
  expect([lib.halved(3), lib.halved(4), lib.cleanup(false), lib.effect(false)(), lib.effect(true)()]).toEqual([undefined, 2, undefined, undefined, 1]);
});

// A struct with a flattened field, made outside JSX, is an object of its
// fields and the flattened one's: a struct made there inlined, another
// spread, `{ ...item, severity }`, as react.dev's runESLint writes
// `{...item, severity: severity[item.severity]}` (ADR 0204). A flattened
// field may be a reference to a struct.
test("a struct with a flattened field made outside JSX spreads it", async () => {
  const dir = fixture("flattened-made");
  writeFileSync(join(dir, "lib.rs"), `pub struct Message {
    pub line: f64,
    pub severity: f64,
}
pub struct Reported<'a> {
    #[rust_js::flatten]
    pub item: &'a Message,
    pub severity: &'static str,
}
pub fn reported(item: &Message) -> Reported<'_> {
    Reported {
        item,
        severity: if item.severity > 1.0 { "error" } else { "warning" },
    }
}
pub fn line(report: &Reported) -> f64 {
    report.item.line
}
pub struct Named {
    pub label: &'static str,
    #[rust_js::flatten]
    pub message: Message,
}
pub fn named() -> Named {
    Named {
        label: "x",
        message: Message { line: 1.0, severity: 2.0 },
    }
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain('return { ...item, severity: item.severity > 1 ? "error" : "warning" };');
  expect(js).toContain("return report.line;");
  expect(js).toContain('return { label: "x", line: 1, severity: 2 };');
  const lib = await import(join(dir, "lib.js"));
  const made = lib.reported({ line: 3, severity: 2 });
  expect([made, lib.line(made), lib.named()]).toEqual([{ line: 3, severity: "error" }, 3, { label: "x", line: 1, severity: 2 }]);
});

// A callback that tests its argument's truth is `Boolean`, as react.dev
// writes `.filter(Boolean)`: what `filter_map` keeps of a value never falsy
// (ADR 0298), and `!s.is_empty()` of a string. A number's keeps `!= null`.
test("a truth test as a callback is Boolean", async () => {
  const dir = fixture("filter-boolean");
  writeFileSync(join(dir, "lib.rs"), `pub struct Item {
    pub n: u32,
}
pub fn items(xs: &[u32]) -> Vec<Item> {
    xs.iter().filter_map(|x| (*x > 1).then(|| Item { n: *x })).collect()
}
pub fn words(text: &str) -> Vec<String> {
    text.split(',').map(|g| g.trim().to_string()).filter(|g| !g.is_empty()).collect()
}
pub fn less(xs: &[u32]) -> Vec<u32> {
    xs.iter().filter_map(|x| x.checked_sub(1)).collect()
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("return xs.map((x) => (x > 1 ? { n: x } : undefined)).filter(Boolean);");
  expect(js).not.toContain("(g) => !!g");
  expect(js).toContain(".filter(Boolean);");
  expect(js).toContain(".filter((item) => item != null)");
  const lib = await import(join(dir, "lib.js"));
  expect([lib.items([1, 2]), lib.words("a, ,b"), lib.less([0, 2])]).toEqual([[{ n: 2 }], ["a", "b"], [1]]);
});

// `?` of an Option of a value never falsy tests it by its truth, as
// react.dev's runESLint writes `if (!error) return undefined;` (ADR 0298).
// A number's tests it against `null`.
test("? of an Option never falsy tests its truth", async () => {
  const dir = fixture("question-truth");
  writeFileSync(join(dir, "lib.rs"), `pub struct Item {
    pub n: u32,
}
pub fn first(item: Option<&Item>) -> Option<u32> {
    let item = item?;
    Some(item.n + 1)
}
pub fn counted(n: Option<u32>) -> Option<u32> {
    let n = n?;
    Some(n + 1)
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("if (!item) {\n    return;\n  }");
  expect(js).toContain("if (n == null) {\n    return;\n  }");
  const lib = await import(join(dir, "lib.js"));
  expect([lib.first({ n: 1 }), lib.first(undefined), lib.counted(0), lib.counted(undefined)]).toEqual([2, undefined, 1, undefined]);
});

// A nullable field given another's value, or a copy of it, is given it as
// it is: it's `null` already where it's `None`, as react.dev's runESLint
// gives `ruleId: error.ruleId` (ADR 0275).
test("a nullable field given a nullable field's copy takes it as it is", async () => {
  const dir = fixture("nullable-copied");
  writeFileSync(join(dir, "lib.rs"), `pub struct Message {
    #[rust_js::nullable]
    pub id: Option<String>,
}
pub struct Diagnostic {
    #[rust_js::nullable]
    pub id: Option<String>,
}
pub fn copied(message: &Message) -> Diagnostic {
    Diagnostic { id: message.id.clone() }
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("return { id: message.id };");
  const lib = await import(join(dir, "lib.js"));
  expect([lib.copied({ id: null }), lib.copied({ id: "x" })]).toEqual([{ id: null }, { id: "x" }]);
});

// A thread-local's `with` of a closure of one statement is the statement,
// the thread-local in its parameter's place, as react.dev's runESLint
// writes `linter.defineRules({..})`: no closure called in place.
test("with of a one-statement closure is its statement", async () => {
  const dir = fixture("with-statement");
  writeFileSync(join(dir, "lib.rs"), `#[cfg_attr(rust_js, rust_js::link_name = "globalThis.seen.push")] #[allow(unused_variables)] fn seen(s: &str) { unreachable!() }
pub struct Logger {
    pub prefix: &'static str,
}
thread_local! {
    static LOGGER: Logger = Logger { prefix: "a" };
}
pub fn logged() {
    LOGGER.with(|it| {
        seen(it.prefix);
    });
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("export function logged() {\n  globalThis.seen.push(LOGGER.prefix);\n}");
  (globalThis as { seen?: string[] }).seen = [];
  const lib = await import(join(dir, "lib.js"));
  lib.logged();
  expect((globalThis as { seen?: string[] }).seen).toEqual(["a"]);
});

// ADR 0308: a function written in a function's body is that body's, where
// it's written, as react.dev's useSandpackLint has `isReactRuleError` in
// its callback: a function declaration, which its block may call before
// it, as Rust's may. One a static's initializer calls stays the module's,
// where that initializer is.
test("a function written in a body is a local function there", async () => {
  const dir = fixture("local-functions");
  writeFileSync(join(dir, "lib.rs"), `pub fn kept(errors: Vec<Option<u32>>) -> (Vec<Option<u32>>, u32) {
    let early = twice(1);
    fn twice(n: u32) -> u32 {
        n * 2
    }
    let kept = errors
        .into_iter()
        .filter(|e| {
            fn present(e: &Option<u32>) -> bool {
                e.is_some()
            }
            present(e)
        })
        .collect();
    (kept, early)
}

pub fn started() -> u32 {
    fn start() -> u32 {
        5
    }
    thread_local! {
        static START: u32 = start();
    }
    START.with(|s| *s) + start()
}

mod util {
    pub fn three() -> u32 {
        3
    }
}

struct Noisy;

impl Drop for Noisy {
    fn drop(&mut self) {}
}

pub fn nested() -> u32 {
    fn outer() -> u32 {
        fn inner() -> u32 {
            4
        }
        inner()
    }
    outer()
}

pub fn listed(n: u32) -> Vec<u32> {
    fn list_of(n: u32) -> Vec<u32> {
        let mut list = Vec::new();
        list.push(n);
        list
    }
    list_of(n)
}

pub fn guarded() -> u32 {
    let first = later();
    let _noisy = Noisy;
    fn later() -> u32 {
        util::three() + 4
    }
    first + later()
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("export function kept(errors) {\n  const early = twice(1);\n  function twice(n) {");
  expect(js).toContain("(e) => {\n    function present(e) {");
  expect(js).toContain("\nfunction start() {");
  const lib = await import(join(dir, "lib.js"));
  // Made readable as a module's is: a `let mut` nothing sets again, a `const`.
  expect(js).toContain("  function listOf(n) {\n    const list = [];\n    list.push(n);");
  // Called before a destructor's `try`, it's written before it.
  expect(js).toContain("  function later() {\n    return (three() + 4) >>> 0;\n  }\n  try {\n    result = (first + later()) >>> 0;\n  } finally {\n    noisyDrop_drop({ value: _noisy });");
  // One in another's body is a declaration there too.
  expect(js).toContain("  function outer() {\n    function inner() {");
  expect([lib.kept([1, undefined, 3]), lib.started(), lib.guarded(), lib.nested()]).toEqual([[[1, 3], 2], 10, 14, 4]);
});

// ADR 0309: a match that only gives a value is one expression, as a person
// writes it, as the playground's app has `current`, `text` and `file`: an
// option's value or a default, `e ?? d`; a part of it never null or
// undefined, `e?.[1] ?? d`; otherwise `m ? f(m) : d`, of a `const m`. And
// `map_or` is the same expression.
test("a match that only gives a value is one expression", async () => {
  const dir = fixture("value-matches");
  writeFileSync(join(dir, "lib.rs"), `pub struct State {
    pub n: u32,
}

fn find(states: &[State], n: u32) -> Option<&State> {
    states.iter().find(|s| s.n == n)
}

pub fn picked<'a>(states: &'a [State], blank: &'a State) -> u32 {
    let current = match find(states, 1) {
        Some(state) => state,
        None => blank,
    };
    current.n + current.n
}

pub fn shown_text(files: &[(String, String)], shown: &str) -> String {
    let text = match files.iter().find(|(path, _)| path == shown) {
        Some((_, text)) => text.clone(),
        None => String::new(),
    };
    format!("{text}{text}")
}

pub fn after(path: &str) -> u32 {
    let n = match path.find('x') {
        Some(i) => i as u32 + 1,
        None => 0,
    };
    n * n
}

pub fn mapped(path: &str) -> &str {
    path.rsplit_once('/').map_or(path, |(_, file)| file)
}

pub fn field_of(state: Option<&State>) -> u32 {
    let n = match state {
        Some(s) => s.n,
        None => 0,
    };
    n * n
}

fn pair_of(n: u32) -> Option<(u32, Option<u32>)> {
    match n {
        0 => None,
        1 => Some((1, None)),
        _ => Some((n, Some(n))),
    }
}

pub fn inner(n: u32) -> u32 {
    let inner = match pair_of(n) {
        Some((_, inner)) => inner,
        None => Some(7),
    };
    inner.unwrap_or(0) + inner.unwrap_or(0)
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("  const current = state || blank;\n");
  expect(js).toContain('  const text = find ? find[1] : "";\n');
  expect(js).toMatch(/  const i = \$find\(path, "x"\);\n  const n = i != null \? \(i \+ 1\) >>> 0 : 0;\n/);
  expect(js).toContain('  return $rsplitOnce(path, "/")?.[1] ?? path;\n');
  // Of a variable, as `map_or` of it.
  expect(js).toContain("  const n = state ? state.n : 0;\n");
  const lib = await import(join(dir, "lib.js"));
  const states = [{ n: 1 }, { n: 2 }];
  expect([lib.picked(states, { n: 9 }), lib.picked([], { n: 9 })]).toEqual([2, 18]);
  expect([lib.shownText([["a", "A"], ["b", "B"]], "b"), lib.shownText([], "b")]).toEqual(["BB", ""]);
  expect([lib.after("abxd"), lib.after("ab"), lib.mapped("a/b/c"), lib.mapped("c")]).toEqual([9, 0, "c", "c"]);
  // A part that may be `None` itself isn't `?? d`'s: `Some((1, None))` gives `None`.
  expect([lib.inner(0), lib.inner(1), lib.inner(3)]).toEqual([14, 0, 6]);
  expect([lib.fieldOf({ n: 3 }), lib.fieldOf(undefined)]).toEqual([9, 0]);
});

// ADR 0311: `.flatten()` of a read of items that may be `None` is the JS read
// itself, as react.dev's Console reads `args.shift()` and `data[0]` of any
// values: JS's `undefined` is both no item and a `None` one, as `flatten`
// makes them one.
test("flatten of a read of items that may be None is the read", async () => {
  const dir = fixture("flattened-reads");
  writeFileSync(join(dir, "lib.rs"), `use std::collections::VecDeque;

pub fn shifted(items: Vec<Option<u32>>) -> (Option<u32>, Option<u32>, usize) {
    let mut args: VecDeque<Option<u32>> = items.into_iter().collect();
    let first = args.pop_front().flatten();
    let last = args.pop_back().flatten();
    (first, last, args.len())
}

pub fn read(items: &[Option<u32>], i: usize) -> (Option<u32>, Option<u32>, Option<u32>) {
    (items.first().copied().flatten(), items.get(i).copied().flatten(), items.last().copied().flatten())
}

pub fn popped(mut items: Vec<Option<u32>>) -> Option<u32> {
    items.pop().flatten()
}
`);
  // From MIR too (ADR 0364).
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain("const first = args.shift();");
  expect(js).toContain("const last = args.pop();");
  expect(js).toContain("return [items[0], items[i], items.at(-1)];");
  expect(js).toContain("return items.pop();");
  expect(js).not.toContain("$someValue");
  const lib = await import(join(dir, "mir.js"));
  expect([lib.shifted([1, undefined, 3]), lib.shifted([undefined]), lib.shifted([])]).toEqual([[1, 3, 1], [undefined, undefined, 0], [undefined, undefined, 0]]);
  expect([lib.read([undefined, 2], 1), lib.read([], 0)]).toEqual([[undefined, 2, 2], [undefined, undefined, undefined]]);
  expect([lib.popped([1, undefined]), lib.popped([]), lib.popped([4])]).toEqual([undefined, undefined, 4]);
});

// ADR 0313: what react.dev's Console writes, as a person writes it: `!x` of
// text that's `None` or empty; `i < args.length` of a range of a length
// nothing in the loop changes, and a deque's `args[i]` in it; `[...a, ...b]`
// of two chained; and no trailing `undefined` argument.
test("Console's loops, spreads and calls read as written", async () => {
  const dir = fixture("console-forms");
  writeFileSync(join(dir, "lib.rs"), `use std::collections::VecDeque;

pub fn unescaped(escaped: Option<&str>) -> bool {
    escaped.is_none_or(str::is_empty)
}

pub fn joined(args: VecDeque<u32>) -> String {
    let mut formatted = String::new();
    for i in 0..args.len() {
        formatted += &format!(" {}", args[i]);
    }
    formatted
}

pub fn shifted(mut args: VecDeque<u32>) -> String {
    let mut take = || args.pop_front();
    take();
    let mut formatted = String::new();
    for i in 0..args.len() {
        formatted += &format!(" {}", args[i]);
    }
    formatted
}

pub fn drained(mut args: Vec<u32>) -> u32 {
    let mut total = 0;
    for _ in 0..args.len() {
        total += args.pop().unwrap_or(0);
    }
    total
}

pub fn both(prev: &VecDeque<u32>, next: Vec<u32>) -> VecDeque<u32> {
    prev.iter().copied().chain(next).collect()
}

fn greet(name: &str, title: Option<&str>) -> String {
    format!("{}{name}", title.unwrap_or(""))
}

pub fn hi() -> String {
    greet("Ada", None)
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("return !escaped;");
  expect(js).toContain("for (let i = 0; i < args.length; i++) {\n    formatted += ` ${args[i]}`;");
  // Changed before the loop only, by a closure even, as Rust's borrows
  // allow nothing else while the loop reads it.
  expect(js).toMatch(/export function shifted\(args\) \{[^]*for \(let i = 0; i < args\.length; i\+\+\) \{\n    formatted \+= ` \$\{args\[i\]\}`;/);
  // One whose body changes the length reads its end once, as Rust does.
  expect(js).toContain("const end = args.length;");
  expect(js).toContain("return [...prev, ...next];");
  expect(js).toContain('return greet("Ada");');
  const lib = await import(join(dir, "lib.js"));
  expect([lib.unescaped(undefined), lib.unescaped(""), lib.unescaped("%")]).toEqual([true, true, false]);
  expect([lib.joined([1, 2]), lib.shifted([1, 2, 3]), lib.drained([1, 2, 3]), lib.both([1], [2, 3]), lib.hi()]).toEqual([" 1 2", " 2 3", 6, [1, 2, 3], "Ada"]);
});

// `unwrap_or_default()` of a call reads the call once, `get(n) ?? ""`, as
// `unwrap_or` does, where it was kept in a `const option` first. A boxed
// `Some`, which `??` can't read, is still kept first.
test("unwrap_or_default of a call is its value or the default", async () => {
  const dir = fixture("unwrap-or-default");
  writeFileSync(join(dir, "lib.rs"), `pub fn get(n: u32) -> Option<String> {
    if n > 0 { Some(n.to_string()) } else { None }
}

pub fn shown(n: u32) -> String {
    get(n).unwrap_or_default()
}

pub fn nested(n: u32) -> Option<u32> {
    let inner = if n > 1 { Some(Some(n)) } else if n > 0 { Some(None) } else { None };
    inner.map(|x| x).unwrap_or_default()
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain('export function shown(n) {\n  return get(n) ?? "";\n}');
  const lib = await import(join(dir, "lib.js"));
  expect([lib.shown(3), lib.shown(0), lib.nested(2), lib.nested(1), lib.nested(0)]).toEqual(["3", "", 2, undefined, undefined]);
});

// `unwrap_or_else(f)` of a call reads it once too, and calls `f` only where
// it's `None`, as react.dev's calculateNestedToc writes
// `currentAncestors.get(item.depth - 1) || root`; a closure of statements
// is its arrow, made first.
test("unwrap_or_else of a call is its value or the closure's", async () => {
  const dir = fixture("unwrap-or-else");
  writeFileSync(join(dir, "lib.rs"), `use std::collections::HashMap;

pub fn parent(ancestors: &HashMap<u32, String>, depth: u32, root: &str) -> String {
    ancestors.get(&depth).cloned().unwrap_or_else(|| root.to_string())
}

pub fn counted(ancestors: &HashMap<u32, String>, depth: u32, log: &mut Vec<u32>) -> String {
    ancestors.get(&depth).cloned().unwrap_or_else(|| {
        log.push(depth);
        String::from("none")
    })
}

pub fn run() -> (String, String, String, String, Vec<u32>) {
    let mut ancestors = HashMap::new();
    ancestors.insert(1, String::from("one"));
    let mut log = Vec::new();
    let found = counted(&ancestors, 1, &mut log);
    let missing = counted(&ancestors, 2, &mut log);
    (parent(&ancestors, 1, "root"), parent(&ancestors, 2, "root"), found, missing, log)
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("return ancestors.get(depth) ?? root;");
  expect(js).toContain("return ancestors.get(depth) ?? fallback();");
  const lib = await import(join(dir, "lib.js"));
  expect(lib.run()).toEqual(["one", "root", "one", "none", [2]]);
});

// A flattened field is a struct's (ADR 0204): an enum variant's was nested,
// `{ type: "article", base: { .. } }`, where JS holds it flat. It's refused,
// at the field.
test("a flattened field of an enum's variant is refused", () => {
  const dir = fixture("flattened-variant");
  writeFileSync(join(dir, "lib.rs"), `#[derive(Default)]
pub struct Base<'a> {
    pub title: Option<&'a str>,
}

#[cfg_attr(rust_js, rust_js::tag = "type")]
pub enum OpenGraph<'a> {
    #[cfg_attr(rust_js, rust_js::name = "article")]
    Article {
        #[cfg_attr(rust_js, rust_js::flatten)]
        base: Base<'a>,
    },
}

pub fn og() -> OpenGraph<'static> {
    OpenGraph::Article { base: Base { title: Some("Hi") } }
}
`);
  const failed = Bun.spawnSync([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")], { cwd: root, stderr: "pipe" });
  expect([failed.exitCode === 0, failed.stderr.toString()]).toEqual([false, expect.stringContaining("rust-js does not support flattening an enum variant's field yet")]);
});

// A parameter of a \`&mut\` to a \`&mut [i32]\`, whose place JS can't share,
// is refused: given a copy, \`*s = other\` would leave the caller's \`rest\`
// as it was.
test("a parameter of a `&mut` to a slice reference is refused from MIR", () => {
  const dir = fixture("mut-slice-ref-param");
  writeFileSync(join(dir, "lib.rs"), `pub fn reset<'a>(s: &mut &'a mut [i32], other: &'a mut [i32]) {
    *s = other;
}
`);
  const failed = Bun.spawnSync([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")], {
    cwd: root,
    stderr: "pipe",
  });
  expect([failed.exitCode === 0, failed.stderr.toString()]).toEqual([false, expect.stringContaining("rust-js does not support values of type `&mut &mut [i32]` yet")]);
});

// A variable of a type JS can't hold, a struct of a \`&mut\` to a \`&mut [i32]\`,
// is refused: \`*h.s = &mut ys\` would change a copy, leaving \`rest\` on
// \`xs\`.
test("a variable of a type JS can't hold is refused from MIR", () => {
  const dir = fixture("mut-slice-ref-held");
  writeFileSync(join(dir, "lib.rs"), `struct Holder<'a, 'b> {
    s: &'a mut &'b mut [i32],
}
pub fn run() -> Vec<i32> {
    let mut xs = vec![1, 2];
    let mut ys = vec![9];
    let mut rest: &mut [i32] = &mut xs;
    let h = Holder { s: &mut rest };
    *h.s = &mut ys;
    rest[0] = 5;
    xs
}
`);
  const failed = Bun.spawnSync([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")], {
    cwd: root,
    stderr: "pipe",
  });
  expect([failed.exitCode === 0, failed.stderr.toString()]).toEqual([false, expect.stringContaining("rust-js does not support values of type `&mut &mut [i32]` yet")]);
  // And one assigned again, which MIR declares first.
  writeFileSync(join(dir, "lib.rs"), `struct Holder<'a, 'b> {
    s: &'a mut &'b mut [i32],
}
pub fn run(again: bool) -> Vec<i32> {
    let mut xs = vec![1, 2];
    let mut ys = vec![9];
    let mut rest: &mut [i32] = &mut xs;
    let mut h = Holder { s: &mut rest };
    if again {
        h = Holder { s: h.s };
    }
    *h.s = &mut ys;
    rest[0] = 5;
    xs
}
`);
  const again = Bun.spawnSync([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")], {
    cwd: root,
    stderr: "pipe",
  });
  expect([again.exitCode === 0, again.stderr.toString()]).toEqual([false, expect.stringContaining("rust-js does not support values of type `&mut &mut [i32]` yet")]);
});

// An iterator adapter over items with a destructor, \`v.into_iter().map(f)\`
// dropped before it's stepped, is refused: a JS iterator has no fields to
// drop them through, and its items would be left undropped.
test("an adapter of items with a destructor dropped unstepped is refused", () => {
  const dir = fixture("adapter-drops");
  writeFileSync(join(dir, "lib.rs"), `pub struct Noisy(pub &'static str);
impl Drop for Noisy {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}
pub fn run() {
    let v = vec![Noisy("a"), Noisy("b")];
    drop(v.into_iter().scan(0, |_, n| Some(n)));
}
`);
  const failed = Bun.spawnSync([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")], {
    cwd: root,
    stderr: "pipe",
  });
  expect([failed.exitCode === 0, failed.stderr.toString()]).toEqual([false, expect.stringContaining("with a destructor")]);
});

// What JS can't do soundly is refused: each compiled to output that ran
// unlike Rust, found running them both.
for (const [name, source, message] of [
  ["a map's key looked up through the crate's Borrow", "use std::borrow::Borrow;\n#[derive(PartialEq, Eq, Hash)]\npub struct Name(pub String);\nimpl Borrow<str> for Name { fn borrow(&self) -> &str { &self.0 } }\npub fn f(m: &std::collections::HashMap<Name, i32>) -> Option<&i32> { m.get(\"a\") }", "as what its own `Borrow` gives"],
  ["a collection of the crate's extending a Vec", "struct Bag { items: Vec<i32> }\nimpl IntoIterator for Bag { type Item = i32; type IntoIter = std::vec::IntoIter<i32>; fn into_iter(self) -> Self::IntoIter { self.items.into_iter() } }\npub fn f(v: &mut Vec<i32>, b: Bag) { v.extend(b) }", "whose `IntoIterator` is the crate's"],
  ["an untagged enum's variants of one kind, matched", "#[rust_js::untagged] pub enum E { A(&'static str), B(char) }\npub fn f(e: E) -> u32 { match e { E::A(_) => 1, E::B(_) => 2 } }", "which JS can't tell apart"],
  ["a Box<dyn Debug> of a value with a destructor", "pub struct D;\nimpl Drop for D { fn drop(&mut self) {} }\nimpl std::fmt::Debug for D { fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result { f.write_str(\"D\") } }\npub fn f() { let _: Box<dyn std::fmt::Debug> = Box::new(D); }", "a `dyn` of a value with a destructor"],
  ["a new closure assigned through a &mut to one", "pub fn replace<F: FnMut()>(f: &mut F, g: F) { *f = g; }", "assigning a whole value through a `&mut`"],
  ["a Fn closure holding a destructor given away", "pub struct R; impl Drop for R { fn drop(&mut self) {} } fn g<F: FnOnce()>(f: F) { f() } pub fn f() { let r = R; let c = move || { let _r = &r; }; c(); g(c); }", "giving `g`, which calls it once, a function called by `&`"],
  ["{:#?} of a kept &dyn Debug", "pub fn f(x: &[u8]) -> String { let d: &dyn std::fmt::Debug = &x; format!(\"{:#?}\", d) }", "`{:#?}` of a `&dyn Debug` made elsewhere"],
] as [string, string, string][]) {
  test(`MIR refuses ${name}`, () => {
    const dir = fixture("mir-refused");
    writeFileSync(join(dir, "lib.rs"), source);
    const failed = Bun.spawnSync([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")], {
      cwd: root,
      stderr: "pipe",
    });
    expect([failed.exitCode === 0, failed.stderr.toString()]).toEqual([false, expect.stringContaining(message)]);
  });
}

// MIR's cell only its function uses is a plain variable (tidy's
// \`plain_cells\`), but not where a callback's parameter of its name reads
// a \`value\` of its own, \`{:?}\`'s of a \`Vec<&mut str>\`.
test("a cell a callback's parameter shadows stays a cell", async () => {
  const dir = fixture("cell-shadowed");
  writeFileSync(join(dir, "lib.rs"), `pub fn show() -> String {
    let item = Box::leak(Box::new(5));
    *item += 1;
    let v: Vec<&'static mut str> = vec![String::from("a").leak()];
    format!("{item} {v:?}")
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const lib = await import(join(dir, "mir.js"));
  expect(lib.show()).toBe('6 ["a"]');
});

// \`v.iter().map(f).next_back()\` maps the last item alone, as Rust's
// \`Map::next_back\` does, from MIR: \`f\` runs once, and the map nothing
// steps is no statement.
test("MIR's next_back of a map runs its closure once", async () => {
  const dir = fixture("map-next-back");
  writeFileSync(join(dir, "lib.rs"), `pub fn last(v: Vec<i32>) -> (Option<i32>, u32) {
    let mut calls = 0;
    let last = v.iter().map(|x| {
        calls += 1;
        x * 10
    }).next_back();
    (last, calls)
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")], 600_000);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).not.toContain(".map(");
  const lib = await import(join(dir, "mir.js"));
  expect([lib.last([1, 2, 3]), lib.last([])]).toEqual([[30, 1], [undefined, 0]]);
});

// `s[a..b].make_ascii_uppercase()` gives `s` its text with that part
// changed (ADR 0334), not a handle on a part.
test("a string's part changed in place gives the string its text", async () => {
  const dir = fixture("str-part-case");
  writeFileSync(join(dir, "lib.rs"), `pub fn shout(mut name: String) -> String {
    name[2..4].make_ascii_uppercase();
    name[..].make_ascii_lowercase();
    name[2..].make_ascii_uppercase();
    name
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain("name = $asciiCase(name, true, 2, 4);");
  expect(js).toContain("name = $asciiCase(name, false);");
  expect(js).toContain("name = $asciiCase(name, true, 2);");
  const lib = await import(join(dir, "mir.js"));
  expect(lib.shout("élan Vital")).toBe("éLAN VITAL");
});

// A loop over an iterator of the crate's own that holds a value with a
// destructor drops, from MIR, as Rust does: each item, then the iterator
// where the loop ends or breaks.
test("MIR's loop over an iterator holding a value with a destructor drops as Rust does", async () => {
  const dir = fixture("iterator-holding-drop");
  writeFileSync(join(dir, "lib.rs"), `use std::cell::RefCell;

thread_local! {
    static LOG: RefCell<Vec<String>> = RefCell::new(Vec::new());
}

fn note(line: String) {
    LOG.with_borrow_mut(|log| log.push(line));
}

struct R(u32);

impl Drop for R {
    fn drop(&mut self) {
        note(format!("drop {}", self.0));
    }
}

struct It(R, u8);

impl Iterator for It {
    type Item = R;
    fn next(&mut self) -> Option<R> {
        if self.1 == 0 {
            return None;
        }
        self.1 -= 1;
        Some(R(10 + self.1 as u32))
    }
}

pub fn run() -> Vec<String> {
    for r in It(R(1), 2) {
        note(format!("got {}", r.0));
    }
    for r in It(R(2), 3) {
        if r.0 == 11 {
            break;
        }
        note(format!("got {}", r.0));
    }
    LOG.with_borrow_mut(std::mem::take)
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")], 600_000);
  const lib = await import(join(dir, "mir.js"));
  expect(lib.run()).toEqual(["got 11", "drop 11", "got 10", "drop 10", "drop 1", "got 12", "drop 12", "drop 11", "drop 2"]);
});

// `fold` given a function of the crate's is `reduce` given it as it is,
// which takes no more than it's given (ADR 0279).
test("a fold given a function of the crate's gives it as it is", async () => {
  const dir = fixture("fold-crate-fn");
  writeFileSync(join(dir, "lib.rs"), `fn add(total: i32, x: &i32) -> i32 {
    total + x
}

pub fn sum(v: Vec<i32>) -> i32 {
    v.iter().fold(0, add)
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  expect(readFileSync(join(dir, "mir.js"), "utf8")).toContain("return v.reduce(add, 0);");
  const lib = await import(join(dir, "mir.js"));
  expect([lib.sum([1, 2, 3]), lib.sum([])]).toEqual([6, 0]);
});

// A function's `if` of an `||` as its value: one conditional, the `||` its
// one test; an `&&` written as the value, one value returned.
test("an if of a logical is one conditional", async () => {
  const dir = fixture("if-logical-returns");
  writeFileSync(join(dir, "lib.rs"), `pub fn prec(op: char) -> u8 {
    if op == '+' || op == '-' { 1 } else { 2 }
}

pub fn both(a: u8, b: u8) -> bool {
    a > 1 && b > 1
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain('  return op === "+" || op === "-" ? 1 : 2;\n');
  expect(js).toContain("return a > 1 && b > 1;");
  const lib = await import(join(dir, "mir.js"));
  expect([lib.prec("+"), lib.prec("*"), lib.both(2, 2), lib.both(2, 0)]).toEqual([1, 2, true, false]);
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

// ADR 0054, from MIR (ADR 0364): a writer returns the string it writes,
// `return s` of each way through that writes once, `let f = s` of a first
// write; a derived impl's parameters named as the program's are.
test("a writer from its MIR returns the string it writes", async () => {
  const dir = fixture("mir-writers");
  writeFileSync(join(dir, "lib.rs"), `use std::fmt;

#[derive(Debug)]
pub struct Point {
    pub x: i32,
    pub y: i32,
}

pub enum Shape {
    Dot,
    Line(u32),
}

impl fmt::Display for Shape {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Shape::Dot => write!(f, "a dot"),
            Shape::Line(n) => write!(f, "a line of {n}"),
        }
    }
}

pub struct List(pub Vec<u32>);

impl fmt::Display for List {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "[")?;
        for (i, x) in self.0.iter().enumerate() {
            if i > 0 {
                write!(f, ", ")?;
            }
            write!(f, "{x}")?;
        }
        write!(f, "]")
    }
}

pub fn shown(p: &Point, s: &Shape, l: &List) -> String {
    format!("{p:?} {s} {l}")
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("function pointDebug_fmt(point) {\n  return `Point { x: ${point.x}, y: ${point.y} }`;\n}");
  expect(js).toContain('function shapeDisplay_fmt(shape) {\n  if (shape === "Dot") {\n    return "a dot";\n  }\n  return `a line of ${shape._0}`;\n}');
  expect(js).toContain('function listDisplay_fmt(list) {\n  let f = "[";');
  const lib = await import(join(dir, "lib.js"));
  expect([lib.shown({ x: 1, y: 2 }, { TAG: "Line", _0: 3 }, [[1, 2]]), lib.shown({ x: 1, y: 2 }, "Dot", [[]])]).toEqual(["Point { x: 1, y: 2 } a line of 3 [1, 2]", "Point { x: 1, y: 2 } a dot []"]);
});

// ADRs 0058, 0137, from MIR: a writer takes its `Formatter`'s options, and a
// derived `Debug` gives them to each field.
test("a writer from its MIR takes its Formatter's options", async () => {
  const dir = fixture("mir-writer-options");
  writeFileSync(join(dir, "lib.rs"), `use std::fmt;

#[derive(Debug)]
pub struct Point {
    pub x: i32,
    pub y: i32,
}

pub struct Name(pub String);

impl fmt::Display for Name {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.pad(&self.0)
    }
}

pub fn shown(p: &Point, n: &Name) -> String {
    format!("{p:#?}|{n:>6}|{n:<4}|")
}

`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("function pointDebug_fmt(point, options) {\n  const shown = $formatted(String(point.x), options, true);");
  expect(js).toContain("function nameDisplay_fmt(name, options) {\n  return $formatted(name[0], options);\n}");
  const lib = await import(join(dir, "lib.js"));
  expect(lib.shown({ x: 1, y: 2 }, ["ab"])).toBe("Point {\n    x: 1,\n    y: 2,\n}|    ab|ab  |");
});

// ADRs 0137, 0143, from MIR: `f.alternate()` and `f.width()` are what the
// writer's `Formatter` was given.
test("a writer from its MIR answers what its Formatter is asked", async () => {
  const dir = fixture("mir-writer-questions");
  writeFileSync(join(dir, "lib.rs"), `use std::fmt;

pub struct Size(pub u32);

impl fmt::Display for Size {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if f.alternate() {
            write!(f, "{} bytes", self.0)
        } else {
            match f.width() {
                Some(w) => write!(f, "{}B/{w}", self.0),
                None => write!(f, "{}B", self.0),
            }
        }
    }
}

pub fn shown(s: &Size) -> String {
    format!("{s} {s:#} {s:7}")
}

`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("  if (options?.alternate === true) {\n    return `${size[0]} bytes`;\n  }\n  const w = options?.width;");
  const lib = await import(join(dir, "lib.js"));
  expect(lib.shown([5])).toBe("5B 5 bytes 5B/7");
});

// ADR 0136, from MIR: a `Debug` builder's chain is the string it writes, its
// `entries(v.iter())` all of `v`; a writer writing once each way returns it.
test("a writer's Debug builder from its MIR is the string its chain writes", async () => {
  const dir = fixture("mir-writer-builders");
  writeFileSync(join(dir, "lib.rs"), `use std::fmt;

pub struct P {
    pub x: i32,
}

impl fmt::Debug for P {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("P").field("x", &self.x).finish()
    }
}

pub struct Bag(pub Vec<u32>);

impl fmt::Debug for Bag {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_list().entry(&0).entries(self.0.iter()).finish()
    }
}

pub struct Mark(pub bool);

impl fmt::Display for Mark {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let shown = if self.0 { "yes" } else { "no" };
        if self.0 {
            f.write_str(shown)
        } else {
            f.write_str("-")
        }
    }
}

pub struct Flag(pub bool);

impl fmt::Display for Flag {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.0 {
            f.write_str("y")?;
        } else {
            f.write_str("n")?;
        }
        Ok(())
    }
}

pub fn shown(p: &P, b: &Bag, m: &Mark) -> String {
    format!("{p:?} {b:?} {m} {} {}", Flag(true), Flag(false))
}

`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("function pDebug_fmt(p) {\n  return `P { x: ${p.x} }`;\n}");
  expect(js).toContain('return `[${["0"].concat(Array.from(bag[0], (entry) => String(entry))).join(", ")}]`;');
  expect(js).toContain('  if (mark[0]) {\n    return shown;\n  }\n  return "-";\n}');
  const lib = await import(join(dir, "lib.js"));
  expect(js).toContain('function flagDisplay_fmt(flag) {\n  if (flag[0]) {\n    return "y";\n  }\n  return "n";\n}');
  expect(lib.shown({ x: 1 }, [[2, 3]], [true])).toBe("P { x: 1 } [0, 2, 3] yes y n");
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

// A `Copy` value a function gives, a variable of its own: handed over, as
// nothing reads it after, uncopied; a copy where what it was read from goes on.
test("a variable a function gives is handed over uncopied", async () => {
  const dir = fixture("returned-uncopied");
  writeFileSync(join(dir, "lib.rs"), `#[derive(Clone, Copy)]
pub struct P { pub x: i32 }

pub fn moved(x: i32, dx: i32) -> P {
    let mut p = P { x };
    p.x += dx;
    p
}

pub fn twice(p: &mut P) -> P {
    let q = *p;
    p.x += 1;
    q
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("  p.x = (p.x + dx) | 0;\n  return p;\n}");
  expect(js).toContain("  const q = { ...p };\n  p.x = (p.x + 1) | 0;\n  return q;");
  const lib = await import(join(dir, "lib.js"));
  const d = { x: 10 };
  const e = lib.twice(d);
  d.x += 100;
  expect([lib.moved(1, 2), d.x, e.x]).toEqual([{ x: 3 }, 111, 10]);
});

// A function's tail `if` of two small values is one conditional, as a
// variable's is, React's scheduler's `return diff !== 0 ? diff : a.id - b.id`
// (ADR 0575); it was each branch returned, rust-js's form of react.dev's.
test("a tail if of two small values is one conditional", async () => {
  const dir = fixture("tail-if-returns");
  writeFileSync(join(dir, "lib.rs"), `pub struct Email(pub String);

pub fn email(s: String) -> Result<Email, String> {
    if s.contains('@') { Ok(Email(s)) } else { Err(format!("{s} is not an email")) }
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  expect(readFileSync(join(dir, "mir.js"), "utf8")).toContain('  return s.includes("@") ? { TAG: "Ok", _0: [s] } : { TAG: "Err", _0: `${s} is not an email` };');
  const lib = await import(join(dir, "mir.js"));
  expect([lib.email("a@b"), lib.email("x")]).toEqual([{ TAG: "Ok", _0: ["a@b"] }, { TAG: "Err", _0: "x is not an email" }]);
});

// Each branch adding to a variable, `out += a`: the branches, as written,
// not `out = c ? out + a : out + b`.
test("branches that each add to a variable stay branches", async () => {
  const dir = fixture("branch-updates");
  writeFileSync(join(dir, "lib.rs"), `pub fn report(results: Vec<Result<u32, String>>) -> String {
    let mut out = String::new();
    for r in results {
        match r {
            Ok(n) => out += &format!("ok {n}\\n"),
            Err(e) => out += &format!("err {e}\\n"),
        }
    }
    out
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain("out += `ok ${");
  expect(js).toContain("out += `err ${");
  const lib = await import(join(dir, "mir.js"));
  expect(lib.report([{ TAG: "Ok", _0: 1 }, { TAG: "Err", _0: "x" }])).toBe("ok 1\nerr x\n");
});

// A `?` and a `return` in a loop, and a `while` in it: each `return` where
// it's written, and the `while` tested at its head, not MIR's one exit, a
// labeled block, nor a `while (true)`.
test("returns in a loop are where they're written", async () => {
  const dir = fixture("loop-returns");
  writeFileSync(join(dir, "lib.rs"), `pub fn numbers(src: &str) -> Result<Vec<f64>, String> {
    let chars: Vec<char> = src.chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c == ' ' {
            i += 1;
            continue;
        }
        if c.is_ascii_digit() {
            let start = i;
            while i < chars.len() && chars[i].is_ascii_digit() {
                i += 1;
            }
            let text: String = chars[start..i].iter().collect();
            out.push(text.parse::<f64>().map_err(|_| format!("bad {text}"))?);
            continue;
        }
        return Err(format!("unexpected {c}"));
    }
    Ok(out)
}

pub fn tokens(src: &str) -> Result<Vec<u32>, String> {
    let chars: Vec<char> = src.chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c.is_ascii_digit() || c == '_' {
            let start = i;
            while i < chars.len() && chars[i].is_ascii_digit() {
                i += 1;
            }
            let text: String = chars[start..i].iter().collect();
            out.push(text.parse::<u32>().map_err(|_| format!("bad {text}"))?);
            continue;
        }
        match c {
            '+' => out.push(0),
            _ => return Err(format!("unexpected {c}")),
        }
        i += 1;
    }
    Ok(out)
}

pub fn scan(v: &[u32]) -> u32 {
    let mut i = 0;
    let mut again = true;
    for &x in v {
        'inner: loop {
            if i < x {
                i += 1;
                continue;
            }
            for _ in 0..2 {
                if again {
                    again = false;
                    i = 0;
                    continue 'inner;
                }
            }
            return i * 10 + x;
        }
    }
    0
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  const numbers = js.slice(0, js.indexOf("function tokens"));
  // What the program wrote first, first.
  const tokens = js.slice(js.indexOf("function tokens"), js.indexOf("function scan"));
  expect(tokens).toContain('if (/^[0-9]$/.test(c) || c === "_") {\n      const start = i;');
  expect(numbers).not.toMatch(/\bbb\d+\b/);
  expect(numbers).not.toContain("while (true)");
  // In the program's order, its `?` a guard.
  expect(numbers).toContain('if (/^[0-9]$/.test(c)) {\n      const start = i;');
  expect(numbers).toContain('      if (result$1.TAG === "Err") {\n        return result$1;\n      }\n      out.push(result$1._0);\n');
  expect(js).toContain("while (i < chars.length && /^[0-9]$/.test(chars[i])) {");
  const lib = await import(join(dir, "mir.js"));
  expect([lib.numbers("12 3"), lib.numbers("1x")]).toEqual([{ TAG: "Ok", _0: [12, 3] }, { TAG: "Err", _0: "unexpected x" }]);
  expect([lib.tokens("12+3"), lib.tokens("1-")]).toEqual([{ TAG: "Ok", _0: [12, 0, 3] }, { TAG: "Err", _0: "unexpected -" }]);
  // A loop whose rest jumps to it, from a loop of its own, stays one, by
  // the label the program gave it.
  expect(js).toContain("inner: while (true) {");
  expect([lib.scan([3]), lib.scan([])]).toEqual([33, 0]);
});

// `into()` an untagged enum, whose `from` is the variant of its argument,
// is the value (ADR 0214).
test("into an untagged enum is the value", async () => {
  const dir = fixture("into-untagged");
  writeFileSync(join(dir, "lib.rs"), `#[rust_js::untagged]
pub enum Src<'a> {
    Text(&'a str),
    Number(f64),
}

impl<'a> From<&'a str> for Src<'a> {
    fn from(text: &'a str) -> Self {
        Src::Text(text)
    }
}

pub fn text(s: &str) -> Src<'_> {
    s.into()
}

pub fn from(s: &str) -> Src<'_> {
    Src::from(s)
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain("export function text(s) {\n  return s;\n}");
  expect(js).toContain("export function from(s) {\n  return s;\n}");
});

// A range's count the body doesn't read, `for _ in 0..n`: `i`.
test("an unread count of a range is i", async () => {
  const dir = fixture("unread-count");
  writeFileSync(join(dir, "lib.rs"), `pub fn repeat(s: &str, times: u32) -> String {
    let mut out = String::new();
    for _ in 0..times {
        out.push_str(s);
    }
    out
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  expect(readFileSync(join(dir, "mir.js"), "utf8")).toContain("for (let i = 0; i < times; i++) {");
  const lib = await import(join(dir, "mir.js"));
  expect([lib.repeat("ab", 3), lib.repeat("x", 0)]).toEqual(["ababab", ""]);
});

// A shared cell, `Rc<Cell<bool>>`, cloned in a closure that sets it: its
// function's `let`, which the closures share (ADR 0287).
test("a cell cloned in a closure is its function's let", async () => {
  const dir = fixture("cell-cloned-in-closure");
  writeFileSync(join(dir, "lib.rs"), `use std::cell::Cell;
use std::rc::Rc;

pub fn toggles() -> (bool, bool) {
    let running = Rc::new(Cell::new(false));
    let start = {
        let running = running.clone();
        move || {
            if running.get() {
                return false;
            }
            running.set(true);
            let running = running.clone();
            let stop = move || running.set(false);
            stop();
            true
        }
    };
    (start(), running.get())
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain("let running = false;");
  expect(js).not.toContain(".value");
  const lib = await import(join(dir, "mir.js"));
  expect(lib.toggles()).toEqual([true, false]);
});

// A function's tail `loop` that `break a` gives its value: `return a` where
// it breaks.
test("a tail loop's break with a value returns it", async () => {
  const dir = fixture("loop-break-value");
  writeFileSync(join(dir, "lib.rs"), `pub fn fib_loop(n: u32) -> u32 {
    let mut a = 0;
    let mut b = 1;
    let mut i = 0;
    loop {
        if i == n {
            break a;
        }
        let next = a + b;
        a = b;
        b = next;
        i += 1;
    }
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  expect(readFileSync(join(dir, "mir.js"), "utf8")).toContain("    if (i === n) {\n      return a;\n    }");
  const lib = await import(join(dir, "mir.js"));
  expect([lib.fibLoop(0), lib.fibLoop(10)]).toEqual([0, 55]);
});

// A division by a named constant that isn't zero, nor a signed -1, can't
// panic: JS's `/`.
test("a division by a named constant is JS's", async () => {
  const dir = fixture("named-divisor");
  writeFileSync(join(dir, "lib.rs"), `const HALVES: u32 = 2;
const NEGATIVE: i32 = -1;

pub fn half(x: u32) -> u32 {
    x / HALVES
}

pub fn negated(x: i32) -> i32 {
    x / NEGATIVE
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain("return (x / HALVES) >>> 0;");
  expect(js).toContain("$div(x, NEGATIVE, -2147483648)");
  const lib = await import(join(dir, "mir.js"));
  expect([lib.half(7), lib.negated(6)]).toEqual([3, -6]);
  expect(() => lib.negated(-2147483648)).toThrow("attempt to divide with overflow");
});

// A generic iterator is stepped as JS's, `Iterator.from(items)`, as an
// array may be given (ADR 0061); the crate's own given from behind an
// `impl Iterator` is a JS iterator of it.
test("a generic iterator takes the crate's own from behind impl Iterator", async () => {
  const dir = fixture("generic-iterator-opaque");
  writeFileSync(join(dir, "lib.rs"), `pub struct Countdown {
    n: u32,
}

impl Iterator for Countdown {
    type Item = u32;
    fn next(&mut self) -> Option<u32> {
        if self.n == 0 {
            return None;
        }
        self.n -= 1;
        Some(self.n)
    }
}

pub fn countdown(n: u32) -> impl Iterator<Item = u32> {
    Countdown { n }
}

pub fn middle(items: impl Iterator<Item = u32>, k: usize) -> Vec<u32> {
    items.skip(1).take(k).collect()
}

pub fn of_countdown(n: u32) -> Vec<u32> {
    middle(countdown(n), 2)
}

pub fn mapped(n: u32) -> u32 {
    countdown(n).map(|x| x + 1).sum()
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  expect(readFileSync(join(dir, "mir.js"), "utf8")).toContain("return Iterator.from(items).drop(1).take(k).toArray();");
  const lib = await import(join(dir, "mir.js"));
  expect([lib.ofCountdown(5), lib.middle([7, 8, 9, 10], 2), lib.mapped(3)]).toEqual([[3, 2], [8, 9], 6]);
});

// `rev()` of an array's items skipped and taken from, which no closure
// runs over: the array sliced, reversed.
test("rev of skipped and taken items is the slice reversed", async () => {
  const dir = fixture("rev-counted");
  writeFileSync(join(dir, "lib.rs"), `pub fn middle(v: &[i32]) -> Vec<i32> {
    v.iter().skip(1).take(2).rev().copied().collect()
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  expect(readFileSync(join(dir, "mir.js"), "utf8")).toContain("return v.slice(1).slice(0, 2).toReversed();");
  const lib = await import(join(dir, "mir.js"));
  expect([lib.middle([1, 2, 3, 4]), lib.middle([1, 2]), lib.middle([])]).toEqual([[3, 2], [2], []]);
});

// A long function, each call a block of its MIR: lowered block by block,
// as deep as it's long, on a stack grown as rustc grows its own.
test("a long function is lowered block by block", async () => {
  const dir = fixture("long-function");
  const pushes = Array.from({ length: 3000 }, (_, i) => `    v.push(n + ${i});`).join("\n");
  writeFileSync(join(dir, "lib.rs"), `pub fn long(n: u32) -> Vec<u32> {\n    let mut v = Vec::new();\n${pushes}\n    v\n}\n`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const lib = await import(join(dir, "mir.js"));
  const items = lib.long(1);
  expect([items.length, items[0], items[2999]]).toEqual([3000, 1, 3000]);
});

// `VecDeque::remove` is `$removeOpt`; of items that may look like `None`,
// refused, as its `Some` isn't boxed.
test("a deque's remove gives what was there", async () => {
  const dir = fixture("deque-remove");
  writeFileSync(join(dir, "lib.rs"), `use std::collections::VecDeque;

pub fn removed(items: Vec<u32>, at: usize) -> (Option<u32>, Vec<u32>) {
    let mut d: VecDeque<u32> = items.into_iter().collect();
    let removed = d.remove(at);
    (removed, d.into_iter().collect())
}
`);
  writeFileSync(join(dir, "options.rs"), `use std::collections::VecDeque;

pub fn removed(mut d: VecDeque<Option<u32>>) -> Option<Option<u32>> {
    d.remove(0)
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const lib = await import(join(dir, "mir.js"));
  expect([lib.removed([1, 2, 3], 1), lib.removed([1], 5)]).toEqual([[2, [1, 3]], [undefined, [1]]]);
  const refused = Bun.spawnSync([compiler, join(dir, "options.rs"), "-o", join(dir, "options.js")], { stderr: "pipe" });
  expect(refused.exitCode).not.toBe(0);
});

// MIR's `rest.split_off_mut(..n)` gives `rest` what's left; through `r`,
// which holds a `&mut` to `rest`, JS would give `r` it, so it's refused.
test("MIR's split_off through a held `&mut` is refused", () => {
  const dir = fixture("mir-split-off-held");
  writeFileSync(join(dir, "lib.rs"), `pub fn run() -> Vec<i32> {
    let mut w = vec![1, 2, 3, 4, 5];
    let mut rest: &mut [i32] = &mut w;
    let r = &mut rest;
    if let Some(h) = r.split_off_mut(..2) {
        h[0] = 9;
    }
    rest[0] = -7;
    w
}
`);
  const failed = Bun.spawnSync([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")], {
    cwd: root,
    stderr: "pipe",
  });
  expect([failed.exitCode === 0, failed.stderr.toString()]).toEqual([false, expect.stringContaining("rust-js does not support splitting this slice yet")]);
});

// `&v[..]`, all of a slice, is the slice itself, as `&s[..]` of a string
// is and `&mut v[..]` is (ADR 0335): nothing changes it while it's
// borrowed, so the copy a part is made as isn't needed.
test("a shared slice of all of a slice is the slice", async () => {
  const dir = fixture("whole-slice");
  writeFileSync(join(dir, "lib.rs"), `pub fn take(v: &[&str]) -> usize {
    v.len()
}

pub fn whole(v: &Vec<u32>) -> &[u32] {
    &v[..]
}

pub fn literal() -> usize {
    take(&["a", "b"][..])
}

pub fn part(v: &Vec<u32>) -> Vec<u32> {
    v[1..].to_vec()
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("export function whole(v) {\n  return v;\n}");
  expect(js).toContain('return take(["a", "b"]);');
  const lib = await import(join(dir, "lib.js"));
  expect([lib.whole([1, 2]), lib.literal(), lib.part([1, 2, 3])]).toEqual([[1, 2], 2, [2, 3]]);
});

// `<[T; N]>::try_from(&v[..])` is an array of its own, as Rust's is, which
// a write to `v` after doesn't change; `<&[T; N]>`'s is `v`'s, a borrow.
test("an array made of a whole slice is a copy", async () => {
  const dir = fixture("array-of-slice");
  writeFileSync(join(dir, "lib.rs"), `pub fn owned() -> (u8, u8) {
    let mut bytes = vec![1u8, 2];
    let arr: [u8; 2] = bytes[..].try_into().unwrap();
    bytes[0] = 9;
    (arr[0], bytes[0])
}

pub fn part() -> (u8, u8) {
    let mut bytes = vec![1u8, 2, 3];
    let arr = <[u8; 2]>::try_from(&bytes[1..]).unwrap();
    bytes[1] = 9;
    (arr[0], bytes[1])
}

pub fn borrowed() -> u8 {
    let bytes = vec![1u8, 2];
    let arr = <&[u8; 2]>::try_from(&bytes[..]).unwrap();
    arr[1]
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const lib = await import(join(dir, "lib.js"));
  expect([lib.owned(), lib.part(), lib.borrowed()]).toEqual([[1, 9], [2, 9], 2]);
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

// An untagged enum's variant of a fieldless enum is a string, its variant's
// name (ADR 0013), as next/font's `weight: "400" | Array<"400">` has it:
// made, and told from an array where it's matched.
test("an untagged enum's variant may hold a fieldless enum, a string", async () => {
  const dir = fixture("untagged-fieldless");
  writeFileSync(join(dir, "lib.rs"), `pub enum Weight {
    #[cfg_attr(rust_js, rust_js::name = "400")]
    Normal,
    #[cfg_attr(rust_js, rust_js::name = "700")]
    Bold,
}

#[cfg_attr(rust_js, rust_js::untagged)]
pub enum Weights<'a> {
    One(Weight),
    Many(&'a [Weight]),
}

pub fn one() -> Weights<'static> {
    Weights::One(Weight::Bold)
}

pub fn many() -> Weights<'static> {
    Weights::Many(&[Weight::Normal, Weight::Bold])
}

pub fn count(weights: Weights) -> usize {
    match weights {
        Weights::One(_) => 1,
        Weights::Many(all) => all.len(),
    }
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const lib = await import(join(dir, "lib.js"));
  expect([lib.one(), lib.many(), lib.count("700"), lib.count(["400", "700"])]).toEqual(["700", ["400", "700"], 1, 2]);
});

// An untagged enum's variant without fields is its name, a string literal,
// as TypeScript's `boolean | "blocking"` is: made, matched, compared and
// shown as one (ADR 0214). Beside a variant of any string, which JS can't
// tell it from, it's refused where it's matched.
test("an untagged enum's variant without fields is its name's string", async () => {
  const dir = fixture("untagged-literal");
  writeFileSync(join(dir, "lib.rs"), `#[cfg_attr(rust_js, rust_js::untagged)]
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Fallback {
    Bool(bool),
    #[cfg_attr(rust_js, rust_js::name = "blocking")]
    Blocking,
}

pub fn fallback(b: bool) -> Fallback {
    if b { Fallback::Bool(true) } else { Fallback::Blocking }
}

pub fn describe(f: Fallback) -> String {
    match f {
        Fallback::Bool(b) => format!("bool {b}"),
        Fallback::Blocking => "blocking".to_string(),
    }
}

pub fn same(a: Fallback, b: Fallback) -> bool {
    a == b
}

pub fn shown(f: Fallback) -> String {
    format!("{f:?}")
}

pub fn is_blocking(f: Fallback) -> bool {
    matches!(f, Fallback::Blocking)
}

pub struct Loose(pub u32);

impl PartialEq for Loose {
    fn eq(&self, other: &Loose) -> bool {
        self.0 % 10 == other.0 % 10
    }
}

// Compared part by part, a part's own == among them.
#[cfg_attr(rust_js, rust_js::untagged)]
#[derive(PartialEq)]
pub enum Choice {
    Loose(Loose),
    #[cfg_attr(rust_js, rust_js::name = "off")]
    Off,
}

pub fn same_choice(a: Choice, b: Choice) -> bool {
    a == b
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain('  return b || "blocking";');
  expect(js).toContain('return f === "blocking";');
  const lib = await import(join(dir, "lib.js"));
  expect([lib.fallback(true), lib.fallback(false), lib.describe(true), lib.describe("blocking"), lib.same("blocking", "blocking"),
    lib.same(true, "blocking"), lib.same(true, true), lib.shown("blocking"), lib.shown(false), lib.isBlocking("blocking"), lib.isBlocking(false),
    lib.sameChoice("off", "off"), lib.sameChoice("off", [3]), lib.sameChoice([13], [3]), lib.sameChoice([3], "off")])
    .toEqual([true, "blocking", "bool true", "blocking", true, false, true, "Blocking", "Bool(false)", true, false, true, false, true, false]);
  writeFileSync(join(dir, "text.rs"), `#[cfg_attr(rust_js, rust_js::untagged)]
pub enum Mode<'a> {
    Text(&'a str),
    #[cfg_attr(rust_js, rust_js::name = "auto")]
    Auto,
}

pub fn auto(m: Mode) -> bool {
    matches!(m, Mode::Auto)
}
`);
  const failed = Bun.spawnSync([compiler, join(dir, "text.rs"), "-o", join(dir, "text.js")], { cwd: root, stderr: "pipe" });
  expect([failed.exitCode === 0, failed.stderr.toString()]).toEqual([false, expect.stringContaining("`Auto` and `Text` hold values of one kind")]);
});

test("a closure's last `_` parameters are left out", async () => {
  const dir = fixture("mir-closure-wild-params");
  writeFileSync(join(dir, "lib.rs"), `pub fn ignored(n: u32) -> u32 {
    let f = |_: u32| n + 1;
    let g = |a: u32, _: &str| a * 2;
    f(1) + g(2, "x")
}
pub struct Loud(pub u32);
impl Drop for Loud {
    fn drop(&mut self) {}
}
pub fn dropped(n: u32) -> u32 {
    let h = |_: Loud| n + 1;
    h(Loud(3))
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain(`const f = () => (n + 1) >>> 0;`);
  expect(js).toContain(`const g = (a) => Math.imul(a, 2) >>> 0;`);
  // What it's given and drops, it's given.
  expect(js).toMatch(/const h = \(_\d*\) =>/);
});

test("a value returned after a loop, the function returning early before it, is returned once", async () => {
  const dir = fixture("mir-returned-after-loop");
  writeFileSync(join(dir, "lib.rs"), `pub fn parse(search: &str) -> Vec<String> {
    if search.is_empty() {
        return Vec::new();
    }
    let mut args = Vec::new();
    let queries: Vec<&str> = search.split('&').collect();
    for i in 0..queries.len() {
        let query = queries[i];
        if query.starts_with("a") {
            args.push(query.to_string());
        }
    }
    args
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain(`export function parse(search) {
  if (!search) {
    return [];
  }
  const args = [];
  const queries = search.split("&");
  for (let i = 0; i < queries.length; i++) {
    const query = queries[i];
    if (query.startsWith("a")) {
      args.push(query);
    }
  }
  return args;
}`);
});

// A function's value of a \`match\`'s arms is returned from each arm; its
// \`if\`/\`else\` of two values is one conditional, as a variable's is.
test("a function's match returns each arm, and its if of two values is one conditional", async () => {
  const dir = fixture("mir-returned-branches");
  writeFileSync(join(dir, "lib.rs"), `pub fn two(s: &str) -> u32 {
    match s {
        "a" => 1,
        _ => 2,
    }
}
pub fn cond(s: &str) -> u32 {
    if s == "a" { 1 } else { 2 }
}
pub fn bools(b: bool) -> u32 {
    match b {
        true => 1,
        false => 2,
    }
}
pub fn opt(o: Option<u32>) -> u32 {
    match o {
        Some(x) => x,
        None => 0,
    }
}
pub fn early(g: bool, x: u32) -> bool {
    if !g {
        return false;
    }
    x > 1
}
pub fn any(a: bool, b: bool) -> bool {
    a || b
}
pub fn shown(o: Option<u32>) -> String {
    match o {
        Some(_) => format!("#{}", 1),
        None => String::new(),
    }
}
pub struct F { pub n: u32 }
pub fn either(o: Option<&F>, r: bool) -> bool {
    o.is_some() || r
}
pub fn colors(message: &str) -> &'static str {
    match message {
        "warn" => "yellow",
        "error" => "red",
        _ => "gray",
    }
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain(`export function two(s) {
  if (s === "a") {
    return 1;
  }
  return 2;
}`);
  expect(js).toContain(`export function cond(s) {
  return s === "a" ? 1 : 2;
}`);
  expect(js).toContain(`export function bools(b) {
  if (b) {
    return 1;
  }
  return 2;
}`);
  expect(js).toContain(`export function opt(o) {
  if (o != null) {
    return o;
  }
  return 0;
}`);
  expect(js).toContain(`export function colors(message) {
  if (message === "warn") {
    return "yellow";
  }
  if (message === "error") {
    return "red";
  }
  return "gray";
}`);
  expect(js).toContain("  return a || b;\n");
  // A `return` the program wrote stays where it wrote it.
  expect(js).toContain("export function early(g, x) {\n  if (!g) {\n    return false;\n  }\n  return x > 1;\n}");
  // An arm a macro begins is in its place too.
  expect(js).toContain("export function shown(o) {\n  if (o != null) {\n");
  // A `bool` of what's not one, `!!o`.
  expect(js).toContain("  return !!o || r;\n");
  const lib = await import(join(dir, "mir.js"));
  expect([lib.either({ n: 1 }, false), lib.either(undefined, false)]).toEqual([true, false]);
});

test("an `if !c` from MIR, which MIR tests as `c`, keeps its branch first", async () => {
  const dir = fixture("mir-negated-if");
  writeFileSync(join(dir, "lib.rs"), `pub fn guard(active: bool, v: &mut Vec<u32>) {
    if !active {
        v.push(0);
        return;
    }
    v.push(1);
    v.push(2);
}
pub fn either(active: bool) -> u32 {
    if !active { 1 } else { 2 }
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain(`export function guard(active, v) {
  if (!active) {
    v.push(0);
    return;
  }
  v.push(1);
  v.push(2);
}`);
  expect(js).toContain(`export function either(active) {
  return !active ? 1 : 2;
}`);
});

test("a `match` of an `Option`'s strings from MIR is the `else if` chain a person writes", async () => {
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
  expect(js).toContain(`export function flagged(flag, arg) {
  if (flag === "s") {
    arg += "";
  } else if (flag === "d" || flag === "i") {
    arg = $trim(arg);
  } else if (flag === "f") {
    arg = arg.toUpperCase();
  }
  return arg;
}`);
  expect(js).toContain(`export function named(flag, v) {
  if (flag === "s") {
    v.push(1);
  } else if (flag === "d") {
    v.push(2);
  } else {
    v.push(3);
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

test("tests and values from MIR read as a person writes them: De Morgan's, and `filter` then `or`", async () => {
  const dir = fixture("mir-tests-values");
  writeFileSync(join(dir, "lib.rs"), `pub fn search(section: &str, is_blog_index: bool, order: u32) -> Option<u32> {
    let mut search_order = None;
    if section == "learn" || (section == "blog" && !is_blog_index) {
        search_order = Some(order);
    }
    search_order
}
pub struct Meta { pub title: Option<&'static str> }
pub fn title(meta: &Meta, route: Option<&'static str>) -> &'static str {
    meta.title.filter(|t| !t.is_empty()).or(route.filter(|t| !t.is_empty())).unwrap_or("")
}
pub struct Route { pub title: Option<&'static str> }
pub fn routed(route: Option<&Route>) -> &'static str {
    route.and_then(|r| r.title).filter(|t| !t.is_empty()).unwrap_or("")
}
pub fn page_title(meta: &Meta, route: Option<&Route>) -> &'static str {
    meta.title.filter(|t| !t.is_empty()).or(route.and_then(|r| r.title).filter(|t| !t.is_empty())).unwrap_or("")
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain('  if (section === "learn" || (section === "blog" && !isBlogIndex)) {\n');
  expect(js).toContain('  return meta.title || route || "";\n');
  expect(js).toContain('  return route?.title || "";\n');
  expect(js).toContain('  return meta.title || route?.title || "";\n');
  const lib = await import(join(dir, "mir.js"));
  expect([lib.search("learn", true, 1), lib.search("blog", true, 2), lib.search("blog", false, 3), lib.search("x", false, 4)]).toEqual([1, undefined, 3, undefined]);
  expect([lib.title({ title: "" }, "r"), lib.title({ title: "t" }, "r"), lib.title({ title: undefined }, ""), lib.title({}, undefined)]).toEqual(["r", "t", "", ""]);
});

test("a closure from MIR names its `&x` parameter and knows the `bool`s it captures", async () => {
  const dir = fixture("mir-closure-captured-bools");
  writeFileSync(join(dir, "lib.rs"), `pub fn flags(force: bool, items: &[u32]) -> Vec<bool> {
    items.iter().map(|&n| force || n > 2).collect()
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain("  return items.map((n) => force || n > 2);\n");
  const lib = await import(join(dir, "mir.js"));
  expect([lib.flags(false, [1, 3]), lib.flags(true, [1])]).toEqual([[false, true], [true]]);
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

test("an index from MIR nothing proves in bounds is checked where it's read or written", async () => {
  const dir = fixture("mir-checked-index");
  writeFileSync(join(dir, "lib.rs"), `pub struct Crumb { pub path: Option<&'static str> }
pub fn last_is(crumbs: &[Crumb], path: Option<&str>) -> bool {
    crumbs.len() > 1 && crumbs[crumbs.len() - 1].path == path
}
pub fn second(xs: &[u32]) -> u32 {
    xs[1] + 1
}
pub fn set(xs: &mut [u32], i: usize) {
    xs[i] = 3;
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain("  return crumbs.length > 1 && crumbs[crumbs.length - 1].path == path;\n");
  expect(js).toContain("  return ($index(xs, 1) + 1) >>> 0;\n");
  expect(js).toContain("  xs[$at(xs, i)] = 3;\n");
  const lib = await import(join(dir, "mir.js"));
  expect(() => lib.second([1])).toThrow("index out of bounds: the len is 1 but the index is 1");
  expect(() => lib.set([1], 4)).toThrow("index out of bounds: the len is 1 but the index is 4");
  expect([lib.second([1, 2]), lib.lastIs([{}, { path: "a" }], "a")]).toEqual([3, true]);
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

test("an `if … else if` given as a value from MIR is a conditional where it's read", async () => {
  const dir = fixture("mir-else-if-value");
  writeFileSync(join(dir, "lib.rs"), `pub fn link(href: &str) -> String {
    format!(
        "[{}]",
        if href.starts_with("https://") {
            "ext"
        } else if href.starts_with('#') {
            "hash"
        } else {
            href
        }
    )
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain('  return `[${href.startsWith("https://") ? "ext" : href.startsWith("#") ? "hash" : href}]`;\n');
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

test("a function's value from MIR that branches assign is returned from each, its guards first", async () => {
  const dir = fixture("mir-assigned-returns");
  writeFileSync(join(dir, "lib.rs"), `pub fn footer(href: Option<&str>, children: Vec<String>) -> String {
    let classes = format!("c{}", children.len());
    let Some(href) = href.filter(|href| !href.is_empty()) else {
        return format!("<div {classes}>");
    };
    if href.starts_with("https://") {
        return format!("<ext {href} {classes}>");
    }
    format!("<next {href} {classes}>")
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain(`export function footer(href, children) {
  const classes = \`c\${children.length}\`;
  if (!href) {
    return \`<div \${classes}>\`;
  }
  if (href.startsWith("https://")) {
    return \`<ext \${href} \${classes}>\`;
  }
  return \`<next \${href} \${classes}>\`;
}`);
});

test("a range loop from MIR whose end is a call holds it first, and a slice of `i..i + 1` reads its bounds", async () => {
  const dir = fixture("mir-range-loops");
  writeFileSync(join(dir, "lib.rs"), `pub fn plain(code: &str) -> u32 {
    let mut n = 0;
    for i in 0..code.len() {
        if i > 2 {
            n += 1;
        }
    }
    n
}
pub fn sliced(code: &str) -> u32 {
    let mut n = 0;
    for i in 0..3 {
        if &code[i..i + 1] == "a" {
            n += 1;
        }
    }
    n
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain("  const end = $byteLen(code);\n  for (let i = 0; i < end; i++) {\n");
  expect(js).toContain('    if ($strSlice(code, i, i + 1) === "a") {\n');
  const lib = await import(join(dir, "mir.js"));
  expect([lib.plain("abcdef"), lib.sliced("aba")]).toEqual([3, 2]);
});

test("a `format!`'s arguments read out of order, and a string's `chars()` iterated, from MIR read as written", async () => {
  const dir = fixture("mir-format-chars");
  writeFileSync(join(dir, "lib.rs"), `pub fn lines(code: &str) -> Vec<String> {
    let mut line_output: Vec<String> = Vec::new();
    let mut final_output = Vec::new();
    let mut line_index = 0;
    for c in code.chars() {
        if c == '\\n' {
            let current = line_index;
            final_output.push(format!("{current}:{}", line_output.join("")));
            line_output = Vec::new();
            line_index += 1;
        } else {
            line_output.push(c.to_string());
        }
    }
    final_output
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain("  for (const c of code) {\n");
  expect(js).toContain('      const arg = lineOutput.join("");\n      finalOutput.push(`${current}:${arg}`);\n');
  const lib = await import(join(dir, "mir.js"));
  expect(lib.lines("ab\ncd\n")).toEqual(["0:ab", "1:cd"]);
});

test("a cell from MIR only its function reads and sets is its variable", async () => {
  const dir = fixture("mir-plain-cells");
  writeFileSync(join(dir, "lib.rs"), `use std::cell::Cell;
use std::rc::Rc;
pub fn counter(f: &dyn Fn(&dyn Fn())) -> u32 {
    let n = Rc::new(Cell::new(0));
    let m = n.clone();
    f(&move || m.set(m.get() + 1));
    n.get()
}
pub fn local(f: &dyn Fn(&dyn Fn())) -> bool {
    let active = Cell::new(true);
    f(&|| active.set(false));
    active.get()
}
fn bump(c: &Cell<u32>) {
    c.set(c.get() + 1);
}
pub fn lent() -> u32 {
    let c = Cell::new(1);
    bump(&c);
    c.get()
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain(`export function counter(f) {
  let n = 0;
  f(() => {
    n = (n + 1) >>> 0;
  });
  return n;
}`);
  expect(js).toContain(`export function local(f) {
  let active = true;
  f(() => {
    active = false;
  });
  return active;
}`);
  // One given whole is a cell.
  expect(js).toContain("  const c = { value: 1 };\n  bump(c);\n");
  const lib = await import(join(dir, "mir.js"));
  expect([lib.counter((g: () => void) => { g(); g(); }), lib.local((g: () => void) => g()), lib.lent()]).toEqual([2, false, 2]);
});

test("a `while` loop from MIR tests where it's written", async () => {
  const dir = fixture("mir-while-tests");
  writeFileSync(join(dir, "lib.rs"), `pub fn trimmed(mut messages: Vec<u32>, max: usize) -> Vec<u32> {
    while messages.len() > max {
        messages.remove(0);
    }
    messages
}
pub fn counted(mut n: u32) -> u32 {
    let mut steps = 0;
    while n > 1 {
        n /= 2;
        steps += 1;
    }
    steps
}
pub fn stopped(mut n: u32) -> u32 {
    while n > 1 {
        if n == 7 {
            return 70;
        }
        if n == 9 {
            break;
        }
        n /= 2;
    }
    n + 100
}
pub fn looped(mut n: u32) -> u32 {
    loop {
        if n <= 1 {
            return 1;
        }
        if n == 9 {
            break;
        }
        n /= 2;
    }
    n + 100
}
pub fn halved(mut n: u32, out: &mut Vec<u32>) {
    while n > 1 {
        n /= 2;
        if n == 7 {
            break;
        }
        out.push(n);
    }
    out.push(0);
}
pub fn spun(mut n: u32) -> u32 {
    loop {
        if n <= 1 {
            return n;
        }
        n /= 2;
    }
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain("  while (messages.length > max) {\n    $removeAt(messages, 0);\n  }\n  return messages;\n");
  expect(js).toContain("  while (n > 1) {\n    n = (n / 2) >>> 0;\n    steps = (steps + 1) >>> 0;\n  }\n  return steps;\n");
  expect(js).toContain("  while (n > 1) {\n");
  // A `loop` the program wrote stays one.
  expect(js).toContain("export function looped(n) {\n  while (true) {\n    if (n <= 1) {\n      return 1;\n    }\n");
  expect(js).toContain("export function spun(n) {\n  while (true) {\n    if (n <= 1) {\n      return n;\n    }\n");
  expect(js).toContain("  while (n > 1) {\n    if (n === 7) {\n      return 70;\n    }\n    if (n === 9) {\n      break;\n    }\n    n = (n / 2) >>> 0;\n  }\n  return n + 100;\n");
  const lib = await import(join(dir, "mir.js"));
  const pushed: number[] = [];
  lib.halved(30, pushed);
  expect([lib.trimmed([1, 2, 3], 1), lib.counted(9), pushed, lib.stopped(15), lib.stopped(9), lib.stopped(40), lib.looped(18), lib.looped(4)]).toEqual([[3], 3, [15, 0], 70, 109, 101, 109, 1]);
});

test("a chain collected from MIR is a spread", async () => {
  const dir = fixture("mir-chain-collect");
  writeFileSync(join(dir, "lib.rs"), `use std::collections::VecDeque;
const MAX: usize = 2;
pub fn capped(prev: &VecDeque<u32>, more: Vec<u32>) -> VecDeque<u32> {
    let mut messages: VecDeque<u32> = prev.iter().cloned().chain(more).collect();
    while messages.len() > MAX {
        messages.pop_front();
    }
    messages
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain("  const messages = [...prev, ...more];\n");
  const lib = await import(join(dir, "mir.js"));
  expect(lib.capped([1, 2], [3])).toEqual([2, 3]);
});

test("a loop's `continue`s from MIR are where the program wrote them", async () => {
  const dir = fixture("mir-written-continues");
  writeFileSync(join(dir, "lib.rs"), `pub struct Route { pub path: Option<String>, pub header: bool, pub title: Option<String> }
pub fn pages(routes: &[Route], out: &mut Vec<String>) {
    for route in routes {
        if route.header {
            continue;
        }
        if let Some(p) = &route.path && p.starts_with("http") {
            continue;
        }
        if let Some(t) = &route.title {
            out.push(t.clone());
        } else if let Some(p) = &route.path {
            out.push(p.clone());
        }
    }
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain(`  for (const route of routes) {
    if (route.header) {
      continue;
    }
    if (route.path != null && route.path.startsWith("http")) {
      continue;
    }
    if (route.title != null) {
      out.push(route.title);
    } else if (route.path != null) {
      out.push(route.path);
    }
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
  expect(js).toContain("  const f = xs.find((x) => x % 2 === 0);\n  if (f == null) {\n    return;\n  }\n  return Math.imul(f, 10);\n");
  const lib = await import(join(dir, "mir.js"));
  expect([lib.firstEven([1, 4]), lib.firstEven([1])]).toEqual([40, undefined]);
});

// An unstable sort and a selection from MIR (ADR 0342).
test("an unstable sort from MIR sorts as Rust's", async () => {
  const dir = fixture("mir-unstable-sort");
  writeFileSync(join(dir, "lib.rs"), `pub fn down(mut v: Vec<i32>) -> Vec<i32> {
    v.sort_unstable_by(|a, b| b.cmp(a));
    v
}
pub fn by_len(mut v: Vec<&'static str>) -> Vec<&'static str> {
    v.sort_unstable_by_key(|s| s.len());
    v
}
pub fn third(mut v: Vec<u32>) -> u32 {
    *v.select_nth_unstable(2).1
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const lib = await import(join(dir, "mir.js"));
  expect([lib.down([3, 1, 2]), lib.byLen(["ccc", "a", "bb"]), lib.third([5, 1, 4, 2, 3])]).toEqual([[3, 2, 1], ["a", "bb", "ccc"], 3]);
});

// A slice's own methods from MIR: split by a test, filled, checked sorted,
// searched, cut in chunks.
test("a slice's own methods from MIR run as Rust's", async () => {
  const dir = fixture("mir-slice-methods");
  writeFileSync(join(dir, "lib.rs"), `pub fn parts(v: &[i32]) -> Vec<usize> {
    v.split(|&x| x < 0).map(|p| p.len()).collect()
}
pub fn zeroed(mut v: Vec<i32>) -> Vec<i32> {
    v.fill(0);
    v
}
pub fn facts(v: &[i32]) -> (bool, usize, usize) {
    (v.is_sorted(), v.partition_point(|&x| x < 3), v.chunks_exact(2).count())
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const lib = await import(join(dir, "mir.js"));
  expect([lib.parts([3, -1, 4, 5, -2, 6]), lib.zeroed([1, 2]), lib.facts([1, 2, 3, 4, 5])]).toEqual([[1, 2, 1], [0, 0], [true, 2, 2]]);
});

// Two iterators equal from MIR, \`a.eq(b)\`: each item in turn, as Rust
// steps them, till one differs or either ends.
test("iterators compared equal from MIR step both", async () => {
  const dir = fixture("mir-iter-eq");
  writeFileSync(join(dir, "lib.rs"), `pub fn palindrome(w: &str) -> bool {
    w.chars().eq(w.chars().rev())
}
pub fn same(a: &[u32], b: &[u32]) -> bool {
    a.iter().eq(b.iter())
}
pub fn differ(a: Vec<String>, b: Vec<String>) -> bool {
    a.into_iter().ne(b)
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")], 600_000);
  const lib = await import(join(dir, "mir.js"));
  expect(["civic", "bob", "dog", ""].map(lib.palindrome)).toEqual([true, true, false, true]);
  expect([lib.same([1, 2], [1, 2]), lib.same([1, 2], [1]), lib.differ(["a"], ["a"]), lib.differ(["a"], ["b"])]).toEqual([true, false, false, true]);
});

// A B-tree's range of its keys from MIR (ADR 0325).
test("a B-tree's range from MIR gives the entries in it", async () => {
  const dir = fixture("mir-tree-range");
  writeFileSync(join(dir, "lib.rs"), `use std::collections::BTreeMap;
pub fn from_two(m: &BTreeMap<u32, &'static str>) -> Vec<u32> {
    m.range(2..).map(|(k, _)| *k).collect()
}
pub fn upto(m: &BTreeMap<u32, &'static str>) -> Vec<&'static str> {
    m.range(..=2).map(|(_, v)| *v).collect()
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const lib = await import(join(dir, "mir.js"));
  const m = new Map([[3, "c"], [1, "a"], [2, "b"]]);
  expect([lib.fromTwo(m), lib.upto(m)]).toEqual([[2, 3], ["a", "b"]]);
});

// A derived order of an enum with fields: by variant, as declared, then
// the variant's fields in turn, as Rust's derive compares them; a float's
// \`NaN\` among them unordered.
test("a derived order of an enum with fields is by variant, then fields", async () => {
  const dir = fixture("derived-enum-order");
  writeFileSync(join(dir, "lib.rs"), `#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Shape { Circle { r: u32 }, Rect(u32, u32), Dot }
#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub enum Scaled { Dot, Scale(f64) }
pub fn sorted(mut v: Vec<Shape>) -> Vec<Shape> {
    v.sort();
    v
}
pub fn order(a: &Scaled, b: &Scaled) -> Option<std::cmp::Ordering> {
    a.partial_cmp(b)
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const lib = await import(join(dir, "mir.js"));
  const rect = (a: number, b: number) => ({ TAG: "Rect", _0: a, _1: b });
  const circle = (r: number) => ({ TAG: "Circle", r });
  expect(lib.sorted([rect(2, 3), "Dot", circle(2), rect(1, 9), circle(1), rect(2, 1)])).toEqual([circle(1), circle(2), rect(1, 9), rect(2, 1), rect(2, 3), "Dot"]);
  const scale = (x: number) => ({ TAG: "Scale", _0: x });
  expect([lib.order(scale(NaN), scale(1)), lib.order("Dot", scale(NaN)), lib.order(scale(2), scale(1))]).toEqual([undefined, -1, 1]);
});

// \`replacen\` of any count, as Rust's: the first \`n\` matches, an empty
// pattern's at each \`char\`'s boundary.
test("replacen replaces as many as it's told", async () => {
  const dir = fixture("replacen-count");
  writeFileSync(join(dir, "lib.rs"), `pub fn two(s: &str) -> String {
    s.replacen('l', "_", 2)
}
pub fn many(s: &str, n: usize) -> String {
    s.replacen("ab", "X", n)
}
pub fn empty(s: &str) -> String {
    s.replacen("", "-", 2)
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const lib = await import(join(dir, "mir.js"));
  expect([lib.two("hello world"), lib.many("ababab", 0), lib.many("ababab", 2), lib.many("ababab", 9), lib.empty("héé")]).toEqual(["he__o world", "ababab", "XXab", "XXX", "-h-éé"]);
});

// An enum made a number, \`ordering as i32\` or a fieldless one's, from
// MIR: its discriminant.
test("an enum cast to a number from MIR is its discriminant", async () => {
  const dir = fixture("mir-enum-cast");
  writeFileSync(join(dir, "lib.rs"), `#[derive(Clone, Copy)]
pub enum Color { Red, Green = 5, Blue }
pub fn order(a: &str, b: &str) -> i32 {
    a.cmp(b) as i32
}
pub fn code(c: Color) -> i32 {
    c as i32
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const lib = await import(join(dir, "mir.js"));
  expect([lib.order("abc", "abd"), lib.order("b", "a"), lib.code("Red"), lib.code("Green"), lib.code("Blue")]).toEqual([-1, 1, 0, 5, 6]);
});

// A float's sum from MIR starts at -0.0, as Rust's does: an empty sum and
// one of negative zeros are -0.0, not 0.
test("a float's sum from MIR keeps the sign of zero", async () => {
  const dir = fixture("mir-float-sum");
  writeFileSync(join(dir, "lib.rs"), `pub fn total(xs: &[f64]) -> f64 {
    xs.iter().sum()
}
pub fn owned(xs: Vec<f64>) -> f64 {
    xs.into_iter().sum()
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const lib = await import(join(dir, "mir.js"));
  expect([Object.is(lib.total([]), -0), Object.is(lib.total([-0]), -0), Object.is(lib.owned([-0, -0]), -0), lib.total([1, 2])]).toEqual([true, true, true, 3]);
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

// Sets and maps from MIR made of arrays and read as arrays:
// \`new Set([1, 2, 3])\`, a set's algebra and a sorted map's entries mapped.
test("sets and maps from MIR are made of arrays and read as them", async () => {
  const dir = fixture("mir-sets-maps");
  writeFileSync(join(dir, "lib.rs"), `use std::collections::{BTreeMap, HashSet};
pub fn both() -> Vec<u32> {
    let a: HashSet<u32> = [1, 2, 3].into_iter().collect();
    let b: HashSet<u32> = [2, 3, 4].into_iter().collect();
    let mut both: Vec<u32> = a.intersection(&b).copied().collect();
    both.sort();
    both
}
pub fn keys(m: &BTreeMap<String, u32>) -> Vec<String> {
    m.iter().map(|(k, v)| format!("{k}{v}")).collect()
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain("  const a = new Set([1, 2, 3]);\n");
  expect(js).toContain('  const both = $setAlgebra(a, b, "intersection");\n');
  expect(js).toContain("  return $sortedEntries(m, $cmp).map(");
  const lib = await import(join(dir, "mir.js"));
  expect([lib.both(), lib.keys(new Map([["b", 1], ["a", 2]]))]).toEqual([[2, 3], ["a2", "b1"]]);
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

// A loop the program leaves with \`break\`, its function's value after it,
// from MIR, breaks, the value returned after it, as written:
// not the function's \`return\` moved into the loop.
test("a loop's break from MIR is a break, the return after it", async () => {
  const dir = fixture("mir-loop-break-return");
  writeFileSync(join(dir, "lib.rs"), `pub fn odds() -> Vec<i32> {
    let mut out = Vec::new();
    let mut i = 0;
    loop {
        i += 1;
        if i > 7 {
            break;
        }
        out.push(i);
    }
    out
}
pub fn drained(mut stack: Vec<u32>) -> String {
    let mut out = String::new();
    while let Some(top) = stack.pop() {
        out.push_str(&top.to_string());
    }
    out
}
pub fn early(xs: &[i32]) -> i32 {
    for &x in xs {
        if x > 2 {
            return x;
        }
    }
    0
}
pub fn root(n: u32) -> u32 {
    let mut i = 0;
    loop {
        i += 1;
        if i * i > n {
            return i;
        }
    }
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain("    if (i > 7) {\n      break;\n    }\n");
  expect(js).toContain("  }\n  return out;\n}\n\nexport function drained");
  expect(js).toMatch(/\n  }\n  return out;\n}\n\nexport function early/);
  // A \`return\` the program wrote stays.
  expect(js).toContain("      return x;\n");
  expect(js).toContain("      return i;\n");
  const lib = await import(join(dir, "mir.js"));
  expect([lib.odds(), lib.drained([1, 2]), lib.early([1, 5]), lib.early([]), lib.root(10)]).toEqual([[1, 2, 3, 4, 5, 6, 7], "21", 5, 0, 4]);
});

// An \`else if\` the program wrote at a loop's end, after an \`if\` it ended
// with a \`continue\`, stays an \`else if\` from MIR: not a \`continue\` of
// its own and an \`if\` after, as react.dev's llms.txt sections its routes.
test("an else-if at a loop's end from MIR stays one past a written continue", async () => {
  const dir = fixture("mir-else-if-after-continue");
  writeFileSync(join(dir, "lib.rs"), `#![allow(non_snake_case, dead_code)]
struct RouteItem {
    title: Option<String>,
    path: Option<String>,
    routes: Option<Vec<RouteItem>>,
    has_section_header: Option<bool>,
    section_header: Option<String>,
}

struct Sidebar {
    title: String,
    routes: Vec<RouteItem>,
}

struct Page {
    title: String,
    url: String,
}

struct SubGroup {
    heading: String,
    pages: Vec<Page>,
}

struct Section {
    heading: Option<String>,
    pages: Vec<Page>,
    sub_groups: Vec<SubGroup>,
}

// Clean up section header names (remove version placeholders)
fn cleanSectionHeader(header: &str) -> String { header.trim().to_string() }
pub fn extractSectionedRoutes(routes: &[RouteItem], baseUrl: &str) -> Vec<Section> {
    let mut sections: Vec<Section> = Vec::new();
    let mut currentSection: Option<Section> = None;

    for route in routes {
        // Skip external links
        if route.path.as_ref().is_some_and(|path| path.starts_with("http")) {
            continue;
        }

        // Start a new section when we hit a section header
        if route.has_section_header == Some(true)
            && let Some(sectionHeader) = &route.section_header
        {
            if let Some(section) = currentSection {
                sections.push(section);
            }
            currentSection = Some(Section {
                heading: Some(cleanSectionHeader(sectionHeader)),
                pages: Vec::new(),
                sub_groups: Vec::new(),
            });
            continue;
        }

        // If no section started yet, skip
        let Some(currentSection) = &mut currentSection else {
            continue;
        };

        // Route with children - create a sub-group
        if let Some(title) = &route.title
            && let Some(children) = &route.routes
            && !children.is_empty()
        {
            let mut subGroup = SubGroup {
                heading: title.clone(),
                pages: Vec::new(),
            };

            // Include parent page if it has a path
            if let Some(path) = &route.path {
                subGroup.pages.push(Page {
                    title: title.clone(),
                    url: format!("{baseUrl}{path}.md"),
                });
            }

            // Add child pages
            for child in children {
                if let Some(childTitle) = &child.title
                    && let Some(childPath) = &child.path
                    && !childPath.starts_with("http")
                {
                    subGroup.pages.push(Page {
                        title: childTitle.clone(),
                        url: format!("{baseUrl}{childPath}.md"),
                    });
                }
            }

            if !subGroup.pages.is_empty() {
                currentSection.sub_groups.push(subGroup);
            }
        }
        // Single page without children
        else if let Some(title) = &route.title
            && let Some(path) = &route.path
        {
            currentSection.pages.push(Page {
                title: title.clone(),
                url: format!("{baseUrl}{path}.md"),
            });
        }
    }

    // Don't forget the last section
    if let Some(section) = currentSection {
        sections.push(section);
    }

    sections
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain("    } else if (route.title != null && route.path != null) {\n");
  // Nor a \`continue\` where the body ends anyway.
  expect(js).not.toMatch(/\n      continue;\n    } else if/);
  const lib = await import(join(dir, "mir.js"));
  const routes = [
    { hasSectionHeader: true, sectionHeader: "A" },
    { title: "t", path: "/p" },
    { title: "g", routes: [{ title: "c", path: "/c" }] },
  ];
  expect(lib.extractSectionedRoutes(routes, "x").map((s: { heading: string; pages: unknown[]; subGroups: unknown[] }) => [s.heading, s.pages.length, s.subGroups.length])).toEqual([["A", 1, 1]]);
});

// react.dev's llms.txt groups its routes: an `else if` the loop's body ends
// with, after an `if` that doesn't end in a `continue`, stays an `else if`,
// not a `continue` MIR's block for it makes and an `if` after.
test("react.dev's llms.txt groups its routes with an else-if at the loop's end", async () => {
  const dir = fixture("mir-grouped-routes");
  writeFileSync(join(dir, "lib.rs"), `#![allow(non_snake_case, dead_code)]
pub struct RouteItem {
    title: Option<String>,
    path: Option<String>,
    routes: Option<Vec<RouteItem>>,
    has_section_header: Option<bool>,
}

pub struct Page {
    title: String,
    url: String,
}

pub struct SubGroup {
    heading: String,
    pages: Vec<Page>,
}

pub fn extractGroupedRoutes(routes: &[RouteItem], baseUrl: &str) -> Vec<SubGroup> {
    let mut groups: Vec<SubGroup> = Vec::new();

    for route in routes {
        // Skip section headers
        if route.has_section_header == Some(true) {
            continue;
        }

        // Skip external links
        if route.path.as_ref().is_some_and(|path| path.starts_with("http")) {
            continue;
        }

        // Route with children - create a group
        if let Some(title) = &route.title
            && let Some(children) = &route.routes
            && !children.is_empty()
        {
            let mut pages: Vec<Page> = Vec::new();

            // Include parent page if it has a path
            if let Some(path) = &route.path {
                pages.push(Page {
                    title: title.clone(),
                    url: format!("{baseUrl}{path}.md"),
                });
            }

            // Add child pages
            for child in children {
                if let Some(childTitle) = &child.title
                    && let Some(childPath) = &child.path
                    && !childPath.starts_with("http")
                {
                    pages.push(Page {
                        title: childTitle.clone(),
                        url: format!("{baseUrl}{childPath}.md"),
                    });
                }
            }

            if !pages.is_empty() {
                groups.push(SubGroup {
                    heading: title.clone(),
                    pages,
                });
            }
        }
        // Single page without children - group under its own heading
        else if let Some(title) = &route.title
            && let Some(path) = &route.path
        {
            groups.push(SubGroup {
                heading: title.clone(),
                pages: vec![Page {
                    title: title.clone(),
                    url: format!("{baseUrl}{path}.md"),
                }],
            });
        }
    }

    groups
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain("        groups.push({ heading: route.title, pages });\n      }\n    } else if (route.title != null && route.path != null) {\n");
  expect(js).not.toContain("\n    if (route.title != null && route.path != null) {");
  const lib = await import(join(dir, "mir.js"));
  const routes = [
    { hasSectionHeader: true, title: "H", path: "/h" },
    { title: "E", path: "https://e" },
    { title: "G", path: "/g", routes: [{ title: "c", path: "/c" }, { title: "x", path: "http://x" }] },
    { title: "T", path: "/t" },
    { title: "N" },
  ];
  const grouped = lib.extractGroupedRoutes(routes, "b").map((g: { heading: string; pages: { url: string }[] }) => [g.heading, g.pages.map((p) => p.url)]);
  expect(grouped).toEqual([["G", ["b/g.md", "b/c.md"]], ["T", ["b/t.md"]]]);
});

// react.dev's CodeBlock places its inline decorators: a loop's body ending
// in an `if`'s branch, here its last insert past the `panic!`s' `if`s, ends
// with no `continue` MIR's block for it makes, which does nothing there.
test("react.dev's CodeBlock places its decorators with no continue at the loop's end", async () => {
  const dir = fixture("mir-decorator-ends");
  writeFileSync(join(dir, "lib.rs"), `use std::collections::HashMap;

pub struct InlineDecorator {
    pub line: u32,
    pub start_column: u32,
    pub end_column: u32,
    pub class_name: String,
}

pub fn ends(lines: &[&str], inline_decorators: Vec<InlineDecorator>) -> Vec<(u32, String)> {
    let mut decorator_starts = HashMap::new();
    let mut decorator_ends = HashMap::new();
    for decorator in inline_decorators {
        // Find where inline highlight starts and ends.
        let mut decorator_start = 0;
        for i in 0..decorator.line - 1 {
            decorator_start += lines[i as usize].len() as u32 + 1;
        }
        decorator_start += decorator.start_column;
        let decorator_end = decorator_start + (decorator.end_column - decorator.start_column);
        if decorator_starts.contains_key(&decorator_start) {
            panic!("Already opened decorator at {decorator_start}");
        }
        decorator_starts.insert(decorator_start, decorator.class_name.clone());
        if decorator_ends.contains_key(&decorator_end) {
            panic!("Already closed decorator at {decorator_end}");
        }
        decorator_ends.insert(decorator_end, decorator.class_name);
    }
    let mut out: Vec<(u32, String)> = decorator_ends.into_iter().collect();
    out.sort();
    out
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain("    decoratorEnds.set(decoratorEnd, decorator.className);\n  }\n");
  expect(js).not.toContain("continue;");
  const lib = await import(join(dir, "mir.js"));
  const decorator = (line: number, startColumn: number, endColumn: number, className: string) => ({ line, startColumn, endColumn, className });
  expect(lib.ends(["ab", "cde"], [decorator(2, 1, 3, "k"), decorator(1, 0, 1, "s")])).toEqual([[1, "s"], [6, "k"]]);
  expect(() => lib.ends(["ab"], [decorator(1, 0, 1, "a"), decorator(1, 1, 1, "b")])).toThrow("Already closed decorator at 1");
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

// A wide field updated through a \`&mut\`, from MIR, is written where it's
// made, \`m.cents = BigInt.asIntN(64, m.cents + 1n)\`: wrapping it does
// nothing a place could see.
test("a wide field updated from MIR is written where it's made", async () => {
  const dir = fixture("mir-wide-field");
  writeFileSync(join(dir, "lib.rs"), `pub struct Money { pub cents: i64 }
pub fn bump(m: &mut Money) {
    m.cents += 1;
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain("export function bump(m) {\n  m.cents = BigInt.asIntN(64, m.cents + 1n);\n}");
  const lib = await import(join(dir, "mir.js"));
  const m = { cents: 9223372036854775807n };
  lib.bump(m);
  expect(m.cents).toBe(-9223372036854775808n);
});

// What's left of a string's \`chars()\` once stepped, \`as_str()\`, from MIR:
// the rest of it, \`$restStr(c)\`.
test("the rest of a string's chars from MIR is a string", async () => {
  const dir = fixture("mir-chars-rest");
  writeFileSync(join(dir, "lib.rs"), `pub fn cap(w: &str) -> String {
    let mut c = w.chars();
    match c.next() {
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
        None => String::new(),
    }
}
// Stepped from its end, and not stepped at all.
pub fn init(w: &str) -> String {
    let mut c = w.chars();
    c.next_back();
    c.as_str().to_string()
}
pub fn whole(w: &str) -> String {
    w.chars().as_str().to_string()
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain("$restStr(c)");
  const lib = await import(join(dir, "mir.js"));
  expect([lib.cap("hello"), lib.cap(""), lib.cap("é!")]).toEqual(["Hello", "", "É!"]);
  expect([lib.init("abc"), lib.init(""), lib.whole("abc")]).toEqual(["ab", "", "abc"]);
  expect(js).toContain('  return Array.from(w).join("");\n');
});

// A match of three arms or more giving a variable its value, from MIR, is
// the chain of them: not its last two folded, \`c ? a : b\`.
test("a variable's value of three arms from MIR is their chain", async () => {
  const dir = fixture("mir-three-arms");
  writeFileSync(join(dir, "lib.rs"), `#[derive(Clone, Copy)]
pub enum Kind { Note, Warn { level: u8 }, Tip(&'static str) }
pub fn class(k: Kind) -> String {
    let cls = match k {
        Kind::Note => "note",
        Kind::Warn { .. } => "warn",
        Kind::Tip(_) => "tip",
    };
    format!("{cls}!")
}
// Of a value MIR holds, \`tmp\`, its first branch doing more than giving
// it, as react.dev's CodeDiagram picks a child.
pub fn pick(x: i32) -> String {
    let shown: Box<dyn std::fmt::Display> = if x > 5 {
        let m = x * 2;
        Box::new(m + 1)
    } else if x < 0 {
        Box::new("neg")
    } else {
        Box::new(x)
    };
    shown.to_string()
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain('  if (k === "Note") {\n    cls = "note";\n  } else if (k.TAG === "Warn") {\n    cls = "warn";\n  } else {\n    cls = "tip";\n  }\n');
  expect(js).toMatch(/} else if \(x < 0\) {\n    \w+ = /);
  const lib = await import(join(dir, "mir.js"));
  expect([lib.class$("Note"), lib.class$({ TAG: "Warn", level: 1 }), lib.class$({ TAG: "Tip", _0: "x" })]).toEqual(["note!", "warn!", "tip!"]);
  expect([lib.pick(6), lib.pick(-1), lib.pick(2)]).toEqual(["13", "neg", "2"]);
});

// A count kept in a map, `*m.entry(k).or_insert(0) += 1`, from MIR is the
// value there or put in, then set: no handle on the key's value.
test("a count kept in a map from MIR is its value, then set", async () => {
  const dir = fixture("mir-map-count");
  writeFileSync(join(dir, "lib.rs"), `use std::collections::HashMap;
pub fn counts(words: &str) -> Vec<(String, u32)> {
    let mut counts: HashMap<&str, u32> = HashMap::new();
    for w in words.split(' ') {
        *counts.entry(w).or_insert(0) += 1;
    }
    let mut all: Vec<(String, u32)> = counts.into_iter().map(|(k, v)| (k.to_string(), v)).collect();
    all.sort();
    all
}
pub fn bumped(mut m: HashMap<&'static str, u32>) -> u32 {
    *m.entry("a").or_insert(0) += 5;
    m["a"]
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain("    const current = $orInsert(counts, w, 0);\n    counts.set(w, (current + 1) >>> 0);\n");
  expect(js).toContain('  const current = $orInsert(m, "a", 0);\n  m.set("a", (current + 5) >>> 0);\n');
  const lib = await import(join(dir, "mir.js"));
  expect(lib.counts("a b a")).toEqual([["a", 2], ["b", 1]]);
  expect([lib.bumped(new Map()), lib.bumped(new Map([["a", 1]]))]).toEqual([5, 6]);
});

// A slice's items with their indices, `iter().enumerate()`, from MIR are
// the array's `entries()`, and a loop's label the program's own; a
// `break 'outer` out of the loop inside keeps both loops.
test("nested loops from MIR read as written: entries, labels, and a labeled break", async () => {
  const dir = fixture("mir-nested-loops");
  writeFileSync(join(dir, "lib.rs"), `pub fn place(grid: &[Vec<u32>]) -> Option<(usize, usize)> {
    for (r, row) in grid.iter().enumerate() {
        for (c, &v) in row.iter().enumerate() {
            if v == 4 {
                return Some((r, c));
            }
        }
    }
    None
}
pub fn skip(grid: &[Vec<u32>]) -> u32 {
    let mut n = 0;
    'rows: for row in grid {
        for &v in row {
            if v == 0 {
                continue 'rows;
            }
            n += v;
        }
    }
    n
}
pub fn first(grid: &[Vec<u32>]) -> Option<u32> {
    let mut found = None;
    'outer: for row in grid {
        for &v in row {
            if v > 3 {
                found = Some(v);
                break 'outer;
            }
        }
    }
    found
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain("  for (const [r, row] of grid.entries()) {\n    for (const [c, v] of row.entries()) {\n");
  expect(js).toContain("  rows: for (const row of grid) {");
  expect(js).toContain("        continue rows;");
  expect(js).toContain(`  outer: for (const row of grid) {
    for (const v of row) {
      if (v > 3) {
        found = v;
        break outer;
      }
    }
  }
  return found;`);
  const lib = await import(join(dir, "mir.js"));
  const grid = [[1, 2], [0, 5], [3, 4]];
  expect([lib.place(grid), lib.place([[1]]), lib.skip(grid), lib.first(grid), lib.first([[1]])]).toEqual([[2, 1], undefined, 10, 5, undefined]);
});

// MIR's checks of a `%` or `/`, `2 === -1` of a constant divisor, are
// none: the division checks what can fail, `$rem(a, b, min)`.
test("a division by a constant from MIR has no overflow check", async () => {
  const dir = fixture("mir-constant-divisor");
  writeFileSync(join(dir, "lib.rs"), `pub fn odds(n: i32) -> Vec<i32> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < n {
        i += 1;
        if i % 2 == 0 {
            continue;
        }
        out.push(i / 3);
    }
    out
}
pub fn by(a: i32, b: i32) -> i32 {
    a % b
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain(`    if (i % 2 === 0) {
      continue;
    }
    out.push((i / 3) | 0);`);
  const lib = await import(join(dir, "mir.js"));
  expect(lib.odds(7)).toEqual([0, 1, 1, 2]);
  // A divisor not known is checked where it divides, once.
  expect(js).toContain("export function by(a, b) {\n  return $rem(a, b, -2147483648) | 0;\n}");
  expect(() => lib.by(-2147483648, -1)).toThrow("remainder with overflow");
  expect(() => lib.by(1, 0)).toThrow("divisor of zero");
});

test("a test from MIR that `x` is set and true reads `x`", async () => {
  const dir = fixture("mir-set-and-true");
  writeFileSync(join(dir, "lib.rs"), `pub enum Message {
    Start { first_load: Option<bool> },
    Refresh,
}
pub fn starts(message: &Message, out: &mut Vec<u32>) {
    if matches!(message, Message::Start { first_load: Some(true) }) || matches!(message, Message::Refresh) {
        out.push(1);
    }
}
pub fn titled(error_code: Option<&str>) -> String {
    match error_code {
        Some(code) if !code.is_empty() => format!("#{code}"),
        _ => "none".to_string(),
    }
}
// A field set and true, \`props.meta\`, and one after another test.
pub struct Props {
    pub meta: Option<String>,
}
pub fn meta(props: &Props) -> usize {
    let mut n = 0;
    if let Some(meta) = &props.meta
        && !meta.is_empty()
    {
        n = meta.len();
    }
    n
}
pub fn logs(message: &Message, out: &mut Vec<u32>) {
    if let Message::Start { first_load } = message
        && *first_load == Some(true)
    {
        out.push(2);
    }
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).not.toContain("!= null &&");
  expect(js).toContain("  if (props.meta) {\n");
  const lib = await import(join(dir, "mir.js"));
  const pushed: number[] = [];
  lib.starts({ TAG: "Start", firstLoad: true }, pushed);
  lib.starts({ TAG: "Start", firstLoad: false }, pushed);
  lib.starts({ TAG: "Start" }, pushed);
  lib.starts("Refresh", pushed);
  expect([pushed, lib.titled("e"), lib.titled(""), lib.titled(undefined)]).toEqual([[1, 1], "#e", "none", "none"]);
  lib.logs({ TAG: "Start", firstLoad: true }, pushed);
  lib.logs({ TAG: "Start" }, pushed);
  expect([pushed, lib.meta({ meta: "ab" }), lib.meta({ meta: "" }), lib.meta({})]).toEqual([[1, 1, 2], 2, 0, 0]);
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

test("a closure from MIR captures a variable itself where nothing changes it after", async () => {
  const dir = fixture("mir-captured-after");
  writeFileSync(join(dir, "lib.rs"), `pub fn last(n: u32, hover: &dyn Fn(&dyn Fn() -> u32)) {
    let mut index = 0;
    for _ in 0..n {
        index += 1;
    }
    hover(&move || index);
}
pub fn each(n: u32) -> Vec<u32> {
    let mut made: Vec<Box<dyn Fn() -> u32>> = Vec::new();
    let mut index = 0;
    while index < n {
        made.push(Box::new(move || index));
        index += 1;
    }
    made.iter().map(|f| f()).collect()
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain("  hover(() => index);\n");
  const lib = await import(join(dir, "mir.js"));
  let seen = 0;
  lib.last(3, (f: () => number) => { seen = f(); });
  expect([seen, lib.each(3)]).toEqual([3, [0, 1, 2]]);
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

test("a thread-local only its module reads and sets, from MIR, is its variable (ADR 0270)", async () => {
  const dir = fixture("mir-plain-statics");
  writeFileSync(join(dir, "lib.rs"), `use std::cell::Cell;
thread_local! {
    static COUNT: Cell<u32> = const { Cell::new(0) };
    static SEEN: Cell<bool> = const { Cell::new(false) };
}
pub fn bump() -> u32 {
    COUNT.set(COUNT.get() + 1);
    COUNT.get()
}
pub fn seen() -> bool {
    SEEN.get()
}
thread_local! {
    static LAST: Cell<u32> = const { Cell::new(0) };
}
pub fn remember(f: &dyn Fn(&dyn Fn(u32))) -> u32 {
    f(&|n| LAST.set(n));
    LAST.get()
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain("let COUNT = 0;\nconst SEEN = false;\n");
  expect(js).toContain("let LAST = 0;\n");
  expect(js).toContain("  COUNT = (COUNT + 1) >>> 0;\n  return COUNT;\n");
  const lib = await import(join(dir, "mir.js"));
  expect([lib.bump(), lib.bump(), lib.seen(), lib.remember((g: (n: number) => void) => g(7))]).toEqual([1, 2, false, 7]);
});

test("a `RefCell` field only ever borrowed for a moment, from MIR, is its value (ADR 0362)", async () => {
  const dir = fixture("mir-plain-ref-cells");
  writeFileSync(join(dir, "lib.rs"), `use std::cell::RefCell;
use std::rc::Rc;
pub struct Node {
    pub item: u32,
    pub children: RefCell<Vec<Rc<Node>>>,
}
pub fn tree(items: &[u32]) -> Rc<Node> {
    let root = Rc::new(Node { item: 0, children: RefCell::new(vec![]) });
    for &i in items {
        let node = Rc::new(Node { item: i, children: RefCell::new(vec![]) });
        root.children.borrow_mut().push(node);
    }
    root
}
pub fn count(node: &Node) -> usize {
    if node.children.borrow().len() < 2 {
        return 0;
    }
    node.children.borrow().len()
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain("    const node = { item: i, children: [] };\n    root.children.push(node);\n");
  expect(js).toContain("  if (node.children.length < 2) {\n    return 0;\n  }\n  return node.children.length;\n");
  const lib = await import(join(dir, "mir.js"));
  const root = lib.tree([1, 2, 3]);
  expect([lib.count(root), lib.count(root.children[0])]).toEqual([3, 0]);
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

test("a call from MIR is read in place past an object of constants before it", async () => {
  const dir = fixture("mir-past-constant-objects");
  writeFileSync(join(dir, "lib.rs"), `pub struct Colors { pub accent: &'static str, pub base: &'static str }
pub struct Font { pub body: String, pub mono: String, pub size: &'static str }
pub struct Theme { pub colors: Colors, pub font: Font }
fn joined(parts: &[&str]) -> String { parts.join(", ") }
fn unquoted(s: &str) -> String { s.replace('"', "") }
pub fn theme() -> Theme {
    Theme {
        colors: Colors { accent: "inherit", base: "inherit" },
        font: Font { body: unquoted(&joined(&["a", "b"])), mono: unquoted(&joined(&["c"])), size: "14px" },
    }
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain('    font: { body: unquoted(joined(["a", "b"])), mono: unquoted(joined(["c"])), size: "14px" },\n');
});

// `{:?}` of a value it reads more than once, a `Result`'s tag and what it
// holds, given as it's made: held first, as a value read twice is
// everywhere else, not a function called in place, `((result) => ..)(f())`.
// Not past a call before it, whose order it would change.
test("a value `{:?}` reads twice is held first, not given to a function called in place", async () => {
  const dir = fixture("mir-applied-values");
  writeFileSync(join(dir, "lib.rs"), `fn parse(s: &str) -> Result<u8, String> { s.parse::<u8>().map_err(|_| s.to_string()) }
fn bump(n: &mut u8) -> Option<u8> { *n += 1; Some(*n) }
pub fn main() {
    let first = parse("1");
    println!("{:?} {:?}", first, parse("x"));
    let mut n = 1;
    println!("{} {:?}", n.clone(), bump(&mut n));
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain('const value = parse("x");');
  expect(js).not.toContain("((result) =>");
});

test("an `||` of `matches!` from MIR is one test, at a function's end or not", async () => {
  const dir = fixture("mir-or-matches");
  writeFileSync(join(dir, "lib.rs"), `pub enum Message {
    Start { first_load: Option<bool> },
    Refresh,
    Other,
}
pub fn starts(message: &Message, out: &mut Vec<u32>) {
    if matches!(message, Message::Start { first_load: Some(true) }) || matches!(message, Message::Refresh) {
        out.push(1);
    }
    out.push(2);
}
pub fn tail(message: &Message, out: &mut Vec<u32>) {
    if matches!(message, Message::Start { first_load: Some(true) }) || matches!(message, Message::Refresh) {
        out.push(1);
    }
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  const test = '  if ((message.TAG === "Start" && message.firstLoad) || message === "Refresh") {\n    out.push(1);\n  }\n';
  expect(js).toContain(`export function starts(message, out) {\n${test}  out.push(2);\n}`);
  expect(js).toContain(`export function tail(message, out) {\n${test}}`);
  const lib = await import(join(dir, "mir.js"));
  const pushed: number[] = [];
  lib.starts({ TAG: "Start", firstLoad: true }, pushed);
  lib.tail("Refresh", pushed);
  lib.tail("Other", pushed);
  expect(pushed).toEqual([1, 2, 1]);
});

// A guard of what a variant binds, `Some(code) if !code.is_empty()` of an
// `as_deref()`: `errorCode`, its truth, as react.dev's error page tests it,
// not `errorCode != null && errorCode` and a `const code = errorCode;`.
test("a guarded binding of an Option's string is its truth", async () => {
  const dir = fixture("guarded-string");
  writeFileSync(join(dir, "lib.rs"), `pub fn title(error_code: Option<String>) -> String {
    match error_code.as_deref() {
        Some(code) if !code.is_empty() => format!("Minified React error #{code}"),
        _ => "Minified Error Decoder".to_string(),
    }
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  // MIR's one value, as react.dev's original writes it.
  expect(js).toContain(
    'export function title(errorCode) {\n  return errorCode ? `Minified React error #${errorCode}` : "Minified Error Decoder";\n}',
  );
  const lib = await import(join(dir, "mir.js"));
  expect([lib.title("1"), lib.title(""), lib.title(undefined)]).toEqual(["Minified React error #1", "Minified Error Decoder", "Minified Error Decoder"]);
});

// Text's truth, `!!code` of `!code.is_empty()`, isn't a presence test: an
// empty string is there. `code ? code.toUpperCase() : undefined` is not
// `code?.toUpperCase()`, which gives `""` where Rust gives `None`; an
// `Option` never falsy's `!!o` is one still (ADR 0298).
test("a test of text's truth is no optional chain", async () => {
  const dir = fixture("text-truth-chain");
  writeFileSync(join(dir, "lib.rs"), `pub fn held(code: Option<&str>) -> Option<String> {
    let s = match code {
        Some(code) if !code.is_empty() => Some(code.to_uppercase()),
        _ => None,
    };
    s
}
pub fn upper(s: &str) -> Option<String> {
    let upper = if !s.is_empty() { Some(s.to_uppercase()) } else { None };
    upper
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).not.toContain("?.toUpperCase()");
  const lib = await import(join(dir, "mir.js"));
  expect([lib.held(""), lib.held("a"), lib.held(undefined), lib.upper(""), lib.upper("b")]).toEqual([undefined, "A", undefined, undefined, "B"]);
});

// ADR 0275: a nullable field given `Some(..)` or `None` by a branch, as
// react.dev's errors page gives `errorMessage`: the branch's `null`, `code
// ? errorCodes[code] : null`, not `(c ? v : undefined) ?? null`.
test("a nullable field of Some or None by a branch is the branch's null", async () => {
  const dir = fixture("nullable-branch");
  writeFileSync(join(dir, "lib.rs"), `pub struct Props {
    #[cfg_attr(rust_js, rust_js::nullable)]
    pub message: Option<String>,
}

pub fn props(code: Option<&str>) -> Props {
    Props {
        message: match code {
            Some(code) if !code.is_empty() => Some(code.to_uppercase()),
            _ => None,
        },
    }
}

pub fn later(code: Option<&str>) -> Props {
    Props {
        message: match code {
            Some(c) if c.is_empty() => None,
            Some(c) => c.strip_prefix("x").map(|s| s.to_string()),
            None => None,
        },
    }
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain("return { message: code ? code.toUpperCase() : null };");
  const lib = await import(join(dir, "mir.js"));
  expect([lib.props("a"), lib.props(""), lib.props(undefined)]).toEqual([{ message: "A" }, { message: null }, { message: null }]);
  expect([lib.later("xa"), lib.later("a"), lib.later(""), lib.later(undefined)]).toEqual([{ message: "a" }, { message: null }, { message: null }, { message: null }]);
});

// A let chain's tests, each read where it's tested once what holds them
// is gone, one `if`: `val != null` of `Array.isArray(val)` none, as
// react.dev's errors page revives its elements.
test("a let chain from MIR is one test, past what held its parts", async () => {
  const dir = fixture("let-chain-tests");
  writeFileSync(join(dir, "lib.rs"), `use js::{Kind, Unknown, classify};

pub fn revive(val: Option<&'static Unknown>) -> Option<&'static Unknown> {
    if let Some(val) = val
        && matches!(classify(val), Kind::Array(_))
        && matches!(js::get(val, &0).map(classify), Some(Kind::String("$r")))
    {
        return js::get(val, &1);
    }
    val
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain('export function revive(val) {\n  if (Array.isArray(val) && val[0] === "$r") {\n    return val[1];\n  }\n  return val;\n}');
  const lib = await import(join(dir, "mir.js"));
  expect([lib.revive(["$r", 1]), lib.revive(["x", 1]), lib.revive(undefined), lib.revive("$r")]).toEqual([1, ["x", 1], undefined, "$r"]);
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
  expect(js).toContain("    if (c.path != null && !(c.skip ?? false)) {\n      return c.path + c.title;\n    }\n");
  const lib = await import(join(dir, "mir.js"));
  expect(lib.crumbs([{ path: "/a", title: "A" }, { path: "/b", skip: true, title: "B" }, { title: "C" }])).toEqual(["/aA", undefined, undefined]);
});

// `ids.iter().position(..)` of an iterator made for it, as react.dev's
// Challenges finds the one the URL names: `$position(ids, ..)`, not of
// `$lent($iter(ids))`, which nothing else steps.
test("a position of an array's own iterator from MIR is of the array", async () => {
  const dir = fixture("position-of-array");
  writeFileSync(join(dir, "lib.rs"), `pub fn second(ids: &[String], path: &str) -> Option<usize> {
    ids.iter().position(|id| Some(id.as_str()) == path.split('#').nth(1))
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain('return $position(ids, (id) => id === path.split("#")[1]);');
  const lib = await import(join(dir, "mir.js"));
  expect([lib.second(["a", "b"], "x#b"), lib.second(["a"], "x")]).toEqual([1, undefined]);
});

// An `||` of `&&`s, `(a && b) || (c && (d || e)) || f`, which MIR jumps
// out of where each holds, is the one test written, as react.dev's Search
// opens on its keys.
test("an or of ands from MIR is one test", async () => {
  const dir = fixture("or-of-ands");
  writeFileSync(join(dir, "lib.rs"), `pub struct Ev {
    pub key_code: u32,
    pub key: String,
    pub meta: bool,
    pub ctrl: bool,
}
fn editing(ev: &Ev) -> bool {
    ev.key == "x"
}
pub fn handle(ev: &Ev, open: bool, out: &mut Vec<u32>) {
    if (ev.key_code == 27 && open) || (ev.key == "k" && (ev.meta || ev.ctrl)) || (!editing(ev) && ev.key == "/" && !open) {
        out.push(1);
        if open {
            out.push(2);
        }
    }
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain('  if (\n    (ev.keyCode === 27 && open) ||\n    (ev.key === "k" && (ev.meta || ev.ctrl)) ||\n    (!editing(ev) && ev.key === "/" && !open)\n  ) {\n    out.push(1);\n');
  const lib = await import(join(dir, "mir.js"));
  const pushed: number[] = [];
  for (const [ev, open] of [[{ keyCode: 27, key: "", meta: false, ctrl: false }, true], [{ keyCode: 0, key: "k", meta: false, ctrl: true }, false], [{ keyCode: 0, key: "k", meta: false, ctrl: false }, false], [{ keyCode: 0, key: "/", meta: false, ctrl: false }, false], [{ keyCode: 0, key: "/", meta: false, ctrl: false }, true]] as const) lib.handle(ev, open, pushed);
  expect(pushed).toEqual([1, 2, 1, 1]);
});

// A `VecDeque`'s iterator from MIR, of the array it is, copied where it's
// collected, `prev.slice()`, as react.dev's Console keeps its latest
// messages.
test("a VecDeque's items collected from MIR are the array's copy", async () => {
  const dir = fixture("deque-collect");
  writeFileSync(join(dir, "lib.rs"), `use std::collections::VecDeque;
pub fn update(f: &dyn Fn(&dyn Fn(&VecDeque<u32>) -> VecDeque<u32>)) {
    f(&|prev| {
        let mut messages: VecDeque<u32> = prev.iter().cloned().collect();
        while messages.len() > 3 {
            messages.pop_front();
        }
        messages
    });
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain("    const messages = prev.slice();\n");
  const lib = await import(join(dir, "mir.js"));
  const prev = [1, 2, 3, 4, 5];
  let got: number[] = [];
  lib.update((f: (p: number[]) => number[]) => {
    got = f(prev);
  });
  expect([got, prev]).toEqual([[3, 4, 5], [1, 2, 3, 4, 5]]);
});

// A chain MIR makes lazily for one spread alone, of stages that do nothing
// else, is an array, which spreads alike, as react.dev's Console takes its
// new logs: `more.filter(..).map(..)`, not `$iter(more)`.
test("a lazy chain from MIR spread once is an array", async () => {
  const dir = fixture("chain-spread-once");
  writeFileSync(join(dir, "lib.rs"), `#[cfg_attr(rust_js, rust_js::link_name = "Math.random")] fn random() -> f64 { unreachable!() }
pub fn joined(prev: &[u32], more: &[u32]) -> Vec<u32> {
    let new_logs = more.iter().filter(|&&x| x > 1).map(|&x| x * 2);
    let messages: Vec<u32> = prev.iter().cloned().chain(new_logs).collect();
    messages
}
pub fn rolled(prev: &[f64], more: &[f64]) -> usize {
    let rolled = more.iter().map(|&x| x + random());
    let all: Vec<f64> = prev.iter().cloned().chain(rolled).collect();
    all.len()
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain("  const newLogs = more.filter((x) => x > 1).map((x) => Math.imul(x, 2) >>> 0);\n");
  // Not of `Math.random()`, which does something: lazy, the array's own
  // iterator.
  expect(js).toContain("  const rolled = more.values().map((x) => x + Math.random());\n");
  const lib = await import(join(dir, "mir.js"));
  expect([lib.joined([1], [1, 2, 3]), lib.rolled([1], [2])]).toEqual([[1, 4, 6], 2]);
});

// A `while` in a closure from MIR, whose test reads what MIR held first,
// `const _7 = messages.length;`, is the `while` written, as react.dev's
// Console keeps its latest messages: not `while (true)` and a `return`;
// incremental too, where the closure's spans are its own.
test("a while in a closure from MIR keeps its test", async () => {
  const dir = fixture("closure-while");
  writeFileSync(join(dir, "lib.rs"), `pub fn listen(on: &dyn Fn(u32) -> Box<dyn Fn(Vec<u32>) -> Vec<u32>>) -> Vec<u32> {
    let f = on(2);
    f(vec![1, 2, 3, 4])
}
pub fn run() -> Vec<u32> {
    listen(&|keep| {
        Box::new(move |prev: Vec<u32>| {
            let mut messages = prev;
            while messages.len() > keep as usize {
                messages.remove(0);
            }
            messages
        })
    })
}
`);
  // Incremental, as an app's build is: spans of a closure's body hold it.
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js"), "--", "-C", `incremental=${join(dir, "incremental")}`]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain("    while (messages.length > keep) {\n      $removeAt(messages, 0);\n    }\n    return messages;\n");
  const lib = await import(join(dir, "mir.js"));
  expect(lib.run()).toEqual([3, 4]);
});

// A temporary MIR makes of what calls, given to a builtin's function,
// `Object.fromEntries(_5)`, is read in place: `Object`'s members never
// change, so reading one first is the same, as react.dev's RSC template
// makes its files an object.
test("a call's temporary from MIR is read in place by a builtin's function", async () => {
  const dir = fixture("builtin-callee-inline");
  writeFileSync(join(dir, "lib.rs"), `use js::dict;
pub fn files(code: String, other: String) -> &'static js::Dict<String> {
    dict::from_entries(vec![
        ("/index.js".to_string(), code.to_uppercase()),
        ("/other.js".to_string(), other),
    ])
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain('  return { "/index.js": code.toUpperCase(), "/other.js": other };\n');
  const lib = await import(join(dir, "mir.js"));
  expect(lib.files("a", "b")).toEqual({ "/index.js": "A", "/other.js": "b" });
});

// ADR 0233: a `match` whose arms each give the field of their variant's
// name of one table is the table read by what's matched, as react.dev's
// ExpandableCallout picks its variant: `MAP[kind]`.
const lib_tags = (lib: { tag: (k: string) => string }) => ["note", "pitfall", "wip"].map((k) => lib.tag(k));
test("a match reading a table from MIR is the table read by its key", async () => {
  const dir = fixture("match-table");
  writeFileSync(join(dir, "lib.rs"), `#[derive(Clone, Copy)]
pub enum Kind {
    #[cfg_attr(rust_js, rust_js::name = "note")]
    Note,
    #[cfg_attr(rust_js, rust_js::name = "pitfall")]
    Pitfall,
    #[cfg_attr(rust_js, rust_js::name = "wip")]
    Wip,
}
pub struct Map {
    pub note: &'static str,
    pub pitfall: &'static str,
    pub wip: &'static str,
}
static MAP: Map = Map { note: "n", pitfall: "p", wip: "w" };
pub fn label(kind: Kind) -> String {
    let variant = match kind {
        Kind::Note => &MAP.note,
        Kind::Pitfall => &MAP.pitfall,
        Kind::Wip => &MAP.wip,
    };
    format!("{variant}!")
}
pub fn name(kind: Kind) -> &'static str {
    match kind {
        Kind::Note => "note",
        Kind::Pitfall => "pitfall",
        Kind::Wip => "wip",
    }
}
pub fn mixed(kind: Kind) -> &'static str {
    match kind {
        Kind::Note => MAP.note,
        Kind::Pitfall => MAP.pitfall,
        Kind::Wip => "wip",
    }
}
pub struct Look {
    pub name: &'static str,
    pub classes: &'static str,
}
pub struct Looks {
    pub note: Look,
    pub pitfall: Look,
    pub wip: Look,
}
static LOOKS: Looks = Looks {
    note: Look { name: "Note", classes: "green" },
    pitfall: Look { name: "Pitfall", classes: "yellow" },
    wip: Look { name: "WIP", classes: "gray" },
};
pub fn tag(kind: Kind) -> String {
    let Look { name, classes } = match kind {
        Kind::Note => &LOOKS.note,
        Kind::Pitfall => &LOOKS.pitfall,
        Kind::Wip => &LOOKS.wip,
    };
    format!("{name} {classes}")
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain("  const variant = MAP[kind];\n");
  // Taken apart, as react.dev's Tag takes its variant's name and classes.
  expect(js).toContain("  const { name, classes } = LOOKS[kind];\n");
  expect(lib_tags(await import(join(dir, "mir.js")))).toEqual(["Note green", "Pitfall yellow", "WIP gray"]);
  expect(js).toContain("export function name(kind) {\n  return kind;\n}");
  const lib = await import(join(dir, "mir.js"));
  expect(["note", "pitfall", "wip"].map((k) => lib.label(k))).toEqual(["n!", "p!", "w!"]);
  expect(["note", "pitfall", "wip"].map((k) => [lib.name(k), lib.mixed(k)])).toEqual([["note", "n"], ["pitfall", "p"], ["wip", "wip"]]);
});

// A consumer that stops early, `position`, `any`, `find`, `all`, of an
// iterator stepped again after: it takes only what Rust's takes, and what's
// left is still there for `next()`, as it is in Rust.
test("an early-stopping consumer leaves the rest of the iterator", async () => {
  const dir = fixture("early-stop-rest");
  writeFileSync(join(dir, "lib.rs"), `pub fn after(ids: &[u32]) -> Vec<Option<u32>> {
    let mut out = Vec::new();
    let mut it = ids.iter();
    out.push(it.position(|&id| id == 2).map(|i| i as u32));
    out.push(it.next().copied());
    let mut it = ids.iter();
    out.push(if it.any(|&id| id == 2) { Some(1) } else { None });
    out.push(it.next().copied());
    let mut it = ids.iter();
    out.push(it.find(|&&id| id == 2).copied());
    out.push(it.next().copied());
    let mut it = ids.iter();
    out.push(if it.all(|&id| id != 2) { Some(1) } else { None });
    out.push(it.next().copied());
    let mut it = ids.iter();
    out.push(it.find_map(|&id| (id == 2).then_some(id * 10)));
    out.push(it.next().copied());
    let mut it = ids.iter();
    out.push(it.nth(1).copied());
    out.push(it.next().copied());
    let mut it = ids.iter().peekable();
    out.push(it.peek().map(|&&id| id));
    out.push(it.position(|&id| id == 2).map(|i| i as u32));
    out.push(it.next().copied());
    out
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const lib = await import(join(dir, "mir.js"));
  expect(lib.after([1, 2, 3])).toEqual([1, 3, 1, 3, 2, 3, undefined, 3, 20, 3, 2, 3, 1, 1, 3]);
});

// ADR 0298: `is_some()` of an `Option` never falsy, `!!o`, given as a
// value, a `bool`'s: `!!o && f()` is `false` where it's `None`, never the
// `undefined` of `o && f()`, as react.dev's DownloadButton keeps whether
// import maps are supported.
test("a truthy option's is_some given as a value is a boolean", async () => {
  const dir = fixture("is-some-value");
  writeFileSync(join(dir, "lib.rs"), `use std::cell::Cell;
use js::JsObject;
#[cfg_attr(rust_js, rust_js::link_name = "get globalThis.maybeSupports")]
fn scriptSupports() -> Option<&'static JsObject> {
    unreachable!()
}

#[cfg_attr(rust_js, rust_js::link_name = "globalThis.supportsIt")]
#[allow(unused_variables)]
fn supports(what: &str) -> bool {
    unreachable!()
}
thread_local! {
    static SUPPORTED: Cell<bool> = const { Cell::new(false) };
}
pub fn update() -> bool {
    SUPPORTED.set(scriptSupports().is_some() && supports("importmap"));
    SUPPORTED.get()
}
pub fn negated() -> bool {
    if scriptSupports().is_none() { false } else { true }
}
pub fn or_none() -> bool {
    if scriptSupports().is_none() { supports("x") } else { true }
}
`);
  const globals = globalThis as typeof globalThis & { maybeSupports?: object; supportsIt?: (what: string) => boolean };
  globals.supportsIt = () => true;
  try {
    run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
    const js = readFileSync(join(dir, "mir.js"), "utf8");
    expect(js).toContain('  SUPPORTED = !!globalThis.maybeSupports && globalThis.supportsIt("importmap");\n');
    const lib = await import(join(dir, "mir.js"));
    delete globals.maybeSupports;
    const absent = [lib.update(), lib.negated(), lib.orNone()];
    globals.maybeSupports = {};
    expect([absent, [lib.update(), lib.negated(), lib.orNone()]]).toEqual([[false, false, true], [true, true, true]]);
  } finally {
    delete globals.maybeSupports;
    delete globals.supportsIt;
  }
});

// A consumer of an array's own iterator, read in place only once MIR's
// temporary of it is, after an early return, `any(f)`: the array's `some`,
// as react.dev's DownloadButton checks its files.
test("an array's iterator read in place late from MIR is the array's method", async () => {
  const dir = fixture("late-eager-chain");
  writeFileSync(join(dir, "lib.rs"), `pub const SUPPORTED: [&str; 2] = ["/App.js", "/styles.css"];
pub struct Props {
    pub provided_files: Vec<String>,
}
pub fn button(Props { provided_files }: Props, supported: bool) -> Option<u32> {
    if !supported {
        return None;
    }
    if provided_files.iter().any(|file| !SUPPORTED.contains(&file.as_str())) {
        return None;
    }
    let n = provided_files.len() as u32;
    Some(n)
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain("  if (providedFiles.some((file) => !SUPPORTED.includes(file))) {\n");
  const lib = await import(join(dir, "mir.js"));
  expect([lib.button({ providedFiles: ["/App.js"] }, true), lib.button({ providedFiles: ["/x.js"] }, true), lib.button({ providedFiles: [] }, false)]).toEqual([1, undefined, undefined]);
});

// As react.dev's createFileMap reads a code block's meta:
// `!matches!(t, "pre")` is `t !== "pre"`; a string method's `bool`,
// `includes`, is one already, no `!!`; and `any(|t| t == "hidden")` of
// strings is `includes("hidden")`.
test("negated matches, string methods and any of equals from MIR read as written", async () => {
  const dir = fixture("file-map-shapes");
  writeFileSync(join(dir, "lib.rs"), `pub fn not_pre(t: &str, u: &str, out: &mut Vec<u32>) {
    if !matches!(t, "pre") && !matches!(u, "pre") {
        out.push(1);
    }
}
pub fn found(tokens: &[String]) -> Option<&String> {
    tokens.iter().find(|token| token.contains('/') || token.contains('.'))
}
pub fn hidden(tokens: &[String]) -> bool {
    tokens.iter().any(|token| token == "hidden")
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain('  if (t !== "pre" && u !== "pre") {\n');
  expect(js).toContain('  return tokens.find((token) => token.includes("/") || token.includes("."));\n');
  expect(js).toContain('  return tokens.includes("hidden");\n');
  const lib = await import(join(dir, "mir.js"));
  const pushed: number[] = [];
  lib.notPre("a", "b", pushed);
  lib.notPre("pre", "b", pushed);
  expect([pushed, lib.found(["a", "b.js"]), lib.found(["a"]), lib.hidden(["hidden"]), lib.hidden(["x"])]).toEqual([[1], "b.js", undefined, true, false]);
});

// `count()` of an array's items, a `split`'s: its `length`, not a `reduce`
// counting each, as react.dev's CustomPreset counts a file's lines.
test("a count of an array's items from MIR is its length", async () => {
  const dir = fixture("count-length");
  writeFileSync(join(dir, "lib.rs"), `pub fn lines(code: &str) -> usize {
    code.split('\\n').count()
}
pub fn long(words: &[String]) -> usize {
    words.iter().filter(|w| w.len() > 2).count()
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain('  return code.split("\\n").length;\n');
  expect(js).toContain("  return words.filter((w) => $byteLen(w) > 2).length;\n");
  expect(js).toContain("  return words.filter((w) => $byteLen(w) > 2).length;\n");
  const lib = await import(join(dir, "mir.js"));
  expect([lib.lines("a\nb\nc"), lib.long(["a", "abc", "abcd"])]).toEqual([3, 2]);
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

// A `Cell` field a `let` pattern binds by reference, only read, is its
// value where it's bound (ADR 0287), as react.dev's ErrorMessage reads its
// error's title: `const { message, title } = error`.
test("a cell field a let pattern binds, only read, is its value", async () => {
  const dir = fixture("pattern-cell-field");
  writeFileSync(join(dir, "lib.rs"), `use std::cell::Cell;
pub struct E {
    pub title: Cell<Option<&'static str>>,
    pub message: String,
}
pub fn show(error: &E) -> String {
    let E { message, title } = error;
    format!("{}: {message}", title.get().filter(|t| !t.is_empty()).unwrap_or("Error"))
}
pub fn retitle(error: &E) -> String {
    let E { message, title } = error;
    title.set(Some("New"));
    format!("{}: {message}", title.get().unwrap_or("Error"))
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain("export function show(error) {\n  const { message, title } = error;\n");
  const lib = await import(join(dir, "mir.js"));
  const error = { title: "T", message: "m" };
  expect([lib.show(error), lib.show({ message: "m" }), lib.retitle(error), error.title]).toEqual(["T: m", "Error: m", "New: m", "New"]);
});

// `flatten()` of `Option` items, a JS iterator of what's there, collected:
// the array's `filter`, not `Iterator.from(..)` of `$iter`'s chain, as a
// class name's parts are joined.
test("flattened options collected from MIR are the array's filter", async () => {
  const dir = fixture("flatten-collect");
  writeFileSync(join(dir, "lib.rs"), `pub fn cn(parts: &[Option<&str>]) -> String {
    parts.iter().flatten().copied().collect::<Vec<_>>().join(" ")
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain('  return parts.filter((item) => item != null).join(" ");\n');
  const lib = await import(join(dir, "mir.js"));
  expect(lib.cn(["a", undefined, "b"])).toBe("a b");
});

// Returns the program wrote, which MIR's lowering makes assignments of the
// result and jumps out of a block to its one `return`, from a loop's too:
// each a `return` where it's written, as react.dev's getRouteMeta finds a
// route's breadcrumbs.
test("written returns MIR jumps out of a block for are returns where they're written", async () => {
  const dir = fixture("block-returns");
  writeFileSync(join(dir, "lib.rs"), `pub struct Route {
    pub path: Option<String>,
    pub routes: Option<Vec<Route>>,
}
pub fn crumbs<'a>(path: &str, current: &'a Route, crumbs_so_far: Vec<&'a Route>) -> Vec<&'a Route> {
    if current.path.as_deref() == Some(path) {
        return crumbs_so_far;
    }
    let Some(routes) = &current.routes else {
        return Vec::new();
    };
    for route in routes {
        let child = crumbs(path, route, [crumbs_so_far.as_slice(), &[current]].concat());
        if !child.is_empty() {
            return child;
        }
    }
    Vec::new()
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain("    if (child.length !== 0) {\n      return child;\n    }\n  }\n  return [];\n}");
  const lib = await import(join(dir, "mir.js"));
  const tree = { path: "/", routes: [{ path: "/a", routes: [{ path: "/a/b" }] }] };
  expect([lib.crumbs("/a/b", tree, []).map((r: { path: string }) => r.path), lib.crumbs("/x", tree, [])]).toEqual([["/", "/a"], []]);
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

// ADR 0264: a function giving each variant its own name gives what it's
// given: `section.as_str()` is `section`, as react.dev's Page names its
// section's image.
test("a call giving each variant its own name from MIR is what it's given", async () => {
  const dir = fixture("own-name-call");
  writeFileSync(join(dir, "lib.rs"), `#[derive(Clone, Copy)]
pub enum Section {
    #[cfg_attr(rust_js, rust_js::name = "learn")]
    Learn,
    #[cfg_attr(rust_js, rust_js::name = "unknown")]
    Unknown,
}
impl Section {
    pub fn as_str(self) -> &'static str {
        match self {
            Section::Learn => "learn",
            Section::Unknown => "unknown",
        }
    }
}
pub fn image(section: Option<Section>) -> String {
    format!("/images/og-{}.png", section.unwrap_or(Section::Unknown).as_str())
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain('  return `/images/og-${section ?? "unknown"}.png`;\n');
  const lib = await import(join(dir, "mir.js"));
  expect([lib.image("learn"), lib.image(undefined)]).toEqual(["/images/og-learn.png", "/images/og-unknown.png"]);
});

// An `if` of an `||`, its branches as the program wrote them: `home ||
// !meta.title ? a : b`, not `!home && meta.title ? b : a`, as react.dev's
// Page picks its image.
test("an if of an or from MIR keeps its branches in the order written", async () => {
  const dir = fixture("or-branch-order");
  writeFileSync(join(dir, "lib.rs"), `pub struct Meta {
    pub title: &'static str,
    pub path: &'static str,
}
pub fn image(home: bool, meta: &Meta) -> String {
    let image = if home || meta.title.is_empty() {
        format!("/images/og-{}.png", meta.path)
    } else {
        format!("/images/og/{}.png", meta.path.replace('/', "-"))
    };
    image
}
// A test's truth, not two values: as written.
pub fn flagged(heading: bool, crumb: bool, expanded: bool) -> Option<&'static str> {
    (!heading && crumb && !expanded).then_some("on")
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain("    home || !meta.title\n      ? `/images/og-${meta.path}.png`\n");
  expect(js).toContain('  return !heading && crumb && !expanded ? "on" : undefined;\n');
  const lib = await import(join(dir, "mir.js"));
  expect([lib.image(true, { title: "t", path: "/a" }), lib.image(false, { title: "t", path: "/a/b" }), lib.flagged(false, true, false)]).toEqual(["/images/og-/a.png", "/images/og/-a-b.png", "on"]);
});

// A chain read past the alias MIR makes of what its callback borrows
// mutably, `const list$1 = list;`: reading a variable nothing sets again is
// the same before the chain is made or after, so it's the array's own
// `forEach`, as react.dev's toCommaSeparatedList steps its items.
test("a chain is read in place past a closure's mutable borrow", async () => {
  const dir = fixture("chain-past-borrow");
  writeFileSync(join(dir, "lib.rs"), `pub fn list(array: &[String]) -> Vec<String> {
    let mut list: Vec<String> = Vec::new();
    array.iter().enumerate().for_each(|(index, item)| {
        if index > 0 {
            list.push(format!("{index}{item}"));
        }
    });
    list
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain("  array.forEach((item, index) => {\n");
  const lib = await import(join(dir, "mir.js"));
  expect(lib.list(["a", "b", "c"])).toEqual(["1b", "2c"]);
});

// An element's place checked then set, `args[0] = v`, from MIR is
// `args[$at(args, 0)] = v`, its index read in place where nothing changes
// it, as react.dev's ErrorDecoder sets its first argument. What's set is
// made before the check, as Rust makes it.
test("a checked element set is one statement from MIR", async () => {
  const dir = fixture("checked-set");
  writeFileSync(join(dir, "lib.rs"), `pub fn fill(args: &mut Vec<String>) {
    if args.len() == 1 && args[0].is_empty() {
        args[0] = "HTML".to_string();
    }
}
pub fn put(args: &mut Vec<String>, i: usize, made: impl Fn() -> String) {
    args[i] = made();
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  // One the length's test shows in bounds is set as it is (ADR 0576).
  expect(js).toContain('    args[0] = "HTML";\n');
  expect(js).toMatch(/  args\[\$at\(args, i\)\] = \w+;\n}/);
  const lib = await import(join(dir, "mir.js"));
  const args = [""];
  lib.fill(args);
  const made: string[] = [];
  expect(() => lib.put(args, 1, () => (made.push("x"), "x"))).toThrow();
  expect([args, made]).toEqual([["HTML"], ["x"]]);
});

// A clone of a nullable field's `Option`, `m.rule_id.clone()`, is its JS
// value, null or the string, so given to another nullable field it's read
// as it is, as react.dev's runESLint gives each diagnostic its rule.
test("a nullable field's clone is given to a nullable field as it is", async () => {
  const dir = fixture("nullable-clone");
  writeFileSync(join(dir, "lib.rs"), `pub struct Msg {
    #[cfg_attr(rust_js, rust_js::nullable)]
    pub rule_id: Option<String>,
    pub line: u32,
}
pub struct Diag {
    #[cfg_attr(rust_js, rust_js::nullable)]
    pub rule_id: Option<String>,
    pub line: u32,
}
pub fn diags(msgs: Vec<&Msg>) -> Vec<Diag> {
    msgs.iter().map(|m| Diag { rule_id: m.rule_id.clone(), line: m.line }).collect()
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain("({ ruleId: m.ruleId, line: m.line })");
  const lib = await import(join(dir, "mir.js"));
  expect(lib.diags([{ ruleId: null, line: 1 }, { ruleId: "r", line: 2 }])).toEqual([{ ruleId: null, line: 1 }, { ruleId: "r", line: 2 }]);
});

// A trait's default method that's a binding, `get ruleId`, called through
// a generic's bound, `error.rule_id()` of `E: RuleIdentified`: the
// binding of the value, `error.ruleId`, as react.dev's useSandpackLint
// keeps the errors of a rule.
test("a binding trait method called through a generic is the binding", async () => {
  const dir = fixture("generic-binding-method");
  writeFileSync(join(dir, "lib.rs"), `pub struct LintError {
    pub line: u32,
}
pub struct Diagnostic {
    pub from: f64,
}
trait RuleIdentified {
    #[cfg_attr(rust_js, rust_js::link_name = "get ruleId")]
    fn rule_id(&self) -> &Option<String> {
        unreachable!()
    }
}
impl RuleIdentified for LintError {}
impl RuleIdentified for Diagnostic {}
pub fn kept(errors: Vec<&'static LintError>, diags: Vec<&'static Diagnostic>) -> (usize, Vec<&'static Diagnostic>) {
    fn isReactRuleError<E: RuleIdentified>(error: &E) -> bool {
        error.rule_id().is_some()
    }
    let n = errors.into_iter().filter(|e| isReactRuleError(*e)).count();
    (n, diags.into_iter().filter(|d| isReactRuleError(*d)).collect())
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain("return error.ruleId != null;");
  const lib = await import(join(dir, "mir.js"));
  expect(lib.kept([{ line: 1, ruleId: "a" }, { line: 2, ruleId: null }], [{ from: 1, ruleId: null }, { from: 2, ruleId: "b" }])).toEqual([1, [{ from: 2, ruleId: "b" }]]);
});

// ADR 0250: a struct update whose every field is given, `Hit { url, ..*item
// }`, still spreads its base, though it reads none of its fields: what JS
// gave holds more than Rust names, react.dev's Search gives DocSearch its
// hits' hierarchy back.
test("a struct update giving every field spreads its base from MIR", async () => {
  const dir = fixture("update-all-given");
  writeFileSync(join(dir, "lib.rs"), `pub struct Hit {
    pub url: String,
}
pub fn cleaned(items: Vec<&'static Hit>) -> Vec<Hit> {
    items.into_iter().map(|item| Hit { url: item.url.replace("#x", ""), ..*item }).collect()
}
pub fn moved(hit: Hit) -> Hit {
    Hit { url: hit.url.replace("#x", ""), ..hit }
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain('({ ...item, url: item.url.replaceAll("#x", "") })');
  expect(js).toContain('return { ...hit, url: hit.url.replaceAll("#x", "") };');
  const lib = await import(join(dir, "mir.js"));
  expect([lib.cleaned([{ url: "a#x", level: 1 }]), lib.moved({ url: "b#x", level: 2 })]).toEqual([[{ url: "a", level: 1 }], { url: "b", level: 2 }]);
});

// ADR 0250: a struct update that reads one field of its base, `Props {
// title, ..props }`, spreads it, as its THIR says it's an update: not
// `{ title, level: props.level }`, which drops what JS's object held
// besides: react.dev's SandpackWithHTMLOutput gives on its props. One that
// names each field, `level: a.level`, isn't.
test("a struct update reading one field of its base spreads it from MIR", async () => {
  const dir = fixture("update-one-read");
  writeFileSync(join(dir, "lib.rs"), `pub struct Props {
    pub title: String,
    pub level: u32,
}
pub fn retitled(props: Props) -> Props {
    Props { title: "x".to_string(), ..props }
}
pub fn copied(a: &Props) -> Props {
    Props { title: "y".to_string(), level: a.level }
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain('return { ...props, title: "x" };');
  expect(js).toContain('return { title: "y", level: a.level };');
  const lib = await import(join(dir, "mir.js"));
  expect(lib.retitled({ title: "a", level: 1, extra: 2 })).toEqual({ title: "x", level: 1, extra: 2 });
});

// An `||` chain, `a || b || c`, not `!(!a && !b) || c`: a `bool` field,
// `element.editable`, is one, as react.dev's Search, Preview and
// useTocHighlight test theirs.
test("an or chain from MIR is written as one", async () => {
  const dir = fixture("or-chain");
  writeFileSync(join(dir, "lib.rs"), `pub struct El {
    pub editable: bool,
    pub tag: String,
}
pub fn editing(element: &El) -> bool {
    let tag_name = &element.tag;
    element.editable || tag_name == "INPUT" || tag_name == "SELECT" || tag_name == "TEXTAREA"
}
pub fn hidden(error: Option<u32>, height: Option<f64>, ready: bool) -> bool {
    let hide = error.is_some() || height.is_none_or(|h| h == 0.0) || !ready;
    hide
}
pub fn heading(names: Vec<String>) -> Vec<String> {
    names.into_iter().filter(|n| n.as_str() == "H1" || n.as_str() == "H2" || n.as_str() == "H3").collect()
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain('return element.editable || tagName === "INPUT" || tagName === "SELECT" || tagName === "TEXTAREA";');
  expect(js).toContain("const hide = error != null || height == null || height === 0 || !ready;");
  expect(js).toContain('names.filter((n) => n === "H1" || n === "H2" || n === "H3")');
  const lib = await import(join(dir, "mir.js"));
  expect([
    lib.editing({ editable: false, tag: "SELECT" }),
    lib.editing({ editable: false, tag: "B" }),
    lib.hidden(undefined, 3, true),
    lib.hidden(undefined, 0, true),
    lib.heading(["H2", "P"]),
  ]).toEqual([true, false, false, true, ["H2"]]);
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

// `next()` of an array's iterator that nothing else steps, a split's,
// `path.split(..).next()`, is its first item, `xs[0]`, as react.dev's
// PageHeading and NavigationBar take theirs.
test("next of a fresh iterator of an array is its first item", async () => {
  const dir = fixture("next-first");
  writeFileSync(join(dir, "lib.rs"), `pub fn clean(path: &str) -> &str {
    path.split(['?', '#']).next().unwrap_or("")
}
pub fn first(keys: Vec<String>) -> Option<String> {
    keys.into_iter().next()
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain('return $splitBy(path, (c) => ["?", "#"].includes(c))[0] ?? "";');
  expect(js).toContain("return keys[0];");
  const lib = await import(join(dir, "mir.js"));
  expect([lib.clean("/a?b"), lib.clean(""), lib.first([]), lib.first(["k"])]).toEqual(["/a", "", undefined, "k"]);
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

// An `if let .. else if let .. else { panic!() }` chain is its arms in
// the order written, not the panic's first in a labeled block, as
// react.dev's TerminalBlock reads its message.
test("an if-let chain ending in a panic keeps its arms in written order", async () => {
  const dir = fixture("if-let-chain-panic");
  writeFileSync(join(dir, "lib.rs"), `pub enum Kind {
    Text(String),
    Element(Option<String>),
    Other,
}
pub fn message(kind: Kind) -> String {
    let message: String;
    if let Kind::Text(text) = &kind {
        message = text.to_string();
    } else if let Kind::Element(child) = &kind
        && let Some(text) = child
    {
        message = text.to_string();
    } else {
        panic!("Expected a plain string.");
    }
    message
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain('  if (kind.TAG === "Text") {\n    message = kind._0;\n  } else if (kind.TAG === "Element" && kind._0 != null) {\n    message = kind._0;\n  } else {\n    throw');
  const lib = await import(join(dir, "mir.js"));
  expect([lib.message({ TAG: "Text", _0: "a" }), lib.message({ TAG: "Element", _0: "b" })]).toEqual(["a", "b"]);
  expect(() => lib.message({ TAG: "Element", _0: undefined })).toThrow("plain string");
});

// `x != null && typeof x === "string"`, an `Option`'s test before a
// classification's: the `typeof` test, which holds of no `null`, as
// react.dev's TerminalBlock reads its children's text from MIR.
test("a typeof test after a null check is the typeof test from MIR", async () => {
  const dir = fixture("typeof-after-null");
  writeFileSync(join(dir, "lib.rs"), `pub fn text(ok: bool, given: Option<&'static js::Unknown>) -> &'static str {
    if ok
        && let Some(t) = given
        && let js::Kind::String(s) = js::classify(t)
    {
        s
    } else {
        "none"
    }
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target], 600_000);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain('return ok && typeof given === "string" ? given : "none";');
  const lib = await import(join(dir, "mir.js"));
  expect([lib.text(true, "a"), lib.text(true, undefined), lib.text(true, 3), lib.text(false, "a")]).toEqual(["a", "none", "none", "none"]);
});

// An `||` chain whose part is an `Option`'s `is_some()`, of an object:
// `!!error || ..`, a `bool` part by part, not `!(!error && ..)`; and one
// only tested, `error || ..`, as react.dev's Preview hides its content.
test("an or chain of a value's truth from MIR is a bool part by part", async () => {
  const dir = fixture("or-chain-truth");
  writeFileSync(join(dir, "lib.rs"), `pub struct Err {
    pub title: String,
}
pub fn hidden(error: Option<&Err>, height: Option<f64>, ready: bool) -> bool {
    error.is_some() || height.is_none_or(|h| h == 0.0) || !ready
}
pub fn kept(error: Option<&Err>, height: Option<f64>, ready: bool) -> String {
    let hide = error.is_some() || height.is_none_or(|h| h == 0.0) || !ready;
    if hide { "a".to_string() } else { "b".to_string() }
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain("return !!error || height == null || height === 0 || !ready;");
  expect(js).toContain("const hide = error || height == null || height === 0 || !ready;");
  const lib = await import(join(dir, "mir.js"));
  expect([lib.hidden({ title: "t" }, 3, true), lib.hidden(undefined, 3, true), lib.kept({ title: "t" }, 3, true), lib.kept(undefined, 3, true)]).toEqual([true, false, "a", "b"]);
});

// An `else if` chain's last arm before a panic is an `else if`, not a
// check in the `else`, `if (x !== "language-css") throw`, as react.dev's
// createFileMap picks a code block's file.
test("an else-if chain's arm before a panic stays an else if", async () => {
  const dir = fixture("else-if-panic");
  writeFileSync(join(dir, "lib.rs"), `pub fn path(meta: Option<&str>, class_name: &str) -> String {
    let file_path;
    if let Some(meta) = meta {
        file_path = format!("/{meta}");
    } else if class_name == "language-js" {
        file_path = "/App.js".to_string();
    } else if class_name == "language-css" {
        file_path = "/styles.css".to_string();
    } else {
        panic!("Code block is missing a filename: {class_name}");
    }
    file_path
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain('  } else if (className === "language-css") {\n    filePath = "/styles.css";\n  } else {\n    throw');
  const lib = await import(join(dir, "mir.js"));
  expect([lib.path(undefined, "language-css"), lib.path("a.js", "x")]).toEqual(["/styles.css", "/a.js"]);
  expect(() => lib.path(undefined, "x")).toThrow("missing a filename");
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

// An `||` chain whose first test is a getter's `bool`, `element.isContentEditable`,
// is one chain returned, as react.dev's Search tells whether a key goes to
// what's being edited.
test("an or chain of a getter's bool is returned as one", async () => {
  const dir = fixture("getter-or-chain");
  writeFileSync(join(dir, "lib.rs"), `use react::webapi::HTMLElement;
pub fn editing(element: &HTMLElement) -> bool {
    let tag_name = element.tag_name();
    element.is_content_editable() || tag_name == "INPUT" || tag_name == "SELECT" || tag_name == "TEXTAREA"
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js"), "--", "--extern", `react=${join(target, "libreact.rmeta")}`, "-L", target]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain("  return (\n    element.isContentEditable ||\n");
  const lib = await import(join(dir, "mir.js"));
  expect([lib.editing({ isContentEditable: false, tagName: "SELECT" }), lib.editing({ isContentEditable: false, tagName: "B" })]).toEqual([true, false]);
});

// A call of a function that gives a `bool`, `isItem(item)`, is one, so
// it's no `!!isItem(item)` in a test, as react.dev's CodeDiagram keeps its
// images.
test("a call giving a bool is one in a test", async () => {
  const dir = fixture("bool-call");
  writeFileSync(join(dir, "lib.rs"), `pub struct Item {
    pub kind: String,
}
#[cfg_attr(rust_js, rust_js::link_name = "globalThis.isItem")]
#[allow(unused_variables)]
fn is_item(x: &Item) -> bool {
    unreachable!()
}
pub fn images(items: Vec<&Item>) -> Vec<&Item> {
    items.into_iter().filter(|item| is_item(item) && item.kind == "img").collect()
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain('items.filter((item) => globalThis.isItem(item) && item.kind === "img")');
  const lib = await import(join(dir, "mir.js"));
  (globalThis as any).isItem = (x: { kind: string }) => x.kind !== "";
  expect(lib.images([{ kind: "img" }, { kind: "p" }, { kind: "" }])).toEqual([{ kind: "img" }]);
});

// ADR 0279, of a function item given as the callback, `filter(is_rule)`, as
// react.dev's useSandpackLint has it: the crate's function itself; a JS
// function's still takes the item alone.
test("a crate function given as a callback is passed as itself", async () => {
  const dir = fixture("fn-item-callback");
  writeFileSync(join(dir, "lib.rs"), `#[cfg_attr(rust_js, rust_js::link_name = "parseFloat")] #[allow(unused_variables)] fn parse_float(text: &str) -> f64 { unreachable!() }
pub struct E {
    pub rule: Option<String>,
}
fn is_rule(e: &&E) -> bool {
    e.rule.is_some()
}
fn twice(x: i32) -> i32 {
    x * 2
}
pub fn kept(errors: Vec<&E>) -> usize {
    errors.into_iter().filter(is_rule).count()
}
pub fn doubled(v: Vec<i32>) -> Vec<i32> {
    v.into_iter().map(twice).collect()
}
pub fn parsed(v: Vec<&str>) -> Vec<f64> {
    v.into_iter().map(parse_float).collect()
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain(".filter(isRule)");
  expect(js).toContain("v.map(twice)");
  expect(js).toContain("v.map((text) => parseFloat(text))");
  const lib = await import(join(dir, "mir.js"));
  expect(lib.kept([{ rule: "a" }, { rule: undefined }])).toBe(1);
  expect(lib.doubled([1, 2])).toEqual([2, 4]);
  expect(lib.parsed(["1", "2"])).toEqual([1, 2]);
});

// ADR 0308: a function written in a body, a closure's too, is declared
// there, before what's written after it, as react.dev's useSandpackLint has
// `isReactRuleError` in its linter's callback.
test("a function written in a closure is declared where it's written", async () => {
  const dir = fixture("written-fn-place");
  writeFileSync(join(dir, "lib.rs"), `pub fn counted(xs: Vec<i32>, m: i32) -> usize {
    let k = m + 1;
    fn small(x: i32) -> bool {
        x < 0
    }
    let f = move || {
        let n = xs.len();
        fn big(x: &&i32) -> bool {
            **x > 1
        }
        xs.iter().filter(big).count() + n
    };
    if small(k) { 0 } else { f() }
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toMatch(/const k = .*\n  function small\(x\)/);
  expect(js).toMatch(/const n = xs\.length;\n    function big\(x\)/);
  const lib = await import(join(dir, "mir.js"));
  expect([lib.counted([1, 2, 3], 0), lib.counted([1, 2, 3], -2)]).toEqual([5, 0]);
});

// An `async` closure's `|_|`, which takes nothing it reads, takes nothing
// in JS either, `async () => ..`, as react.dev's PageHeading has its copy
// button's handler.
test("an async closure's ignored argument is no parameter", async () => {
  const dir = fixture("async-wild");
  writeFileSync(join(dir, "lib.rs"), `pub async fn run(f: impl AsyncFn(u32) -> u32) -> u32 {
    f(2).await
}
pub async fn made() -> u32 {
    run(async move |_| 1).await + run(async move |x| x).await
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain("run(async () => 1)");
  const lib = await import(join(dir, "mir.js"));
  expect(await lib.made()).toBe(3);
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

// Arrays joined, `[first, vec![..]].concat()`, are one array of the first's
// items and the made ones, `[...first, made("a"), ..]`: a call can't change
// the items of an array only a `&mut` changes, as react.dev's
// SandpackWithHTMLOutput adds its files to its children.
test("arrays joined are one array of their items", async () => {
  const dir = fixture("concat-spread");
  writeFileSync(join(dir, "lib.rs"), `use js::Unknown;
#[cfg_attr(rust_js, rust_js::link_name = "globalThis.made")]
#[allow(unused_variables)]
fn made(name: &str) -> &'static Unknown {
    unreachable!()
}
pub fn all(first: Vec<&'static Unknown>) -> Vec<&'static Unknown> {
    let children = [first, vec![made("a"), made("b")]].concat();
    children
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain('const children = [...first, globalThis.made("a"), globalThis.made("b")];');
  const lib = await import(join(dir, "mir.js"));
  (globalThis as any).made = (name: string) => ({ name });
  expect(lib.all([{ name: "x" }])).toEqual([{ name: "x" }, { name: "a" }, { name: "b" }]);
});

// A loop's `if`s the program ends with `continue`,
// `if c == "{" { depth += 1; continue; }`, are written so, each then what
// follows, not an `else if` chain, as react.dev's createFileMap splits a
// meta string.
test("continues a loop's ifs end with are kept", async () => {
  const dir = fixture("ending-continues");
  writeFileSync(join(dir, "lib.rs"), `pub fn split(meta: &str) -> Vec<String> {
    let mut tokens: Vec<String> = Vec::new();
    let mut current = String::new();
    let mut depth: i32 = 0;
    for c in meta.chars() {
        let c = c.to_string();
        if c == "{" {
            depth += 1;
            continue;
        }
        if c == "}" {
            depth -= 1;
            continue;
        }
        if depth > 0 {
            continue;
        }
        if c == " " {
            if !current.is_empty() {
                tokens.push(current);
                current = String::new();
            }
            continue;
        }
        current += &c;
    }
    tokens
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain('      depth = (depth + 1) | 0;\n      continue;\n    }\n    if (c === "}") {');
  expect(js).toContain("    if (depth > 0) {\n      continue;\n    }");
  expect(js).not.toContain("else");
  const lib = await import(join(dir, "mir.js"));
  expect(lib.split("a {b c} d e")).toEqual(["a", "d"]);
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

// A value the program wrote with `&&`, `s != "en" && (if t { .. } else {
// .. })`, is one value returned, `s !== "en" && (t ? .. : ..)`, not a
// `return` of each branch MIR makes of it, as react.dev's MDXComponents
// filters its LanguageList; a tail `if` of two values is one conditional.
test("a value written with && is returned as one", async () => {
  const dir = fixture("logical-values");
  writeFileSync(join(dir, "lib.rs"), `pub fn a(t: bool, x: u32) -> u32 {
    if t { x + 1 } else { x * 2 }
}
pub fn c(t: bool, u: bool, s: &str) -> bool {
    s != "en" && (if t { s.is_empty() } else { u })
}
pub fn kept(all: &[String], complete: bool, done: &[&str]) -> usize {
    all.iter()
        .filter(|code| *code != "en" && (if complete { done.contains(&code.as_str()) } else { !done.contains(&code.as_str()) }))
        .count()
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain('return s !== "en" && (t ? !s : u);');
  expect(js).toContain('(code) => code !== "en" && (complete ? done.includes(code) : !done.includes(code))');
  expect(js).toContain("  return t ? (x + 1) >>> 0 : Math.imul(x, 2) >>> 0;\n");
  const lib = await import(join(dir, "mir.js"));
  expect([lib.c(true, false, ""), lib.c(false, true, "en"), lib.kept(["en", "fr", "de"], true, ["fr"])]).toEqual([true, false, 1]);
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

// A float THIR's facts know whole and in range where it's cast, `index as
// u32` of `indexOf`'s once it isn't -1, is the value as it is
// (`whole_casts`), not `$f64ToInt(index, ..)`, as react.dev's CodeBlock
// finds a highlight's column.
test("a whole float's cast is the float from MIR too", async () => {
  const dir = fixture("whole-cast");
  writeFileSync(join(dir, "lib.rs"), `use js::string;
pub fn column(line: &str, substr: &str) -> u32 {
    let index = string::index_of(line, substr, None);
    if index == -1.0 {
        panic!("Could not find: '{substr}'");
    }
    index as u32 + string::length(substr)
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain("return (index + substr.length) >>> 0;");
  const lib = await import(join(dir, "mir.js"));
  expect(lib.column("abcd", "cd")).toBe(4);
});

// `mem::take(&mut current)` of a variable takes it and gives it a new
// value, `const old = current; current = [];`, not `$take(current, [])` of
// the object, a closure's `&mut` capture of one too, which is the
// variable; `mem::swap` of two, each given the other's.
test("taking a variable assigns the variable", async () => {
  const dir = fixture("take-variable");
  writeFileSync(join(dir, "lib.rs"), `pub fn direct(items: &[u32]) -> Vec<Vec<u32>> {
    let mut all = Vec::new();
    let mut current = Vec::new();
    for &item in items {
        if item == 0 {
            all.push(std::mem::take(&mut current));
        } else {
            current.push(item);
        }
    }
    all
}
pub fn captured(items: &[u32]) -> Vec<Vec<u32>> {
    let mut all = Vec::new();
    let mut current = Vec::new();
    items.iter().for_each(|&item| {
        if item == 0 {
            all.push(std::mem::take(&mut current));
        } else {
            current.push(item);
        }
    });
    all
}
pub fn swapped(mut a: Vec<u32>, mut b: Vec<u32>) -> (Vec<u32>, Vec<u32>) {
    std::mem::swap(&mut a, &mut b);
    (a, b)
}
pub fn half(mut a: Vec<u32>, b: &mut Vec<u32>) -> Vec<u32> {
    std::mem::swap(&mut a, b);
    a
}
pub fn kept(items: &[u32]) -> (Vec<Vec<u32>>, Vec<u32>) {
    let mut all = Vec::new();
    let mut current = Vec::new();
    items.iter().for_each(|&item| {
        if item == 0 {
            all.push(std::mem::take(&mut current));
        } else {
            current.push(item);
        }
    });
    (all, current)
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).not.toContain("$take(");
  expect(js).toContain("  const t = a;\n  a = b;\n  b = t;");
  expect(js).toContain("$exchange(a, b);");
  expect(js).toContain("      const old = current;\n      current = [];\n      all.push(old);");
  const lib = await import(join(dir, "mir.js"));
  expect([lib.direct([1, 2, 0, 3, 0]), lib.captured([1, 0, 0])]).toEqual([[[1, 2], [3]], [[1], []]]);
  expect(lib.kept([1, 0, 2, 3])).toEqual([[[1]], [2, 3]]);
});

// A value made in each branch of a `try`, a match of what a call gives,
// and given to a variable, `children = match .. {}`, is given to the
// variable in each, not to a temporary assigned after, as react.dev's
// Console shows what's logged.
test("a value a try makes in each branch is the variable's there", async () => {
  const dir = fixture("try-assigned");
  writeFileSync(join(dir, "lib.rs"), `use js::{Unknown, json, object};
pub fn shown(msg: Option<&Unknown>, tagged: bool) -> Option<String> {
    let children;
    if tagged {
        children = Some("tag".to_string());
    } else {
        children = match json::stringify_with(&msg, None, 2) {
            Ok(json) => json,
            Err(_) => match object::to_string(msg) {
                Ok(tag) => Some(tag),
                Err(_) => Some(format!("[{}]", js::type_of(msg))),
            },
        };
    }
    children
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain("    try {\n      children = JSON.stringify(msg, null, 2);");
  const lib = await import(join(dir, "mir.js"));
  const cyclic: any = {};
  cyclic.self = cyclic;
  expect([lib.shown({ a: 1 }, false), lib.shown(cyclic, false), lib.shown(undefined, true)]).toEqual(['{\n  "a": 1\n}', "[object Object]", "tag"]);
});

// An iterator's adapter of a JS iterator, `.map(..)` of a range's, is a JS
// iterator already: the next adapter is its own, not `Iterator.from(..)`'s,
// nor an array's made of it, `Array.from(..)`, as react.dev's
// useTocHighlight finds its anchors.
test("an adapter of a JS iterator is not made one again", async () => {
  const dir = fixture("iterator-from-adapter");
  writeFileSync(join(dir, "lib.rs"), `pub fn evens(n: u32) -> Vec<u32> {
    (0..n).filter_map(|i| if i % 2 == 0 { Some(i * 10) } else { None }).filter(|x| *x > 5).collect()
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).not.toMatch(/(?:Iterator|Array)\.from\(/);
  expect(js).toContain(".filter((x) => x > 5)\n    .toArray()");
  const lib = await import(join(dir, "mir.js"));
  expect(lib.evens(10)).toEqual([20, 40, 60, 80]);
});

// From MIR: an array's iterator made for an adapter alone, `log.iter()
// .filter(..)` of closures that do something, is the array's own,
// `log.values().filter(..)`, lazy as Rust's adapters are; no other name
// for it steps it, so no `$iter`, which knows where it is, is needed, as
// react.dev's Console filters what's logged.
test("an array's iterator for an adapter alone is its values", async () => {
  const dir = fixture("values-adapter");
  writeFileSync(join(dir, "lib.rs"), `#[cfg_attr(rust_js, rust_js::link_name = "globalThis.seen")] #[allow(unused_variables)] fn seen(x: u32) -> u32 { unreachable!() }
pub fn joined(prev: &[u32], log: &[u32]) -> Vec<u32> {
    let new_logs = log.iter().filter(|x| seen(**x) != 3).map(|x| seen(x * 2));
    prev.iter().cloned().chain(new_logs).collect()
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain("const newLogs = log\n    .values()\n    .filter(");
  const lib = await import(join(dir, "mir.js"));
  const order: number[] = [];
  (globalThis as any).seen = (x: number) => (order.push(x), x);
  expect(lib.joined([1], [3, 4])).toEqual([1, 8]);
  expect(order).toEqual([3, 4, 8]);
});

// An update's derived `Default` base, `..Default::default()`, is each
// default the update reads, where it reads it, `{ size, name: "", tags: []
// }`, made without the base's `default()`, which a generic component's
// `children: C` would need a dictionary for.
test("an update's default base is the defaults it reads", async () => {
  const dir = fixture("default-base");
  writeFileSync(join(dir, "lib.rs"), `#[derive(Default, Debug)]
pub struct Options {
    pub name: String,
    pub size: u32,
    pub tags: Vec<String>,
    pub label: Option<String>,
}
pub fn sized(size: u32) -> Options {
    Options { size, ..Default::default() }
}
pub fn named(name: String, size: u32) -> String {
    let o = Options { name, size, ..Default::default() };
    format!("{} {} {}", o.name, o.size, o.tags.len())
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain('return { name: "", size, tags: [] };');
  expect(js).toContain("const o = { name, size, tags: [] };");
  const lib = await import(join(dir, "mir.js"));
  expect([lib.sized(3), lib.named("a", 2)]).toEqual([{ name: "", size: 3, tags: [] }, "a 2 0"]);
});

// A range's items each mapped, all collected, `(0..n).map(f).collect()`,
// are `$range(0, n).map(f)`, an array made as big as what's collected
// anyway; one filtered or taken from stays lazy, as a big range's array
// would be made for little.
test("a range mapped and collected is its array mapped", async () => {
  const dir = fixture("range-mapped");
  writeFileSync(join(dir, "lib.rs"), `pub fn squares(n: u32) -> Vec<u32> {
    (0..n).map(|i| i * i).collect()
}
pub fn few() -> Vec<u32> {
    (0..u32::MAX).filter(|i| i % 2 == 0).take(3).collect()
}
pub fn wide(n: i64) -> Vec<i64> {
    (0..n).map(|i| i * 2).collect()
}
// Its last included, backwards, or not mapped at all.
pub fn stars(n: usize) -> Vec<String> {
    (1..=n).rev().map(|k| "*".repeat(k)).collect()
}
pub fn down(n: u32) -> Vec<u32> {
    (0..n).rev().collect()
}
pub fn upto(n: u32) -> Vec<u32> {
    (1..=n).collect()
}
// Filtered, its numbers made as they're stepped, which a filter may keep
// few of: no array of them all.
pub fn odd_squares(n: u32) -> Vec<u32> {
    (0..n).filter(|i| i % 2 == 1).map(|i| i * i).collect()
}
// To a constant, included: its end the next, \`$range(1, 4)\`.
pub fn doubled() -> Vec<u32> {
    (1..=3).map(|i| i * 2).collect()
}
pub fn odd_to_five() -> Vec<u32> {
    (1..=5).filter(|i| i % 2 == 1).collect()
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain("return $range(0, n).map((i) => Math.imul(i, i) >>> 0);");
  expect(js).toContain('return $range(1, n + 1)\n    .toReversed()\n    .map((k) => "*".repeat(k));');
  expect(js).toContain("return $range(0, n).toReversed();");
  expect(js).toContain("return $range(1, n + 1);");
  expect(js).toContain("return $lazyRange(0, n)\n    .filter(");
  expect(js).toContain("return $range(1, 4).map(");
  expect(js).toContain("return $lazyRange(1, 6)\n");
  const lib = await import(join(dir, "mir.js"));
  expect([lib.squares(4), lib.wide(3n)]).toEqual([[0, 1, 4, 9], [0n, 2n, 4n]]);
  expect([lib.stars(3), lib.stars(0), lib.down(3), lib.upto(3), lib.upto(0)]).toEqual([["***", "**", "*"], [], [2, 1, 0], [1, 2, 3], []]);
  expect(lib.oddSquares(6)).toEqual([1, 9, 25]);
  expect([lib.doubled(), lib.oddToFive()]).toEqual([[2, 4, 6], [1, 3, 5]]);
  expect(lib.few()).toEqual([0, 2, 4]);
});

// A closure that calls only a method named as a slice's that only reads,
// `at`, isn't therefore one that does nothing: the crate's own `at`,
// `P.at(p)`, may print. Its chain runs item by item, as Rust's does; a
// \`str\`'s, as Rust's types tell, is the array's own methods.
test("a crate's method named as a slice's reader still runs item by item", async () => {
  const dir = fixture("named-reader");
  writeFileSync(join(dir, "lib.rs"), `pub struct P;
impl P {
    pub fn at(&self) -> u32 {
        println!("at");
        1
    }
}
pub fn run() -> usize {
    let p = P;
    let v: Vec<u32> = vec![1, 2];
    let w: Vec<u32> = v.iter().map(|&x| x + p.at()).filter(|&x| { println!("f{x}"); true }).collect();
    w.len()
}
pub fn a_names(names: Vec<String>) -> usize {
    let w: Vec<&String> = names.iter().filter(|n| n.starts_with("a")).inspect(|n| println!("{n}")).collect();
    w.len()
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const lib = await import(join(dir, "mir.js"));
  const logs: string[] = [];
  const log = console.log;
  console.log = (line: string) => logs.push(line);
  try {
    expect([lib.run(), lib.aNames(["ab", "b", "ac"])]).toEqual([2, 2]);
  } finally {
    console.log = log;
  }
  expect(logs).toEqual(["at", "f2", "at", "f3", "ab", "ac"]);
  // A \`str\`'s \`starts_with\` only reads: the array's own \`filter\`.
  expect(readFileSync(join(dir, "mir.js"), "utf8")).toContain(`names\n    .filter((n) => n.startsWith("a"))\n`);
});

// A handle's value read or written in place, `Cell::set` of `&self.n`, is
// its place's, `t.n = ..`: no object of a getter and a setter made to be
// used once.
test("a handle used in place from MIR is its place", async () => {
  const dir = fixture("handle-in-place");
  writeFileSync(join(dir, "lib.rs"), `use std::cell::Cell;
pub struct T { n: Cell<u32> }
impl T {
    pub fn bump(&self) -> u32 { self.n.set(self.n.get() + 1); self.n.get() }
}
pub fn run() -> u32 {
    let t = T { n: Cell::new(0) };
    t.bump();
    t.bump()
}
pub fn pair() -> String {
    match &mut (1, 2) {
        (a, b) => {
            *a += 10;
            *b += 20;
            format!("{a} {b}")
        }
    }
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  expect(readFileSync(join(dir, "mir.js"), "utf8")).toContain("  bump(t) {\n    t.n = (t.n + 1) >>> 0;\n    return t.n;\n  },");
  expect(readFileSync(join(dir, "mir.js"), "utf8")).toContain("  return `${value[0]} ${value[1]}`;");
  expect((await import(join(dir, "mir.js"))).run()).toBe(2);
  expect((await import(join(dir, "mir.js"))).pair()).toBe("11 22");
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

// `Math`'s functions but `random` only compute their value: a range's end
// of one, `..len.saturating_sub(1)`, is read where it's used, not a
// `{ end }` object made first.
test("a slice to a computed end from MIR reads the end where it's used", async () => {
  const dir = fixture("slice-computed-end");
  writeFileSync(join(dir, "lib.rs"), `pub fn crumbs(segments: Vec<&str>) -> Vec<&str> {
    segments[..segments.len().saturating_sub(1)].to_vec()
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  expect(readFileSync(join(dir, "mir.js"), "utf8")).toContain("$slice(segments, 0, Math.max(segments.length - 1, 0))");
  const lib = await import(join(dir, "mir.js"));
  expect([lib.crumbs(["a", "b", "c"]), lib.crumbs([])]).toEqual([["a", "b"], []]);
});

// A slice's `iter_mut()` is an array's items: its chain the array's own
// methods, `v.find(f)`, `$mutItems(v).filter(f)`, not a JS iterator's of
// it, `$lent(Iterator.from(v)).find(f)`.
test("iter_mut chains from MIR are array methods", async () => {
  const dir = fixture("iter-mut-chains");
  writeFileSync(join(dir, "lib.rs"), `pub fn count(v: &mut Vec<(u32, u32)>, w: u32) {
    if let Some(e) = v.iter_mut().find(|(k, _)| *k == w) {
        e.1 += 1;
    } else {
        v.push((w, 1));
    }
}
pub fn big(v: &mut [u32]) -> usize {
    v.iter_mut().filter(|x| **x > 2).count()
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain("const e = v.find(([k]) => k === w);");
  expect(js).toContain("return $mutItems(v).filter((x) => x.value > 2).length;");
  const lib = await import(join(dir, "mir.js"));
  const v = [[1, 1]];
  lib.count(v, 1);
  lib.count(v, 2);
  expect([v, lib.big([1, 3, 5])]).toEqual([[[1, 2], [2, 1]], 2]);
});

// An untagged enum is its payload (ADR 0214): an `Option` of one whose every
// payload is never falsy, an array or a JS object, as react.dev's
// NavigationBar's `contentBoxSize` is, is there where it's truthy, `if
// (size)`; of one that may be text, `""`, it's `size != null`.
test("an option of an untagged enum never falsy is tested by its truth", async () => {
  const dir = fixture("untagged-never-falsy");
  writeFileSync(join(dir, "lib.rs"), `pub struct Size { pub w: u32 }
#[cfg_attr(rust_js, rust_js::untagged)]
pub enum Boxes<'a> {
    Many(&'a [Size]),
    #[cfg_attr(rust_js, rust_js::otherwise)]
    One(&'a Size),
}
#[cfg_attr(rust_js, rust_js::untagged)]
pub enum Label<'a> {
    Text(&'a str),
    #[cfg_attr(rust_js, rust_js::otherwise)]
    At(&'a Size),
}
pub fn width(size: Option<Boxes>) -> u32 {
    if let Some(size) = size {
        match size { Boxes::Many(all) => all[0].w, Boxes::One(one) => one.w }
    } else {
        0
    }
}
pub fn labeled(label: Option<Label>) -> bool {
    label.is_some()
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain("  if (size) {\n");
  expect(js).toContain("return label != null;");
  const lib = await import(join(dir, "mir.js"));
  expect([lib.width([{ w: 3 }]), lib.width({ w: 4 }), lib.width(undefined), lib.labeled(""), lib.labeled(undefined)]).toEqual([3, 4, 0, true, false]);
});

// A `let … else { continue; }` the program wrote, as react.dev's llmsTxt
// has of its current section, stays the guard it is from MIR, `if (!s) {
// continue; }` and what follows, whatever its test.
test("a let-else's written continue from MIR stays a guard", async () => {
  const dir = fixture("let-else-continue");
  writeFileSync(join(dir, "lib.rs"), `pub struct Section { pub pages: Vec<u32> }
pub fn sections(items: &[u32]) -> Vec<Section> {
    let mut out = Vec::new();
    let mut current: Option<Section> = None;
    for &item in items {
        if item == 0 {
            if let Some(section) = current {
                out.push(section);
            }
            current = Some(Section { pages: Vec::new() });
            continue;
        }
        let Some(current) = &mut current else {
            continue;
        };
        if item > 10 {
            current.pages.push(item / 10);
        } else {
            current.pages.push(item);
        }
    }
    if let Some(section) = current {
        out.push(section);
    }
    out
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  expect(readFileSync(join(dir, "mir.js"), "utf8")).toContain("    if (!current) {\n      continue;\n    }\n    if (item > 10) {\n");
  expect((await import(join(dir, "mir.js"))).sections([5, 0, 30, 2, 0, 7])).toEqual([{ pages: [3, 2] }, { pages: [7] }]);
});


// What MIR assigns once on each way, `classNameToken(if .. { "string" } else
// { "static" })`'s argument, isn't a variable the program names: it doesn't
// read alike, so what's made before it in a `vec!` stays where it's written,
// as react.dev's CodeBlock's `getSyntaxHighlight` has its specs, not held in
// `const`s first.
test("a vec's specs from MIR stay in place past a branch's temporary", async () => {
  const dir = fixture("specs-in-place");
  writeFileSync(join(dir, "lib.rs"), `use std::any::Any; use std::marker::PhantomData; use js::JsObject;
pub struct Syntax { pub string: String } pub struct Theme { pub syntax: Syntax }
pub struct Tag(PhantomData<JsObject>);
pub struct HighlightStyle(PhantomData<JsObject>);
#[cfg_attr(rust_js, rust_js::link_name = "@codemirror/language#HighlightStyle.define")]
#[allow(unused_variables)]
fn define(specs: Vec<&dyn Any>) -> &'static HighlightStyle {
    unreachable!()
}
mod tags {
    use super::Tag;

    unsafe extern "Rust" {
        #[link_name = "@lezer/highlight#tags.link"]
        pub safe static link: &'static Tag;
        #[link_name = "@lezer/highlight#tags.emphasis"]
        pub safe static emphasis: &'static Tag;
        #[link_name = "@lezer/highlight#tags.strong"]
        pub safe static strong: &'static Tag;
        #[link_name = "@lezer/highlight#tags.keyword"]
        pub safe static keyword: &'static Tag;
        #[link_name = "@lezer/highlight#tags.atom"]
        pub safe static atom: &'static Tag;
        #[link_name = "@lezer/highlight#tags.number"]
        pub safe static number: &'static Tag;
        #[link_name = "@lezer/highlight#tags.bool"]
        pub safe static bool: &'static Tag;
        #[link_name = "@lezer/highlight#tags.tagName"]
        pub safe static tagName: &'static Tag;
        #[link_name = "@lezer/highlight#tags.variableName"]
        pub safe static variableName: &'static Tag;
        #[link_name = "@lezer/highlight#tags.propertyName"]
        pub safe static propertyName: &'static Tag;
        #[link_name = "@lezer/highlight#tags.literal"]
        pub safe static literal: &'static Tag;
        #[link_name = "@lezer/highlight#tags.inserted"]
        pub safe static inserted: &'static Tag;
        #[link_name = "@lezer/highlight#tags.punctuation"]
        pub safe static punctuation: &'static Tag;
        #[link_name = "@lezer/highlight#tags.comment"]
        pub safe static comment: &'static Tag;
        #[link_name = "@lezer/highlight#tags.quote"]
        pub safe static quote: &'static Tag;
    }

    #[cfg_attr(rust_js, rust_js::link_name = "@lezer/highlight#tags.standard")]
    #[allow(unused_variables)]
    pub fn standard(tag: &Tag) -> &'static Tag {
        unreachable!()
    }

    #[cfg_attr(rust_js, rust_js::link_name = "@lezer/highlight#tags.function")]
    #[allow(unused_variables)]
    pub fn function(tag: &Tag) -> &'static Tag {
        unreachable!()
    }

    #[cfg_attr(rust_js, rust_js::link_name = "@lezer/highlight#tags.definition")]
    #[allow(unused_variables)]
    pub fn definition(tag: &Tag) -> &'static Tag {
        unreachable!()
    }
}

/// A spec's tag, or several.
#[cfg_attr(rust_js, rust_js::untagged)]
enum Tags {
    One(&'static Tag),
    Many(Vec<&'static Tag>),
}

struct TextDecoratorStyle {
    tag: Tags,
    textdecorator: &'static str,
}

struct FontStyleStyle {
    tag: Tags,
    font_style: &'static str,
}

struct FontWeightStyle {
    tag: Tags,
    font_weight: &'static str,
}

struct ClassStyle {
    tag: Tags,
    class: String,
}
fn classNameToken(name: &str) -> String {
    format!("sp-syntax-{name}")
}

pub fn getSyntaxHighlight(theme: &Theme) -> &'static HighlightStyle {
    define(vec![
        &TextDecoratorStyle {
            tag: Tags::One(tags::link),
            textdecorator: "underline",
        },
        &FontStyleStyle {
            tag: Tags::One(tags::emphasis),
            font_style: "italic",
        },
        &FontWeightStyle {
            tag: Tags::One(tags::strong),
            font_weight: "bold",
        },
        &ClassStyle {
            tag: Tags::One(tags::keyword),
            class: classNameToken("keyword"),
        },
        &ClassStyle {
            tag: Tags::Many(vec![tags::atom, tags::number, tags::bool]),
            class: classNameToken("static"),
        },
        &ClassStyle {
            tag: Tags::One(tags::standard(tags::tagName)),
            class: classNameToken("tag"),
        },
        &ClassStyle {
            tag: Tags::One(tags::variableName),
            class: classNameToken("plain"),
        },
        &ClassStyle {
            // Highlight function call
            tag: Tags::One(tags::function(tags::variableName)),
            class: classNameToken("definition"),
        },
        &ClassStyle {
            // Highlight function definition differently (eg: functional component def in React)
            tag: Tags::Many(vec![
                tags::definition(tags::function(tags::variableName)),
                tags::tagName,
            ]),
            class: classNameToken("definition"),
        },
        &ClassStyle {
            tag: Tags::One(tags::propertyName),
            class: classNameToken("property"),
        },
        &ClassStyle {
            tag: Tags::Many(vec![tags::literal, tags::inserted]),
            class: classNameToken(if !theme.syntax.string.is_empty() {
                "string"
            } else {
                "static"
            }),
        },
        &ClassStyle {
            tag: Tags::One(tags::punctuation),
            class: classNameToken("punctuation"),
        },
        &ClassStyle {
            tag: Tags::Many(vec![tags::comment, tags::quote]),
            class: classNameToken("comment"),
        },
    ])
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  expect(readFileSync(join(dir, "mir.js"), "utf8")).toContain('    { tag: tags.keyword, class: classNameToken("keyword") },\n');
});

// A range of numbers made where it's stepped, `(0..n).find(f)`, is its
// numbers made as they're stepped from MIR, `$lazyRange(0, n).find(f)`, which
// stops where Rust's does: not an array of all of them made first, which a
// big range can't be, nor a range object stepped by `$rangeNext`. Kept lazy,
// as react.dev's getHeaderAnchors keeps a chain whose closure may panic, too.
// Of \`char\`s, a range stays one.
test("a range made where it's stepped from MIR is its numbers, made lazily", async () => {
  const dir = fixture("range-items");
  writeFileSync(join(dir, "lib.rs"), `pub fn first(n: u32, f: impl Fn(u32) -> bool) -> Option<u32> {
    (0..n).find(|&i| f(i))
}
pub fn picked(v: &[Option<u32>]) -> Vec<u32> {
    (0..v.len()).filter_map(|i| v[i]).filter(|x| x.checked_sub(1).unwrap() > 0).collect()
}
pub fn letter(f: impl Fn(char) -> bool) -> Option<char> {
    ('a'..'e').find(|&c| f(c))
}
pub fn evens(n: u32) -> Vec<u32> {
    (0..n).filter(|i| i % 2 == 0).collect()
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const lib = await import(join(dir, "mir.js"));
  expect([lib.first(5, (i: number) => i > 2), lib.first(2, () => false), lib.picked([3, undefined, 2])]).toEqual([3, undefined, [3, 2]]);
  expect([lib.letter((c: string) => c > "b"), lib.letter(() => false)]).toEqual(["c", undefined]);
  expect(lib.evens(5)).toEqual([0, 2, 4]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  // Of a filter, which may keep few of many, no array of them all either.
  expect(js).toContain("return $lazyRange(0, n)\n    .filter((i) => i % 2 === 0)\n    .toArray();");
  expect(js).toContain("return $lazyRange(0, n).find(f);");
  expect(js).toContain("return $lazyRange(0, v.length)\n    .map(");
  // A billion numbers, the first that holds the third: as quick as Rust's.
  const { first } = await import(join(dir, "mir.js"));
  expect(first(1_000_000_000, (i: number) => i === 2)).toBe(2);
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

// `String::from_iter(items)` from MIR: an array's joined, `["h",
// "i"].join("")`, or what an iterator steps through, joined.
test("a string from items from MIR is them joined", async () => {
  const dir = fixture("string-from-iter");
  writeFileSync(join(dir, "lib.rs"), `pub fn letters() -> String {
    String::from_iter(['h', 'é', '!'])
}
pub fn backwards(s: &str) -> String {
    String::from_iter(s.chars().rev().filter(|c| *c != 'x'))
}
fn of<I: Iterator<Item = char>>(items: I) -> String {
    String::from_iter(items)
}
pub fn counted(n: u32) -> String {
    of((0..n).map(|i| char::from_digit(i, 10).unwrap()))
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain('return ["h", "é", "!"].join("");');
  // A JS iterator has no \`join\` but Bun's: its items, an array's.
  expect(js).toContain('return Iterator.from(items).toArray().join("");');
  const lib = await import(join(dir, "mir.js"));
  expect([lib.letters(), lib.backwards("axbc"), lib.counted(4)]).toEqual(["hé!", "cba", "0123"]);
});

// A range cloned and stepped is the copy stepped: the range itself, read
// after, is as it was, `1..4`. Found in review: MIR stepped the range
// itself, `$iterator(r, $rangeNext)`, which `4..4` after.
test("a range cloned and stepped leaves the range as it was", async () => {
  const dir = fixture("range-clone-stepped");
  writeFileSync(join(dir, "lib.rs"), `pub fn counted(a: i32, z: i32) -> String {
    let r = a..z;
    let c = r.clone().count();
    format!("{c} {:?}", r)
}
pub fn summed(a: i32, z: i32) -> String {
    let r = a..=z;
    let s: i32 = r.clone().sum();
    format!("{s} {:?}", r)
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const lib = await import(join(dir, "mir.js"));
  expect([lib.counted(1, 4), lib.summed(1, 3)]).toEqual(["3 1..4", "6 1..=3"]);
});

// An iterator of the crate's `len()`: std's of its `size_hint()`, which
// panics where that isn't exact, or its own where it has one.
test("an iterator's len from MIR is its own or its size hint's", async () => {
  const dir = fixture("iterator-len");
  writeFileSync(join(dir, "lib.rs"), `pub struct Span { pub low: u32, pub high: u32 }
impl Iterator for Span {
    type Item = u32;
    fn next(&mut self) -> Option<u32> {
        if self.low < self.high { self.low += 1; Some(self.low - 1) } else { None }
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        let n = (self.high - self.low) as usize;
        (n, Some(n))
    }
}
impl ExactSizeIterator for Span {}
pub struct Own;
impl Iterator for Own {
    type Item = u8;
    fn next(&mut self) -> Option<u8> { None }
}
impl ExactSizeIterator for Own {
    fn len(&self) -> usize { 42 }
}
pub fn lens() -> (usize, usize) {
    let mut s = Span { low: 0, high: 5 };
    s.next();
    (s.len(), Own.len())
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  expect((await import(join(dir, "mir.js"))).lens()).toEqual([4, 42]);
});

// A chunks' `remainder()` from MIR: of one made there, its array's,
// `$chunksExact(v, 2).remainder`; of one stepped, which MIR keeps as a
// `$iter`, its items' still.
test("a chunks' remainder from MIR, made there or stepped", async () => {
  const dir = fixture("chunks-remainder");
  writeFileSync(join(dir, "lib.rs"), `pub fn rest(v: &[u32]) -> Vec<u32> {
    v.chunks_exact(2).remainder().to_vec()
}
pub fn stepped(v: &[u32]) -> (Vec<u32>, Vec<u32>) {
    let mut c = v.chunks_exact(2);
    let first = c.next().map(|s| s.to_vec()).unwrap_or_default();
    (first, c.remainder().to_vec())
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  expect(readFileSync(join(dir, "mir.js"), "utf8")).toContain("$chunksExact(v, 2).remainder");
  const lib = await import(join(dir, "mir.js"));
  expect([lib.rest([1, 2, 3]), lib.stepped([1, 2, 3, 4, 5])]).toEqual([[3], [[1, 2], [5]]]);
});

// A `Vec`'s range pushed again, `extend_from_within`, or replaced,
// `splice`, all of it too: `..` reads the `Vec` itself, no range helper's
// call.
test("a vec's range pushed again or replaced, all of it too", async () => {
  const dir = fixture("vec-range-edits");
  writeFileSync(join(dir, "lib.rs"), `pub fn doubled(mut v: Vec<u32>) -> Vec<u32> {
    v.extend_from_within(..);
    v
}
pub fn tail_again(mut v: Vec<u32>) -> Vec<u32> {
    v.extend_from_within(1..);
    v
}
pub fn replaced(mut v: Vec<u32>) -> (Vec<u32>, Vec<u32>) {
    let old: Vec<u32> = v.splice(.., [9, 8]).collect();
    (v, old)
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const lib = await import(join(dir, "mir.js"));
  expect([lib.doubled([1, 2]), lib.tailAgain([1, 2, 3]), lib.replaced([1, 2, 3])]).toEqual([[1, 2, 1, 2], [1, 2, 3, 2, 3], [[9, 8], [1, 2, 3]]]);
});

// An iterator of the crate's from MIR: what its impl writes itself, its
// own `count`, is its; what it takes from `Iterator`, `map`, `fuse` or
// `collect`, std's over its own `next`; and given to another's `chain`, its
// items, not its fields.
test("the crate's iterator from MIR: its own methods, std's, and given on", async () => {
  const dir = fixture("user-iterator-methods");
  writeFileSync(join(dir, "lib.rs"), `pub struct Counter(pub u32);
impl Iterator for Counter {
    type Item = u32;
    fn next(&mut self) -> Option<u32> {
        if self.0 < 3 { self.0 += 1; Some(self.0) } else { None }
    }
    fn count(self) -> usize { 99 }
}
pub fn run() -> (usize, Vec<u32>, Vec<u32>) {
    let own = Counter(0).count();
    let mapped: Vec<u32> = Counter(0).map(|n| n * 10).fuse().collect();
    let chained: Vec<u32> = [7].into_iter().chain(Counter(1)).collect();
    (own, mapped, chained)
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  expect((await import(join(dir, "mir.js"))).run()).toEqual([99, [10, 20, 30], [7, 2, 3]]);
});

// The crate's iterator stepped through a `&mut` to it, `by_ref()`, from
// MIR: std's `next` of a `&mut I` is `I`'s own, and what's left is still
// there after.
test("the crate's iterator stepped through by_ref from MIR", async () => {
  const dir = fixture("user-iterator-by-ref");
  writeFileSync(join(dir, "lib.rs"), `pub struct Counter(pub u32);
impl Iterator for Counter {
    type Item = u32;
    fn next(&mut self) -> Option<u32> {
        if self.0 < 5 { self.0 += 1; Some(self.0) } else { None }
    }
}
pub fn run() -> (Vec<u32>, Vec<u32>) {
    let mut all = Counter(0);
    let mut first = Vec::new();
    for n in all.by_ref() {
        first.push(n);
        if n == 2 { break; }
    }
    let rest: Vec<u32> = all.collect();
    (first, rest)
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  expect((await import(join(dir, "mir.js"))).run()).toEqual([[1, 2], [3, 4, 5]]);
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

// An arm that ends in a loop, which leaves the match where it ends:
// the loop's own `break`, not a labeled block's.
test("a match arm that ends in a loop leaves it with the loop's break", async () => {
  const dir = fixture("arm-loop-exit");
  writeFileSync(join(dir, "lib.rs"), `pub fn words(s: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut chars = s.chars().peekable();
    while let Some(&c) = chars.peek() {
        match c {
            'a'..='z' => {
                let mut w = String::new();
                while let Some(c) = chars.next_if(|c| c.is_ascii_lowercase()) {
                    w.push(c);
                }
                out.push(w);
            }
            _ => {
                chars.next();
            }
        }
    }
    out
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).not.toMatch(/\bbb\d+\b/);
  expect(js).toContain(`    if (c >= "a" && c <= "z") {\n      let w = "";`);
  expect(js).toContain(`          out.push(w);\n          break;\n`);
  const lib = await import(join(dir, "lib.js"));
  expect(lib.words("ab cd,e")).toEqual(["ab", "cd", "e"]);
  expect(lib.words("")).toEqual([]);
});

// Counted loops in each branch of a loop's body, after another loop: each
// its own `for`, the branches' end where the `if` does, not the outer loop's.
test("counted loops in a loop's branches stay where they're written", async () => {
  const dir = fixture("branch-loops");
  writeFileSync(join(dir, "lib.rs"), `pub fn scaled(seeds: &[u32]) -> Vec<f32> {
    let mut out = Vec::new();
    let mut power = 1.0f32;
    for _ in 0..3 {
        power *= 2.0;
    }
    out.push(power);
    for &seed in seeds {
        let mut x = seed as f32;
        let scale = seed % 7;
        if scale < 3 {
            for _ in 0..scale + 1 {
                x *= 0.5;
            }
        } else {
            for _ in 0..scale - 3 {
                x *= 2.0;
            }
        }
        if seed % 2 == 0 {
            x = -x;
        }
        out.push(x);
    }
    out
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).not.toContain("$rangeNext");
  const lib = await import(join(dir, "lib.js"));
  expect(lib.scaled([1, 5, 10, 13])).toEqual([8, 0.25, 20, -10, 104]);
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

// What rustc promotes in generic code it can't evaluate, `&size_of::<T>()`,
// `&None`, and a closure that captures nothing: made where they're used.
test("promoted values of generic code are made where they're used", async () => {
  const dir = fixture("promoted-generic");
  writeFileSync(join(dir, "lib.rs"), `use std::cell::Cell;
fn size_ref<T>() -> usize {
    let s: &usize = &std::mem::size_of::<T>();
    *s
}
pub fn sizes() -> Vec<usize> {
    vec![size_ref::<u8>(), size_ref::<u64>(), size_ref::<(u8, u32)>()]
}
fn none_ref<T: Clone>(v: Option<T>) -> Option<T> {
    let n: &Option<T> = &None;
    if v.is_some() { v } else { n.clone() }
}
pub fn nones() -> (Option<u32>, Option<u32>) {
    (none_ref(Some(3)), none_ref(None))
}
thread_local! {
    static LAST: Cell<u32> = const { Cell::new(0) };
}
fn apply(f: &dyn Fn(u32), n: u32) {
    f(n)
}
pub fn last() -> u32 {
    apply(&|n| LAST.with(|l| l.set(n)), 7);
    LAST.with(|l| l.get())
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const lib = await import(join(dir, "lib.js"));
  expect([lib.sizes(), lib.nones(), lib.last()]).toEqual([[1, 8, 8], [3, undefined], 7]);
});

// A `char` range stepped as Rust steps it, by its code point, past the
// surrogates no `char` is: in a `for`, given to `extend`, and an iterator
// turned around after it's been stepped from both ends.
test("char ranges step by code point, and a stepped iterator turns around", async () => {
  const dir = fixture("char-ranges");
  writeFileSync(join(dir, "lib.rs"), `pub fn letters() -> String {
    let mut s = String::new();
    for c in 'a'..'e' {
        s.push(c);
    }
    for c in 'x'..='z' {
        s.push(c);
    }
    s
}
pub fn from(a: char, b: char) -> String {
    let mut s = String::new();
    for c in a..b {
        s.push(c);
    }
    s
}
pub fn across() -> Vec<u32> {
    let mut out = Vec::new();
    for c in '\\u{d7fe}'..'\\u{e001}' {
        out.push(c as u32);
    }
    out
}
pub fn extended() -> Vec<u32> {
    let mut v: Vec<char> = vec!['a'];
    v.extend('b'..'d');
    v.extend('\\u{d7fe}'..'\\u{e001}');
    v.into_iter().map(|c| c as u32).collect()
}
pub fn counted(n: u32) -> u32 {
    let mut total = 0;
    for i in 0..n {
        total += i;
    }
    total
}
pub fn between(k: u32, n: u32) -> u32 {
    let mut total = 0;
    for i in k..n {
        total += i;
    }
    total
}
pub fn reversed() -> Vec<u32> {
    let mut it = [1u32, 2, 3, 4].iter();
    it.next_back();
    it.next();
    it.rev().copied().collect()
}
pub fn kept_reversed(v: &[u32]) -> Vec<u32> {
    let it = v.iter();
    it.rev().copied().collect()
}
pub fn owned_reversed(v: Vec<u32>) -> Vec<u32> {
    let mut it = v.into_iter();
    it.next();
    it.rev().collect()
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const lib = await import(join(dir, "lib.js"));
  expect(lib.letters()).toBe("abcdxyz");
  expect(lib.from("p", "t")).toBe("pqrs");
  expect(lib.across()).toEqual([0xd7fe, 0xd7ff, 0xe000]);
  expect(lib.extended()).toEqual([97, 98, 99, 0xd7fe, 0xd7ff, 0xe000]);
  expect(lib.counted(5)).toBe(10);
  expect(lib.reversed()).toEqual([3, 2]);
  expect([lib.keptReversed([1, 2, 3]), lib.ownedReversed([1, 2, 3])]).toEqual([[3, 2, 1], [3, 2]]);
  // A number's range is still JS's counted `for`.
  expect(lib.between(2, 5)).toBe(9);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("for (let i = 0; i < n; i++) {");
  expect(js).toContain("for (let i = k; i < n; i++) {");
});

// A `for` whose binding is `mut`, which MIR declares before the loop and
// assigns each turn: a `for…of` all the same, of an iterator stepped
// first too.
test("a for loop of a mut binding is a for...of", async () => {
  const dir = fixture("for-mut-binding");
  writeFileSync(join(dir, "lib.rs"), `pub fn plain_mut(v: Vec<Vec<u32>>) -> usize {
    let mut n = 0;
    for mut row in v {
        row.push(0);
        n += row.len();
    }
    n
}
pub fn rest_mut(v: Vec<Vec<u32>>) -> usize {
    let mut it = v.into_iter();
    it.next();
    let mut n = 0;
    for mut row in it {
        row.push(0);
        n += row.len();
    }
    n
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("  for (const row of v) {\n    row.push(0);");
  expect(js).toContain("  for (const row of it) {\n    row.push(0);");
  expect(js).not.toContain("let row;");
  const lib = await import(join(dir, "lib.js"));
  expect([lib.plainMut([[1], [2, 3]]), lib.restMut([[1], [2, 3], [4]])]).toEqual([5, 5]);
});

// ADR 0420: what `Vec::set_len` leaves past a vector's length is read
// through a pointer, or a shared slice, a copy; a mutable slice over it,
// which would be a view of the array, isn't, rust-js's error where it's made.
test("a mutable slice over what set_len left is an error where it's made", async () => {
  const dir = fixture("set-len-spare");
  writeFileSync(join(dir, "lib.rs"), `pub fn shared() -> Vec<u32> {
    let mut v = vec![1, 2, 3];
    unsafe {
        v.set_len(1);
        std::slice::from_raw_parts(v.as_ptr(), 3).to_vec()
    }
}
pub fn mutable() -> u32 {
    let mut v = vec![1, 2, 3];
    unsafe {
        v.set_len(1);
        std::slice::from_raw_parts_mut(v.as_mut_ptr(), 3)[2]
    }
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const lib = await import(join(dir, "lib.js"));
  expect(lib.shared()).toEqual([1, 2, 3]);
  expect(() => lib.mutable()).toThrow("rust-js does not support a mutable slice past a vector's length");
});

// A `no_std` crate that names `alloc` its own way, hashbrown's `extern crate
// alloc as stdalloc`: rustc names std's items through it, and what rust-js
// knows of each is still std's, `Vec::capacity` and the allocator's.
test("std's items named through a crate's own name for alloc are std's", async () => {
  const dir = fixture("aliased-alloc");
  writeFileSync(join(dir, "lib.rs"), `#![no_std]
extern crate alloc as stdalloc;
use stdalloc::vec::Vec;
pub fn made(n: usize) -> usize {
    let mut v: Vec<u8> = Vec::with_capacity(n);
    v.push(1);
    v.capacity()
}
pub fn sized(n: usize) -> usize {
    let layout = stdalloc::alloc::Layout::from_size_align(n, 8).unwrap();
    unsafe {
        let p = stdalloc::alloc::alloc(layout);
        stdalloc::alloc::dealloc(p, layout);
    }
    layout.size()
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--crate-type", "lib", "--crate-name", "aliased"]);
  const lib = await import(join(dir, "lib.js"));
  expect([lib.made(4), lib.sized(16)]).toEqual([4, 16]);
});

// An `if` the program wrote with an `else` keeps it, in a loop too, where
// MIR's structure makes a guard of an `else` that returns, `if (!c) {
// return; } …`, and a `continue` of a branch the loop's body ends with;
// its `else if` too, React's scheduler's `siftDown` (ADR 0575). A guard
// the program wrote, an `if` with no `else`, stays one.
test("an if written with an else keeps it, in a loop and of an else-if", async () => {
  const dir = fixture("written-elses");
  writeFileSync(join(dir, "lib.rs"), `pub fn sift_down(heap: &mut Vec<f64>, i: usize) -> usize {
    let mut index = i;
    let length = heap.len();
    while index < length / 2 {
        let left = index * 2 + 1;
        let right = left + 1;
        if heap[left] < heap[index] {
            if right < length && heap[right] < heap[left] {
                heap.swap(index, right);
                index = right;
            } else {
                heap.swap(index, left);
                index = left;
            }
        } else if right < length && heap[right] < heap[index] {
            heap.swap(index, right);
            index = right;
        } else {
            // Neither child is smaller.
            return index;
        }
    }
    index
}

pub fn compare(a: f64, b: f64, c: f64, d: f64) -> f64 {
    let diff = a - b;
    if diff != 0.0 { diff } else { c - d }
}

pub fn clamped(n: u32) -> u32 {
    if n > 10 {
        return 10;
    }
    n
}

pub fn named(give: bool, name: String) -> Option<String> {
    if give { Some(name) } else { None }
}

pub fn first_big(items: &[u32]) -> u32 {
    for &item in items {
        if item > 10 {
            return item;
        }
    }
    0
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("      } else {\n        $swap(heap, index, left);");
  expect(js).toContain("    } else if (right < length && ");
  expect(js).toContain("    } else {\n      // Neither child is smaller.\n      return index;\n    }\n  }");
  expect(js).not.toContain("continue;");
  expect(js).toContain("    if (item > 10) {\n      return item;\n    }");
  // A tail `if` of two small values, each path's return: one conditional.
  expect(js).toContain("  return diff !== 0 ? diff : c - d;");
  // A guard at the function's end, and a branch that gives nothing: as written.
  expect(js).toContain("  if (n > 10) {\n    return 10;\n  }\n  return n;");
  expect(js).toContain("  if (give) {\n    return name;\n  }");
  const lib = await import(join(dir, "lib.js"));
  const heap = [5, 1, 2, 3];
  expect([lib.siftDown(heap, 0), heap, lib.firstBig([1, 20, 30]), lib.firstBig([1])]).toEqual([3, [1, 3, 2, 5], 20, 0]);
});
