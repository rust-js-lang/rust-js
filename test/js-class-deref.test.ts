// A JS class's `Deref` to the class it extends, as webapi declares one for
// each interface (ADR 0024): the object itself wherever it's used, never a
// function of its own. Each test is a program of its own, compiled.

import { beforeAll, expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { buildReact, compiler, fixture, run, target } from "./support";

// What builds the js crate the program is compiled against.
beforeAll(buildReact, 600_000);

// A JS class's `Deref` to the class it extends, which webapi declares for
// each, is the object itself where it's used, and its body, a pointer cast
// that never runs, is no JS: lowered, it would be a function that makes a
// pointer to nothing.
test("a JS class's Deref to the class it extends is no JS", () => {
  const withJs = ["--", "--crate-type", "lib", "--edition", "2024", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];
  const dir = fixture("js-class-deref");
  writeFileSync(join(dir, "lib.rs"), `use std::marker::PhantomData;
use std::ops::Deref;
use js::JsObject;

pub struct Base(PhantomData<JsObject>);
pub struct Derived(PhantomData<JsObject>);

impl Deref for Derived {
    type Target = Base;
    fn deref(&self) -> &Base {
        unsafe { &*(self as *const Self as *const Base) }
    }
}

#[cfg_attr(rust_js, rust_js::link_name = "name")]
fn base_name(_b: &Base) -> String {
    unreachable!()
}

pub fn named(d: &Derived) -> String {
    base_name(d)
}

pub fn as_fn(d: &Derived) -> &Base {
    let f: fn(&Derived) -> &Base = Deref::deref;
    f(d)
}

fn generic<T: Deref<Target = Base>>(t: &T) -> String {
    base_name(t)
}

pub fn through(d: &Derived) -> String {
    generic(d)
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withJs]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("return name(d);");
  // Its `deref` as a value is the object itself, not a function of its own.
  expect(js).toContain("const f = (x) => x;");
  // Met by a generic function's bound, its dictionary's `deref` is too
  // (#403), where rust-js panicked looking for the impl's function.
  expect(js).toContain("return generic(d, { deref: (x) => x });");
  expect(js).not.toContain("Deref_deref");
});
