// The timers, `js::set_timeout` and `js::set_interval`, every JS runtime's
// globals (ADR 0102), are the one way a program sets one (ADR 1020): a
// timeout's handler is TypeScript's `TimerHandler`, `string | Function`, a
// function taken as it is, a closure, a `fn`, a boxed or borrowed `dyn Fn`,
// or text of code; each is a value a `thread_local!` holds where the host
// has it, as React DOM's `scheduleTimeout` is; and webapi's
// `window.set_timeout`, a second way, is gone, an error naming the js
// crate's.

import { beforeAll, expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";

import { runSync } from "./child";
import { buildReact, compiler, fixture, root, target } from "./support";

beforeAll(buildReact, 600_000);

const externs = ["--", "--extern", `webapi=${join(target, "libwebapi.rmeta")}`, "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];

/** `source` compiled: whether it was, what rustc and rust-js said, its JS. */
function compiled(name: string, source: string): { code: number | null; stderr: string; dir: string; js: string } {
  const dir = fixture(name);
  writeFileSync(join(dir, "lib.rs"), source);
  const p = runSync([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...externs], root, 120_000);
  const js = p.code === 0 ? readFileSync(join(dir, "lib.js"), "utf8") : "";
  return { code: p.code, stderr: p.stderr, dir, js };
}

test("a timer's handler is a function as it is, or text of code", async () => {
  const { code, stderr, dir, js } = compiled("timer-handlers", `fn ping() {
    println!("ping");
}

pub fn later(task: Box<dyn FnOnce()>) -> &'static js::TimeoutId {
    js::set_timeout(task, None)
}

pub fn shared(task: &'static dyn Fn()) {
    js::set_timeout(task, Some(1.0));
}

pub fn pinged() {
    js::set_timeout(ping, None);
}

pub fn greeted(name: String) {
    js::set_timeout(move || println!("hello, {name}"), None);
}

pub fn boxed(name: String) {
    js::set_timeout(Box::new(move || println!("hi, {name}")), None);
}

pub fn run(code: &str) {
    js::set_timeout(code, None);
}

pub fn every(tick: Box<dyn FnMut()>) -> &'static js::IntervalId {
    js::set_interval(tick, Some(1.0))
}

pub fn stop(id: &js::IntervalId) {
    js::clear_interval(Some(id));
}
`);
  expect([code, stderr]).toEqual([0, ""]);
  expect(js).toContain("export function later(task) {\n  return setTimeout(task);\n}");
  expect(js).toContain("export function shared(task) {\n  setTimeout(task, 1);\n}");
  expect(js).toContain("export function pinged() {\n  setTimeout(ping);\n}");
  expect(js).toContain("export function greeted(name) {\n  setTimeout(() => {\n    console.log(`hello, ${name}`);\n  });\n}");
  expect(js).toContain("export function boxed(name) {\n  setTimeout(() => {\n    console.log(`hi, ${name}`);\n  });\n}");
  expect(js).toContain("export function run(code) {\n  setTimeout(code);\n}");
  expect(js).toContain("export function every(tick) {\n  return setInterval(tick, 1);\n}");
  const lib = await import(join(dir, "lib.js"));
  await new Promise<void>((resolve) => lib.later(() => resolve()));
  await new Promise<void>((resolve) => lib.shared(() => resolve()));
  let ticks = 0;
  const counted = lib.every(() => {
    ticks += 1;
    if (ticks === 2) lib.stop(counted);
  });
  await new Promise((resolve) => setTimeout(resolve, 40));
  expect(ticks).toBe(2);
  // Text is a browser's: Node and Bun throw a TypeError of it, as JS does
  // there. What each is given is the text, as it is.
  const given: unknown[] = [];
  const timeout = globalThis.setTimeout;
  Object.assign(globalThis, { setTimeout: (h: unknown) => given.push(h) });
  try {
    lib.run("tick()");
  } finally {
    Object.assign(globalThis, { setTimeout: timeout });
  }
  expect(given).toEqual(["tick()"]);
});

// React DOM's `scheduleTimeout`, `cancelTimeout` and `scheduleMicrotask`,
// each the host's function where it has one: a generic binding is asked of
// by any of its instances, `set_timeout::<fn()>`, as `prevent_extensions`'s
// is (ADR 0942), and held as the instance its type says.
test("a timer and queueMicrotask are held as values where the host has them", () => {
  const { code, stderr, js } = compiled("timer-values", `type SetTimeout = fn(Box<dyn FnOnce()>, Option<f64>) -> &'static js::TimeoutId;

thread_local! {
    pub static SCHEDULE_TIMEOUT: Option<SetTimeout> = if js::is_function(js::set_timeout::<fn()>) {
        Some(js::set_timeout)
    } else {
        None
    };
    pub static CANCEL_TIMEOUT: Option<fn(Option<&js::TimeoutId>)> = if js::is_function(js::clear_timeout) {
        Some(js::clear_timeout)
    } else {
        None
    };
    pub static SCHEDULE_MICROTASK: Option<fn(Box<dyn FnOnce()>)> = if js::is_function(webapi::global::queue_microtask) {
        Some(webapi::global::queue_microtask)
    } else {
        None
    };
}

pub fn later(task: Box<dyn FnOnce()>) {
    if let Some(schedule) = SCHEDULE_TIMEOUT.with(|s| *s) {
        schedule(task, Some(5.0));
    }
}
`);
  expect([code, stderr]).toEqual([0, ""]);
  expect(js).toContain(
    'export const scheduleTimeout = typeof setTimeout === "function" ? setTimeout : undefined;\n' +
      'export const cancelTimeout = typeof clearTimeout === "function" ? clearTimeout : undefined;\n' +
      'export const scheduleMicrotask = typeof queueMicrotask === "function" ? queueMicrotask : undefined;\n',
  );
  expect(js).toContain("    scheduleTimeout(task, 5);\n");
});

// One way: a window's timers were a second, of text only (ADR 1020).
test("webapi's window.set_timeout is gone, an error naming js::set_timeout", () => {
  for (const call of [`webapi::window.set_timeout("tick()", None)`, `webapi::window.set_timeout(|| (), None)`, `webapi::window.clear_timeout(Some(1.0))`, `webapi::window.set_interval("tick()", None, ())`]) {
    const { code, stderr } = compiled("window-timers-gone", `pub fn later() {
    ${call};
}
`);
    expect([code === 0, stderr]).toEqual([false, expect.stringContaining("a window's timers are gone: a timer is the js crate's global, `js::set_timeout(handler, ms)`")]);
  }
});

// A closure rustc infers `Fn`, as it only borrows what it took, is called
// as Rust's `call_once` calls it, the timer's one call: what it took is
// dropped as that call ends, after its body's own (ADR 1020).
test("a timer's closure that only borrows what it owns drops it after its one call", async () => {
  const { code, stderr, dir, js } = compiled("timer-drops", `pub struct Lease(pub &'static str);

impl Drop for Lease {
    fn drop(&mut self) {
        println!("released {}", self.0);
    }
}

pub fn hold(lease: Lease) {
    js::set_timeout(
        move || {
            let held = &lease;
            let note = Lease("note");
            println!("holding {} {}", held.0, note.0);
        },
        Some(1.0),
    );
}

pub fn lease(name: &'static str) -> Lease {
    Lease(name)
}
`);
  expect([code, stderr]).toEqual([0, ""]);
  expect(js).toContain("setTimeout(() => {\n    const held = lease;\n");
  const lib = await import(join(dir, "lib.js"));
  const logged: string[] = [];
  const log = console.log;
  console.log = (line: string) => logged.push(line);
  try {
    lib.hold(lib.lease("desk"));
    expect(logged).toEqual([]);
    await new Promise((resolve) => setTimeout(resolve, 20));
  } finally {
    console.log = log;
  }
  expect(logged).toEqual(["holding desk note", "released note", "released desk"]);
});

// React's SchedulerPostTask holds the window's own `setTimeout` at load,
// `const setTimeout = window.setTimeout;`, which its test replaces with a
// `window` of its own: read as a value alone, `js::method((window,
// Window::set_timeout))`, it's the window's; a call stays the js crate's
// (ADR 1020, amended, rd314, the user's decision).
test("a window's setTimeout is read as a value, the window's own", async () => {
  const { code, stderr, dir, js } = compiled("window-set-timeout-value", `use webapi::{Window, window};

thread_local! {
    static SET_TIMEOUT: &'static dyn Fn(Box<dyn FnOnce()>, Option<f64>) -> &'static js::TimeoutId =
        js::method((window, Window::set_timeout));
}

pub fn report(message: String) {
    SET_TIMEOUT.with(|set_timeout| set_timeout(Box::new(move || println!("{message}")), None));
}
`);
  expect([code, stderr]).toEqual([0, ""]);
  expect(js).toContain("window.setTimeout");
  const handlers: (() => void)[] = [];
  (globalThis as any).window = { setTimeout: (handler: () => void) => handlers.push(handler) };
  try {
    const lib = await import(join(dir, "lib.js"));
    lib.report("Oops!");
    expect(handlers.length).toBe(1);
  } finally {
    delete (globalThis as any).window;
  }
});

