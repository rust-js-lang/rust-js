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
  // Every name fb's file exports, as upstream writes it (#593).
  expect(testing).toContain('export * from "./fb.js";\n');
  expect(readFileSync(join(dir, "app", "nested.js"), "utf8")).toContain('export * from "./testing.js";\n');
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

// #593: a glob whose names are every one its module's file exports but
// `default` is `export * from`, as upstream's ReactDOMTestingFB writes it.
// JS's `export *` re-exports each of them, so the names stay listed where
// the glob's aren't all: a private item its module exports for its own
// private module, which the glob doesn't see, and a name a private `use` of
// the glob's module shadows, as Rust's glob is shadowed and JS's isn't.
test("a glob of every name its module's file exports is export * from", async () => {
  const dir = fixture("glob-every-export");
  writeFileSync(join(dir, "lib.rs"), `pub mod all {
    pub fn one() -> u32 {
        1
    }

    pub const TWO: u32 = 2;
}

pub mod shared {
    mod inner {
        pub fn four() -> u32 {
            super::helper()
        }
    }

    pub fn three() -> u32 {
        3
    }

    fn helper() -> u32 {
        4
    }

    pub fn four() -> u32 {
        inner::four()
    }
}

pub mod other {
    pub fn five() -> u32 {
        5
    }
}

pub mod shadowed {
    pub fn five() -> u32 {
        50
    }

    pub fn six() -> u32 {
        6
    }
}

pub mod every {
    pub use crate::all::*;
}

pub mod some {
    pub use crate::shared::*;
}

pub mod past {
    use crate::other::five;
    pub use crate::shadowed::*;

    pub fn uses() -> u32 {
        five()
    }
}

pub fn total() -> u32 {
    every::one() + every::TWO + some::three() + some::four() + past::six() + past::uses()
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "out", "lib.js"), "--", ...js]);
  const read = (file: string) => readFileSync(join(dir, "out", file), "utf8");
  expect(read("every.js")).toContain('export * from "./all.js";\n');
  expect(read("some.js")).toContain('export { three, four } from "./shared.js";\n');
  expect(read("past.js")).toContain('export { six } from "./shadowed.js";\n');
  const keys = async (file: string) => Object.keys(await import(join(dir, "out", file))).sort();
  expect([await keys("every.js"), await keys("some.js"), await keys("past.js")]).toEqual([["TWO", "one"], ["four", "three"], ["six", "uses"]]);
  expect((await import(join(dir, "out", "lib.js"))).total()).toBe(21);
});

// React's markup Flight client config, `pub use
// react_client::react_client_console_config_plain::*;`, printed `export
// {bindToConsole} from 'react-client/src/ReactClientConsoleConfigPlain'`,
// where upstream writes `export * from` (rd801, #629): a glob of a
// library's module whose names are each its file's exports is `export *
// from`, as #593 prints the crate's own, by what the library's manifest
// says its file exports but its default, which a glob never gives; one of
// some of them, a name the module's own item shadows, or of its default by
// its Rust name, which `export *` leaves out, lists them.
test("a glob of a library's module of every name its file exports is export * from", async () => {
  const dir = fixture("glob-library-every");
  writeFileSync(join(dir, "dep.rs"), `pub mod console {
    pub fn bind_to_console(n: f64) -> f64 {
        n + 1.0
    }

    pub const BADGE: f64 = 2.0;

    fn fallback(n: f64) -> f64 {
        n
    }

    js::export_default!(fallback);
}

pub mod entry {
    pub fn start(n: f64) -> f64 {
        n * 7.0
    }

    js::export_default!(start);
}

pub mod debug {
    pub fn check_eval(n: f64) -> f64 {
        n * 3.0
    }

    pub fn other(n: f64) -> f64 {
        n * 5.0
    }
}
`);
  run([compiler, join(dir, "dep.rs"), "-o", join(dir, "dep", "lib.js"), "--library", "--manifest", join(dir, "dep", "lib.manifest.json"),
    "--", "--crate-name", "dep", `--emit=metadata=${join(dir, "libdep.rmeta")}`, ...js]);
  writeFileSync(join(dir, "app.rs"), `pub mod config {
    pub use dep::console::*;
}

pub mod started {
    pub use dep::entry::*;
}

pub mod some {
    pub use dep::debug::*;

    pub fn other(n: f64) -> f64 {
        n
    }
}
`);
  const p = runSync([compiler, join(dir, "app.rs"), "-o", join(dir, "app", "lib.js"), "--dependency", join(dir, "dep", "lib.manifest.json"),
    "--", "--crate-name", "app", "--extern", `dep=${join(dir, "libdep.rmeta")}`, "-L", dir, ...js], root, 60_000);
  expect([p.code, p.stderr]).toEqual([0, ""]);
  const read = (file: string) => readFileSync(join(dir, "app", file), "utf8");
  expect(read("config.js")).toContain('export * from "../dep/console.js";\n');
  expect(read("some.js")).toContain('export { checkEval } from "../dep/debug.js";\n');
  // Its default, which `export *` leaves out, the glob's `start`.
  expect(read("started.js")).toContain('export { default as start } from "../dep/entry.js";\n');
  const config = await import(join(dir, "app", "config.js"));
  expect([Object.keys(config).sort(), config.bindToConsole(1), config.BADGE]).toEqual([["BADGE", "bindToConsole"], 2, 2]);
  expect(Object.keys(await import(join(dir, "app", "some.js"))).sort()).toEqual(["checkEval", "other"]);
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

// rustc's imports/import-glob-crate.rs: the crate root's `use
// std::mem::*;` and an unused function's `mod foo { pub use *; }`, of
// edition 2015, which gives `foo` std's names within the crate. Since #592
// each was refused, "a `pub use` of every name of a module, of `forget`",
// where the program passed before. std's items have no JS module, a call
// of one is std's wherever it's named, and no JS outside the crate sees
// those names, so the glob says nothing of them; of what it gives JS, the
// crate's own, it says each. A public glob of std's is refused still.
test("a glob of std's names the crate alone sees says nothing of them", async () => {
  const dir = fixture("glob-of-std-within-the-crate");
  writeFileSync(join(dir, "main.rs"), `use std::mem::*;

pub fn bytes() -> usize {
    size_of::<u16>()
}

pub fn main() {
    mod foo {
        pub use *;
    }
    assert_eq!(foo::bytes(), 2);
}
`);
  run([compiler, join(dir, "main.rs"), "-o", join(dir, "main.js"), "--", "--edition", "2015", "-Aunused_imports"]);
  const lib = await import(join(dir, "main.js"));
  expect([lib.main(), lib.bytes()]).toEqual([undefined, 2]);
  writeFileSync(join(dir, "public.rs"), "pub mod all {\n    pub use std::mem::*;\n}\n");
  const p = runSync([compiler, join(dir, "public.rs"), "-o", join(dir, "public.js"), "--", "--crate-type=lib"], root, 60_000);
  expect([p.code === 0, p.stderr]).toEqual([false, expect.stringContaining("a `pub use` of every name of a module, of `forget`")]);
});
