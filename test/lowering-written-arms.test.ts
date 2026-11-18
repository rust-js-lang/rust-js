// What the compiler writes of arms the program wrote, as it wrote them:
// an `else` after a branch that returns, a function's last match of arms
// that give `()`, an `if let` of a field by its own name, and a variable's
// value of a chain of arms from MIR (ADRs 0237, 0866, 0879, 0969). Split
// from lowering-written-branches.test.ts by concept, at its cap.

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

// The user's decision (2026-10-27), ADR 0879, amended: React's
// insertOrAppendPlacementNode writes `if (isHost) { ..; return; } else if
// (tag === HostPortal) { return; } ..more`. An `else` the program wrote
// after a branch that returns stays, where statements follow the `if`; a
// chain the function ends with is still each `if` after the last (ADR
// 0237), and `if`s written apart stay apart.
// The user's decision (2026-10-28), ADR 0237, amended: every `else` the
// program wrote stays, statements after the `if` or not, and an `else if`
// chain is one, as React's BeginWork writes both arms.
test("an else after a branch that returns stays, as written", async () => {
  const { js, lib } = await compiled(
    "else-after-return",
    `pub fn place(tag: u8, log: &mut Vec<u8>) {
    let is_host = tag == 5 || tag == 6;
    if is_host {
        log.push(tag);
        return;
    } else if tag == 4 {
        // A portal's children are placed on their own.
        return;
    }
    log.push(0);
}

pub fn counted(key: u32, total: &mut u32) -> u32 {
    if key == 0 {
        return 0;
    } else {
        *total += key;
    }
    *total
}

pub fn turned(key: u32, total: &mut u32) -> u32 {
    if key != 0 {
        *total += key;
    } else {
        return 0;
    }
    *total
}

pub fn apart(key: u32) -> u32 {
    if key == 1 {
        return 10;
    }
    if key == 2 {
        return 20;
    }
    key
}

pub fn last(key: u32) -> u32 {
    if key == 1 {
        return 10;
    } else if key == 2 {
        return 20;
    } else {
        key + 1
    }
}

pub fn first(a: &[u32], b: &[u32]) -> u32 {
    if let Some(x) = a.first() {
        *x
    } else if let Some(y) = b.last() {
        y + 1
    } else {
        0
    }
}

pub fn inner(key: u32, total: &mut u32) -> u32 {
    if key > 0 {
        *total += 1;
        if key == 2 {
            return 0;
        } else if key == 3 {
            return 1;
        } else {
            *total += key;
        }
    }
    *total
}
`,
  );
  expect(js).toContain(
    "  if (isHost) {\n    log.push(tag);\n    return;\n  } else if (tag === 4) {\n    // A portal's children are placed on their own.\n    return;\n  }\n  log.push(0);\n",
  );
  expect(js).toContain("  if (key === 0) {\n    return 0;\n  } else {\n    total.value = (total.value + key) >>> 0;\n  }\n  return total.value;\n");
  expect(js).toContain("  if (key) {\n    total.value = (total.value + key) >>> 0;\n  } else {\n    return 0;\n  }\n  return total.value;\n");
  expect(js).toContain("  if (key === 1) {\n    return 10;\n  }\n  if (key === 2) {\n    return 20;\n  }\n  return key;\n");
  expect(js).toContain(
    "  if (key === 1) {\n    return 10;\n  } else if (key === 2) {\n    return 20;\n  } else {\n    return (key + 1) >>> 0;\n  }\n",
  );
  expect(js).toContain(
    "    if (key === 2) {\n      return 0;\n    } else if (key === 3) {\n      return 1;\n    } else {\n      total.value = (total.value + key) >>> 0;\n    }\n  }\n  return total.value;\n",
  );
  const placed = [5, 4, 3].map((tag) => {
    const log: number[] = [];
    lib.place(tag as never, log as never);
    return log;
  });
  expect(placed).toEqual([[5], [], [0]]);
  const total = { value: 1 };
  expect([lib.counted(0 as never, total as never), lib.counted(2 as never, total as never)]).toEqual([0, 3]);
  expect([lib.turned(0 as never, total as never), lib.turned(4 as never, total as never)]).toEqual([0, 7]);
  expect([1, 2, 3].map((k) => [lib.apart(k as never), lib.last(k as never)])).toEqual([[10, 10], [20, 20], [3, 4]]);
  // An `else if let`'s binding is no JS `else if`: each link after the last.
  expect(js).toContain("  if (x != null) {\n    return x;\n  }\n  const y = b.at(-1);\n  return y != null ? (y + 1) >>> 0 : 0;\n");
  expect([lib.first([4] as never, [1] as never), lib.first([] as never, [1] as never), lib.first([] as never, [] as never)]).toEqual([4, 2, 0]);
  const inner = { value: 0 };
  expect([0, 2, 3, 4].map((k) => lib.inner(k as never, inner as never))).toEqual([0, 0, 1, 7]);
});

// The user's decision (2026-10-27), ADR 0879, amended: React's
// commitLayoutEffectOnFiber's `if (current === null) { if (..) { .. } else
// { try { .. } catch (error) { .. } } } else { .. }`. A function's last
// `match` of arms that give `()` is its arms as written, an `if`'s and a
// `try`'s branches too, with no `return;` MIR made at their ends.
test("a function's last match of arms that give () is its arms, no returns", async () => {
  const { js, lib } = await compiled(
    "tail-match-arms",
    `use std::panic::{AssertUnwindSafe, catch_unwind};

pub fn lifecycles(current: Option<u32>, profile: bool, log: &mut Vec<u32>) {
    match current {
        None => {
            if profile {
                log.push(1);
            } else if let Err(error) = catch_unwind(AssertUnwindSafe(|| {
                if log.len() > 2 {
                    panic!("full");
                }
                log.push(2);
            })) && let Some(error) = error.downcast_ref::<&'static str>()
            {
                log.push(error.len() as u32);
            }
        }
        Some(prev) => log.push(prev),
    }
}
`,
  );
  expect(js).toContain("  if (current == null) {\n    if (profile) {\n      log.push(1);\n    } else ");
  expect(js).toContain("  } else {\n    log.push(current);\n  }\n}");
  expect(js).not.toContain("return;");
  const log: number[] = [];
  lib.lifecycles(undefined as never, true as never, log as never);
  lib.lifecycles(undefined as never, false as never, log as never);
  lib.lifecycles(7 as never, false as never, log as never);
  lib.lifecycles(undefined as never, false as never, log as never);
  expect(log).toEqual([1, 2, 7, 4]);
});

// The user's decision (2026-10-27), ADR 0866, amended: React's
// getViewTransitionName, `if (instance.autoName !== null) { return
// instance.autoName; }`. An `if let` of a plain field whose binding is the
// field's own name, the `then` returning it first, reads the field in place
// each time; a binding of another name keeps it.
test("an if let of a field by its own name reads the field in place", async () => {
  const { js, lib } = await compiled(
    "own-name-field-read",
    `use std::cell::RefCell;

pub struct ViewTransitionState {
    pub auto_name: RefCell<Option<String>>,
}

pub fn get_view_transition_name(instance: &ViewTransitionState) -> String {
    if let Some(auto_name) = &*instance.auto_name.borrow() {
        return auto_name.clone();
    }
    String::new()
}

pub fn named(instance: &ViewTransitionState) -> String {
    if let Some(name) = &*instance.auto_name.borrow() {
        return name.clone();
    }
    String::new()
}
`,
  );
  expect(js).toContain("  if (instance.autoName != null) {\n    return instance.autoName;\n  }\n");
  expect(js).toContain("  const name = instance.autoName;\n");
  expect([
    lib.getViewTransitionName({ autoName: "a" } as never),
    lib.getViewTransitionName({ autoName: null } as never),
    lib.named({ autoName: "b" } as never),
    lib.named({ autoName: null } as never),
  ]).toEqual(["a", "", "b", ""]);
});

// ADR 0969: a variable's value of a chain of four arms or more, from MIR, is
// the chain, each arm its own, as of three: not its first arm a statement
// and the rest a conditional, `else { color = a ? .. : b ? .. : .. }`, nor an
// arm MIR wrote the other way round negated, `!(selfTime < 100) ? .. : ..`.
// Where every arm is a value alone, a literal or a name, the chain is one
// nested conditional, `const color = a ? "x" : b ? "y" : "z";` (amended).
// React's ReactFiberPerformanceTrack's colors.
test("a variable's value of four arms from MIR is their chain", async () => {
  const dir = fixture("mir-four-arms");
  writeFileSync(join(dir, "lib.rs"), `pub fn log(yield_duration: f64, was_hydrated: bool, self_time: f64) -> String {
    let color = if yield_duration < 5.0 {
        "primary-light"
    } else if yield_duration < 10.0 {
        "primary"
    } else if yield_duration < 100.0 {
        "primary-dark"
    } else {
        "error"
    };
    let other = if self_time < 0.5 {
        if was_hydrated { "tertiary-light" } else { "primary-light" }
    } else if self_time < 10.0 {
        if was_hydrated { "tertiary" } else { "primary" }
    } else if self_time < 100.0 {
        if was_hydrated { "tertiary-dark" } else { "primary-dark" }
    } else {
        "error"
    };
    color.to_string() + other
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain(`  const color =
    yieldDuration < 5
      ? "primary-light"
      : yieldDuration < 10
        ? "primary"
        : yieldDuration < 100
          ? "primary-dark"
          : "error";`);
  expect(js).toContain(`  } else if (selfTime < 100) {
    other = wasHydrated ? "tertiary-dark" : "primary-dark";
  } else {
    other = "error";
  }`);
  const lib = await import(join(dir, "lib.js"));
  expect([lib.log(1, true, 0.1), lib.log(50, false, 50), lib.log(500, true, 500)]).toEqual([
    "primary-lighttertiary-light",
    "primary-darkprimary-dark",
    "errorerror",
  ]);
});

// A match of three arms or more giving a variable its value, from MIR, is
// the chain of them: not its last two folded, \`c ? a : b\`. Where every
// arm is a value alone, a literal or a name, it's one nested conditional,
// \`const cls = a ? "note" : b ? "warn" : "tip"\` (ADR 0969, amended).
test("a variable's value of three arms from MIR is their chain", async () => {
  const dir = fixture("mir-three-arms");
  writeFileSync(join(dir, "lib.rs"), `#[derive(Clone, Copy)]
pub enum Kind { Note, Warn { level: u8 }, Tip(&'static str) }
pub fn class(k: Kind) -> String {
    let cls = match k {
        Kind::Note => "note",
        Kind::Warn { .. } => "warn",
        Kind::Tip(_) => "tip",
    };
    format!("{cls}!")
}
pub fn count(k: Kind, n: i32) -> i32 {
    let counted = match k {
        Kind::Note => n + 1,
        Kind::Warn { .. } => 0,
        Kind::Tip(_) => n,
    };
    counted + 1
}
pub fn named(o: Option<Kind>) -> String {
    let name = match o {
        None => "none",
        Some(Kind::Note) => "note",
        Some(_) => "other",
    };
    format!("{name}.")
}
// Its \`None\` last, as std's \`Formatter::align\` is matched.
pub fn tagged(o: Option<Kind>) -> String {
    let tag = match o {
        Some(Kind::Note) => "note",
        Some(Kind::Warn { .. }) => "warn",
        Some(Kind::Tip(_)) => "tip",
        None => "none",
    };
    format!("{tag}.")
}
// Of a value MIR holds, \`tmp\`, its first branch doing more than giving
// it, as react.dev's CodeDiagram picks a child.
pub fn pick(x: i32) -> String {
    let shown: Box<dyn std::fmt::Display> = if x > 5 {
        let m = x * 2;
        Box::new(m + 1)
    } else if x < 0 {
        Box::new("neg")
    } else {
        Box::new(x)
    };
    shown.to_string()
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain('  const cls = k === "Note" ? "note" : k.TAG === "Warn" ? "warn" : "tip";\n');
  expect(js).toContain('  const name = o == null ? "none" : o === "Note" ? "note" : "other";\n');
  expect(js).toContain('  const tag = o == null ? "none" : o === "Note" ? "note" : o.TAG === "Warn" ? "warn" : "tip";\n');
  expect(js).toContain('  if (k === "Note") {\n    counted = (n + 1) | 0;\n  } else if (k.TAG === "Warn") {\n    counted = 0;\n  } else {\n    counted = n;\n  }\n');
  expect(js).toMatch(/} else if \(x < 0\) {\n    \w+ = /);
  const lib = await import(join(dir, "mir.js"));
  expect([lib.class("Note"), lib.class({ TAG: "Warn", level: 1 }), lib.class({ TAG: "Tip", _0: "x" })]).toEqual(["note!", "warn!", "tip!"]);
  expect([lib.pick(6), lib.pick(-1), lib.pick(2)]).toEqual(["13", "neg", "2"]);
  expect([lib.count("Note", 3), lib.count({ TAG: "Warn", level: 1 }, 3), lib.count({ TAG: "Tip", _0: "x" }, 3)]).toEqual([5, 1, 4]);
  expect([lib.named(undefined), lib.named("Note"), lib.named({ TAG: "Tip", _0: "x" })]).toEqual(["none.", "note.", "other."]);
  expect([lib.tagged(undefined), lib.tagged("Note"), lib.tagged({ TAG: "Warn", level: 1 }), lib.tagged({ TAG: "Tip", _0: "x" })]).toEqual(["none.", "note.", "warn.", "tip."]);
});
