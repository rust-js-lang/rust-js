// A function value passed on as it is: a `dyn Fn` parameter each call
// fills with the crate's function, and a binding that reads only its
// parameters, its function as a value. Split from lowering-passed-on.test.ts
// by concept (#1334).

import { beforeAll, expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { buildReact, compiler, fixture, run, target } from "./support";

beforeAll(buildReact, 600_000);

const externs = ["--", "--extern", `webapi=${join(target, "libwebapi.rmeta")}`, "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];

// rd616: a `dyn Fn` parameter of a function only the crate's calls reach,
// which each call fills with a function that reads no `this` and nothing
// it's given more, is one, so a closure that ignores what it's given and
// calls it is that parameter, a listener's too: React Flight's Node
// client's `stream.on('end', onEnd)`. Not one a call fills with a
// `with_this` function unbound, which reads the `this` a listener is
// called with.
test("a dyn Fn parameter each call fills with the crate's function is that function", async () => {
  const dir = fixture("own-callback-params");
  writeFileSync(
    join(dir, "lib.rs"),
    `use std::cell::Cell;
use std::rc::Rc;

use js::function;
use webapi::{EventTarget, EventTargetExt};

pub struct State {
    pub ended: Cell<f64>,
}

unsafe impl js::Defined for State {}

thread_local! {
    static ON_END: &'static dyn Fn() = function::with_this(|this, ()| {
        // Safety: it's a state's listener.
        let state: &State = unsafe { js::cast(this) };
        state.ended.set(state.ended.get() + 1.0);
    });
}

fn close(state: &State) {
    state.ended.set(state.ended.get() + 10.0);
}

fn start_reading(target: &EventTarget, on_end: Rc<dyn Fn()>) {
    target.add_event_listener_named("end", move |_| on_end(), None);
}

fn listen_unbound(target: &EventTarget, on_end: &'static dyn Fn()) {
    target.add_event_listener_named("end", move |_| on_end(), None);
}

pub fn read(a: &EventTarget, b: &EventTarget, state: &'static State) {
    let handle_end: Rc<dyn Fn()> = {
        let ended = Rc::new(Cell::new(0));
        Rc::new(move || {
            ended.set(ended.get() + 1);
            if ended.get() == 2 {
                close(state);
            }
        })
    };
    start_reading(a, Rc::clone(&handle_end));
    start_reading(b, handle_end);
}

pub fn unbound(target: &EventTarget) {
    listen_unbound(target, ON_END.with(|f| *f));
}

fn listen_made(target: &EventTarget, on_end: &'static dyn Fn()) {
    target.add_event_listener_named("end", move |_| on_end(), None);
}

thread_local! {
    static GIVEN: Cell<Option<&'static js::Unknown>> = const { Cell::new(None) };
}

/// How many the last count was given.
pub fn given() -> Option<&'static js::Unknown> {
    GIVEN.get()
}

fn count_given() {
    GIVEN.set(js::get(js::arguments(), "length"));
}

fn listen_counting(target: &EventTarget, on_end: &'static dyn Fn()) {
    target.add_event_listener_named("end", move |_| on_end(), None);
}

fn listen_counting_with_this(target: &EventTarget, on_end: &'static dyn Fn()) {
    target.add_event_listener_named("end", move |_| on_end(), None);
}

pub fn counting(target: &EventTarget, made: &EventTarget) {
    listen_counting(target, &count_given);
    listen_counting_with_this(
        made,
        function::with_this(|_, ()| GIVEN.set(js::get(js::arguments(), "length"))),
    );
}

pub fn made(target: &EventTarget) {
    listen_made(
        target,
        function::with_this(|this, ()| {
            // Safety: it's a state's listener.
            let state: &State = unsafe { js::cast(this) };
            state.ended.set(state.ended.get() + 2.0);
        }),
    );
}
`,
  );
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...externs]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain(
    'function startReading(target, onEnd) {\n  target.addEventListener("end", onEnd);\n}\n',
  );
  expect(js).toContain(
    'function listenUnbound(target, onEnd) {\n  target.addEventListener("end", () => {\n    onEnd();\n  });\n}\n',
  );
  expect(js).toContain(
    'function listenMade(target, onEnd) {\n  target.addEventListener("end", () => {\n    onEnd();\n  });\n}\n',
  );
  const lib = await import(join(dir, "lib.js"));
  const state = { ended: 0 };
  const [a, b] = [new EventTarget(), new EventTarget()];
  lib.read(a, b, state);
  a.dispatchEvent(new Event("end"));
  expect(state.ended).toBe(0);
  b.dispatchEvent(new Event("end"));
  expect(state.ended).toBe(10);
  // One that reads `js::arguments()` counts what it's given, none, where
  // the listener would give it the event.
  const [counted, made] = [new EventTarget(), new EventTarget()];
  lib.counting(counted, made);
  counted.dispatchEvent(new Event("end"));
  expect(lib.given()).toBe(0);
  made.dispatchEvent(new Event("end"));
  expect(lib.given()).toBe(0);
});

// rd648, the user's decision (2026-11-03): a binding that says it reads
// only its parameters, `#[rust_js::reads_only_its_parameters]`, is its
// function as a value, React's `clones.map(measureClonedInstance)`. One
// that doesn't keeps its arrow, as `map` gives the index and the array too,
// which a JS function may read, `parseInt`'s radix.
test("a binding that reads only its parameters is its function as a value", async () => {
  const dir = fixture("binding-reads-only-its-parameters");
  writeFileSync(join(dir, "lib.rs"), `#[cfg_attr(rust_js, rust_js::link_name = "measure")]
#[cfg_attr(rust_js, rust_js::reads_only_its_parameters)]
fn measure(item: u32) -> u32 {
    let _ = item;
    unreachable!()
}

#[cfg_attr(rust_js, rust_js::link_name = "parseItem")]
fn parse_item(item: u32) -> u32 {
    let _ = item;
    unreachable!()
}

pub fn measured(items: &[u32]) -> Vec<u32> {
    items.iter().copied().map(measure).collect()
}

pub fn parsed(items: &[u32]) -> Vec<u32> {
    items.iter().copied().map(parse_item).collect()
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...externs]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("return items.map(measure);");
  expect(js).toContain("return items.map((item) => parseItem(item));");
  Object.assign(globalThis, {
    measure: (item: number) => item * 10,
    parseItem: (item: number, index?: number) => item + (index === undefined ? 0 : 100),
  });
  const lib = await import(join(dir, "lib.js"));
  expect(lib.measured([1, 2])).toEqual([10, 20]);
  expect(lib.parsed([1, 2])).toEqual([1, 2]);
});

test("a binding's mark that it reads only its parameters is refused where it can't be said", () => {
  const dir = fixture("binding-reads-only-its-parameters-refused");
  writeFileSync(join(dir, "lib.rs"), `pub struct Node(js::JsObject);

impl Node {
    #[cfg_attr(rust_js, rust_js::link_name = "measure")]
    #[cfg_attr(rust_js, rust_js::reads_only_its_parameters)]
    pub fn measure(&self, depth: u32) -> u32 {
        let _ = depth;
        unreachable!()
    }
}

#[cfg_attr(rust_js, rust_js::reads_only_its_parameters)]
pub fn own(n: u32) -> u32 {
    n
}

#[cfg_attr(rust_js, rust_js::link_name = "console.log")]
#[cfg_attr(rust_js, rust_js::variadic)]
#[cfg_attr(rust_js, rust_js::reads_only_its_parameters)]
pub fn log(data: impl js::Rest<u32>) {
    let _ = data;
    unreachable!()
}
`);
  const p = Bun.spawnSync([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...externs], { stderr: "pipe" });
  expect(p.exitCode).not.toBe(0);
  const stderr = p.stderr.toString();
  expect(stderr).toContain("a method reads the value it's called on");
  expect(stderr).toContain("this one's body is the program's");
  expect(stderr).toContain("a binding of rest arguments reads all JS gives it");
});
