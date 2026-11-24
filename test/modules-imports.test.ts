// Where a module's imports and re-exports are, and what names them: where
// their bindings are declared, a `pub use` as written with its comments, a
// namespace import named by its link_name, a binding's rust_js::name, and a
// function read through its module's namespace. Split from modules.test.ts
// by concept (#1334).

import { beforeAll, expect, test } from "bun:test";
import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { buildReact, compiler, fixture, run, target } from "./support";

beforeAll(buildReact, 600_000);

// ADR 0295: a module's imports are one block, as a person orders them:
// packages, then its own relative modules, bindings and the crate's alike,
// each by path; then what it imports for its effect, as written, as
// react.dev's _app has its CSS after; then rust-js's runtime.
test("imports are where their bindings are declared, then by path, then effects, then the runtime", () => {
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

mod ext {
    #[cfg_attr(rust_js, rust_js::link_name = "@scope/alpha#alpha")]
    pub fn alpha() -> u32 {
        unreachable!()
    }
}

pub fn all(items: &[u32], i: usize) -> u32 {
    zeta() + local() + ext::alpha() + helper::h(items, i) + items[i]
}
`);
  buildReact();
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  const top = readFileSync(join(dir, "lib.js"), "utf8");
  const imports = top.split("\n").filter((line) => line.startsWith("import") || line === "").slice(1, 8).join("\n");
  expect(imports).toBe(`import { zeta } from "zeta";
import { local } from "./local";
import { alpha } from "@scope/alpha";
import * as Helper from "./helper.js";
import "./app.css";
import { $index } from "@rust-js/std";
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
  // Its import where its own `use` is, after the `pub use`s before it, a
  // blank line between as the Rust has (ADR 1069).
  expect(js).toContain('export * as math from "./math.js";\n\nimport { area as _areaDEV } from "./shapes.js";\n');
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
  const source = (first: string, second: string) => `#[cfg_attr(rust_js, rust_js::link_name = "${first}")]
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
  const compile = (first: string, second: string) => {
    writeFileSync(join(dir, "lib.rs"), source(first, second));
    run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
    return readFileSync(join(dir, "lib.js"), "utf8");
  };
  // Named PascalCase by what it's imported from where no path names it, as
  // every namespace is (rd114b, ADR 1018).
  const plain = compile("scheduler#*.unstable_now", "scheduler#*.unstable_yieldValue");
  expect(plain).toContain('import * as Scheduler from "scheduler";\n');
  expect(plain).toContain("export const now = Scheduler.unstable_now;\n");
  const js = compile("scheduler#* as Sched.unstable_now", "scheduler#* as Sched.unstable_yieldValue");
  expect(js).toContain('import * as Sched from "scheduler";\n');
  expect(js).toContain("export const now = Sched.unstable_now;\n");
  expect(js).toContain("  Sched.unstable_yieldValue(value);\n");
  for (const second of ["scheduler#*.unstable_yieldValue", "scheduler#* as Other.unstable_yieldValue"]) {
    expect(() => compile("scheduler#* as Sched.unstable_now", second)).toThrow(
      "each `link_name` of its `*` names it the same, `scheduler#* as Name.path`",
    );
  }
  // One that names it as it's named anyway says it twice.
  expect(() => compile("scheduler#* as Scheduler.unstable_now", "scheduler#* as Scheduler.unstable_yieldValue")).toThrow(
    "`scheduler#* as Scheduler` names the namespace as it's named anyway: write `scheduler#*.path`",
  );
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

// rd114 (ADR 1018): a function called through its module's path is read
// through that module's namespace, from a module below too; a path
// through `super` alone names its own module's function by its name; and
// one through a module that re-exports another's is read through the
// re-exporting module's namespace, by the name it re-exports it as,
// React DOM's `SimpleEventPlugin.registerEvents()` (rd137), where its file
// re-exports it.
test("a function called through its module is read through its namespace", async () => {
  const dir = fixture("module-namespace");
  writeFileSync(join(dir, "lib.rs"), `#[path = "events/DOMPluginEventSystem.rs"]
pub mod dom_plugin_event_system;
pub mod plugin;
pub mod plugins;
pub mod shared;

pub fn base() -> u32 {
    1
}

pub fn run() -> u32 {
    plugin::nested::depth() + plugins::doubled(2) + plugins::twice(3) + plugins::paired(1) + dom_plugin_event_system::dispatch()
}
`);
  mkdirSync(join(dir, "events"), { recursive: true });
  writeFileSync(join(dir, "events", "DOMPluginEventSystem.rs"), "pub fn dispatch() -> u32 {\n    4\n}\n");
  writeFileSync(join(dir, "plugin.rs"), `pub fn extract_events(n: u32) -> u32 {
    n * 2
}

pub mod nested {
    pub fn depth() -> u32 {
        super::extract_events(1) + super::super::plugin::extract_events(2) + crate::base()
    }
}
`);
  writeFileSync(join(dir, "plugins.rs"), "pub use crate::shared::doubled;\npub use crate::shared::doubled as twice;\npub(crate) use crate::shared::doubled as paired;\n");
  writeFileSync(join(dir, "shared.rs"), "pub fn doubled(n: u32) -> u32 {\n    n * 2\n}\n");
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "out", "lib.js")]);
  const nested = readFileSync(join(dir, "out", "plugin", "nested.js"), "utf8");
  expect(nested).toContain('import { base } from "../lib.js";\nimport { extractEvents } from "../plugin.js";\nimport * as Plugin from "../plugin.js";\n');
  expect(nested).toContain("return (((extractEvents(1) + Plugin.extractEvents(2)) >>> 0) + base()) >>> 0;");
  const lib = readFileSync(join(dir, "out", "lib.js"), "utf8");
  expect(lib).toContain('import * as Plugins from "./plugins.js";\n');
  // One its file doesn't re-export, `pub(crate) use`, is its own module's.
  expect(lib).toContain('import { doubled } from "./shared.js";\n');
  expect(lib).toContain('import * as Nested from "./plugin/nested.js";\n');
  // A #[path] module's namespace is named as its JS file is, as upstream's
  // file names it (rd114c, ADR 1018).
  expect(lib).toContain('import * as DOMPluginEventSystem from "./events/DOMPluginEventSystem.js";\n');
  expect(lib).toContain("Nested.depth() + Plugins.doubled(2)");
  expect(lib).toContain("Plugins.twice(3)");
  const { run: ran } = await import(join(dir, "out", "lib.js"));
  expect(ran()).toBe(23);
});
