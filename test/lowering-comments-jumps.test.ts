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
