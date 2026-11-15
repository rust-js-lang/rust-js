// A type's depth costs its compile linearly (#1821). A chain of enums, each
// of four variants holding the one before, deriving `Clone`, `Debug`,
// `PartialEq`, `Eq`, `Hash`, `PartialOrd`, `Ord` and `Default`, beside one
// whose payload has a `Drop` of its own, cloned out of an `Rc<RefCell>`,
// compared, hashed, ordered in a B-tree, made by its `Default` and dropped:
// each question rust-js asks of a type answered once a type, not once a
// way to it, and each variant's copy, clone or comparison written once for
// the variants of the same fields. Twenty deep, the metamorphic net's
// rc-refcell rewrite of nested_type_chain.rs never finished compiling,
// its JS growing as 4 to the power of its depth. Its lines are counted, as
// a run's time isn't the same twice: each doubling of the depth at most
// eight times the lines, a quadratic's, as each level's path is longer.

import { beforeAll, expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { underLoad } from "../scripts/timing";
import { buildCompiler, compiler, fixture, run } from "./support";

beforeAll(buildCompiler, 600_000);

/** The chain `depth` enums deep, and a `main` that asks each question of
 * its last. */
export function chain(depth: number): string {
  const last = depth - 1;
  const links = Array.from({ length: depth - 1 }, (_, i) => `chain!(C${i + 1} C${i}); dropping!(E${i + 1} E${i});`);
  return `use std::cell::RefCell;
use std::collections::{BTreeSet, HashSet};
use std::rc::Rc;

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
struct Leaf(u32);

macro_rules! chain {
    ($cur:ident $prev:ty) => {
        #[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
        enum $cur {
            #[default]
            Empty,
            First($prev),
            Second($prev),
            Third($prev),
            Fourth($prev),
        }
    };
}

struct D(u32);

impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

macro_rules! dropping {
    ($cur:ident $prev:ty) => {
        enum $cur {
            Empty,
            First($prev),
            Second($prev),
            Third($prev),
            Fourth($prev),
        }
    };
}

chain!(C0 Leaf);
dropping!(E0 D);
${links.join("\n")}

fn main() {
    let shared = Rc::new(RefCell::new(C${last}::Second(C${last - 1}::First(Default::default()))));
    let a = (*shared.borrow()).clone();
    let b = a.clone();
    println!("{} {:?} {}", a == b, a.cmp(&b), a < b);
    let mut hashed = HashSet::new();
    hashed.insert(a.clone());
    let mut sorted = BTreeSet::new();
    sorted.insert(b);
    println!("{} {} {:?} {:?}", hashed.contains(&a), sorted.len(), C${last}::default(), shared.borrow());
    let held = vec![E${last}::Third(E${last - 1}::First(E${last - 2}::Empty)), E${last}::Empty];
    drop(held);
}
`;
}

/** The lines of `depth`'s JS, each build's. */
function lines(depth: number, flags: string[]): number {
  const dir = fixture(`type-depth-${depth}${flags.join("")}`);
  const source = join(dir, "main.rs");
  writeFileSync(source, chain(depth));
  // Well within a minute: forty deep took 2 s, where twenty never ended.
  run([compiler, ...flags, source, "-o", join(dir, "main.js")], 60_000);
  return readFileSync(join(dir, "main.js"), "utf8").split("\n").length;
}

for (const [build, flags] of [
  ["plain", []],
  ["std's iterator structs", ["--std-iterators"]],
] as const) {
  test(`an enum chain's JS and its compile grow with its depth, not as its variants to its power, ${build}`, () => {
    const [ten, twenty, forty] = [10, 20, 40].map((depth) => lines(depth, [...flags]));
    expect(twenty / ten).toBeLessThan(8);
    expect(forty / twenty).toBeLessThan(8);
  }, 300_000);
}

/** A `Defer`'s drop, generic in its closure, that calls a chain of generic
 * functions `depth` long, each calling the one below it twice: a call
 * graph of 2 to the power of its depth paths to its last function. */
export function calls(depth: number): string {
  const chain = Array.from({ length: depth - 1 }, (_, i) => `fn g${i + 1}<F: FnMut()>(f: &mut F) {\n    g${i}(f);\n    g${i}(f);\n}`);
  return `struct Defer<F: FnMut()>(F);

impl<F: FnMut()> Drop for Defer<F> {
    fn drop(&mut self) {
        g${depth - 1}(&mut self.0)
    }
}

fn g0<F: FnMut()>(f: &mut F) {
    f()
}

${chain.join("\n\n")}

fn main() {
    let mut n = 0u64;
    {
        let _d = Defer(|| n += 1);
    }
    println!("{n}");
}
`;
}

// Whether a generic function's drop, given its arguments, never unwinds
// walks the calls it reaches: each instance is judged once a walk, where
// it was once for each way to it, 2.2 s twenty deep, twice that each
// level more (#1821's follow-up, ADR 1160).
test("a generic drop's calls are each judged once, not once for each way to them", () => {
  const dir = fixture("type-depth-calls");
  const source = join(dir, "main.rs");
  writeFileSync(source, calls(40));
  run([compiler, source, "-o", join(dir, "main.js")], 60_000);
}, underLoad(120_000));
