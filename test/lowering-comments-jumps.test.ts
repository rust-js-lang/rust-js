// Where the compiler writes a comment of control flow MIR has no statement
// of: before a `break` or a `continue` the program wrote that tidy takes,
// inside its branch, not above the loop (rd109); and in a `switch`, between
// an arm's patterns and in a last `_` of only comments (rd138, rd139). Each
// test is a program of its own, compiled and run.

import { beforeAll, expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { buildReact, compiler, fixture, run, target } from "./support";

beforeAll(buildReact, 600_000);

/** `rust` compiled in a fixture of `name`, against the js crate: its JS
 * and its module. */
async function compiled(name: string, rust: string): Promise<{ js: string; lib: Record<string, (...args: never[]) => unknown> }> {
  const dir = fixture(name);
  writeFileSync(join(dir, "lib.rs"), rust);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  return { js, lib: await import(join(dir, "lib.js")) };
}

// React DOM's replayUnblockedFormActions: a `break` the function's end
// is MIR's `return;`, and a `continue` the loop's end is anyway, which tidy
// takes, each with comments before it. Each jump is kept where it's
// written, its comments before it (the user's decision, 2026-10-29: as
// upstream's `if (blockedOn === null) { ..; continue; } else { break; }`),
// and what follows an `if` written without an `else` after it. The
// comments were above the `while`, then at the end of the branch.
test("a comment before a break or continue tidy takes is where it was written", async () => {
  const { js, lib } = await compiled("comment-before-a-taken-jump", `fn blocked(x: i32) -> Option<i32> {
    (x > 10).then_some(x)
}

fn ours(x: i32) -> Option<i32> {
    (x > 5).then_some(x)
}

pub fn replay(queue: &mut Vec<i32>, log: &mut Vec<i32>) {
    let mut i = 0;
    while i < queue.len() {
        let action = queue[i];
        if action < 0 {
            let blocked_on = blocked(-action);
            if blocked_on.is_none() {
                // Not blocked: skip past it.
                i += 3;
                continue;
            } else {
                // We're blocked. We'll retry later.
                break;
            }
        }
        let inst = ours(action);
        if let Some(inst) = inst {
            // Ready: delete it from the queue.
            queue.remove(i);
            log.push(inst);
            // Continue without incrementing the index.
            continue;
        }
        // Another instance's.
        i += 3;
    }
}

pub fn replay_uncommented(queue: &[i32], seen: &mut Vec<i32>) {
    let mut i = 0;
    while i < queue.len() {
        let action = queue[i];
        if action < 0 {
            if blocked(-action).is_none() {
                i += 3;
                continue;
            } else {
                break;
            }
        }
        seen.push(action);
        i += 1;
    }
}
`);
  expect(js).toContain("  let i = 0;\n  while (i < queue.length) {\n");
  expect(js).toContain(
    "        i += 3;\n        continue;\n      } else {\n        // We're blocked. We'll retry later.\n        break;\n      }\n    }\n    const inst = ours(action);\n",
  );
  expect(js).toContain("      log.push(inst);\n      // Continue without incrementing the index.\n      continue;\n    }\n    // Another instance's.\n    i += 3;\n  }\n");
  // Of none, the jumps as written too.
  expect(js).toContain("      if (blocked(-action | 0) == null) {\n        i += 3;\n        continue;\n      } else {\n        break;\n      }\n    }\n    seen.push(action);\n");
  const replayed = (queue: number[]) => {
    const log: number[] = [];
    lib.replay(queue as never, log as never);
    return [queue, log];
  };
  expect(replayed([7, 1, 2, 3, -2, 0, 0, 8, -20, 9])).toEqual([[1, 2, 3, -2, 0, 0, -20, 9], [7, 8]]);
  expect(replayed([1, 0, 0, 6, -11, 5])).toEqual([[1, 0, 0, -11, 5], [6]]);
  const seen = (queue: number[]) => {
    const seen: number[] = [];
    lib.replayUncommented(queue as never, seen as never);
    return seen;
  };
  expect([seen([1, -2, 0, 0, 3, -20, 1]), seen([-1, 2, 3, 4])]).toEqual([[1, 3], [4]]);
});

// A `continue` the program wrote last in a loop's body, or last in a match
// arm there, not an `if`'s branch, which tidy takes as the turn ends there
// anyway: its comments at the end of the body or the branch, where it was
// written (ADR 0753), not above the `for`.
test("a comment before a continue tidy takes at a loop's end is where it was written", async () => {
  const { js, lib } = await compiled("comment-before-a-tail-continue", `pub fn tail(xs: &[i32], out: &mut Vec<i32>) {
    for &x in xs {
        out.push(x);
        // On to the next.
        continue;
    }
}

pub fn arm(xs: &[i32], out: &mut Vec<i32>) {
    for &x in xs {
        match x {
            0 => {
                out.push(0);
                // Zero: the next.
                continue;
            }
            _ => out.push(x * 2),
        }
    }
}
`);
  expect(js).toContain("  for (const x of xs) {\n    out.push(x);\n    // On to the next.\n  }\n");
  expect(js).toContain("    if (x === 0) {\n      out.push(0);\n      // Zero: the next.\n    } else {\n");
  const pushed = (f: string, xs: number[]) => {
    const out: number[] = [];
    lib[f](xs as never, out as never);
    return out;
  };
  expect([pushed("tail", [1, 2]), pushed("arm", [0, 3])]).toEqual([
    [1, 2],
    [0, 6],
  ]);
});

// A `break` the program wrote as all of a match arm, its comments before
// it, in the loop the function ends with, which MIR makes the function's
// `return;`: the comments where it's written, in that branch, not lost.
test("a comment before a match arm's break MIR made a return is in its branch", async () => {
  const { js, lib } = await compiled("comment-before-an-arms-break", `pub fn doubled(xs: &[i32], out: &mut Vec<i32>) {
    let mut i = 0;
    while i < xs.len() {
        match xs[i] {
            0 => {
                // Zero: no more.
                break;
            }
            x => out.push(x * 2),
        }
        i += 1;
    }
}
`);
  expect(js).toContain("    if (xs[i] === 0) {\n      // Zero: no more.\n");
  const out: number[] = [];
  lib.doubled([1, 2, 0, 3] as never, out as never);
  expect(out).toEqual([2, 4]);
});

// React DOM's SimpleEventPlugin: a guarded arm that falls through to the
// next, a comment between that arm's patterns, `"mouseup" // TODO: ..
// | "mouseout"`, and a last `_` of only comments. The first comment between
// the two `case`s, as upstream writes it, not in the case's body (rd139);
// the last in the `default`, which tidy ends with a `break` the printer
// leaves out, not after the switch's `}` (rd138).
test("a comment between an arm's patterns or in a last _ of a guarded switch is in its clause", async () => {
  const { js, lib } = await compiled("comments-of-a-switch", `pub fn extract(name: &str, log: &mut Vec<String>) {
    let mut kind = 0;
    match name {
        "keypress" if log.is_empty() => {
            return;
        }
        "keypress" | "keydown"
        // A comment between patterns.
        | "keyup" => {
            kind = 1;
        }
        "click" => {
            kind = 2;
        }
        "drag" | "drop" => {
            kind = 3;
        }
        _ => {
            // Unknown event. Nothing to do.
        }
    }
    log.push(format!("{name} {kind}"));
}
`);
  expect(js).toContain(`    // Fallthrough
    case "keydown":
    // A comment between patterns.
    case "keyup":
      kind = 1;
      break;
`);
  expect(js).toContain(`    default:
    // Unknown event. Nothing to do.
  }
  log.push(\`\${name} \${kind}\`);
`);
  const log: string[] = [];
  for (const name of ["keypress", "click", "keypress", "keyup", "drop", "blur"]) lib.extract(name as never, log as never);
  expect(log).toEqual(["click 2", "keypress 1", "keyup 1", "drop 3", "blur 0"]);
});

// React's reappearLayoutEffects and validateDOMNesting's
// isTagValidWithParent: a comment, or a blank line, above an arm of more
// than one pattern is above its first `case`, as upstream writes it, not
// between its `case`s, where one between its patterns is (rd143).
test("a comment or a blank line above an arm of patterns is above its first case", async () => {
  const { js, lib } = await compiled("comments-above-stacked-cases", `const A: u8 = 0;
const B: u8 = 1;
const C: u8 = 2;
const D: u8 = 3;

pub fn effects(tag: u8, log: &mut Vec<u8>) {
    match tag {
        A | B => {
            log.push(1);
        }
        // Above the arm.
        // A second line.
        C | D => {
            log.push(2);
        }
        _ => {}
    }
}

pub fn valid(tag: &str, parent: Option<&str>) -> bool {
    match tag {
        "x" => return true,

        "rp" | "rt" => {
            return parent.is_none();
        }
        _ => {}
    }
    false
}
`);
  expect(js).toContain("      break;\n    // Above the arm.\n    // A second line.\n    case C:\n    case D:\n");
  expect(js).toContain('      return true;\n\n    case "rp":\n    case "rt":\n');
  const log: number[] = [];
  for (const tag of [0, 3, 7]) lib.effects(tag as never, log as never);
  expect([log, lib.valid("rt" as never, undefined as never), lib.valid("y" as never, undefined as never)]).toEqual([[1, 2], true, false]);
});
