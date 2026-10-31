import { beforeAll, expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { chromium } from "playwright";
import { buildReact, compiler, fixture, root, run, target } from "./support";

beforeAll(buildReact, 300_000);
const flags = ["--", "--extern", `webapi=${join(target, "libwebapi.rmeta")}`, "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];

function compileEvents(): string {
  const out = join(fixture("webapi-events"), "events.js");
  run([compiler, "test/webapi-events.rs", "-o", out, ...flags]);
  return readFileSync(out, "utf8");
}

test("event methods and reusable listeners emit direct JavaScript", () => {
  const js = compileEvents();
  expect(js).toContain('button.addEventListener("click", (e) => {\n    globalThis.record(e.clientX);\n  });');
  expect(js).toContain('document.addEventListener("keydown", (e) => {');
  // A shared callback is the function itself, made once, not a copy of it.
  expect(js).toContain('function listener(e) {\n    globalThis.record(e.clientX);\n  }\n  button.addEventListener("click", listener);\n  button.removeEventListener("click", listener);');
  expect(js).toContain('button.removeEventListener("click", listener, true);');
  // A named `fn` is the function itself, added and removed.
  expect(js).toContain('button.addEventListener("custom", onCustom);\n  button.removeEventListener("custom", onCustom);');
  expect(js).not.toMatch(/unreachable|\.add_event_listener|\.client_x|\.prevent_default/);
  // A listener JS gave is passed as it is, and called as JS would (rd185).
  expect(js).toContain('export function givenRemoved(button, given) {\n  button.addEventListener("custom", given);\n  button.removeEventListener("custom", given);\n}');
  // checkPassiveEvents' options object, the listener and both options.
  expect(js).toContain('button.addEventListener("test", listener, options);\n  button.removeEventListener("test", listener, options);');
  expect(js).toContain('if (typeof given === "function") {\n      given.call(this, event);\n    } else {\n      given.handleEvent(event);\n    }');
  // A listener typed `&'static dyn Fn(&Event)`, of every event's lifetime, is
  // one too: made, assigned, returned and removed as the function itself.
  expect(js).toContain('if (once) {\n    const original = shared;\n    shared = (event) => {\n      button.removeEventListener("custom", original);\n      original(event);\n    };\n  }\n  button.addEventListener("custom", shared);\n  return shared;');
});

// FragmentInstance's \`once\` wrapper, a \`function::with_this\` function kept
// as an \`EventListener\` to add to each child and remove by identity:
// \`with_this(..)\`'s function itself (rd209).
test("a function of its this is kept as a listener, the function itself", async () => {
  const js = compileEvents();
  expect(js).toContain('  function attached(event) {\n    globalThis.record(20);\n    if (typeof given === "function") {\n      given.call(this, event);\n    }\n  }\n  for (const button of buttons) {\n    button.addEventListener("custom", attached);\n  }\n  return attached;');
  const out = join(fixture("webapi-events"), "events.js");
  writeFileSync(out, js);
  const { keptWrapped } = await import(out);
  const added: [unknown, Function][] = [];
  const button = (name: string) => ({ name, addEventListener: (_: string, f: Function) => void added.push([name, f]) });
  const seen: unknown[] = [];
  (globalThis as Record<string, unknown>).record = (n: number) => void seen.push(n);
  const kept = keptWrapped([button("a"), button("b")], function (this: unknown, event: unknown) {
    seen.push([this, event]);
  });
  expect(added.map(([name, f]) => [name, f === kept])).toEqual([["a", true], ["b", true]]);
  added[1][1].call("b", "e");
  expect(seen).toEqual([20, ["b", "e"]]);
});

test("event methods keep target types and retained callback lifetimes", () => {
  const dir = fixture("event-errors");
  const source = join(dir, "lib.rs");
  for (const [body, diagnostic] of [
    ['button.add_event_listener(Keydown, |e: &KeyboardEvent| { let _ = e.client_x(); }, None);', 'no method named `client_x`'],
    ['button.add_event_listener(Message, |_: &Event| {}, None);', 'Listen<webapi::events::Message>'],
    // A listener's parameter says its event: the union a listener is,
    // a closure or one JS gave, gives rustc no closure signature (rd185).
    ['button.add_event_listener(Click, |e| record(e.client_x() as i32), None);', 'type annotations needed'],
    ['button.add_event_listener(Click, |e: &KeyboardEvent| { let _ = e.key(); }, None);', 'expected closure signature `fn(&\'static webapi::PointerEvent) -> _`'],
    ['let count = 1; button.add_event_listener(Click, |_: &PointerEvent| record(count), None);', 'closure may outlive the current function'],
    ['let count = 1; let listener = listener::<PointerEvent>(|_| record(count)); button.add_event_listener(Click, listener, None);', 'closure may outlive the current function'],
    ['let listener = listener::<KeyboardEvent>(|_| {}); button.add_event_listener(Click, listener, None);', 'is not an `EventListenerOrEventListenerObject`'],
    ['webapi::event_target::add_event_listener(button, Click, |_| {});', 'cannot find function `add_event_listener`'],
    ['button.add_listener(Click, |_| {});', 'no method named `add_listener`'],
    ['button.add_event_listener(Click, |e| webapi::event::prevent_default(e), None);', 'cannot find function `prevent_default`'],
    // Removal finds the same function, so a closure written in place, a new
    // one, never removes anything, by value or borrowed, which rustc makes
    // `'static` as it captures nothing: Scala.js's listener leak, refused.
    // A shared one, `listener(..)`'s, or a named `fn` can (ADR 1021).
    ['button.remove_event_listener(Click, |_: &PointerEvent| {}, None);', 'a closure written in place is a new function, which `remove_event_listener` never finds'],
    ['button.remove_event_listener_named("custom", |_: &Event| {}, None);', 'a closure written in place is a new function'],
    ['let n = 1; button.remove_event_listener_named("custom", move |_: &Event| record(n), None);', 'a closure written in place is a new function'],
    ['let held = String::new(); button.remove_event_listener_named("custom", move |_: &Event| record(held.len() as i32), None);', '`std::string::String: std::marker::Copy` is not satisfied'],
    ['button.remove_event_listener(Click, &|_: &PointerEvent| {}, None);', 'a closure written in place is a new function'],
    ['button.remove_event_listener_named("custom", &|e: &Event| e.prevent_default(), Some(&true));', 'hold the function `webapi::listener(..)` makes, or pass a named `fn`'],
    ['button.remove_event_listener_named("custom", webapi::EventListenerOrEventListenerObject::EventListener(&|_| {}), None);', 'a closure written in place is a new function'],
    ['fn off(button: &HTMLButtonElement, l: impl webapi::IntoAddedEventListener<Event>) { button.remove_event_listener_named("custom", l, None); } off(button, &|_: &Event| {});', 'a closure written in place is a new function'],
    // An event handler property's setter takes the same union, a handler
    // borrowed, which JS keeps: a closure owning what it would drop isn't
    // one, and a JS function of another argument isn't either (rd192).
    ['button.set_onclick(Some(&|e| record(e.client_x() as i32)));', 'type annotations needed'],
    ['button.set_onclick(Some(&|_: &KeyboardEvent| {}));', 'expected closure signature `fn(&\'static webapi::PointerEvent) -> _`'],
    ['let name = String::new(); button.set_onclick(Some(&move |_: &PointerEvent| record(name.len() as i32)));', '`std::string::String: std::marker::Copy` is not satisfied'],
    ["let f: &'static dyn Fn(f64) = &|_| {}; button.set_onclick(Some(&f));", 'is not an event handler of a `webapi::PointerEvent`'],
  ]) {
    writeFileSync(source, `use webapi::{Event, EventTargetExt, HTMLButtonElement, listener, PointerEvent, KeyboardEvent};
use webapi::events::{Click, Keydown, Message};
#[cfg_attr(rust_js, rust_js::link_name = "record")] #[allow(unused_variables)] fn record(n: i32) { unreachable!() }
pub fn wire(button: &HTMLButtonElement) { ${body} }
`);
    const p = Bun.spawnSync([compiler, source, "-o", join(dir, "lib.js"), ...flags], { cwd: root, stderr: "pipe", timeout: 60_000 });
    expect(p.exitCode).not.toBe(0);
    expect(p.stderr.toString()).toContain(diagnostic);
  }
});

test("event listeners dispatch and remove by identity in Chromium", async () => {
  const js = compileEvents();
  const browser = await chromium.launch({ headless: true });
  try {
    for (const [name, expected] of Object.entries({ methods: [17, 17, 5], mutable: [1, 2], sharedMutable: [1, 2], removed: [], duplicate: [17, 17], different: [17, 17], once: [17], aborted: [], capture: [17, 17], captureRemoved: [], named: [1, 1], namedOnce: [1], namedRemoved: [], namedFnRemoved: [], namedCaptureRemoved: [] })) {
      const page = await browser.newPage();
      await page.setContent('<button id="button">Test</button>');
      const result = await page.evaluate(async ({ js, name }) => {
        const values: number[] = [];
        Object.assign(globalThis, { record: (n: number) => values.push(n) });
        const url = URL.createObjectURL(new Blob([js], { type: "text/javascript" }));
        const program = await import(url);
        const button = document.querySelector("button")!;
        program[name](button);
        for (let i = 0; i < 2; i++) {
          const event = name.startsWith("named") ? new Event("custom", { cancelable: true }) : new PointerEvent("click", { clientX: 17 });
          button.dispatchEvent(event);
          if (name === "named" && !event.defaultPrevented) throw Error("preventDefault was lost");
        }
        if (name === "methods") document.dispatchEvent(new KeyboardEvent("keydown", { key: "Enter" }));
        URL.revokeObjectURL(url);
        return values;
      }, { js, name });
      expect(result, name).toEqual(expected);
      await page.close();
    }
  } finally {
    await browser.close();
  }
}, 60_000);

test("a listener JS gave, a function or a handleEvent object, is added and removed as it is in Chromium", async () => {
  const js = compileEvents();
  const browser = await chromium.launch({ headless: true });
  try {
    for (const [name, expected] of Object.entries({ given: [1, 2, 1, 2], givenRemoved: [], givenWrapped: [10, 1, 10, 2, 10, 1, 10, 2], passive: [1, 1] })) {
      const page = await browser.newPage();
      await page.setContent('<button id="button">Test</button>');
      const result = await page.evaluate(async ({ js, name }) => {
        const values: number[] = [];
        Object.assign(globalThis, { record: (n: number) => values.push(n) });
        const url = URL.createObjectURL(new Blob([js], { type: "text/javascript" }));
        const program = await import(url);
        const button = document.querySelector("button")!;
        // A function's `this` is the target; an object's handleEvent's, the object.
        const object = { handleEvent(this: unknown) { values.push(this === object ? 2 : -2); } };
        program[name](button, function (this: unknown) { values.push(this === button ? 1 : -1); });
        program[name](button, object);
        for (let i = 0; i < 2; i++) button.dispatchEvent(new Event("custom"));
        URL.revokeObjectURL(url);
        return values;
      }, { js, name });
      expect(result, name).toEqual(expected);
      await page.close();
    }
  } finally {
    await browser.close();
  }
}, 60_000);

// rd192: an event handler property takes a Rust function or closure, or any
// JS function value as it is, a promise's `resolve` and `reject` too; an
// element's `onerror` is called with the event alone, a window's with what
// the error was, and a window's read and set back is what it was.
test("an event handler is a closure or any JS function, each as it is, in Chromium", async () => {
  const js = compileEvents();
  expect(js).toContain('link._p = new Promise((resolve, reject) => {\n    link.onload = resolve;\n    link.onerror = reject;\n  });');
  expect(js).toContain('image.onerror = (e) => {\n    globalThis.record(e.type === "error" ? 7 : 0);\n  };');
  const browser = await chromium.launch({ headless: true });
  try {
    const page = await browser.newPage();
    await page.setContent('<link id="a"><link id="b"><img id="image">');
    const result = await page.evaluate(async ({ js }) => {
      const values: number[] = [];
      Object.assign(globalThis, { record: (n: number) => values.push(n) });
      const url = URL.createObjectURL(new Blob([js], { type: "text/javascript" }));
      const program = await import(url);
      const [a, b] = document.querySelectorAll("link") as unknown as (HTMLLinkElement & { _p: Promise<Event> })[];
      program.settled(a);
      program.settled(b);
      const load = new Event("load");
      const error = new Event("error");
      a.dispatchEvent(load);
      b.dispatchEvent(error);
      values.push((await a._p) === load ? 1 : 0, await b._p.then(() => 0, (reason) => (reason === error ? 2 : 0)));
      program.errors(document.querySelector("img"));
      document.querySelector("img")!.dispatchEvent(new ErrorEvent("error", { lineno: 8 }));
      const handled = new ErrorEvent("error", { message: "boom", lineno: 9, cancelable: true });
      window.dispatchEvent(handled);
      values.push(handled.defaultPrevented ? 1 : 0);
      const own = () => false;
      window.onerror = own;
      program.restored();
      values.push(window.onerror === own ? 1 : 0);
      URL.revokeObjectURL(url);
      return values;
    }, { js });
    expect(result).toEqual([1, 2, 7, 9, 1, 1]);
    await page.close();
  } finally {
    await browser.close();
  }
}, 60_000);
