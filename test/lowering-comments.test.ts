// Where the compiler writes a program's comments: a line comment after the
// code it's after where JS has more on that line (ADR 0733), and one in a
// macro before the `fn` it writes before the function (ADR 0735). Each
// test is a program of its own, compiled and run.

import { beforeAll, expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { buildReact, compiler, fixture, run, target } from "./support";

beforeAll(buildReact, 600_000);

/** `rust` compiled in a fixture of `name`, against the js crate: its JS,
 * and its module. */
async function compiled(name: string, rust: string): Promise<{ js: string; lib: Record<string, (...args: never[]) => unknown> }> {
  const dir = fixture(name);
  writeFileSync(join(dir, "lib.rs"), rust);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  return { js: readFileSync(join(dir, "lib.js"), "utf8"), lib: await import(join(dir, "lib.js")) };
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
  expect(js).toContain("  let current = prev\n    ? // A nested one shares the parent's,\n      // as they're entangled.\n      prev.types\n    : undefined;\n");
  expect(js).toContain("  // Its own, where it has one,\n  // else the one given.\n  let current = n ?? given;\n");
  expect(js).toContain("      ? // Its count,\n        // told.\n        String(n)\n");
  expect(js).toContain("  // One more than the parent's,\n  // as it's inside it.\n  if (prev) {\n");
  expect(js).toContain("  if (prev) {\n    // The parent's,\n    // as it is.\n    return prev.types;\n");
  expect(js).toContain("  // A nested one's,\n  // its parent's.\n  if (prev) {\n");
  expect([lib.shared({ types: 3 } as never), lib.shared(undefined as never), lib.counted({ types: 3 } as never), lib.branched(undefined as never)]).toEqual([3, undefined, 4, 0]);
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
