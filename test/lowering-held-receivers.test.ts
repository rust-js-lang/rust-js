// A function a field holds, called with what it's given needing statements
// first: its object is held, `const d = internals.d; ..; d.L(..)`, so the
// call keeps its `this` (ADR 0262's exception, rd285). Each test is a
// program of its own, compiled and run.

import { beforeAll, expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { buildReact, compiler, fixture, run, target } from "./support";

beforeAll(buildReact, 600_000);

/** `rust` compiled in a fixture of `name`, against the js crate: its JS
 * and its module. */
async function compiled(name: string, rust: string): Promise<{ js: string; lib: Record<string, (...args: never[]) => unknown> }> {
  const dir = fixture(name);
  writeFileSync(join(dir, "lib.rs"), rust);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  return { js, lib: await import(join(dir, "lib.js")) };
}

const dispatcher = `use std::marker::PhantomData;

use js::{Kind, Unknown};

pub struct Internals(PhantomData<js::JsObject>);

pub struct Dispatcher {
    #[cfg_attr(rust_js, rust_js::name = "L")]
    pub preload: &'static dyn Fn(&str, Option<&'static str>),
}

impl Internals {
    #[cfg_attr(rust_js, rust_js::link_name = "get d")]
    pub fn d(&self) -> &'static Dispatcher {
        unreachable!()
    }
}
`;

// React DOM's Float, `ReactDOMSharedInternals.d.L(href, as, {integrity:
// typeof options.integrity === 'string' ? .. })`: the call's argument needs
// statements, so the dispatcher is held and called as its method, `this`
// the dispatcher, as upstream's is, where a held `L` would be called with
// none. Its `L` is read after `options.integrity`, where Rust reads it
// first: ADR 0262's exception, the user's (2026-10-31).
test("a field's function called with arguments that need statements keeps its object", async () => {
  const { js, lib } = await compiled(
    "held-receivers",
    `${dispatcher}
pub fn preload(internals: &Internals, href: &str, options: &Unknown) {
    (internals.d().preload)(
        href,
        match js::classify(js::get(options, "integrity")) {
            Kind::String(integrity) => Some(integrity),
            _ => None,
        },
    );
}

pub fn plain(internals: &Internals, href: &str) {
    (internals.d().preload)(href, None);
}

pub fn given(d: &Dispatcher, href: &str, options: &Unknown) {
    (d.preload)(
        href,
        match js::classify(js::get(options, "integrity")) {
            Kind::String(integrity) => Some(integrity),
            _ => None,
        },
    );
}

pub struct Inner {
    pub f: fn(u32) -> u32,
}

pub struct Outer {
    pub inner: Inner,
}

fn double(n: u32) -> u32 {
    n * 2
}

fn triple(n: u32) -> u32 {
    n * 3
}

pub struct Hooks {
    pub f: std::cell::Cell<fn(u32) -> u32>,
}

pub fn celled(n: u32) -> u32 {
    let hooks = Hooks { f: std::cell::Cell::new(double) };
    called(&hooks, n)
}

fn called(hooks: &Hooks, n: u32) -> u32 {
    (hooks.f.get())({
        hooks.f.set(triple);
        n
    })
}

pub fn swapped(n: u32) -> u32 {
    let mut outer = Outer { inner: Inner { f: double } };
    let inner = &mut outer.inner;
    (inner.f)({
        inner.f = triple;
        n
    })
}
`,
  );
  expect(js).toContain("const d = internals.d;");
  expect(js).toMatch(/d\.L\(href, /);
  expect(js).not.toMatch(/const \w+ = internals\.d\.L;/);
  expect(js).toContain("internals.d.L(href);");
  // Of a variable, read in place: nothing between assigns it.
  expect(js).toContain("export function given(d, href, options) {\n  const integrity = options.integrity;\n  d.L(href, ");
  // What Rust can change between, read where Rust reads it: of a `&mut`,
  // and a cell's.
  expect(js).toContain("const f = inner.f;\n  inner.f = triple;\n  return f(n);");
  expect([(lib.swapped as (n: number) => number)(5), (lib.celled as (n: number) => number)(5)]).toEqual([10, 10]);
  const seen: unknown[] = [];
  const d = {
    L(this: unknown, href: string, integrity?: string) {
      seen.push([this === d, href, integrity]);
    },
  };
  (lib.preload as (internals: unknown, href: string, options: unknown) => void)({ d }, "a.css", { integrity: "sha" });
  (lib.preload as (internals: unknown, href: string, options: unknown) => void)({ d }, "b.css", { integrity: 1 });
  (lib.plain as (internals: unknown, href: string) => void)({ d }, "c.css");
  (lib.given as (d: unknown, href: string, options: unknown) => void)(d, "d.css", { integrity: "x" });
  expect(seen).toEqual([
    [true, "a.css", "sha"],
    [true, "b.css", undefined],
    [true, "c.css", undefined],
    [true, "d.css", "x"],
  ]);
});

// One a with_this function of its signature may be is called with no
// `this`, as Rust calls it (ADR 0972, amended): its function held, `const
// f = s.f; f(..)`, whatever its arguments need.
test("a field's function a with_this function may be is still held", async () => {
  const { js, lib } = await compiled(
    "held-receivers-with-this",
    `use js::{Kind, Unknown};

pub struct S {
    pub f: &'static dyn Fn(Option<&'static str>) -> bool,
}

pub fn field(options: &Unknown) -> bool {
    let s: &'static S = Box::leak(Box::new(S {
        f: js::function::with_this(|this: Option<&'static Unknown>, (_,): (Option<&'static str>,)| this.is_some()),
    }));
    called(s, options)
}

fn called(s: &S, options: &Unknown) -> bool {
    (s.f)(match js::classify(js::get(options, "integrity")) {
        Kind::String(integrity) => Some(integrity),
        _ => None,
    })
}
`,
  );
  expect(js).toContain("  const f = s.f;\n  return f(typeof integrity");
  expect([(lib.field as (o: unknown) => boolean)({ integrity: "x" })]).toEqual([false]);
});
