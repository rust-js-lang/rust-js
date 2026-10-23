// JS's object reflection, as the js crate binds `Object` (ADR 0666), and
// `console` as a value, its functions read as values (ADR 0667): the
// shapes React's `shared/ReactSerializationErrors.js` and
// `shared/ConsolePatchingDev.js` are written in. Each test is a program of
// its own, compiled and run.

import { beforeAll, expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";

import { runSync } from "./child";
import { buildReact, compiler, fixture, root, target } from "./support";

beforeAll(buildReact, 600_000);

const withJs = ["--", "--extern", `js=${join(target, "libjs.rmeta")}`, "--extern", `webapi=${join(target, "libwebapi.rmeta")}`, "-L", target];

/** `source` compiled, its JS, the module it is, loaded, and what rustc
 * warned of, a line each. */
async function compiled(name: string, source: string): Promise<{ js: string; lib: any; warnings: string[] }> {
  const dir = fixture(name);
  writeFileSync(join(dir, "lib.rs"), source);
  const built = runSync([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withJs], root, 600_000);
  expect(built.code, built.stderr).toBe(0);
  const warnings = built.stderr.split("\n").filter((line) => line.startsWith("warning:") && !/warnings? emitted/.test(line));
  return { js: readFileSync(join(dir, "lib.js"), "utf8"), lib: await import(join(dir, "lib.js")), warnings };
}

/** What rust-js says of `source`, which it refuses. */
function refused(name: string, source: string): string {
  const dir = fixture(name);
  writeFileSync(join(dir, "lib.rs"), source);
  const result = Bun.spawnSync([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withJs], { cwd: dir });
  expect(result.exitCode).not.toBe(0);
  return result.stderr.toString();
}

// ADR 0666: each of TypeScript's `ObjectConstructor` statics, typed as it
// types them, a descriptor a dictionary: a struct of `Option`s, each `None`
// left out, one descriptor's update of another a spread.
test("Object's statics are JS's, a property descriptor a dictionary", async () => {
  const { js, lib } = await compiled("js-object", `use js::{PropertyDescriptor, Unknown, dict, object};
pub fn defined(o: &Unknown) -> (bool, bool, Vec<String>, Vec<String>, String) {
    let o = object::define_property(o, "id", &PropertyDescriptor { value: Some(&"fixed"), writable: Some(false), ..Default::default() });
    let hidden = PropertyDescriptor { value: Some(&1), ..Default::default() };
    let shown = PropertyDescriptor { enumerable: Some(true), ..hidden };
    object::define_properties(o, dict::from_entries(vec![("hidden".to_string(), hidden), ("shown".to_string(), shown)]));
    object::define_property(o, "got", &PropertyDescriptor { get: Some(&|| Some(js::unknown("read"))), ..Default::default() });
    let read_only = object::get_own_property_descriptor(o, "id").is_some_and(|d| d.writable == Some(false));
    let getter = object::get_own_property_descriptor(o, "got").is_some_and(|d| d.get.is_some());
    let all = object::get_own_property_descriptors(o);
    (read_only, getter, object::keys(o), dict::keys(all), js::string(js::get(o, "got")))
}
pub fn sealed(o: &Unknown) -> (bool, bool, bool, bool, bool) {
    let before = object::is_extensible(o);
    object::prevent_extensions(o);
    let after = object::is_extensible(o);
    object::seal(o);
    let sealed = object::is_sealed(o);
    object::freeze(o);
    (before, after, sealed, object::is_frozen(o), object::is_frozen(object::prototype))
}
pub fn made() -> (bool, bool, bool, Vec<u32>) {
    let bare = object::create(None, None);
    let n = PropertyDescriptor { value: Some(&2), enumerable: Some(true), ..Default::default() };
    let made = object::create(Some(object::prototype), Some(dict::from_entries(vec![("n".to_string(), n)])));
    let moved = object::set_prototype_of(bare, Some(made));
    let counts = dict::from_entries(vec![("a".to_string(), 1u32), ("b".to_string(), 2)]);
    (object::get_prototype_of(bare) == Some(made), object::has_own(made, "n"), js::has(moved, "n") && !object::has_own(moved, "n"), dict::values(counts))
}
pub fn grouped(words: Vec<String>) -> Vec<(String, Vec<String>)> {
    dict::entries(object::group_by(words, |word, _| if word.starts_with('a') { "a".to_string() } else { "other".to_string() }))
}
pub fn symbols(o: &Unknown) -> usize {
    object::get_own_property_symbols(o).len()
}
`);
  expect(js).toContain('o = Object.defineProperty(o, "id", { value: "fixed", writable: false });');
  expect(js).toContain("const hidden = { value: 1 };");
  expect(js).toContain("const shown = { ...hidden, enumerable: true };");
  // A dictionary of descriptors written out is the object of them.
  expect(js).toContain("Object.defineProperties(o, { hidden, shown });");
  expect(js).toContain('Object.defineProperty(o, "got", { get: () => "read" });');
  expect(js).toContain("const bare = Object.create(null);");
  // A nullable parameter's `Some(..)` is its value (ADR 0275).
  expect(js).toContain("Object.create(Object.prototype, ");
  expect(js).toContain("Object.setPrototypeOf(bare, made");
  expect(js).toContain("Object.isFrozen(Object.prototype)");
  expect(js).toContain("Object.groupBy(words, (word) =>");
  expect(lib.defined({})).toEqual([true, true, ["shown"], ["id", "hidden", "shown", "got"], "read"]);
  expect(lib.sealed({ a: 1 })).toEqual([true, false, true, true, false]);
  expect(lib.made()).toEqual([true, true, true, [1, 2]]);
  expect(lib.grouped(["ant", "bee", "asp"])).toEqual([["a", ["ant", "asp"]], ["other", ["bee"]]]);
  expect(lib.symbols({ [Symbol("s")]: 1, t: 2 })).toBe(1);
});

// ADR 0666: React's `isObjectPrototype`, `isGetter` and `objectName`, of
// `Object.prototype` as a value, `Object.getOwnPropertyNames`, `in`, and a
// descriptor's getter.
test("React's serialization errors read an object as upstream does", async () => {
  const { js, lib } = await compiled("js-object-react-serialization", `use js::{Unknown, object};
pub fn is_object_prototype(object: Option<&Unknown>) -> bool {
    let Some(object) = object else {
        return false;
    };
    let object_prototype = object::prototype;
    if object == object_prototype {
        return true;
    }
    // It might be an object from a different Realm which is
    // still just a plain simple object.
    if object::get_prototype_of(object).is_some() {
        return false;
    }
    let names = object::get_own_property_names(object);
    for name in &names {
        if !js::has(object_prototype, name) {
            return false;
        }
    }
    true
}
pub fn is_getter(object: Option<&Unknown>, name: &str) -> bool {
    let object_prototype = object::prototype;
    let Some(object) = object else {
        return false;
    };
    if object == object_prototype {
        return false;
    }
    match object::get_own_property_descriptor(object, name) {
        None => is_getter(object::get_prototype_of(object), name),
        Some(descriptor) => descriptor.get.is_some(),
    }
}
pub fn object_name(object: Option<&Unknown>) -> String {
    let name = object::to_string(object).unwrap();
    // Extract 'Object' from '[object Object]':
    name[8..name.len() - 1].to_string()
}
`);
  expect(js).toContain(`  const objectPrototype = Object.prototype;
  if (object === objectPrototype) {
    return true;
  }
  // It might be an object from a different Realm which is
  // still just a plain simple object.
  if (Object.getPrototypeOf(object) != null) {
    return false;
  }
  const names = Object.getOwnPropertyNames(object);
  for (const name of names) {
    if (!(name in objectPrototype)) {
      return false;
    }
  }
  return true;`);
  expect(js).toContain(`  const descriptor = Object.getOwnPropertyDescriptor(object, name);
  if (!descriptor) {
    return isGetter(Object.getPrototypeOf(object), name);
  }
  return !!descriptor.get;`);
  expect(js).toContain("const name = Object.prototype.toString.call(object);");
  expect([lib.isObjectPrototype(Object.prototype), lib.isObjectPrototype({}), lib.isObjectPrototype(undefined)]).toEqual([true, false, false]);
  // Another realm's `Object.prototype`, of no prototype and only its names.
  expect(lib.isObjectPrototype(Object.create(null, { toString: { value: 1 } }))).toBe(true);
  expect(lib.isObjectPrototype(Object.create(null, { other: { value: 1 } }))).toBe(false);
  const props = Object.defineProperty({ a: 1 }, "key", { get: () => 1 });
  expect([lib.isGetter(props, "key"), lib.isGetter(props, "a"), lib.isGetter(Object.create(props), "key"), lib.isGetter({}, "toString")]).toEqual([true, false, true, false]);
  expect([lib.objectName([]), lib.objectName(new Map()), lib.objectName({})]).toEqual(["Array", "Map", "Object"]);
});

// ADR 0667: `console` is a value, `webapi::console`, and its functions are
// read as values as any binding is (ADR 0602): a variadic one taken as a
// `fn` of rest arguments, `&dyn js::Rest<T>`, is its JS function itself,
// which a call spreads. React's `disableLogs` and `reenableLogs`. Its
// `Logs`, a struct given `Object.defineProperties` as a `&dyn
// PropertyDescriptorMap`, has each field read by JS, as rust-js's liveness
// tells rustc (ADR 0440): none is warned of as never read.
test("console is a value, its functions read as values", async () => {
  const methods = ["log", "info", "warn", "error", "group", "group_collapsed", "group_end"];
  const camel = (m: string) => m.replace(/_(\w)/g, (_, c) => c.toUpperCase());
  const upper = (m: string) => m.toUpperCase();
  const field = (m: string, value: string) => `                ${m}: PropertyDescriptor { value: Some(&PREV_${upper(m)}.get()), ..${value} },`;
  const { js, lib, warnings } = await compiled("js-object-react-console", `use std::cell::Cell;

use js::{IntoUnknown, PropertyDescriptor, PropertyDescriptorMap, Rest, object};
use webapi::console;

type ConsoleFn = fn(&'static dyn Rest<dyn IntoUnknown>);

thread_local! {
    static DISABLED_DEPTH: Cell<i32> = const { Cell::new(0) };
${methods.map((m) => `    static PREV_${upper(m)}: Cell<Option<${m === "group_end" ? "fn()" : "ConsoleFn"}>> = const { Cell::new(None) };`).join("\n")}
}

fn disabled_log() {}
js::on_load! {
    js::set(js::unknown(&disabled_log), "__reactDisabledLog", true);
}

struct Logs<'a> {
${["info", "log", "warn", "error", "group", "group_collapsed", "group_end"].map((m) => `    ${m}: PropertyDescriptor<'a>,`).join("\n")}
}

unsafe impl PropertyDescriptorMap for Logs<'_> {}

pub fn disable_logs() {
    if DISABLED_DEPTH.get() == 0 {
${methods.map((m) => `        PREV_${upper(m)}.set(Some(console::${m}));`).join("\n")}
        // https://github.com/react/react/issues/19099
        let props = PropertyDescriptor {
            configurable: Some(true),
            enumerable: Some(true),
            value: Some(&disabled_log),
            writable: Some(true),
            ..Default::default()
        };
        object::define_properties(
            webapi::console,
            &Logs {
${["info", "log", "warn", "error", "group", "group_collapsed", "group_end"].map((m) => `                ${m}: props,`).join("\n")}
            },
        );
    }
    DISABLED_DEPTH.set(DISABLED_DEPTH.get() + 1);
}

pub fn reenable_logs() {
    DISABLED_DEPTH.set(DISABLED_DEPTH.get() - 1);
    if DISABLED_DEPTH.get() == 0 {
        let props = PropertyDescriptor {
            configurable: Some(true),
            enumerable: Some(true),
            writable: Some(true),
            ..Default::default()
        };
        object::define_properties(
            webapi::console,
            &Logs {
${methods.map((m) => field(m, "props")).join("\n")}
            },
        );
    }
    if DISABLED_DEPTH.get() < 0 {
        console::error((concat!("disabledDepth fell below zero. ", "This is a bug in React. Please file an issue."),));
    }
}

pub fn called(name: &'static str) {
    let log: ConsoleFn = console::log;
    log(&("named", 1));
    console::warn(&(name,) as &dyn Rest<dyn IntoUnknown>);
}
`);
  expect(warnings).toEqual([]);
  expect(js).toContain("let prevLog;\nlet prevInfo;");
  expect(js).toContain("function disabledLog() {}\ndisabledLog.__reactDisabledLog = true;\n\nexport function disableLogs() {");
  expect(js).toContain("    prevLog = console.log;\n    prevInfo = console.info;");
  expect(js).toContain("prevGroupCollapsed = console.groupCollapsed;");
  expect(js).toContain("const props = { configurable: true, enumerable: true, value: disabledLog, writable: true };");
  expect(js).toContain("Object.defineProperties(console, {\n      info: props,\n      log: props,");
  expect(js).toContain("const props = { configurable: true, enumerable: true, writable: true };");
  // Each object's keys as written, upstream's: `log` first here (ADR 0674).
  expect(js).toContain("Object.defineProperties(console, {\n      log: { ...props, value: prevLog },\n      info: { ...props, value: prevInfo },");
  expect(js).toContain("groupCollapsed: { ...props, value: prevGroupCollapsed },");
  // A `fn` of rest arguments is called with them spread.
  expect(js).toContain('  const log = console.log;\n  log("named", 1);\n  console.warn(name);');
  const saved = Object.fromEntries(methods.map((m) => [camel(m), (console as any)[camel(m)]]));
  try {
    lib.disableLogs();
    lib.disableLogs();
    expect((console.log as any).name).toBe("disabledLog");
    // What React DevTools reads to tell React's stand-in from the console's.
    expect((console.log as any).__reactDisabledLog).toBe(true);
    expect(console.groupEnd).toBe(console.info);
    lib.reenableLogs();
    expect((console.warn as any).name).toBe("disabledLog");
    lib.reenableLogs();
    for (const m of methods) expect((console as any)[camel(m)]).toBe(saved[camel(m)]);
  } finally {
    Object.assign(console, saved);
  }
});

// ADR 0673: a function is a JS value of any shape, `js::unknown(&f)`,
// whose property `js::set` writes, as React's `ConsolePatchingDev` marks
// its `disabledLog` when its module loads, right after it, for DevTools to
// read; a function item, a closure, a `fn` pointer and a `dyn Fn` alike.
test("a function's property is set where its module loads it", async () => {
  const { js, lib } = await compiled("js-object-function-property", `use js::{IntoUnknown, PropertyDescriptor, Rest, object};
use webapi::console;

type ConsoleFn = fn(&'static dyn Rest<dyn IntoUnknown>);

fn disabled_log() {}
js::on_load! {
    js::set(js::unknown(&disabled_log), "__reactDisabledLog", true);
}

pub fn disable_log() {
    let props = PropertyDescriptor {
        configurable: Some(true),
        value: Some(&disabled_log),
        writable: Some(true),
        ..Default::default()
    };
    object::define_property(webapi::console, "log", &props);
}

pub fn is_disabled() -> bool {
    let log: ConsoleFn = console::log;
    js::get(js::unknown(&log), "__reactDisabledLog").is_some()
}

pub fn marked(by: &str) -> Vec<String> {
    let add = |n: f64| n + 1.0;
    let pointer: fn(f64) -> f64 = add;
    let held: &dyn Fn(f64) -> f64 = &add;
    js::set(js::unknown(&add), "by", by);
    js::set(js::unknown(&pointer), "kind", "pointer");
    js::set(js::unknown(held), "held", true);
    object::keys(js::unknown_of(add))
}
`);
  expect(js).toContain('function disabledLog() {}\ndisabledLog.__reactDisabledLog = true;\n');
  expect(js).toContain("const log = console.log;\n  return log.__reactDisabledLog != null;");
  expect(js).toContain('add.by = by;\n  pointer.kind = "pointer";\n  held.held = true;');
  const log = console.log;
  try {
    expect(lib.isDisabled()).toBe(false);
    lib.disableLog();
    // What React DevTools reads of the console's `log`.
    expect((console.log as any).__reactDisabledLog).toBe(true);
    expect(lib.isDisabled()).toBe(true);
  } finally {
    console.log = log;
  }
  expect(lib.marked("me")).toEqual(["by", "kind", "held"]);
});

// ADR 0667: rest arguments as a value are a binding's, which a `fn`
// pointer's call spreads; a Rust function or closure that takes them, or a
// `dyn Fn` of them, whose call gives them as one array, is refused.
test("rest arguments as a value are refused where a call wouldn't spread them", () => {
  expect(refused("js-object-rest-fn", `use js::{IntoUnknown, Rest};
pub fn mine(data: &'static dyn Rest<dyn IntoUnknown>) {
    webapi::console::log(data);
}
`)).toContain("rust-js does not support a `&dyn js::Rest<T>` parameter, rest arguments as a value, of a Rust function or closure yet");
  expect(refused("js-object-rest-dyn", `use js::{IntoUnknown, Rest};
pub fn held() {
    let log: &dyn Fn(&'static dyn Rest<dyn IntoUnknown>) = &webapi::console::log;
    log(&("a", 1));
}
`)).toContain("rust-js does not support values of type `dyn std::ops::Fn(&dyn js::Rest<dyn js::IntoUnknown>)` yet");
});

// ADR 0695: JS's `new` of a function value is the js crate's, of a value
// of any shape, as TypeScript has it of an `any`'s: `js::new(js::unknown(f),
// (a, b))` is `new f(a, b)`, its arguments a rest parameter's. A closure is
// an arrow, which `new` throws of, a panic.
test("new of a function value is JS's new", async () => {
  const { js, lib, warnings } = await compiled("js-object-new", `use js::{IntoUnknown, Unknown};

pub fn made(class: &Unknown, a: f64, rest: &[&dyn IntoUnknown]) -> (&'static Unknown, &'static Unknown, &'static Unknown) {
    (js::new(class, ()), js::new(class, (a, "b")), js::new(class, rest))
}

pub fn arrow() -> bool {
    let f = || 1;
    std::panic::catch_unwind(|| js::new(js::unknown(&f), ())).is_err()
}
`);
  expect(warnings).toEqual([]);
  expect(js).toContain('return [new class$(), new class$(a, "b"), new class$(...rest)];');
  class Thing {
    args: unknown[];
    constructor(...args: unknown[]) {
      this.args = args;
    }
  }
  const [none, two, spread] = lib.made(Thing, 1, ["x", 2]);
  expect([none.args, two.args, spread.args]).toEqual([[], [1, "b"], ["x", 2]]);
  expect(none).toBeInstanceOf(Thing);
  expect(lib.arrow()).toBe(true);
});

// ADR 0696: a constructor Rust writes is a `fn` item, a function
// declaration, constructed and with a `prototype`: where it's written, in
// the innermost block that uses it, after its comments.
test("a constructor is a fn item, declared where it's written", async () => {
  const { js, lib, warnings } = await compiled("js-object-constructor", `use js::Unknown;

pub fn point(x: f64) -> (bool, f64) {
    #[cfg_attr(rust_js, rust_js::name = "Point")]
    fn point() -> Option<&'static Unknown> {
        None
    }
    if x > 0.0 {
        // Where a positive one starts.
        #[cfg_attr(rust_js, rust_js::name = "Origin")]
        fn origin() -> Option<&'static Unknown> {
            None
        }
        let prototype: &Unknown = unsafe { js::cast(js::get(js::unknown(&origin), "prototype")) };
        js::set(prototype, "x", x);
        let made = js::new(js::unknown(&origin), ());
        return (js::get(made, "constructor") == Some(js::unknown(&origin)), unsafe { js::cast(js::get(made, "x")) });
    }
    (js::get(js::new(js::unknown(&point), ()), "constructor") == Some(js::unknown(&point)), 0.0)
}
`);
  expect(warnings).toEqual([]);
  expect(js).toContain(
    "export function point(x) {\n  function Point() {}\n  if (x > 0) {\n    // Where a positive one starts.\n    function Origin() {}\n    const prototype = Origin.prototype;",
  );
  expect(js).toContain("const made = new Origin();");
  expect(lib.point(3)).toEqual([true, 3]);
  expect(lib.point(-1)).toEqual([true, 0]);
});

// ADR 0697: JS's `delete` is the js crate's, of a value of any shape, as
// TypeScript has it of an `any`'s: `js::delete(o, "p")` is `delete o.p`, a
// statement where nothing reads whether it's gone.
test("delete is JS's operator", async () => {
  const { js, lib, warnings } = await compiled("js-object-delete", `use js::Unknown;

pub fn deleted(o: &Unknown, key: &str) -> (bool, bool) {
    js::delete(o, "props");
    let gone = js::delete(o, key);
    (gone, js::has(o, "props"))
}

pub fn had(o: &Unknown) -> (bool, bool) {
    let gone = js::delete(o, "a");
    let has = js::has(o, "a");
    (has, gone)
}
`);
  expect(warnings).toEqual([]);
  expect(js).toContain("delete o.props;\n  const gone = delete o[key];");
  // Before what's after it, as it's written: it changes what that reads.
  expect(lib.had({ a: 1 })).toEqual([false, true]);
  const o: any = { props: 1, kept: 2, gone: 3 };
  expect(lib.deleted(o, "gone")).toEqual([true, false]);
  expect(o).toEqual({ kept: 2 });
  expect(() => lib.deleted(Object.freeze({ props: 1 }), "x")).toThrow(TypeError);
});

// ADR 0694: TypeScript's `Function` is `&dyn Fn() -> Option<&'static
// Unknown>`, one form everywhere: called of nothing, what it gives is any
// value, as `js::Kind::Function` holds it and `Reflect` takes it.
test("TypeScript's Function gives any value", async () => {
  const { js, lib, warnings } = await compiled("js-object-function", `use js::{Kind, Unknown, reflect};

pub fn called(value: Option<&'static Unknown>) -> Option<&'static Unknown> {
    match js::classify(value) {
        Kind::Function(f) => f(),
        _ => None,
    }
}

pub fn applied(f: &dyn Fn() -> Option<&'static Unknown>, this: &Unknown) -> Option<&'static Unknown> {
    reflect::apply(f, Some(this), &[Some(js::unknown("a"))])
}

pub fn given() -> Option<&'static Unknown> {
    fn one() -> Option<&'static Unknown> {
        Some(js::unknown(&1))
    }
    called(Some(js::unknown(&one)))
}
`);
  expect(warnings).toEqual([]);
  expect(js).toContain("return value();");
  expect(js).toContain('return Reflect.apply(f, this$, ["a"]);');
  expect(lib.called(() => "made")).toBe("made");
  expect(lib.called(() => {})).toBeUndefined();
  expect(lib.called("text")).toBeUndefined();
  expect(lib.applied(function (this: any, a: string) { return this.n + a; }, { n: "n" })).toBe("na");
  expect(lib.given()).toBe(1);
});

// ADRs 0694 to 0699: React's `shared/ReactComponentStackFrame.js` as a port writes
// it, each shape upstream's: `Fake` a function declaration where it's
// written, `new fn()`, `delete fn.prototype.props`, `fn()` for what it
// gives, `maybePromise.catch(() => {})`, and its cache a `WeakMap` or, of a
// host without one, a `Map`. Run of a function component, a class, and an
// async component whose rejection is silenced.
test("React's component stack frame is found as upstream finds it", async () => {
  const { js, lib, warnings } = await compiled("js-object-component-frame", `use js::{JsError, JsObject, Kind, Promise, PropertyDescriptor, Unknown, WeakMap, object, reflect};
use std::panic::{AssertUnwindSafe, catch_unwind};

thread_local! {
    static COMPONENT_FRAME_CACHE: &'static WeakMap<dyn Fn() -> Option<&'static Unknown>, String> =
        if js::is_defined(js::weak_map::new::<JsObject, String>) {
            js::weak_map::new()
        } else {
            js::map::new()
        };
}

/// What the \`finally\` without \`Reflect.construct\` puts back.
struct RestoreProps {
    prototype: &'static Unknown,
    prev_props: Option<PropertyDescriptor<'static>>,
}

impl Drop for RestoreProps {
    fn drop(&mut self) {
        match &self.prev_props {
            Some(prev_props) => {
                object::define_property(self.prototype, "props", prev_props);
            }
            None => {
                js::delete(self.prototype, "props");
            }
        }
    }
}

#[cfg_attr(rust_js, rust_js::name = "DetermineComponentFrameRoot")]
pub fn determine_component_frame_root(
    r#fn: &dyn Fn() -> Option<&'static Unknown>,
    construct: bool,
    reflected: bool,
) -> (Option<String>, Option<String>) {
    let mut control: Option<&'static JsError> = None;
    let tried = catch_unwind(AssertUnwindSafe(|| {
        // This should throw.
        if construct {
            // Something should be setting the props in the constructor.
            #[cfg_attr(rust_js, rust_js::name = "Fake")]
            fn fake() -> Option<&'static Unknown> {
                panic!();
            }
            object::define_property(
                unsafe { js::cast::<&Unknown>(js::get(js::unknown(&fake), "prototype")) },
                "props",
                &PropertyDescriptor {
                    set: Some(&|_| panic!()),
                    ..Default::default()
                },
            );
            if reflected && js::is_defined(reflect::construct) {
                if let Err(x) = catch_unwind(|| reflect::construct(&fake, &[], None)) {
                    control = x.downcast_ref::<&'static JsError>().copied();
                }
                reflect::construct(r#fn, &[], Some(&fake));
            } else {
                if let Err(x) = catch_unwind(fake) {
                    control = x.downcast_ref::<&'static JsError>().copied();
                }
                let prototype: &'static Unknown = unsafe { js::cast(js::get(js::unknown(r#fn), "prototype")) };
                let prev_props = object::get_own_property_descriptor(prototype, "props");
                object::define_property(
                    prototype,
                    "props",
                    &PropertyDescriptor {
                        configurable: Some(true),
                        set: Some(&|_| panic!()),
                        ..Default::default()
                    },
                );
                let _restore_props = RestoreProps { prototype, prev_props };
                js::new(js::unknown(r#fn), ());
            }
        } else {
            control = Some(js::js_error::new(""));
            let maybe_promise = r#fn();
            // If the function component returns a promise, it's likely an async
            // component, which we don't yet support. Attach a noop catch handler to
            // silence the error.
            if let Some(maybe_promise) = maybe_promise
                && let Kind::Function(_) = js::classify(js::get(maybe_promise, "catch"))
            {
                unsafe { js::cast::<Promise<Option<&'static Unknown>>>(Some(maybe_promise)) }.catch(|_| {});
            }
        }
    }));
    if let Err(sample) = tried
        && let Some(sample) = sample.downcast_ref::<&'static JsError>()
        && let Some(control) = control
        && let Some(sample_stack) = sample.stack()
    {
        return (Some(sample_stack), control.stack());
    }
    (None, None)
}

pub fn cached(r#fn: &dyn Fn() -> Option<&'static Unknown>, frame: String) -> Option<String> {
    COMPONENT_FRAME_CACHE.with(|cache| {
        let before = cache.get(r#fn);
        cache.set(r#fn, frame);
        before
    })
}
`);
  expect(warnings).toEqual([]);
  expect(js).toContain('const componentFrameCache = typeof WeakMap !== "undefined" ? new WeakMap() : new Map();');
  expect(js).toContain(
    '    if (construct) {\n      // Something should be setting the props in the constructor.\n      function Fake() {\n        throw new Error("explicit panic");\n      }\n      Object.defineProperty(Fake.prototype, "props", {',
  );
  expect(js).toContain("Reflect.construct(Fake, []);");
  expect(js).toContain("Reflect.construct(fn, [], Fake);");
  expect(js).toContain("new fn();");
  expect(js).toContain('delete _restoreProps.prototype.props;');
  expect(js).toContain("const maybePromise = fn();");
  expect(js).toContain("maybePromise.catch(() => {});");
  // A class's constructor sets its props, which the setter throws of:
  // constructed as `Fake`, by `Reflect.construct`, or as itself, by `new`.
  const seen: string[] = [];
  class Component {
    constructor() {
      seen.push(new.target.name);
      (this as any).props = {};
      seen.push("set");
    }
  }
  function FunctionComponent(): never {
    throw Error();
  }
  // The sample is the setter's throw, in the class's frame; the control
  // `Fake`'s, each under `DetermineComponentFrameRoot`.
  for (const reflected of [true, false]) {
    const [sample, control] = lib.DetermineComponentFrameRoot(Component, true, reflected);
    expect(sample).toContain("at new Component");
    expect(sample).toContain("DetermineComponentFrameRoot");
    expect(control).toContain("Fake");
    expect(control).not.toContain("Component (");
  }
  expect(seen).toEqual(["Fake", "Component"]);
  expect(Object.hasOwn(Component.prototype, "props")).toBe(false);
  const [sample, control] = lib.DetermineComponentFrameRoot(FunctionComponent, false, true);
  expect(sample).toContain("FunctionComponent");
  expect(control).not.toContain("FunctionComponent");
  let unhandled = 0;
  const count = () => unhandled++;
  process.on("unhandledRejection", count);
  try {
    expect(lib.DetermineComponentFrameRoot(async () => { throw new Error("async"); }, false, true)).toEqual([undefined, undefined]);
    await new Promise((resolve) => setTimeout(resolve, 10));
  } finally {
    process.off("unhandledRejection", count);
  }
  expect(unhandled).toBe(0);
  expect([lib.cached(FunctionComponent, "a"), lib.cached(FunctionComponent, "b")]).toEqual([undefined, "a"]);
});
