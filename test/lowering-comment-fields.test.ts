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
// the comment before `_threadCount`. Where no key follows, it's before the
// object's `}`, the `None` left out still (rd321).
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
  };`);
  const keys = [lib.createContext(), lib.element(1 as never, 2 as never), lib.last(1 as never)].map((made) => Object.keys(made as object));
  expect(keys).toEqual([["_threadCount", "Provider", "Consumer"], ["props", "_owner", "ref"], ["props", "_owner", "ref"]]);
});

// React's Flight client's stream controller, `{ .., error(error) { .. } else
// { .. // We shouldn't get any more enqueues ..\n previous.then(..) } }`: a
// comment inside a field's value, a method's body, is its code's, not one
// after the object's fields, though no key follows it (rd321).
test("a comment inside a field's value stays there", async () => {
  const { js, lib } = await compiled("comment-inside-field-value", `pub struct Controller {
    pub close: Box<dyn Fn(u32) -> u32>,
}

pub fn controller(base: u32) -> Controller {
    Controller {
        close: Box::new(move |n| {
            let m = n + base;
            // We shouldn't get any more after this.
            m * 2
        }),
    }
}

pub fn closed(n: u32) -> u32 {
    (controller(1).close)(n)
}
`);
  expect(js).toMatch(/const m = .*;\n\s*\/\/ We shouldn't get any more after this\.\n\s*return /);
  expect(js).not.toMatch(/\n\s*\/\/ We shouldn't get any more after this\.\n\s*};/);
  expect(lib.closed(2 as never)).toBe(6);
});

// React's ReactFiberThenable makes its thenable's `ReactIOInfo` with `// We
// don't know the requesting owner nor stack.` before `owner: None`, and
// `None`s after it, each `#[rust_js::nullable, rust_js::optional]`: absent
// until set (ADR 0810). A comment is no reason for a key: each `None` is
// left out as without it, and the comment, after the last key written, is
// before the object's `}` (rd321). Before, the last `None` it led was
// written, `owner: undefined`, so `"owner" in ioInfo`, which the program
// asks itself, was true for a comment's sake.
test("a comment before a last None is before the object's closing brace, the None left out", async () => {
  const { js, lib } = await compiled("comment-before-last-none", `pub struct ReactIOInfo {
    pub name: String,
    pub start: f64,
    #[cfg_attr(rust_js, rust_js::nullable, rust_js::optional)]
    pub env: Option<String>,
    #[cfg_attr(rust_js, rust_js::nullable, rust_js::optional)]
    pub owner: Option<u32>,
    #[cfg_attr(rust_js, rust_js::nullable, rust_js::optional)]
    pub stack: Option<String>,
    pub debug_task: Option<u32>,
}

pub fn io_info(name: String, start: f64) -> ReactIOInfo {
    ReactIOInfo {
        name,
        start,
        env: None,
        // We don't know the requesting owner nor stack.
        owner: None,
        stack: None,
        debug_task: None,
    }
}

pub fn led_last(name: String) -> ReactIOInfo {
    ReactIOInfo {
        name,
        start: 1.0,
        env: None,
        stack: None,
        debug_task: None,
        // Set later.
        owner: None,
    }
}

pub struct Pending {
    pub owner: Option<u32>,
    pub info: ReactIOInfo,
}

pub fn unknown() -> Pending {
    Pending {
        // Nothing is known yet.
        owner: None,
        info: ReactIOInfo {
            name: "c".to_string(),
            // Not started.
            start: 0.0,
            env: None,
            // Its owner's.
            owner: None,
            stack: None,
            debug_task: None,
        },
    }
}

pub fn none_known() -> Pending {
    Pending {
        info: ReactIOInfo {
            name: "d".to_string(),
            start: 0.0,
            env: None,
            owner: None,
            stack: None,
            debug_task: None,
        },
        // Nothing else.
        owner: None,
    }
}

pub struct Later {
    pub owner: Option<u32>,
}

pub fn later() -> Later {
    Later {
        // Set later.
        owner: None,
    }
}

pub struct Boundary {
    pub status: u32,
    #[cfg_attr(rust_js, rust_js::nullable)]
    pub row: std::cell::RefCell<Option<std::rc::Rc<u32>>>,
    #[cfg_attr(rust_js, rust_js::nullable, rust_js::optional)]
    pub error_message: std::cell::RefCell<Option<String>>,
}

pub fn boundary(row: Option<std::rc::Rc<u32>>) -> Boundary {
    Boundary {
        status: 0,
        row: std::cell::RefCell::new(row.clone()),
        // DEV-only fields for hidden class
        error_message: std::cell::RefCell::new(None),
    }
}

pub fn has_owner() -> (bool, bool, bool) {
    let info = io_info("a".to_string(), 0.0);
    let last = led_last("b".to_string());
    (js::has(&info, "owner"), js::has(&info, "debugTask"), js::has(&last, "owner"))
}
`);
  expect(js).toContain(`  return {
    name,
    start,
    // We don't know the requesting owner nor stack.
  };`);
  expect(js).toContain(`    start: 1,
    // Set later.
  };`);
  expect(js).toContain(`  return {
    // Nothing is known yet.
    info: {
      name: "c",
      // Not started.
      start: 0,
      // Its owner's.
    },
  };`);
  expect(js).toContain(`    info: {
      name: "d",
      start: 0,
    },
    // Nothing else.
  };`);
  expect(js).toContain(`export function later() {
  return {
    // Set later.
  };
}`);
  // Of a value the JS writes of no place of its own, `row ?? null`, where
  // its literal writes it, React's createSuspenseBoundary's.
  expect(js).toContain(`    row: row ?? null,
    // DEV-only fields for hidden class
  };`);
  expect(js).not.toContain("undefined");
  expect(Object.keys(lib.ioInfo("a" as never, 0 as never) as object)).toEqual(["name", "start"]);
  expect(Object.keys(lib.ledLast("b" as never) as object)).toEqual(["name", "start"]);
  expect([lib.unknown(), lib.noneKnown(), lib.later()].map((made) => JSON.stringify(made))).toEqual([
    '{"info":{"name":"c","start":0}}',
    '{"info":{"name":"d","start":0}}',
    "{}",
  ]);
  expect(lib.hasOwner()).toEqual([false, false, false]);
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

// ADR 0444, amended (React port gap rd64e, the user's decision): React DOM's
// SyntheticEvent documents its prototype object's fields, `/** Checks if
// this event should be released back into the pool. .. */ isPersistent:
// functionThatReturnsTrue`. The port's fields are a struct's, documented
// where it's declared, which has no JS: a field's `///` is printed before
// that field in the struct's literal where the crate writes one literal of
// it, the one object it describes; a struct made in two places keeps them
// on its declaration alone, as a `//` before a `let` stays `//`.
test("a field's doc comment is before it in its struct's one literal", async () => {
  const { js, lib } = await compiled("field-docs-one-literal", `pub struct Prototype {
    pub prevent_default: fn() -> bool,
    /// We release all dispatched \`SyntheticEvent\`s after each event loop.
    pub persist: fn() -> bool,
    /// Checks if this event should be released back into the pool.
    ///
    /// @return {boolean} True if this should not be released, false otherwise.
    pub is_persistent: fn() -> bool,
}

pub struct Twice {
    /// Made in two places.
    pub count: u32,
}

fn yes() -> bool {
    true
}

pub fn prototype() -> Prototype {
    // A plain comment.
    let made = Prototype {
        prevent_default: yes,
        persist: yes,

        is_persistent: yes,
    };
    made
}

pub fn one() -> Twice {
    Twice { count: 1 }
}

pub fn two() -> Twice {
    Twice { count: 2 }
}
`);
  expect(js).toContain(`    preventDefault: yes,
    /** We release all dispatched \`SyntheticEvent\`s after each event loop. */
    persist: yes,

    /**
     * Checks if this event should be released back into the pool.
     *
     * @return {boolean} True if this should not be released, false otherwise.
     */
    isPersistent: yes,
`);
  expect(js).toContain("  // A plain comment.\n");
  expect(js).not.toContain("Made in two places");
  expect((lib.prototype() as unknown as { isPersistent: () => boolean }).isPersistent()).toBe(true);
});

// The playground's `Loaded { module: module.await, crates: crate_files, .. }`:
// a field's value MIR moves through a temporary, `_5 = crate_files`, is the
// variable read where the temporary was, so the field's doc comment, which
// is placed by where its value is, stays before it (`alias_variables`).
test("a field's doc comment is before it where its value is a variable MIR moved", async () => {
  const { js, lib } = await compiled("field-docs-moved-value", `pub struct Loaded {
    pub module: u32,
    /// Each crate a program may use, by its name.
    pub crates: Vec<(String, u32)>,
    pub count: u32,
}

async fn fetch(n: u32) -> u32 {
    n + 1
}

pub async fn load(names: Vec<String>) -> Loaded {
    let module = fetch(0);
    let files: Vec<_> = names.into_iter().map(|name| (name, fetch(1))).collect();
    let mut crate_files = Vec::new();
    for (name, file) in files {
        crate_files.push((name, file.await));
    }
    let count = fetch(2);
    Loaded {
        module: module.await,
        crates: crate_files,
        count: count.await,
    }
}
`);
  expect(js).toContain(`  return {
    module: await module,
    /** Each crate a program may use, by its name. */
    crates: crateFiles,
    count: await count,
  };`);
  expect(await (lib.load as (names: string[]) => Promise<unknown>)(["a", "b"])).toEqual({
    module: 1,
    crates: [
      ["a", 2],
      ["b", 2],
    ],
    count: 3,
  });
});

// A field whose value is a method call, `parent: x.unwrap()`, starts where
// the call's receiver does, a part of its own: of two parts at one start
// the outer's, the field's, so the blank line above its comment is kept,
// as the struct literal has it, Prettier's between properties (ADR 0708,
// amended).
test("a comment after a blank line before a method call's field keeps the line", async () => {
  const { js, lib } = await compiled("commented-call-field-apart", `pub struct Pooled {
    pub pool: u32,
    pub parent: String,
}

pub fn made(x: Option<String>, n: u32) -> Pooled {
    Pooled {
        pool: n + 1,

        // The parent, a refresh's.
        parent: x.unwrap(),
    }
}
`);
  expect(js).toContain("    pool: (n + 1) >>> 0,\n\n    // The parent, a refresh's.\n    parent: $unwrap(x),\n");
  const made = lib.made as (x: string, n: number) => unknown;
  expect(made("p", 1)).toEqual({ pool: 2, parent: "p" });
});
