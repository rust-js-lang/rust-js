// `js::arguments()`, JS's `arguments` (rd332, the user's decision):
// react-art's fill classes name their parameters, `.length` 5, and keep
// every argument they're given, `slice.call(arguments)`, which `applyFill`
// passes on, `node.fillLinear.apply(node, this._args)`. Each call rust-js
// writes of a function that reads it gives every argument the Rust does,
// and no closure forwarding to it is it; read where its JS's `arguments`
// isn't what the Rust gives, a compile error.

import { beforeAll, expect, test } from "bun:test";
import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { runSync } from "./child";
import { buildReact, compiler, fixture, run, target } from "./support";

beforeAll(buildReact, 600_000);

const js = ["--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];

const fill = `use std::marker::PhantomData;

use js::array::SLICE as slice;
use js::{JsObject, Unknown, function};

/// A node of art's, as react-art's binding has it.
pub struct Node(PhantomData<JsObject>);

unsafe impl js::Defined for Node {}

impl Node {
    /// \`node.fillLinear\`, its method as a value.
    #[cfg_attr(rust_js, rust_js::link_name = "get fillLinear")]
    pub fn fill_linear(&self) -> &'static dyn Fn(Option<&Unknown>, Option<&Unknown>) -> Option<&'static Unknown> {
        unreachable!()
    }
}

#[cfg_attr(rust_js, rust_js::class)]
pub struct LinearGradient {
    args: Option<&'static Unknown>,
}

#[cfg_attr(rust_js, rust_js::class)]
impl LinearGradient {
    pub fn new(
        _stops: Option<&Unknown>,
        _x1: Option<&Unknown>,
        _y1: Option<&Unknown>,
        _x2: Option<&Unknown>,
        _y2: Option<&Unknown>,
    ) -> Self {
        LinearGradient {
            args: function::call(slice, Some(js::arguments()), ()),
        }
    }

    pub fn apply_fill(&self, node: &Node) {
        function::apply(node.fill_linear(), Some(node), self.args.map(|args| args as &dyn js::IntoUnknown));
    }
}

/// How many it was given.
pub fn given(_first: Option<&Unknown>, _second: Option<&Unknown>) -> Option<&'static Unknown> {
    js::get(js::arguments(), "length")
}

pub fn made() -> LinearGradient {
    LinearGradient::new(None, None, None, None, None)
}

pub fn counted() -> Option<&'static Unknown> {
    given(None, None)
}

/// A closure that only passes on what it's given: an arrow, not \`given\`,
/// which would count what its JS caller gives it.
pub fn passing() -> Box<dyn Fn(Option<&Unknown>, Option<&Unknown>) -> Option<&'static Unknown>> {
    Box::new(|first, second| given(first, second))
}

/// A closure that passes on what it captured, then what it's given: an
/// arrow, not \`given.bind(null, first)\`.
pub fn captured(first: Option<&'static Unknown>) -> Box<dyn Fn(Option<&'static Unknown>) -> Option<&'static Unknown>> {
    Box::new(move |second| given(first, second))
}

fn one(_first: Option<&Unknown>) -> Option<&'static Unknown> {
    js::get(js::arguments(), "length")
}

pub struct Counter {
    pub count: fn(Option<&Unknown>, Option<&Unknown>) -> Option<&'static Unknown>,
}

/// A closure as a \`fn\` that gives \`one\` less than it's given: an arrow,
/// not \`one\`, which would count both.
pub static COUNTER: Counter = Counter {
    count: |first, _| one(first),
};

/// A closure of it bound, an arrow too.
pub fn rebound() -> Box<dyn Fn(Option<&'static Unknown>) -> Option<&'static Unknown>> {
    let g = function::bind(one, None, ());
    Box::new(move |first| g(first))
}

/// A \`function::with_this\` function, a JS \`function\`, whose \`arguments\`
/// is its own (rd509).
pub fn counter() -> &'static dyn Fn(Option<&'static Unknown>) -> Option<&'static Unknown> {
    function::with_this(|_, (_first,): (Option<&'static Unknown>,)| js::get(js::arguments(), "length"))
}

/// Called through its \`dyn Fn\`, given every argument.
pub fn counted_by_value() -> Option<&'static Unknown> {
    counter()(None)
}

/// A closure that passes on to it what it's given, an arrow.
pub fn forwarding_value() -> Box<dyn Fn(Option<&'static Unknown>) -> Option<&'static Unknown>> {
    let f = counter();
    Box::new(move |first| f(first))
}

/// Through a pointer, every argument too.
pub fn pointed() -> Option<&'static Unknown> {
    let f: fn(Option<&Unknown>, Option<&Unknown>) -> Option<&'static Unknown> = given;
    f(None, None)
}
`;

test("a function that reads arguments names its parameters and is given each", async () => {
  const dir = fixture("arguments");
  writeFileSync(join(dir, "lib.rs"), fill);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", ...js]);
  const out = readFileSync(join(dir, "lib.js"), "utf8");
  expect(out).toContain("export function LinearGradient(stops, x1, y1, x2, y2) {\n  this.args = Array.prototype.slice.call(arguments);\n}\n");
  expect(out).toContain("node.fillLinear.apply(node, this.args);");
  expect(out).toContain("return new LinearGradient(undefined, undefined, undefined, undefined, undefined);");
  expect(out).toContain("return given(undefined, undefined);");
  expect(out).toContain("(first, second) => given(first, second)");
  const lib = await import(join(dir, "lib.js"));
  expect(lib.LinearGradient.length).toBe(5);
  // A JS caller's every argument, more or fewer.
  const seen: unknown[][] = [];
  const node = { fillLinear(...args: unknown[]) { seen.push(args); return null; } };
  new lib.LinearGradient("stops", 1, 2, 3, 4, 5).applyFill(node);
  new lib.LinearGradient("stops", 1).applyFill(node);
  lib.made().applyFill(node);
  expect(seen).toEqual([["stops", 1, 2, 3, 4, 5], ["stops", 1], [undefined, undefined, undefined, undefined, undefined]]);
  expect([lib.counted(), lib.pointed(), lib.given(1, 2, 3), lib.passing()(1, 2, 3), lib.captured(1)(2, 3), lib.COUNTER.count(1, 2), lib.rebound()(1, 2), lib.counter()(1, 2), lib.countedByValue(), lib.forwardingValue()(1, 2)]).toEqual([2, 2, 3, 2, 2, 1, 1, 2, 1, 1]);
});

test("arguments read where its JS's aren't the Rust's is refused", () => {
  const dir = fixture("arguments-refused");
  const refused = (body: string) => {
    writeFileSync(join(dir, "lib.rs"), `use js::Unknown;\n\n${body}`);
    const ran = runSync([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", ...js], dir, 120_000);
    expect(ran.code).not.toBe(0);
    return ran.stderr;
  };
  expect(refused("pub fn f() -> Box<dyn Fn() -> &'static js::Arguments> {\n    Box::new(js::arguments)\n}\n\npub fn g() -> impl Fn() -> Option<&'static Unknown> {\n    || js::get(js::arguments(), \"length\")\n}\n"))
    .toContain("a closure is an arrow function, whose `arguments` is the function's around it");
  expect(refused("pub fn f<T>(value: T) -> T {\n    let _ = js::arguments();\n    value\n}\n"))
    .toContain("a generic function's JS is given its type parameters' dictionaries too");
  expect(refused("pub struct Point;\n\nimpl Point {\n    pub fn count(&self) -> Option<&'static Unknown> {\n        js::get(js::arguments(), \"length\")\n    }\n}\n"))
    .toContain("a method of what's no class is its type's function, given the value first");
  expect(refused("pub trait Counts {\n    fn count(&self) -> u32;\n}\n\npub struct Point;\n\nimpl Counts for Point {\n    fn count(&self) -> u32 {\n        let _ = js::arguments();\n        0\n    }\n}\n"))
    .toContain("a trait's method is called through its dictionary");
});

// A library's function that reads it is given every argument by its
// consumers' calls too, its contract's `arguments` (rd332).
test("a library's function that reads arguments is given each by its consumer", async () => {
  const dir = fixture("arguments-library");
  mkdirSync(join(dir, "dep"));
  writeFileSync(join(dir, "dep.rs"), "use js::Unknown;\n\npub fn given(_first: Option<&Unknown>, _second: Option<&Unknown>) -> Option<&'static Unknown> {\n    js::get(js::arguments(), \"length\")\n}\n");
  run([compiler, join(dir, "dep.rs"), "-o", join(dir, "dep", "lib.js"), "--library", "--manifest", join(dir, "dep", "lib.manifest.json"),
    "--", "--crate-name", "dep", `--emit=metadata=${join(dir, "libdep.rmeta")}`, ...js]);
  mkdirSync(join(dir, "app"));
  writeFileSync(join(dir, "app", "lib.rs"), "use js::Unknown;\n\npub fn counted() -> Option<&'static Unknown> {\n    dep::given(None, None)\n}\n\npub fn passing() -> Box<dyn Fn(Option<&Unknown>, Option<&Unknown>) -> Option<&'static Unknown>> {\n    Box::new(|first, second| dep::given(first, second))\n}\n");
  run([compiler, join(dir, "app", "lib.rs"), "-o", join(dir, "app", "lib.js"), "--dependency", join(dir, "dep", "lib.manifest.json"),
    "--", "--crate-name", "app", "--extern", `dep=${join(dir, "libdep.rmeta")}`, "-L", dir, ...js]);
  const app = readFileSync(join(dir, "app", "lib.js"), "utf8");
  expect(app).toContain("(undefined, undefined)");
  const lib = await import(join(dir, "app", "lib.js"));
  expect([lib.counted(), lib.passing()(1, 2, 3)]).toEqual([2, 2]);
});
