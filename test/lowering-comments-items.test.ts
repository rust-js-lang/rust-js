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
  expect(js).toContain(`// A type's, which makes no import.
import { ownerName } from "./fiber.js";
// TODO: direct imports like some-package/src/* are bad. Fix me.
import { lane } from "./lanes.js";
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
    #[allow(unused_variables)]
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
  expect(tree.js).toContain(`// Of a module.
import * as Flags from "./common/flags.js";
// Both.
import { ownerName } from "./fiber.js";
import { lane } from "./lanes.js";
`);
  expect((tree.lib.named as () => string)()).toBe("App 3");
});

// The pilot's modules start with what each is for, `//! Where the app is:
// ..`, above their `use`s: a plain comment above the imports (ADR 0444's
// rule 3), but JS has no doc comment of what it's in, so it's JS's plain
// one, `// Where the app is: ..`, and a `/*! .. */` a `/* .. */`, above
// the imports or, with none, the code.
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
    lane()
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
  expect((await import(join(dir, "lib.js"))).named()).toBe(1);
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
