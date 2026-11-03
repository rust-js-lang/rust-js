// A constructor the program writes is held as a typed `js::Class`,
// `Class::of(f)` (ADR 1009): React's createSyntheticEvent makes a
// constructor function per interface, closed over it, and its plugins
// construct the one they hold, `new SyntheticEventCtor(reactName, ..)`.
// Each test is a program of its own, compiled, and its JS run.

import { beforeAll, expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";

import { runSync } from "./child";
import { buildReact, compiler, fixture, root, target } from "./support";

beforeAll(buildReact, 600_000);

const withJs = ["--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];

/** `source` compiled, with `--declarations`, beside the JS modules `files` its bindings import: its JS, its `.d.ts`, and the module it is, loaded. */
async function compiled(
  name: string,
  source: string,
  files: Record<string, string> = {},
): Promise<{ js: string; dts: string; lib: any }> {
  const dir = fixture(name);
  writeFileSync(join(dir, "lib.rs"), source);
  for (const [file, text] of Object.entries(files)) writeFileSync(join(dir, file), text);
  const built = runSync([compiler, "--declarations", join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withJs], root, 600_000);
  expect(built.code, built.stderr).toBe(0);
  return {
    js: readFileSync(join(dir, "lib.js"), "utf8"),
    dts: readFileSync(join(dir, "lib.d.ts"), "utf8"),
    lib: await import(join(dir, "lib.js")),
  };
}

/** What rust-js says of `source`, which it refuses. */
function refused(name: string, source: string): string {
  const dir = fixture(name);
  writeFileSync(join(dir, "lib.rs"), source);
  const result = Bun.spawnSync([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withJs], { cwd: dir });
  expect(result.exitCode).not.toBe(0);
  return result.stderr.toString();
}

const factory = `use std::cell::Cell;

use js::object::assign;
use js::{Class, Unknown, function};

fn function_that_returns_true() -> bool {
    true
}

fn function_that_returns_false() -> bool {
    false
}

pub struct BaseSyntheticEvent {
    #[cfg_attr(rust_js, rust_js::name = "_reactName")]
    pub react_name: Cell<Option<&'static str>>,
    pub r#type: Cell<&'static str>,
    pub native_event: Cell<&'static Unknown>,
    pub is_default_prevented: Cell<fn() -> bool>,
}

unsafe impl js::Defined for BaseSyntheticEvent {}

pub type SyntheticEventClass = Class<fn(Option<&'static str>, &'static str, &'static Unknown) -> BaseSyntheticEvent>;

struct SyntheticEventPrototype {
    prevent_default: &'static dyn Fn(),
    is_persistent: fn() -> bool,
}

unsafe impl js::Defined for SyntheticEventPrototype {}

// This is intentionally a factory so that we have different returned constructors.
pub fn create_synthetic_event(
    #[cfg_attr(rust_js, rust_js::name = "Interface")] interface: &'static Unknown,
) -> &'static SyntheticEventClass {
    // Synthetic events are dispatched by event plugins.
    #[cfg_attr(rust_js, rust_js::name = "SyntheticBaseEvent")]
    let synthetic_base_event = Class::of(function::with_this(
        move |this, (react_name, react_event_type, native_event): (Option<&'static str>, &'static str, &'static Unknown)| {
            // Safety: \`new\` gives its constructor the event it makes.
            let this: &'static BaseSyntheticEvent = unsafe { js::cast(this) };
            this.react_name.set(react_name);
            this.r#type.set(react_event_type);
            this.native_event.set(native_event);
            for prop_name in js::keys_in(interface) {
                js::set(this, &prop_name, js::get(native_event, &prop_name));
            }
            this.is_default_prevented.set(function_that_returns_false);
            this
        },
    ));
    assign(
        synthetic_base_event.prototype(),
        (&SyntheticEventPrototype {
            prevent_default: function::with_this(|this, ()| {
                // Safety: it's called on an event.
                let this: &'static BaseSyntheticEvent = unsafe { js::cast(this) };
                this.is_default_prevented.set(function_that_returns_true);
            }),
            is_persistent: function_that_returns_true,
        },),
    );
    synthetic_base_event
}

pub fn dispatched(ctor: &'static SyntheticEventClass, native_event: &'static Unknown) -> (&'static str, bool) {
    let event = js::new(ctor, (Some("onClick"), "click", native_event));
    (event.r#type.get(), (event.is_default_prevented.get())())
}

pub fn held(native_event: &'static Unknown) -> &'static BaseSyntheticEvent {
    #[cfg_attr(rust_js, rust_js::name = "SyntheticEvent")]
    let synthetic_event = create_synthetic_event(native_event);
    let made = synthetic_event;
    js::new(made, (None, "change", native_event))
}
`;

// createSyntheticEvent as upstream writes it: a named constructor function
// closed over its interface, its prototype assigned, the constructor given
// back; each call's its own, of its own prototype; constructed from Rust
// by the class it holds, rustc checking each argument.
test("a factory makes a constructor function closed over what it's given", async () => {
  const { js, lib } = await compiled("constructor-factory", factory);
  expect(js).toContain(`export function createSyntheticEvent(Interface) {
  // Synthetic events are dispatched by event plugins.
  function SyntheticBaseEvent(reactName, reactEventType, nativeEvent) {
    // Safety: \`new\` gives its constructor the event it makes.
    this._reactName = reactName;
    this.type = reactEventType;
    this.nativeEvent = nativeEvent;
    for (const propName in Interface) {
      this[propName] = nativeEvent[propName];
    }
    this.isDefaultPrevented = functionThatReturnsFalse;
    return this;
  }
  Object.assign(SyntheticBaseEvent.prototype, {
    preventDefault: function () {
      // Safety: it's called on an event.
      this.isDefaultPrevented = functionThatReturnsTrue;
    },
    isPersistent: functionThatReturnsTrue,
  });
  return SyntheticBaseEvent;
}`);
  expect(js).toContain('return new SyntheticEvent(undefined, "change", nativeEvent);');
  const UIEvent = lib.createSyntheticEvent({ view: 0, detail: 0 });
  const KeyboardEvent = lib.createSyntheticEvent({ key: 0 });
  expect([UIEvent.name, UIEvent === KeyboardEvent, UIEvent.prototype === KeyboardEvent.prototype]).toEqual([
    "SyntheticBaseEvent",
    false,
    false,
  ]);
  const event = new UIEvent("onScroll", "scroll", { view: "window", detail: 2, other: 3 });
  expect(Object.keys(event)).toEqual(["_reactName", "type", "nativeEvent", "view", "detail", "isDefaultPrevented"]);
  expect([event instanceof UIEvent, event instanceof KeyboardEvent, event.view, event.detail]).toEqual([true, false, "window", 2]);
  expect([event.isDefaultPrevented(), event.isPersistent()]).toEqual([false, true]);
  event.preventDefault();
  expect(event.isDefaultPrevented()).toBe(true);
  expect(Object.hasOwn(event, "preventDefault")).toBe(false);
  expect(lib.dispatched(KeyboardEvent, { key: "a" })).toEqual(["click", false]);
  const held = lib.held({});
  expect([held.type, held._reactName, held.constructor.name]).toEqual(["change", undefined, "SyntheticBaseEvent"]);
});

// React's `export const SyntheticEvent = createSyntheticEvent(EventInterface);`:
// a `pub` thread-local made as its module loads (ADR 1006) of a factory
// whose constructor calls a fn pointer; making the constructor runs none of
// it, which only `new` does.
test("a pub thread-local is made of a factory of constructors as its module loads", async () => {
  const { js, lib } = await compiled(
    "constructor-exported",
    `use js::{Class, Unknown, function};

pub type Made = Class<fn(&'static Unknown) -> Unknown>;

fn twice(value: f64) -> f64 {
    value * 2.0
}

fn create(normalize: fn(f64) -> f64) -> &'static Made {
    #[cfg_attr(rust_js, rust_js::name = "SyntheticBaseEvent")]
    let synthetic_base_event = Class::of(function::with_this(move |this, (native_event,): (&'static Unknown,)| {
        let value: f64 = unsafe { js::cast(js::get(native_event, "value")) };
        js::set(this, "value", normalize(value));
    }));
    synthetic_base_event
}

thread_local! {
    #[cfg_attr(rust_js, rust_js::name = "SyntheticEvent")]
    pub static SYNTHETIC_EVENT: &'static Made = create(twice);
}
`,
  );
  expect(js).toContain("export const SyntheticEvent = create(twice);");
  expect(new lib.SyntheticEvent({ value: 2 }).value).toBe(4);
});

// A typed class is declared as its constructor's type, which TypeScript's
// `new` checks as rustc does.
test("a typed class is declared as its constructor's type", () => {
  const dir = fixture("constructor-declared");
  writeFileSync(
    join(dir, "lib.rs"),
    `pub fn made(ctor: &'static js::Class<fn(&'static str, f64) -> js::Unknown>) -> &'static js::Unknown {
    js::new(ctor, ("a", 1.0))
}
`,
  );
  const built = runSync([compiler, "--declarations", join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withJs], root, 600_000);
  expect(built.code, built.stderr).toBe(0);
  expect(readFileSync(join(dir, "lib.d.ts"), "utf8")).toContain(
    "export function made(ctor: new (value: string, value2: number) => unknown): unknown;",
  );
});

// ADR 1027, amended: a class held, an untyped `Class`, a binding's or the
// crate's, is made by its constructor's binding, `call_static`, the one way;
// rustc refuses `js::new` of it, naming that.
test("new of a class held is refused, naming its constructor's binding", () => {
  const error = refused(
    "held-class-new",
    `use js::{Class, Unknown};

pub fn made(class: &'static Class) -> &'static Unknown {
    js::new(class, ())
}
`,
  );
  expect(error).toContain("`js::Class` isn't a constructor `new` is given `()` of");
  expect(error).toContain("a class held, an untyped `&Class`, is made by its constructor's binding, `class.call_static(webapi::abort_controller::new, ())`");
});

// rustc checks what `new` of a typed class is given, as a call's arguments.
test("new of a typed class is given what its constructor takes", () => {
  const error = refused(
    "constructor-typed-arguments",
    `use js::{Class, Unknown};

pub fn made(ctor: &'static Class<fn(&'static str, f64) -> Unknown>) -> &'static Unknown {
    js::new(ctor, (1.0, "a"))
}
`,
  );
  expect(error).toContain("expected `&str`, found floating-point number");
  expect(error).toContain("expected `f64`, found `&str`");
});

// A constructor the program writes is held as a `Class`, the one way: its
// value of any shape, constructed or its prototype read, is refused,
// naming `Class::of`; a `Class` held as any value, naming it given as it is.
test("a constructor held as a value of any shape is refused", () => {
  const constructed = refused(
    "constructor-as-unknown",
    `use js::Unknown;

pub fn made() -> &'static Unknown {
    #[cfg_attr(rust_js, rust_js::name = "Fake")]
    fn fake() {}
    js::new(js::unknown(&fake), ())
}
`,
  );
  expect(constructed).toContain("a constructor is held as a `js::Class`, `Class::of(&f)`");
  const prototype = refused(
    "constructor-prototype-as-unknown",
    `use js::Unknown;

pub fn prototype() -> Option<&'static Unknown> {
    #[cfg_attr(rust_js, rust_js::name = "Fake")]
    fn fake() {}
    js::get(js::unknown(&fake), "prototype")
}
`,
  );
  expect(prototype).toContain("`Class::of(&f).prototype()`");
  const ofClass = refused(
    "class-as-unknown",
    `use js::Unknown;

pub fn made(class: &'static js::Class) -> &'static Unknown {
    js::new(js::unknown(class), ())
}
`,
  );
  expect(ofClass).toContain("a class held, an untyped `js::Class`, is made by its constructor's binding, `class.call_static(webapi::abort_controller::new, ())`");
  const typed = refused(
    "typed-class-as-unknown",
    `use js::Unknown;

pub fn made(class: &'static js::Class<fn() -> Unknown>) -> &'static Unknown {
    js::new(js::unknown(class), ())
}
`,
  );
  expect(typed).toContain("`js::new(class, ..)`");
  const arrow = refused(
    "class-of-a-closure",
    `use js::{Class, Unknown};

pub fn made() -> &'static Unknown {
    let make = || {};
    js::new(Class::of(&make), ())
}
`,
  );
  expect(arrow).toContain("a closure is an arrow function, which `new` throws of");
});

// ADR 0861, amended (React port gap rd64d): a `function::with_this`
// closure's tuple of parameters takes `#[rust_js::nullable(..)]`, naming the
// bindings that are `T | null`, as a function's own does: React's
// SyntheticBaseEvent stores `reactName`, `targetInst` and `nativeEventTarget`
// as they're given, `this._reactName = reactName;`, not `?? null`; one not
// named is `?? null` still, and one set `None` in the body is `null`.
test("a with_this closure's nullable parameters are stored as they're given", async () => {
  const { js, lib } = await compiled(
    "constructor-nullable-parameters",
    `use std::cell::Cell;

use js::{Class, Unknown, function};

pub struct Event {
    #[cfg_attr(rust_js, rust_js::name = "_reactName", rust_js::nullable)]
    pub react_name: Cell<Option<&'static str>>,
    pub r#type: Cell<&'static str>,
    #[cfg_attr(rust_js, rust_js::nullable)]
    pub target: Cell<Option<&'static Unknown>>,
    #[cfg_attr(rust_js, rust_js::nullable)]
    pub current_target: Cell<Option<&'static Unknown>>,
    #[cfg_attr(rust_js, rust_js::nullable)]
    pub related: Cell<Option<&'static Unknown>>,
}

unsafe impl js::Defined for Event {}

pub type EventClass = Class<
    fn(Option<&'static str>, &'static str, Option<&'static Unknown>, Option<&'static Unknown>, Option<&'static Unknown>) -> Event,
>;

thread_local! {
    #[cfg_attr(rust_js, rust_js::name = "SyntheticEvent")]
    pub static SYNTHETIC_EVENT: &'static EventClass = Class::of(function::with_this(
        |this,
         #[cfg_attr(rust_js, rust_js::nullable(react_name, native_event_target, current))] (
            react_name,
            react_event_type,
            native_event_target,
            mut current,
            related,
        ): (
            Option<&'static str>,
            &'static str,
            Option<&'static Unknown>,
            Option<&'static Unknown>,
            Option<&'static Unknown>,
        )| {
            // Safety: \`new\` gives its constructor the event it makes.
            let this: &'static Event = unsafe { js::cast(this) };
            this.react_name.set(react_name);
            this.r#type.set(react_event_type);
            this.target.set(native_event_target);
            if react_event_type == "none" {
                current = None;
            }
            this.current_target.set(current);
            this.related.set(related);
            this
        },
    ));
}

thread_local! {
    pub static UNMARKED: &'static Class<fn(Option<&'static str>) -> Event> = Class::of(function::with_this(
        |this, (react_name,): (Option<&'static str>,)| {
            // Safety: \`new\` gives its constructor the event it makes.
            let this: &'static Event = unsafe { js::cast(this) };
            this.react_name.set(react_name);
            this
        },
    ));
}
`,
  );
  expect(js).toContain("  this._reactName = reactName;\n");
  expect(js).toContain("  this.target = nativeEventTarget;\n");
  expect(js).toContain("    current = null;\n");
  expect(js).toContain("  this.currentTarget = current;\n");
  expect(js).toContain("  this.related = related ?? null;\n");
  expect(js).toContain("  this._reactName = reactName ?? null;\n");
  const event = new lib.SyntheticEvent(null, "none", null, 3, undefined);
  expect([event._reactName, event.type, event.target, event.currentTarget, event.related]).toEqual([null, "none", null, null, null]);
  const named = new lib.SyntheticEvent("onClick", "click", 1, 2);
  expect([named._reactName, named.target, named.currentTarget]).toEqual(["onClick", 1, 2]);
});

// React's SyntheticBaseEvent stores its target as any value, `this.target.
// set(native_event_target.map(js::unknown))` of a nullable `Option<&
// EventTarget>`: `js::unknown` is its argument in JS, so the map is what it
// maps, `null` or a value already, stored as given, `this.target =
// nativeEventTarget;`, where it was `?? null` (the port's gap rd94). A map
// of another function is stored as its branches are, `: null`.
test("an identity map of a nullable parameter is stored as it's given", async () => {
  const { js, lib } = await compiled(
    "constructor-nullable-identity-map",
    `use std::cell::Cell;

use js::{Class, Unknown, function};

pub struct Event {
    #[cfg_attr(rust_js, rust_js::nullable)]
    pub target: Cell<Option<&'static Unknown>>,
    #[cfg_attr(rust_js, rust_js::nullable)]
    pub other: Cell<Option<&'static Unknown>>,
}

unsafe impl js::Defined for Event {}

fn given(target: &'static str) -> &'static Unknown {
    js::unknown(target)
}

thread_local! {
    pub static SYNTHETIC_EVENT: &'static Class<fn(Option<&'static str>) -> Event> = Class::of(function::with_this(
        |this, #[cfg_attr(rust_js, rust_js::nullable(native_event_target))] (native_event_target,): (Option<&'static str>,)| {
            // Safety: \`new\` gives its constructor the event it makes.
            let this: &'static Event = unsafe { js::cast(this) };
            this.target.set(native_event_target.map(js::unknown));
            this.other.set(native_event_target.map(given));
            this
        },
    ));
}
`,
  );
  expect(js).toContain("  this.target = nativeEventTarget;\n");
  expect(js).toContain("  this.other = nativeEventTarget != null ? given(nativeEventTarget) : null;\n");
  const event = new lib.syntheticEvent(null);
  expect([event.target, event.other]).toEqual([null, null]);
  const named = new lib.syntheticEvent("div");
  expect([named.target, named.other]).toEqual(["div", "div"]);
});

// rd628: a nullable cell field each constructor of its struct sets first,
// before anything is given its `this` or reads the field, is present, `T |
// null`: React's Flight client's `prepareDestinationForModule(
// response._moduleLoading, ..)` reads it as it is, where it was `?? null`.
// One read before it's set, set in an `if`, after its `this` is given, cast
// or not, or after a `return`, only by one of two constructors, or never,
// may be absent, so it's still `?? null`.
test("a nullable field each constructor sets first is present, given on as it is", async () => {
  const { js, dts, lib } = await compiled(
    "constructor-set-first",
    `use std::cell::Cell;

use js::{Class, Unknown, function};

pub struct Response {
    #[cfg_attr(rust_js, rust_js::nullable)]
    pub module_loading: Cell<Option<&'static Unknown>>,
    #[cfg_attr(rust_js, rust_js::nullable)]
    pub nonce: Cell<Option<&'static Unknown>>,
    #[cfg_attr(rust_js, rust_js::nullable, rust_js::optional)]
    pub task: Cell<Option<&'static Unknown>>,
    #[cfg_attr(rust_js, rust_js::nullable)]
    pub copy: Cell<Option<&'static Unknown>>,
    #[cfg_attr(rust_js, rust_js::nullable)]
    pub read_first: Cell<Option<&'static Unknown>>,
    #[cfg_attr(rust_js, rust_js::nullable)]
    pub in_if: Cell<Option<&'static Unknown>>,
    #[cfg_attr(rust_js, rust_js::nullable)]
    pub after_given: Cell<Option<&'static Unknown>>,
    #[cfg_attr(rust_js, rust_js::nullable)]
    pub never: Cell<Option<&'static Unknown>>,
}

unsafe impl js::Defined for Response {}

pub struct Twice {
    #[cfg_attr(rust_js, rust_js::nullable)]
    pub both: Cell<Option<&'static Unknown>>,
    #[cfg_attr(rust_js, rust_js::nullable)]
    pub one: Cell<Option<&'static Unknown>>,
    #[cfg_attr(rust_js, rust_js::nullable)]
    pub late: Cell<Option<&'static Unknown>>,
}

unsafe impl js::Defined for Twice {}

pub struct Early {
    #[cfg_attr(rust_js, rust_js::nullable)]
    pub value: Cell<Option<&'static Unknown>>,
}

unsafe impl js::Defined for Early {}

pub struct Held {
    #[cfg_attr(rust_js, rust_js::nullable)]
    pub held: Cell<Option<&'static Unknown>>,
}

unsafe impl js::Defined for Held {}

#[cfg_attr(rust_js, rust_js::link_name = "./config.js#prepare", rust_js::nullable(module_loading))]
fn prepare(module_loading: Option<&'static Unknown>) -> &'static Unknown {
    let _ = module_loading;
    unreachable!()
}

#[cfg_attr(rust_js, rust_js::link_name = "./config.js#register")]
fn register(response: &'static Response) {
    let _ = response;
    unreachable!()
}

#[cfg_attr(rust_js, rust_js::link_name = "./config.js#keep")]
fn keep(value: Option<&'static Unknown>) {
    let _ = value;
    unreachable!()
}

type Made = fn(Option<&'static Unknown>, bool) -> Response;

thread_local! {
    #[cfg_attr(rust_js, rust_js::name = "Response")]
    pub static RESPONSE: &'static Class<Made> = Class::of(function::with_this(
        |this, (module_loading, dev): (Option<&'static Unknown>, bool)| {
            // Safety: \`new\` gives its constructor the object it makes.
            let response: &'static Response = unsafe { js::cast(this) };
            let chunks = js::unknown("chunks");
            response.module_loading.set(module_loading);
            response.nonce.set(Some(chunks));
            response.task.set(None);
            response.copy.set(response.read_first.get());
            response.read_first.set(module_loading);
            if dev {
                response.in_if.set(module_loading);
            }
            register(response);
            response.after_given.set(module_loading);
            response
        },
    ));
    #[cfg_attr(rust_js, rust_js::name = "Twice")]
    pub static TWICE: &'static Class<fn(Option<&'static Unknown>) -> Twice> = Class::of(function::with_this(
        |this, (value,): (Option<&'static Unknown>,)| {
            // Safety: \`new\` gives its constructor the object it makes.
            let twice: &'static Twice = unsafe { js::cast(this) };
            twice.both.set(value);
            twice.one.set(value);
            keep(this);
            twice.late.set(value);
            twice
        },
    ));
    #[cfg_attr(rust_js, rust_js::name = "Once")]
    pub static ONCE: &'static Class<fn(Option<&'static Unknown>) -> Twice> = Class::of(function::with_this(
        |this, (value,): (Option<&'static Unknown>,)| {
            // Safety: \`new\` gives its constructor the object it makes.
            let twice: &'static Twice = unsafe { js::cast(this) };
            twice.both.set(value);
            twice.late.set(value);
            twice
        },
    ));
    #[cfg_attr(rust_js, rust_js::name = "Early")]
    pub static EARLY: &'static Class<fn(Option<&'static Early>) -> Early> = Class::of(function::with_this(
        |this, (other,): (Option<&'static Early>,)| {
            // Safety: \`new\` gives its constructor the object it makes.
            let early: &'static Early = unsafe { js::cast(this) };
            if let Some(other) = other {
                return other;
            }
            early.value.set(None);
            early
        },
    ));
    #[cfg_attr(rust_js, rust_js::name = "Held")]
    pub static HELD: &'static Class<fn(bool) -> Held> = Class::of(function::with_this(|this, (now,): (bool,)| {
        // Safety: \`new\` gives its constructor the object it makes.
        let held: &'static Held = unsafe { js::cast(this) };
        let given = move || keep(this);
        held.held.set(None);
        if now {
            given();
        }
        held
    }));
}

pub fn task(response: &Response) -> &'static Unknown {
    prepare(response.task.get())
}

pub fn held(held: &Held) -> &'static Unknown {
    prepare(held.held.get())
}

pub fn copy_present(response: &Response, twice: &Twice) {
    twice.both.set(response.module_loading.get());
}

pub fn copy_absent(response: &Response, twice: &Twice) {
    twice.both.set(response.read_first.get());
}

pub fn module_loading(response: &Response) -> &'static Unknown {
    prepare(response.module_loading.get())
}

pub fn nonce(response: &Response) -> &'static Unknown {
    prepare(response.nonce.get())
}

pub fn copy(response: &Response) -> &'static Unknown {
    prepare(response.copy.get())
}

pub fn read_first(response: &Response) -> &'static Unknown {
    prepare(response.read_first.get())
}

pub fn in_if(response: &Response) -> &'static Unknown {
    prepare(response.in_if.get())
}

pub fn after_given(response: &Response) -> &'static Unknown {
    prepare(response.after_given.get())
}

pub fn never(response: &Response) -> &'static Unknown {
    prepare(response.never.get())
}

pub fn both(twice: &Twice) -> &'static Unknown {
    prepare(twice.both.get())
}

pub fn one(twice: &Twice) -> &'static Unknown {
    prepare(twice.one.get())
}

pub fn late(twice: &Twice) -> &'static Unknown {
    prepare(twice.late.get())
}

pub fn value(early: &Early) -> &'static Unknown {
    prepare(early.value.get())
}

// A field set first is tested \`=== null\`; one set in an \`if\` may be
// absent, \`== null\` (rd647).
pub fn has_in_if(response: &Response) -> bool {
    response.in_if.get().is_some()
}

pub fn has_module_loading(response: &Response) -> bool {
    response.module_loading.get().is_some()
}
`,
    {
      "config.js":
        "export const prepare = (m) => (m === null ? 'null' : m === undefined ? 'undefined' : m);\n" +
        "export const register = () => {};\n" +
        "export const keep = () => {};\n",
    },
  );
  for (const read of ["response.moduleLoading", "response.nonce", "response.copy", "twice.both"]) {
    expect(js).toContain(`  return prepare(${read});\n`);
  }
  for (const read of ["task", "readFirst", "inIf", "afterGiven", "never"]) {
    expect(js).toContain(`  return prepare(response.${read} ?? null);\n`);
  }
  // Another nullable cell is given it as it is.
  expect(js).toContain("  twice.both = response.moduleLoading;\n");
  expect(js).toContain("  twice.both = response.readFirst ?? null;\n");
  for (const read of ["twice.one", "twice.late", "early.value", "held.held"]) {
    expect(js).toContain(`  return prepare(${read} ?? null);\n`);
  }
  expect(js).toContain("  return response.inIf != null;\n");
  expect(js).toContain("  return response.moduleLoading !== null;\n");
  // Rust gives each `None` as `null`, absent or not.
  const response = new lib.Response(undefined, false);
  expect([lib.hasInIf(response), lib.hasModuleLoading(response)]).toEqual([false, false]);
  const reads = [lib.moduleLoading, lib.copy, lib.readFirst, lib.inIf, lib.afterGiven, lib.never];
  expect(reads.map((f) => f(response))).toEqual(Array(6).fill("null"));
  expect(lib.nonce(response)).toBe("chunks");
  const once = new lib.Once(undefined);
  expect([lib.both(once), lib.one(once), lib.late(once)]).toEqual(["null", "null", "null"]);
  expect(lib.value(new lib.Early({}))).toBe("null");
  // Its `.d.ts` says so: present, `T | null`.
  for (const field of ["moduleLoading", "nonce", "copy", "both"]) {
    expect(dts).toContain(`  ${field}: unknown | null;\n`);
  }
  for (const field of ["task", "readFirst", "inIf", "afterGiven", "never", "one", "late", "value", "held"]) {
    expect(dts).toContain(`  ${field}?: unknown | null | undefined;\n`);
  }
});

// What a closure's `#[rust_js::nullable(..)]` names is a binding of its own
// parameter that's an `Option`; on a function's parameter it's the
// function's, `#[rust_js::nullable(name)]` on the function.
test("a closure's nullable parameter names its own Option bindings", () => {
  const error = refused(
    "closure-nullable-not-an-option",
    `pub fn made() -> &'static js::Class<fn(&'static str) -> js::Unknown> {
    js::Class::of(js::function::with_this(
        |_, #[cfg_attr(rust_js, rust_js::nullable(name, other))] (name,): (&'static str,)| {
            let _ = name;
        },
    ))
}
`,
  );
  expect(error).toContain("`#[rust_js::nullable(name)]` names no binding of this parameter that's an `Option`");
  expect(error).toContain("`#[rust_js::nullable(other)]` names no binding of this parameter that's an `Option`");
  const onFunction = refused(
    "fn-param-nullable",
    `pub fn given(#[cfg_attr(rust_js, rust_js::nullable)] value: Option<&'static str>) -> Option<&'static str> {
    value
}
`,
  );
  expect(onFunction).toContain("a function's parameter is `#[rust_js::nullable(value)]` on the function");
});

// React port gap rd91: React DOM's SyntheticBaseEvent as the port writes it,
// a `move` closure in its factory whose bindings MIR makes of the tuple's
// items, and `*this.target_inst.borrow_mut() = target_inst` of a nullable
// `RefCell<Option<Rc<Fiber>>>`: its parameters are the function's, `(name,
// inst)`, each stored as it's given, where they were `(_3, _3$2)`, the
// tuple made again and each `?? null`; and a function's own nullable
// parameter stored through a borrow so too.
test("a with_this closure's nullable bindings are its parameters, stored through a borrow as given", async () => {
  const { js, lib } = await compiled(
    "constructor-nullable-borrowed",
    `use std::cell::{Cell, RefCell};
use std::rc::Rc;

use js::{Class, function};

pub struct Fiber {
    pub tag: Cell<u8>,
}

pub struct Made {
    #[cfg_attr(rust_js, rust_js::nullable)]
    pub name: Cell<Option<&'static str>>,
    #[cfg_attr(rust_js, rust_js::nullable)]
    pub inst: RefCell<Option<Rc<Fiber>>>,
}

pub type MadeClass = Class<fn(Option<&'static str>, Option<Rc<Fiber>>) -> Made>;

pub fn make() -> &'static MadeClass {
    #[cfg_attr(rust_js, rust_js::name = "Made")]
    let made = Class::of(function::with_this(
        move |this,
              #[cfg_attr(rust_js, rust_js::nullable(name, inst))] (name, inst): (
            Option<&'static str>,
            Option<Rc<Fiber>>,
        )| {
            // Safety: \`new\` gives its constructor the event it makes.
            let this: &'static Made = unsafe { js::cast(this) };
            this.name.set(name);
            *this.inst.borrow_mut() = inst;
            this
        },
    ));
    made
}

#[cfg_attr(rust_js, rust_js::nullable(inst))]
pub fn set(made: &Made, inst: Option<Rc<Fiber>>) {
    *made.inst.borrow_mut() = inst;
}
`,
  );
  expect(js).toContain(`  function Made(name, inst) {
    // Safety: \`new\` gives its constructor the event it makes.
    this.name = name;
    this.inst = inst;
    return this;
  }`);
  expect(js).toContain("  made.inst = inst;\n");
  const Made = lib.make();
  const made = new Made(null, { tag: 1 });
  expect([made.name, made.inst]).toEqual([null, { tag: 1 }]);
  lib.set(made, null);
  expect(made.inst).toBe(null);
});

// rd173: `new Error(message, { cause })`, as React DOM's
// customizeViewTransitionError wraps what the browser threw: the options
// are an `Option`, left out where they're `None` (ADR 0443), and `cause` of
// any value, a `&dyn IntoUnknown`, given as it is (ADR 0654).
test("an Error is made of its message and options, its cause", async () => {
  const { js, lib } = await compiled("error-cause", `use js::{Error, ErrorOptions, Unknown};

pub fn wrapped(message: &str, error: &Unknown) -> &'static Error {
    js::error::new(Some(message), Some(&ErrorOptions { cause: Some(error) }))
}

pub fn plain(message: &str) -> &'static Error {
    js::error::new(Some(message), None)
}
`);
  expect(js).toContain("  return new Error(message, { cause: error });\n");
  expect(js).toContain("  return new Error(message);\n");
  const thrown = new TypeError("no");
  const error = lib.wrapped("view transition", thrown);
  expect([error.message, error.cause]).toEqual(["view transition", thrown]);
  expect("cause" in lib.plain("plain")).toBe(false);
});

// rd400: a function's arity is one list of up to 16 parameters (ADR 1044):
// React's Flight client's ResponseInstance takes 14, createResponse's, so
// its `Class::of(function::with_this(..))` and `new` of it take them, each
// checked as a call's is.
test("a constructor function takes as many parameters as a function", async () => {
  const names = ["a", "b", "c", "d", "e", "f", "g", "h", "i", "j", "k", "l", "m", "n"];
  const typed = names.map(() => "f64").join(", ");
  const { js, lib } = await compiled(
    "constructor-fourteen-parameters",
    `use std::cell::Cell;

use js::{Class, function};

pub struct Response {
    pub sum: Cell<f64>,
}

unsafe impl js::Defined for Response {}

pub type ResponseClass = Class<fn(${typed}) -> Response>;

pub fn create() -> &'static ResponseClass {
    #[cfg_attr(rust_js, rust_js::name = "ResponseInstance")]
    let response_instance = Class::of(function::with_this(
        |this, (${names.join(", ")}): (${typed})| {
            // Safety: \`new\` gives its constructor the response it makes.
            let this: &'static Response = unsafe { js::cast(this) };
            this.sum.set(${names.map((n, i) => `${n} * ${i + 1}.0`).join(" + ")});
            this
        },
    ));
    response_instance
}

pub fn made(ctor: &'static ResponseClass) -> f64 {
    js::new(ctor, (${names.map((_, i) => `${i + 1}.0`).join(", ")})).sum.get()
}
`,
  );
  expect(js).toContain(`function ResponseInstance(${names.join(", ")}) {`);
  expect(js).toContain(`new ctor(${names.map((_, i) => i + 1).join(", ")})`);
  const expected = names.reduce((sum, _, i) => sum + (i + 1) * (i + 1), 0);
  expect(lib.made(lib.create())).toBe(expected);
  expect(new (lib.create())(...names.map(() => 1)).sum).toBe(names.reduce((sum, _, i) => sum + i + 1, 0));
});

// #449 (P0): a constructor's write through a `RefCell` field of its
// `this`, in a block, `*this._held.borrow_mut() = Some(..)`, as React's
// Flight client's ResponseInstance sets `this._weakResponse` in DEV, sets
// the instance's field, `this._held = ..`. A handle on a place of `this`
// that stays one, `&mut` lent, `bump(&mut this._count.borrow_mut())`,
// reads it through `const this$ = this;`: its object's getter and setter
// read their own `this`, that object, and the instance never had it.
test("a constructor's write through its this's RefCell field sets the instance's", async () => {
  const { js, lib } = await compiled("constructor-this-cell-write", `use std::cell::{Cell, RefCell};
use std::rc::Rc;

use js::function;

pub struct Holder {
    pub n: f64,
    pub weak: &'static js::WeakRef<Instance>,
    #[cfg_attr(rust_js, rust_js::nullable)]
    pub me: Cell<Option<&'static Instance>>,
}

unsafe impl js::Defined for Holder {}

pub struct Instance {
    #[cfg_attr(rust_js, rust_js::optional)]
    pub _held: RefCell<Option<Rc<Holder>>>,
    pub _name: RefCell<Option<String>>,
    pub _count: RefCell<f64>,
}

fn bump(count: &mut f64) {
    *count += 1.0;
}

unsafe impl js::Defined for Instance {}
unsafe impl js::WeakKey for Instance {}

type Constructor = fn(f64, bool) -> Instance;

thread_local! {
    static INSTANCE: &'static js::Class<Constructor> =
        js::Class::of(function::with_this(move |this, (n, dev): (f64, bool)| {
            // SAFETY: \`new\` gives the constructor the object it makes.
            let this: &'static Instance = unsafe { js::cast(this) };
            if dev {
                *this._held.borrow_mut() = Some(Rc::new(Holder {
                    n,
                    weak: js::weak_ref::new(this),
                    me: Cell::new(Some(this)),
                }));
                let name = format!("n{n}");
                *this._name.borrow_mut() = Some(name);
                this._count.replace(n);
                bump(&mut this._count.borrow_mut());
            }
            this
        }));
}

pub fn held(n: f64) -> f64 {
    let instance: &'static Instance = INSTANCE.with(|&class| js::new(class, (n, true)));
    instance._held.borrow().as_ref().unwrap().n
}

pub fn count(n: f64) -> f64 {
    let instance: &'static Instance = INSTANCE.with(|&class| js::new(class, (n, true)));
    *instance._count.borrow()
}

pub fn name(n: f64) -> String {
    let instance: &'static Instance = INSTANCE.with(|&class| js::new(class, (n, true)));
    instance._name.borrow().clone().unwrap()
}
`);
  expect(js).toContain("  if (dev) {\n    this._held = {\n");
  expect(js).toContain("    this._name = name;\n");
  expect(js).toContain("function instance(n, dev) {\n  const this$ = this;\n");
  expect(js).toContain("      get value() {\n        return this$._count;\n");
  expect(js).not.toMatch(/get value\(\) \{\n\s*return this\./);
  expect([lib.held(2), lib.name(3), lib.count(4)]).toEqual([2, "n3", 5]);
});

// ADR 1049: a typed array's class, and a `DataView`'s, is typed by one
// view's constructor, `Class<fn(&'static ArrayBuffer, f64, f64) ->
// ArrayBufferView>`, so one function takes any of them and `new` makes
// the view, as React's Flight client's `resolveTypedArray(.., Int8Array,
// 1, ..)` does.
test("a typed array's class is one view's constructor, which new makes the view of", async () => {
  const { js, lib } = await compiled(
    "typed-array-class",
    `use js::{ArrayBuffer, ArrayBufferView, Class, Uint8Array};

type ViewConstructor = Class<fn(&'static ArrayBuffer, f64, f64) -> ArrayBufferView>;

fn view_of(chunk: &'static Uint8Array, constructor: &'static ViewConstructor, bytes_per_element: f64) -> &'static ArrayBufferView {
    js::new(constructor, (chunk.buffer(), chunk.byte_offset(), chunk.byte_length() / bytes_per_element))
}

pub fn views(chunk: &'static Uint8Array) -> Vec<&'static ArrayBufferView> {
    vec![
        view_of(chunk, js::class::<js::Int16Array>(), 2.0),
        view_of(chunk, js::class::<js::DataView>(), 1.0),
    ]
}
`,
  );
  expect(js).toContain("return new constructor(chunk.buffer, chunk.byteOffset, chunk.byteLength / bytesPerElement);");
  expect(js).toContain("[viewOf(chunk, Int16Array, 2), viewOf(chunk, DataView, 1)]");
  const [int16, view] = lib.views(new Uint8Array([1, 0, 2, 0]));
  expect(int16).toBeInstanceOf(Int16Array);
  expect([...int16]).toEqual([1, 2]);
  expect(view).toBeInstanceOf(DataView);
  expect(view.byteLength).toBe(4);
});
