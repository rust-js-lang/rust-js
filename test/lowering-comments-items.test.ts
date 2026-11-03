// Where the compiler writes a comment among a module's items: before the
// next item that has JS, past one that has none, a `pub type`'s (ADR 0444,
// amended). Each test is a program of its own, compiled and run.

import { expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { compiler, fixture, run } from "./support";

/** `rust` compiled in a fixture of `name`: its JS and its module. */
async function compiled(name: string, rust: string): Promise<{ js: string; lib: Record<string, unknown> }> {
  const dir = fixture(name);
  writeFileSync(join(dir, "lib.rs"), rust);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  return { js, lib: await import(join(dir, "lib.js")) };
}

// React's ReactFiberConfigWithNoHydration heads a section, `// Hydration
// (when unsupported)`, whose first lines are types, then its constants:
// Flow's JS keeps the heading before `export const supportsHydration =
// false;`. A `pub type` writes no JS, so the comment was lost.
test("a comment before an item with no JS is before the next item that has", async () => {
  const { js, lib } = await compiled("comment-before-a-type", `pub fn first() -> u8 {
    1
}

// Hydration (when unsupported)
pub type ActivityInstance = u8;
#[allow(dead_code)]
pub(crate) type SuspenseInstance = u8;
pub const SUPPORTS_HYDRATION: bool = false;
`);
  expect(js).toContain("// Hydration (when unsupported)\nexport const SUPPORTS_HYDRATION = false;");
  expect(lib.SUPPORTS_HYDRATION).toBe(false);
});

// React's getAttributeAlias, `new Map([['acceptCharset', 'accept-charset'],
// // HTML and SVG attributes, .. ['crossOrigin', 'crossorigin'], ..])`: a
// comment between an array's items is before the item after it, as it's
// written, where it was lost, or before the statement (ADR 0708, amended).
test("a comment between an array's items is before the next item", async () => {
  const { js, lib } = await compiled("comment-between-items", `use std::collections::HashMap;

thread_local! {
    static ALIASES: HashMap<&'static str, &'static str> = HashMap::from([
        ("acceptCharset", "accept-charset"),
        // HTML and SVG attributes, but the SVG attribute is case sensitive.
        ("crossOrigin", "crossorigin"),
        ("xHeight", "x-height"), // after an item
    ]);
}

pub fn alias(name: &str) -> String {
    ALIASES.with(|aliases| aliases.get(name).copied().unwrap_or(name).to_string())
}

pub fn pairs(x: u8) -> Vec<(u8, u8)> {
    vec![
        (x, 1),
        // Before the second.
        (x, 2),
    ]
}
`);
  expect(js).toContain(`  ["acceptCharset", "accept-charset"],
  // HTML and SVG attributes, but the SVG attribute is case sensitive.
  ["crossOrigin", "crossorigin"],
  ["xHeight", "x-height"], // after an item
]);`);
  expect(js).toContain("  return [\n    [x, 1],\n    // Before the second.\n    [x, 2],\n  ];");
  expect([(lib.alias as (n: string) => string)("crossOrigin"), (lib.pairs as (x: number) => number[][])(3)]).toEqual(["crossorigin", [[3, 1], [3, 2]]]);
});

// React's SchedulerMock re-exports its priorities last, `pub use
// crate::scheduler_priorities::IDLE_PRIORITY as unstable_IdlePriority;`,
// as upstream's export list is. Printed first (ADR 0240), at where it's
// written, the comments of a block of only comments before it, an empty
// `else`'s and `function forceFrameRate() { // No-op }`'s, printed above
// it.
test("a re-export printed first takes no comment of a block before it", async () => {
  const { js, lib } = await compiled("reexport-takes-no-comment", `use std::cell::Cell;

pub mod priorities {
    pub const IDLE_PRIORITY: u8 = 5;
}

thread_local! {
    static SHOULD_YIELD: Cell<bool> = const { Cell::new(false) };
}

pub fn after_continuation() -> bool {
    if SHOULD_YIELD.get() {
        return true;
    } else {
        // We keep flushing synchronously.
    }
    false
}

pub fn force_frame_rate() {
    // No-op
}

/// The idle priority, by upstream's name.
pub use crate::priorities::IDLE_PRIORITY as UNSTABLE_IDLE_PRIORITY;
`);
  expect(js).toContain("export function forceFrameRate() {\n  // No-op\n}");
  // The `else`'s, in its function, as it is without the re-export.
  expect(js.indexOf("// We keep flushing synchronously.")).toBeGreaterThan(js.indexOf("export function afterContinuation"));
  expect(js).toContain('/** The idle priority, by upstream\'s name. */\nexport { IDLE_PRIORITY as UNSTABLE_IDLE_PRIORITY } from "./priorities.js";');
  expect([lib.UNSTABLE_IDLE_PRIORITY, (lib.afterContinuation as () => boolean)()]).toEqual([5, false]);
});

// m6-wife's P3: a module made only of re-exports, a package's entry, keeps
// its inner doc, `//!`, above them, and the blank line after it, as the
// file it ports has its header comment (rd620's header holds them).
test("a module of re-exports alone keeps its inner doc and the blank line after it", async () => {
  const { js, lib } = await compiled("reexports-alone-doc", `//! The client's entry.
//! It re-exports what its module makes.

pub use crate::shapes::area;
// Its side, a plain comment.

pub use crate::sides::side;

mod shapes {
    pub fn area(r: f64) -> f64 {
        r * r
    }
}

mod sides {
    pub fn side(a: f64) -> f64 {
        a.sqrt()
    }
}
`);
  expect(js).toContain('// The client\'s entry.\n// It re-exports what its module makes.\n\nexport { area } from "./shapes.js";\n');
  expect(js).toContain('\n// Its side, a plain comment.\n\nexport { side } from "./sides.js";\n');
  expect((lib.area as (r: number) => number)(3)).toBe(9);
});

// React's ReactInternalTypes and ReactFiberHooks document their types,
// `/// What useMemo calls for its value.\npub type MemoCreate = ..;`: a
// doc comment is its item's, which has no JS, so it was a stray `/** .. */`
// before the next item (41 in the port). A plain comment there still
// heads the next item that has JS.
test("a doc comment of an item with no JS is nowhere in the JS", async () => {
  const { js, lib } = await compiled("doc-of-a-type", `pub fn first() -> f64 {
    1.0
}

/// What \`useMemo\` calls for its value.
pub type MemoCreate = dyn Fn() -> f64;

fn mount_memo(next_create: &MemoCreate) -> f64 {
    next_create()
}

pub fn run() -> f64 {
    mount_memo(&|| 2.0)
}

/// What warns that a child in a list has no key.
#[allow(dead_code)]
type Warn = fn(f64);

// Hydration (when unsupported)
pub type Lanes = u32;

pub fn last() -> Lanes {
    3
}
`);
  expect(js).not.toContain("/**");
  expect(js).toContain("// Hydration (when unsupported)\n\nexport function last() {");
  expect([(lib.run as () => number)(), (lib.last as () => number)()]).toEqual([2, 3]);
});

// React's ReactDOMInput heads an import with a TODO, `// TODO: direct
// imports like some-package/src/* are bad. Fix me.`, above `import
// {getCurrentFiberOwnerNameInDevOrNull} from
// 'react-reconciler/src/ReactCurrentFiber';`. Each import is where the `use`
// that makes it is, so a comment between two `use`s is above the import
// the second makes, wherever the imports' order puts it; one above a `use`
// that makes none, a type's, is above the next import, or with none the
// first statement, the blank line after the `use`s between, as before
// a type alias. It was lost.
test("a comment above a use is above the import it makes", async () => {
  const { js, lib } = await compiled("comment-above-a-use", `pub mod fiber {
    pub fn owner_name() -> String {
        "App".to_string()
    }
}

pub mod lanes {
    pub fn lane() -> u32 {
        1
    }
}

use std::cell::Cell;
// TODO: direct imports like some-package/src/* are bad. Fix me.
use crate::lanes::lane;
// A type's, which makes no import.
use std::rc::Rc;
use crate::fiber::owner_name;

pub fn named() -> String {
    let count = Cell::new(lane());
    format!("{} {}", owner_name(), Rc::new(count.get()))
}
`);
  // In the `use`s' order (ADR 1063).
  expect(js).toContain(`// TODO: direct imports like some-package/src/* are bad. Fix me.
import { lane } from "./lanes.js";
// A type's, which makes no import.
import { ownerName } from "./fiber.js";
`);
  expect((lib.named as () => string)()).toBe("App 1");
  const alone = await compiled("comment-above-a-type-use", `use std::cell::Cell;
// TODO: direct imports like some-package/src/* are bad. Fix me.
use std::rc::Rc;

pub fn tag(value: &Cell<u32>) -> Rc<u32> {
    Rc::new(value.get())
}
`);
  expect(alone.js).toContain("// TODO: direct imports like some-package/src/* are bad. Fix me.\n\nexport function tag(value) {");
  // A binding's, imported from its JS file.
  const dir = fixture("comment-above-a-binding-use");
  writeFileSync(join(dir, "math.js"), "export const twice = (n) => n * 2;\n");
  writeFileSync(join(dir, "lib.rs"), `pub mod math {
    #[cfg_attr(rust_js, rust_js::link_name = "./math.js#twice")]
    pub fn twice(n: f64) -> f64 {
        unreachable!()
    }
}

use std::cell::Cell;
// Doubled by JS.
use crate::math::twice;

pub fn doubled(n: &Cell<f64>) -> f64 {
    twice(n.get())
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const bound = readFileSync(join(dir, "lib.js"), "utf8");
  expect(bound).toContain('// Doubled by JS.\nimport { twice } from "./math.js";\n');
  // One \`use\` of two, above the first printed; one of a module, above
  // what's imported of a module in it.
  const tree = await compiled("comment-above-a-use-tree", `pub mod fiber {
    pub fn owner_name() -> String {
        "App".to_string()
    }
}

pub mod lanes {
    pub fn lane() -> u32 {
        1
    }
}

pub mod common {
    pub mod flags {
        pub fn flag() -> u32 {
            2
        }
    }
}

use std::cell::Cell;
// Both.
use crate::{fiber::owner_name, lanes::lane};
// Of a module.
use crate::common as shared;

pub fn named() -> String {
    let count = Cell::new(lane() + shared::flags::flag());
    format!("{} {}", owner_name(), count.get())
}
`);
  expect(tree.js).toContain(`// Both.
import { ownerName } from "./fiber.js";
import { lane } from "./lanes.js";
// Of a module.
import * as Flags from "./common/flags.js";
`);
  expect((tree.lib.named as () => string)()).toBe("App 3");
});

// The pilot's modules start with what each is for, `//! Where the app is:
// ..`, above their `use`s: a plain comment above the imports (ADR 0444's
// rule 3), but JS has no doc comment of what it's in, so it's JS's plain
// one, `// Where the app is: ..`, and a `/*! .. */` a `/* .. */`, above
// the imports or, with none, the code, a blank line after it where the
// Rust has one.
test("a module's inner doc comment is a plain JS comment", async () => {
  const dir = fixture("inner-doc-above-imports");
  writeFileSync(join(dir, "lib.rs"), `//! Where the app is: its URL's hash,
//! followed as it changes.

use crate::lanes::lane;

pub mod lanes {
    /*! The lanes. */

    pub fn lane() -> u32 {
        1
    }
}

pub fn named() -> u32 {
    lane() + 1
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain(`// Where the app is: its URL's hash,
// followed as it changes.

import { lane } from "./lanes.js";
`);
  expect(js).not.toContain("//!");
  const lanes = readFileSync(join(dir, "lanes.js"), "utf8");
  expect(lanes).toContain("/* The lanes. */\n\nexport function lane() {");
  expect(lanes).not.toContain("/*!");
  expect((await import(join(dir, "lib.js"))).named()).toBe(2);
});

// #486: a module's `//!` is the module's, never an item's: above its JS,
// as a file's description is, its imports' too, though what it's above in
// the Rust has no JS, a trait or a struct, or an enum's methods first; a
// blank line after it where the Rust has one. A struct's own `///` stays
// its, nowhere in the JS.
test("a module's inner doc comment is above its JS", async () => {
  const dir = fixture("inner-doc-past-no-js");
  writeFileSync(join(dir, "lib.rs"), `//! A header comment, above a trait and its impl.
pub trait Letter {
    fn letter(&self) -> u8;
}

/// The struct's.
pub struct Upper;

impl Letter for Upper {
    fn letter(&self) -> u8 {
        b'A'
    }
}

pub mod lanes {
    /*! The lanes. */

    /// No JS.
    pub struct Lane;

    // Above the function.
    pub fn lane() -> u32 {
        let _lane = Lane;
        1
    }
}

pub fn show() -> String {
    Upper.letter().to_string()
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("// A header comment, above a trait and its impl.\n\nexport const upperLetter = {");
  expect(js.split("A header comment").length).toBe(2);
  expect(js).not.toContain("The lanes.");
  expect(js).not.toContain("The struct's.");
  expect((await import(join(dir, "lib.js"))).show()).toBe("65");
  const lanes = readFileSync(join(dir, "lanes.js"), "utf8");
  expect(lanes).toContain("/* The lanes. */\n\n// Above the function.\nexport function lane() {");
  expect(lanes).not.toContain("No JS.");
  // Above a type's `use`, which makes no import, and an enum's methods.
  const typed = fixture("inner-doc-past-a-type-use");
  writeFileSync(join(typed, "lib.rs"), `//! A type's \`use\` first.

//! Then the enum.

use crate::kinds::Count;
use crate::lanes::lane;

pub mod kinds {
    pub type Count = u32;
}

pub mod lanes {
    pub fn lane() -> u32 {
        1
    }
}

pub enum Shade {
    Light,
    Dark,
}

impl Shade {
    pub fn count(&self) -> Count {
        match self {
            Shade::Light => lane(),
            Shade::Dark => lane() + 1,
        }
    }
}
`);
  run([compiler, join(typed, "lib.rs"), "-o", join(typed, "lib.js")]);
  const doc = "// A type's `use` first.\n\n// Then the enum.\n\nimport { lane } from \"./lanes.js\";\n";
  expect(readFileSync(join(typed, "lib.js"), "utf8")).toContain(doc);
  // Right above its first `use`, no blank line between.
  const above = fixture("inner-doc-right-above-a-use");
  writeFileSync(join(above, "lib.rs"), `//! Right above.
use crate::lanes::lane;

pub mod lanes {
    pub fn lane() -> u32 {
        1
    }
}

pub fn named() -> u32 {
    //! A function's own, in its body.
    lane() * 2
}
`);
  run([compiler, join(above, "lib.rs"), "-o", join(above, "lib.js")]);
  const right = readFileSync(join(above, "lib.js"), "utf8");
  expect(right).toContain(`// Right above.\nimport { lane } from "./lanes.js";\n`);
  expect(right).toContain("  // A function's own, in its body.\n  return");
});

// React's EventRegistry.registerDirectEvent: a blank line, then
// `NAMES.with(|names| { .. })`, whose body is in its place: the blank line
// before its statement, as the Rust and upstream have it (rd44). A closure
// that stays one, `move || {`, takes none inside it.
test("a blank line before a closure whose body is in its place is before its body", async () => {
  const { js, lib } = await compiled("blank-before-a-closure-body", `use std::cell::RefCell;

thread_local! {
    static SEEN: RefCell<Vec<u32>> = const { RefCell::new(Vec::new()) };
}

pub fn register(n: u32) -> usize {
    if n > 9 {
        println!("big");
    }

    SEEN.with(|seen| {
        seen.borrow_mut().push(n);
    });
    let count = SEEN.with(|seen| seen.borrow().len());

    let shown = move || {
        println!("{n}");
    };
    shown();
    count
}
`);
  expect(js).toContain('    console.log("big");\n  }\n\n  seen.push(n);\n  const count = seen.length;\n\n  function shown() {\n    console.log(`${n}`);\n  }');
  expect((lib.register as (n: number) => number)(3)).toBe(1);
});

// A file's leading comments right above its code, no blank line between,
// are right above its JS too, of a module that imports nothing: React's
// generated InlineCodeStrings, `// This is a generated file.` right above
// its first `export` (#419).
test("leading comments right above the code are right above its JS", async () => {
  const { js, lib } = await compiled("header-on-code", `// This is a generated file.
// Run \`yarn generate\` to generate.
#[cfg_attr(rust_js, rust_js::name = "markShellTime")]
pub const MARK_SHELL_TIME: &str = r#"requestAnimationFrame(function(){$RT=performance.now()});"#;
`);
  expect(js).toContain("// This is a generated file.\n// Run `yarn generate` to generate.\nexport const markShellTime =");
  expect(lib.markShellTime).toBe("requestAnimationFrame(function(){$RT=performance.now()});");
});

// React's ReactFlightPerformanceTrack, `const trackNames = ['Primary',
// 'Parallel', 'Parallel​', // Padded with zero-width space ..`: a line
// comment after a `static` array's item, on its line, is after that item,
// which is where it's written in the map (rd344a); a `const`'s too.
test("a comment after an item of a static's array is after that item", async () => {
  const { js, lib } = await compiled("comment-after-a-static-item", `pub static TRACK_NAMES: [&str; 4] = [
    "Primary",
    "Parallel",
    "Parallel\\u{200b}", // Padded with zero-width space to give each track a unique name.
    "Parallel\\u{200b}\\u{200b}",
];

pub const SIZES: [u32; 3] = [
    1, // one
    2,
    3,
];
`);
  expect(js).toContain(
    '  "Parallel\\u200b", // Padded with zero-width space to give each track a unique name.\n  "Parallel\\u200b\\u200b",\n];',
  );
  expect(js).toContain("export const SIZES = [\n  1, // one\n  2,\n  3,\n];");
  expect(lib.TRACK_NAMES).toEqual(["Primary", "Parallel", "Parallel​", "Parallel​​"]);
  expect(lib.SIZES).toEqual([1, 2, 3]);
});

// React's Flight client, `fn(.., [''], // path`: a line comment after an
// argument that's an array of constants, which rustc promotes to one, the
// map placing it at its `]`, is after that argument, not the call's `)`
// (rd525).
test("a comment after a promoted array argument is after it", async () => {
  const { js, lib } = await compiled("comment-after-a-promoted-argument", `pub fn visit(id: u32, path: &[&str], name: &str, deep: bool) -> usize {
    id as usize + path.len() + name.len() + usize::from(deep)
}

pub fn walk(id: u32) -> usize {
    visit(
        id,
        &[""], // path
        "root", // name
        true,
    )
}
`);
  expect(js).toContain('    id,\n    [""], // path\n    "root", // name\n    true,\n  );');
  expect((lib.walk as (id: number) => number)(1)).toBe(7);
});

// The same of an array that isn't promoted, `&[key], // path`: made where
// it's written, its JS is there, as a value a statement makes is, so the
// map has its line's JS to put the comment after; it had none, its line's
// JS carried the argument before it's place, `id`'s, and the comment was
// lost.
test("a comment after an array argument made of a variable is after it", async () => {
  const { js, lib } = await compiled("comment-after-an-array-argument", `pub fn visit(id: u32, path: &[&str], name: &str, deep: bool) -> usize {
    id as usize + path.len() + path[0].len() + name.len() + usize::from(deep)
}

pub fn walk(id: u32, key: &str) -> usize {
    visit(
        id,
        &[key], // path
        "root", // name
        true,
    )
}
`);
  expect(js).toContain('    id,\n    [key], // path\n    "root", // name\n    true,\n  );');
  expect((lib.walk as (id: number, key: string) => number)(1, "ab")).toBe(9);
});
