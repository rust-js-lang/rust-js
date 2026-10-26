// Where the compiler writes a comment on a struct literal's field: a
// `None`'s key kept for it, `undefined`, the comment's place (ADR 0280,
// amended). Each test is a program of its own, compiled and run.

import { beforeAll, expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { buildReact, compiler, fixture, run, target } from "./support";

beforeAll(buildReact, 600_000);

/** `rust` compiled in a fixture of `name`, against the js crate: its JS
 * and its module. */
async function compiled(name: string, rust: string): Promise<{ js: string; lib: Record<string, (...args: never[]) => unknown> }> {
  const dir = fixture(name);
  writeFileSync(join(dir, "lib.rs"), rust);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  return { js, lib: await import(join(dir, "lib.js")) };
}

// React's createCacheNode: `v: undefined, // value, either the cached
// result or an error, depending on s`. A `None` field is no key (ADR 0280),
// but one a comment is written on keeps it, `undefined`, the comment's
// place, after it or before it (amended); one without stays out.
test("a None field a comment is written on keeps its key and its comment", async () => {
  const { js, lib } = await compiled("commented-none-field", `use std::cell::Cell;

pub struct CacheNode {
    pub s: Cell<u32>,
    pub v: Cell<Option<u32>>,
    #[cfg_attr(rust_js, rust_js::nullable)]
    pub o: Cell<Option<u32>>,
    pub p: Cell<Option<u32>>,
}

pub fn create_cache_node() -> CacheNode {
    CacheNode {
        s: Cell::new(0), // status
        v: Cell::new(None), // value, either the cached result or an error, depending on s
        o: Cell::new(None), // object cache
        p: Cell::new(None),
    }
}

pub fn led() -> CacheNode {
    CacheNode {
        s: Cell::new(0),
        // a value, before it
        v: Cell::new(None),
        o: Cell::new(None),
        p: Cell::new(None),
    }
}

pub fn ended() -> CacheNode {
    CacheNode {
        // these at the root
        v: Cell::new(None),
        s: Cell::new(0),
        p: Cell::new(None),
        o: Cell::new(None),
    }
}

pub fn parted() -> CacheNode {
    CacheNode {
        s: Cell::new(0),
        // these at the root
        v: Cell::new(None),

        p: Cell::new(None),
        o: Cell::new(None),
    }
}
`);
  expect(js).toContain(`  return {
    s: 0, // status
    v: undefined, // value, either the cached result or an error, depending on s
    o: null, // object cache
  };`);
  // A comment before a field speaks of the \`None\`s written right after
  // it too, as React's CacheContext's \`// We'll initialize these at the
  // root.\` does of \`_currentValue\` and \`_currentValue2\` (amended again),
  // up to another field or a blank line.
  expect(js).toContain(`  return {
    s: 0,
    // a value, before it
    v: undefined,
    o: null,
    p: undefined,
  };`);
  expect(js).toContain(`  return {
    // these at the root
    v: undefined,
    s: 0,
    o: null,
  };`);
  expect(js).toContain(`  return {
    s: 0,
    // these at the root
    v: undefined,
    o: null,
  };`);
  const keys = [lib.createCacheNode(), lib.led(), lib.ended(), lib.parted()].map((made) => Object.keys(made as object));
  expect(keys).toEqual([["s", "v", "o"], ["s", "v", "o", "p"], ["v", "s", "o"], ["s", "v", "o"]]);
});
