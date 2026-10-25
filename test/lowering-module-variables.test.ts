// What an exported thread-local's cell is to the modules that import it: its
// module's `export let`, written only there and read live by its importers,
// JS's and Rust's, in its crate and in another; a write from another module
// refused (ADR 0270, amended). Each test compiles a program and runs it.

import { beforeAll, expect, test } from "bun:test";
import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { runSync } from "./child";
import { buildReact, compiler, fixture, root, run, target } from "./support";

beforeAll(buildReact, 600_000);

const js = ["--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];

const flags = `use std::cell::{Cell, RefCell};

thread_local! {
    pub static ENABLED: Cell<bool> = const { Cell::new(false) };
    pub(crate) static COUNT: Cell<u32> = const { Cell::new(0) };
    pub static LIMIT: Cell<u32> = const { Cell::new(4) };
    pub static LABEL: RefCell<String> = const { RefCell::new(String::new()) };
    pub static SEEN: RefCell<Vec<u32>> = const { RefCell::new(Vec::new()) };
}

pub fn enable(label: &str) {
    ENABLED.set(true);
    COUNT.set(COUNT.get() + 1);
    LABEL.set(label.to_string());
}
`;

// React's BadMapPolyfill exports `let hasBadMapPolyfill`, which its module
// sets as it loads and its importers read as it is now, ES's live binding:
// a `pub` thread-local's cell only its module sets is that `export let`, an
// importer's read of it its name, and one nothing sets `export const`.
test("an exported cell its module alone sets is its export let, which importers read", async () => {
  const dir = fixture("exported-cell-let");
  writeFileSync(join(dir, "lib.rs"), "pub mod flags;\npub mod users;\n");
  writeFileSync(join(dir, "flags.rs"), flags);
  writeFileSync(join(dir, "users.rs"), `use crate::flags::{COUNT, ENABLED, LABEL, LIMIT};

pub fn told() -> (bool, u32, u32, String) {
    (ENABLED.get(), COUNT.get(), LIMIT.get(), LABEL.with_borrow(|label| label.clone()))
}
`);
  run([compiler, "--declarations", join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", ...js]);
  const flagsJs = readFileSync(join(dir, "flags.js"), "utf8");
  expect(flagsJs).toContain("export let enabled = false;\nexport let count = 0;\nexport const limit = 4;\nexport let label = \"\";\n");
  expect(flagsJs).toContain("  enabled = true;\n  count = (count + 1) >>> 0;\n  label = label$1;\n");
  expect(flagsJs).not.toContain(".value");
  const usersJs = readFileSync(join(dir, "users.js"), "utf8");
  expect(usersJs).toContain("  return [enabled, count, limit, label];\n");
  expect(readFileSync(join(dir, "flags.d.ts"), "utf8")).toContain("export let enabled: boolean;\n\nexport const limit: number;\n\nexport let label: string;\n");
  // A JS module of the program's imports it, as React's JS imports
  // `hasBadMapPolyfill`, and reads what it is now.
  writeFileSync(join(dir, "reader.js"), 'import { enabled, label } from "./flags.js";\nexport const read = () => [enabled, label];\n');
  const reader = await import(join(dir, "reader.js"));
  const flagsModule = await import(join(dir, "flags.js"));
  const users = await import(join(dir, "users.js"));
  expect(reader.read()).toEqual([false, ""]);
  flagsModule.enable("on");
  expect(reader.read()).toEqual([true, "on"]);
  expect(users.told()).toEqual([true, 1, 4, "on"]);
});

// JS can't set another module's `let`: a write of an exported cell from
// another module, `set`, `replace`, `take`, a `with_borrow_mut`, or its cell
// handed out, `with`, is refused, saying to write a function of its module
// that sets it, as React's ReactCurrentFiber exports `setIsRendering`.
test("another module's write of an exported cell is refused", () => {
  const dir = fixture("exported-cell-written");
  const refused = (name: string, body: string) => {
    mkdirSync(join(dir, name), { recursive: true });
    writeFileSync(join(dir, name, "lib.rs"), "pub mod flags;\npub mod users;\n");
    writeFileSync(join(dir, name, "flags.rs"), flags);
    writeFileSync(join(dir, name, "users.rs"), `use crate::flags;

pub fn written() -> u32 {
    ${body}
}
`);
    return runSync([compiler, join(dir, name, "lib.rs"), "-o", join(dir, name, "lib.js"), "--", ...js], root, 120_000);
  };
  for (const [name, body] of [
    ["set", "flags::ENABLED.set(true);\n    0"],
    ["replace", "flags::COUNT.replace(3)"],
    ["take", "flags::COUNT.take()"],
    ["borrow-mut", "flags::LABEL.with_borrow_mut(|label| label.push('!'));\n    0"],
    ["with", "flags::ENABLED.with(|enabled| enabled.set(true));\n    0"],
  ]) {
    const out = refused(name, body);
    expect([name, out.code === 0, out.stderr]).toEqual([
      name,
      false,
      expect.stringContaining("is its module's variable, which JS lets only that module set: write a `pub fn` in that module that sets it"),
    ]);
  }
  // A read is a read of its variable; a `RefCell` of an array is that
  // array, which any module may change, as JS's importer of it may.
  const read = refused("read", "flags::SEEN.with_borrow_mut(|seen| seen.push(1));\n    flags::COUNT.get() + u32::from(flags::ENABLED.get())");
  expect([read.code, read.stderr]).toEqual([0, ""]);
});

// A library's exported cell is its `export let` to its consumers too, whose
// manifest says so: a consumer reads it by its name and can't set it.
test("a library's exported cell is a variable its consumers read and can't set", async () => {
  const dir = fixture("exported-cell-library");
  const manifest = join(dir, "dep", "lib.manifest.json"), metadata = join(dir, "libdep.rmeta");
  writeFileSync(join(dir, "dep.rs"), `use std::cell::{Cell, RefCell};

thread_local! {
    pub static HAS_BAD_MAP_POLYFILL: Cell<bool> = const { Cell::new(true) };
    pub static NAME: RefCell<String> = const { RefCell::new(String::new()) };
}

js::on_load! {
    HAS_BAD_MAP_POLYFILL.set(false);
}

pub fn mark() {
    HAS_BAD_MAP_POLYFILL.set(true);
    NAME.set("marked".to_string());
}
`);
  run([compiler, join(dir, "dep.rs"), "-o", join(dir, "dep", "lib.js"), "--library", "--manifest", manifest,
    "--", "--crate-name", "dep", `--emit=metadata=${metadata}`, ...js]);
  expect(readFileSync(join(dir, "dep", "lib.js"), "utf8")).toContain("export let hasBadMapPolyfill = true;\nexport let name = \"\";\n\nhasBadMapPolyfill = false;\n");
  const app = (name: string, body: string) => {
    mkdirSync(join(dir, name), { recursive: true });
    writeFileSync(join(dir, name, "lib.rs"), `pub fn bad() -> bool {\n    ${body}\n}\n`);
    return runSync([compiler, join(dir, name, "lib.rs"), "-o", join(dir, name, "lib.js"), "--dependency", manifest,
      "--", "--crate-name", name, "--extern", `dep=${metadata}`, "-L", dir, ...js], root, 120_000);
  };
  const reads = app("reads", "dep::HAS_BAD_MAP_POLYFILL.get() && dep::NAME.with_borrow(|name| !name.is_empty())");
  expect([reads.code, reads.stderr]).toEqual([0, ""]);
  const readsJs = readFileSync(join(dir, "reads", "lib.js"), "utf8");
  expect(readsJs).toContain("  return hasBadMapPolyfill && !!name;\n");
  const { bad } = await import(join(dir, "reads", "lib.js"));
  const { mark } = await import(join(dir, "dep", "lib.js"));
  expect(bad()).toBe(false);
  mark();
  expect(bad()).toBe(true);
  // A `with_borrow` that runs what may borrow it, the library's own code,
  // can't be read so: nothing counts its borrows, where Rust's would panic.
  const unquiet = app("unquiet", "dep::NAME.with_borrow(|name| {\n        dep::mark();\n        name.is_empty()\n    })");
  expect([unquiet.code === 0, unquiet.stderr]).toEqual([false, expect.stringContaining("is its module's variable, which JS lets only that module set")]);
  const sets = app("sets", "dep::HAS_BAD_MAP_POLYFILL.set(false);\n    true");
  expect([sets.code === 0, sets.stderr]).toEqual([false, expect.stringContaining("is its module's variable, which JS lets only that module set")]);
});

// React's BadMapPolyfill, as its port writes it: upstream's JS.
test("React's BadMapPolyfill is its export let", () => {
  const dir = fixture("bad-map-polyfill");
  writeFileSync(join(dir, "lib.rs"), `use std::cell::Cell;
use std::panic::catch_unwind;

use js::Kind;

unsafe extern "Rust" {
    pub safe static __DEV__: bool;
}

thread_local! {
    pub static HAS_BAD_MAP_POLYFILL: Cell<Option<bool>> = const { Cell::new(None) };
}

js::on_load! {
    if __DEV__ {
        HAS_BAD_MAP_POLYFILL.set(Some(false));
        if let Err(_e) = catch_unwind(|| {
            let frozen_object = js::object::freeze(js::dict::new::<Kind>());
            /* eslint-disable no-new */
            js::map::new(Some(&[(frozen_object, Kind::Null)]));
            js::set::new(Some(&[frozen_object]));
            /* eslint-enable no-new */
        }) {
            // TODO: Consider warning about bad polyfills
            HAS_BAD_MAP_POLYFILL.set(Some(true));
        }
    }
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", ...js]);
  expect(readFileSync(join(dir, "lib.js"), "utf8")).toContain(`export let hasBadMapPolyfill;

if (__DEV__) {
  hasBadMapPolyfill = false;
  try {
    const frozenObject = Object.freeze({});
    /* eslint-disable no-new */
    new Map([[frozenObject, null]]);
    new Set([frozenObject]);
    /* eslint-enable no-new */
  } catch (_e) {
    // TODO: Consider warning about bad polyfills
    hasBadMapPolyfill = true;
  }
}
`);
});
