// What the compiler writes of a `const` (ADR 0031): rustc's value of it,
// `export const SIZE = 4096;`, or, where it's written of other constants,
// its initializer as it's written, `A | B`, each read by its name (ADR 0031,
// amended). Each test is a program of its own, compiled and run.

import { beforeAll, expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
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
  expect(js).toContain("let nextTransitionUpdateLane = TransitionLane1;\n");
  // After what it reads: `DefaultLane`, written after, made first.
  expect(js.indexOf("export const DefaultLane =")).toBeLessThan(js.indexOf("export const SyncUpdateLanes ="));
  expect(js.indexOf("const TransitionLane1 =")).toBeLessThan(js.indexOf("export const SomeTransitionLane ="));
  // Where rustc needs the value, as before.
  expect(js).toContain("[0, 0].length");
  const lib = await import(join(dir, "lib.js"));
  expect([lib.SyncUpdateLanes, lib.SomeTransitionLane, lib.HALF, lib.NoEventPriority]).toEqual([42, 256, 2048, [2]]);
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
