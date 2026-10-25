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
