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
  // Through the library's namespace, as their paths name them (ADR 1018).
  expect(app).toContain('import * as Dep from "../dep/lib.js";\n');
  expect(app).toContain("  return [Dep.REACT_MEMO_TYPE, Dep.LIMIT, Dep.NAME];\n");
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
  // Its function called through its root, its namespace's (ADR 1018).
  expect(app).toContain('import { Pending, Resolved, Uninitialized } from "../dep/lib.js";\nimport * as Dep from "../dep/lib.js";\n');
  expect(app).toContain("  return status === Uninitialized ? Pending : Resolved;\n");
  expect(app).toContain("return Dep.started(status) && status === Resolved;");
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
  expect(dep).toContain('import * as Helpers from "./helpers.js";\nimport noop from "./noop.js";\nimport page from "./page.js";\nimport VERSION from "./version.js";\n');
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
  // Called through its module, by what it exports it as there (ADR 1018).
  expect(dep).toContain('import * as Scheduler from "./scheduler.js";\n');
  expect(dep).toContain("  return Scheduler.unstable_now();\n");
  const manifest = JSON.parse(readFileSync(join(dir, "dep", "lib.manifest.json"), "utf8")).library;
  const aliased = manifest.items.filter((item: any) => item.exported_as).map((item: any) => [item.export, item.exported_as]);
  expect(aliased.sort()).toEqual([["LIMIT", "MAX"], ["getCurrentTime", "unstable_now"]]);
  expect(app).toContain('import * as Scheduler from "../dep/scheduler.js";\n');
  expect(app).toContain("  return Scheduler.unstable_now() + Scheduler.unstable_now() + Scheduler.MAX;\n");
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

// React DOM's ReactFiberConfigDOM, `export * from
// 'react-reconciler/src/ReactFiberConfigWithNoPersistence'`: a `pub use
// m::*;` re-exports each name it gives the module, as rustc resolves it,
// from where it's declared, past what the module's own items shadow, its
// types nothing, a private item not at all (rd153).
test("a pub use of every name of a module re-exports each it gives", async () => {
  const dir = fixture("cross-crate-glob-reexports");
  compiled(dir, `pub fn version() -> u32 {
    1
}

pub mod no_persistence {
    pub const SUPPORTS_PERSISTENCE: bool = false;

    fn shim() -> u32 {
        7
    }

    pub fn clone_instance() -> u32 {
        shim()
    }

    pub fn replace_container_children() -> u32 {
        shim() + 1
    }

    pub struct ChildSet;
}
`, `mod own {
    pub fn helper() -> u32 {
        2
    }

    pub fn shadowed() -> u32 {
        3
    }
}

pub use dep::no_persistence::*;
pub use own::*;

pub fn shadowed() -> u32 {
    4
}

pub fn persists(_: Option<ChildSet>) -> bool {
    SUPPORTS_PERSISTENCE
}
`);
  const app = readFileSync(join(dir, "app", "lib.js"), "utf8");
  expect(app).toContain('export {\n  SUPPORTS_PERSISTENCE,\n  cloneInstance,\n  replaceContainerChildren,\n} from "../dep/no_persistence.js";\n');
  expect(app).toContain('export { helper } from "./own.js";\n');
  const module = await import(join(dir, "app", "lib.js"));
  expect([module.SUPPORTS_PERSISTENCE, module.cloneInstance(), module.replaceContainerChildren(), module.helper(), module.shadowed()]).toEqual([false, 7, 8, 2, 4]);
  expect(Object.keys(module).sort()).toEqual(["SUPPORTS_PERSISTENCE", "cloneInstance", "helper", "persists", "replaceContainerChildren", "shadowed"]);
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

// rd114: a library's function called through its module's path, with no
// `use` of it, is read through that module's namespace, as the crate's
// own module's is, `Events.extractEvents(..)`, and its root's by its
// crate's name, `Dep.helper()`; one the crate `use`s is its named import.
test("a library's function called through its module is read through its namespace", async () => {
  const dir = fixture("cross-crate-namespace");
  const { app } = compiled(dir, `pub mod events {
    pub fn extract_events(n: u32) -> u32 {
        n * 2
    }

    pub fn name() -> &'static str {
        "events"
    }
}

pub fn helper() -> u32 {
    3
}
`, `use dep::events::name;

pub fn run() -> String {
    format!("{} {}", dep::events::extract_events(2) + dep::helper(), name())
}
`);
  expect(app).toContain('import { name } from "../dep/events.js";\nimport * as Events from "../dep/events.js";\n');
  expect(app).toContain('import * as Dep from "../dep/lib.js";\n');
  expect(app).toContain("Events.extractEvents(2) + Dep.helper()");
  const { run } = await import(join(dir, "app", "lib.js"));
  expect(run()).toBe("7 events");
});

// React's ReactFiberNewContext could `use shared::object_is::OBJECT_IS as
// IS;`, shared's default thread-local, as shared's own shallowEqual does:
// the name a `use` renames it to is written as the item's own is, a
// thread-local's camelCase, `is` (ADR 0202, amended).
test("a use's name of a library's thread-local is cased as the thread-local's own", async () => {
  const dir = fixture("cross-crate-renamed-local");
  const { app } = compiled(dir, `pub mod object_is {
    fn is(x: f64, y: f64) -> bool {
        x == y
    }

    thread_local! {
        pub static OBJECT_IS: fn(f64, f64) -> bool = is;
        pub static CALL_LIMIT: u32 = 3;
    }

    js::export_default!(OBJECT_IS);
}

pub const LIMIT: u32 = 1;
`, `use dep::object_is::{CALL_LIMIT as MAX_CALLS, OBJECT_IS as IS};

pub fn same(x: f64, y: f64) -> bool {
    IS.with(|is| is(x, y)) && MAX_CALLS.with(|&max| max > 1)
}
`);
  expect(app).toContain('import is, { CALL_LIMIT as MAX_CALLS } from "../dep/object_is.js";\n');
  expect(app).toContain("  return is(x, y) && MAX_CALLS > 1;\n");
  const { same } = await import(join(dir, "app", "lib.js"));
  expect([same(1, 1), same(1, 2)]).toEqual([true, false]);
});

// React's ReactFiberFlags: `Hydrate = enableCreateEventHandleAPI ? .. :
// ..`, a feature flag of `shared`'s, which React's build swaps a fork of
// in: a const written of a library's is as it's written, the flag imported
// by its name, not rustc's value of it, which would bake one fork's answer
// in (ADR 0031, amended).
test("a const written of a library's consts imports them by name", async () => {
  const dir = fixture("cross-crate-flag");
  const { app } = compiled(dir, `pub const ENABLE_CREATE_EVENT_HANDLE_API: bool = false;
pub const SHARED_LANE: i32 = 0b0100;
`, `use dep::{ENABLE_CREATE_EVENT_HANDLE_API, SHARED_LANE};

#[cfg_attr(rust_js, rust_js::name = "Snapshot")]
pub const SNAPSHOT: i32 = 0b0001;
#[cfg_attr(rust_js, rust_js::name = "Update")]
pub const UPDATE: i32 = 0b0010;
#[cfg_attr(rust_js, rust_js::name = "BeforeMutationMask")]
pub const BEFORE_MUTATION_MASK: i32 = UPDATE
    | if ENABLE_CREATE_EVENT_HANDLE_API {
        SNAPSHOT
    } else {
        0
    };
#[cfg_attr(rust_js, rust_js::name = "Shared")]
pub const SHARED: i32 = SHARED_LANE | UPDATE;
#[cfg_attr(rust_js, rust_js::name = "Flagged")]
pub const FLAGGED: i32 = if ENABLE_CREATE_EVENT_HANDLE_API {
    SHARED_LANE
} else {
    0
};
`);
  expect(app).toContain('import { ENABLE_CREATE_EVENT_HANDLE_API, SHARED_LANE } from "../dep/lib.js";\n');
  expect(app).toContain("export const Shared = SHARED_LANE | Update;\n");
  expect(app).toContain("export const Flagged = ENABLE_CREATE_EVENT_HANDLE_API ? SHARED_LANE : 0;\n");
  expect(app).toMatch(/export const BeforeMutationMask = Update \| \(ENABLE_CREATE_EVENT_HANDLE_API \? Snapshot : 0\);\n/);
  const { BeforeMutationMask, Shared } = await import(join(dir, "app", "lib.js"));
  expect([BeforeMutationMask, Shared]).toEqual([2, 6]);
});

// React's ReactFiberCacheComponent: an `AbortController` shim's listeners,
// `Rc<RefCell<Vec<Rc<dyn Fn()>>>>`, and elsewhere in the crate a closure
// given to a library's generic binding, `Thenable::then(impl FnOnce)`, that
// holds an `Rc<RefCell<Vec<&dyn Fn(..)>>>`. The closure is the crate's: what
// it holds the crate made and sees, not a `dyn Fn` the library hands it,
// so no `dyn Fn` drops, and no `Rc` is counted, the listeners an array
// (ADR 0626, amended); even where the library makes a `dyn Fn` that drops,
// `later`'s, and is given a closure of the crate's holding a `&dyn Fn`.
test("a crate's closure given to a library's binding hands the crate nothing", async () => {
  const dir = fixture("cross-crate-closure-given");
  const { app } = compiled(dir, `use std::marker::PhantomData;

use js::{JsObject, Unknown};

pub struct Thenable(PhantomData<JsObject>);

unsafe impl js::Defined for Thenable {}

impl Thenable {
    #[cfg_attr(rust_js, rust_js::link_name = "then")]
    pub fn then(&self, on_fulfill: impl FnOnce(Option<&'static Unknown>) + 'static) {
        let _ = on_fulfill;
        unreachable!()
    }
}

pub fn settled(thenable: &Thenable) {
    thenable.then(|_| {});
}

pub struct Guard;

impl Drop for Guard {
    fn drop(&mut self) {
        println!("guard dropped");
    }
}

pub fn later(f: impl FnOnce() + 'static) {
    let guard = Guard;
    let held: Box<dyn Fn()> = Box::new(move || {
        let _ = &guard;
    });
    held();
    f();
}
`, `use std::cell::{Cell, RefCell};
use std::rc::Rc;

use dep::Thenable;

type Listener = Rc<dyn Fn()>;

pub struct Signal {
    pub aborted: Cell<bool>,
    pub add: Box<dyn Fn(Listener)>,
}

pub fn controller() -> (Rc<Signal>, Box<dyn Fn()>) {
    let listeners: Rc<RefCell<Vec<Listener>>> = Rc::new(RefCell::new(Vec::new()));
    let signal = Rc::new(Signal {
        aborted: Cell::new(false),
        add: {
            let listeners = Rc::clone(&listeners);
            Box::new(move |listener| listeners.borrow_mut().push(listener))
        },
    });
    let abort: Box<dyn Fn()> = {
        let signal = Rc::clone(&signal);
        Box::new(move || {
            signal.aborted.set(true);
            let count = listeners.borrow().len();
            for index in 0..count {
                let listener = Rc::clone(&listeners.borrow()[index]);
                listener();
            }
        })
    };
    (signal, abort)
}

pub fn count_later(thenable: &Thenable) {
    let waiters: Rc<RefCell<Vec<&'static dyn Fn(Option<&'static js::Unknown>)>>> = Rc::new(RefCell::new(Vec::new()));
    thenable.then(move |_| println!("{}", waiters.borrow().len()));
}

pub fn count_soon() {
    let waiters: Rc<RefCell<Vec<&'static dyn Fn()>>> = Rc::new(RefCell::new(Vec::new()));
    dep::later(move || println!("{}", waiters.borrow().len()));
}
`);
  expect(app).not.toContain("strong");
  expect(app).not.toContain("$dropping");
  expect(app).toContain("  const listeners = [];\n");
  const { controller } = await import(join(dir, "app", "lib.js"));
  const [signal, abort] = controller();
  const heard: string[] = [];
  signal.add(() => heard.push("one"));
  signal.add(() => heard.push(String(signal.aborted)));
  abort();
  expect(heard).toEqual(["one", "true"]);
});

// ADR 0605, amended: React's ReactStartTransition writes
// `returnValue.then(noop, reportGlobalError)`, `noop` shared's. A closure
// given to a binding's `FnOnce` callback that only passes its call on to a
// library's function is that function, as to the crate's own: a library's
// compiled Rust takes what Rust gives it alone. Not one of rest arguments,
// which reads all JS gives it.
test("a once-callback that passes its call on to a library's function is the function", async () => {
  const dir = fixture("cross-crate-passed-on");
  const { app } = compiled(dir, `use std::cell::Cell;

thread_local! {
    static COUNT: Cell<u32> = const { Cell::new(0) };
}

pub fn noop() {
    COUNT.set(COUNT.get() + 1);
}

pub fn bump(by: u32) {
    COUNT.set(COUNT.get() + by * 10);
}

pub fn many(#[cfg_attr(rust_js, rust_js::rest)] xs: &[u32]) {
    COUNT.set(COUNT.get() + xs.len() as u32 * 100);
}

pub fn count() -> u32 {
    COUNT.get()
}
`, `/// \`callback.call(undefined, a, b)\`: JS calls the callback once.
#[cfg_attr(rust_js, rust_js::link_name = "call")]
fn once_with(this: Box<dyn FnOnce(u32, u32)>, this_arg: (), a: u32, b: u32) {
    let () = this_arg;
    this(a, b)
}

pub fn run() -> u32 {
    once_with(Box::new(|_, _| dep::noop()), (), 1, 2);
    once_with(Box::new(|by, _| dep::bump(by)), (), 3, 4);
    once_with(Box::new(|x, _| dep::many(&[x])), (), 5, 6);
    dep::count()
}
`);
  // Through its root's namespace, a name too (ADR 1018).
  expect(app).toContain("  Dep.noop.call(undefined, 1, 2);\n");
  expect(app).toContain("  Dep.bump.call(undefined, 3, 4);\n");
  expect(app).toContain("  ((x) => {\n    Dep.many(x);\n  }).call(undefined, 5, 6);\n");
  const { run: ran } = await import(join(dir, "app", "lib.js"));
  expect(ran()).toBe(131);
});

// React's react_isomorphic and shared hand the reconciler `dyn Fn()`s, made
// of none that drops, and the reconciler's chainThenableValue<T> boxes a
// closure that holds only its listeners. Neither makes the crate's `dyn Fn`s
// drop: a library's contract says whether it makes a `dyn Fn` of what
// drops, and a closure of a generic function's is judged by what it holds
// (ADRs 0521, 0626, amended).
test("a library's dyn Fn drops in its consumer only where a library makes one of what drops", async () => {
  const dir = fixture("cross-crate-dyn-fn-plain");
  const { app } = compiled(dir, `pub fn call(f: &dyn Fn() -> u32) -> u32 {
    f()
}

pub fn counter() -> Box<dyn Fn() -> u32> {
    Box::new(|| 1)
}

// React's useReducer, a \`dyn Fn\` of its type parameters given on.
pub fn use_reducer<S, A>(reducer: &'static dyn Fn(S, A) -> S, state: S, action: A) -> S {
    reduce(reducer, state, action)
}

fn reduce<S, A>(reducer: &dyn Fn(S, A) -> S, state: S, action: A) -> S {
    reducer(state, action)
}

// A closure that holds a type parameter's value, made no \`dyn Fn\`.
pub fn repeat<T: Clone>(value: T, n: usize) -> Vec<T> {
    (0..n).map(move |_| value.clone()).collect()
}
`, `use std::cell::RefCell;
use std::rc::Rc;

type Listener = Rc<dyn Fn()>;

pub fn controller() -> (Box<dyn Fn(Listener)>, Box<dyn Fn() -> u32>) {
    let listeners: Rc<RefCell<Vec<Listener>>> = Rc::new(RefCell::new(Vec::new()));
    let add: Box<dyn Fn(Listener)> = {
        let listeners = Rc::clone(&listeners);
        Box::new(move |listener| listeners.borrow_mut().push(listener))
    };
    let abort: Box<dyn Fn() -> u32> = Box::new(move || {
        let count = listeners.borrow().len();
        for index in 0..count {
            let listener = Rc::clone(&listeners.borrow()[index]);
            listener();
        }
        dep::call(&|| count as u32) + dep::counter()() + dep::use_reducer(&|s: u32, a: u32| s + a, 1, 2)
    });
    (add, abort)
}

pub fn chained<T: Copy + 'static>(result: T) -> (T, Box<dyn Fn(Listener)>) {
    let listeners: Rc<RefCell<Vec<Listener>>> = Rc::new(RefCell::new(Vec::new()));
    (result, Box::new(move |listener| listeners.borrow_mut().push(listener)))
}

pub fn chained_count() -> u32 {
    let (n, add) = chained(2);
    add(Rc::new(|| {}));
    n
}
`);
  expect(JSON.parse(readFileSync(join(dir, "dep", "lib.manifest.json"), "utf8")).library.makes_dropping_fn_dyns).toBe(false);
  expect(app).not.toContain("strong");
  expect(app).not.toContain("$dropping");
  expect(app).not.toContain("$dispose");
  const { controller, chainedCount } = await import(join(dir, "app", "lib.js"));
  const [add, abort] = controller();
  const heard: string[] = [];
  add(() => heard.push("one"));
  add(() => heard.push("two"));
  expect([abort(), heard, chainedCount()]).toEqual([6, ["one", "two"], 2]);
});

// And where a library makes a `dyn Fn` of a closure that holds a value with
// a destructor, its contract says so, and the crate's `dyn Fn`s carry their
// drops: the one it hands the crate is dropped where Rust drops it.
test("a library's dyn Fn of what drops is dropped where its consumer drops it", async () => {
  const dir = fixture("cross-crate-dyn-fn-drops");
  const { app } = compiled(dir, `pub struct Loud(pub u32);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("dropped {}", self.0);
    }
}

pub fn loud(n: u32) -> Box<dyn Fn() -> u32> {
    let loud = Loud(n);
    Box::new(move || {
        let loud = &loud;
        loud.0 + 1
    })
}
`, `pub fn run(n: u32) -> u32 {
    let f = dep::loud(n);
    let value = f();
    println!("called {value}");
    value
}
`);
  expect(JSON.parse(readFileSync(join(dir, "dep", "lib.manifest.json"), "utf8")).library.makes_dropping_fn_dyns).toBe(true);
  const logged: unknown[] = [];
  const log = console.log;
  console.log = (...args: unknown[]) => logged.push(args.join(" "));
  try {
    const { run } = await import(join(dir, "app", "lib.js"));
    expect(run(4)).toBe(5);
  } finally {
    console.log = log;
  }
  expect(logged).toEqual(["called 5", "dropped 4"]);
});

// A library's `dyn Fn`s drop where it makes one of what drops, as its
// contract says: one whose do drops the consumer's it's given where Rust
// drops it. One whose `dyn Fn`s drop nothing, given one by a consumer whose
// may hold what drops, would drop nothing of it: the consumer is refused,
// naming it (ADR 0626, amended).
test("a library drops a consumer's dyn Fn where its own drop, and refuses one it would drop nothing of", async () => {
  const loud = `pub struct Loud(pub u32);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("dropped {}", self.0);
    }
}

`;
  const run = `${loud}pub fn run(n: u32) -> u32 {
    let loud = Loud(n);
    dep::call_boxed(Box::new(move || {
        let loud = &loud;
        loud.0 + 1
    }))
}
`;
  const callBoxed = `pub fn call_boxed(f: Box<dyn Fn() -> u32>) -> u32 {
    let value = f();
    println!("called {value}");
    value
}
`;
  const dir = fixture("cross-crate-dyn-fn-given");
  compiled(dir, `${callBoxed}
pub struct Tick(pub u32);

impl Drop for Tick {
    fn drop(&mut self) {
        println!("tick {}", self.0);
    }
}

pub fn ticker(n: u32) -> Box<dyn Fn() -> u32> {
    let tick = Tick(n);
    Box::new(move || {
        let tick = &tick;
        tick.0
    })
}
`, run);
  expect(JSON.parse(readFileSync(join(dir, "dep", "lib.manifest.json"), "utf8")).library.makes_dropping_fn_dyns).toBe(true);
  const logged: unknown[] = [];
  const log = console.log;
  console.log = (...args: unknown[]) => logged.push(args.join(" "));
  try {
    const app = await import(join(dir, "app", "lib.js"));
    expect(app.run(7)).toBe(8);
  } finally {
    console.log = log;
  }
  expect(logged).toEqual(["called 8", "dropped 7"]);
  const refused = fixture("cross-crate-dyn-fn-refused");
  expect(() => compiled(refused, callBoxed, run)).toThrow("this crate's `dyn Fn`s may hold a value with a destructor, which their drop runs, but `dep`, a rust-js library whose `dyn Fn`s drop nothing, holds one of them");
  expect(JSON.parse(readFileSync(join(refused, "dep", "lib.manifest.json"), "utf8")).library.makes_dropping_fn_dyns).toBe(false);
});

// A library's generic function takes a copy function only where its own
// crate's calls give one (ADR 0914): its consumer gives the dictionaries its
// contract says it takes, each in its place, not a `TCopy` the library's JS
// hasn't, which put `undefined` where `UDisplay` is and threw. One that
// gives a value whose copy isn't itself compiles its own (`copy_given`).
test("a library's export is given the dictionaries its contract says it takes", async () => {
  const dir = fixture("cross-crate-copy-bound");
  const { dep, app } = compiled(dir, `pub fn pair<T: Copy, U: std::fmt::Display>(t: T, u: U) -> (T, String) {
    (t, format!("{u}!"))
}

// Its \`Clone\` too, which takes no dictionary where its crate gives none
// (ADR 0914, amended)...
pub fn pair_clone<T: Clone, U: std::fmt::Display>(t: &T, u: U) -> (T, String) {
    (t.clone(), format!("{u}?"))
}

// ...and one where it gives a value that isn't its own clone.
pub fn dup<T: Clone>(value: &T) -> T {
    value.clone()
}

pub fn names() -> Vec<String> {
    dup(&vec!["a".to_string()])
}
`, `use std::fmt;

pub struct Wrap(pub u32);

impl fmt::Display for Wrap {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "w{}", self.0)
    }
}

pub fn run() -> String {
    dep::pair(1u32, Wrap(2)).1 + &dep::pair_clone(&1u32, Wrap(3)).1
}

pub fn twice() -> Vec<u32> {
    let mut copy = dep::dup(&vec![1u32]);
    copy.push(2);
    copy
}
`);
  expect(dep).toContain("export function pair(t, u, UDisplay) {");
  expect(dep).toContain("export function pairClone(t, u, UDisplay) {");
  expect(dep).toContain("export function dup(value, TClone) {");
  expect(app).toContain("pair(1, [2], wrapDisplay)");
  expect(app).toContain("pairClone(1, [3], wrapDisplay)");
  // The library's own, which its contract says takes a \`TClone\`.
  expect(app).toContain("dup([1], { clone: ");
  expect(app).not.toContain("function dup(");
  const { run, twice } = await import(join(dir, "app", "lib.js"));
  expect([run(), twice()]).toEqual(["w2!w3?", [1, 2]]);
});

// A consumer that gives a library's generic function a value that isn't its
// own clone, where the library's own crate gives none, compiles its own
// (ADR 0914, amended): its copy is apart from what it was given.
test("a consumer cloning what isn't its own clone compiles its own of the function", async () => {
  const dir = fixture("cross-crate-clone-given");
  const { app } = compiled(dir, `pub fn cloned<T: Clone>(value: &T) -> T {
    value.clone()
}
`, `pub fn apart() -> (Vec<u32>, Vec<u32>) {
    let original = vec![1u32];
    let mut copy = dep::cloned(&original);
    copy.push(2);
    (original, copy)
}
`);
  expect(app).toContain("function cloned(value, TClone) {");
  const { apart } = await import(join(dir, "app", "lib.js"));
  expect(apart()).toEqual([[1], [1, 2]]);
});

// A library's generic function its own crate gives nothing that may look
// like \`None\` makes its \`Some\` the value, as React's reconciler's
// flushSyncFromReconciler gives its JS caller what the callback gives; a
// consumer that gives it \`()\` or an \`Option\` compiles its own, whose
// \`Some(None)\` is boxed, still \`Some\` (ADR 0051, amended). One the
// library itself gives an \`Option\` boxes in its own JS, and is called.
test("a consumer giving what may look like None compiles its own of an unboxed function", async () => {
  const dir = fixture("cross-crate-unboxed");
  const { dep, app } = compiled(dir, `pub fn flush_sync<R>(f: Option<impl FnOnce() -> R>) -> Option<R> {
    if let Some(f) = f { Some(f()) } else { None }
}

pub fn wrap<T>(x: T) -> Option<T> {
    Some(x)
}

pub fn wrapped_none() -> bool {
    wrap(None::<u8>).is_some()
}
`, `pub fn plain() -> Option<u32> {
    dep::flush_sync(Some(|| 3))
}

pub fn units() -> (bool, bool, bool) {
    let unit = dep::flush_sync(Some(|| ())).is_some();
    let none = dep::flush_sync(Some(|| None::<u8>)).is_some();
    (unit, none, dep::wrap(()).is_some())
}
`);
  expect(dep).toContain("export function flushSync(f) {\n  if (f != null) {\n    return f();\n  }\n}");
  // A function JS calls takes what JS gives as it gives it, though its
  // crate's Rust gives it an Option, which calls a copy that boxes (ADR
  // 0051, amended); a consumer giving it `()` compiles its own.
  expect(dep).toContain("export function wrap(x) {\n  return x;\n}");
  expect(dep).toContain("function wrapBoxed(x) {\n  return $some(x);\n}");
  expect(app).toContain("function flushSync(f) {\n  if (f != null) {\n    return $some(f());\n  }\n}");
  expect(app).toContain("function wrap(x) {\n  return $some(x);\n}");
  const { plain, units } = await import(join(dir, "app", "lib.js"));
  expect([plain(), units()]).toEqual([3, [true, true, true]]);
  const library = await import(join(dir, "dep", "lib.js"));
  expect([library.flushSync(() => undefined), library.flushSync(() => null), library.wrappedNone()]).toEqual([undefined, null, true]);
  expect([library.wrap(undefined), library.wrap(null), library.wrap(2)]).toEqual([undefined, null, 2]);
});

// React's reconciler reads a context, `readContext(CacheContext)`, of
// `react`'s `ReactContext<T>`, whose `_currentValue` is a `RefCell<T>` it
// borrows unchecked: `read_context<T: Clone>` clones it under the borrow, and
// keeps the context, `Rc::clone(context)`, in what it records. Each is
// judged by what its callers give it, `Option<Rc<Cache>>`, its own clone,
// which drops nothing: no `TClone`, no borrow held while code runs, no
// counted `Rc` (ADR 0914, amended).
test("a generic function's clone and its Rc are judged by what its callers give", async () => {
  const dir = fixture("cross-crate-clone-context");
  const { app } = compiled(dir, `use std::cell::RefCell;
use std::rc::Rc;

pub struct Context<T: 'static> {
    pub current_value: RefCell<T>,
}

pub fn create_context<T: Copy>(default_value: T) -> Rc<Context<T>> {
    Rc::new(Context {
        current_value: RefCell::new(default_value),
    })
}

pub fn set_value<T>(context: &Context<T>, value: T) {
    context.current_value.replace(value);
}
`, `use std::cell::{Cell, RefCell};
use std::rc::Rc;

use dep::Context;
use js::Unknown;

pub struct Cache {
    pub size: u32,
}

thread_local! {
    pub static CACHE_CONTEXT: Rc<Context<Option<Rc<Cache>>>> = Rc::new(Context {
        current_value: RefCell::new(None),
    });
    static LAST_READ: Cell<Option<&'static Unknown>> = const { Cell::new(None) };
    static READS: Cell<usize> = const { Cell::new(0) };
}

pub fn read_context<T: Clone + 'static>(context: &Rc<Context<T>>) -> T {
    let value = context.current_value.borrow().clone();
    LAST_READ.set(js::any_of(Rc::clone(context)));
    READS.set(READS.get() + 1);
    value
}

pub fn cache_size() -> u32 {
    CACHE_CONTEXT
        .with(|context| read_context(context))
        .map_or(0, |cache| cache.size)
}

pub fn fill(size: u32) {
    CACHE_CONTEXT.with(|context| dep::set_value(context, Some(Rc::new(Cache { size }))));
}

pub fn read_count() -> usize {
    READS.get()
}

pub fn last_read() -> Option<&'static Unknown> {
    LAST_READ.get()
}
`);
  expect(app).toContain("export function readContext(context) {\n  const value = context.currentValue;\n");
  expect(app).not.toContain("TClone");
  expect(app).not.toContain("strong");
  expect(app).not.toContain("$borrow");
  const { cacheSize, fill, readCount, lastRead, cacheContext } = await import(join(dir, "app", "lib.js"));
  expect(cacheSize()).toBe(0);
  fill(4);
  expect([cacheSize(), readCount(), lastRead() === cacheContext]).toEqual([4, 2, true]);
});

// ADR 0914, amended: a library's function whose binding call gives a value
// of what the crate changes in place, `js::object::keys(props)`'s
// `Vec<String>`, which the crate pushes to, copies nothing of the crate's:
// a binding's result is JS's own, the same however it's compiled. The
// crate imports it, as React's reconciler does shared's
// `addObjectToProperties`.
test("a library's function of a binding's Vec is imported, not compiled again", async () => {
  const dir = fixture("cross-crate-binding-vec");
  const { app } = compiled(dir, `use js::Unknown;

pub fn key_count(props: &Unknown) -> usize {
    let keys = js::object::keys(props);
    keys.len()
}
`, `use js::Unknown;

pub fn names(props: &Unknown) -> Vec<String> {
    let mut names = Vec::new();
    names.push(dep::key_count(props).to_string());
    names
}
`);
  expect(app).toContain('import * as Dep from "../dep/lib.js";\n');
  expect(app).not.toContain("function keyCount");
  const { names } = await import(join(dir, "app", "lib.js"));
  expect(names({ a: 1, b: 2 })).toEqual(["2"]);
});

// ADR 0914, amended: a slice's `concat()` makes an array of its own, `[...a,
// ...b]`, so what a library's function of one may share with what it's
// given is its items: React's shared's addValueToProperties, whose
// `[&array[..1], &[Some(..)]].concat()` of `Option<&Unknown>`s the
// reconciler's memo cache's `Vec` of them is pushed to, is imported. One of
// items the crate changes in place, `Vec<u32>`s it pushes to, is compiled
// again, its copies the crate's: the crate's push leaves what it was given
// as it was, as Rust's clones do.
test("a library's function of a concat is imported, unless its items are what the crate changes", async () => {
  const dir = fixture("cross-crate-concat");
  const { app } = compiled(dir, `use js::Unknown;

pub fn truncated(array: &[Option<&'static Unknown>]) -> usize {
    [&array[..1], &[Some(js::unknown("…"))]].concat().len()
}

pub fn joined(a: &[Vec<u32>], b: &[Vec<u32>]) -> Vec<Vec<u32>> {
    [a, b].concat()
}
`, `use js::Unknown;

pub fn log(array: &[Option<&'static Unknown>]) -> usize {
    dep::truncated(array)
}

pub fn push(data: &mut Vec<Option<&'static Unknown>>, value: Option<&'static Unknown>) {
    data.push(value);
}

pub fn pushed(x: &[Vec<u32>], y: &[Vec<u32>]) -> usize {
    let mut joined = dep::joined(x, y);
    joined[0].push(9);
    x[0].len()
}
`);
  expect(app).toContain('import * as Dep from "../dep/lib.js";\n');
  expect(app).not.toContain("function truncated");
  expect(app).toContain("function joined");
  const lib = await import(join(dir, "app", "lib.js"));
  expect([lib.log([null, null]), lib.pushed([[1], [2]], [[3]])]).toEqual([2, 1]);
});

// A library's thread-local `RefCell` none of whose borrows there can meet
// another (ADR 0863) is read by a consumer's `with_borrow` as its module's
// variable, `const child = current;` (gap rd76), where nothing it runs
// meanwhile may borrow it: one whose closure calls the library's code that
// sets it, which Rust's `BorrowMutError` stops, is refused.
test("a library's thread-local is borrowed quietly only where nothing it runs may borrow it", () => {
  const dir = fixture("cross-crate-quiet-key");
  const dep = `use std::cell::RefCell;

thread_local! {
    pub static CURRENT: RefCell<Option<u32>> = const { RefCell::new(None) };
}

pub fn set_current(tag: Option<u32>) {
    CURRENT.set(tag);
}
`;
  const { app } = compiled(dir, dep, `use dep::CURRENT;

pub fn current() -> Option<u32> {
    CURRENT.with_borrow(|current| *current)
}
`);
  expect(app).toContain("export function current() {\n  return current$1;\n}\n");
  expect(() => compiled(dir, dep, `use dep::CURRENT;

pub fn clash() {
    CURRENT.with_borrow(|_| dep::set_current(None));
}
`)).toThrow("rust-js:");
  // Nor one its library keeps as a cell, lent by `with`.
  const log = `use std::cell::RefCell;

thread_local! {
    pub static LOG: RefCell<Vec<u32>> = const { RefCell::new(Vec::new()) };
}

pub fn push(entry: u32) {
    LOG.with(|log| log.borrow_mut().push(entry));
}
`;
  expect(() => compiled(dir, log, `use dep::LOG;

pub fn clash() -> usize {
    LOG.with_borrow(|log| {
        dep::push(1);
        log.len()
    })
}
`)).toThrow("is held while code rust-js can't see runs");
});

// The user's decision (React port gap rd62): a `From` between a
// library's `#[repr(transparent)]` struct and its field is the value it's
// given, as the struct is its field (ADR 0584, amended): React DOM's
// `EventPriority::from(ReactDOMSharedInternals.p)` of the reconciler's
// `EventPriority(Lane)` reads `ReactDOMSharedInternals.p`, as upstream's
// getCurrentUpdatePriority does, and writes no `from` of its own.
test("a library's transparent struct's From is the value it's given", async () => {
  const dir = fixture("cross-crate-newtype-from");
  const { app } = compiled(dir, `#[derive(Clone, Copy, PartialEq)]
#[repr(transparent)]
pub struct EventPriority(i32);

impl From<i32> for EventPriority {
    fn from(lane: i32) -> Self {
        EventPriority(lane)
    }
}

pub const DEFAULT_EVENT_PRIORITY: EventPriority = EventPriority(32);

pub fn event_priority_to_lane(priority: EventPriority) -> i32 {
    priority.0
}
`, `use dep::{DEFAULT_EVENT_PRIORITY, EventPriority, event_priority_to_lane};

pub fn current(p: i32) -> i32 {
    let priority = EventPriority::from(p);
    if priority == DEFAULT_EVENT_PRIORITY {
        return 0;
    }
    event_priority_to_lane(priority)
}
`);
  expect(app).toContain("  const priority = p;\n");
  expect(app).not.toContain("_from");
  const { current } = await import(join(dir, "app", "lib.js"));
  expect([current(32), current(8)]).toEqual([0, 8]);
});

// React port gap rd79: ReactDOMEventListener's `use
// scheduler::{IDLE_PRIORITY as IDLE_SCHEDULER_PRIORITY, ..}` of the
// scheduler's `IdlePriority` and `getCurrentPriorityLevel`, each its
// `#[rust_js::name]`, imports them as upstream does, `IdlePriority as
// IdleSchedulerPriority`: the name a `use` renames an item to is cased as
// the item's own JS name is (ADR 0202, amended). A SCREAMING one's stays.
test("a use's name of an item its rust_js::name names is cased as that name", async () => {
  const dir = fixture("cross-crate-renamed-given");
  const { dep, app } = compiled(dir, `pub mod scheduler {
    thread_local! {
        #[cfg_attr(rust_js, rust_js::name = "IdlePriority")]
        pub static IDLE_PRIORITY: i32 = 5;
    }
    #[cfg_attr(rust_js, rust_js::name = "getCurrentPriorityLevel")]
    pub static GET_CURRENT_PRIORITY_LEVEL: fn() -> i32 = || 5;
    #[cfg_attr(rust_js, rust_js::name = "MAX_LANE")]
    pub const MAX: i32 = 31;
}

use scheduler::IDLE_PRIORITY as IDLE_SCHEDULER_PRIORITY;

pub fn idle_here() -> i32 {
    IDLE_SCHEDULER_PRIORITY.with(|p| *p)
}
`, `use dep::scheduler::{
    IDLE_PRIORITY as IDLE_SCHEDULER_PRIORITY, MAX as TOP,
    GET_CURRENT_PRIORITY_LEVEL as GET_CURRENT_SCHEDULER_PRIORITY_LEVEL,
};

pub fn idle() -> bool {
    GET_CURRENT_SCHEDULER_PRIORITY_LEVEL() == IDLE_SCHEDULER_PRIORITY.with(|p| *p) && TOP > 0
}
`);
  expect(dep).toContain("IdlePriority as IdleSchedulerPriority");
  expect(app).toContain("IdlePriority as IdleSchedulerPriority");
  expect(app).toContain("getCurrentPriorityLevel as getCurrentSchedulerPriorityLevel");
  expect(app).toContain("return getCurrentSchedulerPriorityLevel() === IdleSchedulerPriority && TOP > 0;");
  const { idle } = await import(join(dir, "app", "lib.js"));
  expect(idle()).toBe(true);
});

// A struct a library makes values of is the program's own data in its
// consumer too, as in the library: React DOM's getParent, of a fiber the
// reconciler makes, is `inst != null ? inst.return : null`, as upstream's,
// not a getter's `inst?.return` (rd221, ADR 1022, amended). A struct no
// code makes is JS's still, its field read once.
test("a struct a library builds is the program's own data in its consumer", async () => {
  const dir = fixture("cross-crate-built");
  const { app } = compiled(dir, `use std::cell::Cell;
use std::rc::Rc;

use js::Unknown;

pub struct Fiber {
    pub parent: Cell<Option<Rc<Fiber>>>,
}

pub struct Props {
    pub key: Option<&'static Unknown>,
}

pub fn fiber(parent: Option<Rc<Fiber>>) -> Rc<Fiber> {
    Rc::new(Fiber { parent: Cell::new(parent) })
}
`, `use std::rc::Rc;

use dep::Fiber;
use js::Unknown;

pub fn fiber_of(node: &Unknown) -> Option<Rc<Fiber>> {
    // Safety: a node's key holds its fiber.
    unsafe { js::cast(js::get(node, "fiber")) }
}

pub fn reparent(fiber: &Fiber, parent: Rc<Fiber>) {
    fiber.parent.replace(Some(parent));
}

pub fn keyed(given: &Unknown) -> bool {
    // Safety: what's given is props.
    let props: &dep::Props = unsafe { js::cast(Some(given)) };
    let key = props.key;
    key.is_some() && js::truthy(key)
}
`);
  expect(JSON.parse(readFileSync(join(dir, "dep", "lib.manifest.json"), "utf8")).library.built).toHaveLength(1);
  expect(app).toContain("export function reparent(fiber, parent) {\n  fiber.parent = parent;\n}");
  expect(app).toContain("  const key = props.key;\n");
  const { reparent, keyed } = await import(join(dir, "app", "lib.js"));
  const fiber = { parent: null };
  reparent(fiber, fiber);
  expect([fiber.parent === fiber, keyed({ key: 0 })]).toEqual([true, false]);
});
