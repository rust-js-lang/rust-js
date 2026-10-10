// A crate's modules as JS files: each beside its source, importing what
// it uses, re-exporting what it says, running what it runs when it's
// loaded. Each test is a crate of its own, compiled and run.

import { beforeAll, expect, test } from "bun:test";
import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { buildReact, buildWebapi, compiler, fixture, root, run, target } from "./support";

beforeAll(buildReact, 600_000);

// ADR 0270: a thread-local only read and set, in its own module, is the
// module's variable, a `let` if it's set, as react.dev's errors page caches
// the codes it fetched: nothing shares its cell, so it needs no `{ value }`.
// One another module may read, or whose cell `with` hands out, keeps it.
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
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  expect(readFileSync(join(dir, "counter.js"), "utf8")).toContain("export const TICKS = { value: 0 };");
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("let COUNT = 0;");
  expect(js).toContain("const LOG = [];");
  expect(js).toContain("export const SHARED = { value: 0 };");
  expect(js).toContain("COUNT = (COUNT + 1) >>> 0;");
  expect(js).not.toContain("COUNT.value");
  expect(js).not.toContain("LOG.value");
  // One made by a `const { .. }` is too, by its own name.
  expect(js).not.toContain("__RUST_STD_INTERNAL_INIT");
  expect(js).toContain("let SEEN;\n");
  expect(js).toContain("  SEEN ??= n;\n");
  const { seen } = await import(join(dir, "lib.js"));
  expect([seen(3), seen(4)]).toEqual([3, 3]);
  const { unseen } = await import(join(dir, "lib.js"));
  expect(unseen(5)).toBe(0);
  const { bump, note, shared, ticked } = await import(join(dir, "lib.js"));
  expect([bump(), bump(), note("a"), note("b"), shared(), shared(), ticked()]).toEqual([1, 2, 1, 2, 2, 4, 1]);
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

test("methods across modules, in a thread-local, and camelCase", async () => {
  const { fixture, compiler } = await import("./support");
  const { writeFileSync } = await import("node:fs");
  const dir = fixture("methods");
  writeFileSync(join(dir, "lib.rs"), `#[rust_js::camel_case]
const _: () = ();
use std::cell::Cell;

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
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const lib = await Bun.file(join(dir, "lib.js")).text();
  const shapes = await Bun.file(join(dir, "shapes.js")).text();
  expect(lib).toContain("const SIDE = Square.sideLength(Square.new(3));");
  expect(lib).toContain("  return Square.area(Square.new(side));");
  expect(shapes).toContain("export const Square = {\n  new(side) {");
  expect(shapes).toContain("  area(square) {\n    return Math.imul(Square.sideLength(square), Square.sideLength(square)) >>> 0;");
  expect(shapes).toContain("\nconst Tally = {\n  doubled(tally) {\n    return Math.imul(tally[0], 2) >>> 0;\n  },\n};\n\nexport function tallyOfFour() {");
  // Laid out on lines of its own, a lone method still maps back to its Rust.
  const { decodeMappings, lookup } = await import("./sourcemap.ts");
  const segments = decodeMappings((await Bun.file(join(dir, "shapes.js.map")).json()).mappings);
  const jsLines = shapes.split("\n");
  const rsLines = (await Bun.file(join(dir, "shapes.rs")).text()).split("\n");
  for (const [jsText, rustText] of [["Math.imul(tally[0], 2)", "self.0 * 2"], ["doubled(tally) {", "doubled(&self)"]]) {
    const line = jsLines.findIndex((l) => l.includes(jsText));
    const hit = lookup(segments, line, jsLines[line].indexOf(jsText));
    expect([jsText, hit && rsLines[hit.srcLine].slice(hit.srcCol).startsWith(rustText)]).toEqual([jsText, true]);
  }
  const module = await import(join(dir, "lib.js"));
  expect(module.areaOf(4)).toBe(16);
  expect(module.firstSide()).toBe(3);
  expect((await import(join(dir, "shapes.js"))).tallyOfFour()).toBe(8);
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
  expect(lib).toContain('import { COUNT, SIZE } from "./util.js";');
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
  buildWebapi();
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  const top = readFileSync(join(dir, "lib.js"), "utf8"), panel = readFileSync(join(dir, "panel.js"), "utf8");
  expect([top.includes('import "./app.css";'), top.includes('import "./direct.css";'), panel.includes('import "./panel.css";')]).toEqual([true, true, true]);
  expect(top).not.toContain("const _");
  expect(panel).not.toContain("app.css");
});

// `js::directive!("use client");` is the module's directive, its first
// statement, and `js::export_default!(page);` its default export, of a
// function that stays named for the crate's other modules (ADR 0192): what a
// Next.js route is, `app/page.jsx`.
test("js::directive! and js::export_default! make a module a Next.js route", async () => {
  const dir = fixture("js-route");
  writeFileSync(join(dir, "lib.rs"), 'js::directive!("use client");\npub fn page() -> u32 {\n    about::about() + 1\n}\njs::export_default!(page);\npub mod about {\n    pub fn about() -> u32 {\n        2\n    }\n    js::export_default!(about);\n}\npub mod generic {\n    pub fn first<T>(items: Vec<T>) -> Option<T> {\n        items.into_iter().next()\n    }\n    js::export_default!(first);\n}\n');
  buildWebapi();
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  const top = readFileSync(join(dir, "lib.js"), "utf8"), about = readFileSync(join(dir, "about.js"), "utf8");
  const statements = (js: string) => js.split("\n").filter((line) => line && !line.startsWith("//"));
  expect(statements(top)[0]).toBe('"use client";');
  expect(statements(about)[0]).not.toBe('"use client";');
  expect(top).toContain("export default page;");
  expect(top).not.toContain("const _");
  const [lib, sub] = [await import(join(dir, "lib.js")), await import(join(dir, "about.js"))];
  expect([lib.default(), lib.page(), sub.default()]).toEqual([3, 3, 2]);
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
`);
  buildWebapi();
  const extern = ["--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...extern]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  const statements = js.split("\n").filter((line) => line && !line.startsWith("//"));
  expect(statements[0]).toBe("export async function cached(n) {");
  expect(js).toContain('export async function cached(n) {\n  "use cache";\n  return (n + 1) >>> 0;\n}');
  expect(js).toContain('export function plain() {\n  "use strict";\n  return 2;\n}');
  expect(js).toContain('return async (n) => {\n    "use server";\n    return (n + step) >>> 0;\n  };');
  const lib = await import(join(dir, "lib.js"));
  expect([await lib.cached(1), lib.plain(), await lib.action(2)(3)]).toEqual([2, 2, 5]);
  writeFileSync(join(dir, "block.rs"), `pub async fn later() -> u32 {
    async {
        js::directive!("use cache");
        1
    }
    .await
}
`);
  const block = Bun.spawnSync([compiler, join(dir, "block.rs"), "-o", join(dir, "block.js"), ...extern], { cwd: root, stderr: "pipe" });
  expect(block.exitCode).not.toBe(0);
  expect(block.stderr.toString()).toContain("a directive is a module's, a function's or a closure's");
});

test("a namespace import is named as the module of its bindings", async () => {
  const dir = fixture("namespace-module");
  writeFileSync(join(dir, "menu.js"), "export function Root() { return 1; }\nexport function Item() { return 2; }\n");
  writeFileSync(join(dir, "lib.rs"), `#[allow(non_snake_case)]
mod ContextMenu {
    unsafe extern "Rust" {
        #[link_name = "./menu.js#*.Root"]
        pub safe fn Root() -> u32;
        #[link_name = "./menu.js#*.Item"]
        pub safe fn Item() -> u32;
    }
}

pub fn both() -> u32 {
    ContextMenu::Root() + ContextMenu::Item()
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain('import * as ContextMenu from "./menu.js";');
  expect(js).toContain("ContextMenu.Root() + ContextMenu.Item()");
  const lib = await import(join(dir, "lib.js"));
  expect(lib.both()).toBe(3);
});

// A default import used only for its statics, a class's in a module named
// for it, `mod greeter`, is named as the class, `Greeter.hello()`, as
// Next.js's docs write `App.getInitialProps`.
test("a default import of a class's statics is named as the class", async () => {
  const dir = fixture("default-statics");
  writeFileSync(join(dir, "greeter.js"), "export default class Greeter {\n  static hello() { return 1; }\n  static bye() { return 2; }\n}\n");
  writeFileSync(join(dir, "lib.rs"), `mod greeter {
    unsafe extern "Rust" {
        #[link_name = "./greeter.js#default.hello"]
        pub safe fn hello() -> u32;
        #[link_name = "./greeter.js#default.bye"]
        pub safe fn bye() -> u32;
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
  expect(readFileSync(join(dir, "index.js"), "utf8")).toContain('export { page as default, props } from "./page.js";');
  expect(readFileSync(join(dir, "only.js"), "utf8")).toContain('export { page as default } from "./page.js";');
  const index = await import(join(dir, "index.js"));
  expect([index.default(), index.props()]).toEqual([1, 2]);
});

// ADR 0267: what a module runs when it's loaded, `js::on_load!`, is its
// JS's own statements, as react.dev's Page prefetches CodeBlock with a bare
// `import()`: rustc checks them as a function's body nothing calls.
test("js::on_load! is what the module runs when it's loaded", async () => {
  const withWeb = ["--", "--extern", `webapi=${join(target, "libwebapi.rmeta")}`, "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];
  const dir = fixture("on-load");
  writeFileSync(join(dir, "lib.rs"), `unsafe extern "Rust" {
    #[link_name = "globalThis.loaded"]
    safe fn loaded(what: &str);
}

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
  expect(at("const FIRST =")).toBeLessThan(at("const SEEN ="));
  expect(at("const LATER =")).toBeLessThan(at("function read_later("));
  // After what they read: LATER, directly or through read_later; SEEN.
  expect(at("const LATER =")).toBeLessThan(at("const AFTER ="));
  expect(at("const LATER =")).toBeLessThan(at("const CALLED ="));
  expect(at("const SEEN =")).toBeLessThan(at(/seen\.value =/i));
  const { values } = await import(join(dir, "lib.js"));
  expect(values()).toEqual([2, 11, 11, 1]);
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
  buildWebapi();
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
