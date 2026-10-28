// What the compiler writes of a `const` (ADR 0031): rustc's value of it,
// `export const SIZE = 4096;`, or, where it's written of other constants,
// its initializer as it's written, `A | B`, each read by its name (ADR 0031,
// amended). Each test is a program of its own, compiled and run.

import { beforeAll, expect, test } from "bun:test";
import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { basename, join } from "node:path";
import { buildReact, compiler, fixture, run, target } from "./support";

beforeAll(buildReact, 600_000);

const withJs = ["--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];

// React's ReactFiberLane: its masks are of its lanes, `SyncUpdateLanes =
// SyncLane | InputContinuousLane | DefaultLane`, `SomeTransitionLane =
// TransitionLane1`, and a thread-local starts at one, `let
// nextTransitionUpdateLane = TransitionLane1`; ReactEventPriorities makes a
// priority of one, `EventPriority(NO_LANE)`. Each is as it's written, after
// what it reads, a later one too; a `match` and an array's length, where
// rustc needs the value, are as before.
test("a const written of other consts is as it's written", async () => {
  const dir = fixture("const-of-consts");
  writeFileSync(join(dir, "lib.rs"), `use std::cell::Cell;

pub type Lane = i32;

#[cfg_attr(rust_js, rust_js::name = "SyncLane")]
pub const SYNC_LANE: Lane = /*                 */ 0b0000000000000000000000000000010;
#[cfg_attr(rust_js, rust_js::name = "InputContinuousLane")]
pub const INPUT_CONTINUOUS_LANE: Lane = /*      */ 0b0000000000000000000000000001000;
#[cfg_attr(rust_js, rust_js::name = "SyncUpdateLanes")]
pub const SYNC_UPDATE_LANES: Lane = SYNC_LANE | INPUT_CONTINUOUS_LANE | DEFAULT_LANE;
#[cfg_attr(rust_js, rust_js::name = "DefaultLane")]
pub const DEFAULT_LANE: Lane = /*               */ 0b0000000000000000000000000100000;
#[cfg_attr(rust_js, rust_js::name = "SomeTransitionLane")]
pub const SOME_TRANSITION_LANE: Lane = TRANSITION_LANE_1;
#[cfg_attr(rust_js, rust_js::name = "TransitionLane1")]
const TRANSITION_LANE_1: Lane = /*              */ 0b0000000000000000000000100000000;
pub const SIZE: u32 = 4 * 1024;
pub const HALF: u32 = SIZE / 2;
pub const DOUBLED: u32 = doubled(SIZE);

const fn doubled(n: u32) -> u32 {
    n * 2
}

pub struct EventPriority(pub Lane);
#[cfg_attr(rust_js, rust_js::name = "NoEventPriority")]
pub const NO_EVENT_PRIORITY: EventPriority = EventPriority(SYNC_LANE);
#[repr(transparent)]
pub struct Lanes(pub Lane);
#[cfg_attr(rust_js, rust_js::name = "SyncLanes")]
pub const SYNC_LANES: Lanes = Lanes(SYNC_LANE);

thread_local! {
    static NEXT_TRANSITION_UPDATE_LANE: Cell<Lane> = const { Cell::new(TRANSITION_LANE_1) };
}

pub fn next() -> Lane {
    let lane = NEXT_TRANSITION_UPDATE_LANE.get();
    NEXT_TRANSITION_UPDATE_LANE.set(lane << 1);
    lane
}

pub fn named(lane: Lane) -> i32 {
    match lane {
        SYNC_UPDATE_LANES => 1,
        SOME_TRANSITION_LANE => 2,
        _ => [0; HALF as usize / 1024].len() as i32 + NO_EVENT_PRIORITY.0,
    }
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withJs]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  // As written, a literal as rustc's value is, `SIZE`.
  expect(js).toContain("export const DefaultLane = /*               */ 0b0000000000000000000000000100000;\n");
  expect(js).toContain("export const SyncUpdateLanes = SyncLane | InputContinuousLane | DefaultLane;\n");
  expect(js).toContain("export const SomeTransitionLane = TransitionLane1;\n");
  expect(js).toContain("export const SIZE = 4096;\nexport const HALF = (SIZE / 2) >>> 0;\n");
  // One that runs the crate's code is rustc's value, as before.
  expect(js).toContain("export const DOUBLED = 8192;\n");
  expect(js).toContain("export const NoEventPriority = [SyncLane];\n");
  // A transparent struct is its field (ADR 0584): the constant it's made of.
  expect(js).toContain("export const SyncLanes = SyncLane;\n");
  expect(js).toContain("let nextTransitionUpdateLane = TransitionLane1;\n");
  // After what it reads: `DefaultLane`, written after, made first.
  expect(js.indexOf("export const DefaultLane =")).toBeLessThan(js.indexOf("export const SyncUpdateLanes ="));
  expect(js.indexOf("const TransitionLane1 =")).toBeLessThan(js.indexOf("export const SomeTransitionLane ="));
  // Where rustc needs the value, as before.
  expect(js).toContain("[0, 0].length");
  const lib = await import(join(dir, "lib.js"));
  expect([lib.SyncUpdateLanes, lib.SomeTransitionLane, lib.HALF, lib.NoEventPriority, lib.SyncLanes]).toEqual([42, 256, 2048, [2], 2]);
  expect([lib.next(), lib.next(), lib.named(42), lib.named(256), lib.named(7)]).toEqual([256, 512, 1, 2, 4]);
});

// React's ReactFiberHiddenContext makes its cursors where it loads,
// \`createCursor(NoLanes)\`, of ReactFiberLane's lane, which it imports: a
// module it imports, of no import cycle, loads first, so its constant is
// read by its name (ADR 0031, amended). Where the two import each other,
// either may load first, so the value is read, as before.
test("a constant another module makes is read by its name where its module loads first", async () => {
  const dir = fixture("load-reads-by-name");
  writeFileSync(
    join(dir, "lib.rs"),
    `pub mod lane {
    pub type Lanes = u32;
    #[cfg_attr(rust_js, rust_js::name = "NoLanes")]
    pub const NO_LANES: Lanes = 0;
    #[cfg_attr(rust_js, rust_js::name = "SyncLane")]
    pub const SYNC_LANE: Lanes = 2;
}

pub mod hidden {
    use super::lane::{Lanes, NO_LANES, SYNC_LANE};
    use std::cell::RefCell;

    pub struct StackCursor<T> {
        pub current: RefCell<T>,
    }

    #[cfg_attr(rust_js, rust_js::link_name = "./stack.js#createCursor")]
    fn create_cursor<T>(default_value: T) -> StackCursor<T> {
        StackCursor { current: RefCell::new(default_value) }
    }

    thread_local! {
        pub static ENTANGLED: StackCursor<Lanes> = create_cursor(NO_LANES);
    }

    pub static BOTH: [Lanes; 2] = [NO_LANES, SYNC_LANE];

    pub fn read() -> Lanes {
        ENTANGLED.with(|cursor| *cursor.current.borrow()) | BOTH[1]
    }
}

pub mod first {
    #[cfg_attr(rust_js, rust_js::name = "Base")]
    pub const BASE: u32 = 7;

    pub fn twice() -> u32 {
        super::second::LOADED[0] * 2
    }
}

pub mod second {
    pub static LOADED: [u32; 1] = [super::first::BASE];
}
`,
  );
  writeFileSync(join(dir, "stack.js"), "export function createCursor(defaultValue) {\n  return { current: defaultValue };\n}\n");
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withJs]);
  const hidden = readFileSync(join(dir, "hidden.js"), "utf8");
  expect(hidden).toContain("export const entangled = createCursor(NoLanes);\n");
  expect(hidden).toContain("export const BOTH = [NoLanes, SyncLane];\n");
  // \`first\` imports \`second\`, which reads its constant where it loads: its
  // value, as either may load first.
  const second = readFileSync(join(dir, "second.js"), "utf8");
  expect(second).toContain("export const LOADED = [7];\n");
  const { read } = await import(join(dir, "hidden.js"));
  expect(read()).toBe(2);
  const { twice } = await import(join(dir, "first.js"));
  expect(twice()).toBe(14);
});

// React's ReactFiberLane binds ReactFiberDevToolsHook.js, a JS file of its
// own, which imports ReactEventPriorities, which reads ReactFiberLane's
// lane where it loads, `NoEventPriority = NoLane`: an import cycle through
// the JS file. ReactFiberLane loaded first would load it before it makes
// `NoLane`, so its value is read (ADR 0031, amended). Where the JS file
// imports nothing of the crate that imports it back, nor does what it
// imports, the name is; where what it imports isn't found, the value.
test("a constant read where a module loads is its value where a bound JS file imports back", async () => {
  const lib = (hook: string, effects = "") => `pub mod lane {
    pub type Lane = i32;${effects}

    unsafe extern "Rust" {
        #[cfg_attr(rust_js, rust_js::name = "isDevToolsPresent")]
        #[link_name = "${hook}#isDevToolsPresent"]
        safe static IS_DEV_TOOLS_PRESENT: bool;
    }

    #[cfg_attr(rust_js, rust_js::name = "NoLane")]
    pub const NO_LANE: Lane = 0;
    #[cfg_attr(rust_js, rust_js::name = "SyncLane")]
    pub const SYNC_LANE: Lane = 2;

    pub fn marked(lane: Lane) -> Lane {
        if IS_DEV_TOOLS_PRESENT {
            lane
        } else {
            SYNC_LANE
        }
    }
}

pub mod priorities {
    use super::lane::{Lane, NO_LANE};

    #[derive(Clone, Copy)]
    #[repr(transparent)]
    pub struct EventPriority(Lane);

    #[cfg_attr(rust_js, rust_js::name = "NoEventPriority")]
    pub const NO_EVENT_PRIORITY: EventPriority = EventPriority(NO_LANE);

    pub fn lane_of(priority: EventPriority) -> Lane {
        priority.0
    }
}

pub mod tags {
    #[cfg_attr(rust_js, rust_js::name = "LegacyRoot")]
    pub const LEGACY_ROOT: i32 = 0;
}
`;
  // Through a file it imports, a directory down, which names the module
  // without its extension, as React's do.
  const cyclic = fixture("load-cycle-through-js");
  writeFileSync(join(cyclic, "lib.rs"), lib("./src/DevToolsHook.js"));
  mkdirSync(join(cyclic, "src", "shared"), { recursive: true });
  writeFileSync(
    join(cyclic, "src", "DevToolsHook.js"),
    "import {priorityOf} from './shared/priority.js';\nimport {join} from 'node:path';\n\nexport const isDevToolsPresent = join('a', 'b') === 'a';\nexport {priorityOf};\n",
  );
  writeFileSync(
    join(cyclic, "src", "shared", "priority.js"),
    "import {NoEventPriority} from '../../priorities';\n\nexport function priorityOf() {\n  return NoEventPriority;\n}\n",
  );
  run([compiler, join(cyclic, "lib.rs"), "-o", join(cyclic, "lib.js"), ...withJs]);
  expect(readFileSync(join(cyclic, "priorities.js"), "utf8")).toContain("export const NoEventPriority = 0;\n");
  // Loaded first, as an app that imports it first loads it.
  const { marked } = await import(join(cyclic, "lane.js"));
  expect(marked(8)).toBe(2);
  const { priorityOf } = await import(join(cyclic, "src", "DevToolsHook.js"));
  expect(priorityOf()).toBe(0);

  // A file that imports a module of the crate's that imports nothing back,
  // a file of its own that imports nothing, a package and a style: its
  // module loads first. The build read it, and the file it imports: of the
  // manifest's sources, which Cargo is told of too.
  const acyclic = fixture("load-order-through-js");
  writeFileSync(join(acyclic, "lib.rs"), lib("./hook.js"));
  writeFileSync(join(acyclic, "util.js"), "export const absent = false;\n");
  writeFileSync(join(acyclic, "hook.css"), "");
  writeFileSync(
    join(acyclic, "hook.js"),
    "import {absent} from './util.js';\nimport {LegacyRoot} from './tags.js';\nimport 'node:path';\nimport './hook.css';\n\nexport const isDevToolsPresent = absent && LegacyRoot === 0;\n",
  );
  const manifest = join(acyclic, "lib.manifest.json");
  run([compiler, join(acyclic, "lib.rs"), "-o", join(acyclic, "lib.js"), "--manifest", manifest, ...withJs]);
  expect(readFileSync(join(acyclic, "priorities.js"), "utf8")).toContain("export const NoEventPriority = NoLane;\n");
  const { marked: unmarked } = await import(join(acyclic, "lane.js"));
  expect(unmarked(8)).toBe(2);
  const { NoEventPriority } = await import(join(acyclic, "priorities.js"));
  expect(NoEventPriority).toBe(0);
  const { sources } = JSON.parse(readFileSync(manifest, "utf8"));
  expect(sources.map((file: string) => basename(file)).sort()).toEqual(["hook.js", "lib.rs", "util.js"]);

  // One that imports a file that isn't there, which may be any; and one
  // that isn't there itself.
  const unknown = fixture("load-order-unknown-js");
  writeFileSync(join(unknown, "lib.rs"), lib("./hook.js"));
  writeFileSync(join(unknown, "hook.js"), "export {isDevToolsPresent} from './generated.js';\n");
  run([compiler, join(unknown, "lib.rs"), "-o", join(unknown, "lib.js"), ...withJs]);
  expect(readFileSync(join(unknown, "priorities.js"), "utf8")).toContain("export const NoEventPriority = 0;\n");
  const absent = fixture("load-order-absent-js");
  writeFileSync(join(absent, "lib.rs"), lib("./hook.js"));
  run([compiler, join(absent, "lib.rs"), "-o", join(absent, "lib.js"), ...withJs]);
  expect(readFileSync(join(absent, "priorities.js"), "utf8")).toContain("export const NoEventPriority = 0;\n");

  // One imported for its effects, `js::import!`, that imports it back.
  const effects = fixture("load-cycle-through-effects");
  writeFileSync(join(effects, "lib.rs"), lib("./hook.js", '\n\n    js::import!("./setup.js");'));
  writeFileSync(join(effects, "hook.js"), "export const isDevToolsPresent = false;\n");
  writeFileSync(join(effects, "setup.js"), "import {NoEventPriority} from './priorities.js';\n\nglobalThis.priority = () => NoEventPriority;\n");
  run([compiler, join(effects, "lib.rs"), "-o", join(effects, "lib.js"), ...withJs]);
  expect(readFileSync(join(effects, "priorities.js"), "utf8")).toContain("export const NoEventPriority = 0;\n");
  await import(join(effects, "lane.js"));
});

// React's ReactFiberCacheComponent's DEV checks read `enableCache`, a `bool`
// constant: `ENABLED && COUNT.get() > 0.0` was `!!ENABLED && count > 0`,
// as if a constant read by its name might be any value. It's a `bool`, as
// Rust's types say, of the crate's module, another's, or a library's.
test("a bool constant read by its name is a boolean in && and ||", async () => {
  const dir = fixture("bool-consts");
  mkdirSync(join(dir, "dep"), { recursive: true });
  writeFileSync(join(dir, "dep.rs"), "pub const TRACED: bool = false;\n\npub fn traced() -> bool {\n    TRACED\n}\n");
  run([compiler, join(dir, "dep.rs"), "-o", join(dir, "dep", "lib.js"), "--library", "--manifest", join(dir, "dep", "lib.manifest.json"),
    "--", "--crate-name", "dep", `--emit=metadata=${join(dir, "libdep.rmeta")}`, "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  writeFileSync(join(dir, "lib.rs"), `use std::cell::Cell;

pub mod flags {
    pub const LOGGED: bool = true;
}

pub const ENABLED: bool = true;

thread_local! {
    static COUNT: Cell<f64> = const { Cell::new(0.0) };
}

pub fn bump() {
    COUNT.set(COUNT.get() + 1.0);
}

pub fn has_warnings() -> bool {
    ENABLED && COUNT.get() > 0.0
}

pub fn logged(quiet: bool) -> bool {
    flags::LOGGED && !quiet || dep::TRACED
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--dependency", join(dir, "dep", "lib.manifest.json"),
    "--", "--extern", `dep=${join(dir, "libdep.rmeta")}`, "-L", dir, "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("  return ENABLED && count > 0;\n");
  expect(js).toContain("  return (LOGGED && !quiet) || TRACED;\n");
  const lib = await import(join(dir, "lib.js"));
  expect([lib.hasWarnings(), lib.logged(false), lib.logged(true)]).toEqual([false, true, false]);
  lib.bump();
  expect(lib.hasWarnings()).toBe(true);
});

// React's onCommitRoot, `switch (eventPriority) { case
// DiscreteEventPriority: .. case ContinuousEventPriority: .. default: .. }`,
// of priorities its port makes a `#[repr(transparent)]` struct, which is its
// number in JS (ADR 0584): a `match` of its constants tests each by its
// name, a `switch`, as one of numbers does (ADR 0590, amended).
test("a match of a transparent struct's constants names them in a switch", async () => {
  const dir = fixture("transparent-consts-match");
  writeFileSync(join(dir, "lib.rs"), `#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct EventPriority(pub i32);

#[cfg_attr(rust_js, rust_js::name = "DiscreteEventPriority")]
pub const DISCRETE_EVENT_PRIORITY: EventPriority = EventPriority(2);
#[cfg_attr(rust_js, rust_js::name = "ContinuousEventPriority")]
pub const CONTINUOUS_EVENT_PRIORITY: EventPriority = EventPriority(8);

pub fn scheduler_priority(event_priority: EventPriority) -> u8 {
    match event_priority {
        DISCRETE_EVENT_PRIORITY => 1,
        CONTINUOUS_EVENT_PRIORITY => 2,
        _ => 3,
    }
}

pub fn priority(n: i32) -> EventPriority {
    EventPriority(n)
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withJs]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain(
    "  switch (eventPriority) {\n    case DiscreteEventPriority:\n      return 1;\n    case ContinuousEventPriority:\n      return 2;\n    default:\n      return 3;\n  }\n",
  );
  const lib = await import(join(dir, "lib.js"));
  expect([2, 8, 32].map((n) => lib.schedulerPriority(lib.priority(n)))).toEqual([1, 2, 3]);
});

// React's renderRootConcurrent, `switch (workInProgressSuspendedReason) {
// .. case SuspendedOnInstanceAndReadyToContinue: { switch
// (workInProgress.tag) { case HostHoistable: .. case HostComponent: .. } }
// }`, two constants of one value, 5: each names it where its `match` or
// `let` tests it, of where in what's matched, not its number for both (ADR
// 0590, amended). A value its `match` writes two ways is its number, as
// before.
test("constants of one value each name it where they're tested", async () => {
  const dir = fixture("same-valued-consts");
  writeFileSync(
    join(dir, "lib.rs"),
    `#[cfg_attr(rust_js, rust_js::name = "SuspendedOnError")]
pub const SUSPENDED_ON_ERROR: u8 = 1;
#[cfg_attr(rust_js, rust_js::name = "SuspendedOnInstanceAndReadyToContinue")]
pub const SUSPENDED_ON_INSTANCE_AND_READY_TO_CONTINUE: u8 = 5;
#[cfg_attr(rust_js, rust_js::name = "HostHoistable")]
pub const HOST_HOISTABLE: u8 = 26;
#[cfg_attr(rust_js, rust_js::name = "HostComponent")]
pub const HOST_COMPONENT: u8 = 5;
#[cfg_attr(rust_js, rust_js::name = "HostText")]
pub const HOST_TEXT: u8 = 6;

pub fn resume(reason: u8, tag: u8) -> u8 {
    match reason {
        SUSPENDED_ON_ERROR => 1,
        SUSPENDED_ON_INSTANCE_AND_READY_TO_CONTINUE => match tag {
            HOST_HOISTABLE | HOST_COMPONENT => 2,
            _ => 3,
        },
        _ => 4,
    }
}

pub fn paired(reason: u8, tag: u8) -> u8 {
    match (reason, tag) {
        (SUSPENDED_ON_INSTANCE_AND_READY_TO_CONTINUE, HOST_TEXT) => 1,
        (_, HOST_COMPONENT) => 2,
        _ => 3,
    }
}

pub fn tested(tag: Option<u8>, first: bool) -> u8 {
    if let Some(HOST_COMPONENT) = tag {
        return 1;
    }
    let Some(HOST_TEXT) = tag else {
        return match tag {
            Some(5) if first => 2,
            Some(HOST_COMPONENT) => 3,
            _ => 4,
        };
    };
    5
}

pub fn nested(reason: u8, tag: u8, other: u8) -> u8 {
    match (reason, (tag, other)) {
        (SUSPENDED_ON_INSTANCE_AND_READY_TO_CONTINUE, (HOST_COMPONENT, _)) => 1,
        _ => 2,
    }
}

pub fn both(reason: u8, tag: u8) -> bool {
    matches!(reason, SUSPENDED_ON_INSTANCE_AND_READY_TO_CONTINUE) && matches!(tag, HOST_COMPONENT)
}
`,
  );
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withJs]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain(
    "    case SuspendedOnInstanceAndReadyToContinue:\n      switch (tag) {\n        case HostHoistable:\n        case HostComponent:\n",
  );
  expect(js).toContain("  if (reason === SuspendedOnInstanceAndReadyToContinue && tag === HostText) {\n");
  expect(js).toContain("  if (tag === HostComponent) {\n    return 2;\n");
  expect(js).toContain("  if (tag === HostComponent) {\n    return 1;\n");
  expect(js).toContain("  if (tag === HostText) {\n    return 5;\n");
  // Of the inner tuple's first, not the outer's.
  expect(js).toMatch(/ return reason === SuspendedOnInstanceAndReadyToContinue && .*\] === HostComponent\n {4}\? 1\n/);
  // Of each `matches!`, written alike in core.
  expect(js).toContain("  return reason === SuspendedOnInstanceAndReadyToContinue && tag === HostComponent;\n");
  // Written two ways in one `match`: its number there.
  expect(js).toContain("  if (tag === 5) {\n");
  expect(js).not.toMatch(/case 5:|=== 5 &&|=== 5;|tag === 5 \|\|/);
  const lib = await import(join(dir, "lib.js"));
  const resumed = [
    [1, 0],
    [5, 26],
    [5, 5],
    [5, 7],
    [9, 5],
  ].map(([r, t]) => lib.resume(r, t));
  expect(resumed).toEqual([1, 2, 2, 3, 4]);
  const paired = [
    [5, 6],
    [1, 5],
    [5, 7],
  ].map(([r, t]) => lib.paired(r, t));
  expect(paired).toEqual([1, 2, 3]);
  expect([lib.tested(5, true), lib.tested(null, true), lib.tested(6, false)]).toEqual([1, 4, 5]);
  expect([lib.nested(5, 5, 0), lib.nested(5, 6, 0), lib.both(5, 5), lib.both(5, 6)]).toEqual([1, 2, true, false]);
});
