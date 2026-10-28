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
    prevent_default: Box<dyn Fn()>,
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
  expect(ofClass).toContain("`js::new(class, ..)`");
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
