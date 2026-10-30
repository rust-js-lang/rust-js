// Where the compiler writes a comment at the end of a list: after a
// parameter on its line, `start, // DEV-only`, and on lines of its own
// after a struct literal's last field, before its `}` (ADR 0708, amended).
// Each test is a program of its own, compiled and run.

import { expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { compiler, fixture, run } from "./support";

/** `rust` compiled in a fixture of `name`: its JS and its module. */
async function compiled(name: string, rust: string): Promise<{ js: string; lib: Record<string, (...args: never[]) => unknown> }> {
  const dir = fixture(name);
  writeFileSync(join(dir, "lib.rs"), rust);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  return { js, lib: await import(join(dir, "lib.js")) };
}

// React's ReactFiberCacheComponent and ReactPerformanceTrack write
// `startTime: number, // DEV-only` in a parameter list, and a comment
// after an object literal's last property: `{ name, start, // We don't
// know the owner. }`. Its port's comments were the function body's first
// line and above the `return`.
test("a comment after a parameter or a struct literal's last field is where it's written", async () => {
  const { js, lib } = await compiled("comments-at-list-ends", `pub struct Info {
    pub name: &'static str,
    pub start: f64,
}

pub fn track(
    name: &'static str,
    start: f64, // DEV-only
) -> Info {
    Info {
        name,
        start,
        // We don't know the owner.
    }
}

pub fn first(
    name: &'static str, // The name.
    start: f64,
) -> f64 {
    start + name.len() as f64
}

pub fn made(name: &'static str) -> Info {
    Info {
        name,
        // Not yet started.
        start: 0.0,
    }
}
`);
  expect(js).toContain(`export function track(
  name,
  start, // DEV-only
) {
  return {
    name,
    start,
    // We don't know the owner.
  };
}`);
  expect(js).toContain(`export function first(
  name, // The name.
  start,
) {`);
  // One before a field is before it, as before (ADR 0708).
  expect(js).toContain("  return {\n    name,\n    // Not yet started.\n    start: 0,\n  };\n");
  expect(lib.track("a" as never, 2 as never)).toEqual({ name: "a", start: 2 });
  expect(lib.first("ab" as never, 1 as never)).toBe(3);
});

// React's getActivityInstanceFromFiber ends `// TODO: Implement this on
// ActivityComponent.` then `return null;`. The port's tail `None` of a
// nullable result is `return null;`, where the `None`'s comments end: they
// were a block's of only comments there, printed after it (ADR 0862,
// amended).
test("a comment before a tail None is before its return", async () => {
  const { js, lib } = await compiled("comment-before-tail-none", `#[cfg_attr(rust_js, rust_js::nullable)]
pub fn instance(tag: u8) -> Option<u32> {
    if tag == 31 {
        return Some(1);
    }
    // TODO: Implement this on ActivityComponent.
    None
}
`);
  expect(js).toContain("  // TODO: Implement this on ActivityComponent.\n  return null;\n}");
  expect([lib.instance(31 as never), lib.instance(1 as never)]).toEqual([1, null]);
});

// ADR 0753, amended: a `match` arm of only comments, ReactFiberApplyGesture's
// `case HostPortal: { // TODO: .. break; }`, and a guarded arm's,
// finishConcurrentRender's `// Commit the placeholder.` before its
// `break;`, keep their comments before the `break;` tidy ends them with,
// not in the next case nor past the `break;`.
test("an arm of only comments keeps them before its break", async () => {
  const { js, lib } = await compiled("comment-arm-break", `pub fn visit(tag: u8, log: &mut Vec<u8>) {
    match tag {
        0 => {
            log.push(0);
        }
        4 => {
            // TODO: Consider what should happen to Portals.
        }
        _ => {
            log.push(9);
        }
    }
}

fn includes(lanes: u32, lane: u32) -> bool {
    lanes & lane != 0
}

pub fn finish(status: u32, lanes: u32, log: &mut Vec<u32>) {
    match status {
        4 if !includes(lanes, 1) && !includes(lanes, 2) => {
            // Commit the placeholder.
        }
        4 | 5 => {
            log.push(1);
            return;
        }
        6 => {}
        _ => panic!("Unknown root exit status."),
    }
    log.push(2);
}
`);
  expect(js).toContain("    case 4:\n      // TODO: Consider what should happen to Portals.\n      break;\n    default:\n      log.push(9);\n");
  expect(js).toContain("      if (!includes(lanes, 1) && !includes(lanes, 2)) {\n        // Commit the placeholder.\n        break;\n      }\n");
  const seen = (f: (log: number[]) => void) => {
    const log: number[] = [];
    f(log);
    return log;
  };
  expect([0, 4, 7].map((tag) => seen((log) => (lib.visit as (t: number, l: number[]) => void)(tag, log)))).toEqual([[0], [], [9]]);
  const finish = lib.finish as (s: number, l: number, log: number[]) => void;
  expect([seen((log) => finish(4, 4, log)), seen((log) => finish(4, 1, log)), seen((log) => finish(5, 3, log)), seen((log) => finish(6, 3, log))])
    .toEqual([[2], [1], [1], [2]]);
  expect(() => finish(7, 0, [])).toThrow("Unknown root exit status.");
});

// React's throwException writes `} else { // Otherwise, fall through to the
// error path. }` after a branch that returns each way, its test an `&&`:
// preparation made the test again of its parts, which lost its written
// `else`, and the comment printed at the function's end (rg3).
test("an else of only comments after a branch that leaves stays", async () => {
  const { js, lib } = await compiled("comment-else-after-return", `pub fn thrown(hydrating: bool, legacy: bool, boundary: Option<&u32>, log: &mut Vec<u32>) -> bool {
    if hydrating && (legacy || log.is_empty()) {
        if let Some(b) = boundary {
            log.push(*b);
            return true;
        } else {
            log.push(2);
            return false;
        }
    } else {
        // Otherwise, fall through to the error path.
    }

    log.push(3);
    false
}
`);
  expect(js).toContain("  } else {\n    // Otherwise, fall through to the error path.\n  }\n\n  log.push(3);\n  return false;\n}");
  const thrown = lib.thrown as (h: boolean, l: boolean, b: number | undefined, log: number[]) => boolean;
  const logs = [true, false].map((h) => {
    const log: number[] = [];
    return [thrown(h, false, h ? 7 : undefined, log), log];
  });
  expect(logs).toEqual([[true, [7]], [false, [3]]]);
});

// React's WorkLoop handleThrow: a match's last arm of only comments,
// `_ => { // .. }`, the `default:` JS falls out of, no `break` after its
// comments to be before, nor a next case: they were lost, or after the
// function (ADR 0753, amended).
test("a last arm of only comments keeps them in its clause", async () => {
  const { js, lib } = await compiled("comments-in-the-last-clause", `pub fn mark(reason: i32, log: &mut Vec<i32>) {
    if reason > 0 {
        match reason {
            1 => log.push(1),
            2 => {
                // Second: nothing to do.
            }
            _ => {
                // Last: this comment belongs to the default case.
            }
        }
    }
}

pub fn last(reason: i32, log: &mut Vec<i32>) {
    match reason {
        1 => log.push(1),
        2 => log.push(2),
        3 => {
            // The last case: its comment stays in it.
        }
        _ => {}
    }
}
`);
  expect(js).toContain("      default:\n      // Last: this comment belongs to the default case.\n    }\n  }\n}");
  expect(js).toContain("// The last case: its comment stays in it.\n  }\n}");
  const log: number[] = [];
  for (const reason of [1, 2, 3]) {
    lib.mark(reason as never, log as never);
    lib.last(reason as never, log as never);
  }
  expect(log).toEqual([1, 1, 2]);
});

// ADR 0658, amended: ReactFiberApplyGesture's `nextPhase = CLONE_UNHIDE;
// // TODO: Mark the name and find a pair.`, written `let next_phase = if
// .. { // .. CLONE_UNHIDE // TODO: .. } else { phase };`, kept a
// conditional as clippy's needless_late_init has it: the comment after a
// branch's value is right after it, `2 // TODO`, not lost or above the loop.
test("a comment after a branch's value is right after it", async () => {
  const { js, lib } = await compiled("comment-after-branch-value", `pub fn visit(phases: &[u8]) -> Vec<u8> {
    let mut seen = Vec::new();
    for &phase in phases {
        let next_phase = if phase == 1 {
            // This was an Enter of a ViewTransition.
            2
            // TODO: Mark the name and find a pair.
        } else {
            phase
        };
        seen.push(next_phase);
    }
    seen
}

pub fn pick(flag: bool) -> u8 {
    if flag {
        3
        // The first.
    } else {
        4
        // The second.
    }
}
`);
  expect(js).toContain(
    "        ? // This was an Enter of a ViewTransition.\n          2 // TODO: Mark the name and find a pair.\n        : phase;",
  );
  expect(js).toContain("  for (const phase of phases) {\n    const nextPhase");
  expect(js).toContain("return flag\n    ? 3 // The first.\n    : 4; // The second.");
  expect([lib.visit([1, 5] as never), lib.pick(true as never), lib.pick(false as never)]).toEqual([[2, 5], 3, 4]);
});

// React's ReactFiberConfigDOM's startGestureTransition ends a `for` body,
// and an `if`'s block in it, with a comment after an `if`/`else`, which
// gives no `()` of its own that marks where the block's comments end (ADR
// 0753): it was before the `for` (rd219, #428).
test("a comment ending a loop's body after an if/else is at its end", async () => {
  const { js, lib } = await compiled("comments-ending-a-loop", `pub fn scan(names: &[Option<&str>], first: bool) -> (bool, bool) {
    let mut grouped = false;
    let mut exited = false;
    for name in names.iter() {
        if first {
            grouped = name.is_some_and(|n| n.starts_with("-ua-"));
        } else {
            // Only an old state.
            exited = true;
        }
        // TODO: If this has only an old state and no new state,
    }
    (grouped, exited)
}

pub fn count(n: u32, odd: bool) -> u32 {
    let mut i = 0;
    let mut seen = 0;
    while i < n {
        if odd {
            seen += i % 2;
        } else {
            seen += 1;
        }
        // Each one counted.
        i += 1;
        // Then the next.
    }
    seen
}

pub fn walk_last(items: &[u32], found: bool) -> u32 {
    let mut seen = 0;
    for item in items {
        if *item > 1 {
            if found {
                seen += item;
            } else {
                seen += 1;
            }
            // TODO: a comment ending the if's block.
        }
    }
    seen
}
`);
  expect(js).toContain("      exited = true;\n    }\n    // TODO: If this has only an old state and no new state,\n  }\n  return [grouped, exited];");
  expect(js).toContain("        seen = (seen + 1) >>> 0;\n      }\n      // TODO: a comment ending the if's block.\n    }\n  }");
  expect(js).toContain("    i++;\n    // Then the next.\n  }");
  expect([lib.scan([null] as never, false as never), lib.count(4 as never, true as never), lib.walkLast([1, 2, 3] as never, false as never)]).toEqual([[false, true], 2, 2]);
});

// React's ReactFiberConfigDOM's startGestureTransition writes a call an
// argument a line, its last a constant with a comment after it: `true, //
// We let the helper apply the translate`. A constant had no place in the
// map, so the comment, placed by its line, was lost (rd220).
test("a comment after a call's last argument, a constant, is after it", async () => {
  const { js, lib } = await compiled("comments-after-a-last-constant", `#[allow(clippy::too_many_arguments)]
fn animate(a: u32, b: u32, c: u32, d: u32, e: u32, f: u32, g: u32, h: bool, i: bool) -> u32 {
    a + b + c + d + e + f + g + h as u32 + i as u32
}

pub fn run(keyframes: u32, target: u32, timeline: u32, start: u32) -> u32 {
    animate(
        keyframes,
        target,
        timeline,
        1,
        2,
        start,
        3,
        false,
        true, // We let the helper apply the translate
    )
}
`);
  expect(js).toContain("    false,\n    true, // We let the helper apply the translate\n  );");
  expect(lib.run(1 as never, 2 as never, 3 as never, 4 as never)).toBe(17);
});

// React's ReactFiberHooks' rerenderActionState gives a tuple at its end
// with a comment above it: never inside the array, as a constant that
// isn't a call's argument has no place in the map (rd220's follow-up).
test("a comment before a tail tuple with a constant is never inside its array", async () => {
  const { js, lib } = await compiled("comments-before-a-tail-tuple", `pub fn rerender(state: u32) -> (u32, bool) {
    let s = state + 1;

    // For mount, pending is always false.
    (s, false)
}
`);
  expect(js).not.toContain("[\n    // For mount, pending is always false.");
  expect(js).toContain("return [s, false];");
  expect(lib.rerender(1 as never)).toEqual([2, false]);
});
