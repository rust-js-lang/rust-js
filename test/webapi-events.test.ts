import { beforeAll, expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { chromium } from "playwright";
import { buildWebapi, compiler, fixture, root, run, target } from "./support";

beforeAll(buildWebapi, 300_000);
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
  expect(js).toContain('const listener = (e) => {\n    globalThis.record(e.clientX);\n  };\n  button.addEventListener("click", listener);\n  button.removeEventListener("click", listener);');
  expect(js).toContain('button.removeEventListener("click", listener, true);');
  expect(js).not.toMatch(/unreachable|\.add_event_listener|\.client_x|\.prevent_default/);
});

test("event methods keep target types and retained callback lifetimes", () => {
  const dir = fixture("event-errors");
  const source = join(dir, "lib.rs");
  for (const [body, diagnostic] of [
    ['button.add_event_listener(Keydown, |e| { let _ = e.client_x(); });', 'no method named `client_x`'],
    ['button.add_event_listener(Message, |_| {});', 'Listen<webapi::events::Message>'],
    ['let count = 1; button.add_event_listener(Click, |_| record(count));', 'closure may outlive the current function'],
    ['let count = 1; let listener = listener::<PointerEvent>(|_| record(count)); button.add_event_listener(Click, listener);', 'closure may outlive the current function'],
    ['let listener = listener::<KeyboardEvent>(|_| {}); button.add_event_listener(Click, listener);', 'expected an `FnMut(&webapi::PointerEvent)` closure'],
    ['webapi::event_target::add_event_listener(button, Click, |_| {});', 'cannot find function `add_event_listener`'],
    ['button.add_listener(Click, |_| {});', 'no method named `add_listener`'],
    ['button.add_event_listener(Click, |e| webapi::event::prevent_default(e));', 'cannot find function `prevent_default`'],
    // Removal finds the same function, so a closure made for it, a new one,
    // never removes anything: only a shared callback, `listener(..)`'s, can.
    ['button.remove_event_listener(Click, |_| {});', 'mismatched types'],
    ['button.remove_event_listener_named("custom", |_| {});', 'mismatched types'],
  ]) {
    writeFileSync(source, `use webapi::{EventTargetExt, HTMLButtonElement, listener, PointerEvent, KeyboardEvent};
use webapi::events::{Click, Keydown, Message};
unsafe extern "Rust" { safe fn record(n: i32); }
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
    for (const [name, expected] of Object.entries({ methods: [17, 17, 5], mutable: [1, 2], shared_mutable: [1, 2], removed: [], duplicate: [17, 17], different: [17, 17], once: [17], aborted: [], capture: [17, 17], capture_removed: [], named: [1, 1], named_once: [1], named_removed: [], named_capture_removed: [] })) {
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
