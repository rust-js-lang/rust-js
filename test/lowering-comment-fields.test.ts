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
    pub v: Option<u32>,
    #[cfg_attr(rust_js, rust_js::nullable)]
    pub o: Cell<Option<u32>>,
    pub p: Option<u32>,
}

pub fn create_cache_node() -> CacheNode {
    CacheNode {
        s: Cell::new(0), // status
        v: None, // value, either the cached result or an error, depending on s
        o: Cell::new(None), // object cache
        p: None,
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
    pub current_value: Option<u32>,
    #[cfg_attr(rust_js, rust_js::name = "_currentValue2")]
    pub current_value2: Option<u32>,
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
        current_value: None,
        current_value2: None,
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
    pub store: Option<u32>, // 0: not validated, 1: validated, 2: force fail
}

pub fn element(props: u32, owner: u32) -> Element {
    Element {
        props,
        // Record the component responsible for creating this element.
        owner,
        r#ref: None,
        store: None,
    }
}

pub fn last(props: u32) -> Element {
    Element {
        props,
        owner: 0,
        r#ref: None,
        // Set where it's validated.
        store: None,
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

// React's getSuspendedCache: `return {\n // We must also save the parent ..\n
// parent: isPrimaryRenderer ? CacheContext._currentValue : ..`, after a
// blank line. A comment before a field whose value is an `if`, or a method
// call on what a thread-local's `with` gives, is the field's, where the
// `if`'s conditional or the call is read, and the blank line before the
// literal its statement's (ADR 0708, amended).
test("a comment before a field of an if's value or a method call's is the field's", async () => {
  const { js, lib } = await compiled("commented-if-field", `use std::cell::RefCell;
use std::rc::Rc;

pub const IS_PRIMARY_RENDERER: bool = true;

pub struct Context {
    pub current_value: RefCell<Option<Rc<String>>>,
    pub current_value2: RefCell<Option<Rc<String>>>,
}

thread_local! {
    static CACHE_CONTEXT: Context = Context {
        current_value: RefCell::new(Some(Rc::new("a".to_string()))),
        current_value2: RefCell::new(None),
    };
}

pub struct Pooled {
    pub parent: Rc<String>,
    pub pool: Rc<String>,
}

fn peek(x: u32) -> Option<Rc<String>> {
    if x > 1 {
        Some(Rc::new(x.to_string()))
    } else {
        None
    }
}

pub fn suspended(x: u32) -> Option<Pooled> {
    let pool = peek(x)?;

    Some(Pooled {
        // We must also save the parent, so that when we resume we can detect
        // a refresh.
        parent: CACHE_CONTEXT
            .with(|context| {
                if IS_PRIMARY_RENDERER {
                    context.current_value.borrow().clone()
                } else {
                    context.current_value2.borrow().clone()
                }
            })
            .unwrap(),
        pool,
    })
}

pub struct Pair {
    pub first: u32,
    pub second: u32,
}

pub fn pair(x: u32, a: u32, b: u32) -> Pair {
    let first = x + 1;

    Pair {
        first,
        // Which one.
        second: if x > 2 { a } else { b },
    }
}
`);
  expect(js).toContain(`  if (pool == null) {
    return;
  }

  return {
    // We must also save the parent, so that when we resume we can detect
    // a refresh.
    parent: $unwrap(`);
  expect(js).toContain(`  const first = (x + 1) >>> 0;

  return {
    first,
    // Which one.
    second: x > 2 ? a : b,
  };`);
  expect([(lib.suspended(2 as never) as { parent: string }).parent, lib.suspended(1 as never)]).toEqual(["a", undefined]);
  expect(lib.pair(3 as never, 5 as never, 6 as never)).toEqual({ first: 4, second: 5 });
});
