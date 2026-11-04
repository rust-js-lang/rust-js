// What a `pub use` re-exports through another of the crate's modules that
// re-exports it, React's experimental entries' `export { Children,
// Component, Fragment, .. } from './src/ReactClient'`: from that module,
// by the name it re-exports it as, whatever it is, a function, a constant,
// a library's item, a class or a module (rd338, ADR 0874, amended).

import { beforeAll, expect, test } from "bun:test";
import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { buildReact, compiler, fixture, run, target } from "./support";

beforeAll(buildReact, 600_000);

const js = ["--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];

test("a pub use through a module that re-exports its item is that module's, by its name there", async () => {
  const dir = fixture("reexports-through");
  // A library's constant, React's shared `REACT_FRAGMENT_TYPE`.
  mkdirSync(join(dir, "dep"));
  writeFileSync(join(dir, "dep.rs"), 'pub mod symbols {\n    pub const FRAGMENT_TYPE: &str = "react.fragment";\n}\n');
  run([compiler, join(dir, "dep.rs"), "-o", join(dir, "dep", "lib.js"), "--library", "--manifest", join(dir, "dep", "lib.manifest.json"),
    "--", "--crate-name", "dep", `--emit=metadata=${join(dir, "libdep.rmeta")}`, ...js]);
  mkdirSync(join(dir, "app"));
  writeFileSync(join(dir, "app", "lib.rs"), `pub mod again;
mod base;
mod client;
mod hooks;
pub mod runtime;
mod symbols;

pub use client::{
    __COMPILER_RUNTIME, Component, FRAGMENT, LOCAL_FRAGMENT, use_state,
};
#[cfg_attr(rust_js, rust_js::name = "unstable_Fragment")]
pub use client::FRAGMENT as UNSTABLE_FRAGMENT;
`);
  writeFileSync(join(dir, "app", "client.rs"), `pub use crate::base::Component;
#[cfg_attr(rust_js, rust_js::name = "Fragment")]
pub use dep::symbols::FRAGMENT_TYPE as FRAGMENT;
#[cfg_attr(rust_js, rust_js::name = "LocalFragment")]
pub use crate::symbols::LOCAL_FRAGMENT_TYPE as LOCAL_FRAGMENT;
#[cfg_attr(rust_js, rust_js::name = "__COMPILER_RUNTIME")]
pub use crate::runtime as __COMPILER_RUNTIME;
pub use crate::hooks::use_state;
`);
  writeFileSync(join(dir, "app", "again.rs"), '#[cfg_attr(rust_js, rust_js::name = "Frag")]\npub use crate::client::FRAGMENT;\n');
  writeFileSync(join(dir, "app", "hooks.rs"), "pub fn use_state() -> u32 {\n    1\n}\n");
  writeFileSync(join(dir, "app", "symbols.rs"), 'pub const LOCAL_FRAGMENT_TYPE: &str = "local.fragment";\n');
  writeFileSync(join(dir, "app", "runtime.rs"), "pub fn c() -> u32 {\n    3\n}\n");
  writeFileSync(join(dir, "app", "base.rs"), `#[cfg_attr(rust_js, rust_js::class)]
pub struct Component {
    pub props: u32,
}

#[cfg_attr(rust_js, rust_js::class)]
impl Component {
    pub fn new(props: u32) -> Self {
        Component { props }
    }
}
`);
  run([compiler, "--declarations", join(dir, "app", "lib.rs"), "-o", join(dir, "app", "lib.js"), "--dependency", join(dir, "dep", "lib.manifest.json"),
    "--", "--crate-name", "app", "--extern", `dep=${join(dir, "libdep.rmeta")}`, "-L", dir, ...js]);
  const lib = readFileSync(join(dir, "app", "lib.js"), "utf8");
  expect(lib).toContain(`export { __COMPILER_RUNTIME, Component, Fragment, LocalFragment, useState } from "./client.js";
export { Fragment as unstable_Fragment } from "./client.js";
`);
  // A \`#[rust_js::name]\` of a \`use\` that keeps the path's name names it.
  expect(readFileSync(join(dir, "app", "again.js"), "utf8")).toContain('export { Fragment as Frag } from "./client.js";\n');
  expect(readFileSync(join(dir, "app", "lib.d.ts"), "utf8")).toContain('from "./client.js";');
  const app = await import(join(dir, "app", "lib.js"));
  const client = await import(join(dir, "app", "client.js"));
  expect([app.Fragment, app.unstable_Fragment, app.LocalFragment, app.useState(), app.__COMPILER_RUNTIME.c()]).toEqual(["react.fragment", "react.fragment", "local.fragment", 1, 3]);
  expect([app.Component, app.Fragment]).toEqual([client.Component, client.Fragment]);
  expect(new app.Component(2).props).toBe(2);
  expect((await import(join(dir, "app", "again.js"))).Frag).toBe("react.fragment");
});

// And a private `use` through such a module imports from it, by the name it
// re-exports the item as, as upstream's react-markup imports
// `createResumableState` from './ReactFizzConfigMarkup', which re-exports it
// from ReactFizzConfigDOM: a library's function and the crate's alike. One
// whose re-exporting module's load reaches the importer's, a load cycle,
// is from its home (rd650, ADR 0874, amended).
test("a use through a module that re-exports its item imports it from there", async () => {
  const dir = fixture("imports-through");
  mkdirSync(join(dir, "dep"));
  writeFileSync(join(dir, "dep.rs"), "pub mod config {\n    pub fn create_state() -> u32 {\n        7\n    }\n\n    pub fn other() -> u32 {\n        8\n    }\n}\n");
  run([compiler, join(dir, "dep.rs"), "-o", join(dir, "dep", "lib.js"), "--library", "--manifest", join(dir, "dep", "lib.manifest.json"),
    "--", "--crate-name", "dep", `--emit=metadata=${join(dir, "libdep.rmeta")}`, ...js]);
  mkdirSync(join(dir, "app"));
  writeFileSync(join(dir, "app", "lib.rs"), `mod config_markup;
mod home;
pub mod markup;
pub mod renamed;
pub mod user;

pub use home::helper;
pub use user::cycled;
`);
  writeFileSync(join(dir, "app", "config_markup.rs"), `pub use crate::home::helper;
pub use dep::config::{create_state, other};

pub fn own() -> u32 {
    1
}
`);
  writeFileSync(join(dir, "app", "home.rs"), "pub fn helper() -> u32 {\n    2\n}\n");
  writeFileSync(join(dir, "app", "markup.rs"), `use crate::config_markup::{create_state, helper, own};
use dep::config::other;

pub fn render() -> u32 {
    create_state() + helper() + own() + other()
}
`);
  writeFileSync(join(dir, "app", "renamed.rs"), `use crate::config_markup::create_state as make_state;
use crate::config_markup::helper as help;

pub fn render() -> u32 {
    make_state() + help()
}
`);
  writeFileSync(join(dir, "app", "user.rs"), "use crate::helper;\n\npub fn cycled() -> u32 {\n    helper() + 1\n}\n");
  run([compiler, join(dir, "app", "lib.rs"), "-o", join(dir, "app", "lib.js"), "--dependency", join(dir, "dep", "lib.manifest.json"),
    "--", "--crate-name", "app", "--extern", `dep=${join(dir, "libdep.rmeta")}`, "-L", dir, ...js]);
  const markup = readFileSync(join(dir, "app", "markup.js"), "utf8");
  expect(markup).toContain('import { createState, helper, own } from "./config_markup.js";\nimport { other } from "../dep/config.js";\n');
  expect(markup).not.toContain("home.js");
  expect(readFileSync(join(dir, "app", "renamed.js"), "utf8")).toContain('import { createState as makeState, helper as help } from "./config_markup.js";\n');
  // The crate's root re-exports `cycled` from user.js: user.js importing
  // `helper` from it would load it first, a cycle, so it's from its home.
  expect(readFileSync(join(dir, "app", "user.js"), "utf8")).toContain('import { helper } from "./home.js";\n');
  const app = await import(join(dir, "app", "markup.js"));
  expect(app.render()).toBe(18);
  expect((await import(join(dir, "app", "renamed.js"))).render()).toBe(9);
  expect((await import(join(dir, "app", "lib.js"))).cycled()).toBe(3);
});

// A library's item a use names through another of the library's modules,
// one whose pub use re-exports it, is imported from that module's file,
// as upstream's ReactART imports `defaultOnCaughtError` with
// `createContainer` from 'react-reconciler/src/ReactFiberReconciler':
// one import of each use (rd704, ADR 0874, amended).
test("a use through a library's module that re-exports its item imports it from there", async () => {
  const dir = fixture("imports-through-library");
  mkdirSync(join(dir, "dep"));
  writeFileSync(join(dir, "dep.rs"), `pub mod error_logger {
    pub fn default_on_caught_error() -> u32 {
        7
    }

    pub fn log() -> u32 {
        1
    }
}

pub mod reconciler {
    pub use crate::error_logger::default_on_caught_error;
    pub use crate::error_logger::log as log_error;

    pub fn create_container() -> u32 {
        8
    }
}
`);
  run([compiler, join(dir, "dep.rs"), "-o", join(dir, "dep", "lib.js"), "--library", "--manifest", join(dir, "dep", "lib.manifest.json"),
    "--", "--crate-name", "dep", `--emit=metadata=${join(dir, "libdep.rmeta")}`, ...js]);
  mkdirSync(join(dir, "app"));
  writeFileSync(join(dir, "app", "lib.rs"), `use dep::error_logger::log;
use dep::reconciler::{create_container, default_on_caught_error, log_error};

pub fn render() -> u32 {
    create_container() + default_on_caught_error() + log_error() + log()
}
`);
  run([compiler, join(dir, "app", "lib.rs"), "-o", join(dir, "app", "lib.js"), "--dependency", join(dir, "dep", "lib.manifest.json"),
    "--", "--crate-name", "app", "--extern", `dep=${join(dir, "libdep.rmeta")}`, "-L", dir, ...js]);
  const app = readFileSync(join(dir, "app", "lib.js"), "utf8");
  expect(app).toContain('import { createContainer, defaultOnCaughtError } from "../dep/reconciler.js";\n');
  // One item is one import of the module, from where the first path
  // written names it, its home here, named as the module's `use` renames
  // it (ADR 0202, amended).
  expect(app).toContain('import { log as logError } from "../dep/error_logger.js";\n');
  expect((await import(join(dir, "app", "lib.js"))).render()).toBe(17);
});

// React's SchedulerNative reads `use crate as scheduler;`'s
// `scheduler::UNSTABLE_USER_BLOCKING_PRIORITY`, the root's `pub use
// scheduler_priorities::USER_BLOCKING_PRIORITY as ..`, and its functions
// through it: upstream's `Scheduler.unstable_UserBlockingPriority`, as
// `Scheduler.unstable_scheduleCallback`. A constant read through a module
// that re-exports it was imported from its definer, `import {
// UserBlockingPriority } from '../SchedulerPriorities.js'`, where a
// function so read is the module's (rd137): a `const` or a `static` is
// too, by the name the module re-exports it as (rd800, #628).
test("a constant read through a module that re-exports it is that module's", async () => {
  const dir = fixture("constant-through-reexporter");
  writeFileSync(join(dir, "lib.rs"), `pub mod native;
pub mod priorities;

#[cfg_attr(rust_js, rust_js::name = "unstable_UserBlockingPriority")]
pub use priorities::USER_BLOCKING_PRIORITY as UNSTABLE_USER_BLOCKING_PRIORITY;

pub fn schedule(n: u32) -> u32 {
    n + 1
}
`);
  writeFileSync(join(dir, "priorities.rs"), "pub const USER_BLOCKING_PRIORITY: u32 = 2;\n");
  writeFileSync(join(dir, "native.rs"), `use crate as scheduler;

pub fn blocking() -> u32 {
    scheduler::UNSTABLE_USER_BLOCKING_PRIORITY
}

pub fn scheduled(n: u32) -> u32 {
    scheduler::schedule(n) * 2
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "out", "lib.js")]);
  const native = readFileSync(join(dir, "out", "native.js"), "utf8");
  expect(native).toContain('import * as Lib from "./lib.js";\n');
  expect(native).toContain("return Lib.unstable_UserBlockingPriority;\n");
  expect(native).not.toContain("priorities.js");
  const lib = await import(join(dir, "out", "native.js"));
  expect([lib.blocking(), lib.scheduled(1)]).toEqual([2, 4]);
});

