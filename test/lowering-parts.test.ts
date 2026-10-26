// What the compiler writes of a value's parts the program reads into names
// (ADR 0598, amended): a pattern's parts one destructuring, `const { a, b }
// = p;`; a `let` of a name alone, `let x = p.x;`, a statement of its own,
// as written; and a variable given another type, `js::cast`'s, the
// variable. Each test is a program of its own, compiled and run.

import { beforeAll, expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { buildReact, compiler, fixture, run, target } from "./support";

beforeAll(buildReact, 600_000);

// React's `mapIntoArray` (ReactChildren.js) reads a lazy component's
// `_payload` and `_init` of `children` as it is, `(children as any)`:
// upstream writes `const payload = children._payload; const init =
// children._init;`, which the Rust's `let lazy: &LazyComponent = unsafe {
// js::cast(children) };` and two `let`s were, as `const lazy = children;
// const { _payload: payload, _init: init } = lazy;`.
test("a let of a part is its own statement, and a cast's variable is the variable", async () => {
  const dir = fixture("parts-read");
  writeFileSync(join(dir, "lib.rs"), `use std::rc::Rc;

use js::Unknown;

pub struct Payload {
    pub status: i32,
}

pub struct LazyComponent {
    #[cfg_attr(rust_js, rust_js::name = "_payload")]
    pub payload: Rc<Payload>,
    #[cfg_attr(rust_js, rust_js::name = "_init")]
    pub init: fn(&Rc<Payload>) -> i32,
}

pub fn resolved(children: Option<&'static Unknown>) -> i32 {
    let lazy: &LazyComponent = unsafe { js::cast(children) };
    let payload = &lazy.payload;
    let init = lazy.init;
    init(payload)
}

pub fn read_again(children: Option<&'static Unknown>) -> (i32, bool) {
    let lazy: &LazyComponent = unsafe { js::cast(children) };
    let status = lazy.payload.status;
    (status, children.is_some())
}

pub struct Point {
    pub x: f64,
    pub y: f64,
}

pub fn sum(p: &Point) -> f64 {
    let x = p.x;
    let y = p.y;
    x + y
}

pub fn patterned(p: &Point) -> f64 {
    let Point { x, y } = p;
    x - y
}

pub fn paired(pair: (f64, f64)) -> f64 {
    let (a, b) = pair;
    a * b
}

pub struct Labeled {
    pub x: f64,
    pub y: f64,
    pub label: String,
}

pub fn matched(point: &Labeled) -> f64 {
    match point {
        Labeled { x, y, .. } => x * y + point.label.len() as f64,
    }
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain(`export function resolved(children) {
  const payload = children._payload;
  const init = children._init;
  return init(payload);
}`);
  // \`children\` read after it: the name the program gave stays.
  expect(js).toContain("const lazy = children;");
  // Two \`let\`s side by side, each named as the part it reads, as React
  // writes \`const {tag, type} = fiber;\` (ADR 0598, amended).
  expect(js).toContain(`export function sum(p) {
  const { x, y } = p;
  return x + y;
}`);
  expect(js).toContain("const { x, y } = p;");
  expect(js).toContain("const [a, b] = pair;");
  // An arm's pattern of a variable, MIR's temporary of a part after it no
  // part of it, read where it's used (ADR 0598).
  expect(js).toContain("  const { x, y } = point;\n  return x * y + $byteLen(point.label);");
  const lib = await import(join(dir, "lib.js"));
  const lazy = { _payload: { status: 4 }, _init: (p: { status: number }) => p.status * 10 };
  expect([lib.resolved(lazy), lib.readAgain(lazy), lib.sum({ x: 1, y: 2 }), lib.patterned({ x: 5, y: 2 }), lib.paired([3, 4]), lib.matched({ x: 2, y: 3, label: "ab" })])
    .toEqual([40, [4, true], 3, 3, 12, 8]);
});

// React's markUpdateLaneFromFiberToRoot: `const offscreenInstance =
// fiber.stateNode; if (offscreenInstance !== null && ..)`. Its port types
// the field's value, `let instance: Option<&Instance> = unsafe {
// js::cast(node.state_node.get()) };`, and `if let` tests it: the
// program's name for the value, no `let` that ends a borrow (ADR 0866),
// so not read twice in the test in its place (ADR 0598, amended).
test("a let of a cast an if let tests keeps its name", async () => {
  const dir = fixture("cast-let-tested");
  writeFileSync(join(dir, "lib.rs"), `use std::cell::Cell;

use js::Unknown;

pub struct Instance {
    #[cfg_attr(rust_js, rust_js::name = "_visibility")]
    pub visibility: Cell<i32>,
}

pub struct Node {
    #[cfg_attr(rust_js, rust_js::nullable)]
    pub state_node: Cell<Option<&'static Unknown>>,
}

pub fn is_hidden(node: &Node) -> bool {
    let instance: Option<&Instance> = unsafe { js::cast(node.state_node.get()) };
    if let Some(instance) = instance
        && (instance.visibility.get() & 1) == 0
    {
        return true;
    }
    false
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("  const instance = node.stateNode;\n  if (instance && !(instance._visibility & 1)) {\n");
  const lib = await import(join(dir, "lib.js"));
  const hidden = [{ stateNode: null }, { stateNode: { _visibility: 1 } }, { stateNode: { _visibility: 2 } }];
  expect(hidden.map((n) => lib.isHidden(n))).toEqual([false, false, true]);
});
