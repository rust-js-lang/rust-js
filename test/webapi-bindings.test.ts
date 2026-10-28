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

pub fn framed(log: Box<dyn FnOnce(f64)>) -> f64 {
    webapi::window.request_animation_frame(Box::new(move |time| log(time)))
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withWeb]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  // `queueMicrotask` gives `undefined`, as WebIDL says (ADR 0883); `log`, a
  // `dyn FnOnce`, may give anything.
  expect(js).toContain("queueMicrotask(() => queueMicrotask(callback));");
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

// ADR 0891: each constructor a window's global scope has is a window's too,
// its module's `new` a `Window` method of the module's name, `new_error_event`.
test("each class a window has is constructed of the window too", () => {
  const lib = readFileSync(join(root, "webapi", "src", "lib.rs"), "utf8");
  const window = lib.slice(lib.indexOf("/// The constructors a window holds"));
  const made = [...lib.matchAll(/^pub mod (\w+) \{$[^]*?^}$/gm)].flatMap(([module, name]) =>
    [...module.matchAll(/link_name = "(new [\w.]+)"\)\]\n(?:    .*\n)*?    pub fn new\b/g)].map(([, js]) => [name, js]),
  );
  expect(made.length).toBeGreaterThan(200);
  const held = [...window.matchAll(/link_name = "(new [\w.]+)"\)\]\n(?:    .*\n)*?    pub fn new_(\w+)\(&self/g)].map(([, js, name]) => [name, js]);
  expect(held).toEqual(made);
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
  expect(js).toContain("return createTask != null ? createTask(name) : undefined;");
  expect(js).toContain('export function debugTask() {\n  const createTask = console.createTask;\n  return createTask != null ? createTask("lazy()") : undefined;\n}');
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

// ADR 0891: a window's constructors are the window's, as TypeScript's
// `Window & typeof globalThis` has them: `window.new_error_event(..)` is
// `new window.ErrorEvent(..)`, and `js::is_defined(Window::new_error_event)`
// asks `typeof window.ErrorEvent`. React's `reportGlobalError` dispatches
// on the window, which in its server-integration tests is a jsdom window
// that isn't the global object: the global `ErrorEvent` is another, or none.
test("a window's constructor is the window's, apart from the global one", async () => {
  const withWeb = ["--", "--extern", `webapi=${join(target, "libwebapi.rmeta")}`, "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];
  const dir = fixture("window-constructor");
  writeFileSync(join(dir, "lib.rs"), `use webapi::{ErrorEventInit, Window, window};

pub fn report(message: &str) -> Option<bool> {
    if js::is_defined(Window::new_error_event) {
        let event = window.new_error_event(
            "error",
            Some(ErrorEventInit {
                bubbles: Some(true),
                message: Some(message),
                ..Default::default()
            }),
        );
        return Some(window.dispatch_event(event));
    }
    None
}

pub fn wasm(bytes: &js::ArrayBuffer) -> &'static webapi::WebAssemblyModule {
    window.new_web_assembly_module(bytes, None)
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withWeb]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain('if (typeof window !== "undefined" && typeof window.ErrorEvent !== "undefined") {');
  expect(js).toContain('const event = new window.ErrorEvent("error", {');
  expect(js).toContain("return new window.WebAssembly.Module(bytes);");
  const host = globalThis as any;
  const lib = await import(join(dir, "lib.js"));
  expect(lib.report("lost")).toBeUndefined();
  // A window that isn't the global object, of an `ErrorEvent` of its own,
  // as jsdom's is in React's tests: the global one, Bun's, is another.
  const dispatched: unknown[] = [];
  class ErrorEvent {
    constructor(
      public type: string,
      public init: object,
    ) {}
  }
  host.window = { ErrorEvent, dispatchEvent: (event: unknown) => (dispatched.push(event), false) };
  try {
    expect(host.ErrorEvent).toBeFunction();
    expect(host.ErrorEvent).not.toBe(ErrorEvent);
    expect(lib.report("boom")).toBe(false);
    expect(dispatched).toEqual([new ErrorEvent("error", { bubbles: true, message: "boom" })]);
    expect(dispatched[0]).toBeInstanceOf(ErrorEvent);
    expect(dispatched[0]).not.toBeInstanceOf(host.ErrorEvent);
    host.window = {};
    expect(lib.report("lost")).toBeUndefined();
  } finally {
    delete host.window;
  }
  // A path's constructor is its last step: what it makes isn't read of it.
  writeFileSync(join(dir, "lib.rs"), `use webapi::{ErrorEvent, Window};

pub fn asked() -> bool {
    js::is_defined((Window::new_error_event, ErrorEvent::message))
}
`);
  const refused = Bun.spawnSync([compiler, join(dir, "lib.rs"), "-o", join(dir, "wrong.js"), ...withWeb], { cwd: dir });
  expect([refused.exitCode, refused.stderr.toString()]).toEqual([1, expect.stringContaining("a step isn't a method of what the one before gives")]);
});

// ADR 0911: `js::class::<T>()` of a binding's class is the global class
// TypeScript names, as React's ReactFiberCacheComponent keeps
// `AbortController` where the host has one, and its shim where it hasn't;
// `js::new` of either constructs it.
test("a binding's class is the global class TypeScript names", async () => {
  const withWeb = ["--", "--extern", `webapi=${join(target, "libwebapi.rmeta")}`, "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];
  const dir = fixture("binding-class");
  writeFileSync(join(dir, "lib.rs"), `use js::Class;
use webapi::AbortController;

#[cfg_attr(rust_js, rust_js::class)]
pub struct AbortControllerShim {
    pub shimmed: bool,
}

#[cfg_attr(rust_js, rust_js::class)]
impl AbortControllerShim {
    pub fn new() -> Self {
        AbortControllerShim { shimmed: true }
    }
}

thread_local! {
    // In environments without AbortController (e.g. tests)
    // replace it with a lightweight shim that only has the features we use.
    #[cfg_attr(rust_js, rust_js::name = "AbortControllerLocal")]
    static ABORT_CONTROLLER_LOCAL: &'static Class = if js::is_defined(webapi::abort_controller::new) {
        js::class::<AbortController>()
    } else {
        js::class::<AbortControllerShim>()
    };
}

pub fn made() -> &'static AbortController {
    let controller = ABORT_CONTROLLER_LOCAL.with(|class| js::new(*class, ()));
    unsafe { js::cast(Some(controller)) }
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withWeb]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain(`// In environments without AbortController (e.g. tests)
// replace it with a lightweight shim that only has the features we use.
const AbortControllerLocal =
  typeof AbortController !== "undefined" ? AbortController : AbortControllerShim;`);
  expect(js).toContain("const controller = new AbortControllerLocal();");
  // Where the host has one, its own; where it hasn't, the shim.
  const host = globalThis as any;
  expect((await import(join(dir, "lib.js"))).made()).toBeInstanceOf(AbortController);
  writeFileSync(join(dir, "shim.js"), js);
  const real = host.AbortController;
  delete host.AbortController;
  try {
    expect((await import(join(dir, "shim.js"))).made()).toEqual({ shimmed: true });
  } finally {
    host.AbortController = real;
  }
  // A type that names no class TypeScript has, as what JS threw doesn't, is refused.
  for (const [ty, name] of [["js::JsError", "error"], ["js::JsObject", "object"], ["js::Unknown", "unknown"]]) {
    writeFileSync(join(dir, `${name}.rs`), `pub fn class() -> &'static js::Class {\n    js::class::<${ty}>()\n}\n`);
    const refused = Bun.spawnSync([compiler, join(dir, `${name}.rs`), "-o", join(dir, "wrong.js"), ...withWeb], { cwd: dir });
    expect([refused.exitCode, refused.stderr.toString()]).toEqual([1, expect.stringContaining("`js::class::<T>()` of a `T` that's no `#[rust_js::class]` struct, nor a class TypeScript names")]);
  }
});

// ADR 0963: React's performance track logs each phase with Chrome's
// `console.timeStamp(label, start, end, track, trackGroup, color)`, which no
// WebIDL declares, asked first, `typeof console.timeStamp === "function"`;
// and measures with `performance.measure(name, { start, end })`, whose
// times are `DOMHighResTimeStamp`s, doubles: `string | number`'s number of
// a union is the wider of the members two unions of its name have.
test("console.timeStamp is Chrome's, and a measure's times are doubles", async () => {
  const withWeb = ["--", "--edition", "2024", "--extern", `webapi=${join(target, "libwebapi.rmeta")}`, "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];
  const dir = fixture("webapi-time-stamp");
  writeFileSync(join(dir, "lib.rs"), `use webapi::{PerformanceMeasureOptions, console, performance};

pub fn log_render(name: &str, start_time: f64, end_time: f64, color: &str) {
    if js::is_function(console::time_stamp) {
        console::time_stamp(
            name,
            Some(&start_time),
            Some(&end_time),
            Some("Components \u{269B}"),
            None,
            Some(color),
        );
    }
}

pub fn measure(name: &str, start_time: f64, end_time: f64) {
    let options = PerformanceMeasureOptions {
        start: Some(start_time.into()),
        end: Some(end_time.into()),
        ..Default::default()
    };
    performance.measure(name, Some(&options), None);
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withWeb]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain('if (typeof console.timeStamp === "function") {\n    console.timeStamp(name, startTime, endTime, "Components \u269B", undefined, color);\n  }');
  expect(js).toContain("  const options = {\n    start: startTime,\n    end: endTime,\n  };\n  performance.measure(name, options);");
  const host = console as any;
  const stamped: unknown[][] = [];
  const timeStamp = host.timeStamp;
  host.timeStamp = (...args: unknown[]) => stamped.push(args);
  try {
    const lib = await import(join(dir, "lib.js"));
    lib.logRender("App", 1.5, 2.25, "primary");
    expect(stamped).toEqual([["App", 1.5, 2.25, "Components \u269B", undefined, "primary"]]);
  } finally {
    host.timeStamp = timeStamp;
  }
});

// React's SchedulerPostTask goes on with a task by `scheduler.yield({signal})`,
// as Chrome first shipped it; the spec's yield takes nothing now, and a
// host ignores what it isn't given a parameter for. Its test's mock reads
// the signal it's given. A host's extension is the method (ADR 0963,
// amended): `scheduler.r#yield(Some(options))`.
test("scheduler.yield takes the options Chrome first took, as React gives them", async () => {
  const withWeb = ["--", "--edition", "2024", "--extern", `webapi=${join(target, "libwebapi.rmeta")}`, "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];
  const dir = fixture("webapi-scheduler-yield");
  writeFileSync(join(dir, "lib.rs"), `use webapi::{AbortSignal, Scheduler, SchedulerPostTaskOptions};

pub async fn continued(scheduler: &Scheduler, signal: &AbortSignal) {
    let continuation_options = SchedulerPostTaskOptions {
        signal: Some(signal),
        ..Default::default()
    };
    scheduler.r#yield(Some(continuation_options)).await;
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withWeb]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("await scheduler.yield(continuationOptions);");
  const lib = await import(join(dir, "lib.js"));
  const given: unknown[] = [];
  const signal = new AbortController().signal;
  await lib.continued({ yield: async (options: unknown) => void given.push(options) }, signal);
  expect(given).toEqual([{ signal }]);
});

// ADR 0275, amended (the user's decision, React port gap rd46): an
// attribute TypeScript types `T | null` is set of a value or `None`,
// `impl Into<Option<T>>`, `None` `null`: ReactInputSelection's
// `input.selectionStart = start` of a `number | null`, and
// `node.textContent = null`.
test("a nullable attribute's setter takes a value or None", async () => {
  const withWeb = ["--", "--extern", `webapi=${join(target, "libwebapi.rmeta")}`, "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];
  const dir = fixture("webapi-nullable-setters");
  writeFileSync(join(dir, "lib.rs"), `use webapi::{HTMLInputElement, Node};

pub struct Offsets {
    #[cfg_attr(rust_js, rust_js::nullable)]
    pub start: Option<f64>,
    #[cfg_attr(rust_js, rust_js::nullable)]
    pub end: Option<f64>,
}

pub fn restore(input: &HTMLInputElement, offsets: &Offsets) {
    let start = offsets.start;
    let end = offsets.end;
    input.set_selection_start(start);
    input.set_selection_end(js::math::min((end.unwrap_or(0.0), f64::from(js::string::length(&input.value())))));
}

pub fn texts(node: &Node, other: &Node, text: Option<&str>) {
    node.set_text_content("x");
    other.set_text_content(None);
    node.set_text_content(text);
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withWeb]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("  input.selectionStart = start;\n  input.selectionEnd = Math.min(end ?? 0, input.value.length);\n");
  expect(js).toContain('  node.textContent = "x";\n  other.textContent = null;\n  node.textContent = text ?? null;\n');
  const lib = await import(join(dir, "lib.js"));
  const input = { value: "abc", selectionStart: 1 as number | null, selectionEnd: 1 as number | null };
  lib.restore(input, { start: null, end: 9 });
  expect([input.selectionStart, input.selectionEnd]).toEqual([null, 3]);
  const [node, other] = [{ textContent: "" as string | null }, { textContent: "" as string | null }];
  lib.texts(node, other, undefined);
  expect([node.textContent, other.textContent]).toEqual([null, null]);
});

// React port gap rd68: TypeScript's index signature, `[index: number]:
// HTMLOptionElement`, is std's `Index`, so ReactDOMSelect's
// `options[i].value` and ReactDOMInput's `group[i]` read as JS reads them,
// no `$unwrap(options.item(i))` nor cast: a binding's `impl Index` is the
// binding its method declares, `get []`.
test("a collection's index signature is Index, options[i]", async () => {
  const withWeb = ["--", "--extern", `webapi=${join(target, "libwebapi.rmeta")}`, "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];
  const dir = fixture("webapi-index-signature");
  writeFileSync(join(dir, "lib.rs"), `use webapi::{Element, HTMLSelectElement};

pub fn select(node: &HTMLSelectElement, value: &str) -> String {
    let options = node.options();
    for i in 0..options.length() as usize {
        if options[i].value() == value {
            options[i].set_selected(true);
        }
    }
    options[0].value()
}

pub fn first_tag(root: &Element) -> String {
    let group = root.query_selector_all("input");
    group[0].tag_name()
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withWeb]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("    if (options[i].value === value) {\n      options[i].selected = true;\n    }\n");
  expect(js).toContain("  return options[0].value;\n");
  expect(js).toContain("  return group[0].tagName;\n");
  const lib = await import(join(dir, "lib.js"));
  const options = [{ value: "a", selected: false }, { value: "b", selected: false }];
  expect(lib.select({ options }, "b")).toBe("a");
  expect(options.map((o) => o.selected)).toEqual([false, true]);
  expect(lib.firstTag({ querySelectorAll: () => [{ tagName: "INPUT" }] })).toBe("INPUT");
});

// The user's decision (React port gap d13a): a value the DOM makes text of,
// whatever TypeScript's `string` says, takes any value, `impl IntoUnknown`,
// as React DOM gives it, `node.setAttribute(name, value)` of a number or a
// `bool`, no `js::cast::<&str>(..)`.
test("a value the DOM makes text of takes any value", async () => {
  const withWeb = ["--", "--extern", `webapi=${join(target, "libwebapi.rmeta")}`, "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];
  const dir = fixture("webapi-stringified-values");
  writeFileSync(join(dir, "lib.rs"), `use webapi::{CSSStyleDeclaration, Element};

pub fn set(node: &Element, style: &CSSStyleDeclaration, value: &js::Unknown, name: &String) {
    node.set_attribute("tabindex", 3);
    node.set_attribute("hidden", true);
    node.set_attribute("title", name);
    node.set_attribute_ns("http://www.w3.org/1999/xlink", "href", value);
    style.set_property("--x", 1.5, None);
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withWeb]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain('  node.setAttribute("tabindex", 3);\n  node.setAttribute("hidden", true);\n  node.setAttribute("title", name);\n');
  expect(js).toContain('  style.setProperty("--x", 1.5);\n');
  const lib = await import(join(dir, "lib.js"));
  const attributes: Record<string, string> = {};
  const node = { setAttribute: (n: string, v: unknown) => void (attributes[n] = String(v)), setAttributeNS: (_: string, n: string, v: unknown) => void (attributes[n] = String(v)) };
  const props: Record<string, string> = {};
  lib.set(node, { setProperty: (n: string, v: unknown) => void (props[n] = String(v)) }, 0, "Save");
  expect([attributes, props]).toEqual([{ tabindex: "3", hidden: "true", title: "Save", href: "0" }, { "--x": "1.5" }]);
});

// The user's decision (React port gaps fx1, rd68): a WebIDL integer a
// binding gives is a whole number in its type's range, which its
// `#[rust_js::whole]` says (ADR 0994, amended). Its cast into an integer
// that holds each of them is the number as it is, `options.length`, and its
// `{}` JS's text of it, `${response.status}`: both what Rust makes of it. A
// cast into one that doesn't hold them all saturates still, as `as` does.
test("a WebIDL integer is a whole number in its type's range", async () => {
  const withWeb = ["--", "--extern", `webapi=${join(target, "libwebapi.rmeta")}`, "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];
  const dir = fixture("webapi-whole-numbers");
  const source = `use webapi::{HTMLSelectElement, Response};

pub fn selected(node: &HTMLSelectElement) -> Option<usize> {
    let options = node.options();
    (0..options.length() as usize).find(|&i| options[i].selected())
}

pub fn status_line(response: &Response) -> String {
    format!("HTTP {}", response.status())
}

pub fn status_byte(response: &Response) -> u8 {
    response.status() as u8
}

pub fn kind(node: &webapi::Node) -> i16 {
    node.node_type() as i16
}

pub fn scaled(response: &Response) -> String {
    format!("{}", response.status() / 8.0)
}
`;
  writeFileSync(join(dir, "lib.rs"), source);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withWeb]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("options.length");
  expect(js).toContain("  return `HTTP ${response.status}`;\n");
  expect(js).toContain("  return $f64ToInt(response.status, 0, 255);\n");
  expect(js).toContain("  return $f64ToInt(node.nodeType, -32768, 32767);\n");
  expect(js).toContain("$displayF64(response.status / 8)");
  expect(js).not.toContain("$f64ToInt(options.length");
  const lib = await import(join(dir, "lib.js"));
  const options = [{ selected: false }, { selected: true }];
  expect([lib.selected({ options }), lib.selected({ options: [] })]).toEqual([1, undefined]);
  expect([lib.statusLine({ status: 404 }), lib.statusByte({ status: 404 }), lib.kind({ nodeType: 1 }), lib.scaled({ status: 404 })])
    .toEqual(["HTTP 404", 255, 1, "50.5"]);
  // It's said of a binding giving an `f64`, by its least and most numbers;
  // `#[rust_js::position]` was indexOf's, `whole = "-1..=4294967294"`.
  for (const [attr, message] of [
    ['#[rust_js::whole = "0..65535"]', '`#[rust_js::whole = "lo..=hi"]` writes the least and the most whole number'],
    ['#[rust_js::whole = "5..=1"]', '`#[rust_js::whole = "lo..=hi"]` writes the least and the most whole number'],
    ["#[rust_js::position]", '`#[rust_js::position]` is `#[rust_js::whole = "-1..=4294967294"]`'],
  ]) {
    writeFileSync(join(dir, "lib.rs"), `#[rust_js::link_name = "get size"] ${attr} #[allow(unused_variables)] pub fn size(this: &js::Unknown) -> f64 { unreachable!() }\n`);
    const result = Bun.spawnSync([compiler, join(dir, "lib.rs"), "-o", join(dir, "wrong.js"), ...withWeb], { cwd: dir });
    expect(result.exitCode).not.toBe(0);
    expect(result.stderr.toString()).toContain(message);
  }
  writeFileSync(join(dir, "lib.rs"), `#[rust_js::whole = "0..=9"] pub fn size() -> f64 { 1.0 }\n`);
  const own = Bun.spawnSync([compiler, join(dir, "lib.rs"), "-o", join(dir, "wrong.js"), ...withWeb], { cwd: dir });
  expect(own.stderr.toString()).toContain("`#[rust_js::whole]` is of a binding that gives an `f64`");
});
