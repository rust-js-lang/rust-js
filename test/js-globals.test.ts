// The js crate's bindings of JS's globals each typed as TypeScript's lib
// types them, never narrower: React's Flight client's `Array.prototype`,
// `BigInt(text)`, `new Date(Date.parse(text))` and `Symbol()` (rd399);
// its Flight server's `Promise.prototype`, `then` handlers of `void | PromiseLike<void>`,
// an async iterator's `throw` and an async iterable (rd580, rd581, rd584, rd585);
// and a getter's function unwrapped where it's called, `console.createTask` (rd519).
// Each test is a program of its own, compiled, and its JS run.

import { beforeAll, expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";

import { runSync } from "./child";
import { node } from "./programs";
import { buildReact, compiler, fixture, root, target } from "./support";

beforeAll(buildReact, 600_000);

const withJs = ["--", "--extern", `js=${join(target, "libjs.rmeta")}`, "--extern", `webapi=${join(target, "libwebapi.rmeta")}`, "-L", target];

/** `source` compiled, its JS, and the module it is, loaded. */
async function compiled(name: string, source: string): Promise<{ js: string; lib: any; dir: string }> {
  const dir = fixture(name);
  writeFileSync(join(dir, "lib.rs"), source);
  const built = runSync([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withJs], root, 600_000);
  expect(built.code, built.stderr).toBe(0);
  return { js: readFileSync(join(dir, "lib.js"), "utf8"), lib: await import(join(dir, "lib.js")), dir };
}

// ReactFlightClient's parseModelString: `$D` is a date, `$n` a bigint, each
// of the text after its tag; `isArray` asks of a value's prototype. A
// symbol of no description is `Symbol()`, TypeScript's optional
// `string | number`, as its RESPONSE_SYMBOL is.
test("Array.prototype, BigInt, Date.parse and Symbol are typed as TypeScript's lib", async () => {
  const { js, lib } = await compiled(
    "js-globals-flight",
    `use js::{BigInt, Date, Symbol, Unknown, array, date, symbol};

pub fn is_plain_array(value: &Unknown) -> bool {
    js::object::get_prototype_of(Some(value)) == Some(array::prototype)
}

pub fn big(text: &str) -> &'static BigInt {
    js::big_int(&js::string::slice(text, 2.0, None))
}

pub fn of_number(value: f64) -> &'static BigInt {
    js::big_int(value)
}

pub fn dated(text: &str) -> &'static Date {
    date::new(Some(&date::parse(&js::string::slice(text, 2.0, None))))
}

pub fn parsed(text: &str) -> f64 {
    date::parse(text)
}

pub fn symbols() -> (&'static Symbol, &'static Symbol, &'static Symbol) {
    (symbol::new(None), symbol::new(Some(&"react.response")), symbol::new(Some(&1.0)))
}
`,
  );
  expect(js).toContain("Object.getPrototypeOf(value) === Array.prototype");
  expect(js).toContain("BigInt(text.slice(2))");
  expect(js).toContain("BigInt(value)");
  expect(js).toContain("new Date(Date.parse(text.slice(2)))");
  expect(js).toContain('[Symbol(), Symbol("react.response"), Symbol(1)]');
  expect([lib.isPlainArray([1]), lib.isPlainArray({}), lib.isPlainArray(new Uint8Array(1))]).toEqual([true, false, false]);
  expect(lib.big("$n123456789012345678901234567890")).toBe(123456789012345678901234567890n);
  expect(lib.ofNumber(3)).toBe(3n);
  expect(() => lib.ofNumber(1.5)).toThrow(RangeError);
  expect(() => lib.big("$nabc")).toThrow(SyntaxError);
  expect(lib.dated("$D2024-01-02T03:04:05.000Z").toISOString()).toBe("2024-01-02T03:04:05.000Z");
  expect(Number.isNaN(lib.parsed("not a date"))).toBe(true);
  const [none, named, numbered] = lib.symbols();
  expect([typeof none, none.description, named.description, numbered.description]).toEqual([
    "symbol",
    undefined,
    "react.response",
    "1",
  ]);
});

// rd580: ReactFlightWebpackReferences' client proxy pretends to be a
// promise, its `getPrototypeOf` trap giving `Promise.prototype`, and React's
// Flight reply server makes its chunks of it, `Object.create(Promise.prototype)`.
// A class's prototype is `js::class::<T>().prototype()`, the one way, of a
// global class's too: no `js::promise::prototype` beside it, as `Array` and
// `Object`, which are Rust's own types, no class's, have theirs.
test("Promise.prototype is its class's prototype", async () => {
  const { js, dir } = await compiled(
    "js-globals-promise-prototype",
    `use js::{JsObject, Promise, ProxyHandler, Unknown, proxy};

pub fn chunk() -> &'static Unknown {
    js::object::create(Some(js::class::<Promise<()>>().prototype()), None)
}

pub fn pretending(target: &'static JsObject) -> &'static JsObject {
    let handler = ProxyHandler {
        get_prototype_of: Some(Box::new(|_target| js::class::<Promise<()>>().prototype())),
        ..Default::default()
    };
    proxy::new(target, &handler)
}
`,
  );
  expect(js).toContain("return Object.create(Promise.prototype);");
  expect(js).toContain("getPrototypeOf: (_target) => Promise.prototype,");
  writeFileSync(
    join(dir, "run.mjs"),
    `import * as lib from "./lib.js";
const chunk = lib.chunk();
console.log(Object.getPrototypeOf(chunk) === Promise.prototype, chunk instanceof Promise, typeof chunk.then);
console.log(Object.getPrototypeOf(lib.pretending({})) === Promise.prototype, lib.pretending({}) instanceof Promise);
`,
  );
  const ran = runSync([node ?? "node", join(dir, "run.mjs")], root, 30_000);
  expect(ran.stderr).toBe("");
  expect(ran.stdout).toBe("true true function\ntrue true\n");
});

// rd417: a `DataView` and a `ConsoleTask` are objects, never nullish, so
// `js::Defined`: of any shape, `js::unknown(view)`, as React's Flight client
// hands each to what takes any value, and an `Option` of one the value or
// `undefined`.
test("a DataView and a ConsoleTask are values of any shape", async () => {
  const dir = fixture("js-globals-defined");
  writeFileSync(
    join(dir, "lib.rs"),
    `use js::{DataView, Unknown};
use webapi::ConsoleTask;

pub fn of_view(view: &'static DataView) -> &'static Unknown {
    js::unknown(view)
}

pub fn of_task(task: Option<&'static ConsoleTask>) -> Option<&'static Unknown> {
    task.map(|task| js::unknown(task))
}
`,
  );
  const extern = [...withJs, "--extern", `webapi=${join(target, "libwebapi.rmeta")}`];
  const built = runSync([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...extern], root, 600_000);
  expect(built.code, built.stderr).toBe(0);
  const lib = await import(join(dir, "lib.js"));
  const view = new DataView(new ArrayBuffer(1));
  expect([lib.ofView(view), lib.ofTask(undefined)]).toEqual([view, undefined]);
});

// rd404: `new AggregateError(errors, message, options)`, as TypeScript's
// ES2022 types it, its `cause` given to it; and a constructor bound as a
// value, called without `new`, as React's Flight client's fake call stack
// calls `Error.bind(null, message, options)`.
test("an AggregateError is made of its options, and a constructor bound as a value", async () => {
  const { js, lib, dir } = await compiled(
    "js-globals-errors",
    `use js::{AggregateError, Error, ErrorOptions, Kind, Unknown, aggregate_error, error, function};

pub fn caused(errors: &[Option<&Unknown>], cause: &Unknown) -> &'static AggregateError {
    aggregate_error::new(errors, Some("several"), Some(&ErrorOptions { cause: Some(cause) }))
}

pub fn bound_error(message: &str, cause: &'static Unknown) -> &'static Error {
    let make = function::bind(error::new, Some(&Kind::Null), (Some(message), Some(ErrorOptions { cause: Some(cause) })));
    make()
}

pub fn bound_aggregate(errors: &[Option<&'static Unknown>], message: &str) -> &'static AggregateError {
    let make = function::bind(aggregate_error::new, Some(&Kind::Null), (errors, Some(message), None::<ErrorOptions>));
    make()
}
`,
  );
  expect(js).toContain('new AggregateError(errors, "several", { cause })');
  // A constructor JS makes alike called, Error's and AggregateError's, is
  // the class as a value, \`Error.bind(..)\`, as upstream binds it, so the
  // stack it makes has no frame of a function that constructs (rd404).
  expect(js).toContain("Error.bind(null, message, { cause })");
  expect(js).toContain("AggregateError.bind(null, errors, message)");
  expect(js).not.toContain("=> new Error");
  const aggregated = lib.boundAggregate([3], "all");
  expect([aggregated instanceof AggregateError, aggregated.errors, aggregated.message]).toEqual([true, [3], "all"]);
  const cause = { why: 1 };
  const made = lib.caused([1, 2], cause);
  expect([made instanceof AggregateError, made.errors, made.message, made.cause]).toEqual([true, [1, 2], "several", cause]);
  const error = lib.boundError("boom", cause);
  expect([error instanceof Error, error.message, error.cause]).toEqual([true, "boom", cause]);
  // Its stack's one frame of the module, in Node, is its caller's,
  // `boundError`'s, not a function's that constructs.
  const stack = runSync([node ?? "node", "-e", 'import("./lib.js").then((lib) => console.log(lib.boundError("boom", {}).stack))'], dir, 30_000);
  expect(stack.stdout.split("\n").filter((line) => line.includes("lib.js"))).toEqual([expect.stringContaining("boundError")]);
});

// rd581: React's Flight debug channel reader gives the next read's promise
// from `progress`, `return reader.read().then(progress).catch(error);`, and
// nothing once it's done: TypeScript's `void | PromiseLike<void>`, a
// handler's `Option<Promise<()>>`, which fulfils a promise of `()`. So the
// first read's promise settles once the last has, as upstream's chain does.
// Only of a `()`'s: `catch(|_| None)` of an `Option`'s is still told.
test("a then handler gives a promise or nothing, as void | PromiseLike<void>", async () => {
  const dir = fixture("js-promise-then-void");
  writeFileSync(join(dir, "lib.rs"), `use std::cell::RefCell;
use std::rc::Rc;

use js::{IteratorResult, JsError, Promise};

pub struct Reader {
    pub read: &'static dyn Fn() -> Promise<IteratorResult<Option<f64>>>,
    pub seen: RefCell<Vec<f64>>,
}

impl Reader {
    fn progress(self: &Rc<Self>, step: IteratorResult<Option<f64>>) -> Option<Promise<()>> {
        if step.done == Some(true) {
            return None;
        }
        self.seen.borrow_mut().push(step.value.unwrap_or(0.0));
        let reading = Rc::clone(self);
        let erroring = Rc::clone(self);
        Some((self.read)().then(move |step| reading.progress(step)).catch(move |e| erroring.error(e)))
    }

    fn error(&self, e: &JsError) {
        self.seen.borrow_mut().push(-1.0);
        println!("{}", e.message());
    }
}

pub fn start(reader: Rc<Reader>) -> Promise<()> {
    let reading = Rc::clone(&reader);
    let erroring = Rc::clone(&reader);
    (reader.read)().then(move |step| reading.progress(step)).catch(move |e| erroring.error(e))
}

pub fn seen(reader: &Reader) -> Vec<f64> {
    reader.seen.borrow().clone()
}

pub fn quiet(p: Promise<Option<u32>>) -> Promise<Option<u32>> {
    p.catch(|_| None)
}
`);
  const built = runSync([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target], root, 600_000);
  expect(built.code, built.stderr).toBe(0);
  expect(built.stderr).not.toContain("warning");
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("    if (step.done === true) {\n      return;\n    }\n");
  expect(js).toContain("    return reader\n      .read()\n      .then((step) => Reader.progress(reader, step))\n      .catch((e) => Reader.error(reader, e));\n");
  writeFileSync(join(dir, "run.mjs"), `import * as lib from "./lib.js";
const reads = (steps) => () => {
  const step = steps.shift();
  return step instanceof Error ? Promise.reject(step) : new Promise((resolve) => setTimeout(() => resolve(step), 1));
};
const reader = { read: reads([{ value: 1 }, { value: 2 }, { value: 3 }, { done: true }]), seen: [] };
console.log(await lib.start(reader), JSON.stringify(lib.seen(reader)));
const failing = { read: reads([{ value: 1 }, new Error("broken")]), seen: [] };
await lib.start(failing);
console.log(JSON.stringify(lib.seen(failing)), await lib.quiet(Promise.reject(new Error("no"))));
`);
  const ran = runSync([node!, join(dir, "run.mjs")], root, 30_000);
  expect(ran.stderr).toBe("");
  expect(ran.stdout).toBe("undefined [1,2,3]\nbroken\n[1,-1] undefined\n");
});

// rd584: TypeScript's `AsyncIterator`'s optional `throw?(e?: any)`. React's
// Flight reply server asks whether an iterator has one, `typeof
// iterator.throw === 'function'`, as `js::is_function` of a path from a
// value asks, and calls it, `iterator.throw(reason)`. A method's `throw` is
// its member, as each method's name is; `js::throw`'s, a function's, is
// still `throw value`.
test("an async iterator's throw is its optional method", async () => {
  const dir = fixture("js-async-iterator-throw");
  writeFileSync(join(dir, "lib.rs"), `use js::{AsyncIterator, Promise, Unknown};

fn noop() {}

pub fn stop(iterator: &'static AsyncIterator<f64>, reason: Option<&'static Unknown>) -> bool {
    if js::is_function((iterator, AsyncIterator::<f64>::throw)) {
        let _: Promise<()> = iterator.throw(Some(&reason)).then_with(|_| noop(), |_| noop());
        return true;
    }
    false
}

pub fn thrown(iterator: &'static AsyncIterator<f64>) -> Promise<bool> {
    iterator.throw(None).then_with(|step| step.done == Some(true), |_| false)
}

pub fn rethrow(value: Option<&Unknown>) {
    js::throw(value)
}
`);
  const built = runSync([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target], root, 600_000);
  expect(built.code, built.stderr).toBe(0);
  expect(built.stderr).not.toContain("warning");
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain('  if (typeof iterator.throw === "function") {\n    iterator.throw(reason).then(noop, noop);\n    return true;\n  }\n');
  expect(js).toContain("iterator.throw().then(");
  expect(js).toContain("  throw value;\n");
  writeFileSync(join(dir, "run.mjs"), `import * as lib from "./lib.js";
const caught = [];
async function* counted() {
  try {
    yield 1;
    yield 2;
  } catch (e) {
    caught.push(e);
  }
}
const iterator = counted();
await iterator.next();
console.log(lib.stop(iterator, "aborted"), lib.stop({ next: () => Promise.resolve({ done: true }) }, "aborted"));
await new Promise((resolve) => setTimeout(resolve, 1));
const fresh = counted();
await fresh.next();
console.log(JSON.stringify(caught), (await iterator.next()).done, await lib.thrown(fresh), JSON.stringify(caught));
try {
  lib.rethrow("thrown");
} catch (e) {
  console.log(e);
}
`);
  const ran = runSync([node!, join(dir, "run.mjs")], root, 30_000);
  expect(ran.stderr).toBe("");
  expect(ran.stdout).toBe('true false\n["aborted"] true true ["aborted",null]\nthrown\n');
});

// rd585: TypeScript's `AsyncIterable<T>`, whose `[Symbol.asyncIterator]()`
// makes its `AsyncIterator<T>`: React's Flight reply server's
// decodeReplyFromAsyncIterable takes one and calls `iterable[ASYNC_ITERATOR]()`.
// A method keyed by a global's value is a link name of the key, as
// TypeScript declares it, `[Symbol.asyncIterator]`, called of its receiver,
// its `this`; a key that's no global's path, or one of no receiver, is
// refused.
test("an async iterable's [Symbol.asyncIterator]() is its iterator", async () => {
  const dir = fixture("js-async-iterable");
  writeFileSync(join(dir, "lib.rs"), `use std::cell::RefCell;
use std::rc::Rc;

use js::{AsyncIterable, AsyncIterator, Promise};

fn each(iterator: &'static AsyncIterator<f64>, seen: Rc<RefCell<Vec<f64>>>) -> Promise<Vec<f64>> {
    iterator.next().then(move |step| {
        if step.done == Some(true) {
            return js::promise::resolve(seen.borrow().clone());
        }
        seen.borrow_mut().push(step.value);
        each(iterator, seen)
    })
}

pub fn collect(iterable: &AsyncIterable<f64>) -> Promise<Vec<f64>> {
    let iterator = iterable.async_iterator();
    each(iterator, Rc::default())
}
`);
  const built = runSync([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target], root, 600_000);
  expect(built.code, built.stderr).toBe(0);
  expect(built.stderr).not.toContain("warning");
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("  const iterator = iterable[Symbol.asyncIterator]();\n");
  writeFileSync(join(dir, "run.mjs"), `import * as lib from "./lib.js";
async function* counted() {
  yield 1;
  yield 2;
}
const own = {
  items: [3, 4],
  [Symbol.asyncIterator]() {
    let i = 0;
    return { next: async () => (i < this.items.length ? { value: this.items[i++] } : { done: true }) };
  },
};
console.log(JSON.stringify(await lib.collect(counted())), JSON.stringify(await lib.collect(own)));
`);
  const ran = runSync([node!, join(dir, "run.mjs")], root, 30_000);
  expect(ran.stderr).toBe("");
  expect(ran.stdout).toBe("[1,2] [3,4]\n");
  writeFileSync(join(dir, "refused.rs"), `pub struct Keyed(core::marker::PhantomData<js::JsObject>);

impl Keyed {
    #[cfg_attr(rust_js, rust_js::link_name = "[1]")]
    pub fn first(&self) -> f64 {
        unreachable!()
    }
}

#[cfg_attr(rust_js, rust_js::link_name = "[Symbol.iterator]")]
pub fn iterator(of: &Keyed) -> f64 {
    unreachable!()
}
`);
  const refused = runSync([compiler, join(dir, "refused.rs"), "-o", join(dir, "refused.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target], root, 600_000);
  expect(refused.code).not.toBe(0);
  expect(refused.stderr).toContain("`[1]` isn't a binding's path: `1` isn't a JS name");
  expect(refused.stderr).toContain("`[Symbol.iterator]` isn't a binding's path: a key, `[Symbol.asyncIterator]`, is a method's, of `&self` or `this`");
});

// rd519 (ADR 1066, the user's decision): an `unwrap()` of a binding
// getter's function read only as the callee of its call, or as the `this`
// of a binding's method, is the getter's read, React's Flight client's
// `console.createTask(name)` and `console.createTask.bind(console, name)`;
// one kept keeps `$unwrap`. Of a host without it, the call throws a
// `TypeError` where Rust's `unwrap()` panics.
test("a getter's function unwrapped where it's called is the read", async () => {
  const { js, lib } = await compiled(
    "getter-unwrapped-called",
    `use js::function;
use webapi::{ConsoleTask, console};

pub fn root_task(name: &str) -> &'static ConsoleTask {
    console::create_task().unwrap()(name)
}

pub fn bound(name: &str) -> &'static dyn Fn() -> &'static ConsoleTask {
    function::bind(console::create_task().unwrap(), Some(&console), (name.to_string(),))
}

pub fn kept() -> &'static dyn Fn(&str) -> &'static ConsoleTask {
    console::create_task().unwrap()
}

fn named(f: &dyn Fn(&str) -> &'static ConsoleTask) -> &'static ConsoleTask {
    f("given")
}

pub fn given() -> &'static ConsoleTask {
    named(console::create_task().unwrap())
}

pub fn local(f: Option<&dyn Fn(u32) -> u32>) -> u32 {
    f.unwrap()(1)
}

fn maker() -> Option<&'static dyn Fn(u32) -> u32> {
    None
}

pub fn made() -> u32 {
    maker().unwrap()(1)
}
`,
  );
  expect(js).toContain("export function rootTask(name) {\n  return console.createTask(name);\n}");
  expect(js).toContain("export function bound(name) {\n  return console.createTask.bind(console, name);\n}");
  expect(js).toContain("export function kept() {\n  return $unwrap(console.createTask);\n}");
  expect(js).toContain("return named($unwrap(console.createTask));");
  expect(js).toContain("return $unwrap(f)(1);");
  expect(js).toContain("return $unwrap(maker())(1);");
  const host = console as unknown as { createTask?: (name: string) => unknown };
  const before = host.createTask;
  try {
    host.createTask = function (this: unknown, name: string) {
      return { name, self: this === console };
    };
    expect(lib.rootTask("a")).toEqual({ name: "a", self: true });
    expect(lib.bound("b")()).toEqual({ name: "b", self: true });
    expect(lib.kept()("c")).toEqual({ name: "c", self: false });
    delete host.createTask;
    expect(() => lib.rootTask("a")).toThrow(TypeError);
    expect(() => lib.bound("b")).toThrow(TypeError);
    expect(() => lib.kept()).toThrow("called `Option::unwrap()` on a `None` value");
  } finally {
    if (before) host.createTask = before;
    else delete host.createTask;
  }
});
