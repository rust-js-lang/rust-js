// A crate's modules as JS files: each beside its source, importing what
// it uses, re-exporting what it says, running what it runs when it's
// loaded. Each test is a crate of its own, compiled and run.

import { beforeAll, expect, test } from "bun:test";
import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { buildReact, compiler, fixture, root, run, target } from "./support";

beforeAll(buildReact, 600_000);

// ADR 0270: a thread-local only read and set, in its own module, is the
// module's variable, a `let` if it's set, as react.dev's errors page caches
// the codes it fetched: nothing shares its cell, so it needs no `{ value }`.
// One whose cell `with` hands out keeps it; a public one, and one another
// module reads, is its `export let` (ADR 0270, amended).
test("a thread-local only read and set is the module's let", async () => {
  const dir = fixture("module-let");
  writeFileSync(join(dir, "lib.rs"), `use std::cell::{Cell, RefCell};

thread_local! {
    static COUNT: Cell<u32> = Cell::new(0);
    static LOG: RefCell<Vec<String>> = RefCell::new(Vec::new());
    pub static SHARED: Cell<u32> = Cell::new(0);
    static SEEN: Cell<Option<u32>> = const { Cell::new(None) };
}

pub fn seen(n: u32) -> u32 {
    if SEEN.get().is_none() {
        SEEN.set(Some(n));
    }
    SEEN.get().unwrap_or(0)
}

pub fn unseen(n: u32) -> u32 {
    let mut other = None;
    if SEEN.get().is_none() {
        other = Some(n);
    }
    other.unwrap_or(0)
}

pub fn bump() -> u32 {
    COUNT.set(COUNT.get() + 1);
    COUNT.get()
}

pub fn note(line: &str) -> usize {
    LOG.with_borrow_mut(|log| log.push(line.to_string()));
    LOG.with_borrow(|log| log.len())
}

pub fn lines_after(n: usize) -> usize {
    n + LOG.with_borrow(|log| log.len())
}

pub fn shared() -> u32 {
    SHARED.set(SHARED.get() + 2);
    SHARED.get()
}

mod counter;

pub fn ticked() -> u32 {
    counter::tick();
    counter::TICKS.get()
}
`);
  writeFileSync(join(dir, "counter.rs"), `use std::cell::Cell;

thread_local! {
    pub(crate) static TICKS: Cell<u32> = Cell::new(0);
}

pub fn tick() {
    TICKS.set(TICKS.get() + 1);
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(join(dir, "mir"), "lib.js")]);
  expect(readFileSync(join(join(dir, "mir"), "counter.js"), "utf8")).toContain("export let ticks = 0;");
  const js = readFileSync(join(join(dir, "mir"), "lib.js"), "utf8");
  expect(js).toContain("let count = 0;");
  expect(js).toContain("const log = [];");
  expect(js).toContain("export let SHARED = 0;");
  expect(js).toContain("count = (count + 1) >>> 0;");
  // Its borrow's closure where it's read, not first: the closure's body.
  expect(js).toContain("return (n + log.length) >>> 0;");
  expect(js).not.toContain("count.value");
  expect(js).not.toContain("log.value");
  expect(js).toContain("  return log.length;\n");
  // One made by a `const { .. }` is too, by its own name.
  expect(js).not.toContain("__RUST_STD_INTERNAL_INIT");
  expect(js).toContain("let SEEN;\n");
  expect(js).toContain("  SEEN ??= n;\n");
  const { seen } = await import(join(join(dir, "mir"), "lib.js"));
  expect([seen(3), seen(4)]).toEqual([3, 3]);
  const { unseen } = await import(join(join(dir, "mir"), "lib.js"));
  expect(unseen(5)).toBe(0);
  const { bump, note, shared, ticked } = await import(join(join(dir, "mir"), "lib.js"));
  expect([bump(), bump(), note("a"), note("b"), shared(), shared(), ticked()]).toEqual([1, 2, 1, 2, 2, 4, 1]);
});

// A thread-local `RefCell` of a number that `with_borrow_mut` changes is its
// module's `let` too: its closure, given the cell's box to change, reads and
// sets it by `value` alone, in place, as the variable (ADR 0270, amended).
test("MIR's thread-local number changed by with_borrow_mut is its let", async () => {
  const dir = fixture("module-let-number");
  writeFileSync(join(dir, "lib.rs"), `use std::cell::RefCell;

thread_local! {
    static N: RefCell<u32> = RefCell::new(0);
}

pub fn bump() -> u32 {
    N.with_borrow_mut(|n| *n += 1);
    N.with_borrow(|n| *n)
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")], 600_000);
  expect(readFileSync(join(dir, "mir.js"), "utf8")).toContain("let n = 0;\n\nexport function bump() {\n  n = (n + 1) >>> 0;\n  return n;\n}\n");
  const { bump } = await import(join(dir, "mir.js"));
  expect([bump(), bump()]).toEqual([1, 2]);
});

// ADR 0273: a `#[path]` module's JS is beside its file, `x/[y].rs`'s
// `x/[y].js`, as Next.js finds a page, and what it imports is seen from
// there: the crate's root, and a file beside the root's.
test("a #[path] module's JS is beside its file", async () => {
  const dir = fixture("path-module");
  mkdirSync(join(dir, "x"));
  writeFileSync(join(dir, "lib.rs"), `#[path = "x/[y].rs"]
pub mod y;

pub fn top() -> u32 {
    y::f() + 1
}

pub fn base() -> u32 {
    40
}
`);
  writeFileSync(join(dir, "x/[y].rs"), `unsafe extern "Rust" {
    #[link_name = "./data.js#default"]
    safe static data: u32;
}

pub fn f() -> u32 {
    crate::base() + data
}
`);
  writeFileSync(join(dir, "data.js"), "export default 1;\n");
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  expect(readFileSync(join(dir, "lib.js"), "utf8")).toContain('import { f } from "./x/[y].js";');
  const y = readFileSync(join(dir, "x/[y].js"), "utf8");
  expect([y.includes('import { base } from "../lib.js";'), y.includes('import data from "../data.js";')]).toEqual([true, true]);
  const { top } = await import(join(dir, "lib.js"));
  expect(top()).toBe(42);
});

// So is the JS of a module in a `#[path]` directory, `sandpack-rsc/bridge.rs`
// of `#[path = "sandpack-rsc"] mod sandpack_rsc { mod bridge; }`, which no
// module path can name.
test("a module in a #[path] directory has its JS beside its file", async () => {
  const dir = fixture("path-directory");
  mkdirSync(join(dir, "sandpack-rsc"));
  writeFileSync(join(dir, "lib.rs"), `#[path = "sandpack-rsc"]
pub mod sandpack_rsc {
    pub mod bridge;

    pub fn one() -> u32 {
        1
    }
}

pub fn top() -> u32 {
    sandpack_rsc::bridge::f() + sandpack_rsc::one()
}

pub fn base() -> u32 {
    40
}
`);
  writeFileSync(join(dir, "sandpack-rsc/bridge.rs"), `pub fn f() -> u32 {
    crate::base() + 1
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  expect(readFileSync(join(dir, "lib.js"), "utf8")).toContain('import { f } from "./sandpack-rsc/bridge.js";');
  expect(readFileSync(join(dir, "sandpack-rsc/bridge.js"), "utf8")).toContain('import { base } from "../lib.js";');
  const { top } = await import(join(dir, "lib.js"));
  expect(top()).toBe(42);
});

test("methods across modules and in a thread-local, by their names in JS", async () => {
  const { fixture, compiler } = await import("./support");
  const { writeFileSync } = await import("node:fs");
  const dir = fixture("methods");
  writeFileSync(join(dir, "lib.rs"), `use std::cell::Cell;

mod shapes;

thread_local! {
    static SIDE: Cell<u32> = Cell::new(shapes::Square::new(3).side_length());
}

pub fn area_of(side: u32) -> u32 {
    shapes::Square::new(side).area()
}

pub fn first_side() -> u32 {
    SIDE.get()
}
`);
  writeFileSync(join(dir, "shapes.rs"), `pub struct Square {
    pub side: u32,
}

impl Square {
    pub fn new(side: u32) -> Square {
        Square { side }
    }

    pub fn side_length(&self) -> u32 {
        self.side
    }

    pub fn area(&self) -> u32 {
        self.side_length() * self.side_length()
    }
}

/// Only this module uses it, so its object isn't exported.
struct Tally(u32);

impl Tally {
    fn doubled(&self) -> u32 {
        self.0 * 2
    }
}

pub fn tally_of_four() -> u32 {
    Tally(4).doubled()
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(join(dir, "mir"), "lib.js")]);
  const lib = await Bun.file(join(join(dir, "mir"), "lib.js")).text();
  const shapes = await Bun.file(join(join(dir, "mir"), "shapes.js")).text();
  // Its initializer runs the crate's code: made where it's first used, as
  // Rust makes it (ADR 0037, amended).
  expect(lib).toContain("const side = $local(() => ({ value: Square.sideLength(Square.new(3)) }));");
  expect(lib).toContain("  return side().value;");
  expect(lib).toContain("  return Square.area(Square.new(side));");
  expect(shapes).toContain("export const Square = {\n  new(side) {");
  expect(shapes).toContain("  area(square) {\n    return Math.imul(Square.sideLength(square), Square.sideLength(square)) >>> 0;");
  expect(shapes).toContain("\nconst Tally = {\n  doubled(tally) {\n    return (tally[0] * 2) >>> 0;\n  },\n};\n\nexport function tallyOfFour() {");
  // Laid out on lines of its own, a lone method still maps back to its Rust.
  const { decodeMappings, lookup } = await import("./sourcemap.ts");
  const segments = decodeMappings((await Bun.file(join(join(dir, "mir"), "shapes.js.map")).json()).mappings);
  const jsLines = shapes.split("\n");
  const rsLines = (await Bun.file(join(dir, "shapes.rs")).text()).split("\n");
  for (const [jsText, rustText] of [["return (tally[0] * 2)", "self.0 * 2"], ["(tally[0] * 2)", "self.0 * 2"], ["doubled(tally) {", "doubled(&self)"]]) {
    const line = jsLines.findIndex((l) => l.includes(jsText));
    const hit = lookup(segments, line, jsLines[line].indexOf(jsText));
    expect([jsText, hit && rsLines[hit.srcLine].slice(hit.srcCol).startsWith(rustText)]).toEqual([jsText, true]);
  }
  const module = await import(join(join(dir, "mir"), "lib.js"));
  expect(module.areaOf(4)).toBe(16);
  expect(module.firstSide()).toBe(3);
  expect((await import(join(join(dir, "mir"), "shapes.js"))).tallyOfFour()).toBe(8);
});

// ADR 0019: a module is imported when anything of it is used, a `const` or
// a `thread_local!` too; a module that isn't used takes no name from locals.
test("imports follow what's used: functions, consts and thread-locals", async () => {
  const { fixture, compiler } = await import("./support");
  const { writeFileSync } = await import("node:fs");
  const dir = fixture("imports");
  writeFileSync(join(dir, "lib.rs"), `mod editor;
mod helpers;
mod util;

pub fn sized() -> u32 {
    util::SIZE + util::COUNT.get() + helpers::one()
}

pub fn doubled(editor: u32) -> u32 {
    editor * 2
}
`);
  writeFileSync(join(dir, "util.rs"), `use std::cell::Cell;

pub const SIZE: u32 = 4;

thread_local! {
    pub static COUNT: Cell<u32> = Cell::new(3);
}
`);
  writeFileSync(join(dir, "helpers.rs"), "pub fn one() -> u32 {\n    1\n}\n");
  writeFileSync(join(dir, "editor.rs"), "pub fn open() -> u32 {\n    1\n}\n");
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const lib = await Bun.file(join(dir, "lib.js")).text();
  expect(lib).toContain('import { SIZE, count } from "./util.js";');
  expect(lib).not.toContain("editor.js");
  expect(lib).toContain("export function doubled(editor) {");
  const module = await import(join(dir, "lib.js"));
  expect(module.sized()).toBe(8);
  expect(module.doubled(3)).toBe(6);
});

// `js::import!("./app.css");` is `import "./app.css";` in its module's JS, at
// the root and in a module (ADR 0110), where `#![rust_js::import]` was: stable
// Rust has no inner attribute of a tool. Its `const _` writes nothing.
test("js::import! imports a module where it's written, and writes nothing of its own", () => {
  const dir = fixture("js-import");
  writeFileSync(join(dir, "lib.rs"), 'js::import!("./app.css");\npub mod panel {\n    js::import!("./panel.css");\n    pub fn f() -> u32 {\n        1\n    }\n}\n#[rust_js::import = "./direct.css"]\nconst _: () = ();\n');
  buildReact();
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  const top = readFileSync(join(dir, "lib.js"), "utf8"), panel = readFileSync(join(dir, "panel.js"), "utf8");
  expect([top.includes('import "./app.css";'), top.includes('import "./direct.css";'), panel.includes('import "./panel.css";')]).toEqual([true, true, true]);
  expect(top).not.toContain("const _");
  expect(panel).not.toContain("app.css");
});

// A module gets a file for what it has: a root of `mod`s alone, an app's
// list of its files, has none, and a module of a `js::import!` alone has
// one, its import (ADR 0382).
test("a root of modules alone writes no JS, and a module's `js::import!` alone does", () => {
  const dir = fixture("modules-root");
  writeFileSync(join(dir, "lib.rs"), "pub mod panel {\n    pub fn f() -> u32 {\n        1\n    }\n}\npub mod styles {\n    js::import!(\"./styles.css\");\n}\n");
  buildReact();
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  expect([existsSync(join(dir, "lib.js")), existsSync(join(dir, "panel.js"))]).toEqual([false, true]);
  expect(readFileSync(join(dir, "styles.js"), "utf8")).toContain('import "./styles.css";');
});

// `js::directive!("use client");` is the module's directive, its first
// statement, and `js::export_default!(page);` its default export, of a
// function that stays named for the crate's other modules (ADR 0192), and
// exported as the default alone (ADR 0611, amended): what a Next.js route
// is, `app/page.jsx`.
test("js::directive! and js::export_default! make a module a Next.js route", async () => {
  const dir = fixture("js-route");
  writeFileSync(join(dir, "lib.rs"), 'js::directive!("use client");\npub fn page() -> u32 {\n    about::about() + 1\n}\njs::export_default!(page);\npub mod about {\n    pub fn about() -> u32 {\n        2\n    }\n    js::export_default!(about);\n}\npub mod generic {\n    pub fn first<T>(items: Vec<T>) -> Option<T> {\n        items.into_iter().next()\n    }\n    js::export_default!(first);\n}\n');
  buildReact();
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  const top = readFileSync(join(dir, "lib.js"), "utf8"), about = readFileSync(join(dir, "about.js"), "utf8");
  const statements = (js: string) => js.split("\n").filter((line) => line && !line.startsWith("//"));
  expect(statements(top)[0]).toBe('"use client";');
  expect(statements(about)[0]).not.toBe('"use client";');
  expect(top).toContain("export default page;");
  expect(top).not.toContain("const _");
  const [lib, sub] = [await import(join(dir, "lib.js")), await import(join(dir, "about.js"))];
  // The default alone, `pub` as it is (ADR 0611, amended).
  expect([lib.default(), lib.page, sub.default()]).toEqual([3, undefined, 2]);
  expect(top).not.toContain("export function page");
  // A generic function's too, which Rust can't name as a value unless it's
  // given its types.
  expect((await import(join(dir, "generic.js"))).default([7, 8])).toBe(7);
});

// `js::directive!` in a function's body, or a closure's, is that body's
// directive, its first statement, as Next.js's inline Server Actions
// and `"use cache"` functions have one (ADR 0349): not the module's. One in
// an `async` block, which no function is, is an error.
test("js::directive! in a body is the body's directive", async () => {
  const dir = fixture("js-body-directive");
  writeFileSync(join(dir, "lib.rs"), `pub async fn cached(n: u32) -> u32 {
    js::directive!("use cache");
    n + 1
}

pub fn plain() -> u32 {
    js::directive!("use strict");
    2
}

pub fn action(step: u32) -> impl AsyncFn(u32) -> u32 {
    async move |n: u32| {
        js::directive!("use server");
        n + step
    }
}

pub fn handler(step: u32) -> impl Fn(u32) -> u32 {
    move |n: u32| {
        js::directive!("use strict");
        n + step
    }
}

pub fn spawned() -> impl Fn() {
    || {
        js::directive!("use server");
        js::spawn(Box::new(async {}))
    }
}
`);
  buildReact();
  const extern = ["--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];
  writeFileSync(join(dir, "block.rs"), `pub async fn later() -> u32 {
    async {
        js::directive!("use cache");
        1
    }
    .await
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js"), ...extern]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  const statements = js.split("\n").filter((line) => line && !line.startsWith("//"));
  expect(statements[0]).toBe("export async function cached(n) {");
  expect(js).toContain('export async function cached(n) {\n  "use cache";\n  return (n + 1) >>> 0;\n}');
  expect(js).toContain('export function plain() {\n  "use strict";\n  return 2;\n}');
  expect(js).toContain('return async (n) => {\n    "use server";\n    return (n + step) >>> 0;\n  };');
  expect(js).toContain('return (n) => {\n    "use strict";\n    return (n + step) >>> 0;\n  };');
  expect(js).toContain('return () => {\n    "use server";\n    (async () => {})();\n  };');
  const lib = await import(join(dir, "mir.js"));
  expect([await lib.cached(1), lib.plain(), await lib.action(2)(3)]).toEqual([2, 2, 5]);
  const block = Bun.spawnSync([compiler, join(dir, "block.rs"), "-o", join(dir, "block.js"), ...extern], {
    cwd: root,
    stderr: "pipe",
  });
  expect(block.exitCode).not.toBe(0);
  expect(block.stderr.toString()).toContain("a directive is a module's, a function's or a closure's");
});

test("a namespace import is named as the module of its bindings", async () => {
  const dir = fixture("namespace-module");
  writeFileSync(join(dir, "menu.js"), "export function Root() { return 1; }\nexport function Item() { return 2; }\n");
  writeFileSync(join(dir, "lib.rs"), `#[allow(non_snake_case)]
mod ContextMenu {
    // The props its parts take, and what's derived of them: no code of
    // its own.
    #[derive(Clone, Copy)]
    pub struct RootProps {
        pub modal: bool,
    }

    #[cfg_attr(rust_js, rust_js::link_name = "./menu.js#*.Root")]
    pub fn Root(props: RootProps) -> u32 {
        let _ = props;
        unreachable!()
    }

    #[cfg_attr(rust_js, rust_js::link_name = "./menu.js#*.Item")]
    pub fn Item() -> u32 {
        unreachable!()
    }
}

mod side_menu {
    #[cfg_attr(rust_js, rust_js::link_name = "./side.js#*.open")]
    pub fn open() -> u32 {
        unreachable!()
    }
}

mod cache {
    #[cfg_attr(rust_js, rust_js::link_name = "./scheduler.js#*.unstable_now")]
    fn now() -> u32 {
        unreachable!()
    }

    pub fn later() -> u32 {
        now() + 1
    }
}

mod clock_parts {
    #[cfg_attr(rust_js, rust_js::link_name = "./timers.js#*.start")]
    pub fn start() -> u32 {
        unreachable!()
    }

    #[cfg_attr(rust_js, rust_js::link_name = "./clock.js#tick")]
    pub fn tick() -> u32 {
        unreachable!()
    }
}

pub fn both() -> u32 {
    ContextMenu::Root(ContextMenu::RootProps { modal: true }) + ContextMenu::Item() + side_menu::open() + cache::later()
}

pub fn timed() -> u32 {
    clock_parts::start() + clock_parts::tick()
}
`);
  writeFileSync(join(dir, "timers.js"), "export function start() { return 16; }\n");
  writeFileSync(join(dir, "clock.js"), "export function tick() { return 32; }\n");
  writeFileSync(join(dir, "side.js"), "export function open() { return 4; }\n");
  writeFileSync(join(dir, "scheduler.js"), "export function unstable_now() { return 8; }\n");
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain('import * as ContextMenu from "./menu.js";');
  expect(js).toContain("ContextMenu.Root({ modal: true }) + ContextMenu.Item()");
  // A module's name as every Rust name is, camelCase (ADR 0573).
  expect(js).toContain('import * as sideMenu from "./side.js";');
  // A module of other code too, which only calls a member, names it by
  // what it's imported from, as the crate's root does (ADR 0256, amended).
  expect(readFileSync(join(dir, "cache.js"), "utf8")).toContain('import * as scheduler from "./scheduler.js";');
  // And one that holds another module's binding too.
  expect(js).toContain('import * as timers from "./timers.js";');
  const lib = await import(join(dir, "lib.js"));
  expect([lib.both(), lib.timed()]).toEqual([16, 48]);
});

// A default import used only for its statics, a class's in a module named
// for it, `mod greeter`, is named as the class, `Greeter.hello()`, as
// Next.js's docs write `App.getInitialProps`.
test("a default import of a class's statics is named as the class", async () => {
  const dir = fixture("default-statics");
  writeFileSync(join(dir, "greeter.js"), "export default class Greeter {\n  static hello() { return 1; }\n  static bye() { return 2; }\n}\n");
  writeFileSync(join(dir, "lib.rs"), `mod greeter {
    #[cfg_attr(rust_js, rust_js::link_name = "./greeter.js#default.hello")]
    pub fn hello() -> u32 {
        unreachable!()
    }

    #[cfg_attr(rust_js, rust_js::link_name = "./greeter.js#default.bye")]
    pub fn bye() -> u32 {
        unreachable!()
    }
}

pub fn both() -> u32 {
    greeter::hello() + greeter::bye()
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain('import Greeter from "./greeter.js";');
  expect(js).toContain("Greeter.hello() + Greeter.bye()");
  const lib = await import(join(dir, "lib.js"));
  expect(lib.both()).toBe(3);
});

// A default function a module holds two bindings of, signatures that
// differ in a parameter (ADR 0443), is named for the file it's from,
// `scale`, as no one function's name is it, nor a class's whose statics
// the module holds.
test("a default function bound twice is named for its file", async () => {
  const dir = fixture("default-overloads");
  writeFileSync(join(dir, "scale.js"), "export default function scale(x, by) {\n  return typeof x === \"string\" ? x.repeat(by) : x * by;\n}\n");
  writeFileSync(join(dir, "lib.rs"), `mod sizes {
    #[cfg_attr(rust_js, rust_js::link_name = "./scale.js#default")]
    pub fn scaled(x: f64, by: f64) -> f64 {
        unreachable!()
    }

    #[cfg_attr(rust_js, rust_js::link_name = "./scale.js#default")]
    pub fn repeated(x: &str, by: f64) -> String {
        unreachable!()
    }
}

pub fn both() -> String {
    format!("{} {}", sizes::scaled(2.0, 3.0), sizes::repeated("ab", 2.0))
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain('import scale from "./scale.js";');
  const lib = await import(join(dir, "lib.js"));
  expect(lib.both()).toBe("6 abab");
});

// A module's `pub use` of another module's function is a JS re-export,
// `export { helper } from "./inner.js"`, as react.dev's Challenges/index
// re-exports `Challenges` (ADR 0240).
test("a pub use of another module's function is re-exported from it", async () => {
  const dir = fixture("reexports");
  writeFileSync(join(dir, "lib.rs"), "mod inner;\npub use inner::helper;\npub use inner::other as renamed;\nuse inner::other;\n\npub fn own() -> u32 {\n    other() + 1\n}\n");
  writeFileSync(join(dir, "inner.rs"), "pub fn helper() -> u32 {\n    1\n}\n\npub fn other() -> u32 {\n    2\n}\n");
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain('export { helper, other as renamed } from "./inner.js";');
  const lib = await import(join(dir, "lib.js"));
  expect([lib.helper(), lib.renamed(), lib.own()]).toEqual([1, 2, 3]);
});

// A `pub use` of the module's own function, or another module's constant,
// by another name exports it by that, its `#[rust_js::name]` or its name
// camelCase, as React's scheduler exports `getCurrentTime` as `unstable_now`
// and `ImmediatePriority` as `unstable_ImmediatePriority` (ADR 0596). Its
// own item it exports by those alone, as upstream's `function
// getCurrentTime` is in its `export { .. }` alone (ADR 0596, amended): a
// function, and a constant, in the order the `pub use`s are written.
test("a pub use by another name of the module's own function exports it by that", async () => {
  const dir = fixture("aliases");
  writeFileSync(join(dir, "lib.rs"), `mod priorities;

#[cfg_attr(rust_js, rust_js::name = "unstable_ImmediatePriority")]
pub use priorities::IMMEDIATE as UNSTABLE_IMMEDIATE;
pub use priorities::IDLE;
#[cfg_attr(rust_js, rust_js::name = "unstable_now")]
pub use self::current_time as unstable_now;
pub use self::LIMIT as MAX_LIMIT;
pub use self::current_time as time_now;

pub const LIMIT: u32 = 9;

pub fn current_time() -> u32 {
    priorities::IMMEDIATE + priorities::IDLE
}

pub fn to_array() -> [u32; 2] {
    [current_time(), LIMIT]
}
`);
  writeFileSync(join(dir, "priorities.rs"), "pub const IMMEDIATE: u32 = 1;\npub const IDLE: u32 = 5;\n");
  run([compiler, "--declarations", join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const dts = readFileSync(join(dir, "lib.d.ts"), "utf8");
  expect(dts).toContain("\ndeclare const LIMIT: number;\n");
  expect(dts).toContain("\ndeclare function currentTime(): number;\n");
  expect(dts).toContain("\nexport function toArray(): number[];\n");
  expect(dts).toContain("export { currentTime as unstable_now, LIMIT as MAX_LIMIT, currentTime as timeNow };");
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain('export { IMMEDIATE as unstable_ImmediatePriority, IDLE } from "./priorities.js";');
  expect(js).toContain("\nconst LIMIT = 9;\n");
  expect(js).toContain("\nfunction currentTime() {\n");
  expect(js).toContain("\nexport function toArray() {\n");
  expect(js).toContain("export { currentTime as unstable_now, LIMIT as MAX_LIMIT, currentTime as timeNow };");
  // The module reads them by their own names, not the names it exports them by.
  expect(js).toContain('import { IDLE, IMMEDIATE } from "./priorities.js";');
  const lib = await import(join(dir, "lib.js"));
  expect([lib.unstable_now(), lib.timeNow(), lib.MAX_LIMIT, lib.unstable_ImmediatePriority, lib.IDLE]).toEqual([6, 6, 9, 1, 5]);
  expect([lib.currentTime, lib.LIMIT, lib.toArray()]).toEqual([undefined, undefined, [6, 9]]);
});

// One it exports by an alias alone is its module's own, so it takes the
// name of a global its module doesn't read, `console`, as an unexported
// function does (ADR 0352), and its `export { .. }` names it so (ADR 0596,
// amended).
test("a function exported by its alias alone is named as an unexported one", async () => {
  const dir = fixture("aliases-reclaimed");
  writeFileSync(join(dir, "lib.rs"), `pub use self::console as log;

pub fn console() -> f64 {
    1.0
}

pub fn twice() -> f64 {
    console() * 2.0
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("\nfunction console() {\n");
  expect(js).toContain("\nexport { console as log };\n");
  const lib = await import(join(dir, "lib.js"));
  expect([lib.log(), lib.twice()]).toEqual([1, 2]);
});

// A module that exports its own item by another name alone is imported by
// that by every other module of its crate (`default_exports::exported_as`,
// ADR 0596, amended): one that names it by its own path, as the Rust names
// it, `import { unstable_now as getCurrentTime }`; one whose `use` names it
// by the alias, by the name it's exported as; a `pub use` of it, from it.
test("a module's own item a pub use renames is imported by that name", async () => {
  const dir = fixture("aliases-imported");
  writeFileSync(join(dir, "lib.rs"), `mod index;
mod report;
mod scheduler;

use scheduler::unstable_now;

pub fn by_alias() -> f64 {
    unstable_now() + scheduler::to_array()
}

pub fn by_path() -> f64 {
    report::report() + index::get_current_time() + scheduler::MAX
}
`);
  writeFileSync(join(dir, "scheduler.rs"), `#[cfg_attr(rust_js, rust_js::name = "unstable_now")]
pub use self::get_current_time as unstable_now;
pub use self::LIMIT as MAX;

pub const LIMIT: f64 = 100.0;

pub fn get_current_time() -> f64 {
    6.0
}

pub fn to_array() -> f64 {
    get_current_time() + 1.0
}
`);
  writeFileSync(join(dir, "report.rs"), "pub fn report() -> f64 {\n    crate::scheduler::get_current_time() * 10.0 + crate::scheduler::LIMIT\n}\n");
  writeFileSync(join(dir, "index.rs"), "pub use crate::scheduler::get_current_time;\n");
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const read = (file: string) => readFileSync(join(dir, file), "utf8");
  expect(read("scheduler.js")).toContain("\nfunction getCurrentTime() {\n");
  expect(read("scheduler.js")).toContain("export { getCurrentTime as unstable_now, LIMIT as MAX };");
  // A path through a re-export is to the item, read by its alias where the
  // module's `use` names it so, else by its own name.
  expect(read("lib.js")).toContain('import { MAX as LIMIT, toArray, unstable_now } from "./scheduler.js";\n');
  expect(read("lib.js")).toContain("return unstable_now() + toArray();");
  expect(read("report.js")).toContain('import { MAX as LIMIT, unstable_now as getCurrentTime } from "./scheduler.js";');
  expect(read("report.js")).toContain("return getCurrentTime() * 10 + LIMIT;");
  expect(read("index.js")).toContain('export { unstable_now as getCurrentTime } from "./scheduler.js";');
  const lib = await import(join(dir, "lib.js"));
  expect([lib.byAlias(), lib.byPath()]).toEqual([13, 266]);
  const index = await import(join(dir, "index.js"));
  expect(Object.keys(index)).toEqual(["getCurrentTime"]);
  expect(index.getCurrentTime()).toBe(6);
});

// A `pub use` of a binding of a JS module's export is a re-export from that
// module, `export {Challenges} from './Challenges'`, as react.dev's
// Challenges/index re-exports its still-TypeScript Challenges (ADR 0240),
// in the JS and the `.d.ts`; a binding's default by the name it's given.
test("a pub use of a JS module's binding is re-exported from that module", async () => {
  const dir = fixture("reexported-bindings");
  writeFileSync(join(dir, "lib.rs"), `mod index;
mod parts;

pub fn own() -> f64 {
    index::twice(2.0) + index::Half(4.0) + *parts::values::scale
}
`);
  writeFileSync(join(dir, "parts.rs"), "pub mod values;\n");
  mkdirSync(join(dir, "parts"), { recursive: true });
  writeFileSync(join(dir, "parts", "values.rs"), `mod bound {
    unsafe extern "Rust" {
        #[link_name = "./math.js#scale"]
        pub safe static scale: &'static f64;
    }
}

pub use bound::scale;
`);
  writeFileSync(join(dir, "index.rs"), `mod bound {
    #[cfg_attr(rust_js, rust_js::link_name = "./math.js#twice")]
    pub fn twice(x: f64) -> f64 {
        unreachable!()
    }

    #[cfg_attr(rust_js, rust_js::link_name = "./math.js#default")]
    pub fn half(x: f64) -> f64 {
        unreachable!()
    }
}

pub use bound::half as Half;
pub use bound::twice;
`);
  writeFileSync(join(dir, "math.js"), "export const scale = 10;\n\nexport function twice(x) {\n  return x * 2;\n}\n\nexport default function half(x) {\n  return x / 2;\n}\n");
  run([compiler, "--declarations", join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  expect(readFileSync(join(dir, "index.js"), "utf8")).toContain('export { default as Half, twice } from "./math.js";');
  expect(readFileSync(join(dir, "index.d.ts"), "utf8")).toContain('export { default as Half, twice } from "./math.js";');
  // A module of only a static's re-export is a file of it, its JS module
  // named from its folder, as ADR 0028 names an import.
  expect(readFileSync(join(dir, "parts", "values.js"), "utf8")).toContain('export { scale } from "../math.js";');
  expect(readFileSync(join(dir, "parts", "values.d.ts"), "utf8")).toContain('export { scale } from "../math.js";');
  const index = await import(join(dir, "index.js"));
  const lib = await import(join(dir, "lib.js"));
  expect([index.twice(3), index.Half(8), lib.own()]).toEqual([6, 4, 16]);
});

// A `use` that renames another module's function imports it by that name,
// `import { client as sandpack }`, as react.dev's MDXComponents imports
// `{SandpackClient as Sandpack}`.
test("a use that renames another module's function imports it as that", async () => {
  const dir = fixture("renamed-import");
  writeFileSync(join(dir, "lib.rs"), "mod inner;\nuse inner::client as sandpack;\nuse inner::other as second;\n\npub fn run() -> u32 {\n    sandpack() + second() + 1\n}\n");
  writeFileSync(join(dir, "inner.rs"), "pub fn client() -> u32 {\n    1\n}\n\npub fn other() -> u32 {\n    2\n}\n");
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain('import { client as sandpack, other as second } from "./inner.js";');
  expect(js).toContain("sandpack() + second()");
  const lib = await import(join(dir, "lib.js"));
  expect(lib.run()).toBe(4);
});

// A binding's named export a `use` renames is imported by that name, as its
// default is, `import { twice as double }` (ADR 0202, amended); one of a
// member of an export doesn't rename the export.
test("a use that renames a binding imports its export as that", async () => {
  const dir = fixture("renamed-binding-import");
  writeFileSync(join(dir, "lib.rs"), `mod bound {
    #[cfg_attr(rust_js, rust_js::link_name = "./math.js#twice")]
    pub fn twice(x: f64) -> f64 {
        unreachable!()
    }

    #[cfg_attr(rust_js, rust_js::link_name = "./math.js#default")]
    pub fn half(x: f64) -> f64 {
        unreachable!()
    }

    #[cfg_attr(rust_js, rust_js::link_name = "./math.js#tools.scale")]
    pub fn scale(x: f64) -> f64 {
        unreachable!()
    }
}

use bound::half as halve;
use bound::scale as scaled;
use bound::twice as double;

pub fn run(x: f64) -> f64 {
    scaled(double(halve(x)))
}
`);
  writeFileSync(join(dir, "math.js"), "export function twice(x) {\n  return x * 2;\n}\n\nexport const tools = { scale: (x) => x * 10 };\n\nexport default function half(x) {\n  return x / 2;\n}\n");
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  // Of a member of an export, `tools.scale`, the export keeps its name.
  expect(js).toContain('import halve, { tools, twice as double } from "./math.js";');
  expect(js).toContain("return tools.scale(double(halve(x)));");
  const lib = await import(join(dir, "lib.js"));
  expect(lib.run(6)).toBe(60);
});

// A file imports an item once, by one name: two `use`s naming it apart are
// refused, never one of them left unsaid (ADR 0202, amended).
test("two uses naming one item apart are refused", () => {
  const dir = fixture("two-import-names");
  writeFileSync(join(dir, "lib.rs"), "mod inner;\nuse inner::client as first;\nuse inner::client as second;\n\npub fn run() -> u32 {\n    first() + second()\n}\n");
  writeFileSync(join(dir, "inner.rs"), "pub fn client() -> u32 {\n    1\n}\n");
  const built = Bun.spawnSync([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")], { stderr: "pipe" });
  expect(built.exitCode).not.toBe(0);
  expect(built.stderr.toString()).toContain("imports `client` once, by one name, and another `use` names it `first`; name it once");
});

// A `#[rust_js::name]` on a `use` of one name is its import's name, React's
// `import { ImmediatePriority as ImmediateSchedulerPriority }`; one without
// is cased as the item, a thread-local of a primitive as Rust writes it.
// On a group or a glob, which it would name each the same, it's refused.
test("a use of one name is imported by its rust_js::name", async () => {
  const dir = fixture("use-named");
  const priorities = `pub mod priorities {
    thread_local! {
        #[cfg_attr(rust_js, rust_js::name = "ImmediatePriority")]
        pub static IMMEDIATE_PRIORITY: u8 = 1;
        #[cfg_attr(rust_js, rust_js::name = "NormalPriority")]
        pub static NORMAL_PRIORITY: u8 = 3;
    }
}
`;
  writeFileSync(
    join(dir, "lib.rs"),
    `${priorities}
#[cfg_attr(rust_js, rust_js::name = "ImmediateSchedulerPriority")]
use priorities::IMMEDIATE_PRIORITY as IMMEDIATE_SCHEDULER_PRIORITY;
use priorities::NORMAL_PRIORITY as NORMAL_SCHEDULER_PRIORITY;

pub fn both() -> u8 {
    IMMEDIATE_SCHEDULER_PRIORITY.with(|&p| p) + NORMAL_SCHEDULER_PRIORITY.with(|&p| p)
}
`,
  );
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("ImmediatePriority as ImmediateSchedulerPriority,");
  expect(js).toContain("NormalPriority as NORMAL_SCHEDULER_PRIORITY,");
  expect((await import(join(dir, "lib.js"))).both()).toBe(4);
  const group = fixture("use-group-named");
  writeFileSync(
    join(group, "lib.rs"),
    `${priorities}
#[cfg_attr(rust_js, rust_js::name = "SchedulerPriority")]
use priorities::{IMMEDIATE_PRIORITY as IMMEDIATE_SCHEDULER_PRIORITY, NORMAL_PRIORITY as NORMAL_SCHEDULER_PRIORITY};

pub fn both() -> u8 {
    IMMEDIATE_SCHEDULER_PRIORITY.with(|&p| p) + NORMAL_SCHEDULER_PRIORITY.with(|&p| p)
}
`,
  );
  const built = Bun.spawnSync([compiler, join(group, "lib.rs"), "-o", join(group, "lib.js")], { stderr: "pipe" });
  expect(built.exitCode).not.toBe(0);
  expect(built.stderr.toString()).toContain("`#[rust_js::name]` names the import of a `use` of one name");
});

// A module of re-exports only is a file of them, as react.dev's
// Sidebar/index is `export {SidebarLink} from './SidebarLink'`.
test("a module of only pub uses is a file of re-exports", async () => {
  const dir = fixture("reexports-only");
  writeFileSync(join(dir, "lib.rs"), "mod index;\nmod inner;\n\npub fn own() -> u32 {\n    index::helper() + 1\n}\n");
  writeFileSync(join(dir, "index.rs"), "pub use super::inner::helper;\n");
  writeFileSync(join(dir, "inner.rs"), "pub fn helper() -> u32 {\n    1\n}\n");
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  expect(readFileSync(join(dir, "index.js"), "utf8")).toContain('export { helper } from "./inner.js";');
  const index = await import(join(dir, "index.js"));
  expect(index.helper()).toBe(1);
});

// ADR 0616: an import its module reads once, a \`const\` at a function's top
// that copies it, is the import by that name, as React's shallowEqual has
// \`import is from './objectIs'\`, not \`import IS ..; const is = IS;\`.
test("an import read only by a const is the import by its name", async () => {
  const dir = fixture("aliased-imports");
  writeFileSync(join(dir, "lib.rs"), "mod is;\nmod twice;\n\nuse is::IS as SAME;\n\npub fn equal(a: f64, b: f64) -> bool {\n    let same = SAME.with(|same| *same);\n    same(a, b)\n}\n");
  writeFileSync(join(dir, "is.rs"), "fn exact(a: f64, b: f64) -> bool {\n    a == b\n}\n\nthread_local! {\n    pub(crate) static IS: fn(f64, f64) -> bool = exact;\n}\n\njs::export_default!(IS);\n");
  // Read twice: the import keeps its name.
  writeFileSync(join(dir, "twice.rs"), "use crate::is::IS;\n\npub fn both(a: f64, b: f64) -> bool {\n    let first = IS.with(|is| *is);\n    first(a, b) && IS.with(|is| is(b, a))\n}\n");
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain('import same from "./is.js";');
  const twice = await import(join(dir, "twice.js"));
  expect([twice.both(1, 1), twice.both(1, 2)]).toEqual([true, false]);
  expect(js).toContain("  return same(a, b);");
  const lib = await import(join(dir, "lib.js"));
  expect([lib.equal(1, 1), lib.equal(1, 2)]).toEqual([true, false]);
});

// ADR 0611: \`js::export_default!\` of the function itself is its
// declaration as the default, \`export default function noop() {}\`, as
// React's noop.js, which rustc sees used; naming one, \`export default
// recap;\`, as react.dev's components.
test("a default export is the function's declaration where the macro holds it", async () => {
  const dir = fixture("default-where-written");
  writeFileSync(join(dir, "lib.rs"), "mod noop;\nmod recap;\n\npub fn both() -> u32 {\n    noop::noop();\n    recap::recap()\n}\n");
  writeFileSync(join(dir, "noop.rs"), "js::export_default! {\n    pub(crate) fn noop() {}\n}\n");
  writeFileSync(join(dir, "recap.rs"), "pub(crate) fn recap() -> u32 {\n    1\n}\n\njs::export_default!(recap);\n");
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  expect(readFileSync(join(dir, "noop.js"), "utf8")).toContain("export default function noop() {}");
  const recap = readFileSync(join(dir, "recap.js"), "utf8");
  expect(recap).toContain("function recap() {");
  expect(recap).toContain("\nexport default recap;\n");
  const lib = await import(join(dir, "lib.js"));
  expect(lib.both()).toBe(1);
});

// ADR 0611: a function only `js::export_default!` names, as React's
// DefaultPrepareStackTraceV8 has its hook, is used, as JS imports it:
// rust-js's check warns of none, a generic one's either, and of a function
// nothing names still.
test("a function only js::export_default! names is no dead code", () => {
  const dir = fixture("default-only-use");
  writeFileSync(join(dir, "lib.rs"), "mod v8;\nmod first;\n");
  writeFileSync(join(dir, "v8.rs"), "fn prepare_stack_trace(name: &str) -> String {\n    format!(\"{name}: \")\n}\n\njs::export_default!(prepare_stack_trace);\n\nfn unnamed() {}\n");
  writeFileSync(join(dir, "first.rs"), "fn first<T>(items: Vec<T>) -> Option<T> {\n    items.into_iter().next()\n}\n\njs::export_default!(first);\n");
  const built = Bun.spawnSync([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target], { cwd: dir });
  const warned = built.stderr.toString().split("\n").filter((line) => line.startsWith("warning:") && !/warnings? emitted/.test(line));
  expect([built.exitCode, warned]).toEqual([0, ["warning: function `unnamed` is never used"]]);
});

// ADR 0625: of a `const` itself, its value is the default, `export
// default "19.3.0";`, as React's ReactVersion, JS having no declaration of
// a constant as a default; another module reads it as its default.
test("a default export is the constant's value where the macro holds it", async () => {
  const dir = fixture("default-constant");
  writeFileSync(join(dir, "lib.rs"), "mod version;\n\npub fn shown() -> String {\n    format!(\"v{}\", version::REACT_VERSION)\n}\n");
  writeFileSync(join(dir, "version.rs"), "js::export_default! {\n    pub(crate) const REACT_VERSION: &str = \"19.3.0\";\n}\n");
  run([compiler, "--declarations", join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  // TypeScript has no default `const`: it's declared, then exported.
  expect(readFileSync(join(dir, "version.d.ts"), "utf8")).toContain("declare const REACT_VERSION: string;\n\nexport default REACT_VERSION;\n");
  const version = readFileSync(join(dir, "version.js"), "utf8");
  expect(version).toContain('\nexport default "19.3.0";\n');
  expect(version).not.toContain("REACT_VERSION");
  expect((await import(join(dir, "version.js"))).default).toBe("19.3.0");
  expect((await import(join(dir, "lib.js"))).shown()).toBe("v19.3.0");
});

// One its own module reads by name is declared, then exported by default,
// `const VERSION = "19.3.0"; export default VERSION;`, never a name JS
// hasn't (ADR 0625, amended); a `pub` one too, the default alone.
test("a default constant its own module reads is declared, then exported", async () => {
  const dir = fixture("default-constant-read");
  writeFileSync(join(dir, "lib.rs"), 'mod version;\n\npub fn shown() -> String {\n    format!("v{} {}", version::VERSION, version::length())\n}\n');
  writeFileSync(join(dir, "version.rs"), 'js::export_default! {\n    pub const VERSION: &str = "19.3.0";\n}\n\npub fn length() -> usize {\n    VERSION.len()\n}\n');
  run([compiler, "--declarations", join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  const version = readFileSync(join(dir, "version.js"), "utf8");
  expect(version).toContain('\nconst VERSION = "19.3.0";\n');
  expect(version).toContain("\nexport default VERSION;\n");
  expect(version).not.toContain("export const");
  expect(readFileSync(join(dir, "version.d.ts"), "utf8")).toContain("declare const VERSION: string;");
  const module = await import(join(dir, "version.js"));
  expect([module.default, module.length(), module.VERSION]).toEqual(["19.3.0", 6, undefined]);
  expect((await import(join(dir, "lib.js"))).shown()).toBe("v19.3.0 6");
});

// A module's default that's another module's function is re-exported as
// it, `export { page as default } from "./page.js"`, as react.dev's
// errors/index makes the errors page its own (ADR 0240); a module of only
// that is a file of it.
test("a module's default of another module's function is re-exported", async () => {
  const dir = fixture("default-reexport");
  writeFileSync(join(dir, "lib.rs"), "mod index;\nmod only;\nmod page;\n\npub fn own() -> u32 {\n    page::page() + page::props()\n}\n");
  writeFileSync(join(dir, "index.rs"), "pub use super::page::props;\n\njs::export_default!(super::page::page);\n");
  writeFileSync(join(dir, "only.rs"), "js::export_default!(super::page::page);\n");
  writeFileSync(join(dir, "page.rs"), "pub fn page() -> u32 {\n    1\n}\n\npub fn props() -> u32 {\n    2\n}\n");
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  expect(readFileSync(join(dir, "index.js"), "utf8")).toContain('export { props } from "./page.js";\n\nexport { page as default } from "./page.js";');
  expect(readFileSync(join(dir, "only.js"), "utf8")).toContain('export { page as default } from "./page.js";');
  const index = await import(join(dir, "index.js"));
  expect([index.default(), index.props()]).toEqual([1, 2]);
});

// A module's default is what JS imports and calls with anything: not a
// function only the crate's calls reach, whose parameters are what they
// give it (ADR 0577), though Rust's `pub` doesn't say it's exported. Of
// another module's, that module exports it for the default to read, whoever
// else calls it; and the `use` the macro names it by renames nothing.
test("a module's default is called with whatever JS gives it", async () => {
  const dir = fixture("default-called-by-js");
  writeFileSync(join(dir, "lib.rs"), "mod declared;\nmod helpers;\nmod page;\nmod thrice;\nmod twice;\n");
  writeFileSync(join(dir, "helpers.rs"), "pub(crate) fn twice(n: u32) -> u32 {\n    n * 2\n}\n\npub(crate) fn thrice(n: u32) -> u32 {\n    n * 3\n}\n");
  writeFileSync(join(dir, "twice.rs"), "js::export_default!(super::helpers::twice);\n");
  writeFileSync(join(dir, "thrice.rs"), "js::export_default!(super::helpers::thrice);\n\npub fn nine() -> u32 {\n    super::helpers::thrice(3)\n}\n");
  writeFileSync(join(dir, "page.rs"), "fn page(n: u32) -> u32 {\n    n * 5\n}\n\njs::export_default!(page);\n\npub fn fifteen() -> u32 {\n    page(3)\n}\n");
  writeFileSync(join(dir, "declared.rs"), "js::export_default! {\n    fn seven(n: u32) -> u32 {\n        n * 7\n    }\n}\n\npub fn fourteen() -> u32 {\n    seven(2)\n}\n");
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  expect(readFileSync(join(dir, "helpers.js"), "utf8")).toContain("export function twice(n) {\n  return (n * 2) >>> 0;\n}");
  expect(readFileSync(join(dir, "thrice.js"), "utf8")).toContain('import { thrice } from "./helpers.js";\n');
  expect(readFileSync(join(dir, "thrice.js"), "utf8")).toContain("  return thrice(3);\n");
  // u32's product wraps, as Rust's without overflow checks does.
  const big = 4_000_000_000;
  const at = async (file: string) => (await import(join(dir, file))).default(big);
  expect([await at("twice.js"), await at("thrice.js"), await at("page.js"), await at("declared.js")])
    .toEqual([2, 3, 5, 7].map((k) => (k * big) % 2 ** 32));
  expect([(await import(join(dir, "thrice.js"))).nine(), (await import(join(dir, "page.js"))).fifteen()]).toEqual([9, 15]);
});

// ADR 0267: what a module runs when it's loaded, `js::on_load!`, is its
// JS's own statements, as react.dev's Page prefetches CodeBlock with a bare
// `import()`: rustc checks them as a function's body nothing calls.
test("js::on_load! is what the module runs when it's loaded", async () => {
  const withWeb = ["--", "--extern", `webapi=${join(target, "libwebapi.rmeta")}`, "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];
  const dir = fixture("on-load");
  writeFileSync(join(dir, "lib.rs"), `#[cfg_attr(rust_js, rust_js::link_name = "globalThis.loaded")] #[allow(unused_variables)] fn loaded(what: &str) { unreachable!() }

fn which() -> usize {
    1
}

js::on_load! {
    loaded("module");
    loaded(["first", "second"][which()]);
}

pub fn ready() -> u32 {
    1
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withWeb]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain('\nglobalThis.loaded("module");\n');
  expect(js).toContain('import { $index } from "@rust-js/runtime";');
  expect(js).not.toContain("on_load");
  expect(js).not.toContain("const _");
  const seen: string[] = [];
  (globalThis as any).loaded = (what: string) => seen.push(what);
  try {
    const { ready } = await import(join(dir, "lib.js"));
    expect([seen, ready()]).toEqual([["module", "second"], 1]);
  } finally {
    delete (globalThis as any).loaded;
  }
});

// ADR 0306: a module's items are where its Rust has them, as react.dev's
// runESLint has its function, then its consts and statements, each where
// it's written; what's made when it's loaded comes after what it reads, a
// later thread-local's, or one a function it calls reads.
test("a module's items are in source order, each after what it reads when loaded", async () => {
  const dir = fixture("item-order");
  writeFileSync(join(dir, "lib.rs"), `use std::cell::Cell;

fn twice(n: u32) -> u32 {
    n * 2
}

thread_local! {
    static FIRST: u32 = twice(1);
    static AFTER: u32 = LATER.with(|later| *later) + 1;
    static CALLED: u32 = read_later() + 1;
}

js::on_load! {
    SEEN.with(|seen| seen.set(seen.get() + 1));
}

thread_local! {
    static SEEN: Cell<u32> = Cell::new(0);
    static LATER: u32 = 10;
}

fn read_later() -> u32 {
    LATER.with(|later| *later)
}

pub fn values() -> [u32; 4] {
    [FIRST.with(|f| *f), AFTER.with(|a| *a), CALLED.with(|c| *c), SEEN.with(|s| s.get())]
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  const at = (text: string | RegExp) => {
    const index = typeof text === "string" ? js.indexOf(text) : js.search(text);
    expect(index).toBeGreaterThan(-1);
    return index;
  };
  // Where the Rust has them.
  expect(at("function twice(")).toBeLessThan(at("const FIRST ="));
  expect(at("const FIRST =")).toBeLessThan(at("let seen = 0;"));
  expect(at("const LATER =")).toBeLessThan(at("function readLater("));
  // After what they read: LATER, directly or through readLater; SEEN.
  expect(at("const LATER =")).toBeLessThan(at("const AFTER ="));
  expect(at("const LATER =")).toBeLessThan(at("const CALLED ="));
  // Set where the module's loaded, on_load!'s body in its place (ADR
  // 0840): a `let`, not a `const` assigned (#201).
  expect(at("let seen = 0;")).toBeLessThan(at("seen = (seen + 1) >>> 0;"));
  const { values } = await import(join(dir, "lib.js"));
  expect(values()).toEqual([2, 11, 11, 1]);
});

// React's clz32.js: `export const clz32 = Math.clz32 ? Math.clz32 :
// clz32Fallback;`, then the `const`s the fallback reads. A function an
// initializer only holds, as its value, isn't run when it's made: what it
// reads isn't before the initializer, which is where its Rust has it. One
// that calls what holds it, `USED`, is after what that function reads
// (ADR 0306, amended).
test("a function an initializer only holds doesn't move what it reads before it", async () => {
  const dir = fixture("item-order-held");
  writeFileSync(join(dir, "lib.rs"), `// TODO: This is pretty well supported by browsers. Maybe we can drop it.

thread_local! {
    pub static CLZ32: fn(u32) -> u32 = if js::is_defined(u32::leading_zeros) {
        u32::leading_zeros
    } else {
        clz32_fallback
    };
}

// Count leading zeros.
#[cfg_attr(rust_js, rust_js::name = "log")]
const LOG: fn(f64) -> f64 = f64::ln;
const LN2: f64 = std::f64::consts::LN_2;
fn clz32_fallback(x: u32) -> u32 {
    if x == 0 {
        return 32;
    }
    (31.0 - ((LOG(x as f64) / LN2) as u32 as f64)) as u32
}

thread_local! {
    static PICK: fn(u32) -> u32 = scaled;
    static PICKED: [fn(u32) -> u32; 1] = [PICK.with(|pick| *pick)];
    static USED: u32 = PICKED.with(|picked| picked[0](2));
    static SCALE: u32 = 3;
}

fn scaled(n: u32) -> u32 {
    n * SCALE.with(|scale| *scale)
}

pub fn values(x: u32) -> [u32; 3] {
    [CLZ32.with(|f| f(x)), clz32_fallback(x), USED.with(|used| *used)]
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain(
    '// TODO: This is pretty well supported by browsers. Maybe we can drop it.\n\nimport { $f64ToInt, $local } from "@rust-js/runtime";\n\nexport const clz32 = typeof Math.clz32 !== "undefined" ? Math.clz32 : clz32Fallback;\n\n// Count leading zeros.\nconst { log, LN2 } = Math;\nfunction clz32Fallback(x) {\n',
  );
  const at = (text: string) => {
    expect(js).toContain(text);
    return js.indexOf(text);
  };
  // What only holds `scaled` is where it's written; what calls it is after
  // what it reads, through the array and the thread-local holding it, and
  // made where it's first used, as a call through a function pointer may
  // run any of the crate's code (ADR 0037, amended).
  expect(at("const pick = scaled;\nconst picked = [pick];\n")).toBeLessThan(at("const SCALE = 3;"));
  expect(at("const SCALE = 3;")).toBeLessThan(at("const USED ="));
  const { values } = await import(join(dir, "lib.js"));
  expect(values(1 as never)).toEqual([31, 31, 6]);
});

// ADR 0295: a module's imports are one block, as a person orders them:
// packages, then its own relative modules, bindings and the crate's alike,
// each by path; then what it imports for its effect, as written, as
// react.dev's _app has its CSS after; then rust-js's runtime.
test("imports are packages, then relative modules, then effects, then the runtime", () => {
  const dir = fixture("import-order");
  writeFileSync(join(dir, "lib.rs"), `js::import!("./app.css");

mod helper {
    pub fn h(items: &[u32], i: usize) -> u32 {
        items[i]
    }
}

#[cfg_attr(rust_js, rust_js::link_name = "zeta#zeta")]
fn zeta() -> u32 {
    unreachable!()
}

#[cfg_attr(rust_js, rust_js::link_name = "./local#local")]
fn local() -> u32 {
    unreachable!()
}

#[cfg_attr(rust_js, rust_js::link_name = "@scope/alpha#alpha")]
fn alpha() -> u32 {
    unreachable!()
}

pub fn all(items: &[u32], i: usize) -> u32 {
    zeta() + local() + alpha() + helper::h(items, i) + items[i]
}
`);
  buildReact();
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  const top = readFileSync(join(dir, "lib.js"), "utf8");
  const imports = top.split("\n").filter((line) => line.startsWith("import") || line === "").slice(1, 8).join("\n");
  expect(imports).toBe(`import { alpha } from "@scope/alpha";
import { zeta } from "zeta";
import { h } from "./helper.js";
import { local } from "./local";
import "./app.css";
import { $index } from "@rust-js/runtime";
`);
});

// React's ReactClient lists what it re-exports once, in its order, with
// its comments, `// Concurrent Mode`: each `pub use` is an `export { .. }
// from` where it's written, its comments before it, a run of them from one
// module one statement; of a class, its constructor; of a module, its
// namespace, `export * as __COMPILER_RUNTIME from`; of a JS module's
// namespace too; and of what a module re-exports itself, from that module,
// as the path names it, by the name it has there (ADR 0240, amended). A
// private `use`'s `#[rust_js::name]` is its import's name, `_jsxDEV`.
test("a module's pub uses are re-exported as written, with their comments", async () => {
  const dir = fixture("reexports-as-written");
  writeFileSync(join(dir, "lib.rs"), `mod cache_impl;
mod cache_server;
mod index;
pub mod runtime;
mod shapes;

pub use shapes::area;
// A class, by its constructor.
pub use shapes::Circle;
// Concurrent Mode
#[cfg_attr(rust_js, rust_js::name = "__COMPILER_RUNTIME")]
pub use crate::runtime as __COMPILER_RUNTIME;
pub use cache_server::{cache, unstable_cache};
pub use runtime::half;
#[cfg_attr(rust_js, rust_js::name = "unstable_double")]
pub use runtime::double as unstable_double;

mod bound {
    #[cfg_attr(rust_js, rust_js::link_name = "./math.js#*")]
    pub fn math() -> f64 {
        unreachable!()
    }
}

// DEV-only
pub use bound::math;

#[cfg_attr(rust_js, rust_js::name = "_areaDEV")]
use shapes::area as _area_dev;

pub fn twice(circle: &shapes::Circle) -> f64 {
    _area_dev(circle) * 2.0
}
`);
  writeFileSync(join(dir, "cache_impl.rs"), "pub fn cache(n: f64) -> f64 {\n    n + 1.0\n}\n");
  writeFileSync(join(dir, "cache_server.rs"), `#[cfg_attr(rust_js, rust_js::name = "unstable_cache")]
pub use crate::cache_impl::cache as unstable_cache;
pub use crate::cache_impl::cache;
`);
  writeFileSync(join(dir, "runtime.rs"), "pub fn double(n: f64) -> f64 {\n    n * 2.0\n}\n\npub fn half(n: f64) -> f64 {\n    n / 2.0\n}\n");
  writeFileSync(join(dir, "shapes.rs"), `#[cfg_attr(rust_js, rust_js::class)]
pub struct Circle {
    pub radius: f64,
}

#[cfg_attr(rust_js, rust_js::class)]
impl Circle {
    pub fn new(radius: f64) -> Self {
        Circle { radius }
    }
}

pub fn area(circle: &Circle) -> f64 {
    circle.radius * circle.radius
}
`);
  writeFileSync(join(dir, "math.js"), "export const pi = 3;\n");
  writeFileSync(join(dir, "index.rs"), "pub use crate::runtime as rt;\n");
  run([compiler, "--declarations", join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain('import { area as _areaDEV } from "./shapes.js";\n');
  expect(js).toContain(`export { area } from "./shapes.js";
// A class, by its constructor.
export { Circle } from "./shapes.js";
// Concurrent Mode
export * as __COMPILER_RUNTIME from "./runtime.js";
export { cache, unstable_cache } from "./cache_server.js";
export { half, double as unstable_double } from "./runtime.js";
`);
  expect(js).toContain('// DEV-only\nexport * as math from "./math.js";\n');
  // A module of only a namespace's re-export is a file of it.
  expect(readFileSync(join(dir, "index.js"), "utf8")).toContain('export * as rt from "./runtime.js";\n');
  const dts = readFileSync(join(dir, "lib.d.ts"), "utf8");
  expect(dts).toContain('export * as __COMPILER_RUNTIME from "./runtime.js";');
  expect(dts).toContain('export { cache, unstable_cache } from "./cache_server.js";');
  const lib = await import(join(dir, "lib.js"));
  const circle = new lib.Circle(3);
  expect([lib.area(circle), lib.twice(circle), lib.__COMPILER_RUNTIME.double(2), lib.cache(1), lib.unstable_cache(2)]).toEqual([9, 18, 4, 2, 3]);
  expect([lib.unstable_double(3), lib.half(3), lib.math.pi]).toEqual([6, 1.5, 3]);
});

// The user's decision (2026-10-27), ADR 0256, amended: React's reconciler
// Scheduler.js, `import * as Scheduler from 'scheduler'; export const now =
// Scheduler.unstable_now;`. A `link_name`'s `* as Name` names the namespace
// import; each of its module's must name it the same, or it's an error.
test("a namespace import is named as its link_name's * as names it", () => {
  const dir = fixture("namespace-named");
  const source = (second: string) => `#[cfg_attr(rust_js, rust_js::link_name = "scheduler#* as Scheduler.unstable_now")]
fn unstable_now() -> f64 {
    unreachable!()
}

#[cfg_attr(rust_js, rust_js::link_name = "${second}")]
fn unstable_yield_value(value: u32) {
    let _ = value;
    unreachable!()
}

#[cfg_attr(rust_js, rust_js::name = "now")]
pub static NOW: fn() -> f64 = unstable_now;

pub fn yielded(value: u32) {
    unstable_yield_value(value);
}
`;
  writeFileSync(join(dir, "lib.rs"), source("scheduler#* as Scheduler.unstable_yieldValue"));
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain('import * as Scheduler from "scheduler";\n');
  expect(js).toContain("export const now = Scheduler.unstable_now;\n");
  expect(js).toContain("  Scheduler.unstable_yieldValue(value);\n");
  for (const second of ["scheduler#*.unstable_yieldValue", "scheduler#* as Sched.unstable_yieldValue"]) {
    writeFileSync(join(dir, "lib.rs"), source(second));
    expect(() => run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")])).toThrow(
      "each `link_name` of its `*` names it the same, `scheduler#* as Name.path`",
    );
  }
});

// nm1, React's ReactFiberHooks: `import { NotPendingTransition as
// NoPendingHostTransition } from './ReactFiberConfig'`. A binding's
// `#[rust_js::name]` names its import, ADR 0028, amended; one of a global
// or a member, which no import of its name holds, or a second name of one
// import, is an error.
test("a binding's rust_js::name names its import", () => {
  const dir = fixture("binding-named");
  const source = (name: string, path: string) => `use js::Unknown;

unsafe extern "Rust" {
    #[cfg_attr(rust_js, rust_js::name = "NoPendingHostTransition")]
    #[link_name = "./config.js#NotPendingTransition"]
    safe static NO_PENDING_HOST_TRANSITION: Option<&'static Unknown>;
}

#[cfg_attr(rust_js, rust_js::name = "${name}")]
#[cfg_attr(rust_js, rust_js::link_name = "${path}")]
fn ping_it() {
    unreachable!()
}

pub fn initial() -> Option<&'static Unknown> {
    ping_it();
    NO_PENDING_HOST_TRANSITION
}
`;
  const compile = () =>
    run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  writeFileSync(join(dir, "lib.rs"), source("pingHost", "./config.js#ping"));
  compile();
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain('import { NotPendingTransition as NoPendingHostTransition, ping as pingHost } from "./config.js";\n');
  expect(js).toContain("  pingHost();\n  return NoPendingHostTransition;\n");
  // Another module's binding of it, with no name, its own import: React's
  // ReactFiberHostContext imports `NotPendingTransition` as it is.
  writeFileSync(
    join(dir, "lib.rs"),
    `pub mod host;\n${source("pingHost", "./config.js#ping")}`,
  );
  writeFileSync(
    join(dir, "host.rs"),
    `use js::Unknown;

unsafe extern "Rust" {
    #[link_name = "./config.js#NotPendingTransition"]
    safe static NOT_PENDING_TRANSITION: Option<&'static Unknown>;
}

pub fn reset() -> Option<&'static Unknown> {
    NOT_PENDING_TRANSITION
}
`,
  );
  compile();
  expect(readFileSync(join(dir, "lib.js"), "utf8")).toContain("import { NotPendingTransition as NoPendingHostTransition, ping as pingHost }");
  expect(readFileSync(join(dir, "host.js"), "utf8")).toContain('import { NotPendingTransition } from "./config.js";\n');
  for (const [name, path, error] of [
    ["pingHost", "globalPing", "is a global's, which is read by its own name"],
    ["pingHost", "./config.js#host.ping", "imports no export of its own name"],
    ["pingHost", "./config.js#* as Config.ping", "imports no export of its own name"],
    ["Other", "./config.js#NotPendingTransition", "is imported as `NoPendingHostTransition` and as `Other`"],
  ]) {
    writeFileSync(join(dir, "lib.rs"), source(name, path));
    expect(compile).toThrow(error);
  }
});
