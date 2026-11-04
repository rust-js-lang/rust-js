// A function's last parameter marked `#[rust_js::rest]`, a slice, is JS's
// rest parameter (ADR 0664): `function formatProdErrorMessage(code,
// ...args)`, which JS calls with any number of arguments, as React's build
// calls it, and the crate's own code with its slice spread. And a closure's
// last parameter of rest arguments as a value, `&dyn js::Rest<T>`, is one
// too (ADR 0851): `(...args) =>`, which each call through its type spreads,
// as React's `cache` gives JS. Each test is a program of its own, compiled
// and run.

import { beforeAll, expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { buildReact, compiler, fixture, run, target } from "./support";

beforeAll(buildReact, 600_000);

const js = ["--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];

// React's `shared/formatProdErrorMessage.js`, as upstream writes it.
const upstream = `export default function formatProdErrorMessage(code) {
  let url = 'https://react.dev/errors/' + code;
  if (arguments.length > 1) {
    url += '?args[]=' + encodeURIComponent(arguments[1]);
    for (let i = 2; i < arguments.length; i++) {
      url += '&args[]=' + encodeURIComponent(arguments[i]);
    }
  }
  return (
    \`Minified React error #\${code}; visit \${url} for the full message or \` +
    'use the non-minified dev environment for full errors and additional ' +
    'helpful warnings.'
  );
}
`;

test("a function's rest slice is JS's rest parameter, and a call spreads the slice it's given", async () => {
  const dir = fixture("rest-parameters");
  writeFileSync(join(dir, "Cargo.toml"), '[package]\nname = "app"\nversion = "0.0.0"\nedition = "2024"\n');
  writeFileSync(join(dir, "lib.rs"), `use js::Unknown;

js::export_default! {
    fn format_prod_error_message(
        code: f64,
        #[cfg_attr(rust_js, rust_js::rest)] args: &[Option<&Unknown>],
    ) -> String {
        let mut url = format!("https://react.dev/errors/{code}");
        for (i, arg) in args.iter().enumerate() {
            url += if i == 0 { "?args[]=" } else { "&args[]=" };
            url += &js::encode_uri_component(&js::string(*arg));
        }
        format!(
            "Minified React error #{code}; visit {url} for the full message or \\
             use the non-minified dev environment for full errors and additional \\
             helpful warnings."
        )
    }
}

pub fn total(#[cfg_attr(rust_js, rust_js::rest)] xs: &[f64]) -> f64 {
    xs.iter().sum()
}

// Named as any function is.
#[cfg_attr(rust_js, rust_js::name = "countAll")]
pub fn count(#[cfg_attr(rust_js, rust_js::rest)] xs: &[f64]) -> usize {
    xs.len()
}

pub struct Point {
    pub x: f64,
}

impl Point {
    pub fn moved(&self, #[cfg_attr(rust_js, rust_js::rest)] by: &[f64]) -> f64 {
        self.x + total(by)
    }
}

fn apply(f: impl Fn(&[f64]) -> f64, v: &[f64]) -> f64 {
    f(v)
}

pub fn sums(v: &[f64]) -> Vec<f64> {
    let pointer: fn(&[f64]) -> f64 = total;
    let named = total;
    vec![
        total(&[1.0, 2.0]),
        total(v),
        total(&v[1..]),
        total(&[]),
        Point { x: 10.0 }.moved(&[1.0, v[0]]),
        apply(total, v),
        pointer(v),
        named(&[v[1]]),
    ]
}
`);
  run([compiler, "--declarations", join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", ...js]);
  const out = readFileSync(join(dir, "lib.js"), "utf8");
  expect(out).toContain("export default function formatProdErrorMessage(code, ...args) {");
  expect(out).toContain("export function total(...xs) {");
  expect(out).toContain("moved(point, ...by) {");
  expect(out).toContain("export function countAll(...xs) {");
  // Written out, its items; held, spread.
  expect(out).toContain("total(1, 2),");
  expect(out).toContain("total(...v),");
  expect(out).toContain("total(...$slice(v, 1)),");
  expect(out).toContain("total(),");
  expect(out).toContain("Point.moved({ x: 10 }, 1, $index(v, 0)),");
  // As a value, what's given it spread.
  expect(out).toContain("const pointer = (x) => total(...x);");
  expect(out).toContain("apply((x) => total(...x), v),");
  // A variable of it is it, called as it is.
  expect(out).toContain("total($index(v, 1)),");
  const declarations = readFileSync(join(dir, "lib.d.ts"), "utf8");
  expect(declarations).toContain("export default function formatProdErrorMessage(code: number, ...args: (unknown | null | undefined)[]): string;");
  expect(declarations).toContain("export function total(...xs: number[]): number;");

  const lib = await import(join(dir, "lib.js"));
  writeFileSync(join(dir, "upstream.js"), upstream);
  const { default: original } = await import(join(dir, "upstream.js"));
  for (const args of [[], ["a b"], ["x", 2, undefined], [null, { toString: () => "&" }, true]]) {
    expect(lib.default(418, ...args)).toBe(original(418, ...args));
  }
  expect(lib.default(31, "object with keys {a, b}")).toBe(
    "Minified React error #31; visit https://react.dev/errors/31?args[]=object%20with%20keys%20%7Ba%2C%20b%7D for the full message or use the non-minified dev environment for full errors and additional helpful warnings.",
  );
  expect(lib.total()).toBe(-0);
  expect(lib.total(1, 2, 3)).toBe(6);
  expect(lib.countAll(1, 2, 3)).toBe(3);
  expect(lib.sums([4, 5])).toEqual([3, 9, 5, -0, 15, 9, 9, 5]);
});

// A library's function is called by another crate as its JS takes it: its
// manifest says its last parameter is a rest, which rustc's metadata of a
// parameter's attributes doesn't.
test("a library's rest function is called with the slice spread", async () => {
  const dir = fixture("rest-parameters-library");
  writeFileSync(join(dir, "dep.rs"), "pub fn total(#[cfg_attr(rust_js, rust_js::rest)] xs: &[f64]) -> f64 {\n    xs.iter().sum()\n}\n");
  run([compiler, join(dir, "dep.rs"), "-o", join(dir, "dep", "lib.js"), "--library", "--manifest", join(dir, "dep", "lib.manifest.json"),
    "--", "--crate-name", "dep", `--emit=metadata=${join(dir, "libdep.rmeta")}`]);
  expect(readFileSync(join(dir, "dep", "lib.js"), "utf8")).toContain("export function total(...xs) {");
  writeFileSync(join(dir, "app.rs"), "pub fn sums(v: &[f64]) -> Vec<f64> {\n    let f: fn(&[f64]) -> f64 = dep::total;\n    vec![dep::total(&[1.0, 2.0]), dep::total(v), f(v)]\n}\n");
  run([compiler, join(dir, "app.rs"), "-o", join(dir, "app", "lib.js"), "--dependency", join(dir, "dep", "lib.manifest.json"),
    "--", "--crate-name", "app", "--extern", `dep=${join(dir, "libdep.rmeta")}`, "-L", dir]);
  const out = readFileSync(join(dir, "app", "lib.js"), "utf8");
  expect(out).toContain("Dep.total(1, 2), Dep.total(...v)");
  const { sums } = await import(join(dir, "app", "lib.js"));
  expect(sums([4, 5])).toEqual([3, 9, 9]);
});

// #540: a function of rest arguments given to what reads it as JS's function,
// `js::function`'s members and `js::unknown_of`, is the function itself,
// whose `name` is its own and which JS calls with its arguments, rest
// gathered; not the arrow of its call, whose `name` is `""` and which would
// spread its second argument. alien-signals' `isSignal` reads
// `signalOper.name`. Where what JS gives back is called with a slice,
// `function::bind`'s, the arrow stays.
test("a function of rest arguments JS reads as a value is the function itself", async () => {
  const dir = fixture("rest-function-itself");
  writeFileSync(join(dir, "dep.rs"), "pub fn total(#[cfg_attr(rust_js, rust_js::rest)] xs: &[f64]) -> f64 {\n    xs.iter().sum()\n}\n");
  run([compiler, join(dir, "dep.rs"), "-o", join(dir, "dep", "lib.js"), "--library", "--manifest", join(dir, "dep", "lib.manifest.json"),
    "--", "--crate-name", "dep", `--emit=metadata=${join(dir, "libdep.rmeta")}`]);
  writeFileSync(join(dir, "lib.rs"), `use std::cell::Cell;
use std::rc::Rc;

use js::{IntoUnknown, Kind, Rest, Unknown, function};

pub struct Node {
    pub flags: Cell<usize>,
}

fn signal_oper(this: &Rc<Node>, #[cfg_attr(rust_js, rust_js::rest)] value: &[Option<&Unknown>]) -> usize {
    this.flags.get() + value.len()
}

pub fn signal(this: Rc<Node>) -> impl Fn(&dyn Rest<dyn IntoUnknown>) -> usize {
    move |value: &dyn Rest<dyn IntoUnknown>| signal_oper(&this, value)
}

pub fn is_signal(r#fn: &dyn Fn(&dyn Rest<dyn IntoUnknown>) -> usize) -> bool {
    function::name(r#fn) == format!("bound {}", function::name(&signal_oper))
}

pub fn total(#[cfg_attr(rust_js, rust_js::rest)] xs: &[f64]) -> f64 {
    xs.iter().sum()
}

fn offset(by: f64, #[cfg_attr(rust_js, rust_js::rest)] xs: &[f64]) -> f64 {
    by + total(xs)
}

pub fn total_name() -> String {
    function::name(&total)
}

pub fn library_name() -> String {
    function::name(&dep::total)
}

pub fn total_length() -> u32 {
    function::length(&total)
}

pub fn total_called() -> Option<&'static Unknown> {
    function::call(&total, None, (1.0, 2.0, 3.0))
}

pub fn total_value() -> &'static Unknown {
    js::unknown_of(total)
}

pub fn offset_bound(by: f64) -> f64 {
    let add = function::bind(offset, Some(&Kind::Null), (by,));
    add(&[1.0, 2.0])
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--dependency", join(dir, "dep", "lib.manifest.json"),
    "--", ...js, "--extern", `dep=${join(dir, "libdep.rmeta")}`, "-L", dir]);
  const out = readFileSync(join(dir, "lib.js"), "utf8");
  expect(out).toContain("return signalOper.bind(null, this$);");
  expect(out).toContain("`bound ${signalOper.name}`");
  expect(out).toContain("return total.name;");
  expect(out).toContain("return Dep.total.name;");
  expect(out).toContain("return total.length;");
  expect(out).toContain("return total.call(undefined, 1, 2, 3);");
  expect(out).toContain("return total;");
  // What `bind` gives Rust is called with a slice: the arrow spreads it.
  expect(out).toMatch(/\(\((\w+), (\w+)\) => offset\(\1, \.\.\.\2\)\)\.bind\(null, by\)/);
  const lib = await import(join(dir, "lib.js"));
  const node = { flags: 1 };
  const signal = lib.signal(node);
  expect(signal.name).toBe("bound signalOper");
  expect(lib.isSignal(signal)).toBe(true);
  expect(lib.isSignal(() => 0)).toBe(false);
  expect([signal(), signal("a", "b")]).toEqual([1, 3]);
  expect([lib.totalName(), lib.libraryName(), lib.totalLength()]).toEqual(["total", "total", 0]);
  expect(lib.totalCalled()).toBe(6);
  expect(lib.totalValue()).toBe(lib.total);
  expect(lib.totalValue()(1, 2, 3)).toBe(6);
  expect(lib.offsetBound(10)).toBe(13);
});

// #565: a generic one given where JS alone reads it, `function::name(&
// signal_oper::<()>)`, gives its type parameter nothing, though `()` may
// look like `None`: it's its own JS, as the closure that passes all on to
// it binds, so `isSignal` compares `signalOper` with `signalOper` and holds.
// It made a copy that boxed its `Some`, `signalOperUnboxed` bound, and
// `isSignal` was false. What `bind` gives Rust is called still, a copy made
// for what it gives.
test("a generic function of rest arguments JS alone reads is its own JS", async () => {
  const dir = fixture("rest-function-itself-generic");
  writeFileSync(join(dir, "lib.rs"), `use std::cell::Cell;
use std::rc::Rc;

use js::{IntoUnknown, Kind, Rest, Unknown, function};

pub struct Node {
    pub current: Cell<Option<&'static Unknown>>,
}

fn signal_oper<T: 'static>(
    this: &Node,
    #[cfg_attr(rust_js, rust_js::rest)] value: &[Option<&'static Unknown>],
) -> Option<T> {
    if let Some(&value) = value.first() {
        this.current.set(value);
        None
    } else {
        // SAFETY: \`signal\` sets only \`T\`s here.
        Some(unsafe { js::cast::<T>(this.current.get()) })
    }
}

pub fn signal<T: 'static>(initial_value: T) -> impl Fn(&dyn Rest<dyn IntoUnknown>) -> Option<T> {
    let this = Rc::new(Node {
        current: Cell::new(js::any_of(initial_value)),
    });
    move |value: &dyn Rest<dyn IntoUnknown>| signal_oper::<T>(&this, value)
}

pub fn is_signal(r#fn: &dyn Fn(&dyn Rest<dyn IntoUnknown>) -> Option<u32>) -> bool {
    function::name(r#fn) == format!("bound {}", function::name(&signal_oper::<()>))
}

fn first<T: 'static>(#[cfg_attr(rust_js, rust_js::rest)] xs: &[Option<&'static Unknown>]) -> Option<T> {
    // SAFETY: \`first_unit\` gives \`()\`s alone.
    xs.first().map(|&x| unsafe { js::cast::<T>(x) })
}

pub fn first_unit() -> bool {
    let at = function::bind(first::<()>, Some(&Kind::Null), ());
    at(&[None]).is_some()
}

fn plain<T: 'static>(x: Option<&'static Unknown>) -> Option<T> {
    // SAFETY: no caller calls it.
    x.map(|x| unsafe { js::cast::<T>(Some(x)) })
}

pub fn plain_name() -> String {
    function::name(&plain::<()>)
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", ...js]);
  const out = readFileSync(join(dir, "lib.js"), "utf8");
  expect(out).toContain("return signalOper.bind(null, this$);");
  expect(out).toContain("`bound ${signalOper.name}`");
  expect(out).not.toContain("signalOperUnboxed");
  const lib = await import(join(dir, "lib.js"));
  const signal = lib.signal(1);
  expect(signal.name).toBe("bound signalOper");
  expect(lib.isSignal(signal)).toBe(true);
  expect([signal(), signal(2), signal()]).toEqual([1, undefined, 2]);
  // `Some(())`, boxed where Rust reads it.
  expect(lib.firstUnit()).toBe(true);
  // One of no rest arguments, its function too, given its `()` as before.
  expect(lib.plainName()).toBe("plain");
});

// Where it isn't JS's rest parameter, it's an error that says what to write.
for (const [name, source, message] of [
  ["not the last parameter", "pub fn f(#[cfg_attr(rust_js, rust_js::rest)] xs: &[f64], n: f64) -> f64 { xs.len() as f64 + n }",
    "`#[rust_js::rest]` is a function's last parameter, as JS's rest parameter is"],
  ["a Vec", "pub fn f(#[cfg_attr(rust_js, rust_js::rest)] xs: Vec<f64>) -> usize { xs.len() }",
    "`#[rust_js::rest]` is a slice's, `xs: &[T]`, not a `Vec<f64>`'s"],
  ["a slice of &mut", "pub fn f(#[cfg_attr(rust_js, rust_js::rest)] xs: &mut [f64]) { xs[0] = 1.0; }",
    "`#[rust_js::rest]` is a slice's, `xs: &[T]`, not a `&mut [f64]`'s"],
  ["an array", "pub fn f(#[cfg_attr(rust_js, rust_js::rest)] xs: &[f64; 2]) -> f64 { xs[0] }",
    "`#[rust_js::rest]` is a slice's, `xs: &[T]`, not a `&[f64; 2]`'s"],
  ["a closure's parameter", "pub fn f() -> usize { let g = |#[cfg_attr(rust_js, rust_js::rest)] xs: &[f64]| xs.len(); g(&[1.0]) }",
    "`#[rust_js::rest]` is a function's last parameter, not a closure's: a closure takes rest arguments as a value, `args: &dyn js::Rest<T>`, which its type says"],
  ["a trait method's parameter", "pub trait T { fn f(&self, xs: &[f64]) -> usize; }\npub struct S;\nimpl T for S { fn f(&self, #[cfg_attr(rust_js, rust_js::rest)] xs: &[f64]) -> usize { xs.len() } }",
    "`#[rust_js::rest]` is a function's last parameter, not a trait method's, which each impl's call gives alike"],
  ["a function given a dictionary", "pub fn f<T: std::fmt::Display>(#[cfg_attr(rust_js, rust_js::rest)] xs: &[T]) -> Vec<String> { xs.iter().map(|x| x.to_string()).collect() }\npub fn g() -> Vec<String> { f(&[1, 2]) }",
    "a function whose last parameter is `#[rust_js::rest]` given a dictionary, a fact, a drop or its caller's location, which JS's rest parameter can't come before"],
  ["a pattern", "pub fn f(#[cfg_attr(rust_js, rust_js::rest)] &[..]: &[f64]) -> usize { 0 }",
    "`#[rust_js::rest]` is a parameter's name, `args: &[T]`, not a pattern's: JS's rest parameter is an array"],
  // What `js::import!` gives is called with the slice whole.
  ["imported", "pub fn total(#[cfg_attr(rust_js, rust_js::rest)] xs: &[f64]) -> f64 { xs.iter().sum() }\npub async fn later() -> f64 { js::import!(total).await(&[1.0]) }",
    "importing a function whose last parameter is `#[rust_js::rest]`"],
] as const) {
  test(`\`#[rust_js::rest]\` refused: ${name}`, () => {
    const dir = fixture(`rest-refused-${name.replaceAll(/\W+/g, "-")}`);
    writeFileSync(join(dir, "lib.rs"), source);
    const refused = Bun.spawnSync([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", ...js]);
    expect(refused.exitCode).not.toBe(0);
    expect(refused.stderr.toString()).toContain(message);
  });
}

// A component is given its props, React's one object.
test("`#[rust_js::rest]` refused: a component's props", () => {
  const dir = fixture("rest-refused-component");
  writeFileSync(join(dir, "lib.rs"), "#![allow(non_snake_case)]\nuse react::{JSX, jsx};\n\npub fn Tags(#[cfg_attr(rust_js, rust_js::rest)] tags: &[&'static str]) -> JSX::Element {\n    jsx! { <p>{tags.len()}</p> }\n}\n");
  const refused = Bun.spawnSync([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.jsx"), "--", "--extern", `react=${join(target, "libreact.rmeta")}`, "-L", target]);
  expect(refused.exitCode).not.toBe(0);
  expect(refused.stderr.toString()).toContain("`#[rust_js::rest]` is a function's last parameter, not a component's, which React gives its props");
});

// React's `packages/react/src/ReactCacheImpl.js`, its dispatcher's root a
// module's, `begin()` and `end()` of a request, as upstream writes it.
const upstreamCache = `const UNTERMINATED = 0;
const TERMINATED = 1;
const ERRORED = 2;
let root = null;
export function begin() { root = new WeakMap(); }
export function end() { root = null; }
function createCacheNode() {
  return { s: UNTERMINATED, v: undefined, o: null, p: null };
}
export function cache(fn) {
  return function () {
    if (!root) {
      return fn.apply(null, arguments);
    }
    const fnMap = root;
    const fnNode = fnMap.get(fn);
    let cacheNode;
    if (fnNode === undefined) {
      cacheNode = createCacheNode();
      fnMap.set(fn, cacheNode);
    } else {
      cacheNode = fnNode;
    }
    for (let i = 0, l = arguments.length; i < l; i++) {
      const arg = arguments[i];
      if (typeof arg === 'function' || (typeof arg === 'object' && arg !== null)) {
        let objectCache = cacheNode.o;
        if (objectCache === null) {
          cacheNode.o = objectCache = new WeakMap();
        }
        const objectNode = objectCache.get(arg);
        if (objectNode === undefined) {
          cacheNode = createCacheNode();
          objectCache.set(arg, cacheNode);
        } else {
          cacheNode = objectNode;
        }
      } else {
        let primitiveCache = cacheNode.p;
        if (primitiveCache === null) {
          cacheNode.p = primitiveCache = new Map();
        }
        const primitiveNode = primitiveCache.get(arg);
        if (primitiveNode === undefined) {
          cacheNode = createCacheNode();
          primitiveCache.set(arg, cacheNode);
        } else {
          cacheNode = primitiveNode;
        }
      }
    }
    if (cacheNode.s === TERMINATED) {
      return cacheNode.v;
    }
    if (cacheNode.s === ERRORED) {
      throw cacheNode.v;
    }
    try {
      const result = fn.apply(null, arguments);
      const terminatedNode = cacheNode;
      terminatedNode.s = TERMINATED;
      terminatedNode.v = result;
      return result;
    } catch (error) {
      const erroredNode = cacheNode;
      erroredNode.s = ERRORED;
      erroredNode.v = error;
      throw error;
    }
  };
}
`;

/** What `lib`'s `cache` gives of a sequence of calls: what each gave or
 * threw, whether it's what it was before, and how often the function ran. */
function cacheTrace(lib: any): unknown[] {
  const log: unknown[] = [];
  let calls = 0;
  const f = function (this: unknown, ...args: unknown[]) {
    calls++;
    return { self: this, args };
  };
  const thrower = (x: unknown) => {
    calls++;
    throw x;
  };
  const c = lib.cache(f);
  const g = lib.cache(thrower);
  log.push(c(1) === c(1), calls);
  lib.begin();
  const object = {};
  const first = c(1, "a", object);
  log.push(first.self, first.args.length, c(1, "a", object) === first, calls);
  log.push(c(1, "a", {}) === first, c(NaN) === c(NaN), c(0) === c(-0), calls);
  log.push(c(undefined) === c(null), c() === c(), c(1) === c(1, undefined), c(f) === c(f), calls);
  for (const thrown of [0, undefined, "x", new Error("e")]) {
    const caught: unknown[] = [];
    for (let i = 0; i < 2; i++) {
      try {
        g(thrown);
      } catch (error) {
        caught.push(error);
      }
    }
    log.push(caught.length, caught[0] === thrown, caught[1] === thrown, calls);
  }
  lib.end();
  log.push(c(1) === c(1), calls);
  return log;
}

// ADR 0851, with 0848 to 0850 and 0782's amendment: React's `cache` as a
// port writes it, the function it gives a closure of rest arguments as a
// value, JS's `(...args) =>`; `fn.apply(null, args)`; a `Map` of any key;
// a `WeakMap` and a `Map` of `Rc`s; and what was caught stored as a value
// of any shape. Run against upstream's of each kind of argument and throw.
test("a closure of rest arguments as a value is JS's rest parameter: React's cache", async () => {
  const dir = fixture("rest-closure-cache");
  writeFileSync(join(dir, "Cargo.toml"), '[package]\nname = "app"\nversion = "0.0.0"\nedition = "2024"\n');
  writeFileSync(join(dir, "lib.rs"), `use std::cell::Cell;
use std::panic::{self, AssertUnwindSafe};
use std::rc::Rc;

use js::{Function, IntoUnknown, JsError, JsObject, Kind, Map, Rest, Unknown, WeakMap, function, map, weak_map};

#[derive(Clone, Copy, PartialEq)]
#[repr(u8)]
enum Status {
    Unterminated = 0,
    Terminated = 1,
    Errored = 2,
}

/// What a function has cached of the arguments up to one of them.
struct CacheNode {
    s: Cell<Status>,
    v: Cell<Option<&'static Unknown>>,
    #[cfg_attr(rust_js, rust_js::nullable)]
    o: Cell<Option<&'static WeakMap<JsObject, Rc<CacheNode>>>>,
    #[cfg_attr(rust_js, rust_js::nullable)]
    p: Cell<Option<&'static Map<Option<&'static Unknown>, Rc<CacheNode>>>>,
}

unsafe impl js::Defined for CacheNode {}

thread_local! {
    static ROOT: Cell<Option<&'static WeakMap<Function, Rc<CacheNode>>>> = const { Cell::new(None) };
}

pub fn begin() {
    ROOT.set(Some(weak_map::new()));
}

pub fn end() {
    ROOT.set(None);
}

fn create_cache_node() -> Rc<CacheNode> {
    Rc::new(CacheNode {
        s: Cell::new(Status::Unterminated),
        v: Cell::new(None),
        o: Cell::new(None),
        p: Cell::new(None),
    })
}

pub fn cache(r#fn: &'static Function) -> impl Fn(&dyn Rest<dyn IntoUnknown>) -> Option<&'static Unknown> {
    move |args: &dyn Rest<dyn IntoUnknown>| {
        let Some(fn_map) = ROOT.get() else {
            return function::apply(r#fn, Some(&Kind::Null), Some(&args));
        };
        let fn_node = fn_map.get(r#fn);
        let mut cache_node;
        if let Some(fn_node) = fn_node {
            cache_node = fn_node;
        } else {
            cache_node = create_cache_node();
            fn_map.set(r#fn, Rc::clone(&cache_node));
        }
        for &arg in args.iter() {
            if let Kind::Function(_) | Kind::Object(_) | Kind::Array(_) = js::classify(arg) {
                // Objects go into a WeakMap
                let arg: &JsObject = unsafe { js::cast(arg) };
                let object_cache = match cache_node.o.get() {
                    Some(object_cache) => object_cache,
                    None => {
                        let object_cache = weak_map::new();
                        cache_node.o.set(Some(object_cache));
                        object_cache
                    }
                };
                let object_node = object_cache.get(arg);
                if let Some(object_node) = object_node {
                    cache_node = object_node;
                } else {
                    cache_node = create_cache_node();
                    object_cache.set(arg, Rc::clone(&cache_node));
                }
            } else {
                // Primitives go into a regular Map
                let primitive_cache = match cache_node.p.get() {
                    Some(primitive_cache) => primitive_cache,
                    None => {
                        let primitive_cache = map::new(None);
                        cache_node.p.set(Some(primitive_cache));
                        primitive_cache
                    }
                };
                let primitive_node = primitive_cache.get(&arg);
                if let Some(primitive_node) = primitive_node {
                    cache_node = primitive_node;
                } else {
                    cache_node = create_cache_node();
                    primitive_cache.set(&arg, Rc::clone(&cache_node));
                }
            }
        }
        if cache_node.s.get() == Status::Terminated {
            return cache_node.v.get();
        }
        if cache_node.s.get() == Status::Errored {
            js::throw(cache_node.v.get());
        }
        match panic::catch_unwind(AssertUnwindSafe(|| function::apply(r#fn, Some(&Kind::Null), Some(&args)))) {
            Ok(result) => {
                let terminated_node = &cache_node;
                terminated_node.s.set(Status::Terminated);
                terminated_node.v.set(result);
                result
            }
            Err(error) => {
                // We store the first error that's thrown and rethrow it.
                let errored_node = &cache_node;
                errored_node.s.set(Status::Errored);
                errored_node.v.set(error.downcast_ref::<&'static JsError>().and_then(|error| error.as_unknown()));
                panic::resume_unwind(error)
            }
        }
    }
}
`);
  run([compiler, "--declarations", join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", ...js]);
  const out = readFileSync(join(dir, "lib.js"), "utf8");
  expect(out).toContain("  return (...args) => {\n");
  expect(out).toContain("      return fn.apply(null, args);\n");
  expect(out).toContain("    for (let i = 0; i < args.length; i++) {\n      const arg = args[i];\n");
  expect(out).toContain("const primitiveNode = primitiveCache.get(arg);");
  expect(out).toContain("      const result = fn.apply(null, args);\n");
  expect(out).toContain("      erroredNode.v = error;\n      throw error;\n");
  expect(readFileSync(join(dir, "lib.d.ts"), "utf8")).toContain(
    "export function cache(fn: Function): (...args: (unknown | null | undefined)[]) => unknown | null | undefined;",
  );
  writeFileSync(join(dir, "upstream.js"), upstreamCache);
  const lib = await import(join(dir, "lib.js"));
  const original = await import(join(dir, "upstream.js"));
  expect(cacheTrace(lib)).toEqual(cacheTrace(original));
});

// ADR 0851: a call through any type of a closure of rest arguments as a
// value spreads them, as the type says: its own, a `&dyn Fn`, a `Box`, a
// `fn` pointer, a generic `F: Fn(&dyn Rest<T>)`, and the `impl Fn` a
// function gives; a tuple written out is its items, a slice held spread.
test("a call of a closure of rest arguments spreads them, through any type that says so", async () => {
  const dir = fixture("rest-closure-calls");
  writeFileSync(join(dir, "Cargo.toml"), '[package]\nname = "app"\nversion = "0.0.0"\nedition = "2024"\n');
  writeFileSync(join(dir, "lib.rs"), `use js::{Function, IntoUnknown, Rest, Unknown};

type Counted = dyn Fn(&dyn Rest<dyn IntoUnknown>) -> usize;

fn each<F: Fn(&dyn Rest<dyn IntoUnknown>) -> usize>(f: F) -> usize {
    f(&(1, 2))
}

pub fn joined() -> impl Fn(&dyn Rest<str>) -> String {
    |names: &dyn Rest<str>| names.join(", ")
}

pub fn passed_on(f: &'static Function) -> impl Fn(&dyn Rest<dyn IntoUnknown>) -> Option<&'static Unknown> {
    move |args: &dyn Rest<dyn IntoUnknown>| js::function::call(f, None, args)
}

pub fn counts(v: &[f64]) -> Vec<usize> {
    let count = |args: &dyn Rest<dyn IntoUnknown>| args.len();
    let held: &Counted = &count;
    let boxed: Box<Counted> = Box::new(count);
    let pointer: fn(&dyn Rest<dyn IntoUnknown>) -> usize = |args| args.len();
    vec![
        count(&(1, "a", 2)),
        count(&()),
        count(&v),
        held(&("a",)),
        boxed(&(v[0], v[1])),
        pointer(&v),
        each(count),
    ]
}

pub fn named(names: &[&str]) -> (String, String) {
    let join = joined();
    (join(&("a", "b")), join(&names))
}
`);
  run([compiler, "--declarations", join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", ...js]);
  const out = readFileSync(join(dir, "lib.js"), "utf8");
  expect(out).toContain("const count = (...args) => args.length;");
  expect(out).toContain("    count(1, \"a\", 2),\n    count(),\n    count(...v),\n    held(\"a\"),\n    boxed($index(v, 0), $index(v, 1)),\n    pointer(...v),\n");
  expect(out).toContain("  return f(1, 2);");
  expect(out).toContain("return (...names) => names.join(\", \");");
  expect(out).toContain("return (...args) => f.call(undefined, ...args);");
  expect(out).toContain("return [join(\"a\", \"b\"), join(...names)];");
  expect(readFileSync(join(dir, "lib.d.ts"), "utf8")).toContain("export function joined(): (...args: string[]) => string;");
  const lib = await import(join(dir, "lib.js"));
  expect(lib.counts([4, 5])).toEqual([3, 0, 2, 1, 2, 2, 2]);
  expect(lib.passedOn(function (this: unknown, ...args: unknown[]) {
    return [this, ...args];
  })(1, "b")).toEqual([undefined, 1, "b"]);
  expect(lib.named(["x", "y", "z"])).toEqual(["a, b", "x, y, z"]);
  expect(lib.joined()("p", "q")).toBe("p, q");
});

// #576: rest arguments of any sized type, a type parameter's, a struct's, a
// `Rc<T>`, read as themselves (`js::RestItem`), and a Rust caller gives
// them, each of its own type (`js::Takes`): alien-signals' `signal<T>`,
// whose writes are typed. The `.d.ts` is `(...args: T[])` of a type
// parameter's, each sized, as Rust's implicit bound has it.
test("rest arguments of a type parameter or any sized type are read as themselves", async () => {
  const dir = fixture("rest-items-of-any-type");
  writeFileSync(join(dir, "Cargo.toml"), '[package]\nname = "app"\nversion = "0.0.0"\nedition = "2024"\n');
  writeFileSync(join(dir, "lib.rs"), `use std::cell::RefCell;
use std::rc::Rc;

use js::Rest;

pub struct Node<T> {
    current: RefCell<Rc<T>>,
}

fn signal_oper<T>(this: &Node<T>, #[cfg_attr(rust_js, rust_js::rest)] value: &[Rc<T>]) -> Option<Rc<T>> {
    if !value.is_empty() {
        this.current.replace(Rc::clone(&value[0]));
        None
    } else {
        Some(Rc::clone(&this.current.borrow()))
    }
}

pub fn signal<T: 'static>(initial_value: T) -> impl Fn(&dyn Rest<Rc<T>>) -> Option<Rc<T>> {
    let this = Rc::new(Node {
        current: RefCell::new(Rc::new(initial_value)),
    });
    move |value: &dyn Rest<Rc<T>>| signal_oper(&this, value)
}

#[derive(Clone, Debug, PartialEq)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

pub fn sum_x() -> impl Fn(&dyn Rest<Point>) -> f64 {
    |points: &dyn Rest<Point>| points.iter().map(|p| p.x).sum()
}

pub fn firsts<T: Clone + 'static>() -> impl Fn(&dyn Rest<T>) -> Option<T> {
    |items: &dyn Rest<T>| items.first().cloned()
}

pub fn used() -> (Option<Rc<String>>, f64, f64, Option<u8>, bool, usize) {
    let name = signal(String::from("a"));
    name(&(Rc::new(String::from("b")),));
    let held = [Point { x: 1.0, y: 2.0 }, Point { x: 3.0, y: 4.0 }];
    let sum = sum_x();
    let nested = firsts::<Option<u8>>();
    (
        name(&()),
        sum(&(Point { x: 1.0, y: 0.0 }, Point { x: 2.5, y: 0.0 })),
        sum(&&held[..]),
        firsts::<u8>()(&(7, 8)),
        nested(&(None,)).is_some(),
        nested(&(None, Some(1))).iter().count(),
    )
}
`);
  run([compiler, "--declarations", join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", ...js]);
  const out = readFileSync(join(dir, "lib.js"), "utf8");
  // A rest function of them is JS's, its value the function itself.
  expect(out).toContain("function signalOper(this$, ...value) {\n  if (value.length !== 0) {\n    this$.current = value[0];\n");
  expect(out).toContain("return signalOper.bind(null, this$);");
  expect(out).toContain("return (...points) => points.map((p) => p.x).reduce((a, b) => a + b, -0);");
  expect(out).toContain("return (...items) => items[0];");
  // Each given as itself, a tuple's items written out, a slice spread.
  expect(out).toContain('name("b");');
  expect(out).toContain("sum({ x: 1, y: 0 }, { x: 2.5, y: 0 });");
  expect(out).toContain("sum(...held);");
  const dts = readFileSync(join(dir, "lib.d.ts"), "utf8");
  expect(dts).toContain("export function signal<T>(initialValue: T): (...args: T[]) => T | null | undefined;");
  expect(dts).toContain("export function sumX(): (...args: Point[]) => number;");
  expect(dts).toContain("export function firsts<T>(): (...args: T[]) => T | null | undefined;");
  const lib = await import(join(dir, "lib.js"));
  // Native Rust's: `Some("b")`, 3.5, 4, `Some(7)`, `Some(None)` is some, one.
  expect(lib.used()).toEqual(["b", 3.5, 4, 7, true, 1]);
  const count = lib.signal(1);
  expect([count(), count(2), count()]).toEqual([1, undefined, 2]);
  expect(lib.firsts()(undefined, 1)).toBe(undefined);
});

// Rest arguments as a value are their array, as a rest slice is: an index a
// test shows in bounds, `value[0]` after `value.is_empty()` fails, is read
// as JS reads it, no `$index`; one none shows is checked still (#586).
test("rest arguments as a value indexed in bounds are read as a rest slice is", async () => {
  const dir = fixture("rest-value-indexed");
  writeFileSync(join(dir, "lib.rs"), `use js::Rest;

pub fn first_of_rest() -> impl Fn(&dyn Rest<f64>) -> f64 {
    |value: &dyn Rest<f64>| if value.is_empty() { 0.0 } else { value[0] }
}

pub fn ends_of_rest() -> impl Fn(&dyn Rest<f64>) -> f64 {
    |value: &dyn Rest<f64>| {
        if value.len() > 2 {
            value[value.len() - 1] + value[1]
        } else {
            -1.0
        }
    }
}

pub fn second_of_rest() -> impl Fn(&dyn Rest<f64>) -> f64 {
    |value: &dyn Rest<f64>| if value.is_empty() { 0.0 } else { value[1] }
}

pub fn first_of_slice(#[cfg_attr(rust_js, rust_js::rest)] value: &[f64]) -> f64 {
    if value.is_empty() { 0.0 } else { value[0] }
}

pub fn ends_of_slice(#[cfg_attr(rust_js, rust_js::rest)] value: &[f64]) -> f64 {
    if value.len() > 2 {
        value[value.len() - 1] + value[1]
    } else {
        -1.0
    }
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", ...js]);
  const out = readFileSync(join(dir, "lib.js"), "utf8");
  expect(out).toContain("return (...value) => (value.length === 0 ? 0 : value[0]);");
  expect(out).toContain("return (...value) => (value.length > 2 ? value[value.length - 1] + value[1] : -1);");
  expect(out).toContain("return (...value) => (value.length === 0 ? 0 : $index(value, 1));");
  expect(out).toContain("return value.length === 0 ? 0 : value[0];");
  expect(out).toContain("return value.length > 2 ? value[value.length - 1] + value[1] : -1;");
  const lib = await import(join(dir, "lib.js"));
  // Native Rust's: 0 of none, the first; the last and the second; a panic
  // past the end.
  expect([lib.firstOfRest()(), lib.firstOfRest()(4, 5)]).toEqual([0, 4]);
  expect([lib.endsOfRest()(1, 2), lib.endsOfRest()(1, 2, 3)]).toEqual([-1, 5]);
  expect(() => lib.secondOfRest()(7)).toThrow("index out of bounds: the len is 1 but the index is 1");
  expect([lib.firstOfSlice(9, 8), lib.endsOfSlice(1, 2, 3)]).toEqual([9, 5]);
});

// Where rest arguments as a value aren't JS's rest parameter, it's an error
// that says what to write (ADRs 0667, 0851).
for (const [name, source, message] of [
  ["a function's parameter", "use js::{IntoUnknown, Rest};\npub fn f(args: &dyn Rest<dyn IntoUnknown>) -> usize { args.len() }",
    "rust-js does not support a `&dyn js::Rest<T>` parameter, rest arguments as a value, of a Rust function yet: a function's rest parameter is its last slice marked `#[rust_js::rest]`"],
  ["a closure's before its last", "use js::{IntoUnknown, Rest};\npub fn f() -> usize { let g = |args: &dyn Rest<dyn IntoUnknown>, n: usize| args.len() + n; g(&(1,), 2) }",
    "rust-js does not support a closure's `&dyn js::Rest<T>` parameter, rest arguments as a value, before its last yet: JS's rest parameter is a function's last"],
  ["a type parameter's value", "use js::{IntoUnknown, Rest};\nfn call<A>(f: impl Fn(A) -> usize, a: A) -> usize { f(a) }\npub fn f() -> usize {\n    let count = |args: &dyn Rest<dyn IntoUnknown>| args.len();\n    call(count, &(1, 2))\n}",
    "rust-js does not support rest arguments as a value, `&dyn js::Rest<T>`, as a type parameter's value yet: a call of an `F: Fn(A)` would give them as one array"],
] as const) {
  test(`rest arguments as a value refused: ${name}`, () => {
    const dir = fixture(`rest-value-refused-${name.replaceAll(/\W+/g, "-")}`);
    writeFileSync(join(dir, "lib.rs"), source);
    const refused = Bun.spawnSync([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", ...js]);
    expect(refused.exitCode).not.toBe(0);
    expect(refused.stderr.toString()).toContain(message);
  });
}
