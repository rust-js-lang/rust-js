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
