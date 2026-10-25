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
  expect(out).toContain("total(1, 2), total(...v)");
  const { sums } = await import(join(dir, "app", "lib.js"));
  expect(sums([4, 5])).toEqual([3, 9, 9]);
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
  ["a function given a dictionary", "pub fn f<T: Clone>(#[cfg_attr(rust_js, rust_js::rest)] xs: &[T]) -> Vec<T> { xs.to_vec() }\npub fn g() -> Vec<u32> { f(&[1, 2]) }",
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

use js::{IntoUnknown, JsError, JsObject, Kind, Map, Rest, Unknown, WeakMap, function, map, weak_map};

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

/// A JS function of any arguments, TypeScript's \`Function\`.
type AnyFunction = dyn Fn() -> Option<&'static Unknown>;

thread_local! {
    static ROOT: Cell<Option<&'static WeakMap<AnyFunction, Rc<CacheNode>>>> = const { Cell::new(None) };
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

pub fn cache(r#fn: &'static AnyFunction) -> impl Fn(&dyn Rest<dyn IntoUnknown>) -> Option<&'static Unknown> {
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
                        let primitive_cache = map::new();
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
  expect(out).toContain("    for (const arg of args) {\n");
  expect(out).toContain("const primitiveNode = primitiveCache.get(arg);");
  expect(out).toContain("      const result = fn.apply(null, args);\n");
  expect(out).toContain("      erroredNode.v = error;\n      throw error;\n");
  expect(readFileSync(join(dir, "lib.d.ts"), "utf8")).toContain(
    "export function cache(fn: () => unknown | null | undefined): (...args: (unknown | null | undefined)[]) => unknown | null | undefined;",
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
  writeFileSync(join(dir, "lib.rs"), `use js::{IntoUnknown, Rest, Unknown};

type Counted = dyn Fn(&dyn Rest<dyn IntoUnknown>) -> usize;

fn each<F: Fn(&dyn Rest<dyn IntoUnknown>) -> usize>(f: F) -> usize {
    f(&(1, 2))
}

pub fn joined() -> impl Fn(&dyn Rest<str>) -> String {
    |names: &dyn Rest<str>| names.join(", ")
}

pub fn passed_on(f: &'static dyn Fn() -> Option<&'static Unknown>) -> impl Fn(&dyn Rest<dyn IntoUnknown>) -> Option<&'static Unknown> {
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
