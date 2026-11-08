// The async table (#942): panic points across `.await`. Each family of
// async bodies, values live across one await or more, made or given by an
// async fn, held by an async block, an `async move` block or an async
// closure, in a loop, past an early return by `?`, of each shape of value
// it holds, a case. Each case runs once a point, as the unwind table's do
// (#854): every make and drop of a `D`, every drop of a struct with a
// `Drop` of its own, and every await of the crate's `leaf` is the k-th
// point once, which panics. test/iterator-matrix.test.ts runs it
// (`ITERATOR_TABLE=async`) in both builds against native Rust.
//
// Within what rust-js maps futures to (the guide's async): a future is a
// promise, which runs as it's made, where Rust's waits until it's polled.
// That's kept where nothing can tell (ADR 0029, amended, #965): a future
// awaited where it's made, `f().await`, an async block's too, one held past
// only what can't be seen, or one whose body can't be seen, dropped
// unawaited too; rust-js refuses the rest, held past what may be seen or
// dropped unawaited, each a refusal here. std has no `join` or `select`.
// Natively
// each run is driven by a `block_on` of no waker, each future ready when
// it's first polled; as JS, awaited by `js::settle`
// (`asyncPointsRunner`).

import type { Row } from "./iterator-matrix";
import { asyncPointsRunner } from "./panic-points";

/** A shape of value: its type, and how one numbered `n` is made. */
type Shape = { name: string; ty: string; make: (n: string) => string };

export const shapes: Shape[] = [
  { name: "D", ty: "D", make: (n) => `d(${n})` },
  { name: "struct", ty: "P", make: (n) => `P { a: d(${n}), b: d(${n} + 100) }` },
  { name: "struct with Drop", ty: "W", make: (n) => `W { a: d(${n}), b: d(${n} + 100) }` },
  { name: "tuple", ty: "(D, D)", make: (n) => `(d(${n}), d(${n} + 100))` },
  { name: "Box", ty: "Box<D>", make: (n) => `Box::new(d(${n}))` },
  { name: "Option", ty: "Option<D>", make: (n) => `Some(d(${n}))` },
  { name: "Vec", ty: "Vec<D>", make: (n) => `vec![d(${n}), d(${n} + 100)]` },
];

/** A family of async bodies: its statements, of its shape's maker `M` and
 * type `T`. Each future it makes is awaited where it's made. */
type Family = { name: string; body: (M: (n: number | string) => string, T: string) => string };

export const families: Family[] = [
  { name: "across an await", body: (M) => `let a = ${M(1)};\nleaf(1).await;\nlet _ = &a;` },
  {
    name: "across awaits, one dropped between",
    body: (M) => `let a = ${M(1)};\nleaf(1).await;\nlet b = ${M(2)};\nleaf(2).await;\ndrop(a);\nleaf(3).await;\nlet _ = &b;`,
  },
  {
    name: "made by an async fn",
    body: (M, T) => `async fn made(n: u32) -> ${T} {\n    leaf(n).await;\n    ${M("n")}\n}\nlet a = made(1).await;\nleaf(2).await;\nlet _ = &a;`,
  },
  {
    name: "given to an async fn",
    body: (M, T) => `async fn given(x: ${T}) {\n    leaf(9).await;\n    let _ = &x;\n}\nlet a = ${M(1)};\ngiven(${M(2)}).await;\nleaf(3).await;\nlet _ = &a;`,
  },
  {
    name: "in an async block",
    body: (M) => `let a = ${M(1)};\nlet b = async {\n    let c = ${M(2)};\n    leaf(1).await;\n    c\n}\n.await;\nleaf(2).await;\nlet _ = (&a, &b);`,
  },
  { name: "moved into an async move block", body: (M) => `let a = ${M(1)};\nasync move {\n    leaf(1).await;\n    let _ = &a;\n}\n.await;\nleaf(2).await;` },
  { name: "in a loop", body: (M) => `for i in 0..2 {\n    let a = ${M("i")};\n    leaf(i).await;\n    let _ = &a;\n}` },
  {
    name: "past an early return",
    body: (M) =>
      `async fn early(fail: bool) -> Result<u32, u32> {\n    let a = ${M(1)};\n    leaf(1).await;\n    if fail {\n        Err::<u32, u32>(1)?;\n    }\n    let b = ${M(2)};\n    leaf(2).await;\n    let _ = (&a, &b);\n    Ok(2)\n}\nlet _ = early(true).await;\nlet _ = early(false).await;`,
  },
  { name: "in an async closure", body: (M) => `let f = async |n: u32| {\n    let a = ${M("n")};\n    leaf(n).await;\n    let _ = &a;\n};\nf(1).await;\nf(2).await;` },
  { name: "a block's local across its await", body: (M) => `let a = ${M(1)};\nlet n = {\n    let t = ${M(2)};\n    let n = leaf(3).await;\n    let _ = &t;\n    n\n};\nleaf(n).await;\nlet _ = &a;` },
  {
    name: "an awaited value given to an await",
    body: (M) => `let a = ${M(1)};\nlet n = leaf(leaf(1).await + 1).await;\nlet b = ${M(2)};\nleaf(n).await;\nlet _ = (&a, &b);`,
  },
  // Held and awaited later (#965): past what can't be seen, a ready
  // future's await too, as native Rust; past what may be, refused.
  { name: "held past nothing seen", body: (M) => `let a = ${M(1)};\nlet f = leaf(1);\nlet n = 2 + 3;\nlet _ = f.await + n;\nlet _ = &a;` },
  { name: "held past a ready future's await", body: () => `let f = leaf(1);\nlet n = std::future::ready(2).await;\nlet _ = f.await + n;` },
  { name: "held past what's made", body: (M) => `let f = leaf(1);\nlet a = ${M(2)};\nf.await;\nlet _ = &a;` },
  { name: "held past an await", body: (M) => `let a = ${M(1)};\nlet f = leaf(1);\nleaf(2).await;\nf.await;\nlet _ = &a;` },
  // Dropped unawaited: of a body that can't be seen, as native Rust; of one
  // that may be, refused.
  { name: "dropped unawaited, its body quiet", body: (M) => `async fn five() -> u32 {\n    5\n}\nlet f = five();\nlet a = ${M(1)};\ndrop(f);\nlet _ = &a;` },
  { name: "dropped unawaited", body: (M) => `let a = ${M(1)};\nlet f = leaf(1);\ndrop(f);\nlet _ = &a;` },
  {
    name: "a match on what's awaited",
    body: (M) => `let a = ${M(1)};\nmatch leaf(1).await {\n    1 => {\n        let b = ${M(2)};\n        leaf(2).await;\n        let _ = &b;\n    }\n    _ => {}\n}\nlet _ = &a;`,
  },
];

export type AsyncCase = Row & { family: Family; shape: Shape };

/** Each family of each shape. */
export function asyncCases(): AsyncCase[] {
  const out: AsyncCase[] = [];
  for (const family of families) {
    for (const shape of shapes) out.push({ n: out.length, id: `${family.name} | ${shape.name} | points`, family, shape });
  }
  return out;
}

const indent = (text: string) =>
  text
    .split("\n")
    .map((line) => (line === "" ? "" : `    ${line}`))
    .join("\n");

const prelude = `// Generated by test/async-points.ts (#942). Each case prints its name,
// then each make, drop and await, each panic point, and how each run ended.
use std::sync::atomic::{AtomicU32, Ordering};

struct D(u32);

impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
        point();
    }
}

fn d(n: u32) -> D {
    point();
    println!("make {n}");
    D(n)
}

struct P {
    a: D,
    b: D,
}

struct W {
    a: D,
    b: D,
}

impl Drop for W {
    fn drop(&mut self) {
        println!("drop W {} {}", self.a.0, self.b.0);
        point();
    }
}

/** An await of the crate's own async fn: a point, then what it gives. */
async fn leaf(n: u32) -> u32 {
    point();
    println!("await {n}");
    n
}`;

/** A program of `cases`, which runs each in turn. */
export function asyncProgram(cases: AsyncCase[]): string {
  const fns = cases.map((c, i) => {
    const M = (n: number | string) => c.shape.make(String(n));
    return `async fn case_${i}() {\n${indent(c.family.body(M, c.shape.ty))}\n}`;
  });
  const runAll = `async fn run_all() {\n${cases.map((c, i) => `    run_points!(${JSON.stringify(c.id)}, case_${i});`).join("\n")}\n}`;
  return [prelude + asyncPointsRunner, ...fns, runAll].join("\n\n") + "\n";
}

/** What a run without `ITERATOR_MATRIX` covers: every case, few as they are. */
export const asyncStrata = (c: AsyncCase) => [c.id];
