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
  expect(js).not.toMatch(/unreachable|\.add_event_listener|\.client_x|\.prevent_default/);
  // A listener JS gave is passed as it is, and called as JS would (rd185).
  expect(js).toContain('export function givenRemoved(button, given) {\n  button.addEventListener("custom", given);\n  button.removeEventListener("custom", given);\n}');
  expect(js).toContain('if (typeof given === "function") {\n      given.call(this, event);\n    } else {\n      given.handleEvent(event);\n    }');
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
    // Removal finds the same function, so a closure made for it, a new one,
    // never removes anything: only a shared callback, `listener(..)`'s, can.
    ['button.remove_event_listener(Click, |_: &PointerEvent| {}, None);', 'is not a listener added before'],
    ['button.remove_event_listener_named("custom", |_: &Event| {}, None);', 'is not a listener added before'],
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
    for (const [name, expected] of Object.entries({ methods: [17, 17, 5], mutable: [1, 2], sharedMutable: [1, 2], removed: [], duplicate: [17, 17], different: [17, 17], once: [17], aborted: [], capture: [17, 17], captureRemoved: [], named: [1, 1], namedOnce: [1], namedRemoved: [], namedCaptureRemoved: [] })) {
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
    for (const [name, expected] of Object.entries({ given: [1, 2, 1, 2], givenRemoved: [], givenWrapped: [10, 1, 10, 2, 10, 1, 10, 2] })) {
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
