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
  // Across lines, as its literal is, and React's (ADR 0835).
  expect(js).toContain("const props = {\n      configurable: true,\n      enumerable: true,\n      value: disabledLog,\n      writable: true,\n    };");
  expect(js).toContain("Object.defineProperties(console, {\n      info: props,\n      log: props,");
  expect(js).toContain("const props = {\n      configurable: true,\n      enumerable: true,\n      writable: true,\n    };");
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
// pointer's call spreads; a Rust function that takes them, whose `fn`
// pointer would be given only the first, is refused: its rest is its last
// slice marked `#[rust_js::rest]`. A `dyn Fn` of them is called spread, as
// a closure's own type is (ADR 0851).
test("rest arguments as a value are refused where a call wouldn't spread them", async () => {
  expect(refused("js-object-rest-fn", `use js::{IntoUnknown, Rest};
pub fn mine(data: &'static dyn Rest<dyn IntoUnknown>) {
    webapi::console::log(data);
}
`)).toContain("rust-js does not support a `&dyn js::Rest<T>` parameter, rest arguments as a value, of a Rust function yet: a function's rest parameter is its last slice marked `#[rust_js::rest]`");
  const { js } = await compiled("js-object-rest-dyn", `use js::{IntoUnknown, Rest};
pub fn held() {
    let log: &dyn Fn(&'static dyn Rest<dyn IntoUnknown>) = &webapi::console::log;
    log(&("a", 1));
}
`);
  expect(js).toContain('log("a", 1);');
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

// ADR 0776: `js::get`, `js::has` and `js::set` take any value, as
// TypeScript's `any` is, `None` too: `value.key`, `key in value` and
// `value.key = to`, which throw a TypeError of `undefined` and `null` as
// JS's do, as React's `'default' in moduleObject` relies on, a panic.
test("get, has and set take any value, and throw of nothing as JS does", async () => {
  const { js, lib, warnings } = await compiled("js-object-any-get", `use js::{JsObject, Unknown};

pub fn default_of(module_object: Option<&Unknown>) -> Option<&'static Unknown> {
    js::get(module_object, "default")
}

pub fn has_default(module_object: Option<&Unknown>) -> bool {
    js::has(module_object, "default")
}

pub fn name(render: Option<&Unknown>, name: &str) {
    js::set(render, "displayName", name);
}

pub fn present(o: &Unknown, object: &JsObject) -> (bool, bool) {
    js::set(o, "seen", true);
    (js::has(o, "seen"), js::get(object, "k").is_some())
}

pub fn caught(module_object: Option<&'static Unknown>) -> bool {
    std::panic::catch_unwind(|| js::has(module_object, "default")).is_err()
}
`);
  expect(warnings).toEqual([]);
  expect(js).toContain("return moduleObject.default;");
  expect(js).toContain('return "default" in moduleObject;');
  expect(js).toContain("render.displayName = name;");
  expect(js).toContain("o.seen = true;");
  const m = { default: 1 };
  expect([lib.defaultOf(m), lib.hasDefault(m), lib.hasDefault({})]).toEqual([1, true, false]);
  expect(() => lib.defaultOf(undefined)).toThrow(TypeError);
  expect(() => lib.defaultOf(null)).toThrow(TypeError);
  expect(() => lib.hasDefault(undefined)).toThrow(TypeError);
  expect(() => lib.hasDefault(null)).toThrow(TypeError);
  expect(() => lib.name(undefined, "x")).toThrow(TypeError);
  const render: any = {};
  lib.name(render, "Named");
  expect(render.displayName).toBe("Named");
  expect(lib.present({}, { k: 1 })).toEqual([true, true]);
  expect([lib.caught(undefined), lib.caught(m)]).toEqual([true, false]);
});

// ADR 0700: TypeScript's `Function` is `&dyn Fn() -> Option<&'static
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

// ADR 0719: `js::keys_in(o)`, the keys `for…in` visits, its prototypes'
// enumerable ones too, each as the loop reaches it; a `for` loop of it is
// `for (const key in o)`, and of its `enumerate()` a counter each turn ends
// with: React's `shared/ReactPerformanceTrackProperties.js` as a port
// writes it. Anywhere else, a JS iterator of them.
test("a for loop of keys_in is JS's for…in, its enumerate a counter", async () => {
  const { js, lib, warnings } = await compiled("js-object-keys-in", `use js::Unknown;
use js::object::has_own_property;

const OBJECT_WIDTH_LIMIT: usize = 2;

pub fn add_object_to_properties(object: &Unknown, properties: &mut Vec<String>) {
    let mut added_properties = 0;
    for key in js::keys_in(object) {
        if has_own_property.call(object, &key) && !key.starts_with('_') {
            added_properties += 1;
            properties.push(key);
            if added_properties >= OBJECT_WIDTH_LIMIT {
                properties.push("Only 2 properties are shown.".to_string());
                break;
            }
        }
    }
}

pub fn removed(prev: &Unknown, next: &Unknown, properties: &mut Vec<String>) -> bool {
    let mut is_deeply_equal = true;
    for (prev_properties_checked, key) in js::keys_in(prev).enumerate() {
        if prev_properties_checked > OBJECT_WIDTH_LIMIT {
            properties.push("Previous object has more than 2 properties.".to_string());
            is_deeply_equal = false;
            break;
        }
        if js::has(next, &key) {
            continue;
        }
        properties.push(key);
        is_deeply_equal = false;
    }
    is_deeply_equal
}

pub fn deleting(object: &Unknown) -> Vec<String> {
    let mut seen = Vec::new();
    for key in js::keys_in(object) {
        js::delete(object, "b");
        seen.push(key);
    }
    seen
}

pub fn later(object: &Unknown) -> Vec<String> {
    let mut reads: Vec<Box<dyn Fn() -> String>> = Vec::new();
    for (n, key) in js::keys_in(object).enumerate() {
        reads.push(Box::new(move || format!("{n}:{key}")));
    }
    reads.iter().map(|read| read()).collect()
}

pub fn numbered(object: &Unknown) -> Vec<String> {
    let mut out = Vec::new();
    for (n, key) in js::keys_in(object).enumerate() {
        out.push(format!("{n}:{key}"));
    }
    out
}

pub fn twice(object: &Unknown) -> String {
    let mut last = String::new();
    for (n, key) in js::keys_in(object).enumerate() {
        last = format!("{n}{key}");
    }
    let n = last.len();
    for (n, key) in js::keys_in(object).enumerate() {
        last += &format!("{n}{key}");
    }
    format!("{last}{n}")
}

pub fn all(object: &Unknown) -> Vec<String> {
    js::keys_in(object).collect()
}
`);
  expect(warnings).toEqual([]);
  expect(js).toContain("for (const key in object) {\n    if (Object.prototype.hasOwnProperty.call(object, key) && !key.startsWith(\"_\")) {");
  expect(js).toContain("let prevPropertiesChecked = 0;\n  for (const key in prev) {");
  expect(js).toContain("prevPropertiesChecked++;\n      continue;");
  expect(js).toContain("let n = 0;\n  for (const key in object) {\n    out.push(`${n}:${key}`);\n    n++;\n  }");
  // A closure that reads the count keeps its own each turn, as a `const`.
  expect(js).toContain("for (const [n, key] of $keysIn(object).map((x, i) => [i, x])) {");
  expect(js).toContain("return $keysIn(object).toArray();");
  const proto = { inherited: 1, shadowed: 2 };
  const object = Object.assign(Object.create(proto), { own: 3, shadowed: 4, _private: 5 });
  Object.defineProperty(object, "hidden", { value: 6, enumerable: false });
  const properties: string[] = [];
  lib.addObjectToProperties(object, properties);
  expect(properties).toEqual(["own", "shadowed", "Only 2 properties are shown."]);
  expect(lib.all(object)).toEqual(["own", "shadowed", "_private", "inherited"]);
  const removed: string[] = [];
  expect(lib.removed(object, { own: 1 }, removed)).toBe(false);
  expect(removed).toEqual(["shadowed", "_private", "Previous object has more than 2 properties."]);
  const many: string[] = [];
  expect(lib.removed({ a: 1, b: 2, c: 3, d: 4 }, { a: 1, b: 2, c: 3, d: 4 }, many)).toBe(false);
  expect(many).toEqual(["Previous object has more than 2 properties."]);
  expect(lib.removed({ a: 1 }, { a: 2 }, [])).toBe(true);
  // A key deleted before it's reached isn't visited, as `for…in` has it.
  expect(lib.deleting({ a: 1, b: 2, c: 3 })).toEqual(["a", "c"]);
  expect(lib.later({ x: 1, y: 2 })).toEqual(["0:x", "1:y"]);
  expect(lib.numbered(object)).toEqual(["0:own", "1:shadowed", "2:_private", "3:inherited"]);
  // Two counters of one name aren't one \`let\`, which JS refuses.
  expect(lib.twice({ a: 1, b: 2 })).toBe("1b0a1b2");
});

// ADR 0440: a struct made a JS object, `js::Defined`, whose fields only JS
// reads, by key, as React's `ReactBaseClasses.js` reads its deprecated
// APIs: given `js::unknown`, a binding, its fields are read, and rust-js's
// check warns of none; a struct JS isn't given is warned of still.
test("a struct JS reads by key has its fields read", async () => {
  const { lib, warnings } = await compiled("js-object-read-by-key", `use js::{PropertyDescriptor, Unknown, object};
use webapi::console;

struct DeprecatedApis {
    is_mounted: (&'static str, &'static str),
    replace_state: (&'static str, &'static str),
}

unsafe impl js::Defined for DeprecatedApis {}

struct Unread {
    hidden: u32,
}

pub fn unread() -> u32 {
    let _ = Unread { hidden: 1 };
    0
}

pub fn deprecate(target: &'static Unknown) {
    let deprecated_apis = DeprecatedApis {
        is_mounted: ("isMounted", "Clean up instead."),
        replace_state: ("replaceState", "Use setState instead."),
    };
    let define_deprecation_warning = |method_name: &str, info: &'static (&'static str, &'static str)| {
        object::define_property(
            target,
            method_name,
            &PropertyDescriptor {
                get: Some(&|| {
                    console::warn(("%s(...) is deprecated. %s", info.0, info.1));
                    None
                }),
                ..Default::default()
            },
        );
    };
    for fn_name in js::keys_in(js::unknown(&deprecated_apis)) {
        if object::has_own_property.call(js::unknown(&deprecated_apis), &fn_name) {
            define_deprecation_warning(&fn_name, unsafe { js::cast(js::get(js::unknown(&deprecated_apis), &fn_name)) });
        }
    }
}
`);
  expect(warnings).toEqual(["warning: field `hidden` is never read"]);
  const warned: unknown[][] = [];
  const warn = console.warn;
  console.warn = (...args: unknown[]) => warned.push(args);
  try {
    const target = {};
    lib.deprecate(target);
    expect([(target as any).isMounted, (target as any).replaceState]).toEqual([undefined, undefined]);
  } finally {
    console.warn = warn;
  }
  expect(warned).toEqual([
    ["%s(...) is deprecated. %s", "isMounted", "Clean up instead."],
    ["%s(...) is deprecated. %s", "replaceState", "Use setState instead."],
  ]);
});

// ADR 0740: a function's `name`, `length` and source, `Function.prototype`'s
// members, of any function, as React's `shared/ReactPerformanceTrackProperties.js`
// reads them: `f.name`, `f.length`, `Function.prototype.toString.call(f)`.
test("a function's name, length and source are read of any function", async () => {
  const { js, lib, warnings } = await compiled("js-object-function-members", `use js::{Kind, Unknown, function};

pub fn described(value: Option<&Unknown>) -> String {
    match js::classify(value) {
        Kind::Function(f) => {
            let function_name = function::name(f);
            if function_name.is_empty() {
                "() => {}".to_string()
            } else {
                function_name + "() {}"
            }
        }
        _ => String::new(),
    }
}

pub fn same_source(prev_value: Option<&Unknown>, next_value: Option<&Unknown>) -> bool {
    if let Kind::Function(prev_value) = js::classify(prev_value)
        && let Kind::Function(next_value) = js::classify(next_value)
        && function::name(prev_value) == function::name(next_value)
        && function::length(prev_value) == function::length(next_value)
    {
        let prev_src = function::to_string(prev_value);
        let next_src = function::to_string(next_value);
        return prev_src == next_src;
    }
    false
}

pub fn of_signature(f: &dyn Fn(&str) -> u32) -> (String, u32) {
    (function::name(f), function::length(f))
}

pub fn own() -> (String, u32) {
    fn add(a: u32, b: u32) -> u32 {
        a + b
    }
    (function::name(&add), function::length(&add))
}
`);
  expect(warnings).toEqual([]);
  expect(js).toContain("const functionName = value.name;");
  expect(js).toContain("prevValue.name === nextValue.name &&");
  expect(js).toContain("prevValue.length === nextValue.length");
  expect(js).toContain("const prevSrc = Function.prototype.toString.call(prevValue);");
  expect(js).toContain("return [f.name, f.length];");
  expect(js).toContain("return [add.name, add.length];");
  expect(lib.described(function named() {})).toBe("named() {}");
  expect(lib.described(() => {})).toBe("() => {}");
  const made = () => (n: number) => n + 1;
  expect(lib.sameSource(made(), made())).toBe(true);
  expect(lib.sameSource(made(), (n: number) => n + 2)).toBe(false);
  expect(lib.sameSource(function a(x: number) {}, function a(x: number, y: number) {})).toBe(false);
  expect(lib.ofSignature(function measure(text: string) { return text.length; })).toEqual(["measure", 1]);
  expect(lib.own()).toEqual(["add", 2]);
});

// ADR 0848: `Function.prototype.apply`, as TypeScript types it, its `this`
// any value: `null` too, `Some(&Kind::Null)`, which a strict function sees
// apart from `undefined`, `None`; its arguments an array, none of `None`.
// React's `fn.apply(null, arguments)`.
test("a function is applied to a this of any value and an array of arguments", async () => {
  const { js, lib, warnings } = await compiled("js-object-function-apply", `use js::{Kind, Unknown, function};

type AnyFunction = dyn Fn() -> Option<&'static Unknown>;

pub fn applied(f: &AnyFunction, args: &[Option<&'static Unknown>]) -> [Option<&'static Unknown>; 3] {
    [
        function::apply(f, Some(&Kind::Null), Some(&args)),
        function::apply(f, None, None),
        function::apply(f, Some(&"self"), Some(&args)),
    ]
}
`);
  expect(warnings).toEqual([]);
  expect(js).toContain('return [f.apply(null, args), f.apply(), f.apply("self", args)];');
  // A module's function is strict: its `this` is what it's given.
  const strict = function (this: unknown, ...args: unknown[]) {
    return [this, ...args];
  };
  expect(lib.applied(strict, [1, "b"])).toEqual([[null, 1, "b"], [undefined], ["self", 1, "b"]]);
});

// ADR 0615, amended: a value's own `hasOwnProperty`, `value.hasOwnProperty(key)`,
// as React's `ReactFiberHydrationDiffs.js` asks it, which an object with no
// prototype hasn't: JS's `TypeError`, a panic, which `describeDiff`'s `catch`
// makes `""`. `Object.prototype.hasOwnProperty.call(value, key)` would
// answer `false` there and go on.
test("a value's own hasOwnProperty is called on it", async () => {
  const { js, lib, warnings } = await compiled("js-object-own-has-own-property", `use std::panic::{AssertUnwindSafe, catch_unwind};

use js::{JsObject, Unknown};

fn own_names(value: &Unknown) -> String {
    let mut properties = String::new();
    for prop_name in js::keys_in(value) {
        if !value.has_own_property(prop_name.as_str()) {
            continue;
        }
        properties += &prop_name;
    }
    properties
}

pub fn describe_diff(value: &Unknown) -> String {
    match catch_unwind(AssertUnwindSafe(|| "\\n\\n".to_owned() + &own_names(value))) {
        Ok(described) => described,
        Err(_) => String::new(),
    }
}

pub fn has_a(object: &JsObject) -> bool {
    object.has_own_property("a")
}
`);
  expect(warnings).toEqual([]);
  expect(js).toContain("    if (!value.hasOwnProperty(propName)) {\n");
  expect(js).toContain('  return object.hasOwnProperty("a");');
  const inherits = Object.create({ inherited: 1 });
  inherits.mine = 2;
  expect(lib.describeDiff(inherits)).toBe("\n\nmine");
  const bare = Object.create(null);
  bare.mine = 2;
  expect(lib.describeDiff(bare)).toBe("");
  // Its own `hasOwnProperty` answers.
  expect(lib.hasA({ hasOwnProperty: () => true })).toBe(true);
  expect(lib.hasA({ b: 1 })).toBe(false);
});

// ADR 0740, amended: `Function.prototype.bind(thisArg)`, a function of the
// same signature, as React's `ReactFiberCallUserSpace.js` keeps each frame's:
// `callComponent.react_stack_bottom_frame.bind(callComponent)`. Its `this` is
// any value, `null` too; called, it's the function it binds. Bound of no
// first arguments, `()`, it's of the function's own type (ADR 0907,
// amended).
test("a function bound to a this is a function of its signature", async () => {
  const { js, lib, warnings } = await compiled("js-object-function-bind", `use js::{Kind, function};

pub type CallAdd = fn(u32, u32) -> u32;

struct CallAddFrame {
    #[cfg_attr(rust_js, rust_js::name = "react_stack_bottom_frame")]
    react_stack_bottom_frame: CallAdd,
}

#[cfg_attr(rust_js, rust_js::name = "callAdd")]
const CALL_ADD: CallAddFrame = CallAddFrame {
    react_stack_bottom_frame: |a, b| a + b,
};

thread_local! {
    pub static CALL_ADD_IN_DEV: CallAdd =
        function::bind(CALL_ADD.react_stack_bottom_frame, Some(&CALL_ADD), ());
}

pub fn added(a: u32) -> u32 {
    CALL_ADD_IN_DEV.with(|call_add| call_add(a, 1))
}

pub fn bound(f: &dyn Fn(u32) -> u32) -> u32 {
    let to_null = function::bind(f, Some(&Kind::Null), ());
    to_null(1) + function::bind(f, None, ())(2)
}
`);
  expect(warnings).toEqual([]);
  expect(js).toContain("export const callAddInDev = callAdd.react_stack_bottom_frame.bind(callAdd);");
  expect(js).toContain("const toNull = f.bind(null);");
  expect(js).toContain("(toNull(1) + f.bind()(2)) >>> 0");
  expect(lib.added(2)).toBe(3);
  expect(lib.callAddInDev.name).toBe("bound react_stack_bottom_frame");
  // A module's function is strict: its `this` is what it's bound to.
  const seen: unknown[] = [];
  expect(lib.bound(function (this: unknown, n: number) {
    seen.push(this);
    return n * 10;
  })).toBe(30);
  expect(seen).toEqual([null, undefined]);
});

// ADRs 0695 to 0700: React's `shared/ReactComponentStackFrame.js` as a port writes
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
            js::map::new(None)
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
            control = Some(js::error::new(""));
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
  expect(js).toContain("delete prototype.props;");
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

// ADR 0909: a type parameter's value kept with others of other types, as
// React's ReactFiberStack keeps each cursor's in one `Array<any>`, is
// `js::any_of`'s, and `js::cast` gives it back: the value itself, so
// `push(cursor, value, fiber)` is upstream's, no `TypeId` given beside it.
test("a type parameter's value is any value, kept and given back as itself", async () => {
  const { js, lib } = await compiled("value-stack", `use std::cell::{Cell, RefCell};
use std::rc::Rc;

use js::Unknown;

pub struct Fiber {
    pub tag: u32,
}

pub struct StackCursor<T> {
    pub current: RefCell<T>,
}

thread_local! {
    static VALUE_STACK: RefCell<Vec<Option<&'static Unknown>>> = const { RefCell::new(Vec::new()) };
    static INDEX: Cell<i32> = const { Cell::new(-1) };
}

pub fn create_cursor<T>(default_value: T) -> StackCursor<T> {
    StackCursor {
        current: RefCell::new(default_value),
    }
}

pub fn pop<T>(cursor: &StackCursor<T>, fiber: &Fiber) {
    if INDEX.get() < 0 {
        return;
    }
    let index = INDEX.get() as usize;
    // Safety: what's at \`index\` is what \`push\` kept of this cursor's \`T\`.
    cursor.current.replace(unsafe { js::cast(VALUE_STACK.with_borrow(|value_stack| value_stack[index])) });
    VALUE_STACK.with_borrow_mut(|value_stack| value_stack[index] = None);
    INDEX.set(INDEX.get() - 1);
}

pub fn push<T: 'static>(cursor: &StackCursor<T>, value: T, fiber: &Fiber) {
    INDEX.set(INDEX.get() + 1);
    let index = INDEX.get() as usize;
    let previous = js::any_of(cursor.current.replace(value));
    VALUE_STACK.with_borrow_mut(|value_stack| {
        if index < value_stack.len() {
            value_stack[index] = previous;
        } else {
            value_stack.push(previous);
        }
    });
}

pub struct HiddenContext {
    pub base_lanes: u32,
}

thread_local! {
    static HIDDEN: StackCursor<Option<Rc<HiddenContext>>> = create_cursor(None);
    static LANES: StackCursor<u32> = create_cursor(0);
    static NESTED: StackCursor<Option<Option<u32>>> = create_cursor(None);
}

fn seen() -> String {
    let hidden = HIDDEN.with(|cursor| cursor.current.borrow().as_ref().map(|context| context.base_lanes));
    let lanes = LANES.with(|cursor| *cursor.current.borrow());
    let nested = NESTED.with(|cursor| format!("{:?}", cursor.current.borrow()));
    format!("{hidden:?} {lanes} {nested}")
}

pub fn run() -> Vec<String> {
    let fiber = Fiber { tag: 1 };
    let mut seen_each = vec![seen()];
    LANES.with(|cursor| push(cursor, 4, &fiber));
    HIDDEN.with(|cursor| push(cursor, Some(Rc::new(HiddenContext { base_lanes: 8 })), &fiber));
    NESTED.with(|cursor| push(cursor, Some(None), &fiber));
    NESTED.with(|cursor| push(cursor, Some(Some(2)), &fiber));
    seen_each.push(seen());
    NESTED.with(|cursor| pop(cursor, &fiber));
    seen_each.push(seen());
    NESTED.with(|cursor| pop(cursor, &fiber));
    HIDDEN.with(|cursor| pop(cursor, &fiber));
    seen_each.push(seen());
    LANES.with(|cursor| push(cursor, 16, &fiber));
    seen_each.push(seen());
    LANES.with(|cursor| pop(cursor, &fiber));
    LANES.with(|cursor| pop(cursor, &fiber));
    LANES.with(|cursor| pop(cursor, &fiber));
    seen_each.push(seen());
    seen_each
}
`);
  expect(js).toContain(`export function pop(cursor, fiber) {
  if (index < 0) {
    return;
  }
  const index$1 = index >>> 0;
  // Safety: what's at \`index\` is what \`push\` kept of this cursor's \`T\`.
  cursor.current = $index(valueStack, index$1);`);
  expect(js).toContain(`export function push(cursor, value, fiber) {
  index = (index + 1) | 0;
  const index$1 = index >>> 0;
  const previous = cursor.current;
  cursor.current = value;`);
  expect(js).not.toContain("TId");
  expect(js).not.toContain("impl:");
  expect(lib.run()).toEqual(["None 0 None", "Some(8) 4 Some(Some(2))", "Some(8) 4 Some(None)", "None 4 None", "None 16 None", "None 0 None"]);
  // Of a value of a type that's known, what it is: its own conversion.
  for (const [name, value] of [["number", "5u32"], ["option", "Some(\"a\")"]]) {
    const said = refused(`any-of-${name}`, `pub fn any() -> Option<&'static js::Unknown> {\n    js::any_of(${value})\n}\n`);
    expect(said).toContain("`js::any_of` is of a type parameter's value, not of a value of a type that's known: `js::unknown_of(value)` of one that's always there, `value.map(js::unknown_of)` of an `Option`");
  }
});

// ADR 0910: what `typeof value === "object" && value !== null` takes, an
// object or an array, is one value, `kind.object()`, a `&JsObject` that
// keys a weak map, as React's ReactCapturedValue keys `CapturedStacks` by
// any object it's given.
test("an object or an array is one weak key, kind.object()", async () => {
  const { js, lib } = await compiled("object-keys", `use std::rc::Rc;

use js::{JsObject, Unknown, WeakMap};

thread_local! {
    #[cfg_attr(rust_js, rust_js::name = "CapturedStacks")]
    static CAPTURED_STACKS: &'static WeakMap<JsObject, Rc<CapturedValue>> = js::weak_map::new();
}

pub struct CapturedValue {
    pub value: Option<&'static Unknown>,
    pub stack: String,
}

pub fn create_captured_value_at_fiber(value: Option<&'static Unknown>, stack: &str) -> Rc<CapturedValue> {
    // If the value is an error, call this function immediately after it is thrown
    // so the stack is accurate.
    if let Some(object) = js::classify(value).object() {
        let existing = CAPTURED_STACKS.with(|captured_stacks| captured_stacks.get(object));
        if let Some(existing) = existing {
            return existing;
        }
        let captured = Rc::new(CapturedValue {
            value,
            stack: stack.to_string(),
        });
        CAPTURED_STACKS.with(|captured_stacks| captured_stacks.set(object, Rc::clone(&captured)));
        captured
    } else {
        Rc::new(CapturedValue {
            value,
            stack: stack.to_string(),
        })
    }
}

pub fn objects(values: &[Option<&'static Unknown>]) -> Vec<bool> {
    values.iter().map(|&value| js::classify(value).object().is_some()).collect()
}
`);
  expect(js).toContain(`  // If the value is an error, call this function immediately after it is thrown
  // so the stack is accurate.
  if (typeof value === "object" && value !== null) {
    const existing = CapturedStacks.get(value);`);
  expect(js).toContain("CapturedStacks.set(value, captured);");
  // Its presence read in place is its test.
  expect(js).toContain('return values.map((value) => typeof value === "object" && value !== null);');
  const error = new Error("boom");
  const children = ["a"];
  const first = lib.createCapturedValueAtFiber(error, "at A");
  const again = lib.createCapturedValueAtFiber(error, "at B");
  const array = lib.createCapturedValueAtFiber(children, "at C");
  const arrayAgain = lib.createCapturedValueAtFiber(children, "at D");
  const text = lib.createCapturedValueAtFiber("text", "at E");
  const textAgain = lib.createCapturedValueAtFiber("text", "at F");
  expect([first === again, again.stack, array === arrayAgain, arrayAgain.stack, text === textAgain, textAgain.stack]).toEqual([true, "at A", true, "at C", false, "at F"]);
  expect(lib.objects([{}, [], null, undefined, "s", 1, () => 1, Symbol.iterator])).toEqual([true, true, false, false, false, false, false, false]);
});

// React's HotReloading reads `typeof nextType === 'object' && nextType !==
// null ? nextType.$$typeof : null`; its port, `if let Some(next_type) =
// js::classify(next_type).object() { js::get(next_type, "$$typeof") } else
// { None }`, the `Option` `kind.object()` made where it's tested: that
// test, one conditional, not `v = t ? v : undefined; const r = v?.$$typeof;`
// (ADR 0910, amended).
test("an object's property read where its test holds is one conditional", async () => {
  const { js, lib } = await compiled("object-read-where-tested", `use js::Unknown;

pub fn type_of_type(next_type: Option<&'static Unknown>) -> Option<&'static Unknown> {
    let typeof_next_type = if let Some(next_type) = js::classify(next_type).object() {
        js::get(next_type, "\$\$typeof")
    } else {
        None
    };
    typeof_next_type
}
`);
  expect(js).toContain(
    'const typeofNextType =\n    typeof nextType === "object" && nextType !== null ? nextType.$$typeof : undefined;\n',
  );
  const tag = Symbol("tag");
  expect([lib.typeOfType({ $$typeof: tag }), lib.typeOfType(null), lib.typeOfType(3), lib.typeOfType(undefined)]).toEqual([tag, undefined, undefined, undefined]);
});

// React's mountTransition stores the function it returns in its hook,
// `hook.memoizedState = start; return [false, start];`, and
// updateTransition reads it back: an `Rc<dyn Fn>` is of any shape as the
// function it points at is, `js::unknown_of(Rc::clone(&start))` the
// function itself, which `js::cast` gives back as the `Rc` (ADR 0965).
test("a shared function is of any shape, the function itself", async () => {
  const { js, lib } = await compiled("js-object-shared-function", `use std::cell::Cell;
use std::rc::Rc;

use js::Unknown;

pub struct Hook {
    pub memoized_state: Cell<Option<&'static Unknown>>,
}

pub fn mount_transition(hook: &Hook, n: Rc<Cell<f64>>) -> Rc<dyn Fn(f64)> {
    let start: Rc<dyn Fn(f64)> = Rc::new(move |x| n.set(x));
    hook.memoized_state.set(Some(js::unknown_of(Rc::clone(&start))));
    start
}

pub fn update_transition(hook: &Hook) -> Rc<dyn Fn(f64)> {
    // SAFETY: mount_transition stored an \`Rc<dyn Fn(f64)>\` there.
    unsafe { js::cast(hook.memoized_state.get()) }
}

pub fn run() -> (f64, bool, bool) {
    let n = Rc::new(Cell::new(0.0));
    let hook = Hook { memoized_state: Cell::new(None) };
    let start = mount_transition(&hook, Rc::clone(&n));
    let read = update_transition(&hook);
    read(2.0);
    start(n.get() + 1.0);
    let stored = js::strict_equals(hook.memoized_state.get(), js::unknown(&*start));
    let read_back = js::strict_equals(js::unknown(&*read), js::unknown(&*start));
    (n.get(), stored, read_back)
}
`);
  expect(js).toContain("hook.memoizedState = start;\n  return start;");
  expect(lib.run()).toEqual([3, true, true]);
});

// The user's decision (2026-10-27): `function::bind` takes a rest of `f`'s
// first arguments, upstream's `.bind(thisArg, ...args)`: React's
// `dispatchSetState.bind(null, fiber, queue)`, the `Rc`s it keeps lent to
// each call's `&Rc` parameters, and its performance track's
// `console.timeStamp.bind(console, label, start, end, track, group,
// color)`, the binding's function itself, not an arrow of it, as
// `function::name` reads it too. The bound function is a `Box<dyn Fn>` of
// the parameters left (ADR 0907, amended).
test("a function bound to its first arguments is a function of the parameters left", async () => {
  const { js, lib, warnings } = await compiled("js-object-bind-arguments", `use std::cell::Cell;
use std::rc::Rc;

use js::{Kind, function};
use webapi::IntoStrOrNumber;

pub struct Queue {
    pub n: Cell<u32>,
}

fn dispatch_set_state(fiber: &Rc<Queue>, queue: &Rc<Queue>, action: u32) {
    fiber.n.set(fiber.n.get() + action);
    queue.n.set(queue.n.get() * action);
}

pub fn mount_state(fiber: Rc<Queue>, queue: Rc<Queue>) -> Rc<dyn Fn(u32)> {
    let dispatch: Rc<dyn Fn(u32)> = Rc::from(function::bind(dispatch_set_state, Some(&Kind::Null), (fiber, queue)));
    dispatch
}

pub fn dispatched() -> (u32, u32) {
    let fiber = Rc::new(Queue { n: Cell::new(1) });
    let queue = Rc::new(Queue { n: Cell::new(2) });
    let dispatch = mount_state(Rc::clone(&fiber), Rc::clone(&queue));
    dispatch(3);
    dispatch(4);
    (fiber.n.get(), queue.n.get())
}

fn add(a: f64, b: f64) -> f64 {
    a + b
}

pub fn added(n: f64) -> f64 {
    let plus_n = function::bind(add, None, (n,));
    let both = function::bind(add, None, (n, 1.0));
    plus_n(1.0) + both()
}

pub fn stamp(label: &str, start: f64, end: f64) {
    let start: &dyn IntoStrOrNumber = &start;
    let end: &dyn IntoStrOrNumber = &end;
    let log = function::bind(
        webapi::console::time_stamp,
        Some(&webapi::console),
        (label, Some(start), Some(end), Some("Components"), None, Some("primary")),
    );
    log();
}

pub fn stamp_name() -> String {
    function::name(&webapi::console::time_stamp)
}

// A closure the program writes is the function it is, its own name "";
// a function of the crate's is its own.
pub fn closure_name() -> (String, String) {
    (function::name(&|a: f64, b: f64| add(a, b)), function::name(&add))
}

// What JS calls with what it gives: an arrow of what Rust gives.
pub fn parsed(text: js::Promise<&'static str>) -> js::Promise<f64> {
    text.then_resolve(js::parse_float)
}
`);
  expect(warnings).toEqual([]);
  expect(js).toContain("  const dispatch = dispatchSetState.bind(null, fiber, queue);\n  return dispatch;\n");
  expect(js).toContain("const plusN = add.bind(undefined, n);");
  expect(js).toContain('const log = console.timeStamp.bind(\n    console,\n    label,\n    start,\n    end,\n    "Components",\n    undefined,\n    "primary",\n  );');
  expect(js).toContain("return console.timeStamp.name;");
  expect(js).toContain("return [((a, b) => add(a, b)).name, add.name];");
  expect(js).toContain("return text.then((text) => parseFloat(text));");
  expect(lib.closureName()).toEqual(["", "add"]);
  expect(lib.dispatched()).toEqual([8, 24]);
  expect(lib.added(2)).toBe(6);
  const timeStamp = console.timeStamp;
  const seen: unknown[] = [];
  try {
    console.timeStamp = function timeStamp(this: unknown, ...args: unknown[]) {
      seen.push([this === console, ...args]);
    } as typeof console.timeStamp;
    lib.stamp("render", 1, 2);
    expect(lib.stampName()).toBe("timeStamp");
  } finally {
    console.timeStamp = timeStamp;
  }
  expect(seen).toEqual([[true, "render", 1, 2, "Components", undefined, "primary"]]);
});

// ADR 0967: React's DEV copyWithDelete (ReactFiberReconciler.js), `const
// updated = isArray(obj) ? obj.slice() : {...obj};`, then `updated.splice(key,
// 1)` or `delete updated[key]`, and `updated[key] = ..` of a string or a
// number key. `js::object::spread` is `{ ...value }`, which defines a
// `__proto__` key as its own, where `Object.assign({}, value)` sets the
// prototype; `js::array::splice` is `value.splice(start, n)`; `js::set` takes
// a key as `js::get` does.
test("an object spread, a splice and a set by any key are JS's", async () => {
  const { js, lib, warnings } = await compiled("js-object-spread", `use js::{Kind, Unknown};

fn copy_with_delete_impl(obj: &'static Unknown, path: &[&'static Unknown], index: usize) -> &'static Unknown {
    let key = path[index];
    let updated: &Unknown = match js::classify(Some(obj)) {
        Kind::Array(items) => js::unknown_of(items.to_vec()),
        _ => js::object::spread(Some(obj)),
    };
    if index + 1 == path.len() {
        if js::array::is_array(Some(updated)) {
            if let Kind::Number(at) = js::classify(Some(key)) {
                js::array::splice(updated, at as u32, 1);
            }
        } else {
            js::delete(updated, key);
        }
        return updated;
    }
    let inner = js::get(obj, key).expect("a path");
    js::set(updated, key, copy_with_delete_impl(inner, path, index + 1));
    updated
}

pub fn copy_with_delete(obj: &'static Unknown, path: &[&'static Unknown]) -> &'static Unknown {
    copy_with_delete_impl(obj, path, 0)
}

pub fn copied(value: Option<&Unknown>) -> &'static js::JsObject {
    js::object::spread(value)
}
`);
  expect(warnings).toEqual([]);
  expect(js).toContain("  const updated = Array.isArray(obj) ? obj.slice() : { ...obj };\n");
  expect(js).toContain("        updated.splice(value, 1);\n");
  expect(js).toContain("      delete updated[key];\n");
  expect(js).toContain("  updated[key] = copyWithDeleteImpl(inner, path, (index + 1) >>> 0);\n");
  const state = { a: { b: [1, 2, 3], c: 4 }, d: 5 };
  const copy = lib.copyWithDelete(state as never, ["a", "b", 1] as never) as any;
  expect(copy).toEqual({ a: { b: [1, 3], c: 4 }, d: 5 });
  expect(state).toEqual({ a: { b: [1, 2, 3], c: 4 }, d: 5 });
  expect(lib.copyWithDelete(state as never, ["a", "c"] as never)).toEqual({ a: { b: [1, 2, 3] }, d: 5 });
  // A `__proto__` key is the copy's own, its prototype `Object.prototype`.
  const keyed = JSON.parse('{"__proto__": {"x": 1}, "y": 2}');
  const spread = lib.copied(keyed as never) as any;
  expect([Object.getPrototypeOf(spread) === Object.prototype, Object.keys(spread), spread.x]).toEqual([true, ["__proto__", "y"], undefined]);
  expect(lib.copied(undefined as never)).toEqual({});
});
