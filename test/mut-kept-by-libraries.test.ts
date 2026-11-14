// A `&mut` a library's code may keep a raw pointer of is a handle, and one
// it can't, a box (#1781, ADR 0074): each test compiles a library and a
// crate that uses it, and runs what it wrote.

import { beforeAll, expect, test } from "bun:test";
import { join } from "node:path";
import { compiled } from "./crates";
import { buildReact, fixture } from "./support";

beforeAll(buildReact, 600_000);

// #1781: a `&mut` given a function that may keep a raw pointer of it is a
// handle, of what reaches the code that may: React's `describeErrorValue`,
// which writes `name` and `count` through theirs, calls a library given
// neither and holds a crate type's `dyn Fn`, so its box stays. A library
// given it, or a closure that captures it, may keep it.
test("a `&mut` a library's code may get is a handle, and one it can't a box", async () => {
  const dir = fixture("library-given-a-mut");
  const { app } = compiled(dir, `pub fn label(n: u32) -> String {
    format!("#{n}")
}

pub fn run<F: FnOnce()>(f: F) {
    f()
}

thread_local! {
    pub static HELD: std::cell::Cell<*mut u32> = const { std::cell::Cell::new(std::ptr::null_mut()) };
}

pub fn hold(n: &mut u32) {
    HELD.with(|held| held.set(n));
}

pub fn zero_held() {
    HELD.with(|held| unsafe { *held.get() = 0 });
}
`, `use std::cell::Cell;
use std::rc::Rc;

thread_local! {
    static KEPT: Cell<*mut u32> = const { Cell::new(std::ptr::null_mut()) };
}

fn keep(n: &mut u32) {
    KEPT.with(|kept| kept.set(n));
}

fn zero_kept() {
    KEPT.with(|kept| unsafe { *kept.get() = 0 });
}

pub struct Request {
    environment: Rc<dyn Fn() -> u32>,
}

impl Request {
    pub fn new() -> Request {
        Request { environment: Rc::new(|| 1) }
    }
}

fn described(request: &Request, name: &mut String, count: &mut u32) {
    let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        *name = dep::label(*count);
        let environment = Rc::clone(&request.environment);
        *count += environment();
    }));
}

pub fn port_shape() -> (String, u32) {
    let request = Request::new();
    let mut name = "Error".to_owned();
    let mut count = 2;
    described(&request, &mut name, &mut count);
    (name, count)
}

fn kept_by_a_library(n: &mut u32) {
    dep::run(move || keep(n));
}

pub fn stashed(mut n: u32) -> u32 {
    kept_by_a_library(&mut n);
    if n == 0 {
        return 0;
    }
    zero_kept();
    n - 1
}

fn held_by_a_library(n: &mut u32) {
    dep::hold(n);
}

pub fn held(mut n: u32) -> u32 {
    held_by_a_library(&mut n);
    if n == 0 {
        return 0;
    }
    dep::zero_held();
    n - 1
}
`);
  const described = app.slice(app.indexOf("export function portShape"), app.indexOf("function keptByALibrary"));
  expect(described).not.toContain("get value()");
  expect(app.slice(app.indexOf("export function stashed"), app.indexOf("function heldByALibrary"))).toContain("get value()");
  expect(app.slice(app.indexOf("export function held("))).toContain("get value()");
  const lib = await import(join(dir, "app", "lib.js"));
  expect(lib.portShape()).toEqual(["#2", 3]);
  expect(lib.stashed(5)).toBe(4294967295);
  expect(lib.held(7)).toBe(4294967295);
});
