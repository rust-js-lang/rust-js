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

// ADR 0858: a comment before the `None` a function gives at its end, which
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
