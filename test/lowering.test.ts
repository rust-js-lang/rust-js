// What the compiler writes of a Rust program's drops and destructors, and what
// it refuses from MIR, with a match read as a table by its key; the rest of
// lowering is in the lowering-*.test.ts files by concept. Each test is a
// program of its own, compiled and run.

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

// ADR 0489: an iterator's items no caller gives a drop for drop nothing,
// as a type parameter's none's given for: `counted(0..3)`'s `count()` is
// `(n) => n + 1`, not a hook for a drop no dictionary has.
test("a generic count of items no caller gives a drop for drops none", () => {
  const dir = fixture("ungiven-item-slot");
  writeFileSync(join(dir, "lib.rs"), readFileSync(join(root, "test/corpus/yielded_item_drops.rs"), "utf8").replace("fn main()", "pub fn main()"));
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("return $given(IClone.clone(iter), IIterator, true).reduce((n) => n + 1, 0);");
  expect(js).not.toContain("IIterator.$dropItem");
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

// A value with a destructor moved before anything in its scope can leave,
// `hold(l)`'s into what it returns, is never the scope's to drop: no flag,
// no `try` (ADR 0197), as react.dev's ExternalLink moves its children. One
// moved after what may panic keeps them.
test("a value moved before anything can leave needs no drop of its scope", async () => {
  const dir = fixture("moved-first");
  writeFileSync(join(dir, "lib.rs"), 'pub struct Loud(pub u32);\nimpl Drop for Loud {\n    fn drop(&mut self) {\n        println!("drop {}", self.0);\n    }\n}\npub struct Holder {\n    pub l: Loud,\n}\npub fn hold(l: Loud) -> Holder {\n    Holder { l }\n}\npub fn checked(l: Loud, n: u32) -> Holder {\n    assert!(n > 0);\n    Holder { l }\n}\n');
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("export function hold(l) {\n  return { l };\n}");
  // Dropped where the assertion fails, then the panic: no flag, as it's
  // known there, and no \`try\` of a \`throw\` (ADR 0372); its one drop its
  // body (ADR 0589).
  expect(js).toContain('  if (n <= 0) {\n    console.log(`drop ${l[0]}`);\n    throw new Error("assertion failed: n > 0");\n  }\n  return { l };');
  const lib = await import(join(dir, "lib.js"));
  expect([lib.hold([1]).l[0], lib.checked([2], 1).l[0]]).toEqual([1, 2]);
});

// ADR 0589: a \`Drop\` impl's drop is its body where it's dropped, as React's
// scheduler's \`flushWork\` restores its state in a \`finally\`, at each drop,
// a closure's too; the guard made only to be dropped, read only by its
// fields, is its fields, \`let\`s where they're written; a one-field tuple's,
// its field. A guard read whole stays one.
test("a Drop dropped once is its body where it's dropped", async () => {
  const dir = fixture("drop-dropped-once");
  writeFileSync(join(dir, "lib.rs"), `use std::cell::Cell;

thread_local! {
    static DEPTH: Cell<u32> = const { Cell::new(0) };
}

struct Restore {
    previous: u32,
}

impl Drop for Restore {
    fn drop(&mut self) {
        DEPTH.set(self.previous);
    }
}

fn work(n: u32) -> u32 {
    if n > 9 {
        panic!("too deep");
    }
    n + DEPTH.get()
}

fn note(n: u32) {
    println!("depth {n}");
}

pub fn run(n: u32) -> u32 {
    let _restore = Restore { previous: DEPTH.get() };
    DEPTH.set(n);
    note(n);
    work(n)
}

pub fn current() -> u32 {
    DEPTH.get()
}

pub struct Loud {
    name: &'static str,
}

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.name);
    }
}

pub fn twice() {
    let _a = Loud { name: "a" };
    let _b = Loud { name: "b" };
}

struct Seen {
    number: u32,
}

impl Drop for Seen {
    fn drop(&mut self) {
        println!("seen {}", self.number);
    }
}

fn show(seen: &Seen) -> u32 {
    seen.number
}

pub fn shown(id: u32) -> u32 {
    let seen = Seen { number: id };
    show(&seen)
}

pub mod lock {
    pub struct Held {
        pub name: &'static str,
    }

    impl Drop for Held {
        fn drop(&mut self) {
            println!("released {}", self.name);
        }
    }

    pub fn here() {
        let _held = Held { name: "here" };
    }
}

pub fn there() {
    let _held = lock::Held { name: "there" };
}

struct Level(u32);

impl Drop for Level {
    fn drop(&mut self) {
        DEPTH.set(self.0);
    }
}

pub fn wrapped(n: u32) -> impl Fn() -> u32 {
    move || {
        let _level = Level(DEPTH.get());
        DEPTH.set(n);
        work(n)
    }
}

struct More {
    more: bool,
}

impl Drop for More {
    fn drop(&mut self) {
        println!("more {}", self.more);
    }
}

pub fn flushed(n: u32) {
    let mut guard = More { more: true };
    guard.more = work(n) > 3;
}

struct Twice(u32);

impl Drop for Twice {
    fn drop(&mut self) {
        let doubled = self.0 * 2;
        println!("doubled {doubled}");
    }
}

pub fn doubles(doubled: u32) {
    let _twice = Twice(doubled);
}

pub fn doubles_too() {
    let _twice = Twice(4);
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("export function run(n) {\n  const previous = depth;\n  depth = n;\n  try {\n    note(n);\n    return work(n);\n  } finally {\n    depth = previous;\n  }\n}");
  expect(js).not.toContain("restoreDrop_drop");
  expect(js).not.toContain("loudDrop_drop");
  // Where one drop can't be its body, its \`doubled\` the caller's, each
  // calls the function.
  expect(js).toContain("twiceDrop_drop(_twice)");
  expect(js).toContain("  return () => {\n    const level = depth;\n    depth = n;\n    try {\n      return work(n);\n    } finally {\n      depth = level;\n    }\n  };");
  expect(js).toContain("  let more = true;\n  try {\n    more = work(n) > 3;\n  } finally {\n    console.log(`more ${more}`);\n  }");
  expect(js).toContain('const name = "b";\n  try {\n    console.log(`drop ${name}`);\n  } finally {\n    console.log(`drop ${_a.name}`);\n  }');
  expect(js).not.toContain("seenDrop_drop");
  expect(js).toContain("const seen = { number: id };");
  const lib = await import(join(dir, "lib.js"));
  expect([lib.run(2), lib.current()]).toEqual([4, 0]);
  lib.doubles(3);
  lib.doublesToo();
  expect(() => lib.run(10)).toThrow("too deep");
  expect(lib.current()).toBe(0);
  expect(lib.shown(3)).toBe(3);
  // Dropped once in its module and once in another: a function both call.
  expect(readFileSync(join(dir, "lock.js"), "utf8")).toContain("export function heldDrop_drop(held)");
  lib.there();
  (await import(join(dir, "lock.js"))).here();
});

// What isn't a global's binding, a closure or an import, has no global to
// ask of.
test("js::is_defined of what isn't a global's binding is refused", () => {
  const dir = fixture("defined-refused");
  const refused = (code: string) => {
    writeFileSync(join(dir, "lib.rs"), code);
    const failed = Bun.spawnSync([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target], { stderr: "pipe" });
    return [failed.exitCode === 0, failed.stderr.toString()];
  };
  const message = expect.stringContaining("`js::is_defined` of what isn't a JS global's binding");
  expect(refused("pub fn ask() -> bool {\n    js::is_defined(|| 1)\n}\n")).toEqual([false, message]);
  // The crate's own function, which is there.
  expect(refused("fn helper() {}\n\npub fn ask() -> bool {\n    helper();\n    js::is_defined(helper)\n}\n")).toEqual([false, message]);
  // A variable's, or the crate's own static's, which are there.
  expect(refused("pub fn ask() -> bool {\n    let n = 1;\n    js::is_defined(&n)\n}\n")).toEqual([false, message]);
  expect(refused("static LIMIT: u32 = 3;\n\npub fn ask() -> bool {\n    js::is_defined(LIMIT)\n}\n")).toEqual([false, message]);
  // A static is given as itself, as a function binding is (ADR 0645).
  expect(refused("pub fn ask() -> bool {\n    js::is_defined(&js::symbol::iterator)\n}\n")).toEqual([false, expect.stringContaining("given the static itself, `js::is_defined(webapi::performance)`")]);
  // A method of a type two globals hold, which says neither.
  expect(refused(`pub struct Twin(std::marker::PhantomData<js::JsObject>);

impl Twin {
    #[cfg_attr(rust_js, rust_js::link_name = "tick")]
    pub fn tick(&self) {
        unreachable!()
    }
}

unsafe extern "Rust" {
    #[link_name = "rustJsLeft"]
    pub safe static LEFT: &'static Twin;
    #[link_name = "rustJsRight"]
    pub safe static RIGHT: &'static Twin;
}

pub fn ask() -> bool {
    js::is_defined(Twin::tick)
}
`)).toEqual([false, message]);
  // An import, which is there or its module doesn't load.
  expect(refused(`#[cfg_attr(rust_js, rust_js::link_name = "node:path#join")]
pub fn join(a: &str, b: &str) -> String {
    unreachable!()
}

pub fn ask() -> bool {
    js::is_defined(join)
}
`)).toEqual([false, message]);
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
