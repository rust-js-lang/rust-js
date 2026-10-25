// The webapi crate: a page's globals, elements and events, typed as the
// DOM's are. Each test is a program of its own, compiled and run.

import { beforeAll, expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";

import { buildReact, compiler, fixture, root, run, target } from "./support";

beforeAll(buildReact, 600_000);

// ADR 0223: an event's name gives its listener the event its target takes,
// and a tag's name the element it makes, from `@webref/events` and
// `@webref/elements`, as TypeScript's `HTMLElementEventMap` and
// `HTMLElementTagNameMap` do. Each name is a type whose value is its string.
test("webapi's event and tag maps type a listener and an element", () => {
  const withWeb = ["--", "--extern", `webapi=${join(target, "libwebapi.rmeta")}`, "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];
  const dir = fixture("webapi-maps");
  const source = `use webapi::events::{Click, Keydown};
use webapi::tags::Button;
use webapi::{EventTargetExt, document};
pub fn wire() -> &'static webapi::HTMLButtonElement {
    let button = document.create_element(Button);
    button.set_disabled(false);
    button.add_event_listener(Click, |e| {
        let _ = e.client_x();
    }, None);
    document.add_event_listener(Keydown, |e| {
        let _ = e.key();
    }, None);
    button
}
`;
  writeFileSync(join(dir, "lib.rs"), source);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withWeb]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain('const button = document.createElement("button");');
  expect(js).toContain("button.disabled = false;");
  expect(js).toContain('button.addEventListener("click", (e) => {');
  expect(js).toContain('document.addEventListener("keydown", (e) => {');
  // A key's event isn't a mouse's, and a button takes no window's message.
  for (const [wrong, error, message] of [
    ["document.add_event_listener(Keydown, |e| {\n        let _ = e.key();", "document.add_event_listener(Keydown, |e| {\n        let _ = e.client_x();", "no method named `client_x`"],
    ["use webapi::events::{Click, Keydown};", "use webapi::events::{Click, Keydown, Message};\nfn message(b: &webapi::HTMLButtonElement) { b.add_event_listener(Message, |_| (), None); }", "the trait `webapi::Listen<webapi::events::Message>` is not implemented for `webapi::HTMLButtonElement`"],
  ]) {
    writeFileSync(join(dir, "lib.rs"), source.replace(wrong, error));
    const result = Bun.spawnSync([compiler, join(dir, "lib.rs"), "-o", join(dir, "wrong.js"), ...withWeb], { cwd: dir });
    expect(result.exitCode).not.toBe(0);
    expect(result.stderr.toString()).toContain(message);
  }
});

// SVG's elements are bound, and each SVG tag is a type of its own, as
// TypeScript's `SVGElementTagNameMap` has them: `createElementNS` of SVG's
// namespace and a tag makes its element, `svg_tags::Circle` an
// `SVGCircleElement`, which is an `SVGGeometryElement`, an
// `SVGGraphicsElement`, an `SVGElement` and an `Element`.
test("webapi's SVG elements are made by their tags", () => {
  const withWeb = ["--", "--extern", `webapi=${join(target, "libwebapi.rmeta")}`, "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];
  const dir = fixture("webapi-svg");
  writeFileSync(join(dir, "lib.rs"), `use webapi::namespaces::Svg;
use webapi::svg_tags::{Circle, Svg as SvgTag};
use webapi::{SVGCircleElement, SVGSVGElement, document};
pub fn draw() -> (&'static SVGSVGElement, &'static SVGCircleElement, bool) {
    let svg = document.create_element_ns(Svg, SvgTag);
    let circle = document.create_element_ns(Svg, Circle);
    circle.set_attribute("r", "4");
    svg.append((circle,));
    (svg, circle, circle.is_point_in_fill(None))
}
pub fn named() -> &'static webapi::Element {
    document.create_element_ns_named("http://www.w3.org/2000/svg", "g", None)
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withWeb]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain('const svg = document.createElementNS("http://www.w3.org/2000/svg", "svg");');
  expect(js).toContain('const circle = document.createElementNS("http://www.w3.org/2000/svg", "circle");');
  expect(js).toContain("return [svg, circle, circle.isPointInFill()];");
  expect(js).toContain('return document.createElementNS("http://www.w3.org/2000/svg", "g");');
});

// ADR 0269: what every JS global scope has, a window's, a worker's or Node's,
// `fetch` and `queueMicrotask`, is called bare, as react.dev's errors page
// fetches in Next.js's `getStaticProps`, where there's no `window`.
test("the global scope's functions are called bare", async () => {
  const withWeb = ["--", "--extern", `webapi=${join(target, "libwebapi.rmeta")}`, "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];
  const dir = fixture("global-scope");
  writeFileSync(join(dir, "lib.rs"), `use js::Promise;
use webapi::{Response, global};

pub fn codes(url: &str) -> Promise<&'static Response> {
    global::fetch(url, None)
}

pub fn later(then: &'static dyn Fn()) {
    global::queue_microtask(Box::new(move || then()));
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withWeb]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("return fetch(url);");
  expect(js).toContain("queueMicrotask(");
  expect(js).not.toContain("window");
  const fetched: string[] = [];
  const previous = globalThis.fetch;
  globalThis.fetch = ((url: string) => (fetched.push(url), Promise.resolve(new Response("{}")))) as typeof fetch;
  try {
    const { codes, later } = await import(join(dir, "lib.js"));
    await codes("https://example.com/codes.json");
    const ran: number[] = [];
    later(() => ran.push(1));
    await Promise.resolve();
    expect([fetched, ran]).toEqual([["https://example.com/codes.json"], [1]]);
  } finally {
    globalThis.fetch = previous;
  }
});

// ADR 0869: a callback a web API calls once, `queueMicrotask`'s, is a
// `Box<dyn FnOnce()>`, so it may move out what it holds, as React's
// `queueSeveralMicrotasks` gives the callback it's given to the next
// microtask: `queueMicrotask(() => queueMicrotask(callback))`.
test("a callback a web API calls once moves what it holds", async () => {
  const withWeb = ["--", "--extern", `webapi=${join(target, "libwebapi.rmeta")}`, "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];
  const dir = fixture("called-once");
  writeFileSync(join(dir, "lib.rs"), `use webapi::global;

pub fn twice_later(callback: Box<dyn Fn()>) {
    global::queue_microtask(Box::new(move || global::queue_microtask(callback)));
}

pub fn framed(log: Box<dyn FnOnce(f64)>) -> u32 {
    webapi::window.request_animation_frame(Box::new(move |time| log(time)))
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withWeb]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("queueMicrotask(() => {\n    queueMicrotask(callback);\n  });");
  expect(js).toContain("return window.requestAnimationFrame((time) => {\n    log(time);\n  });");
  const { twiceLater } = await import(join(dir, "lib.js"));
  const ran: string[] = [];
  twiceLater(() => ran.push("callback"));
  await Promise.resolve();
  ran.push("one microtask");
  await Promise.resolve();
  await Promise.resolve();
  expect(ran).toEqual(["one microtask", "callback"]);
  const lib = readFileSync(join(root, "webapi", "src", "lib.rs"), "utf8");
  expect(lib).toContain("pub fn queue_microtask(callback: Box<dyn FnOnce()>)");
  // One JS calls again stays an `FnMut`: an observer's, a listener's.
  expect(lib).toMatch(/pub fn new\(callback: Box<dyn FnMut\(&\[&ResizeObserverEntry\]/);
});

// The `history` global, a window's, as `document` is, as react.dev's _app
// sets `history.scrollRestoration` where the browser is Safari.
test("the history global is the page's history", async () => {
  const withWeb = ["--", "--extern", `webapi=${join(target, "libwebapi.rmeta")}`, "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];
  const dir = fixture("history-global");
  writeFileSync(join(dir, "lib.rs"), `use webapi::history;

pub fn restore() {
    history.set_scroll_restoration("auto");
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withWeb]);
  expect(readFileSync(join(dir, "lib.js"), "utf8")).toContain('history.scrollRestoration = "auto";');
  const previous = (globalThis as any).history;
  (globalThis as any).history = { scrollRestoration: "manual" };
  try {
    (await import(join(dir, "lib.js"))).restore();
    expect((globalThis as any).history.scrollRestoration).toBe("auto");
  } finally {
    (globalThis as any).history = previous;
  }
});

// ADR 0225: a WebIDL parameter typed `any` takes a value as JS has it, and
// one the browser copies, `postMessage`'s, `pushState`'s and
// `structuredClone`'s, only a value it copies as it is: `js::StructuredClone`.
// A closure, which it can't copy, and `Some(None)`, which would arrive as
// rust-js's box, are rustc's errors; `reportError` takes anything still.
test("webapi's any parameters take any value as JS has it, and a clone only one it copies", async () => {
  const withWeb = ["--", "--extern", `webapi=${join(target, "libwebapi.rmeta")}`, "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];
  const dir = fixture("any-params");
  const source = `use js::StructuredClone;
use webapi::window;
pub struct Saved {
    pub page: u32,
}
// Its fields are numbers, which the browser copies.
unsafe impl StructuredClone for Saved {}
pub fn save(page: u32) {
    window.history().push_state(Saved { page }, "", None);
}
pub fn copy(text: &str) -> Option<&'static js::Unknown> {
    window.structured_clone(text, None)
}
pub fn copies(blob: &webapi::Blob) -> Option<&'static js::Unknown> {
    window.structured_clone((vec![Some(1.5), None], "a", blob), None)
}
pub fn send(text: String) {
    let options = webapi::WindowPostMessageOptions { target_origin: Some("*"), ..Default::default() };
    window.post_message(Some(text), Some(options));
}
pub fn ping(port: &webapi::MessagePort) {
    port.post_message(1, None);
}
pub fn wake(port: &webapi::MessagePort) {
    port.post_message((), None);
}
pub fn listed(value: Option<&js::Unknown>) -> Option<String> {
    match js::classify(value) {
        js::Kind::Array(items) if items.len() < 100 => js::json::stringify_with(js::unknown(items), None, 0).unwrap(),
        _ => None,
    }
}
pub fn report(error: &js::JsError) {
    window.report_error(error);
    window.report_error(|| ());
}
`;
  writeFileSync(join(dir, "lib.rs"), source);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withWeb]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain('window.history.pushState({ page }, "");');
  expect(js).toContain("return window.structuredClone(text);");
  expect(js).toContain('return window.structuredClone([[1.5, undefined], "a", blob]);');
  // One function, its options' (ADR 0594).
  expect(js).toContain('const options = { targetOrigin: "*" };\n  window.postMessage(text, options);');
  expect(js).toContain("port.postMessage(1);");
  // \`()\` is \`undefined\`, which the browser copies as it is (ADR 0225 amended).
  expect(js).toContain("port.postMessage(undefined);");
  // \`Kind::Array\`'s slice is the array, its value \`js::unknown(items)\` (ADR 0225 amended).
  expect(js).toContain("return JSON.stringify(value, null, 0);");
  expect(js).toContain("window.reportError(error);");
  for (const [written, error] of [
    ["window.structured_clone(|| (), None)", "the trait `js::StructuredClone` is not implemented for closure"],
    ["window.structured_clone(Some(None::<i32>), None)", "the trait `js::Defined` is not implemented for `std::option::Option<i32>`"],
    ["window.structured_clone(window, None)", "the trait `js::StructuredClone` is not implemented for `webapi::Window`"],
    ["window.structured_clone(Some(()), None)", "the trait `js::Defined` is not implemented for `()`"],
  ]) {
    writeFileSync(join(dir, "lib.rs"), source.replace("window.structured_clone(text, None)", written));
    const result = Bun.spawnSync([compiler, join(dir, "lib.rs"), "-o", join(dir, "wrong.js"), ...withWeb], { cwd: dir });
    expect(result.exitCode).not.toBe(0);
    expect(result.stderr.toString()).toContain(error);
  }
});

// An element's constructor is WebIDL's `[HTMLConstructor]`, which only a
// custom element's class can call: `new HTMLDivElement()` in a page throws
// "Illegal constructor". So webapi binds none, and an element is made with
// `document.create_element(..)`. Other constructors, `new Event`, it binds.
test("the webapi crate binds no element constructor, which a page can't call", () => {
  const lib = readFileSync(join(root, "webapi", "src", "lib.rs"), "utf8");
  expect(lib.match(/rust_js::link_name = "new HTML\w*"/g) ?? []).toEqual([]);
  expect(lib).toContain('#[cfg_attr(rust_js, rust_js::link_name = "new Event")]');
  expect(lib).toContain('#[cfg_attr(rust_js, rust_js::link_name = "new TextEncoder")]');
});

// ADR 0269: a window's own functions are called bare too, as TypeScript's DOM
// declares them globals and react.dev's NavigationBar asks `confirm('Clear
// all your edits?')`. `window.confirm(..)` is still the method's.
test("a window's own functions are called bare", async () => {
  const withWeb = ["--", "--extern", `webapi=${join(target, "libwebapi.rmeta")}`, "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];
  const dir = fixture("window-global");
  writeFileSync(join(dir, "lib.rs"), `use webapi::{window, window_global};

pub fn cleared() -> bool {
    window_global::confirm(Some("Clear all your edits?"))
}

pub fn asked() -> bool {
    window.confirm(Some("Sure?"))
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withWeb]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain('return confirm("Clear all your edits?");');
  expect(js).toContain('return window.confirm("Sure?");');
});

// ADR 0640: a TypeScript rest parameter, `error(...data: any[])`, is a
// variadic binding of a tuple, each argument of its own type: React's
// `console.error(format, propName, typeName(value))`, an element's
// `append(node, "text")`; `(x,)` is one, `()` none, and a `None` before
// none is left out, as a trailing one is.
test("a rest parameter is a tuple, each argument its own type", async () => {
  const withWeb = ["--", "--extern", `webapi=${join(target, "libwebapi.rmeta")}`, "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];
  const dir = fixture("rest-tuple");
  const source = `use js::Unknown;
use webapi::{Element, console};

pub fn check_prop(prop_name: &str, type_name: String) {
    console::error((
        "The provided \`%s\` prop is an unsupported type %s. \\
         This value must be coerced to a string before using it here.",
        prop_name,
        type_name,
    ));
}

pub fn logged(value: Option<&Unknown>, n: f64) {
    console::log(("value", value, n));
}

pub fn warned(message: &str) {
    console::warn((message,));
}

pub fn blank() {
    console::log(());
}

pub fn checked(ok: bool, message: &str) {
    console::assert(Some(ok), (message,));
}

pub fn timed() {
    console::time_log(None, ());
}

pub fn spread(held: (&str, f64)) {
    console::time_log(None, held);
}

pub fn filled(el: &Element, child: &Element, name: &str, kind: String) {
    el.append((child, "text"));
    el.class_list().add((name, kind));
}

pub fn listed(el: &Element, children: Vec<&Element>, names: &[&str]) {
    el.append(&children);
    el.class_list().add(names);
}
`;
  writeFileSync(join(dir, "lib.rs"), source);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withWeb]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain('  console.error(\n    "The provided `%s` prop is an unsupported type %s. This value must be coerced to a string before using it here.",\n    propName,\n    typeName,\n  );');
  expect(js).toContain('console.log("value", value, n);');
  expect(js).toContain("console.warn(message);");
  expect(js).toContain("console.log();");
  expect(js).toContain("console.assert(ok, message);");
  expect(js).toContain("console.timeLog();");
  // Before a spread, a `None` is still `undefined`, its place kept.
  expect(js).toContain("console.timeLog(undefined, ...held);");
  expect(js).toContain('el.append(child, "text");');
  expect(js).toContain("el.classList.add(name, kind);");
  // A `Vec` or a slice of them, spread.
  expect(js).toContain("el.append(...children);\n  el.classList.add(...names);");
  const lib = await import(join(dir, "lib.js"));
  const said: unknown[][] = [];
  const kept = { ...console };
  for (const method of ["error", "log", "warn", "assert", "timeLog"] as const) {
    (console as any)[method] = (...args: unknown[]) => said.push([method, ...args]);
  }
  try {
    lib.checkProp("key", "Symbol");
    lib.logged(undefined, 2);
    lib.warned("careful");
    lib.blank();
    lib.checked(false, "no");
    lib.timed();
    lib.spread(["a", 1]);
  } finally {
    Object.assign(console, kept);
  }
  expect(said).toEqual([
    ["error", "The provided `%s` prop is an unsupported type %s. This value must be coerced to a string before using it here.", "key", "Symbol"],
    ["log", "value", undefined, 2],
    ["warn", "careful"],
    ["log"],
    ["assert", false, "no"],
    ["timeLog"],
    ["timeLog", undefined, "a", 1],
  ]);
  const appended: unknown[][] = [];
  const el = { append: (...a: unknown[]) => appended.push(a), classList: { add: (...a: unknown[]) => appended.push(a) } };
  lib.filled(el, "child", "a", "b");
  lib.listed(el, ["c", "d"], ["e"]);
  expect(appended).toEqual([["child", "text"], ["a", "b"], ["c", "d"], ["e"]]);
  // A value that isn't a tuple says what to write.
  writeFileSync(join(dir, "lib.rs"), source.replace("console::warn((message,));", "console::warn(message);"));
  const result = Bun.spawnSync([compiler, join(dir, "lib.rs"), "-o", join(dir, "wrong.js"), ...withWeb], { cwd: dir });
  expect(result.exitCode).not.toBe(0);
  expect(result.stderr.toString()).toContain("`&str` is not rest arguments");
});

// ADR 0811: V8's `console.createTask`, which no TypeScript lib declares and
// Firefox, Safari, jsdom and Bun lack, is a property that may be
// `undefined`, as React reads it: `console.createTask ? .. : ..`, and
// `const createTask = __DEV__ && console.createTask ? console.createTask : ..`.
test("console.createTask is a function the host may lack, and its task runs what it's given", async () => {
  const withWeb = ["--", "--edition", "2024", "--extern", `webapi=${join(target, "libwebapi.rmeta")}`, "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];
  const dir = fixture("webapi-create-task");
  writeFileSync(join(dir, "lib.rs"), `use webapi::{ConsoleTask, console};

unsafe extern "Rust" {
    pub safe static __DEV__: bool;
}

thread_local! {
    static CREATE_TASK: Option<&'static dyn Fn(&str) -> &'static ConsoleTask> =
        if __DEV__ { console::create_task() } else { None };
}

pub fn task_of(name: &str) -> Option<&'static ConsoleTask> {
    CREATE_TASK.with(|create_task| create_task.map(|create_task| create_task(name)))
}

pub fn debug_task() -> Option<&'static ConsoleTask> {
    console::create_task().map(|create_task| create_task("lazy()"))
}

pub fn run_in(task: &ConsoleTask, x: f64) -> f64 {
    task.run(|| x * 2.0)
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withWeb]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("let createTask;\nif (__DEV__) {\n  createTask = console.createTask;\n}");
  expect(js).toContain("return createTask ? createTask(name) : undefined;");
  expect(js).toContain('export function debugTask() {\n  const createTask = console.createTask;\n  return createTask ? createTask("lazy()") : undefined;\n}');
  expect(js).toContain("return task.run(() => x * 2);");
  const host = console as any;
  expect(host.createTask).toBeUndefined();
  const made: string[] = [];
  (globalThis as any).__DEV__ = true;
  host.createTask = (name: string) => (made.push(name), { run: (f: () => unknown) => f() });
  try {
    const lib = await import(join(dir, "lib.js"));
    const task = lib.taskOf("<App>");
    expect([made, lib.runIn(task, 3)]).toEqual([["<App>"], 6]);
    expect(lib.debugTask()).toBeDefined();
    delete host.createTask;
    expect(lib.debugTask()).toBeUndefined();
    expect(made).toEqual(["<App>", "lazy()"]);
  } finally {
    delete host.createTask;
    delete (globalThis as any).__DEV__;
  }
});
