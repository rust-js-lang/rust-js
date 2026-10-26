// Where the compiler writes a comment on a struct literal's field, and
// which `None` keys it keeps for one: a comment after a value on its line
// keeps it, `undefined`, its only place (ADR 0280, amended); one on lines
// of their own before a field is before the next key, which keys there are
// as without it (amended again). Each test is a program of its own,
// compiled and run.

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
// but one a comment is written after on its line keeps it, `undefined`, the
// comment's place (amended); one without stays out.
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
`);
  expect(js).toContain(`  return {
    s: 0, // status
    v: undefined, // value, either the cached result or an error, depending on s
    o: null, // object cache
  };`);
  expect(Object.keys(lib.createCacheNode() as object)).toEqual(["s", "v", "o"]);
});

// A comment on lines of their own before a field heads what follows: it
// changes no key (amended again). React's createContext, `// These are
// circular` before `Provider` and `Consumer`, leaves out its DEV-only
// `_currentRenderer`s, as upstream sets them only in DEV; ReactElement's
// `// Record the component responsible for creating this element.` before
// `_owner` leaves out `_store`; and ReactFiberCacheComponent's `// We'll
// initialize these at the root.` before two `None`s writes them alike,
// the comment before `_threadCount`. Where no key follows, the `None` it's
// before keeps its key, the comment's place.
test("a comment before a field changes no key but where it has no other place", async () => {
  const { js, lib } = await compiled("comment-before-fields", `use std::cell::Cell;

pub struct Context {
    #[cfg_attr(rust_js, rust_js::name = "_currentValue")]
    pub current_value: Cell<Option<u32>>,
    #[cfg_attr(rust_js, rust_js::name = "_currentValue2")]
    pub current_value2: Cell<Option<u32>>,
    #[cfg_attr(rust_js, rust_js::name = "_threadCount")]
    pub thread_count: Cell<f64>,
    #[cfg_attr(rust_js, rust_js::name = "Provider")]
    pub provider: Cell<u32>,
    #[cfg_attr(rust_js, rust_js::name = "Consumer")]
    #[cfg_attr(rust_js, rust_js::nullable)]
    pub consumer: Cell<Option<u32>>,
    #[cfg_attr(rust_js, rust_js::name = "_currentRenderer")]
    #[cfg_attr(rust_js, rust_js::nullable, rust_js::optional)]
    pub current_renderer: Cell<Option<u32>>,
    #[cfg_attr(rust_js, rust_js::name = "_currentRenderer2")]
    #[cfg_attr(rust_js, rust_js::nullable, rust_js::optional)]
    pub current_renderer2: Cell<Option<u32>>,
}

pub fn create_context() -> Context {
    Context {
        // We'll initialize these at the root.
        current_value: Cell::new(None),
        current_value2: Cell::new(None),
        thread_count: Cell::new(0.0),
        // These are circular
        provider: Cell::new(1),
        consumer: Cell::new(None),
        current_renderer: Cell::new(None),
        current_renderer2: Cell::new(None),
    }
}

pub struct Element {
    pub props: u32,
    #[cfg_attr(rust_js, rust_js::name = "_owner")]
    pub owner: u32,
    #[cfg_attr(rust_js, rust_js::nullable)]
    pub r#ref: Option<u32>,
    #[cfg_attr(rust_js, rust_js::name = "_store")]
    pub store: Cell<Option<u32>>, // 0: not validated, 1: validated, 2: force fail
}

pub fn element(props: u32, owner: u32) -> Element {
    Element {
        props,
        // Record the component responsible for creating this element.
        owner,
        r#ref: None,
        store: Cell::new(None),
    }
}

pub fn last(props: u32) -> Element {
    Element {
        props,
        owner: 0,
        r#ref: None,
        // Set where it's validated.
        store: Cell::new(None),
    }
}
`);
  expect(js).toContain(`  return {
    // We'll initialize these at the root.
    _threadCount: 0,
    // These are circular
    Provider: 1,
    Consumer: null,
  };`);
  expect(js).toContain(`  return {
    props,
    // Record the component responsible for creating this element.
    _owner: owner,
    ref: null,
  };`);
  expect(js).toContain(`    ref: null,
    // Set where it's validated.
    _store: undefined,
  };`);
  const keys = [lib.createContext(), lib.element(1 as never, 2 as never), lib.last(1 as never)].map((made) => Object.keys(made as object));
  expect(keys).toEqual([["_threadCount", "Provider", "Consumer"], ["props", "_owner", "ref"], ["props", "_owner", "ref", "_store"]]);
});
