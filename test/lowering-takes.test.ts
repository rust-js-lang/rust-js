// What the compiler writes of a `take()` of a place: an `Option` taken
// then tested is the place tested and taken where it's `Some`, upstream's
// `if (x.f !== null) { const c = x.f; x.f = null; c(); }` (ADR 0996), and
// what a take writes of a `#[rust_js::nullable]` field, `null`. Each test
// is a program of its own, compiled and run.

import { expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { compiler, fixture, run } from "./support";

// ADR 0996: React's prepareFreshStack writes `if (cancelPendingCommit !==
// null) { root.cancelPendingCommit = null; cancelPendingCommit(); }`. A take
// of a place rust-js made, a crate struct's field, a `Cell`'s, or a
// variable, then tested, is the place tested and taken where it's `Some`:
// nothing but the binding reads it between, it reads the same twice, and
// where it's `None`, `None` written over it changes nothing. One whose
// binding is read past its test keeps the take first.
test("an Option taken then tested is the place tested, taken where it's Some", async () => {
  const dir = fixture("taken-tested");
  writeFileSync(join(dir, "lib.rs"), `use std::cell::Cell;

pub struct Root {
    pub cancel: Cell<Option<fn() -> u32>>,
    #[cfg_attr(rust_js, rust_js::nullable)]
    pub pending: Option<Vec<u32>>,
}

pub fn prepare(root: &Root) -> u32 {
    if let Some(cancel) = root.cancel.take() {
        return cancel();
    }
    0
}

pub fn drain(root: &mut Root) -> usize {
    if let Some(pending) = root.pending.take() {
        pending.len()
    } else {
        root.pending.as_ref().map_or(7, |p| p.len())
    }
}

pub fn kept(root: &Root) -> Option<fn() -> u32> {
    let cancel = root.cancel.take();
    if let Some(f) = cancel {
        f();
    }
    cancel
}

pub fn local(mut slot: Option<u32>) -> (u32, Option<u32>) {
    if let Some(n) = slot.take() {
        return (n, slot);
    }
    (0, slot)
}

pub fn emptied(root: &mut Root) -> Option<Vec<u32>> {
    std::mem::take(&mut root.pending)
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("export function prepare(root) {\n  if (root.cancel != null) {\n    const cancel = root.cancel;\n    root.cancel = undefined;\n    return cancel();\n  }\n  return 0;\n}");
  expect(js).toContain("  if (root.pending != null) {\n    const pending = root.pending;\n    root.pending = null;\n    return pending.length;\n  } else {\n");
  expect(js).toContain("  const cancel = root.cancel;\n  root.cancel = undefined;\n  if (cancel != null) {\n");
  expect(js).toContain("  if (slot != null) {\n    const n = slot;\n    slot = undefined;\n    return [n, slot];\n  }\n");
  // A nullable field's `None` is `null`, as each of its writes makes it.
  expect(js).toContain("export function emptied(root) {\n  const old = root.pending;\n  root.pending = null;\n");
  const lib = await import(join(dir, "lib.js"));
  const root = { cancel: () => 5, pending: [1, 2] };
  expect([lib.prepare(root), root.cancel, lib.prepare(root), lib.drain(root), root.pending, lib.drain(root)]).toEqual([5, undefined, 0, 2, null, 7]);
  const f = () => 3;
  expect([lib.kept({ cancel: f }), lib.kept({ cancel: undefined }), lib.local(4), lib.local(undefined)]).toEqual([f, undefined, [4, undefined], [0, undefined]]);
  const full = { cancel: undefined, pending: [9] };
  expect([lib.emptied(full), full.pending]).toEqual([[9], null]);
});
