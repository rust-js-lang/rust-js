// Where the compiler writes a program's comments: a line comment after the
// code it's after where JS has more on that line (ADR 0733), and one in a
// macro before the `fn` it writes before the function (ADR 0735). Each
// test is a program of its own, compiled and run.

import { beforeAll, expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { decodeMappings } from "./sourcemap";
import { buildReact, compiler, fixture, run, target } from "./support";

beforeAll(buildReact, 600_000);

/** `rust` compiled in a fixture of `name`, against the js crate: its JS,
 * its module, and its source map. */
async function compiled(name: string, rust: string): Promise<{ js: string; lib: Record<string, (...args: never[]) => unknown>; map: { mappings: string } }> {
  const dir = fixture(name);
  writeFileSync(join(dir, "lib.rs"), rust);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  return { js, lib: await import(join(dir, "lib.js")), map: JSON.parse(readFileSync(join(dir, "lib.js.map"), "utf8")) };
}

// React's addValueToProperties: `desc = '…'; // ellipsis`, and
// `value.slice(0, 1023) + '…' // ellipsis`. The port's branches are
// one conditional, on one line: each comment is after its `"…"`, not
// both after the line, `// ellipsis // ellipsis`.
test("a line comment is after its code where JS has more on the line", async () => {
  const { js, lib } = await compiled("comment-ends-its-line", `pub fn described(value: f64) -> String {
    if value.is_nan() {
        "\\u{2026}".to_string() // ellipsis
    } else {
        (if value > 1024.0 {
            value.to_string() + "\\u{2026}" // ellipsis
        } else {
            value.to_string()
        }) + "!"
    }
}
`);
  expect(js).toContain('    ? "…" // ellipsis\n');
  expect(js).toContain('? $displayF64(value) + "…" // ellipsis\n');
  expect(js).not.toContain("// ellipsis // ellipsis");
  expect([lib.described(NaN as never), lib.described(2048 as never), lib.described(1 as never)]).toEqual(["…", "2048…!", "1!"]);
});

// React's binaryToComparableString: a comment before the macro, and one
// in it before the `fn` it exports, the port's `// GAP:`, are both the
// function's, in order (ADR 0735).
test("a comment in a macro before its fn is the function's", async () => {
  const { js, lib } = await compiled("comment-in-a-macro", `// A license.

// Turns a number into a string
// to compare.
js::export_default! {
    // GAP: upstream takes any view; this takes the one it can name.
    fn compared(value: f64) -> String {
        value.to_string()
    }
}
`);
  expect(js).toContain(
    "// Turns a number into a string\n// to compare.\n// GAP: upstream takes any view; this takes the one it can name.\nexport default function compared(value) {\n",
  );
  expect(lib.default(2 as never)).toBe("2");
});

// React's addObjectDiffToProperties: `if (indent >= 3) { // Just
// fallthrough .. }` before its `else if`. A branch of only comments isn't
// nothing (decided 2026-10-24): MIR has no statement of it, but its `()`,
// which the branch's span is of, is where its comments are kept, as
// upstream writes them (ADR 0753).
test("a branch of only comments keeps them", async () => {
  const { js, lib } = await compiled("comments-only-branch", `pub fn described(indent: usize, prev: Option<u32>, out: &mut Vec<String>) {
    if indent >= 3 {
        // Just fallthrough to print the two values if we're deep.
        // This will skip nested properties of the objects.
    } else if let Some(prev) = prev
        && prev > 2
    {
        out.push(prev.to_string());
    }
    if prev.is_none() {
        out.push("none".to_string());
    } else {
        /* Nothing more. */
    }
    out.push("end".to_string());
}
`);
  expect(js).toContain(
    "  if (indent >= 3) {\n    // Just fallthrough to print the two values if we're deep.\n    // This will skip nested properties of the objects.\n  } else if (prev != null && prev > 2) {\n",
  );
  expect(js).toContain('  if (prev == null) {\n    out.push("none");\n  } else {\n    /* Nothing more. */\n  }\n');
  const ran = (indent: number, prev: number | undefined) => {
    const out: string[] = [];
    lib.described(indent as never, prev as never, out as never);
    return out;
  };
  expect([ran(4, 5), ran(1, 5), ran(1, 1), ran(1, undefined)]).toEqual([["end"], ["5", "end"], ["end"], ["none", "end"]]);
});

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
`);
  expect(js).toContain(`  return {
    s: 0, // status
    v: undefined, // value, either the cached result or an error, depending on s
    o: null, // object cache
  };`);
  expect(js).toContain(`  return {
    s: 0,
    // a value, before it
    v: undefined,
    o: null,
  };`);
  expect([Object.keys(lib.createCacheNode() as object), Object.keys(lib.led() as object)]).toEqual([["s", "v", "o"], ["s", "v", "o"]]);
});

// React's ReactNoopUpdateQueue: a field that's a closure of statements is
// a method (ADR 0882), and the comment before it is before the method, a
// blank line before it kept, as before a field of any other value.
test("a comment before a field that's a method is before the method", async () => {
  const { js, lib } = await compiled("commented-method-fields", `pub struct Updater {
    pub is_mounted: fn(u32) -> bool,
    pub enqueue_force_update: fn(u32, &str) -> u32,
    pub enqueue_set_state: fn(u32) -> u32,
}

fn warn_noop(n: u32, name: &str) -> u32 {
    n + name.len() as u32
}

pub static UPDATER: Updater = Updater {
    // Checks whether or not this composite component is mounted.
    is_mounted: |_| false,

    // Forces an update.
    //
    // You may want to call this when you know that some deeper aspect of
    // the component's state has changed.
    enqueue_force_update: |n, name| {
        let m = n + 1;
        warn_noop(m, name)
    },

    enqueue_set_state: |n| {
        let m = n * 2;
        warn_noop(m, "setState")
    },
};
`);
  expect(js).toContain(`  // Checks whether or not this composite component is mounted.
  isMounted: () => false,

  // Forces an update.
  //
  // You may want to call this when you know that some deeper aspect of
  // the component's state has changed.
  enqueueForceUpdate(n, name) {
    const m = (n + 1) >>> 0;
`);
  expect(js).toContain(`  },

  enqueueSetState(n) {
    const m = (n * 2) >>> 0;
`);
  const updater = lib.UPDATER as unknown as { enqueueForceUpdate: (n: number, name: string) => number; enqueueSetState: (n: number) => number };
  expect([updater.enqueueForceUpdate(1, "ab"), updater.enqueueSetState(2)]).toEqual([4, 12]);
});

// React's resolveDispatcher: a comment before an `unsafe` block that's its
// function's value, `unsafe { dispatcher.unwrap_unchecked() }`, is before
// the `return` its code is, as one inside the block would be.
test("a comment before an unsafe block is its code's", async () => {
  const { js, lib } = await compiled("comment-before-unsafe", `pub fn resolved(dispatcher: Option<u32>) -> u32 {
    // Will result in a null access error if accessed outside render phase.
    unsafe { dispatcher.unwrap_unchecked() }
}
`);
  expect(js).toContain("export function resolved(dispatcher) {\n  // Will result in a null access error if accessed outside render phase.\n  return dispatcher;\n}");
  expect(lib.resolved(3 as never)).toBe(3);
});

// ADR 0862: a comment before the `None` a function gives at its end, which
// is no `return;`, and one after a block's last code, are where they're
// written, at the end of the body: React's countChildren's callback, `n++;
// // Don't return anything`.
test("a comment before a None given at a function's end, or after its last code, is at its end", async () => {
  const { js, lib } = await compiled("comments-at-ends", `use std::cell::Cell;

fn each(items: &[u32], f: &dyn Fn(u32) -> Option<u32>) -> usize {
    items.iter().filter(|&&i| f(i).is_some()).count()
}

pub fn count(items: &[u32]) -> u32 {
    let n = Cell::new(0);
    each(items, &|_| {
        n.set(n.get() + 1);
        // Don't return anything
        None
    });
    n.get()
}

pub fn found(n: u32) -> Option<u32> {
    if n > 3 {
        return Some(n);
    }
    // Nothing found.
    None
}

pub fn noted(n: u32, out: &mut Vec<u32>) {
    if n > 1 {
        out.push(n);
        // And then nothing more.
    }
    out.push(0);
    // Nothing to do.
}
`);
  expect(js).toContain("    n = (n + 1) >>> 0;\n    // Don't return anything\n  });\n");
  expect(js).toContain("    return n;\n  }\n  // Nothing found.\n}\n");
  expect(js).toContain("    out.push(n);\n    // And then nothing more.\n  }\n  out.push(0);\n  // Nothing to do.\n}\n");
  expect(lib.count([1, 2, 3] as never)).toBe(3);
  expect([lib.found(5 as never), lib.found(1 as never)]).toEqual([5, undefined]);
  const out: number[] = [];
  lib.noted(2 as never, out as never);
  expect(out).toEqual([2, 0]);
});

// React's startTransition writes a comment of five lines before what a
// nested Transition shares, `prevTransition !== null ? // If we're a nested
// transition .. prevTransition.types : null`; its port before the `match`
// arm, `Some(prev_transition) => ..`. The lines are one run, none split
// from the others into the arm (ADR 0590, amended), before the branch's
// value in the conditional, as React's, where what's printed first of it,
// `prev` of `prev.types`, is read where it's bound, or is `String(n)`; and
// where the arm is read away, `n ?? given`, before the statement it's in,
// not lost (ADR 0444, amended).
test("a match arm's comment of many lines is its value's, none lost", async () => {
  const { js, lib } = await compiled("comment-arm-lines", `use std::cell::Cell;

pub struct Transition {
    pub types: Cell<Option<u32>>,
}

pub fn shared(prev: Option<&Transition>) -> Option<u32> {
    let current = Cell::new(None);
    current.set(match prev {
        // A nested one shares the parent's,
        // as they're entangled.
        Some(prev) => prev.types.get(),
        None => None,
    });
    current.get()
}

pub fn grown(prev: Option<&Transition>) -> Option<u32> {
    let current = Cell::new(None);
    current.set(match prev {
        // One more than the parent's,
        // as they're entangled.
        Some(prev) => Some(prev.types.get().unwrap_or(0) + 1),
        None => None,
    });
    current.get()
}

pub fn kept(n: Option<u32>, given: Option<u32>) -> Option<u32> {
    let current = Cell::new(None);
    current.set(match n {
        // Its own, where it has one,
        // else the one given.
        Some(n) => Some(n),
        None => given,
    });
    current.get()
}

pub fn told(n: Option<u32>) -> String {
    let told = match n {
        // Its count,
        // told.
        Some(n) => n.to_string(),
        None => String::new(),
    };
    told
}

pub fn counted(prev: Option<&Transition>) -> Option<u32> {
    match prev {
        // One more than the parent's,
        // as it's inside it.
        Some(prev) => Some(prev.types.get().unwrap_or(0) + 1),
        None => None,
    }
}

pub fn read(prev: Option<&Transition>) -> Option<u32> {
    match prev {
        // The parent's,
        // as it is.
        Some(prev) => prev.types.get(),
        None => None,
    }
}

pub fn branched(prev: Option<&Transition>) -> u32 {
    match prev {
        // A nested one's,
        // its parent's.
        Some(prev) => {
            prev.types.set(Some(7));
            1
        }
        None => 0,
    }
}
`);
  expect(js).toContain("  // A nested one shares the parent's,\n  // as they're entangled.\n  let current = prev?.types;\n");
  expect(js).toContain("  let current = prev\n    ? // One more than the parent's,\n      // as they're entangled.\n      ((prev.types ?? 0) + 1) >>> 0\n    : undefined;\n");
  expect(js).toContain("  // Its own, where it has one,\n  // else the one given.\n  let current = n ?? given;\n");
  expect(js).toContain("      ? // Its count,\n        // told.\n        String(n)\n");
  expect(js).toContain("  // One more than the parent's,\n  // as it's inside it.\n  if (prev) {\n");
  expect(js).toContain("  if (prev) {\n    // The parent's,\n    // as it is.\n    return prev.types;\n");
  expect(js).toContain("  // A nested one's,\n  // its parent's.\n  if (prev) {\n");
  expect([lib.shared({ types: 3 } as never), lib.shared(undefined as never), lib.grown({ types: 3 } as never), lib.grown(undefined as never), lib.counted({ types: 3 } as never), lib.branched(undefined as never)]).toEqual([3, undefined, 4, undefined, 4, 0]);
  expect([lib.kept(0 as never, 5 as never), lib.kept(undefined as never, 5 as never), lib.told(0 as never), lib.told(undefined as never)]).toEqual([0, 5, "0", ""]);
});

// React's cache: `// Objects go into a WeakMap` before the port's `let arg:
// &JsObject = unsafe { js::cast(arg) };`, which has no JS, the value read
// where it's named: the comment is the next statement's, in the `if`'s
// block, as upstream's is, not before the `for` it's in, whether that's a
// `let` of a `match`, whose JS starts at what it matches, or a call (ADR
// 0444, amended).
test("a comment before a let read in place is the next statement's, in its block", async () => {
  const { js, lib } = await compiled("comment-before-read-in-place", `use js::{JsObject, Kind, Unknown};
use std::cell::Cell;

pub struct Node {
    pub o: Cell<Option<u32>>,
}

pub fn count<'a>(node: &Node, args: &[Option<&'a Unknown>], seen: &mut Vec<&'a JsObject>) -> u32 {
    let mut objects = 0;
    for &arg in args.iter() {
        if let Kind::Function(_) | Kind::Object(_) = js::classify(arg) {
            // Objects are kept.
            let arg: &JsObject = unsafe { js::cast(arg) };
            let cache = match node.o.get() {
                Some(cache) => cache,
                None => {
                    let cache = 7;
                    node.o.set(Some(cache));
                    cache
                }
            };
            seen.push(arg);
            objects += cache;
        }
    }
    objects
}

pub fn kept<'a>(args: &[Option<&'a Unknown>], seen: &mut Vec<&'a JsObject>) {
    for &arg in args.iter() {
        if let Kind::Object(_) = js::classify(arg) {
            // Pushed as it is.
            let arg: &JsObject = unsafe { js::cast(arg) };
            seen.push(arg);
        }
    }
}
`);
  expect(js).toContain("    ) {\n      // Objects are kept.\n      let cache = node.o;\n");
  expect(js).toContain("      // Pushed as it is.\n      seen.push(arg);\n");
  expect(js).not.toContain("// Objects are kept.\n  for");
  const seen: unknown[] = [];
  const object = { a: 1 };
  expect(lib.count({ o: undefined } as never, [object, 1, "x", object] as never, seen as never)).toBe(14);
  lib.kept([object, 2] as never, seen as never);
  expect(seen).toEqual([object, object, object]);
});

// React's cache: `// $FlowFixMe[incompatible-type]` inside the closure it
// gives `catch_unwind`, before the call that's its value: in the `try`,
// before what that call is, `const result = fn.apply(null, args)`, as
// upstream's, not before the `try` (ADR 0444, amended).
test("a comment in catch_unwind's closure is in the try", async () => {
  const { js, lib } = await compiled("comment-in-caught-closure", `use std::panic::{self, AssertUnwindSafe};

pub fn run(f: &dyn Fn(u32) -> u32, x: u32) -> u32 {
    match panic::catch_unwind(AssertUnwindSafe(|| {
        // Called once.
        f(x)
    })) {
        Ok(result) => {
            let doubled = result * 2;
            doubled
        }
        Err(_) => 0,
    }
}
`);
  expect(js).toContain("  try {\n    // Called once.\n    const result = f(x);\n");
  expect(js).not.toContain("// Called once.\n  try");
  expect(lib.run(((n: number) => n + 1) as never, 2 as never)).toBe(6);
  expect(
    lib.run(
      (() => {
        throw new Error("no");
      }) as never,
      2 as never,
    ),
  ).toBe(0);
});

// React's BadMapPolyfill: `/* eslint-enable no-new */`, the last line of
// the closure it gives `catch_unwind`, after its last call, is at the end
// of the `try`, as upstream's, not the first of the `catch`; and a comment
// after a `catch`'s last code is at its end, not before its `{` nor after
// the statement (ADR 0862, amended).
test("a comment at the end of catch_unwind's closure or its Err's block is at its block's end", async () => {
  const { js, lib } = await compiled("comment-ends-try", `use std::cell::Cell;
use std::panic::catch_unwind;

use js::Kind;

thread_local! {
    static BAD: Cell<bool> = const { Cell::new(false) };
    static SEEN: Cell<u32> = const { Cell::new(0) };
}

js::on_load! {
    if let Err(_e) = catch_unwind(|| {
        let frozen = js::object::freeze(js::dict::new::<Kind>());
        /* eslint-disable no-new */
        js::set::new(Some(&[frozen]));
        js::set::new(Some(&[frozen]));
        /* eslint-enable no-new */
    }) {
        BAD.set(true);
        // Nothing else to do.
    }
}

pub fn bad() -> bool {
    BAD.get()
}

pub fn tried(f: fn(u32) -> u32) -> u32 {
    SEEN.set(0);
    if let Err(_e) = catch_unwind(|| {
        SEEN.set(f(1));
        SEEN.set(f(SEEN.get()));
        // Both ran.
    }) {
        SEEN.set(SEEN.get() + 100);
        // It threw.
    }
    SEEN.get()
}
`);
  expect(js).toContain("  new Set([frozen]);\n  /* eslint-enable no-new */\n} catch (_e) {\n  BAD = true;\n  // Nothing else to do.\n}\n");
  expect(js).toContain("    seen = f(seen);\n    // Both ran.\n  } catch (_e) {\n    seen = (seen + 100) >>> 0;\n    // It threw.\n  }\n  return seen;");
  expect(lib.bad()).toBe(false);
  expect(lib.tried(((n: number) => n + 1) as never)).toBe(3);
  expect(
    lib.tried((() => {
      throw new Error("no");
    }) as never),
  ).toBe(100);
});

// React's mapIntoArray: `escapedPrefix + // $FlowFixMe[incompatible-type]
// .. (mappedChild.key != null && .. ? .. : '') + childKey`. The port writes
// the comment before the `+` its operand follows, as rustfmt puts an
// operator first: the comment is before that operand, after the `+`, as
// JS puts it last, not lost nor before the statement (ADR 0444, amended).
test("a comment between a binary's operands is before its right one", async () => {
  const { js, lib } = await compiled("comment-between-operands", `pub fn keyed(prefix: &str, key: Option<&str>, child: &str) -> String {
    prefix.to_string()
        // The key, when there's one.
        + &(if let Some(key) = key {
            key.to_string() + "/"
        } else {
            String::new()
        })
        + child
}

pub fn both(a: bool, b: bool) -> bool {
    a
        // And the other.
        && b
}

pub struct Point {
    pub x: u32,
}

pub fn sum(a: u32, p: Option<&Point>) -> u32 {
    match p {
        Some(p) => {
            a
                // And its x.
                + p.x
        }
        None => a,
    }
}

pub fn big(x: Option<u32>) -> bool {
    x.is_some_and(|v| {
        // Past three.
        v > 3
    })
}
`);
  expect(js).toContain("    prefix +\n    // The key, when there's one.\n    (key != null ? key + \"/\" : \"\") +\n    child\n");
  // The closure's test, the right operand of what it's read as.
  expect(js).toContain("    x != null &&\n    // Past three.\n    x > 3\n");
  expect([lib.big(4 as never), lib.big(2 as never), lib.big(undefined as never)]).toEqual([true, false, false]);
  expect(js).toContain("  return (\n    a &&\n    // And the other.\n    b\n  );\n");
  // `p` of `p.x`, read where it's bound, placed where the operand is.
  expect(js).toContain("      (a +\n        // And its x.\n        p.x) >>>\n      0\n");
  expect([lib.sum(1 as never, { x: 2 } as never), lib.sum(1 as never, undefined as never)]).toEqual([3, 1]);
  expect(js.match(/The key/g)?.length).toBe(1);
  expect([lib.keyed("p" as never, "k" as never, "c" as never), lib.keyed("p" as never, undefined as never, "c" as never)]).toEqual(["pk/c", "pc"]);
  expect([lib.both(true as never, true as never), lib.both(true as never, false as never)]).toEqual([true, false]);
});

// React's mapChildren: the port writes `// $FlowFixMe[incompatible-type]`
// before `children?;`, as clippy's `question_mark` asks for its `if
// children.is_none() { return None; }`, a comment inside it or not: the
// comment is before that statement's JS, its `if`, where the Rust has it;
// upstream's, inside its `if`, is before a `return children` the port
// doesn't write.
test("a comment before a ? statement is before its if", async () => {
  const { js, lib } = await compiled("comment-before-question-mark", `pub fn mapped(children: Option<&str>) -> Option<usize> {
    // Nothing to map.
    children?;
    Some(children.map_or(0, str::len))
}
`);
  expect(js).toContain("  // Nothing to map.\n  if (children == null) {\n");
  expect([lib.mapped(undefined as never), lib.mapped("ab" as never)]).toEqual([undefined, 2]);
});

// React's ReactBaseClasses: `const emptyObject = {};` and the `if
// (__DEV__)` that freezes it are one block, as its port's `static` and the
// `js::on_load!` right below it are; its ReactNoopUpdateQueue's object has
// a blank line between its fields, as its struct literal has; its
// ReactChildren's `const iterator =` is apart from the `if` before it, as
// its port's erased cast `let` before it is. A blank line in the JS is
// where the Rust has one between the code its lines are of, and nowhere
// else (ADR 0865).
test("a blank line is where the Rust has one between the code its JS is of", async () => {
  const { js, lib } = await compiled("blank-lines-as-written", `use js::Unknown;

pub struct Empty {}

unsafe impl js::Defined for Empty {}

static EMPTY: Empty = Empty {};
js::on_load! {
    if js::type_of(Some(js::unknown(&EMPTY))) == "object" {
        js::object::freeze(&EMPTY);
    }
}

pub fn glued() -> u32 {
    1
}
pub fn below() -> u32 {
    2
}

pub fn apart() -> u32 {
    3
}

pub struct Unit;
pub fn after_a_type() -> u32 {
    4
}

#[derive(Debug, PartialEq)]
pub struct Shown(pub u32);

pub fn shown(a: &Shown, b: &Shown) -> String {
    both(a, b)
}

fn both<T: std::fmt::Debug + PartialEq>(a: &T, b: &T) -> String {
    format!("{a:?} {}", a == b)
}

pub struct Pair {
    pub a: u32,
    pub b: u32,
}

pub fn pair() -> Pair {
    Pair {
        a: 1,

        b: 2,
    }
}

pub struct Counter(u32);

impl Counter {
    pub fn first(&self) -> u32 {
        self.0
    }

    pub fn second(&self) -> u32 {
        self.0 + 1
    }
}

pub fn erased(value: Option<&'static Unknown>, flag: bool) -> u32 {
    let mut n = 0;
    if flag {
        n += 1;
    }

    let value: &Unknown = unsafe { js::cast(value) };
    let length = js::get(value, "length");
    if length.is_some() {
        n += 2;
    }
    n
}
`);
  expect(js).toContain("const EMPTY = {};\nif (");
  expect(js).toContain("}\nexport function below() {");
  expect(js).toContain("}\n\nexport function apart() {");
  // Apart, as a type with no JS between is.
  expect(js).toContain("}\n\nexport function afterAType() {");
  // What one line, a derive, makes is set apart as a person sets it (ADR
  // 0303): the Rust tells nothing of its lines.
  expect(js).toContain("export const shownDebug = { fmt: shownDebug_fmt };\n\nfunction shownDebug_fmt(");
  expect(js).toContain("    a: 1,\n\n    b: 2,\n");
  expect(js).toContain("  if (flag) {\n    n++;\n  }\n\n  const length = value.length;\n");
  expect([lib.glued(), lib.below(), lib.apart(), lib.afterAType(), lib.shown([1] as never, [1] as never), lib.pair(), lib.erased("ab" as never, true as never)])
    .toEqual([1, 2, 3, 4, "Shown(1) true", { a: 1, b: 2 }, 3]);
});

// React's ReactForwardRef: its `if (render != null && render.$$typeof ===
// REACT_MEMO_TYPE) { .. } else if (..) { .. }` chain is apart from the `if
// (render != null) {` below it, as its port's `if let` chains are. MIR's
// structure makes a labeled block of the first, `l: { if (a) { if (b) { s;
// break l; } } t }`, of no Rust of its own: its `if (a && b) { s } else {
// t }` is where its first `if` is, not at the next statement's start, which
// put the blank line before it (ADR 0865, amended).
test("an if let chain made of a labeled block is where its first if is", async () => {
  const { decodeMappings } = await import("./sourcemap");
  const rust = `use js::{SymbolFor, Unknown};

const MEMO: SymbolFor = SymbolFor("react.memo");

pub fn checked(value: Option<&'static Unknown>) -> u32 {
    let mut n = 0;
    if let Some(value) = value
        && SymbolFor::from_value(js::get(value, "$$typeof")) == Some(MEMO)
    {
        n += 1;
    } else if js::type_of(value) != "function" {
        n += 2;
    }

    if let Some(value) = value
        && js::get(value, "b").is_some()
    {
        n += 4;
    }
    n
}
`;
  const { js, lib, map } = await compiled("if-let-chain-apart", rust);
  expect(js).toContain("  let n = 0;\n  if (value != null && value.$$typeof === MEMO) {\n");
  expect(js).toContain("    n += 2;\n  }\n\n  if (value != null && value.b != null) {\n");
  // Each `if` maps to its own `if let`'s pattern.
  const lines = js.split("\n");
  const rs = rust.split("\n");
  const ifs = lines.flatMap((line, i) => (line.startsWith("  if (") ? [i] : []));
  const mapped = ifs.map((i) => decodeMappings(map.mappings).find((s) => s.jsLine === i && s.jsCol === 2)?.srcLine);
  expect(mapped).toEqual([rs.indexOf("    if let Some(value) = value"), rs.lastIndexOf("    if let Some(value) = value")]);
  const memo = { $$typeof: Symbol.for("react.memo"), b: 1 };
  expect([lib.checked(memo as never), lib.checked(undefined as never), lib.checked((() => 0) as never)]).toEqual([5, 2, 0]);
});

// React's describeNativeComponentFrame: `// We couldn't find our
// intentionally injected common root frame ..` before `if s == .. || c ==
// .. { .. while .. { .. } }`, and `// In V8, the first line ..` before `if
// s != 1 || c != 1 { loop { .. } }`. MIR's structure makes a labeled block
// of each, its `while`'s `break` leaving it, which `decisions` makes the
// chain of its arms: the `if` was where its first arm's first statement
// is, past the `{` the comment is before, so the comment was nobody's,
// before the function's `try` (ADR 0444, amended).
test("an if a decision tree makes is where its first if is", async () => {
  const { decodeMappings } = await import("./sourcemap");
  const rust = `pub fn found(sample: Option<&str>, control: Option<&str>) -> usize {
    let mut n = 0;
    if let Some(sample) = sample
        && let Some(control) = control
    {
        let mut s = sample.len();
        let c = control.len();
        // Neither was found, so count
        // down from the bottom.
        if s == 3 || c == 4 {
            s -= 1;
            while s > 1 {
                // Each step down.
                s -= 1;
            }
        }
        while s >= 1 {
            // Not on the first line.
            if s != 1 || c != 1 {
                loop {
                    s -= 1;
                    if s < 1 {
                        break;
                    }
                }
            }
            n += 1;
            s = 0;
        }
        n += s;
    }
    n
}
`;
  const { js, lib, map } = await compiled("decision-if-placed", rust);
  expect(js).toContain("    // Neither was found, so count\n    // down from the bottom.\n    if (s === 3 || c === 4) {\n");
  expect(js).toContain("      // Not on the first line.\n      if (s !== 1 || c !== 1) {\n");
  // Each `if` maps to its first test.
  const lines = js.split("\n");
  const rs = rust.split("\n");
  const mapped = ["if (s === 3", "if (s !== 1"].map((start) => {
    const at = lines.findIndex((line) => line.trimStart().startsWith(start));
    return decodeMappings(map.mappings).find((s) => s.jsLine === at && s.jsCol === lines[at].indexOf("if"))?.srcLine;
  });
  expect(mapped).toEqual([rs.findIndex((l) => l.includes("if s == 3")), rs.findIndex((l) => l.includes("if s != 1"))]);
  expect([lib.found("abc" as never, "ab" as never), lib.found("a" as never, "abcd" as never), lib.found("ab" as never, "a" as never), lib.found(undefined as never, "a" as never)]).toEqual([1, 0, 1, 0]);
});

// React's ReactTaint: `// $FlowFixMe[invalid-compare]` before `if (lifetime
// === null || ..)`, and inside it, after `if (`, before its first operand.
// The `||` of an `if` whose arm ends, `guarded`'s, was placed where MIR's
// labeled block is, nowhere, so a comment before it was before the next
// `if`, and one inside it lost. It's where its first guard is, and an
// `if`'s test is a start after its `if (` for what's written after the
// `if` (ADR 0444, amended).
test("a comment before an or's if or inside it before its first operand is there", async () => {
  const { js, lib } = await compiled("comment-or-guard", `use js::{Kind, Unknown};

pub fn checked(value: Option<&Unknown>) {
    // before an or
    if value.is_none() || js::type_of(value) != "object" {
        panic!("a");
    }
    // before a single test
    if matches!(js::classify(value), Kind::Null) {
        panic!("b");
    }
}

pub fn taint(lifetime: Option<&Unknown>) {
    if
    // $FlowFixMe[invalid-compare]
    matches!(js::classify(lifetime), Kind::Null)
        || (js::type_of(lifetime) != "object" && js::type_of(lifetime) != "function")
    {
        panic!("To taint a value");
    }
}

pub fn single(value: Option<&Unknown>) -> bool {
    if
    // inside a single test
    value.is_none()
    {
        return true;
    }
    false
}
`);
  expect(js).toContain('  // before an or\n  if (value == null || typeof value !== "object") {\n');
  expect(js).toContain("  // before a single test\n  if (value === null) {\n");
  expect(js).toContain('  if (\n    // $FlowFixMe[invalid-compare]\n    lifetime === null ||\n    (typeof lifetime !== "object" && typeof lifetime !== "function")\n  ) {\n');
  expect(js).toContain("  if (\n    // inside a single test\n    value == null\n  ) {\n");
  expect(() => lib.checked(undefined as never)).toThrow("a");
  expect(() => lib.checked((() => 1) as never)).toThrow("a");
  expect(() => lib.checked({} as never)).not.toThrow();
  expect([lib.single(undefined as never), lib.single(1 as never)]).toEqual([true, false]);
  expect(() => lib.taint(null as never)).toThrow("To taint a value");
  expect(() => lib.taint({} as never)).not.toThrow();
});

// React's startTransition: `*current.types.get_mut() = match prev { // If
// we're a nested transition .. \n Some(prev) => prev.types.borrow().clone(),
// None => None }`, which drops what the place held, so MIR writes it where
// its place is, `*current.types.get_mut()`; read as `prev?.types ?? null`,
// its arm's comment was in no statement and lost. The assignment is where
// its Rust is, its place to its value (ADR 0444, amended).
test("a comment in a value assigned over one dropped is before its statement", async () => {
  const { js, lib } = await compiled("comment-in-dropping-assignment", `use std::cell::RefCell;
use std::rc::Rc;

pub struct Transition {
    #[cfg_attr(rust_js, rust_js::nullable, rust_js::optional)]
    pub types: RefCell<Option<Rc<RefCell<Vec<u32>>>>>,
}

pub fn started(prev: Option<&Transition>) -> usize {
    let mut current = Transition {
        types: RefCell::new(None),
    };
    *current.types.get_mut() = match prev {
        // A nested one shares the parent's,
        // as they're entangled.
        Some(prev) => prev.types.borrow().clone(),
        None => None,
    };
    current
        .types
        .borrow()
        .as_ref()
        .map_or(0, |types| types.borrow().len())
}
`);
  expect(js).toContain("  // A nested one shares the parent's,\n  // as they're entangled.\n  current.types = prev?.types ?? null;\n");
  expect([lib.started({ types: [1, 2] } as never), lib.started(undefined as never)]).toEqual([2, 0]);
});

// React's cache: `if let Kind::Function(_) // $FlowFixMe[..] \n |
// Kind::Object(_) | Kind::Array(_) = js::classify(arg)`, and its
// startTransition's `js::type_of(v) == "object" // $FlowFixMe[..] \n && let
// Some(v) = v`: neither `|` nor a let-chain's `&&` is a binary in rustc's
// tree, but each is one in the JS, whose right operand a comment before it
// is before, after its operator, as upstream's is (ADR 0444, amended). An
// `if let`'s alternatives are tested as written, too.
test("a comment before an or-pattern's alternative or a let-chain's let is before its operand", async () => {
  const { js, lib } = await compiled("comment-before-alternative", `use js::{Kind, Unknown};

pub fn objects(args: &[Option<&Unknown>]) -> u32 {
    let mut objects = 0;
    for &arg in args.iter() {
        if let Kind::Function(_)
        // Not null.
        | Kind::Object(_)
        | Kind::Array(_) = js::classify(arg)
        {
            objects += 1;
        }
    }
    objects
}

pub fn numbers(args: &[Option<&Unknown>]) -> u32 {
    let mut numbers = 0;
    for &arg in args.iter() {
        if let Kind::Number(_)
        // Or a string.
        | Kind::String(_) = js::classify(arg)
        {
            numbers += 1;
        }
    }
    numbers
}

pub enum Shape {
    Circle(f64),
    Square(f64),
    Line,
}

pub fn round(shape: &Shape) -> bool {
    if let Shape::Line
    // Or a circle.
    | Shape::Circle(_) = shape
    {
        return true;
    }
    false
}

pub fn rounds() -> Vec<bool> {
    vec![round(&Shape::Circle(1.0)), round(&Shape::Square(1.0)), round(&Shape::Line)]
}

pub fn chained(value: Option<&Unknown>, given: Option<u32>) -> u32 {
    if js::type_of(value) == "object"
        // Set.
        && let Some(n) = given
        && n > 2
    {
        return n;
    }
    0
}
`);
  expect(js).toContain('      typeof arg === "function" ||\n      // Not null.\n      (typeof arg === "object" && arg !== null)\n');
  expect(js).toContain('      typeof arg === "number" ||\n      // Or a string.\n      typeof arg === "string"\n');
  expect(js).toContain('    typeof value === "object" &&\n    // Set.\n    given != null &&\n');
  expect(js).toContain('    shape === "Line" ||\n    // Or a circle.\n    shape.TAG === "Circle"\n');
  expect(js.match(/\/\/ (Not null|Or a string|Or a circle|Set)\./g)).toHaveLength(4);
  const values = [() => 1, {}, [], null, 1, "a"];
  expect([lib.objects(values as never), lib.numbers(values as never), lib.chained({} as never, 3 as never), lib.chained({} as never, undefined as never), lib.rounds()])
    .toEqual([3, 2, 3, 0, [true, false, true]]);
});

// React's startTransition writes a blank line before `let _finish =
// FinishTransition { .. };`, a guard whose drop is the `finally`, which has
// no JS of its own: it's before the `try`, which starts where its first
// statement does, as upstream's `try {` is set apart (ADR 0865, amended).
// And ReactSharedInternalsClient's `export default`, written after the
// code, made what the Rust tells of its statements off by one, so its `if`
// right below its literal was set apart.
test("a blank line before a guard's let is before its try, and a default export tells nothing", async () => {
  const { js, lib } = await compiled("blank-before-guard", `use std::cell::Cell;

pub struct Restore<'a> {
    cell: &'a Cell<u32>,
    prev: u32,
}

impl Drop for Restore<'_> {
    fn drop(&mut self) {
        self.cell.set(self.prev);
    }
}

pub fn scoped(cell: &Cell<u32>, scope: impl FnOnce() -> u32) -> u32 {
    let prev = cell.get();
    cell.set(prev + 1);

    let _restore = Restore { cell, prev };
    let got = scope();
    got + 1
}

pub fn returned(cell: &Cell<u32>, scope: impl FnOnce() -> u32) -> u32 {
    let prev = cell.get();
    cell.set(prev + 1);

    let _restore = Restore { cell, prev };
    scope()
}

pub fn glued(cell: &Cell<u32>, scope: impl FnOnce() -> u32) -> u32 {
    let prev = cell.get();
    cell.set(prev + 1);
    let _restore = Restore { cell, prev };
    scope()
}
`);
  expect(js).toContain("  cell.value = (prev + 1) >>> 0;\n\n  try {\n    const got = scope();\n");
  expect(js).toContain("  cell.value = (prev + 1) >>> 0;\n\n  try {\n    return scope();\n");
  expect(js).toContain("  cell.value = (prev + 1) >>> 0;\n  try {\n    return scope();\n");
  const cell = { value: 3 };
  const read = (() => cell.value) as never;
  expect([lib.scoped(cell as never, read), cell.value, lib.returned(cell as never, read), lib.glued(cell as never, read), cell.value])
    .toEqual([5, 3, 4, 4, 3]);
  const internals = await compiled("default-export-tells-nothing", `use std::cell::Cell;

pub struct State {
    pub a: Cell<Option<f64>>,
    pub b: Cell<Option<f64>>,
}

thread_local! {
    static STATE: State = State {
        a: Cell::new(None),
        b: Cell::new(None),
    };
}
js::on_load! {
    STATE.with(|state| {
        if state.b.get().is_none() {
            state.a.set(Some(1.0));
        }

        if state.a.get().is_some() {
            state.b.set(Some(2.0));
        }
    });
}

pub fn total() -> f64 {
    STATE.with(|state| state.a.get().unwrap_or(0.0) + state.b.get().unwrap_or(0.0))
}

pub use total as sum;

js::export_default!(STATE);
`);
  expect(internals.js).toContain("};\nif (state.b == null) {\n  state.a = 1;\n}\n\nif (state.a != null) {\n");
  expect(internals.js).toContain("\n\nexport default state;\n\nexport { total as sum };\n");
  expect([internals.lib.default, internals.lib.sum()]).toEqual([{ a: 1, b: 2 }, 3]);
});

// React's ReactTypeOfMode and ReactFiberFlags: `export const NoMode = /*
// .. */ 0b0000000;`, a comment between the `=` and the value that lines the
// values up. It's before the value's JS, after its `=`, its spaces kept, not
// after `export` or a `const`, the code before it on its line that has a
// name (ADR 0630, amended); one after the value is the line's, as before.
test("a comment between an = and its value is before the value", async () => {
  const rust = `use std::cell::Cell;

#[cfg_attr(rust_js, rust_js::name = "NoMode")]
pub const NO_MODE: u32 = /*                         */ 0b0000000;
// TODO: Remove ConcurrentMode by reading from the root tag instead
#[cfg_attr(rust_js, rust_js::name = "ConcurrentMode")]
pub const CONCURRENT_MODE: u32 = /*                 */ 0b0000001;
#[cfg_attr(rust_js, rust_js::name = "Removed")]
pub const REMOVED: u32 = 0b01; // Removed

pub fn modes(given: u32) -> u32 {
    let mode = /*  */ given | CONCURRENT_MODE;
    let mut last = /* first */ mode;
    last = /* again */ last | (mode + 1);
    last
}

pub fn same(a: u32, b: u32) -> bool {
    let same = a == /* the other */ b;
    same || a > 9
}

static FLAGS: u32 = /*  */ 0b0100;

thread_local! {
    static COUNT: Cell<u32> = /* none yet */ const { Cell::new(0) };
}

pub fn flags() -> u32 {
    COUNT.with(|count| count.set(count.get() + 1));
    FLAGS + COUNT.with(Cell::get) + NO_MODE + REMOVED
}
`;
  const { js, lib, map } = await compiled("comment-before-a-value", rust);
  expect(js).toContain("\nexport const NoMode = /*                         */ 0b0000000;\n");
  expect(js).toContain(
    "// TODO: Remove ConcurrentMode by reading from the root tag instead\nexport const ConcurrentMode = /*                 */ 0b0000001;\n",
  );
  expect(js).toContain("\nexport const Removed = 0b01; // Removed\n");
  expect(js).toContain("  const mode = /*  */ (given | ConcurrentMode) >>> 0;\n");
  expect(js).toContain("  let last = /* first */ mode;\n");
  // After an `==`, `a`'s, as before.
  expect(js).toContain("  const same = a /* the other */ === b;\n");
  expect(js).toContain("  last = /* again */ (last | (mode + 1)) >>> 0;\n");
  expect(js).toContain("\nconst FLAGS = /*  */ 0b0100;\n");
  expect(js).toContain("\nlet count = /* none yet */ 0;\n");
  expect([lib.NoMode, lib.ConcurrentMode, lib.Removed, lib.modes(4 as never), lib.same(2 as never, 2 as never), lib.flags(), lib.flags()]).toEqual([0, 1, 1, 7, true, 6, 7]);
  // The JS after the comment maps to its Rust, the comment's width further.
  const segments = decodeMappings(map.mappings);
  const place = (text: string, code: string) => {
    const lines = text.split("\n");
    const line = lines.findIndex(l => l.includes(code));
    return [line, lines[line].indexOf(code)];
  };
  const mapped = (code: string) => {
    const [line, column] = place(js, code);
    const segment = segments.find(s => s.jsLine === line && s.jsCol === column);
    return [segment?.srcLine, segment?.srcCol];
  };
  expect(mapped("given | ConcurrentMode")).toEqual(place(rust, "given | CONCURRENT_MODE"));
  expect(mapped("0b0000001")).toEqual(place(rust, "0b0000001"));
});

// React's ReactFiberLane, its comments as upstream has them: one an `else`'s
// block starts with before its only `if`, `// Nothing has been pinged.`,
// keeps the block; one between an `||`'s operands before a parenthesized
// one; one before a call's argument that's a `const`, `NO_LANES`; and one
// after an `if` before a test MIR places at the keyword, `(lane & a) | ..
// != 0`, with one between its operands (ADR 0444, 0708, 0883, amended).
test("ReactFiberLane's comments in an else, an if's test and a call are where it writes them", async () => {
  const { js, lib } = await compiled("comments-of-lanes", `pub const NO_LANES: i32 = 0;

pub fn next(pinged: i32, pending: i32, has_commit: bool, warm: i32) -> i32 {
    let mut next_lanes = NO_LANES;
    if pinged != NO_LANES {
        next_lanes = pinged & 3;
    } else {
        // Nothing has been pinged. Check for lanes that need to be prewarmed.
        if !has_commit {
            let lanes_to_prewarm = pending & !warm;
            if lanes_to_prewarm != NO_LANES {
                next_lanes = lanes_to_prewarm & 5;
            }
        }
    }
    next_lanes
}

pub fn keep(next_lane: i32, wip_lane: i32) -> i32 {
    if
    // Tests whether the next lane is equal or lower priority than the wip
    // one. This works because the bits decrease in priority as you go left.
    next_lane >= wip_lane
        // Default priority updates should not interrupt transition updates.
        || (next_lane == 32 && (wip_lane & 256) != NO_LANES)
    {
        // Keep working on the existing in-progress tree. Do not interrupt.
        return wip_lane;
    }
    next_lane
}

fn mark_spawned_deferred_lane(lanes: &mut Vec<i32>, spawned_lane: i32, entangled_lanes: i32) {
    lanes.push(spawned_lane | entangled_lanes);
}

pub fn finish(lanes: &mut Vec<i32>, spawned_lane: i32) {
    if spawned_lane != 0 {
        mark_spawned_deferred_lane(
            lanes,
            spawned_lane,
            // This render finished successfully without suspending, so we don't need
            // to entangle the spawned task with the parent task.
            NO_LANES,
        );
    }
}

pub fn entangled(lane: i32, entangled_lanes: i32, entanglements: &[i32], index: usize) -> bool {
    if
    // Is this one of the newly entangled lanes?
    (lane & entangled_lanes)
        // Is this lane transitively entangled with the newly entangled lanes?
        | (entanglements[index] & entangled_lanes)
        != 0
    {
        return true;
    }
    false
}
`);
  expect(js).toContain(
    "  } else {\n    // Nothing has been pinged. Check for lanes that need to be prewarmed.\n    if (!hasCommit) {\n",
  );
  expect(js).toContain(
    "    nextLane >= wipLane ||\n    // Default priority updates should not interrupt transition updates.\n    (nextLane === 32 && (wipLane & 256) !== NO_LANES)\n",
  );
  expect(js).toContain(
    "      spawnedLane,\n      // This render finished successfully without suspending, so we don't need\n      // to entangle the spawned task with the parent task.\n      NO_LANES,\n    );\n",
  );
  expect(js).toContain(
    "  if (\n    // Is this one of the newly entangled lanes?\n    (lane & entangledLanes) |\n    // Is this lane transitively entangled with the newly entangled lanes?\n    ($index(entanglements, index) & entangledLanes)\n  ) {\n",
  );
  const lanes: number[] = [];
  lib.finish(lanes as never, 4 as never);
  expect([lib.next(0 as never, 7 as never, false as never, 2 as never), lib.keep(1 as never, 2 as never), lib.keep(32 as never, 256 as never), lanes, lib.entangled(1 as never, 1 as never, [0] as never, 0 as never), lib.entangled(2 as never, 1 as never, [0] as never, 0 as never)]).toEqual([5, 1, 256, [4], true, false]);
});
