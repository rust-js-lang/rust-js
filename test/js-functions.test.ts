// A JS function of a closure that reads its `this`, `js::function::with_this`
// (ADR 0972): React's `update.callback = function callback() { .. this .. }`,
// whose `this` is the instance commitCallbacks calls it on. Each test is a
// program of its own, compiled, and its JS called on different `this`es.

import { beforeAll, expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";

import { runSync } from "./child";
import { buildReact, compiler, fixture, root, target } from "./support";

beforeAll(buildReact, 600_000);

const withJs = ["--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];

/** `source` compiled, its JS, and the module it is, loaded. */
async function compiled(name: string, source: string): Promise<{ js: string; lib: any }> {
  const dir = fixture(name);
  writeFileSync(join(dir, "lib.rs"), source);
  const built = runSync([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withJs], root, 600_000);
  expect(built.code, built.stderr).toBe(0);
  return { js: readFileSync(join(dir, "lib.js"), "utf8"), lib: await import(join(dir, "lib.js")) };
}

/** What rust-js says of `source`, which it refuses. */
function refused(name: string, source: string): string {
  const dir = fixture(name);
  writeFileSync(join(dir, "lib.rs"), source);
  const result = Bun.spawnSync([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withJs], { cwd: dir });
  expect(result.exitCode).not.toBe(0);
  return result.stderr.toString();
}

const source = `use std::cell::Cell;

use js::{Unknown, function};

fn name_of(this: Option<&Unknown>) -> String {
    match this {
        Some(this) => js::string(js::get(this, "name")),
        None => "none".to_string(),
    }
}

pub fn named(k: u32) -> &'static Unknown {
    js::unknown_of(function::with_this(move |this, (a, b): (u32, u32)| {
        format!("{}:{}", name_of(this), a + b + k)
    }))
}

pub struct Update {
    pub callback: Cell<Option<&'static Unknown>>,
    pub default: Cell<Option<&'static Unknown>>,
}

pub fn update() -> Update {
    Update { callback: Cell::new(None), default: Cell::new(None) }
}

pub fn initialize(update: &Update, label: String) {
    update.callback.set(Some(js::unknown_of(function::with_this(move |this, ()| {
        format!("{label} of {}", name_of(this))
    }))));
}

#[cfg_attr(rust_js, rust_js::link_name = "set data-on")]
fn set_data_on(this: &Unknown, value: &Unknown) {
    unreachable!()
}

pub fn unnamed(update: &Update, on: &'static Unknown) {
    update.default.set(Some(js::unknown_of(function::with_this(move |this, ()| name_of(this)))));
    set_data_on(on, js::unknown_of(function::with_this(move |this, ()| name_of(this))));
}

pub fn shadowing(update: &Update, callback: &'static dyn Fn() -> String) {
    update.callback.set(Some(js::unknown_of(function::with_this(move |_, ()| callback()))));
}

pub fn outer() -> &'static Unknown {
    js::unknown_of(function::with_this(move |outer, ()| {
        let inner = function::with_this(move |this, ()| format!("{} in {}", name_of(this), name_of(outer)));
        let arrow = move || name_of(outer);
        (arrow(), js::unknown_of(inner))
    }))
}

pub fn makers() -> (&'static Unknown, &'static Unknown, u32) {
    let made = Cell::new(0);
    let helper = |k: u32| {
        made.set(made.get() + k);
        js::unknown_of(function::with_this(move |this, ()| format!("{}{k}", name_of(this))))
    };
    (helper(1), helper(2), made.get())
}

pub fn called(on: &'static Unknown) -> (String, String, String) {
    let f = function::with_this(move |this, (a,): (u32,)| format!("{}{a}", name_of(this)));
    let given = js::string(function::call(&f, Some(&on), (2,)));
    let applied = js::string(function::apply(&f, Some(&on), Some(&vec![3])));
    (f(1), given, applied)
}

#[cfg_attr(rust_js, rust_js::class)]
pub struct Counter {
    count: Cell<u32>,
}

#[cfg_attr(rust_js, rust_js::class)]
impl Counter {
    pub fn new() -> Self {
        Counter { count: Cell::new(0) }
    }

    pub fn counted(&self) -> u32 {
        self.count.get()
    }

    pub fn bumper(&self) -> &dyn Fn(u32) -> bool {
        function::with_this(move |this, (by,): (u32,)| {
            self.count.set(self.count.get() + by);
            this.is_some()
        })
    }
}

pub fn second() -> &'static dyn Fn(u32, u32) -> u32 {
    function::with_this(move |_, (_, b): (u32, u32)| b)
}

pub fn whole() -> &'static dyn Fn(u32, u32) -> String {
    function::with_this(move |mut this, pair: (u32, u32)| {
        let before = name_of(this);
        if this.is_some() {
            this = None;
        }
        let (a, b) = pair;
        format!("{before} {} {}", name_of(this), a + b)
    })
}
`;

let made: Promise<{ js: string; lib: any }> | undefined;

/** `source`, compiled once for the tests that read it. */
function program(): Promise<{ js: string; lib: any }> {
  made ??= compiled("with-this", source);
  return made;
}

// Its `this` is what JS calls it on: `call`'s and `apply`'s, an object's
// whose method it is, `undefined` called alone; and from Rust, `f(1)`'s
// none, `function::call`'s and `function::apply`'s given.
test("with_this's closure is a JS function whose `this` is what it's called on", async () => {
  const { js, lib } = await program();
  expect(js).toContain("return function (a, b) {");
  const named = lib.named(10);
  expect(named.call({ name: "a" }, 1, 2)).toBe("a:13");
  expect(named.apply({ name: "b" }, [3, 4])).toBe("b:17");
  expect(named(5, 6)).toBe("none:21");
  const object = { name: "o", named };
  expect(object.named(1, 1)).toBe("o:12");
  expect(named.call(null, 0, 0)).toBe("none:10");
  expect(lib.called({ name: "on" })).toEqual(["none1", "on2", "on3"]);
});

// Stored as a property, it's named as it, as upstream writes React's:
// `update.callback = function callback() { .. }`.
test("a with_this function stored as a property is named as it", async () => {
  const { js, lib } = await program();
  expect(js).toContain("update.callback = function callback() {");
  const update = lib.update();
  lib.initialize(update, "label");
  expect(update.callback.name).toBe("callback");
  expect(update.callback.call({ name: "instance" })).toBe("label of instance");
  const callback = update.callback;
  expect(callback()).toBe("label of none");
  expect(update.callback()).toBe("label of undefined");
  // Not one of a word JS keeps, nor of a name no function has.
  const on: any = { name: "on" };
  lib.unnamed(update, on);
  expect([update.default.name, update.default.call({ name: "d" })]).toEqual(["", "d"]);
  expect([on["data-on"].name, on["data-on"]()]).toEqual(["", "on"]);
  // Not where it reads a variable of the name, which its own would be.
  lib.shadowing(update, () => "given");
  expect(update.callback.name).toBe("");
  expect(update.callback()).toBe("given");
});

// A function nested in it, another with_this's, reads its `this` through
// `const outer = this;`; an arrow in it reads its `this` itself, as JS's
// arrow sees the function's; one bound by a `let` is declared.
test("a function nested in a with_this function reads its `this` through a const", async () => {
  const { js, lib } = await program();
  expect(js).toContain("const outer = this;");
  expect(js).toContain("function inner() {");
  expect(js).toContain("const arrow = () => nameOf(this);");
  const [arrow, inner] = lib.outer().call({ name: "o" });
  expect(arrow).toBe("o");
  expect(inner.call({ name: "i" })).toBe("i in o");
  expect(inner()).toBe("none in o");
  // An arrow of statements a `let` binds is declared, a function in it
  // reading its own `this`, not the arrow's.
  expect(js).toContain("function helper(k) {");
  const [one, two, made] = lib.makers();
  expect([one.call({ name: "x" }), two(), made]).toEqual(["x1", "none2", 3]);
});

// A class's method's `self` is its `this`, which a with_this function in
// it reads through a const: its own `this` is its caller's.
test("a with_this function in a class's method reads `self` through a const", async () => {
  const { js, lib } = await program();
  expect(js).toContain("const counter = this;");
  const counter = new lib.Counter();
  const bump = counter.bumper();
  expect(bump.call({}, 2)).toBe(true);
  expect(bump(3)).toBe(false);
  expect(counter.counted()).toBe(5);
});

// Its `this` set is a variable of its own, `let this$ = this;`, as JS's
// `this` can't be; a tuple it reads whole, the array of its parameters.
test("a with_this closure that sets its `this` or reads its tuple whole", async () => {
  const { js, lib } = await program();
  expect(js).toContain("let this$ = this;");
  expect(js).toContain("const pair = [pair$1, pair$2];");
  const whole = lib.whole();
  expect(whole.call({ name: "w" }, 1, 2)).toBe("w none 3");
  expect(whole(1, 2)).toBe("none none 3");
  // An item it leaves out is a parameter still, of a name of its own.
  expect(lib.second()(1, 2)).toBe(2);
});

// A closure a variable holds isn't one it can write as a function: an
// error says to write it in the call.
test("with_this of a variable is refused", () => {
  const error = refused(
    "with-this-variable",
    `use js::{Unknown, function};
pub fn held() -> &'static dyn Fn() -> bool {
    let f = move |this: Option<&'static Unknown>, ()| this.is_some();
    function::with_this(f)
}
`,
  );
  expect(error).toContain("`js::function::with_this` takes a closure written in its call");
});

// `function::bind` of upstream's widest, React's `completeRoot.bind(null,
// root, finishedWork, ..)`: fifteen arguments, as many as a function takes
// (ADR 0907, amended), each nullable one, `null | Array<..>`, kept as the
// `Option` of what its `Option<&T>` parameter borrows and lent to each call,
// `errors.as_ref()`; an `Option` of an `Option` and of a `()` too, whose
// JS is each's own.
test("a function bound to fifteen arguments, each an Option of what its parameter borrows", async () => {
  const { js, lib } = await compiled("function-bind-wide", `use std::rc::Rc;

use js::{Kind, function};

pub struct Errors(pub Vec<u32>);

#[allow(clippy::too_many_arguments)]
fn complete(
    root: &Rc<u32>,
    errors: Option<&Rc<Errors>>,
    nested: Option<&Option<u32>>,
    unit: Option<&()>,
    name: Option<&String>,
    a: u32,
    b: u32,
    c: u32,
    d: u32,
    e: u32,
    f: u32,
    g: u32,
    h: u32,
    reason: &'static str,
    end: f64,
) -> String {
    let errors = errors.map_or(0, |errors| errors.0.len());
    let nested = match nested {
        Some(Some(n)) => i64::from(*n),
        Some(None) => -1,
        None => -2,
    };
    let sum = a + b + c + d + e + f + g + h;
    let name = name.map_or("none", |n| n.as_str());
    format!("{root} {errors} {nested} {} {name} {sum} {reason} {end}", unit.is_some())
}

pub fn later(
    root: &Rc<u32>,
    errors: Option<Rc<Errors>>,
    nested: Option<Option<u32>>,
    unit: Option<()>,
    name: Option<String>,
) -> &'static dyn Fn() -> String {
    function::bind(
        complete,
        Some(&Kind::Null),
        (Rc::clone(root), errors, nested, unit, name, 1, 2, 3, 4, 5, 6, 7, 8, "Throttled", 2.5),
    )
}

pub fn run() -> String {
    let root = Rc::new(7);
    let some = later(&root, Some(Rc::new(Errors(vec![1, 2]))), Some(None), Some(()), Some("n".to_string()));
    let none = later(&root, None, None, None, None);
    let held = later(&root, None, Some(Some(4)), None, None);
    format!("{} | {} | {} | {}", some(), some(), none(), held())
}
`);
  expect(js.replace(/\s+/g, " ")).toContain(
    `return complete.bind( null, root, errors, nested, unit, name, 1, 2, 3, 4, 5, 6, 7, 8, "Throttled", 2.5, );`,
  );
  expect(lib.run()).toBe(
    "7 2 -1 true n 36 Throttled 2.5 | 7 2 -1 true n 36 Throttled 2.5 | 7 0 -2 false none 36 Throttled 2.5 | 7 0 4 false none 36 Throttled 2.5",
  );
});

// react-dom-bindings' inputValueTracking replaces a node's `value` with a
// setter that calls the original on the node, `set: function (value) {
// currentValue = '' + value; set.call(this, value); }`. A descriptor's
// setter is given any value JS gives, `'static`, so a function `with_this`
// makes is one (ADR 0666, amended).
test("a function with_this makes is a property's setter", async () => {
  const { js, lib } = await compiled("with-this-setter", `use js::{IntoUnknown, PropertyDescriptor, Unknown, function, object};

pub fn track(node: &'static Unknown, field: &'static str, set: &'static dyn Fn(Option<&Unknown>)) {
    let setter = function::with_this(move |this, (value,): (Option<&'static Unknown>,)| {
        function::call(set, this.map(|this| this as &dyn IntoUnknown), (value,));
    });
    object::define_property(
        node,
        field,
        &PropertyDescriptor {
            configurable: Some(true),
            set: Some(&*setter),
            ..Default::default()
        },
    );
}
`);
  expect(js).toContain("  function setter(value) {\n    set.call(this, value);\n  }\n");
  expect(js).toContain("    set: setter,\n");
  const node: Record<string, unknown> = {};
  const seen: unknown[] = [];
  lib.track(node, "value", function (this: unknown, v: unknown) {
    seen.push([this, v]);
  });
  node.value = 42;
  expect(seen).toEqual([[node, 42]]);
});

// React's Scheduler's unstable_wrapCallback returns `function () { ..
// return callback.apply(this, arguments); .. }`: the callback is called on
// the wrapper's `this`, with what it's given. A function `with_this` makes
// whose last parameter is rest arguments as a value is JS's rest
// parameter, `function (...args)`, as a closure's is (ADR 0851), so a JS
// caller's arguments, and a Rust call's spread ones, are each one of them,
// not the first alone. One before its last is refused, as a closure's is.
test("a function with_this makes takes rest arguments as JS's rest parameter", async () => {
  const { js, lib } = await compiled("with-this-rest", `use js::{IntoUnknown, Rest, Unknown, function};

pub fn wrap_callback(
    callback: &'static dyn Fn(&dyn Rest<dyn IntoUnknown>) -> Option<&'static Unknown>,
) -> &'static dyn Fn(&'static dyn Rest<dyn IntoUnknown>) -> Option<&'static Unknown> {
    function::with_this(move |this, (args,): (&'static dyn Rest<dyn IntoUnknown>,)| {
        function::apply(callback, this.map(|this| this as &dyn IntoUnknown), Some(&args))
    })
}

pub fn first_and_rest() -> &'static dyn Fn(f64, &'static dyn Rest<f64>) -> f64 {
    function::with_this(|_, (first, rest): (f64, &'static dyn Rest<f64>)| first * rest.iter().sum::<f64>())
}

pub fn called() -> f64 {
    first_and_rest()(2.0, &(3.0, 4.0))
}
`);
  expect(js).toContain("return function (...args) {\n    return callback.apply(this, args);\n  };");
  expect(js).toContain("function (first, ...rest)");
  const obj = { n: 5, f: lib.wrapCallback(function (this: { n: number }, a: number, b: number) { return [this.n, a, b]; }) };
  expect(obj.f(1, 2)).toEqual([5, 1, 2]);
  expect([lib.firstAndRest()(2, 3, 4), lib.called()]).toEqual([14, 14]);
  expect(refused("with-this-rest-first", `use js::{Rest, function};

pub fn early() -> &'static dyn Fn(&'static dyn Rest<f64>, f64) -> f64 {
    function::with_this(|_, (rest, last): (&'static dyn Rest<f64>, f64)| rest.len() as f64 + last)
}
`)).toContain("does not support values of type `dyn std::ops::Fn(&dyn js::Rest<f64>, f64) -> f64`");
});

// inputValueTracking reads `Object.getOwnPropertyDescriptor(node.constructor.prototype,
// valueField)`: of any value, TypeScript's `any`, so of what `js::get` gives,
// `None` included, of which JS throws a TypeError as upstream's would (ADR
// 0666, amended); getOwnPropertyNames and getPrototypeOf too.
test("Object's reflection takes any value, what js::get gives", async () => {
  const { js, lib } = await compiled("object-reflection-any", `use js::{Unknown, object};

pub fn is_accessor(node: &Unknown, field: &str) -> bool {
    let descriptor = object::get_own_property_descriptor(js::get(js::get(node, "constructor"), "prototype"), field);
    descriptor.is_some_and(|descriptor| descriptor.get.is_some())
}

pub fn names_of(value: Option<&Unknown>) -> Vec<String> {
    object::get_own_property_names(value)
}

pub fn orphan(value: Option<&Unknown>) -> bool {
    object::get_prototype_of(value).is_none()
}
`);
  expect(js).toContain("Object.getOwnPropertyDescriptor(node.constructor.prototype, field);");
  class Input {
    get value() {
      return "";
    }
  }
  expect([lib.isAccessor(new Input(), "value"), lib.isAccessor({}, "value"), lib.namesOf({ a: 1 }), lib.orphan(Object.create(null))]).toEqual([true, false, ["a"], true]);
  expect(() => lib.isAccessor(Object.create(null), "value")).toThrow(TypeError);
  expect(() => lib.namesOf(undefined)).toThrow(TypeError);
});

// #402: a call of what may be a with_this function the crate makes, a
// `dyn Fn` or a generic `F` of its signature a field or an
// item holds, `(s.f)()`, gives it none, as Rust gives a with_this function
// `None` (ADR 0972): its callee held first, `const f = s.f; f()`, not
// `s.f()`, whose `this` is `s`. One of another signature, JS's own, is
// called through its object, `o.g(1)`, as JS calls it (#404, the user's
// decision); a closure of the crate's too, and a fn pointer, which no
// with_this function can be, `t.g()`.
test("a dyn Fn a field or an item holds is called with no this", async () => {
  const { js, lib } = await compiled("dyn-fn-field-this", `use js::Unknown;

pub struct S {
    pub f: &'static dyn Fn() -> bool,
}

pub fn field() -> bool {
    let s = S { f: js::function::with_this(|this: Option<&'static Unknown>, ()| this.is_some()) };
    (s.f)()
}

pub fn first(fs: &[Box<dyn Fn() -> bool>]) -> bool {
    if fs.is_empty() { false } else { fs[0]() }
}

pub fn pair<F: Fn() -> bool>(p: &(F, u32)) -> bool {
    (p.0)()
}

pub struct Table {
    pub g: fn() -> bool,
}

pub fn pointer(t: &Table) -> bool {
    (t.g)()
}

pub fn closure(n: u32) -> bool {
    let p = (move || n > 0, 2u32);
    (p.0)() && p.1 > 1
}

pub struct Other {
    pub g: Box<dyn Fn(u32) -> u32>,
}

pub fn other(o: &Other) -> u32 {
    (o.g)(1)
}
`);
  expect(js).toContain("  const f = s.f;\n  return f();\n");
  expect(js).toContain("const f = fs[0];\n");
  expect(js).toContain("  const f = p[0];\n  return f();\n");
  expect(js).toContain("return t.g();");
  expect(js).toContain("return !!p[0]() && p[1] > 1;");
  expect(js).toContain("return o.g(1);");
  const reads = function (this: unknown) {
    return this !== undefined;
  };
  expect([lib.field(), lib.first([reads]), lib.pair([reads, 1]), lib.pointer({ g: reads }), lib.closure(1), lib.other({ k: 5, g(this: { k: number }, n: number) { return this.k + n; } })]).toEqual([
    false,
    false,
    false,
    true,
    true,
    6,
  ]);
  // A program whose one such function is with_this's, given JS nothing.
  const alone = await compiled("dyn-fn-field-this-alone", `use js::Unknown;

struct S {
    f: &'static dyn Fn() -> bool,
}

pub fn field() -> bool {
    let s = S { f: js::function::with_this(|this: Option<&'static Unknown>, ()| this.is_some()) };
    (s.f)()
}

struct Rust {
    h: fn() -> bool,
}

fn yes() -> bool {
    true
}

pub fn rust() -> bool {
    let r = Rust { h: yes };
    (r.h)()
}
`);
  expect(alone.js).toContain("  const f = s.f;\n  return f();\n");
  expect(alone.js).toContain("return r.h();");
  expect([alone.lib.field(), alone.lib.rust()]).toEqual([false, true]);
  // One generic code makes, of what its callers give, may be any.
  const generic = await compiled("dyn-fn-field-this-generic", `use js::Unknown;

fn reader<T: Copy + 'static>(t: T) -> &'static dyn Fn() -> T {
    js::function::with_this(move |_: Option<&'static Unknown>, ()| t)
}

pub struct S {
    pub f: &'static dyn Fn() -> u32,
}

pub fn call(s: &S) -> u32 {
    (s.f)()
}

pub fn read() -> u32 {
    reader(2u8)() as u32
}
`);
  expect(generic.js).toContain("  const f = s.f;\n  return f();\n");
});

// What with_this makes is a `&dyn Fn`, as `Parameters` is of a tuple
// alone, sealed: never a fn pointer, whose call through what holds it is
// never held first (ADR 0972, amended).
test("with_this's Parameters is of a tuple alone", () => {
  const said = refused("with-this-parameters-sealed", `pub struct Mine;

impl<'l, R> js::function::Parameters<'l, R> for Mine {
    type Function = fn() -> R;
}
`);
  expect(said).toContain("js::sealed::Tuple");
});

// #405: a field of a with_this function's signature that the crate fills
// with its own closures alone, `Box::new(move || ..)`, holds none: called
// through its object, `tracker.stop()`, as upstream writes it. One filled
// with what may be one, a variable of it, is held first.
test("a dyn Fn field the crate fills with its own closures is called through its object", async () => {
  const { js, lib } = await compiled("dyn-fn-field-own-closures", `use js::Unknown;

pub struct Tracker {
    pub stop: Box<dyn Fn() -> bool>,
}

pub struct Held {
    pub f: &'static dyn Fn() -> bool,
}

pub fn tracker(n: u32) -> Tracker {
    Tracker { stop: Box::new(move || n > 1) }
}

pub fn stop(t: &Tracker) -> bool {
    (t.stop)()
}

pub fn held() -> bool {
    let f: &'static dyn Fn() -> bool = js::function::with_this(|this: Option<&'static Unknown>, ()| this.is_some());
    let h = Held { f };
    (h.f)()
}

pub struct Lent {
    pub g: &'static dyn Fn() -> bool,
}

fn set(g: &mut &'static dyn Fn() -> bool) {
    *g = js::function::with_this(|this: Option<&'static Unknown>, ()| this.is_some());
}

pub fn lent() -> bool {
    let mut l = Lent { g: &|| true };
    set(&mut l.g);
    (l.g)()
}
`);
  expect(js).toContain("return t.stop();");
  expect(js).toContain("  const f$1 = h.f;\n  return f$1();\n");
  expect(js).toContain("  const g$1 = l.g;\n  return g$1();\n");
  expect([lib.stop(lib.tracker(2)), lib.held(), lib.lent()]).toEqual([true, false, false]);
});

// React's `callback()`: a trailing `None` given a function value, a `dyn Fn`,
// a pointer or one `function::bind` makes, is no argument, as a person
// leaves an optional one out (ADR 0443, amended). The user's exception, ADR
// 0262's second case: a JS function that counts its arguments sees none.
test("a trailing None given a function value is no argument", async () => {
  const { js, lib } = await compiled("none-left-out", `use js::{Unknown, function};

pub fn called(cb: &dyn Fn(Option<&Unknown>) -> f64, pointer: fn(f64, Option<f64>) -> f64) -> f64 {
    cb(None) + pointer(1.0, None)
}

pub fn bound(f: &'static dyn Fn(Option<f64>) -> f64) -> f64 {
    function::bind(f, None, ())(None)
}
`);
  expect(js).toContain("return cb() + pointer(1);");
  expect(js).toContain("return f.bind()();");
  const count = (...args: unknown[]) => args.length;
  expect([lib.called(count, count), lib.bound(count)]).toEqual([1, 0]);
});

// React DOM's suspensey resources: `function onUnsuspend() { this.count--;
// .. }`, bound to the state, `onUnsuspend.bind(state)`. A thread-local's
// with_this function is declared, and a variable that only holds its
// `this`, cast to what's bound, is `this` where it's read; one a function
// made in it reads keeps its `const` (ADR 0972, amended).
test("a with_this thread-local is a function declaration, its cast this read in place", async () => {
  const { js, lib } = await compiled("with-this-declared", `use std::cell::Cell;

use js::{Promise, function};

pub struct SuspendedState {
    pub count: Cell<f64>,
}

unsafe impl js::Defined for SuspendedState {}

thread_local! {
    static ON_UNSUSPEND: &'static dyn Fn() = function::with_this(|this, ()| {
        // Safety: it's only called bound to a suspended state.
        let state: &SuspendedState = unsafe { js::cast(this) };
        state.count.set(state.count.get() - 1.0);
    });
    pub static ON_DONE: &'static dyn Fn() -> bool = function::with_this(|this, ()| this.is_some());
    static PICK: Cell<&'static dyn Fn() -> f64> = Cell::new(function::with_this(|_, ()| 1.0));
}

pub fn swap() -> f64 {
    let before = PICK.get()();
    PICK.set(function::with_this(|_, ()| 2.0));
    before + PICK.get()()
}

pub fn suspend_on(state: &'static SuspendedState, loading: Promise<()>) {
    let ping = function::bind(ON_UNSUSPEND.with(|f| *f), Some(state), ());
    loading.then_with::<(), _, _>(move |_| ping(), move |_| ping());
}

pub fn nested() -> &'static dyn Fn() -> f64 {
    function::with_this(|this, ()| {
        // SAFETY: called on a suspended state.
        let state: &SuspendedState = unsafe { js::cast(this) };
        let inner = function::with_this(move |_, ()| state.count.get());
        inner() + state.count.get()
    })
}

pub fn shadowed() -> &'static dyn Fn(bool) -> f64 {
    function::with_this(|this, (c,): (bool,)| {
        // SAFETY: called on a suspended state.
        let state: &SuspendedState = unsafe { js::cast(this) };
        let n = state.count.get();
        if c {
            let state = n * 2.0;
            return state + 1.0;
        }
        n
    })
}
`);
  expect(js).toContain("const n = this.count;\n    if (c) {\n      const state = n * 2;\n      return state + 1;");
  expect([lib.shadowed().call({ count: 3 }, true), lib.shadowed().call({ count: 3 }, false)]).toEqual([7, 3]);
  expect(js).toContain(
    "function onUnsuspend() {\n  // Safety: it's only called bound to a suspended state.\n  this.count--;\n}\n",
  );
  expect(js).toContain("export function onDone() {\n  return this != null;\n}\n");
  // One its module sets stays a variable.
  expect(js).toContain("let pick = function () {\n  return 1;\n};\n");
  expect(lib.swap()).toBe(3);
  expect(js).toContain("const state = this;\n    function inner() {\n      return state.count;");
  const state = { count: 2 };
  lib.suspendOn(state, Promise.resolve());
  await Promise.resolve();
  await Promise.resolve();
  expect(state.count).toBe(1);
  expect([lib.onDone.call(state), lib.nested().call({ count: 3 })]).toEqual([true, 6]);
});

// ADR 0602, amended (rd289): a binding's function lent as a `&dyn Fn` is
// the JS function itself, as one taken as a `fn` is, so a caller that
// compares functions, React's registerDefaultIndicator, sees one, where
// it was a new arrow each time.
test("a binding lent as a dyn Fn is the function itself", async () => {
  const dir = fixture("binding-lent-as-dyn");
  writeFileSync(join(dir, "indicator.js"), "export function defaultIndicator() {\n  return 'shown';\n}\n");
  writeFileSync(
    join(dir, "lib.rs"),
    `#[cfg_attr(rust_js, rust_js::link_name = "./indicator.js#defaultIndicator")]
fn default_indicator() -> Option<&'static js::Unknown> {
    unreachable!()
}

pub fn indicator() -> &'static dyn Fn() -> Option<&'static js::Unknown> {
    &default_indicator
}

pub fn shown() -> bool {
    indicator()().is_some()
}
`,
  );
  const built = runSync([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withJs], root, 600_000);
  expect(built.code, built.stderr).toBe(0);
  expect(readFileSync(join(dir, "lib.js"), "utf8")).toContain("return defaultIndicator;");
  const lib = await import(join(dir, "lib.js"));
  const { defaultIndicator } = await import(join(dir, "indicator.js"));
  expect([lib.indicator() === defaultIndicator, lib.shown()]).toEqual([true, true]);
});
