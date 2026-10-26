// A JS class (ADR 0761): a struct and its impl marked `#[rust_js::class]`
// are a constructor function and its prototype, as React's
// ReactBaseClasses.js writes `Component`: `function Component(props) {
// this.props = props; }`, `Component.prototype.setState = function (..) {
// .. }`. A JS `class X extends Component` runs it. Each test is a program
// of its own, compiled and run in Node.

import { beforeAll, expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { runSync } from "./child";
import { node } from "./programs";
import { buildReact, compiler, fixture, root, run, target } from "./support";

beforeAll(buildReact, 600_000);

const flags = (dir: string) => ["--declarations", join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];

/** `rust` compiled in a fixture of `name`, against the js crate: its JS, its declarations, and where they are. */
function compiled(name: string, rust: string): { js: string; ts: string; dir: string } {
  const dir = fixture(name);
  writeFileSync(join(dir, "lib.rs"), rust);
  run([compiler, ...flags(dir)]);
  return { js: readFileSync(join(dir, "lib.js"), "utf8"), ts: readFileSync(join(dir, "lib.d.ts"), "utf8"), dir };
}

/** What the compiler says of `rust`, which it refuses. */
function refused(name: string, rust: string): string {
  const dir = fixture(name);
  writeFileSync(join(dir, "lib.rs"), rust);
  const ran = runSync([compiler, ...flags(dir)], root, 60_000);
  expect(ran.code).not.toBe(0);
  return ran.stderr;
}

// React's packages/react/src/ReactBaseClasses.js, as a port writes it:
// `Component` and `PureComponent` classes, and `PureComponent`'s prototype
// a `ComponentDummy`, which inherits `Component`'s, set as React sets it.
const baseClasses = `use js::object::assign;
use js::{Kind, Unknown};

/// What a component's \`updater\` is: the renderer's.
pub struct Updater {
    pub enqueue_set_state: fn(&Unknown, Option<&Unknown>, Option<&'static dyn Fn()>, &str),
    pub enqueue_force_update: fn(&Unknown, Option<&'static dyn Fn()>, &str),
}

/// This is the abstract API for an update queue.
#[cfg_attr(rust_js, rust_js::name = "ReactNoopUpdateQueue")]
pub static REACT_NOOP_UPDATE_QUEUE: Updater = Updater {
    enqueue_set_state: |_, _, _, _| {},
    enqueue_force_update: |_, _, _| {},
};

pub struct EmptyObject {}

#[cfg_attr(rust_js, rust_js::name = "emptyObject")]
static EMPTY_OBJECT: EmptyObject = EmptyObject {};

/// Base class helpers for the updating state of a component.
#[cfg_attr(rust_js, rust_js::class)]
pub struct Component {
    pub props: Option<&'static Unknown>,
    pub context: Option<&'static Unknown>,
    pub refs: &'static EmptyObject,
    pub updater: &'static Updater,
}

unsafe impl js::Defined for Component {}

#[cfg_attr(rust_js, rust_js::class)]
impl Component {
    pub const IS_REACT_COMPONENT: EmptyObject = EmptyObject {};

    pub fn new(
        props: Option<&'static Unknown>,
        context: Option<&'static Unknown>,
        updater: Option<&'static Updater>,
    ) -> Self {
        Component {
            props,
            context,
            // If a component has string refs, we will assign a different object later.
            refs: &EMPTY_OBJECT,
            // We initialize the default updater but the real one gets injected by the
            // renderer.
            updater: updater.unwrap_or(&REACT_NOOP_UPDATE_QUEUE),
        }
    }

    /// Sets a subset of the state.
    pub fn set_state(&self, partial_state: Option<&Unknown>, callback: Option<&'static dyn Fn()>) {
        if !matches!(
            js::classify(partial_state),
            Kind::Object(_) | Kind::Array(_) | Kind::Function(_) | Kind::Undefined | Kind::Null
        ) {
            panic!(
                "takes an object of state variables to update or a function which returns an object of state variables."
            );
        }
        (self.updater.enqueue_set_state)(js::unknown(self), partial_state, callback, "setState");
    }

    /// Forces an update.
    pub fn force_update(&self, callback: Option<&'static dyn Fn()>) {
        (self.updater.enqueue_force_update)(js::unknown(self), callback, "forceUpdate");
    }
}

#[cfg_attr(rust_js, rust_js::class)]
struct ComponentDummy {}

unsafe impl js::Defined for ComponentDummy {}

#[cfg_attr(rust_js, rust_js::class)]
impl ComponentDummy {
    fn new() -> Self {
        ComponentDummy {}
    }
}

/// Convenience component with default shallow equality check for sCU.
#[cfg_attr(rust_js, rust_js::class)]
pub struct PureComponent {
    pub props: Option<&'static Unknown>,
    pub context: Option<&'static Unknown>,
    pub refs: &'static EmptyObject,
    pub updater: &'static Updater,
}

#[cfg_attr(rust_js, rust_js::class)]
impl PureComponent {
    pub fn new(
        props: Option<&'static Unknown>,
        context: Option<&'static Unknown>,
        updater: Option<&'static Updater>,
    ) -> Self {
        PureComponent {
            props,
            context,
            refs: &EMPTY_OBJECT,
            updater: updater.unwrap_or(&REACT_NOOP_UPDATE_QUEUE),
        }
    }
}

js::on_load! {
    js::class::<ComponentDummy>().set_prototype(js::class::<Component>().prototype());
    let pure_component_prototype = js::unknown_of(ComponentDummy::new());
    js::class::<PureComponent>().set_prototype(pure_component_prototype);
    js::set(pure_component_prototype, "constructor", js::class::<PureComponent>());
    // Avoid an extra prototype jump for these methods.
    assign(pure_component_prototype, (js::class::<Component>().prototype(),));
    js::set(pure_component_prototype, "isPureReactComponent", true);
}

/// A component made and updated by Rust: \`new Component(..)\`, and its
/// method called on it, \`component.forceUpdate(..)\`.
pub fn mounted(props: Option<&'static Unknown>) -> Component {
    let component = Component::new(props, None, None);
    component.force_update(None);
    component
}

/// Its method as a value: the arrow of its call.
pub fn force_each(components: &[Component]) {
    components.iter().for_each(|c| c.force_update(None));
    let update: fn(&Component, Option<&'static dyn Fn()>) = Component::force_update;
    for component in components {
        update(component, None);
    }
}
`;

test("React's Component is a constructor function and its prototype", () => {
  const { js, ts, dir } = compiled("class-base", baseClasses);
  expect(js).toContain(`export function Component(props, context, updater) {
  this.props = props;
  this.context = context;
  // If a component has string refs, we will assign a different object later.
  this.refs = emptyObject;
  // We initialize the default updater but the real one gets injected by the
  // renderer.
  this.updater = updater ?? ReactNoopUpdateQueue;
}

Component.prototype.isReactComponent = {};

/** Sets a subset of the state. */
Component.prototype.setState = function (partialState, callback) {`);
  expect(js).toContain(`  this.updater.enqueueSetState(this, partialState, callback, "setState");
};

/** Forces an update. */
Component.prototype.forceUpdate = function (callback) {
  this.updater.enqueueForceUpdate(this, callback, "forceUpdate");
};

function ComponentDummy() {}
`);
  expect(js).toContain(`ComponentDummy.prototype = Component.prototype;
const purComponentPrototype`.replace("purComponentPrototype", "pureComponentPrototype = new ComponentDummy();"));
  expect(js).toContain(`PureComponent.prototype = pureComponentPrototype;
pureComponentPrototype.constructor = PureComponent;
// Avoid an extra prototype jump for these methods.
Object.assign(pureComponentPrototype, Component.prototype);
pureComponentPrototype.isPureReactComponent = true;`);
  // A Rust call of its constructor is \`new\`, of its method a call of the prototype's.
  expect(js).toContain(`export function mounted(props) {
  const component = new Component(props);
  component.forceUpdate(undefined);
  return component;
}`);
  expect(js).toContain("const update = (a, b) => a.forceUpdate(b);");
  expect(ts).toContain(`/** Base class helpers for the updating state of a component. */
export declare class Component {
    constructor(props?: unknown | null | undefined, context?: unknown | null | undefined, updater?: Updater | null | undefined);
    props?: unknown;
    context?: unknown;
    refs: EmptyObject;
    updater: Updater;
    isReactComponent: EmptyObject;
    /** Sets a subset of the state. */
    setState(partialState?: unknown | null | undefined, callback?: (() => void) | null | undefined): void;
    /** Forces an update. */
    forceUpdate(callback?: (() => void) | null | undefined): void;
}`);
  // In Node: a JS class of each, \`class X extends Component\`, its
  // \`super(props)\` the constructor function, \`this.setState\` the
  // prototype's, seeing the subclass's \`this\`.
  writeFileSync(join(dir, "run.mjs"), `import { Component, PureComponent, mounted, forceEach } from "./lib.js";
const calls = [];
const updater = {
  enqueueSetState(instance, state, callback, name) { calls.push([instance.constructor.name, instance.label(), state, name]); },
  enqueueForceUpdate(instance, callback, name) { calls.push([instance.constructor.name, name]); },
};
class Counter extends Component {
  constructor(props) { super(props, undefined, updater); this.state = { count: 0 }; }
  label() { return "counter " + this.props.start; }
  increment() { this.setState({ count: this.state.count + 1 }); }
}
class Pure extends PureComponent {
  constructor(props) { super(props, undefined, updater); }
  label() { return "pure"; }
}
const counter = new Counter({ start: 3 });
counter.increment();
counter.forceUpdate();
new Pure({}).setState(() => ({}));
console.log(JSON.stringify(calls));
console.log(Object.keys(counter).join(), counter.refs === new Counter({}).refs);
console.log(counter instanceof Component, new Pure({}) instanceof Component, new Pure({}) instanceof PureComponent);
console.log(JSON.stringify(Component.prototype.isReactComponent), PureComponent.prototype.isPureReactComponent, Component.prototype.isPureReactComponent);
console.log(PureComponent.prototype.constructor === PureComponent, Object.hasOwn(PureComponent.prototype, "setState"));
try { counter.setState(5); } catch (e) { console.log(e instanceof Error, e.message); }
const made = mounted({ a: 1 });
console.log(made instanceof Component, made.props.a, made.updater.enqueueSetState.length);
forceEach([counter]);
console.log(calls.length);
`);
  const ran = runSync([node!, join(dir, "run.mjs")], root, 30_000);
  expect(ran.stderr).toBe("");
  expect(ran.stdout).toBe(
    `[["Counter","counter 3",{"count":1},"setState"],["Counter","forceUpdate"],["Pure","pure",null,"setState"]]
props,context,refs,updater,state true
true true true
{} true undefined
true true
true takes an object of state variables to update or a function which returns an object of state variables.
true 1 0
5
`,
  );
});

// What a class is refused: what isn't its constructor function and its
// prototype, each with what to write.
test("what a class is refused says what to write", () => {
  const head = "#[cfg_attr(rust_js, rust_js::class)]\npub struct C {\n    pub n: u32,\n}\n";
  const made = "    pub fn new() -> Self {\n        C { n: 0 }\n    }\n";
  const marked = "#[cfg_attr(rust_js, rust_js::class)]\nimpl C {\n";
  const cases: [string, string, string][] = [
    ["class-trait", `#[derive(Clone)]\n${head}${marked}${made}}\n`, "implements no trait but a marker's, of no items, not `std::clone::Clone`"],
    ["class-by-value", `${head}${marked}${made}    pub fn take(self) -> u32 {\n        self.n\n    }\n}\n`, "method takes `&self` or `&mut self`, `this`, not `self` by value"],
    ["class-generic", "#[cfg_attr(rust_js, rust_js::class)]\npub struct C<T> {\n    pub n: T,\n}\n#[cfg_attr(rust_js, rust_js::class)]\nimpl<T> C<T> {\n    pub fn new(n: T) -> Self {\n        C { n }\n    }\n}\n", "has no type or const parameters"],
    ["class-literal", `${head}${marked}${made}}\npub fn other() -> C {\n    C { n: 1 }\n}\n`, "is made by its constructor, `C::new(..)`, not a struct literal"],
    ["class-unmarked-impl", `${head}impl C {\n${made}}\n`, "struct's impl is marked `#[rust_js::class]` too"],
    ["class-unmade", `${head}${marked}    pub fn get(&self) -> u32 {\n        self.n\n    }\n}\n`, "is made by its constructor, `fn new(..) -> Self`, in its impl"],
    ["class-tuple", "#[cfg_attr(rust_js, rust_js::class)]\npub struct C(pub u32);\n#[cfg_attr(rust_js, rust_js::class)]\nimpl C {\n    pub fn new() -> Self {\n        C(0)\n    }\n}\n", "is a struct of named fields"],
    ["class-method-of-a-field", `${head}${marked}${made}    pub fn n(&self) -> u32 {\n        self.n\n    }\n}\n`, "has no method or constant named as its field, `n`, which `this.n` would hide"],
    ["class-of-no-class", "pub struct S;\npub fn f() -> &'static js::Class {\n    js::class::<S>()\n}\n", "`js::class::<T>()` of a `T` that's no `#[rust_js::class]` struct"],
  ];
  for (const [name, rust, message] of cases) {
    expect(refused(name, rust)).toContain(message);
  }
});

// React's `/** Base class helpers .. */ function Component(..)`: the
// struct's doc is the constructor function's, as its `.d.ts` has it of the
// class, before `new`'s own (ADR 0761, amended).
test("a class struct's doc is its constructor function's", () => {
  const { js } = compiled("class-doc", `/// Base class helpers for the updating state of a component.
#[cfg_attr(rust_js, rust_js::class)]
pub struct Component {
    // What it's given.
    pub props: f64,
}

unsafe impl js::Defined for Component {}

#[cfg_attr(rust_js, rust_js::class)]
impl Component {
    pub const LIMIT: f64 = 1.0;

    /// Made of its props.
    pub fn new(props: f64) -> Self {
        Component { props }
    }
}

/// A plain struct's doc is of no JS.
pub struct Plain {
    pub n: f64,
}

pub fn plain(n: f64) -> Plain {
    Plain { n }
}
`);
  expect(js).toContain(`/** Base class helpers for the updating state of a component. */
/** Made of its props. */
export function Component(props) {
  this.props = props;
}

Component.prototype.limit = 1;
`);
  expect(js).not.toContain("What it's given");
  expect(js).not.toContain("A plain struct's doc");
});

// A field that's a closure of statements is a method (ADR 0882), but not
// one that reads the class's `this`, which a method would read as the
// object it's in: an arrow still, whose `this` is the class's.
test("a field's closure that reads this stays an arrow", async () => {
  const { js, dir } = compiled("class-this-in-a-field", `#[cfg_attr(rust_js, rust_js::class)]
pub struct Counter {
    pub n: u32,
}

pub struct Reader<'a> {
    pub read: Box<dyn Fn(u32) -> u32 + 'a>,
    pub plain: Box<dyn Fn(u32) -> u32>,
}

#[cfg_attr(rust_js, rust_js::class)]
impl Counter {
    pub fn new(n: u32) -> Self {
        Counter { n }
    }

    pub fn reader(&self) -> Reader<'_> {
        Reader {
            read: Box::new(move |k| {
                let x = self.n + k;
                x * 2
            }),
            plain: Box::new(move |k| {
                let x = k + 1;
                x * 2
            }),
        }
    }
}
`);
  expect(js).toContain("    read: (k) => {\n      const x = (this.n + k) >>> 0;\n");
  expect(js).toContain("    plain(k) {\n      const x = (k + 1) >>> 0;\n");
  const { Counter } = await import(join(dir, "lib.js"));
  const reader = new Counter(4).reader();
  expect([reader.read(1), reader.plain(1)]).toEqual([10, 4]);
});

// A class is its module's default, `js::export_default!(Complex)`, as JS's
// `export default class Complex` (#345): its constructor function exported
// as the default alone, declared a class, and imported as the default by
// a module that uses it.
test("a class is its module's default export", async () => {
  const dir = fixture("class-default");
  writeFileSync(join(dir, "complex.rs"), `#[cfg_attr(rust_js, rust_js::class)]
pub struct Complex {
    pub re: f64,
    pub im: f64,
}

#[cfg_attr(rust_js, rust_js::class)]
impl Complex {
    pub fn new(re: f64, im: f64) -> Self {
        Complex { re, im }
    }

    pub fn radius(&self) -> f64 {
        (self.re * self.re + self.im * self.im).sqrt()
    }
}

js::export_default!(Complex);
`);
  writeFileSync(join(dir, "lib.rs"), `mod complex;

use complex::Complex;

pub fn unit(turn: f64) -> f64 {
    Complex::new(turn.cos(), turn.sin()).radius()
}

pub fn real(re: f64) -> Complex {
    Complex::new(re, 0.0)
}
`);
  run([compiler, ...flags(dir)]);
  const complex = readFileSync(join(dir, "complex.js"), "utf8");
  expect(complex).toContain("\nfunction Complex(re, im) {\n");
  expect(complex).toContain("\nexport default Complex;\n");
  expect(complex).not.toContain("export function Complex");
  const declared = readFileSync(join(dir, "complex.d.ts"), "utf8");
  expect(declared).toContain("declare class Complex {\n");
  expect(declared).toContain("\nexport default Complex;\n");
  const lib = readFileSync(join(dir, "lib.js"), "utf8");
  expect(lib).toContain('import Complex from "./complex.js";');
  expect(lib).toContain("new Complex(");
  expect(readFileSync(join(dir, "lib.d.ts"), "utf8")).toContain('import type Complex from "./complex.js";');
  const { default: Default } = await import(join(dir, "complex.js"));
  expect(new Default(3, 4).radius()).toBe(5);
  const { unit } = await import(join(dir, "lib.js"));
  expect(unit(1)).toBeCloseTo(1, 12);
});

// A class's method setting a `Cell` field, `self.reads.set(..)`, sets
// `this.reads` (#359): no handle whose `get value() { return this.reads; }`
// would read the handle's own `reads`. And one it hands on, `bump(&self.reads)`,
// is on the instance's field, its `this` named outside the accessors.
test("a class's method sets its Cell fields on this", async () => {
  const { js, dir } = compiled("class-cell-field", `use std::cell::Cell;

fn bump(count: &Cell<u32>) {
    count.set(count.get() + 1);
}

#[cfg_attr(rust_js, rust_js::class)]
pub struct Counted {
    row: i32,
    reads: Cell<u32>,
    firsts: [Cell<u32>; 2],
}

#[cfg_attr(rust_js, rust_js::class)]
impl Counted {
    pub fn new(row: i32) -> Self {
        Counted {
            row,
            reads: Cell::new(0),
            firsts: [Cell::new(1), Cell::new(2)],
        }
    }

    pub fn row_counted(&self) -> i32 {
        self.reads.set(self.reads.get() + 1);
        self.row
    }

    pub fn row_bumped(&self) -> i32 {
        bump(&self.reads);
        self.row
    }

    pub fn read_count(&self) -> u32 {
        self.reads.get()
    }

    pub fn twice(&self) -> u32 {
        let reads = &self.reads;
        reads.set(reads.get() * 2);
        self.firsts[0].set(self.firsts[0].get() + reads.get());
        self.firsts[0].get()
    }
}
`);
  expect(js).toContain("  this.reads = (this.reads + 1) >>> 0;\n");
  expect(js).toContain("  this.reads = (this.reads * 2) >>> 0;\n");
  expect(js.match(/get value\(\)/g)?.length).toBe(1);
  const { Counted } = await import(join(dir, "lib.js"));
  const counted = new Counted(3);
  expect(counted.rowCounted() + counted.rowCounted() + counted.rowBumped()).toBe(9);
  expect(counted.readCount()).toBe(3);
  expect([counted.twice(), counted.readCount()]).toEqual([7, 6]);
});

// javascript-algorithms' `QueenPosition`: `get leftDiagonal()`, read as
// `position.leftDiagonal` (#357). A method marked `rust_js::getter` is the
// prototype's getter, `Object.defineProperty(QueenPosition.prototype,
// "leftDiagonal", { get() { .. } })`, declared `get leftDiagonal(): number`;
// a Rust call of it reads it, `p.leftDiagonal`, and as a value it's the
// arrow of that read.
test("a class's getter is its prototype's", async () => {
  const { js, ts, dir } = compiled("class-getter", `#[cfg_attr(rust_js, rust_js::class)]
pub struct QueenPosition {
    pub row_index: i32,
    pub column_index: i32,
}

#[cfg_attr(rust_js, rust_js::class)]
impl QueenPosition {
    pub fn new(row_index: i32, column_index: i32) -> Self {
        QueenPosition {
            row_index,
            column_index,
        }
    }

    /// Each position on the same left (\\\\) diagonal has the same difference.
    #[cfg_attr(rust_js, rust_js::getter)]
    pub fn left_diagonal(&self) -> i32 {
        self.row_index - self.column_index
    }
}

pub fn same_diagonal(a: &QueenPosition, b: &QueenPosition) -> bool {
    a.left_diagonal() == b.left_diagonal()
}

pub fn diagonals(positions: &[QueenPosition]) -> Vec<i32> {
    positions.iter().map(QueenPosition::left_diagonal).collect()
}
`);
  expect(js).toContain(`Object.defineProperty(QueenPosition.prototype, "leftDiagonal", {
  get() {
    return (this.rowIndex - this.columnIndex) | 0;
  },
});`);
  expect(js).toContain("return a.leftDiagonal === b.leftDiagonal;");
  expect(js).not.toContain("leftDiagonal()");
  expect(ts).toContain("    get leftDiagonal(): number;\n");
  const { QueenPosition, sameDiagonal, diagonals } = await import(join(dir, "lib.js"));
  const position = new QueenPosition(2, 1);
  expect([position.leftDiagonal, Object.keys(position)]).toEqual([1, ["rowIndex", "columnIndex"]]);
  expect(sameDiagonal(position, new QueenPosition(3, 2))).toBe(true);
  expect(diagonals([position, new QueenPosition(0, 4)])).toEqual([1, -4]);
});

// A getter is a class's method of `&self` alone, which gives a value: what
// else is marked one is refused, saying what to write (#357).
test("what a getter is refused says what to write", () => {
  const head = "#[cfg_attr(rust_js, rust_js::class)]\npub struct C {\n    pub n: u32,\n}\n";
  const marked = "#[cfg_attr(rust_js, rust_js::class)]\nimpl C {\n    pub fn new() -> Self {\n        C { n: 0 }\n    }\n";
  const getter = "    #[cfg_attr(rust_js, rust_js::getter)]\n";
  const cases: [string, string, string][] = [
    ["getter-argument", `${head}${marked}${getter}    pub fn plus(&self, k: u32) -> u32 {\n        self.n + k\n    }\n}\n`, "getter is a method of `&self` alone"],
    ["getter-mut", `${head}${marked}${getter}    pub fn bump(&mut self) -> u32 {\n        self.n += 1;\n        self.n\n    }\n}\n`, "getter is a method of `&self` alone"],
    ["getter-static", `${head}${marked}${getter}    pub fn zero() -> u32 {\n        0\n    }\n}\n`, "getter is a method of `&self` alone"],
    ["getter-of-no-class", `pub struct S {\n    pub n: u32,\n}\nimpl S {\n${getter}    pub fn twice(&self) -> u32 {\n        self.n * 2\n    }\n}\n`, "getter is a `#[rust_js::class]`'s method"],
  ];
  for (const [name, rust, message] of cases) {
    expect(refused(name, rust)).toContain(message);
  }
});

// javascript-algorithms' `QueenPosition`: `toString() { return
// \`${this.rowIndex},${this.columnIndex}\`; }` (#358). A class's `impl
// Display` is its prototype's `toString`: Rust's `{p}` and `p.to_string()`
// are JS's text of it, `${p}`, as JS's `String(p)` is, and a `T: Display`
// it's given to shows it by `String`.
test("a class's Display is its prototype's toString", async () => {
  const { js, ts, dir } = compiled("class-display", `use std::fmt;

#[cfg_attr(rust_js, rust_js::class)]
pub struct QueenPosition {
    pub row_index: i32,
    pub column_index: i32,
}

#[cfg_attr(rust_js, rust_js::class)]
impl QueenPosition {
    pub fn new(row_index: i32, column_index: i32) -> Self {
        QueenPosition {
            row_index,
            column_index,
        }
    }
}

impl fmt::Display for QueenPosition {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{},{}", self.row_index, self.column_index)
    }
}

fn joined<T: fmt::Display>(items: &[T]) -> String {
    items.iter().map(|item| item.to_string()).collect::<Vec<_>>().join(" ")
}

pub fn shown(position: &QueenPosition) -> String {
    format!("at {position}")
}

pub fn text(position: &QueenPosition) -> String {
    position.to_string()
}

pub fn all(positions: &[QueenPosition]) -> String {
    joined(positions)
}
`);
  expect(js).toContain(`QueenPosition.prototype.toString = function () {
  return \`\${this.rowIndex},\${this.columnIndex}\`;
};`);
  expect(js).toContain("return `at ${position}`;");
  expect(js).not.toContain("queenPositionDisplay");
  expect(js).toContain("return joined(positions, { fmt: String });");
  expect(ts).toContain("    toString(): string;\n");
  const { QueenPosition, shown, text, all } = await import(join(dir, "lib.js"));
  const position = new QueenPosition(2, 1);
  expect([position.toString(), String(position), `${position}`]).toEqual(["2,1", "2,1", "2,1"]);
  expect([shown(position), text(position), all([position, new QueenPosition(0, 3)])]).toEqual(["at 2,1", "2,1", "2,1 0,3"]);
});
