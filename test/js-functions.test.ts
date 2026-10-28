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

    pub fn bumper(&self) -> Box<dyn Fn(u32) -> bool + '_> {
        function::with_this(move |this, (by,): (u32,)| {
            self.count.set(self.count.get() + by);
            this.is_some()
        })
    }
}

pub fn second() -> Box<dyn Fn(u32, u32) -> u32> {
    function::with_this(move |_, (_, b): (u32, u32)| b)
}

pub fn whole() -> Box<dyn Fn(u32, u32) -> String> {
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
pub fn held() -> Box<dyn Fn() -> bool> {
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
) -> Box<dyn Fn() -> String> {
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
