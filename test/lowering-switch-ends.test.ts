// How a `switch`'s case ends (ADR 0629): a `break` where it goes on, the
// last case's and the `default`'s too (amended 2026-11-03, the user's
// decision), none after a case that leaves, a `try` each of whose arms
// returns among them; and which `match` from MIR is a `switch` at all.
// Each test is a program of its own, compiled and run.

import { beforeAll, expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { buildReact, compiler, fixture, run, target } from "./support";

beforeAll(buildReact, 600_000);

// React's describeElementType: `case REACT_VIEW_TRANSITION_TYPE: if
// (enableViewTransition) { return 'ViewTransition'; } }`, its last case
// falling out of the `switch`; and its lazy case, `try { return ..; }
// catch { return ''; }`, which never goes on to the next.
test("a case needs no break where it leaves, and the last that goes on has one", async () => {
  const dir = fixture("switch-ends");
  writeFileSync(join(dir, "lib.rs"), `use std::panic::catch_unwind;

pub const SUSPENSE: u32 = 1;
pub const VIEW: u32 = 2;
pub const LAZY: u32 = 3;
pub const MEMO: u32 = 4;
pub const ENABLE_VIEW: bool = true;

fn init(payload: u32) -> u32 {
    if payload == 0 {
        panic!("not loaded");
    }
    payload
}

pub fn described(kind: u32, payload: u32) -> String {
    match kind {
        SUSPENSE => return "Suspense".to_string(),
        VIEW if ENABLE_VIEW => {
            return "ViewTransition".to_string();
        }
        _ => {}
    }
    if payload > 100 {
        match kind {
            LAZY => {
                // Lazy may contain any component type so we recursively resolve it.
                return catch_unwind(|| described(init(payload - 100), 0)).unwrap_or_default();
            }
            MEMO => return "Memo".to_string(),
            _ => {}
        }
    }
    String::new()
}

pub fn counted(kind: u32, on: bool, log: &mut Vec<u32>) {
    match kind {
        SUSPENSE => log.push(1),
        VIEW if on => log.push(2),
        LAZY => log.push(3),
        _ => {}
    }
    log.push(0);
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  // The last case that goes on ends with `break;`, as upstream's do (ADR
  // 0629, amended 2026-11-03, the user's decision).
  expect(js).toContain('    case VIEW:\n      if (ENABLE_VIEW) {\n        return "ViewTransition";\n      }\n      break;\n  }\n');
  expect(js).toContain('        } catch {\n          return "";\n        }\n      case MEMO:\n');
  // A case before another ends in a `break`, and the last too.
  expect(js).toContain("      log.push(2);\n      }\n      break;\n    case LAZY:\n      log.push(3);\n      break;\n  }\n");
  const lib = await import(join(dir, "lib.js"));
  const runs = [1, 2, 3, 4].map((kind) => {
    const log: number[] = [];
    lib.counted(kind, true, log);
    return log;
  });
  expect([lib.described(2, 0), lib.described(3, 101), lib.described(3, 100), lib.described(4, 101), runs]).toEqual([
    "ViewTransition",
    "Suspense",
    "",
    "Memo",
    [[1, 0], [2, 0], [3, 0], [0]],
  ]);
});

// React's addValueToProperties (shared/ReactPerformanceTrackProperties.js):
// `let desc; if (..) { .. } else { switch (typeof value) { case
// "function": .. } } properties.push(..)`. A case's arm whose inner match
// gives `desc` jumps where the `switch` ends, nothing between: its own
// `break`, not a label around the `if` (ADR 0731).
test("a case's break past the switch is the case's own", async () => {
  const dir = fixture("switch-ends-past");
  writeFileSync(join(dir, "lib.rs"), `use js::{Kind, Unknown};

pub fn described(value: Option<&Unknown>, log: &mut Vec<String>) {
    let desc = if let Some(value) = value
        && let Kind::Object(_) | Kind::Array(_) = js::classify(Some(value))
    {
        "object".to_string()
    } else {
        match js::classify(value) {
            Kind::Null => "null".to_string(),
            Kind::Function(function) => match js::classify(js::get(js::unknown(function), "name")) {
                Kind::String(name) if !name.is_empty() => name.to_string() + "() {}",
                _ => "() => {}".to_string(),
            },
            Kind::Undefined => "undefined".to_string(),
            Kind::Bool(value) => if value { "true" } else { "false" }.to_string(),
            _ => js::string(value),
        }
    };
    log.push(desc);
}

// A match of three arms in a case, a switch's own: its block's break past
// it, the case's own.
pub fn numbered(value: Option<&Unknown>, log: &mut Vec<String>) {
    let desc = if let Some(value) = value
        && let Kind::Object(_) = js::classify(Some(value))
    {
        "object".to_string()
    } else {
        match js::classify(value) {
            Kind::Function(function) => match js::classify(js::get(js::unknown(function), "name")) {
                Kind::String(name) if !name.is_empty() => name.to_string() + "() {}",
                Kind::Number(_) => "numbered() {}".to_string(),
                _ => "() => {}".to_string(),
            },
            Kind::Undefined => "undefined".to_string(),
            _ => js::string(value),
        }
    };
    log.push(desc);
}

// A loop's in a case is the loop's: the block's own, still.
pub fn scanned(value: Option<&Unknown>, log: &mut Vec<String>) {
    let desc = 'desc: {
        match js::classify(value) {
            Kind::Null => "null".to_string(),
            Kind::String(text) => {
                for c in text.chars() {
                    if c == '!' {
                        break 'desc "bang".to_string();
                    }
                    log.push(c.to_string());
                }
                "plain".to_string()
            }
            Kind::Undefined => "undefined".to_string(),
            _ => js::string(value),
        }
    };
    log.push(desc);
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  // Its match of two arms, an `if` and its `else`, as upstream's (ADR 0745).
  expect(js).toContain('        if (typeof name === "string" && name !== "") {\n          desc = name + "() {}";\n        } else {\n          desc = "() => {}";\n        }\n        break;\n');
  expect(js.slice(0, js.indexOf("function scanned"))).not.toMatch(/\bbb\d+/);
  const lib = await import(join(dir, "lib.js"));
  const log: string[] = [];
  for (const value of [{}, null, function named() {}, () => {}, undefined, true, 5]) lib.described(value, log);
  expect(log).toEqual(["object", "null", "named() {}", "() => {}", "undefined", "true", "5"]);
  const numbered: string[] = [];
  const counted = Object.defineProperty(function () {}, "name", { value: 3 });
  for (const value of [function named() {}, counted, () => {}, undefined, 5]) lib.numbered(value, numbered);
  expect(numbered).toEqual(["named() {}", "numbered() {}", "() => {}", "undefined", "5"]);
  const scanned: string[] = [];
  for (const value of ["a!b", "ab", null]) lib.scanned(value, scanned);
  expect(scanned).toEqual(["a", "bang", "a", "b", "plain", "null"]);
});

// React's scheduler's unstable_runWithPriority: `default: priorityLevel =
// NormalPriority; }`. Of a guarded case that falls through (ADR 0905), tidy
// ends each case with a `break;`, the `default`'s too: one `break;`, the
// printer's, as each clause that goes on ends with (ADR 0629, amended
// 2026-11-03, the user's decision), as upstream mostly writes it.
test("a default after a guarded case ends with one break", async () => {
  const dir = fixture("switch-ends-guarded");
  writeFileSync(join(dir, "lib.rs"), `pub fn guarded(level: u32, strict: bool) -> u32 {
    let level = match level {
        1 if strict => level + 100,
        1 | 2 | 3 => level,
        _ => 0,
    };
    level + 1
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("    case 3:\n      break;\n    default:\n      level = 0;\n      break;\n  }\n");
  const lib = await import(join(dir, "lib.js"));
  expect([lib.guarded(1, true), lib.guarded(1, false), lib.guarded(3, true), lib.guarded(9, true)]).toEqual([102, 2, 4, 1]);
});

// #389, the user's decision: no JS label where the Rust wrote none. A
// `match` of constants an arm of which breaks the loop around it is a
// `switch` where the loop's turn ends at it, each `break` the `switch`'s and
// one `break;` after it the loop's, each case that goes on `continue;`, a
// value of none of them too; and an `if` chain where the turn goes on past
// it. A loop the program labeled keeps its `break rows;`.
test("a match whose arm breaks its loop has no label the program didn't write", async () => {
  const dir = fixture("switch-loop-breaks");
  writeFileSync(join(dir, "lib.rs"), `pub fn first_stop(codes: &[u8]) -> usize {
    let mut i = 0;
    while i < codes.len() {
        match codes[i] {
            b'.' | b'!' => break,
            b' ' => {}
            _ => i += 1,
        }
        i += 1;
    }
    i
}

pub fn framed(codes: &[u8]) -> u32 {
    let mut i = 0;
    let mut n = 0;
    while i < codes.len() {
        i += 1;
        match codes[i - 1] {
            b'.' | b'!' => break,
            b' ' => n += 2,
            b'-' => n += 1,
            _ => {}
        }
    }
    n
}

pub fn labeled(rows: &[Vec<u8>]) -> u32 {
    let mut n = 0;
    'rows: for row in rows {
        for &b in row {
            match b {
                0 => break 'rows,
                1 => n += 1,
                _ => n += 10,
            }
            n += 100;
        }
    }
    n
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).not.toContain("loop:");
  expect(js).toContain("    if (codes[i] === 46 || codes[i] === 33) {\n      break;\n    }\n");
  expect(js).toContain(
    "      case 46:\n      case 33:\n        break;\n      case 32:\n        n = (n + 2) >>> 0;\n        continue;\n      case 45:\n        n = (n + 1) >>> 0;\n        continue;\n      default:\n        continue;\n    }\n    break;\n",
  );
  expect(js).toContain("        case 0:\n          break rows;\n");
  const lib = await import(join(dir, "lib.js"));
  const bytes = (s: string) => [...s].map((c) => c.charCodeAt(0));
  expect([lib.firstStop(bytes(" .")), lib.firstStop(bytes(".")), lib.firstStop(bytes("a"))]).toEqual([1, 0, 2]);
  expect([lib.framed(bytes("a -.--")), lib.framed(bytes(" x-"))]).toEqual([3, 3]);
  expect(lib.labeled([[1, 2], [0, 1]])).toEqual(1 + 100 + 10 + 100);
});

// #389b: guarded arms that fall through (ADR 0905), in a loop whose turn
// goes on past the `match`, whose `switch` only a label leaves: the `if`
// chain they are, each arm's test of its values and those that fall to
// it, and its guard, as Rust tries them, no label.
test("guarded arms that fall through and break their loop are an if chain", async () => {
  const dir = fixture("switch-guarded-loop-breaks");
  writeFileSync(join(dir, "lib.rs"), `pub fn guarded(tags: &[u32], limit: u32) -> u32 {
    let mut total = 0;
    for &tag in tags {
        match tag {
            1 => total += 1,
            2 | 3 if total > limit => break,
            2 | 3 | 4 => total += 300,
            _ => {}
        }
        total += 1;
    }
    total
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).not.toContain("loop:");
  expect(js).toContain(
    "    if (tag === 1) {\n      total = (total + 1) >>> 0;\n    } else if ((tag === 2 || tag === 3) && total > limit) {\n      break;\n    } else if (tag === 2 || tag === 3 || tag === 4) {\n",
  );
  const lib = await import(join(dir, "lib.js"));
  expect([lib.guarded([1, 2, 4, 3], 0), lib.guarded([1, 2, 4, 3], 1000), lib.guarded([5, 2], 0)]).toEqual([2, 905, 1]);
});

// #389c: of a subject that does something, `match c.next_code()`, the `if`
// chain reads it held in one `const`, named as what it holds, called once
// a turn as the `switch` called it, no label.
test("guarded arms of a subject that runs are an if chain of it held once", async () => {
  const dir = fixture("switch-guarded-loop-held");
  writeFileSync(join(dir, "lib.rs"), `pub struct Codes {
    at: usize,
    codes: Vec<u32>,
}
impl Codes {
    pub fn next_code(&mut self) -> u32 {
        self.at += 1;
        self.codes.get(self.at - 1).copied().unwrap_or(0)
    }
}
pub fn count(codes: Vec<u32>, limit: u32) -> u32 {
    let mut c = Codes { at: 0, codes };
    let mut n = 0;
    loop {
        n += 1;
        match c.next_code() {
            0 => break,
            1 if n > limit => break,
            1 | 2 => n += 20,
            _ => n += 300,
        }
        n += 1;
    }
    n
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).not.toContain("loop:");
  expect(js).toContain(
    "    const nextCode = Codes.nextCode(c);\n    if (nextCode === 0) {\n      break;\n    }\n    if (nextCode === 1 && n > limit) {\n      break;\n    }\n    if (nextCode === 1 || nextCode === 2) {\n",
  );
  const lib = await import(join(dir, "lib.js"));
  expect([lib.count([1, 2, 5, 1], 1000), lib.count([1, 2, 1], 30)]).toEqual([369, 45]);
});

// #389c: of those, an arm of no statements, `1 | 2 => {}`: the next
// link's test, `t !== 1 && t !== 2`, not an empty `then`.
test("an empty arm of guarded arms that fall through is the next link's test", async () => {
  const dir = fixture("switch-guarded-loop-empty");
  writeFileSync(join(dir, "lib.rs"), `pub fn skips(tags: &[u32], g: bool) -> u32 {
    let mut n = 0;
    for &t in tags {
        match t {
            0 => break,
            1 if g => break,
            1 | 2 => {}
            _ => n += 10,
        }
        n += 1;
    }
    n
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).not.toContain("loop:");
  expect(js).not.toContain("{\n    }");
  expect(js).toContain("    if (t !== 1 && t !== 2) {\n      n = (n + 10) >>> 0;\n    }\n");
  const lib = await import(join(dir, "lib.js"));
  expect([lib.skips([1, 2, 5, 0, 5], false), lib.skips([2, 1, 5], true)]).toEqual([13, 1]);
});

// rd379: of a \`match\`'s arm, a \`switch\`'s case whose end leaves it, the
// closure called last is its body, its \`return;\` that case's \`break;\`,
// the function's where the \`switch\` ends it (rd378). Its thread-local is
// then a variable, not a box a closure reads.
test("a thread-local's with that ends a switch's case is its body", async () => {
  const dir = fixture("applied-case");
  writeFileSync(join(dir, "lib.rs"), `use std::cell::Cell;

thread_local! {
    static SEEN: Cell<u32> = const { Cell::new(0) };
}

pub fn seen() -> u32 {
    SEEN.get()
}

pub fn mark(n: u32) -> u32 {
    match n {
        0 => SEEN.set(5),
        1 => SEEN.with(|seen| {
            if seen.get() == 0 {
                return;
            }
            seen.set(seen.get() * 2);
        }),
        2 => SEEN.with(|seen| {
            for i in 0..n {
                if seen.get() > 100 {
                    return;
                }
                seen.set(seen.get() + i);
            }
        }),
        _ => SEEN.set(1),
    }
    SEEN.get() + 1
}

pub fn last(n: u32) {
    match n {
        0 => SEEN.set(5),
        1 => SEEN.with(|seen| {
            if seen.get() == 0 {
                return;
            }
            seen.set(7);
        }),
        _ => SEEN.set(1),
    }
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain(`    case 1:
      if (SEEN === 0) {
        break;
      }
      SEEN = (SEEN * 2) >>> 0;
      break;
    case 2:
      for (let i = 0; i < n; i++) {
        if (SEEN > 100) {
          break;
        }
        SEEN = (SEEN + i) >>> 0;
      }
      break;`);
  expect(js).toContain(`    case 1:
      if (SEEN === 0) {
        return;
      }
      SEEN = 7;
      break;`);
  expect(js).not.toContain("=> {");
  expect(js).toContain("let SEEN = 0;");
  const lib = await import(join(dir, "lib.js"));
  expect([lib.mark(1), lib.mark(0), lib.mark(1), lib.mark(2), lib.last(1), lib.seen(), lib.mark(7)]).toEqual([1, 6, 11, 12, undefined, 7, 2]);
});

test("a `match` of an `Option`'s strings from MIR is the `switch` or `else if` chain a person writes", async () => {
  const dir = fixture("mir-option-string-match");
  writeFileSync(join(dir, "lib.rs"), `pub fn flagged(flag: Option<&str>, mut arg: String) -> String {
    match flag {
        Some("s") => arg += "",
        Some("d") | Some("i") => arg = arg.trim().to_string(),
        Some("f") => arg = arg.to_uppercase(),
        _ => {}
    }
    arg
}
pub fn named(flag: Option<&str>, v: &mut Vec<u32>) {
    match flag {
        Some("s") => v.push(1),
        Some("d") => v.push(2),
        _ => v.push(3),
    }
}
pub fn nested(flag: Option<&str>, v: &mut Vec<u32>) {
    if flag.is_some() {
        if flag == Some("s") {
            v.push(1);
        } else {
            v.push(2);
        }
    }
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  // Of three cases or more, a `switch`, a `Some`'s constants its cases
  // (ADRs 0629, 0634).
  expect(js).toContain(`export function flagged(flag, arg) {
  switch (flag) {
    case "s":
      arg += "";
      break;
    case "d":
    case "i":
      arg = $trim(arg);
      break;
    case "f":
      arg = arg.toUpperCase();
      break;
  }
  return arg;
}`);
  expect(js).toContain(`export function named(flag, v) {
  switch (flag) {
    case "s":
      v.push(1);
      break;
    case "d":
      v.push(2);
      break;
    default:
      v.push(3);
      break;
  }
}`);
  expect(js).toContain(`export function nested(flag, v) {
  if (flag != null) {
    if (flag === "s") {
      v.push(1);
    } else {
      v.push(2);
    }
  }
}`);
  const lib = await import(join(dir, "mir.js"));
  expect([lib.flagged("d", " x "), lib.flagged("f", "x"), lib.flagged(undefined, "y"), lib.flagged("q", "z")]).toEqual(["x", "X", "y", "z"]);
  // A guard its chain's last `else` doesn't say stays.
  const pushed: number[] = [];
  lib.nested(undefined, pushed);
  lib.nested("q", pushed);
  lib.named(undefined, pushed);
  lib.named("d", pushed);
  expect(pushed).toEqual([2, 3, 2]);
});
