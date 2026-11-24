// Where a comment between an `=` and its value is, before the value, on
// its line or its own, and ReactFiberLane's in an else, an if's test and a
// call. Split from lowering-comments.test.ts by concept (#1334).

import { beforeAll, expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { decodeMappings } from "./sourcemap";
import { buildReact, compiler, fixture, run, target } from "./support";

beforeAll(buildReact, 600_000);

/** `rust` compiled in a fixture of `name`, against the js crate: its JS,
 * its module, and its source map. */
async function compiled(name: string, rust: string): Promise<{ js: string; lib: Record<string, (...args: never[]) => unknown>; map: { mappings: string } }> {
  const dir = fixture(name);
  writeFileSync(join(dir, "lib.rs"), rust);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  return { js, lib: await import(join(dir, "lib.js")), map: JSON.parse(readFileSync(join(dir, "lib.js.map"), "utf8")) };
}

// React's ReactTypeOfMode and ReactFiberFlags: `export const NoMode = /*
// .. */ 0b0000000;`, a comment between the `=` and the value that lines the
// values up. It's before the value's JS, after its `=`, its spaces kept, not
// after `export` or a `const`, the code before it on its line that has a
// name (ADR 0630, amended); one after the value is the line's, as before.
test("a comment between an = and its value is before the value", async () => {
  const rust = `use std::cell::Cell;

#[cfg_attr(rust_js, rust_js::name = "NoMode")]
pub const NO_MODE: u32 = /*                         */ 0b0000000;
// TODO: Remove ConcurrentMode by reading from the root tag instead
#[cfg_attr(rust_js, rust_js::name = "ConcurrentMode")]
pub const CONCURRENT_MODE: u32 = /*                 */ 0b0000001;
#[cfg_attr(rust_js, rust_js::name = "Removed")]
pub const REMOVED: u32 = 0b01; // Removed

pub fn modes(given: u32) -> u32 {
    let mode = /*  */ given | CONCURRENT_MODE;
    let mut last = /* first */ mode;
    last = /* again */ last | (mode + 1);
    last
}

pub fn same(a: u32, b: u32) -> bool {
    let same = a == /* the other */ b;
    same || a > 9
}

static FLAGS: u32 = /*  */ 0b0100;

thread_local! {
    static COUNT: Cell<u32> = /* none yet */ const { Cell::new(0) };
}

pub fn flags() -> u32 {
    COUNT.with(|count| count.set(count.get() + 1));
    FLAGS + COUNT.with(Cell::get) + NO_MODE + REMOVED
}
`;
  const { js, lib, map } = await compiled("comment-before-a-value", rust);
  expect(js).toContain("\nexport const NoMode = /*                         */ 0b0000000;\n");
  expect(js).toContain(
    "// TODO: Remove ConcurrentMode by reading from the root tag instead\nexport const ConcurrentMode = /*                 */ 0b0000001;\n",
  );
  expect(js).toContain("\nexport const Removed = 0b01; // Removed\n");
  expect(js).toContain("  const mode = /*  */ (given | ConcurrentMode) >>> 0;\n");
  expect(js).toContain("  let last = /* first */ mode;\n");
  // After an `==`, `a`'s, as before.
  expect(js).toContain("  const same = a /* the other */ === b;\n");
  expect(js).toContain("  last = /* again */ (last | (mode + 1)) >>> 0;\n");
  expect(js).toContain("\nconst FLAGS = /*  */ 0b0100;\n");
  expect(js).toContain("\nlet count = /* none yet */ 0;\n");
  expect([lib.NoMode, lib.ConcurrentMode, lib.Removed, lib.modes(4 as never), lib.same(2 as never, 2 as never), lib.flags(), lib.flags()]).toEqual([0, 1, 1, 7, true, 6, 7]);
  // The JS after the comment maps to its Rust, the comment's width further.
  const segments = decodeMappings(map.mappings);
  const place = (text: string, code: string) => {
    const lines = text.split("\n");
    const line = lines.findIndex(l => l.includes(code));
    return [line, lines[line].indexOf(code)];
  };
  const mapped = (code: string) => {
    const [line, column] = place(js, code);
    const segment = segments.find(s => s.jsLine === line && s.jsCol === column);
    return [segment?.srcLine, segment?.srcCol];
  };
  expect(mapped("given | ConcurrentMode")).toEqual(place(rust, "given | CONCURRENT_MODE"));
  expect(mapped("0b0000001")).toEqual(place(rust, "0b0000001"));
});

// React's ReactFiberFlags: `ViewTransitionNamedStatic =\n /*    */
// SnapshotStatic | MaySuspendCommit`, too long for one line, which rustfmt
// writes with its comment on a line of its own between the `=` and the
// value: before the value still (ADR 0630, amended).
test("a comment on its own line between an = and its value is before the value", async () => {
  const { js, lib } = await compiled("comment-own-line-before-a-value", `#[cfg_attr(rust_js, rust_js::name = "SnapshotStatic")]
pub const SNAPSHOT_STATIC: i32 = /*                    */ 0b0000000000000000000000000000001;
#[cfg_attr(rust_js, rust_js::name = "MaySuspendCommit")]
pub const MAY_SUSPEND_COMMIT: i32 = /*                 */ 0b0000000000000000000000000000010;
#[cfg_attr(rust_js, rust_js::name = "ViewTransitionNamedStatic")]
pub const VIEW_TRANSITION_NAMED_STATIC: i32 =
    /*    */
    SNAPSHOT_STATIC | MAY_SUSPEND_COMMIT;

pub fn masked(given: i32, other: i32) -> bool {
    let mode =
        /* given */
        given | 4;
    mode ==
        // the other
        other
}
`);
  // On one line, the comment before the value; an app's Prettier breaks it after
  // the `=`, as upstream's is.
  expect(js).toContain("\nexport const ViewTransitionNamedStatic = /*    */ SnapshotStatic | MaySuspendCommit;\n");
  expect(js).toContain("  const mode = /* given */ given | 4;\n");
  expect(js).not.toContain("  /* given */\n");
  expect([lib.ViewTransitionNamedStatic, lib.masked(1 as never, 5 as never), lib.masked(2 as never, 5 as never)]).toEqual([3, true, false]);
});

// React's ReactFiberLane, its comments as upstream has them: one an `else`'s
// block starts with before its only `if`, `// Nothing has been pinged.`,
// keeps the block; one between an `||`'s operands before a parenthesized
// one; one before a call's argument that's a `const`, `NO_LANES`; and one
// after an `if` before a test MIR places at the keyword, `(lane & a) | ..
// != 0`, with one between its operands (ADR 0444, 0708, 0883, amended).
test("ReactFiberLane's comments in an else, an if's test and a call are where it writes them", async () => {
  const { js, lib } = await compiled("comments-of-lanes", `pub const NO_LANES: i32 = 0;

pub fn next(pinged: i32, pending: i32, has_commit: bool, warm: i32) -> i32 {
    let mut next_lanes = NO_LANES;
    if pinged != NO_LANES {
        next_lanes = pinged & 3;
    } else {
        // Nothing has been pinged. Check for lanes that need to be prewarmed.
        if !has_commit {
            let lanes_to_prewarm = pending & !warm;
            if lanes_to_prewarm != NO_LANES {
                next_lanes = lanes_to_prewarm & 5;
            }
        }
    }
    next_lanes
}

pub fn keep(next_lane: i32, wip_lane: i32) -> i32 {
    if
    // Tests whether the next lane is equal or lower priority than the wip
    // one. This works because the bits decrease in priority as you go left.
    next_lane >= wip_lane
        // Default priority updates should not interrupt transition updates.
        || (next_lane == 32 && (wip_lane & 256) != NO_LANES)
    {
        // Keep working on the existing in-progress tree. Do not interrupt.
        return wip_lane;
    }
    next_lane
}

fn mark_spawned_deferred_lane(lanes: &mut Vec<i32>, spawned_lane: i32, entangled_lanes: i32) {
    lanes.push(spawned_lane | entangled_lanes);
}

pub fn finish(lanes: &mut Vec<i32>, spawned_lane: i32) {
    if spawned_lane != 0 {
        mark_spawned_deferred_lane(
            lanes,
            spawned_lane,
            // This render finished successfully without suspending, so we don't need
            // to entangle the spawned task with the parent task.
            NO_LANES,
        );
    }
}

pub fn entangled(lane: i32, entangled_lanes: i32, entanglements: &[i32], index: usize) -> bool {
    if
    // Is this one of the newly entangled lanes?
    (lane & entangled_lanes)
        // Is this lane transitively entangled with the newly entangled lanes?
        | (entanglements[index] & entangled_lanes)
        != 0
    {
        return true;
    }
    false
}
`);
  expect(js).toContain(
    "  } else {\n    // Nothing has been pinged. Check for lanes that need to be prewarmed.\n    if (!hasCommit) {\n",
  );
  expect(js).toContain(
    "    nextLane >= wipLane ||\n    // Default priority updates should not interrupt transition updates.\n    (nextLane === 32 && (wipLane & 256) !== NO_LANES)\n",
  );
  expect(js).toContain(
    "      spawnedLane,\n      // This render finished successfully without suspending, so we don't need\n      // to entangle the spawned task with the parent task.\n      NO_LANES,\n    );\n",
  );
  expect(js).toContain(
    "  if (\n    // Is this one of the newly entangled lanes?\n    (lane & entangledLanes) |\n    // Is this lane transitively entangled with the newly entangled lanes?\n    ($index(entanglements, index, \"lib.rs:54:12\") & entangledLanes)\n  ) {\n",
  );
  const lanes: number[] = [];
  lib.finish(lanes as never, 4 as never);
  expect([lib.next(0 as never, 7 as never, false as never, 2 as never), lib.keep(1 as never, 2 as never), lib.keep(32 as never, 256 as never), lanes, lib.entangled(1 as never, 1 as never, [0] as never, 0 as never), lib.entangled(2 as never, 1 as never, [0] as never, 0 as never)]).toEqual([5, 1, 256, [4], true, false]);
});
