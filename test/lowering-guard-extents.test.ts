// How long a `RefCell`'s guard is held around a call: one that's only the
// call's receiver while the call runs, not while its arguments are made;
// one read only for a binding's argument while it's read, not while the
// binding runs. Each test is a program of its own, compiled and run.

import { beforeAll, expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { buildReact, compiler, fixture, run, target } from "./support";

beforeAll(buildReact, 600_000);

const withBindings = [
  "--",
  "--extern",
  `js=${join(target, "libjs.rmeta")}`,
  "--extern",
  `webapi=${join(target, "libwebapi.rmeta")}`,
  "-L",
  target,
];

// ADR 0863, amended, the user's decision (2026-11-01): a guard that's only
// a method call's receiver, `target.borrow_mut().push(show(value))`, is
// held while the method runs, not while its other arguments do, as JS's
// `target.push(show(value))` evaluates them first. So a `show` JS runs,
// which may call back and push to `target`, leaves the array uncounted, as
// React DOM's Fizz config's `stringToChunk(escapeTextForBrowser(value))` of
// a prop. A borrow of the cell the program wrote in an argument, which
// Rust's order meets, is still counted, and panics as Rust's does.
test("a guard that's only a call's receiver is held while the call runs, not its arguments", async () => {
  const dir = fixture("receiver-guard");
  writeFileSync(join(dir, "lib.rs"), `use std::cell::RefCell;

#[cfg_attr(rust_js, rust_js::link_name = "globalThis.show")]
fn show(value: &js::Unknown) -> String {
    let _ = value;
    unreachable!()
}

pub struct Segment {
    pub chunks: RefCell<Vec<String>>,
}

pub fn segment() -> Segment {
    Segment { chunks: RefCell::new(Vec::new()) }
}

pub fn push_text(segment: &Segment, text: &str) {
    segment.chunks.borrow_mut().push(text.to_owned());
}

pub fn push_value(segment: &Segment, value: &js::Unknown) {
    segment.chunks.borrow_mut().push(show(value));
}

pub fn written(segment: &Segment) -> String {
    segment.chunks.borrow().concat()
}

pub struct Counts {
    pub seen: RefCell<Vec<usize>>,
}

pub fn counts() -> Counts {
    Counts { seen: RefCell::new(Vec::new()) }
}

pub fn count(counts: &Counts) {
    counts.seen.borrow_mut().push(counts.seen.borrow().len());
}

pub struct Tally {
    pub items: RefCell<Vec<u8>>,
}

pub fn tally() -> Tally {
    Tally { items: RefCell::new(Vec::new()) }
}

fn len(tally: &Tally) -> usize {
    tally.items.borrow().len()
}

fn size(tally: &Tally) -> u8 {
    len(tally) as u8
}

pub fn tally_size(tally: &Tally) {
    tally.items.borrow_mut().push(size(tally));
}

pub struct Marks {
    pub items: RefCell<Vec<i8>>,
}

pub fn marks() -> Marks {
    Marks { items: RefCell::new(Vec::new()) }
}

fn add(items: &mut Vec<i8>, marks: &Marks) {
    items.push(marks.items.borrow().len() as i8);
}

pub fn mark(marks: &Marks) {
    add(&mut marks.items.borrow_mut(), marks);
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withBindings]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("segment.chunks.push(globalThis.show(value));");
  expect(js).not.toContain("$borrowMut(segment.chunks");
  expect(js).toContain("$borrowMut(counts.seen");
  expect(js).toContain("$borrowMut(tally.items");
  expect(js).toContain("$borrowMut(marks.items");
  const lib = await import(join(dir, "lib.js"));
  const globals = globalThis as any;
  const segment = lib.segment();
  // A value whose text pushes to the array it's pushed to, once: JS's
  // order, the callback's push first.
  let pushed = false;
  globals.show = (value: string) => {
    if (!pushed) {
      pushed = true;
      lib.pushText(segment, "<b>");
    }
    return value;
  };
  try {
    lib.pushValue(segment, "x");
    expect(lib.written(segment)).toBe("<b>x");
  } finally {
    delete globals.show;
  }
  // A borrow of the cell the program wrote, in an argument, through a
  // call, or in the call: Rust's order meets it.
  expect(() => lib.count(lib.counts())).toThrow("RefCell already mutably borrowed");
  expect(() => lib.tallySize(lib.tally())).toThrow("RefCell already mutably borrowed");
  expect(() => lib.mark(lib.marks())).toThrow("RefCell already mutably borrowed");
});

// rd427, rd383 (ADR 0863, amended, the user's decision, 2026-11-02): a guard
// read only for a binding's argument, what it guards lent, `as_deref()`'s,
// or a copy or clone of it, is held while that's read, not while the
// binding runs, as JS's `prepare(response.nonce)` reads it first: React's
// Flight client's `prepareDestinationForModule(response._moduleLoading,
// response._nonce, metadata)` and its DEV `then`'s `wrapperPromise._debugInfo
// = this._debugInfo`. So a binding that calls back and writes the cell meets
// no borrow, where Rust's would panic. A later argument's call of the
// crate's that writes it runs while it's held, and is still counted, and
// panics as Rust's does (`argument_guard_reentered`, against native Rust).
test("a guard read only for a binding's argument is held while it's read, not while the binding runs", async () => {
  const dir = fixture("argument-guard");
  writeFileSync(join(dir, "lib.rs"), `use std::cell::RefCell;

use js::Unknown;

#[cfg_attr(rust_js, rust_js::link_name = "globalThis.prepare")]
fn prepare(loading: Option<&'static Unknown>, nonce: Option<&str>) {
    let _ = (loading, nonce);
    unreachable!()
}

#[cfg_attr(rust_js, rust_js::link_name = "globalThis.logTime")]
fn log_time(time: f64, nonce: String) {
    let _ = (time, nonce);
    unreachable!()
}

pub struct Response {
    pub nonce: RefCell<Option<String>>,
    pub time: RefCell<f64>,
}

pub fn response(nonce: Option<String>) -> Response {
    Response {
        nonce: RefCell::new(nonce),
        time: RefCell::new(1.5),
    }
}

pub fn set_nonce(response: &Response, nonce: Option<String>) {
    *response.nonce.borrow_mut() = nonce;
    *response.time.borrow_mut() += 1.0;
}

pub fn resolve(response: &Response, loading: Option<&'static Unknown>) {
    prepare(loading, response.nonce.borrow().as_deref());
}

pub fn stamp(response: &Response, target: &Unknown) {
    js::set(target, "nonce", response.nonce.borrow().clone());
}

pub fn log(response: &Response) {
    log_time(*response.time.borrow(), String::from("now"));
}

fn renamed(response: &Response) -> String {
    set_nonce(response, Some(String::from("later")));
    String::from("renamed")
}

pub fn log_renamed(response: &Response) {
    log_time(*response.time.borrow(), renamed(response));
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withBindings]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("  globalThis.prepare(loading, response.nonce);\n");
  expect(js).toContain("  target.nonce = response.nonce;\n");
  expect(js).not.toContain('$fieldCell(response, "nonce")');
  expect(js).toContain('  globalThis.logTime($borrow($fieldCell(response, "time")).value, "now");\n');
  expect(js).toContain('$borrow($fieldCell(response, "time"), true)');
  const lib = await import(join(dir, "lib.js"));
  const globals = globalThis as any;
  const response = lib.response("first");
  const given: unknown[] = [];
  // The accepted difference: a binding that calls back and writes the cell
  // it was given a value of, which Rust's borrow would meet with a panic.
  globals.prepare = (_loading: unknown, nonce: string) => {
    given.push(nonce);
    lib.setNonce(response, "second");
  };
  globals.logTime = (time: number, nonce: string) => given.push([time, nonce]);
  try {
    lib.resolve(response, undefined);
    const target: any = {};
    lib.stamp(response, target);
    lib.log(response);
    expect(given).toEqual(["first", [2.5, "now"]]);
    expect(target.nonce).toBe("second");
    expect(() => lib.logRenamed(response)).toThrow("RefCell already borrowed");
  } finally {
    delete globals.prepare;
    delete globals.logTime;
  }
});
