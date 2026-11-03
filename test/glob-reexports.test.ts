// What the compiler writes of a `pub use m::*;` of a module of the crate's
// that re-exports what it gives by other names (#592): each name as that
// module exports it, from it, as upstream's ReactDOMTestingFB re-exports
// ReactDOMFB's, `export * from './ReactDOMFB'`; and a module of such
// globs alone is a file of its own. Each test is a program of its own,
// compiled and run.

import { beforeAll, expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { runSync } from "./child";
import { buildReact, compiler, fixture, root, run, target } from "./support";

beforeAll(buildReact, 600_000);

const js = ["--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];

// #592: ReactDOMClientFB re-exports react-dom-bindings' `create_event_handle`
// as `unstable_createEventHandle` and shared's `REACT_VERSION` as `version`;
// ReactDOMFB re-exports those by name, and ReactDOMTestingFB is `pub use
// crate::react_dom_fb::*;`. Each of its names was its definer's,
// `createEventHandle` from react-dom-bindings, so a JS importer of
// `unstable_createEventHandle` got `undefined`.
test("a glob re-exports each name as the module it names exports it", async () => {
  const dir = fixture("glob-renamed-reexports");
  writeFileSync(join(dir, "dep.rs"), `pub fn create_event_handle(n: f64) -> f64 {
    n + 1.0
}

pub fn plain(n: f64) -> f64 {
    n * 2.0
}

pub const REACT_VERSION: &str = "19";
`);
  run([compiler, join(dir, "dep.rs"), "-o", join(dir, "dep", "lib.js"), "--library", "--manifest", join(dir, "dep", "lib.manifest.json"),
    "--", "--crate-name", "dep", `--emit=metadata=${join(dir, "libdep.rmeta")}`, ...js]);
  writeFileSync(join(dir, "app.rs"), `pub mod bindings {
    pub fn batched_updates(n: f64) -> f64 {
        n * 10.0
    }

    pub const LIMIT: u32 = 3;
}

pub mod client {
    #[cfg_attr(rust_js, rust_js::name = "unstable_createEventHandle")]
    pub use dep::create_event_handle as unstable_create_event_handle;
    #[cfg_attr(rust_js, rust_js::name = "version")]
    pub use dep::REACT_VERSION as VERSION;
    pub use dep::plain;
    #[cfg_attr(rust_js, rust_js::name = "unstable_batchedUpdates")]
    pub use crate::bindings::batched_updates as unstable_batched_updates;
    pub use crate::bindings::LIMIT as MAX;

    pub fn own(n: f64) -> f64 {
        n - 1.0
    }
}

pub mod fb {
    pub use crate::client::{MAX, VERSION, own, plain, unstable_batched_updates, unstable_create_event_handle};
}

pub mod testing {
    pub use crate::fb::*;
}

pub mod nested {
    pub use crate::testing::*;
}

pub fn through() -> f64 {
    testing::unstable_create_event_handle(1.0) + nested::unstable_batched_updates(1.0) + f64::from(testing::MAX)
}
`);
  run([compiler, join(dir, "app.rs"), "-o", join(dir, "app", "lib.js"), "--dependency", join(dir, "dep", "lib.manifest.json"),
    "--", "--crate-name", "app", "--extern", `dep=${join(dir, "libdep.rmeta")}`, "-L", dir, ...js]);
  const testing = readFileSync(join(dir, "app", "testing.js"), "utf8");
  expect(testing).toContain(
    'export {\n  own,\n  MAX,\n  plain,\n  version,\n  unstable_batchedUpdates,\n  unstable_createEventHandle,\n} from "./fb.js";\n',
  );
  expect(readFileSync(join(dir, "app", "nested.js"), "utf8")).toContain('} from "./testing.js";\n');
  const names = ["MAX", "own", "plain", "unstable_batchedUpdates", "unstable_createEventHandle", "version"];
  for (const file of ["testing.js", "nested.js"]) {
    const module = await import(join(dir, "app", file));
    expect(Object.keys(module).sort()).toEqual(names);
    expect([module.unstable_createEventHandle(1), module.unstable_batchedUpdates(2), module.version, module.MAX, module.plain(3), module.own(4)])
      .toEqual([2, 20, "19", 3, 6, 3]);
  }
  const lib = await import(join(dir, "app", "lib.js"));
  expect(lib.through()).toBe(15);
});

// #592: of a library's module, a glob re-exports each name from the file
// that has it, by its own name; one the module renames, `pub use
// super::create_event_handle as unstable_create_event_handle`, has a name
// there the library's manifest doesn't say, so it's refused, never
// exported by its definer's name.
test("a glob of a library's module that renames an item is refused", () => {
  const dir = fixture("glob-library-renamed");
  writeFileSync(join(dir, "dep.rs"), `pub fn create_event_handle(n: f64) -> f64 {
    n + 1.0
}

pub mod client {
    pub use super::create_event_handle as unstable_create_event_handle;
}
`);
  run([compiler, join(dir, "dep.rs"), "-o", join(dir, "dep", "lib.js"), "--library", "--manifest", join(dir, "dep", "lib.manifest.json"),
    "--", "--crate-name", "dep", `--emit=metadata=${join(dir, "libdep.rmeta")}`, ...js]);
  writeFileSync(join(dir, "app.rs"), "pub use dep::client::*;\n");
  const p = runSync([compiler, join(dir, "app.rs"), "-o", join(dir, "app", "lib.js"), "--dependency", join(dir, "dep", "lib.manifest.json"),
    "--", "--crate-name", "app", "--extern", `dep=${join(dir, "libdep.rmeta")}`, "-L", dir, ...js], root, 60_000);
  expect(p.code).not.toBe(0);
  expect(p.stderr).toContain("a `pub use` of every name of a module, of `unstable_create_event_handle`");
});
