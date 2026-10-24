// An or-pattern of an enum's variants, `matches!(payload.status.get(),
// Status::Pending | Status::Uninitialized)`: each variant tested in the
// order the program wrote it, of a field read where it's tested, as React's
// ReactLazy.js writes `payload._status === Pending || payload._status ===
// Uninitialized`. Each test is a program of its own, compiled and run.

import { beforeAll, expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { buildReact, compiler, fixture, run, target } from "./support";

beforeAll(buildReact, 600_000);

function compiled(name: string, rust: string): { js: string; dir: string } {
  const dir = fixture(name);
  writeFileSync(join(dir, "lib.rs"), rust);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  return { js: readFileSync(join(dir, "lib.js"), "utf8"), dir };
}

test("an or-pattern's variants are tested in the order written, of a field read in place", async () => {
  const { js, dir } = compiled("or-pattern-order", `use std::cell::Cell;

#[derive(Clone, Copy, PartialEq)]
pub enum Status {
    Uninitialized,
    Pending,
    Resolved,
    Rejected,
}

pub struct Payload {
    pub status: Cell<Status>,
}

pub fn make() -> Payload {
    Payload {
        status: Cell::new(Status::Uninitialized),
    }
}

pub fn settle(payload: &Payload) -> bool {
    if matches!(payload.status.get(), Status::Pending | Status::Uninitialized) {
        payload.status.set(Status::Resolved);
        return true;
    }
    false
}

pub fn waiting(payload: &Payload) -> bool {
    matches!(payload.status.get(), Status::Rejected | Status::Pending | Status::Uninitialized)
}

pub fn named(payload: &Payload) -> &'static str {
    match payload.status.get() {
        Status::Resolved | Status::Pending => "on",
        Status::Rejected | Status::Uninitialized => "off",
    }
}
`);
  expect(js).toContain(`export function settle(payload) {
  if (payload.status === "Pending" || payload.status === "Uninitialized") {
    payload.status = "Resolved";
    return true;
  }
  return false;
}`);
  expect(js).toContain(`  return (
    payload.status === "Rejected" ||
    payload.status === "Pending" ||
    payload.status === "Uninitialized"
  );`);
  expect(js).toContain(`  if (payload.status === "Resolved" || payload.status === "Pending") {
    return "on";
  }`);
  const lib = await import(join(dir, "lib.js"));
  const payload = lib.make();
  expect([lib.waiting(payload), lib.named(payload), lib.settle(payload), payload.status]).toEqual([true, "off", true, "Resolved"]);
  expect([lib.waiting(payload), lib.named(payload), lib.settle(payload)]).toEqual([false, "on", false]);
});

// A value made once that does something, a call's, stays held: each test
// reads what it made, not a call made again.
test("an or-pattern of a call's value tests what it gave once", async () => {
  const { js } = compiled("or-pattern-call", `use std::cell::Cell;

#[derive(Clone, Copy, PartialEq)]
pub enum Status {
    Uninitialized,
    Pending,
    Resolved,
}

thread_local! {
    static CALLS: Cell<u32> = const { Cell::new(0) };
}

fn status(n: u32) -> Status {
    CALLS.with(|calls| calls.set(calls.get() + 1));
    if n == 0 { Status::Pending } else { Status::Resolved }
}

pub fn waiting(n: u32) -> u32 {
    let waits = matches!(status(n), Status::Pending | Status::Uninitialized);
    CALLS.with(|calls| calls.get() * 10 + waits as u32)
}
`);
  expect(js).toMatch(/const (\w+) = status\(n\);\n\s+const waits = \1 === "Pending" \|\| \1 === "Uninitialized";/);
});

// A binding's property, which a getter may answer, is read once, as Rust
// reads it: each test reads what it gave, not the getter called again.
test("an or-pattern of a getter's value reads it once", async () => {
  const { js, dir } = compiled("or-pattern-getter", `pub trait Thenable {
    #[cfg_attr(rust_js, rust_js::link_name = "get state")]
    fn state(&self) -> u32 {
        unreachable!()
    }
}

pub fn settled<T: Thenable>(thenable: &T) -> bool {
    matches!(thenable.state(), 2 | 1)
}
`);
  expect(js).toContain(`const state = thenable.state;
  return state === 2 || state === 1;`);
  const lib = await import(join(dir, "lib.js"));
  let reads = 0;
  const thenable = { get state() { reads += 1; return reads; } };
  expect([lib.settled(thenable), reads]).toEqual([true, 1]);
});
