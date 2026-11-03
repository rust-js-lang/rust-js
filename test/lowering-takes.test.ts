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

// A take or a replace of a place, whose old value nothing reads and whose
// drop does nothing, is the write alone, `slot.callback = undefined;`: the
// old value lowering reads first, `const old = slot.callback;`, tidy
// leaves out, as it's plain data, never a getter's (`unread_made`).
test("a take or replace of a place whose old value nothing reads is the write alone", async () => {
  const dir = fixture("taken-unread");
  writeFileSync(join(dir, "lib.rs"), `pub struct Slot {
    pub callback: Option<fn() -> u32>,
    pub label: String,
    pub n: u32,
}

pub fn cleared(slot: &mut Slot) {
    slot.callback.take();
}

pub fn renamed(slot: &mut Slot, label: String) {
    let _ = std::mem::replace(&mut slot.label, label);
}

pub fn emptied(slot: &mut Slot) {
    std::mem::take(&mut slot.label);
}

pub fn set(slot: &mut Slot, callback: fn() -> u32, n: u32) {
    slot.callback.replace(callback);
    let _ = std::mem::replace(&mut slot.n, n);
}

pub fn dropped(callback: &mut Option<fn() -> u32>) {
    callback.take();
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("export function cleared(slot) {\n  slot.callback = undefined;\n}");
  expect(js).toContain("export function renamed(slot, label) {\n  slot.label = label;\n}");
  expect(js).toContain('export function emptied(slot) {\n  slot.label = "";\n}');
  expect(js).toContain("export function set(slot, callback, n) {\n  slot.callback = callback;\n  slot.n = n;\n}");
  expect(js).toContain("export function dropped(callback) {\n  callback.value = undefined;\n}");
  const lib = await import(join(dir, "lib.js"));
  const f = () => 1;
  const slot: Record<string, unknown> = { callback: f, label: "a", n: 1 };
  lib.cleared(slot);
  const cleared = { ...slot };
  lib.set(slot, f, 7);
  lib.renamed(slot, "b");
  const renamed = { ...slot };
  lib.emptied(slot);
  const held = { value: f };
  lib.dropped(held);
  expect([cleared, renamed, slot]).toEqual([
    { callback: undefined, label: "a", n: 1 },
    { callback: f, label: "b", n: 7 },
    { callback: f, label: "", n: 7 },
  ]);
  expect(held.value).toBeUndefined();
});
