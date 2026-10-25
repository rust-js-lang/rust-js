// What the compiler writes of an early exit the program wrote, as it wrote
// it: a loop's `break` in the `if` it's in, not an `else` of the rest
// ending in a `continue`, nor an early `continue` the program never wrote
// (ADR 0730). Each test is a program of its own, compiled and run.

import { beforeAll, expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { buildReact, compiler, fixture, run, target } from "./support";

beforeAll(buildReact, 600_000);

/** `rust` compiled in a fixture of `name`, against the js crate: its JS,
 * and its module. */
async function compiled(name: string, rust: string): Promise<{ js: string; lib: Record<string, (...args: never[]) => unknown> }> {
  const dir = fixture(name);
  writeFileSync(join(dir, "lib.rs"), rust);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  return { js: readFileSync(join(dir, "lib.js"), "utf8"), lib: await import(join(dir, "lib.js")) };
}

// React's addObjectDiffToProperties (shared/ReactPerformanceTrackProperties.js):
// `if (checked > OBJECT_WIDTH_LIMIT) { ..; break; }` then the rest of the
// body. MIR has the rest as the `if`'s `else`, ending in a `continue`, and
// the `break` after it; and the rest's own `if (!(key in next))` is as
// written, not `if (key in next) { continue; }`.
test("a loop's early break is in the if the program wrote it in", async () => {
  const { js, lib } = await compiled("early-breaks-in-then", `const LIMIT: usize = 3;

pub fn checked(keys: &[String], next: &[String], log: &mut Vec<String>) -> bool {
    let mut equal = true;
    for (checked, key) in keys.iter().enumerate() {
        if checked > LIMIT {
            log.push("too many".to_string());
            equal = false;
            break;
        }

        if !next.contains(key) {
            log.push(key.clone());
            equal = false;
        }
    }

    let mut next_checked = 0;
    for key in next {
        if next_checked > LIMIT {
            log.push("too many".to_string());
            equal = false;
            break;
        }
        if keys.contains(key) {
            log.push(key.clone());
            if key.is_empty() {
                continue;
            }
        } else {
            equal = false;
        }
        next_checked += 1;
    }
    equal
}
`);
  expect(js).toContain('    if (checked > LIMIT) {\n      log.push("too many");\n      equal = false;\n      break;\n    }\n');
  expect(js).toContain("    if (!next.includes(key)) {\n      log.push(key);\n      equal = false;\n    }\n  }\n");
  expect(js).toContain('    if (nextChecked > LIMIT) {\n      log.push("too many");\n      equal = false;\n      break;\n    }\n    if (keys.includes(key)) {\n');
  // Its count is bounded by the check before it, so needs no wrap (ADR 0720).
  expect(js).toContain("    nextChecked++;\n  }\n");
  expect(js).not.toContain("} else {\n      if (");
  const results = [
    [["a", "b"], ["a", "c"]],
    [["a", "b", "c", "d", "e", "f"], ["a", "", "b", "c", "d", "e", "f"]],
  ].map(([keys, next]) => {
    const log: string[] = [];
    return [lib.checked(keys as never, next as never, log as never), log];
  });
  expect(results).toEqual([
    [false, ["b", "a"]],
    [false, ["too many", "a", "b", "c", "too many"]],
  ]);
});

// ADR 0745: React's addObjectDiffToProperties
// (shared/ReactPerformanceTrackProperties.js): each `continue` the program
// wrote in its chain of `if`s, `if (indent === 0 && key === "children") {
// ..; continue; }`, is a `continue` where it's written; an `if` it ends,
// `if (!nested) { .. } else if (..) { .. } continue;`, keeps its `else`;
// and no block MIR made is left, `bb294: { .. break bb294; .. } continue;`.
test("a continue the program wrote in a chain is where it wrote it", async () => {
  const { js, lib } = await compiled("continues-in-chains", `use js::{Kind, Unknown, function};

fn diff_nested(prev: &Unknown, next: &Unknown, out: &mut Vec<String>) -> bool {
    js::get(prev, "a") == js::get(next, "a") || {
        out.push("nested".to_string());
        false
    }
}

pub fn diff(prev: &Unknown, next: &Unknown, out: &mut Vec<String>, indent: usize) -> bool {
    let mut equal = true;
    let mut checked = 0;
    for key in js::keys_in(next) {
        if checked > 100 {
            equal = false;
            break;
        }

        if js::has(prev, &key) {
            let prev_value = js::get(prev, &key);
            let next_value = js::get(next, &key);
            if prev_value != next_value {
                if indent == 0 && key == "children" {
                    // The top level children prop.
                    out.push(key.clone() + " children");
                    equal = false;
                    continue;
                }
                if indent >= 3 {
                    // Deep: the two values.
                } else if let Some(prev_object) = prev_value
                    && let Some(next_object) = next_value
                    && let Kind::Object(_) = js::classify(prev_value)
                    && let Kind::Object(_) = js::classify(next_value)
                {
                    if js::has(next_object, "type") {
                        if js::get(prev_object, "type") == js::get(next_object, "type")
                            && js::get(prev_object, "key") == js::get(next_object, "key")
                        {
                            // Same element.
                            out.push(key.clone() + " element");
                            equal = false;
                            continue;
                        }
                    } else {
                        let before = out.len();
                        if !diff_nested(prev_object, next_object, out) {
                            equal = false;
                        } else if before == out.len() {
                            // Deeply equal.
                            out.push(key.clone() + " deeply equal");
                        }
                        continue;
                    }
                } else if let Kind::Function(prev_function) = js::classify(prev_value)
                    && let Kind::Function(next_function) = js::classify(next_value)
                    && function::name(prev_function) == function::name(next_function)
                {
                    // Same function.
                    out.push(key.clone() + " function");
                    continue;
                }

                // Otherwise, the change.
                out.push(key.clone() + " changed");
                equal = false;
            }
        } else {
            out.push(key + " added");
            equal = false;
        }

        checked += 1;
    }
    equal
}
`);
  expect(js).not.toMatch(/bb\d+/);
  expect(js).toContain("        if (indent >= 3) {\n          // Deep: the two values.\n        } else if (\n");
  expect(js).toContain('        if (indent === 0 && key === "children") {\n          // The top level children prop.\n          out.push(key + " children");\n          equal = false;\n          continue;\n        }\n');
  expect(js).toContain("            if (!diffNested(prevValue, nextValue, out)) {\n              equal = false;\n            } else if (before === out.length) {\n");
  expect(js).toContain('              out.push(key + " deeply equal");\n            }\n            continue;\n          }\n        } else if (\n');
  expect(js).toContain('          // Same function.\n          out.push(key + " function");\n          continue;\n        }\n\n        // Otherwise, the change.\n');
  const f = () => 1;
  const g = () => 2;
  const runs = [
    [{ children: 1, x: 1 }, { children: 2, x: 1, y: 2 }, 0],
    [{ e: { type: "div", key: 1 }, o: { a: 1 }, p: { a: 1 } }, { e: { type: "div", key: 1 }, o: { a: 2 }, p: { a: 1 } }, 0],
    [{ f, children: 1 }, { f: g, children: 2 }, 1],
    [{ o: { a: 1 } }, { o: { a: 2 } }, 3],
  ].map(([prev, next, indent]) => {
    const out: string[] = [];
    return [lib.diff(prev as never, next as never, out as never, indent as never), out];
  });
  expect(runs).toEqual([
    [false, ["children children", "y added"]],
    [false, ["e element", "nested", "p deeply equal"]],
    [false, ["f function", "children changed"]],
    [false, ["o changed"]],
  ]);
});

// React's enqueueTask (shared/enqueueTask.js), as react-in-rust ports it:
// its last statement an `if let` with no `else`, in the function
// `js::export_default!` holds. MIR's `return` a macro's function ends with
// is where the macro is called, before its arm, so the arm is made the
// `if`'s `else` and a `return;` its `then`: once each branch's `return;`
// the program didn't write is gone, the emptied `then` is no branch, `if
// (enqueueTaskImpl != null) { .. }`, not `if (enqueueTaskImpl == null) {}
// else { .. }`.
test("an if let a macro's function ends with is no empty then and an else", async () => {
  const { js, lib } = await compiled("macro-tail-if-let", `use std::cell::Cell;

type EnqueueTask = fn(&'static dyn Fn());

thread_local! {
    static ENQUEUE_TASK_IMPL: Cell<Option<EnqueueTask>> = const { Cell::new(None) };
}

pub fn set_enqueue_task_impl(f: EnqueueTask) {
    ENQUEUE_TASK_IMPL.set(Some(f));
}

js::export_default! {
    fn enqueue_task(task: &'static dyn Fn()) {
        if let Some(enqueue_task_impl) = ENQUEUE_TASK_IMPL.get() {
            enqueue_task_impl(task);
        }
    }
}
`);
  expect(js).toContain("  if (enqueueTaskImpl != null) {\n    enqueueTaskImpl(task);\n  }\n}\n");
  expect(js).not.toMatch(/\{\n\s*\} else \{/);
  const enqueueTask = lib.default as (task: () => void) => void;
  const ran: string[] = [];
  const task = () => ran.push("task");
  enqueueTask(task);
  expect(ran).toEqual([]);
  lib.setEnqueueTaskImpl(((t: () => void) => {
    ran.push("enqueued");
    t();
  }) as never);
  enqueueTask(task);
  expect(ran).toEqual(["enqueued", "task"]);
});
