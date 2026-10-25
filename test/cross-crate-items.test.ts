// What a crate reads of a library's items, as it reads its own: a `const`
// by its name, imported from the library's module (ADR 0770). Each test
// compiles a library and a crate that uses it, and runs what it wrote.

import { beforeAll, expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { buildReact, compiler, fixture, run, target } from "./support";

beforeAll(buildReact, 600_000);

const js = ["--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];

/** `dep`, a library, and `app`, a crate that uses it, compiled in `dir`:
 * each one's JS. */
function compiled(dir: string, dep: string, app: string): { dep: string; app: string } {
  writeFileSync(join(dir, "dep.rs"), dep);
  run([compiler, join(dir, "dep.rs"), "-o", join(dir, "dep", "lib.js"), "--library", "--manifest", join(dir, "dep", "lib.manifest.json"),
    "--", "--crate-name", "dep", `--emit=metadata=${join(dir, "libdep.rmeta")}`, ...js]);
  writeFileSync(join(dir, "app.rs"), app);
  run([compiler, join(dir, "app.rs"), "-o", join(dir, "app", "lib.js"), "--dependency", join(dir, "dep", "lib.manifest.json"),
    "--", "--crate-name", "app", "--extern", `dep=${join(dir, "libdep.rmeta")}`, "-L", dir, ...js]);
  return { dep: readFileSync(join(dir, "dep", "lib.js"), "utf8"), app: readFileSync(join(dir, "app", "lib.js"), "utf8") };
}

// React's `REACT_MEMO_TYPE`, shared's, which ReactMemo imports by its name
// from `shared/ReactSymbols`: read in its crate and in another alike.
test("a library's const is imported by its name, as its own crate reads it", async () => {
  const dir = fixture("cross-crate-consts");
  const { dep, app } = compiled(dir, `use js::SymbolFor;

pub const REACT_MEMO_TYPE: SymbolFor = SymbolFor("react.memo");
pub const LIMIT: u32 = 5;
pub const NAME: &str = "memo";

pub fn local() -> (SymbolFor<'static>, u32) {
    (REACT_MEMO_TYPE, LIMIT)
}
`, `use js::SymbolFor;

pub fn memo() -> (SymbolFor<'static>, u32, &'static str) {
    (dep::REACT_MEMO_TYPE, dep::LIMIT, dep::NAME)
}
`);
  expect(dep).toContain("  return [REACT_MEMO_TYPE, LIMIT];\n");
  expect(app).toContain('import { LIMIT, NAME, REACT_MEMO_TYPE } from "../dep/lib.js";\n');
  expect(app).toContain("  return [REACT_MEMO_TYPE, LIMIT, NAME];\n");
  const { memo } = await import(join(dir, "app", "lib.js"));
  expect(memo()).toEqual([Symbol.for("react.memo"), 5, "memo"]);
});

// A `#[repr]` enum's variant is its module's `const` (ADR 0722, amended):
// a library's, imported by its name, tested, made and matched as its own
// crate's are.
test("a library's repr enum's variant is imported by its name", async () => {
  const dir = fixture("cross-crate-variants");
  const { dep, app } = compiled(dir, `#[derive(Clone, Copy, PartialEq)]
#[repr(i8)]
pub enum Status {
    Uninitialized = -1,
    Pending = 0,
    Resolved = 1,
}

pub fn started(status: Status) -> bool {
    status != Status::Uninitialized
}
`, `use dep::Status;

pub fn next(status: Status) -> Status {
    match status {
        Status::Uninitialized => Status::Pending,
        Status::Pending | Status::Resolved => Status::Resolved,
    }
}

pub fn settled(status: Status) -> bool {
    dep::started(status) && status == Status::Resolved
}

pub mod report {
    use dep::Status;

    pub fn word(status: Status) -> &'static str {
        match status {
            Status::Uninitialized => "new",
            Status::Pending | Status::Resolved => "started",
        }
    }
}
`);
  expect(dep).toContain("export const Uninitialized = -1;\nexport const Pending = 0;\nexport const Resolved = 1;\n");
  expect(dep).toContain("return status !== Uninitialized;");
  expect(app).toContain('import { Pending, Resolved, Uninitialized, started } from "../dep/lib.js";\n');
  expect(app).toContain("  if (status === Uninitialized) {\n    return Pending;\n  }\n  return Resolved;\n");
  expect(app).toContain("return started(status) && status === Resolved;");
  // Of the variants a `match` tests, those whose test it writes: its last
  // arm's, the rest, is no import.
  const report = readFileSync(join(dir, "app", "report.js"), "utf8");
  expect(report).toContain('import { Uninitialized } from "../dep/lib.js";\n');
  const { next, settled } = await import(join(dir, "app", "lib.js"));
  expect([next(-1), next(0), next(1), settled(1), settled(0)]).toEqual([0, 1, 1, true, false]);
  const { word } = await import(join(dir, "app", "report.js"));
  expect([word(-1), word(0), word(1)]).toEqual(["new", "started", "started"]);
});

// React's `shared/noop`, `export default function noop() {}`, which
// ReactLazy imports as `import noop from 'shared/noop'`: a module's
// default is imported as its default, in its crate and in another,
// whichever form of `js::export_default!` makes it, and through a
// `pub use` of it (ADRs 0611, 0773).
test("a library's default export is imported as its default", async () => {
  const dir = fixture("cross-crate-defaults");
  const { dep, app } = compiled(dir, `pub mod noop {
    js::export_default! {
        pub fn noop() -> u32 {
            7
        }
    }
}

pub mod version {
    js::export_default! {
        pub const VERSION: &str = "19.3.0";
    }

    pub use self::VERSION as RELEASE;
}

pub mod page {
    pub fn page() -> u32 {
        3
    }

    js::export_default!(page);
}

pub mod reexported {
    pub use crate::noop::noop;
    pub use crate::version::VERSION;
}

pub mod again {
    js::export_default!(crate::noop::noop);
}

pub mod helpers {
    pub(crate) fn twice(n: u32) -> u32 {
        n * 2
    }
}

pub mod twice {
    js::export_default!(crate::helpers::twice);
}

pub fn local() -> (u32, u32, &'static str, u32) {
    (noop::noop(), page::page(), version::VERSION, helpers::twice(2))
}
`, `pub fn called() -> (u32, u32, &'static str, u32, &'static str) {
    let noop = dep::noop::noop;
    (
        noop(),
        dep::reexported::noop(),
        dep::version::VERSION,
        dep::page::page(),
        dep::version::RELEASE,
    )
}
`);
  const read = (file: string) => readFileSync(join(dir, "dep", file), "utf8");
  expect(read("noop.js")).toContain("\nexport default function noop() {\n  return 7;\n}\n");
  // A default its module names by another name is declared, as it's read
  // by its name (ADR 0625, amended).
  expect(read("version.js")).toContain('\nconst VERSION = "19.3.0";\n\nexport default VERSION;\n\nexport { VERSION as RELEASE };\n');
  // Another module's default is what that one exports: its default, or
  // its name, where it isn't its own module's.
  expect(dep).toContain('import { twice } from "./helpers.js";\nimport noop from "./noop.js";\nimport page from "./page.js";\nimport VERSION from "./version.js";\n');
  expect(read("reexported.js")).toContain('export { default as noop } from "./noop.js";\nexport { default as VERSION } from "./version.js";\n');
  expect(read("again.js")).toContain('export { default } from "./noop.js";\n');
  expect(read("twice.js")).toContain('export { twice as default } from "./helpers.js";\n');
  const manifest = JSON.parse(read("lib.manifest.json")).library;
  expect(manifest.items.filter((item: any) => item.exported_as === "default").map((item: any) => item.export)).toEqual(["noop", "page", "VERSION"]);
  // Whichever path names it, it's its module's default, imported once.
  expect(app).toContain('import noop from "../dep/noop.js";\nimport page from "../dep/page.js";\nimport VERSION from "../dep/version.js";\n');
  expect(app).toContain("  return [noop(), noop(), VERSION, page(), VERSION];\n");
  const { local } = await import(join(dir, "dep", "lib.js"));
  expect(local()).toEqual([7, 3, "19.3.0", 4]);
  const { default: again } = await import(join(dir, "dep", "again.js"));
  expect(again()).toBe(7);
  const { default: twice } = await import(join(dir, "dep", "twice.js"));
  expect(twice(5)).toBe(10);
  const { called } = await import(join(dir, "app", "lib.js"));
  expect(called()).toEqual([7, 7, "19.3.0", 3, "19.3.0"]);
});

// A library says whether it makes a `dyn Any` of what drops (ADR 0691,
// amended): React's react crate catches what its scope threw as JS's
// `catch (error)`, though shared, its library, has `Drop` guards, as none
// of them is made a `dyn Any`. One that resumes an unwind of a guard drops
// it where the crate's `catch_unwind` drops what it caught.
test("a library's dyn Any drops what it holds where the library makes one of what drops", async () => {
  const dir = fixture("cross-crate-any-drops");
  const { dep, app } = compiled(dir, `use std::panic::resume_unwind;

pub struct Restore(pub u32);

impl Drop for Restore {
    fn drop(&mut self) {
        println!("restored {}", self.0);
    }
}

pub fn guarded(n: u32) -> u32 {
    let _restore = Restore(n);
    n + 1
}

pub fn resumed(n: u32) {
    if n > 2 {
        resume_unwind(Box::new(Restore(n)));
    }
}
`, `use std::panic::{AssertUnwindSafe, catch_unwind};

pub fn caught(scope: &dyn Fn()) -> u32 {
    if let Err(error) = catch_unwind(AssertUnwindSafe(scope)) {
        println!("caught {}", error.is::<dep::Restore>());
        return dep::guarded(1);
    }
    0
}

pub fn run(n: u32) -> u32 {
    caught(&|| dep::resumed(n))
}
`);
  expect(JSON.parse(readFileSync(join(dir, "dep", "lib.manifest.json"), "utf8")).library.makes_dropping_anys).toBe(true);
  expect(dep).toContain("$unwinding(");
  expect(app).toContain("error.impl.$drop");
  const logged: unknown[] = [];
  const log = console.log;
  console.log = (...args: unknown[]) => logged.push(args.join(" "));
  try {
    const { run } = await import(join(dir, "app", "lib.js"));
    expect([run(1), run(3)]).toEqual([0, 2]);
  } finally {
    console.log = log;
  }
  expect(logged).toEqual(["caught true", "restored 1", "restored 3"]);
});

test("a library's guards make no dyn Any of the crate's drop", async () => {
  const dir = fixture("cross-crate-no-any-drops");
  const { app } = compiled(dir, `pub struct Restore(pub u32);

impl Drop for Restore {
    fn drop(&mut self) {
        println!("restored {}", self.0);
    }
}

pub fn guarded(n: u32) -> u32 {
    let _restore = Restore(n);
    n + 1
}
`, `use std::panic::{AssertUnwindSafe, catch_unwind};

pub fn caught(scope: &dyn Fn()) -> u32 {
    if let Err(error) = catch_unwind(AssertUnwindSafe(scope)) {
        println!("caught {}", error.is::<dep::Restore>());
        return dep::guarded(1);
    }
    0
}
`);
  expect(JSON.parse(readFileSync(join(dir, "dep", "lib.manifest.json"), "utf8")).library.makes_dropping_anys).toBe(false);
  expect(app).not.toContain("$drop");
  expect(app).toContain("  } catch (thrown) {\n");
  const { caught } = await import(join(dir, "app", "lib.js"));
  expect(caught(() => { throw new Error("no"); })).toBe(2);
});

// React's scheduler exports `getCurrentTime` as `unstable_now` alone: a
// library's module that exports its own item by the name a `pub use`
// gives it is imported by that by another crate, its manifest's item
// saying so, the import named after the item, as its own crate's path
// names it (ADR 0596, amended).
test("a library's item its module exports by another name is imported by that", async () => {
  const dir = fixture("cross-crate-aliases");
  const { dep, app } = compiled(dir, `pub mod scheduler {
    #[cfg_attr(rust_js, rust_js::name = "unstable_now")]
    pub use self::get_current_time as unstable_now;
    pub use self::LIMIT as MAX;

    pub const LIMIT: f64 = 100.0;

    pub fn get_current_time() -> f64 {
        6.0
    }
}

pub fn local() -> f64 {
    scheduler::get_current_time()
}
`, `pub fn called() -> f64 {
    dep::scheduler::get_current_time() + dep::scheduler::unstable_now() + dep::scheduler::MAX
}
`);
  const scheduler = readFileSync(join(dir, "dep", "scheduler.js"), "utf8");
  expect(scheduler).toContain("\nconst LIMIT = 100;\n\nfunction getCurrentTime() {\n  return 6;\n}\n\nexport { getCurrentTime as unstable_now, LIMIT as MAX };\n");
  expect(dep).toContain('import { unstable_now as getCurrentTime } from "./scheduler.js";\n');
  const manifest = JSON.parse(readFileSync(join(dir, "dep", "lib.manifest.json"), "utf8")).library;
  const aliased = manifest.items.filter((item: any) => item.exported_as).map((item: any) => [item.export, item.exported_as]);
  expect(aliased.sort()).toEqual([["LIMIT", "MAX"], ["getCurrentTime", "unstable_now"]]);
  expect(app).toContain('import { MAX as LIMIT, unstable_now as getCurrentTime } from "../dep/scheduler.js";\n');
  expect(app).toContain("  return getCurrentTime() + getCurrentTime() + LIMIT;\n");
  const { called } = await import(join(dir, "app", "lib.js"));
  expect(called()).toBe(112);
});

// React's ReactClient re-exports what `shared` declares, `export
// {REACT_FRAGMENT_TYPE as Fragment, ReactVersion as version, ..}`: a `pub
// use` of a library's constant, function, default and class is `export {
// .. } from` the library's file, by what that file exports it as, a
// binding a module of its holds too, never nothing (ADR 0240, amended).
test("a pub use of a library's item re-exports it from the library's file", async () => {
  const dir = fixture("cross-crate-reexports");
  compiled(dir, `pub const LIMIT: u32 = 9;

pub fn helper() -> u32 {
    LIMIT
}

pub mod version {
    js::export_default! {
        pub const REACT_VERSION: &str = "19.3.0";
    }
}

pub mod shapes {
    #[cfg_attr(rust_js, rust_js::class)]
    pub struct Circle {
        pub radius: f64,
    }

    #[cfg_attr(rust_js, rust_js::class)]
    impl Circle {
        pub fn new(radius: f64) -> Self {
            Circle { radius }
        }
    }
}

pub mod assign {
    pub use js::object::assign;

    js::export_default!(assign);
}
`, `pub use dep::LIMIT;
pub use dep::helper as renamed;
#[cfg_attr(rust_js, rust_js::name = "version")]
pub use dep::version::REACT_VERSION as VERSION;
pub use dep::shapes::Circle;
pub use dep::assign::assign;
`);
  const app = readFileSync(join(dir, "app", "lib.js"), "utf8");
  expect(app).toContain('export { LIMIT, helper as renamed } from "../dep/lib.js";\n');
  expect(app).toContain('export { default as version } from "../dep/version.js";\n');
  expect(app).toContain('export { Circle } from "../dep/shapes.js";\nexport { default as assign } from "../dep/assign.js";\n');
  const module = await import(join(dir, "app", "lib.js"));
  expect([module.LIMIT, module.renamed(), module.version, new module.Circle(2).radius]).toEqual([9, 9, "19.3.0", 2]);
  expect(module.assign).toBe(Object.assign);
});

// React's ReactAct imports shared's `enqueueTask` as `queueMacrotask`, the
// name its Rust `use` gives it: a library's item is named as the module's
// `use` renames it, its default and its name alike (ADR 0202, amended).
test("a library's item is imported by the name a use renames it to", async () => {
  const dir = fixture("cross-crate-renamed");
  const { app } = compiled(dir, `pub mod enqueue {
    js::export_default! {
        pub fn enqueue_task(task: fn() -> u32) -> u32 {
            task() + 1
        }
    }
}

pub fn helper() -> u32 {
    3
}
`, `#[cfg_attr(rust_js, rust_js::name = "queueMacrotask")]
use dep::enqueue::enqueue_task as queue_macrotask;
#[cfg_attr(rust_js, rust_js::name = "helpOut")]
use dep::helper as assist;

pub fn run() -> u32 {
    queue_macrotask(assist) + assist()
}
`);
  // Its \`#[rust_js::name]\` too, as a crate module's is (ADR 0874).
  expect(app).toContain('import queueMacrotask from "../dep/enqueue.js";\n');
  expect(app).toContain('import { helper as helpOut } from "../dep/lib.js";\n');
  expect(app).toContain("  return (queueMacrotask(helpOut) + helpOut()) >>> 0;\n");
  const { run } = await import(join(dir, "app", "lib.js"));
  expect(run()).toBe(7);
});
