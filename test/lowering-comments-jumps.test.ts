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
  expect(js).toContain("  for (let i = 0; i < xs.length; i++) {\n    const x = xs[i];\n    out.push(x);\n    // On to the next.\n  }\n");
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

pub fn walked(xs: &[i32], out: &mut Vec<i32>) {
    let mut i = 0;
    loop {
        match xs.get(i) {
            Some(0) => break,
            Some(&x) => out.push(x * 2),
            None => {
                // At the end.
                break;
            }
        }
        i += 1;
    }
}
`);
  expect(js).toContain("    if (xs[i] === 0) {\n      // Zero: no more.\n");
  // Of a `loop` the arm's `break` leaves at its end, MIR's `return;`.
  expect(js).toContain("    if (x == null) {\n      // At the end.\n      return;\n    }\n");
  const out: number[] = [];
  lib.doubled([1, 2, 0, 3] as never, out as never);
  expect(out).toEqual([2, 4]);
  const walked: number[] = [];
  lib.walked([1, 2, 3] as never, walked as never);
  lib.walked([4, 0, 5] as never, walked as never);
  expect(walked).toEqual([2, 4, 6, 8]);
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
  expect(js).toContain(`    // falls through
    case "keydown":
    // A comment between patterns.
    case "keyup":
      kind = 1;
      break;
`);
  expect(js).toContain(`    default:
      // Unknown event. Nothing to do.
      break;
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
const E: u8 = 4;
const F: u8 = 5;

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
        // Above an arm of a value.
        E // Between its patterns.
        | F => log.push(3),
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
  // Of an arm of a value whose patterns have a comment between them, each
  // case where its value is (rd139).
  expect(js).toContain("      break;\n    // Above an arm of a value.\n    case E: // Between its patterns.\n    case F:\n      log.push(3);\n");
  expect(js).toContain('      return true;\n\n    case "rp":\n    case "rt":\n');
  const log: number[] = [];
  for (const tag of [0, 3, 5, 7]) lib.effects(tag as never, log as never);
  expect([log, lib.valid("rt" as never, undefined as never), lib.valid("y" as never, undefined as never)]).toEqual([[1, 2, 3], true, false]);
});

// React's Flight client's processFullStringRow: an arm whose pattern has a
// block comment, `114 /* "r" */ => {`, still has the `// Fallthrough`
// written above it before its `case`, not stacked above the `switch`; so
// does a last `_ /* .. */` arm whose first statement is an `if` whose test
// starts it, `if __DEV__ && row.is_empty() {`, before its `default` (rd525).
test("a comment above an arm whose pattern has a comment is before its case", async () => {
  const { js, lib } = await compiled("comment-above-commented-arm", `unsafe extern "C" {
    safe static __DEV__: bool;
}

fn start(log: &mut Vec<String>, kind: Option<&str>) {
    log.push(format!("start {kind:?}"));
}

pub fn process(tag: u8, row: &str) -> Vec<String> {
    let mut log = Vec::new();
    match tag {
        82 /* "R" */ => {
            start(&mut log, None);
        }
        // Fallthrough
        114 /* "r" */ => {
            start(&mut log, Some("bytes"));
        }
        // Fallthrough
        67 /* "C" */ => {
            log.push("stop".to_string());
        }
        // Fallthrough
        125 /* "}" */ => {
            log.push("close".to_string());
        }
        // Fallthrough
        _ /* """ "{" "[" "0" - "9" */ => {
            if __DEV__ && row.is_empty() {
                log.push("halt".to_string());
                return log;
            }
            // We assume anything else is JSON.
            log.push(row.to_string());
        }
    }
    log
}
`);
  expect(js).toContain("  switch (tag) {\n    case 82 /* \"R\" */:");
  expect(js).toContain("    // Fallthrough\n    case 114 /* \"r\" */:");
  expect(js).toContain("    // Fallthrough\n    case 67 /* \"C\" */:");
  expect(js).toContain("    // Fallthrough\n    case 125 /* \"}\" */:");
  // The `_` arm's own pattern comment after its `default:`, as upstream's
  // `default: /* """ "{" .. */ {` (rd525).
  expect(js).toContain('    // Fallthrough\n    default: /* """ "{" "[" "0" - "9" */\n      if (__DEV__ && row === "") {');
  (globalThis as { __DEV__?: boolean }).__DEV__ = true;
  const process = lib.process as (tag: number, row: string) => string[];
  expect([process(82, ""), process(114, ""), process(67, ""), process(125, ""), process(48, ""), process(48, "[1]")]).toEqual([
    ["start None"],
    ['start Some("bytes")'],
    ["stop"],
    ["close"],
    ["halt"],
    ["[1]"],
  ]);
});

// React's cutOffTailIfNeeded: a comment above a `_ =>` arm whose block
// starts with a `let`, whose statement's span is its value's, `let mut
// tail_node = render_state.tail..`, is before its `default: {`, as upstream's
// `// Hidden is now the default.` is, not above the `switch` (rd532); and a
// `_` arm's pattern comment is after `default:`, before its `{` (rd525).
test("a comment above a `_` arm that opens with a `let` is before its default", async () => {
  const { js, lib } = await compiled("comment-above-let-default", `#[derive(Clone, Copy)]
pub enum Mode {
    Visible,
    Collapsed,
    Hidden,
}

fn mode(n: u32) -> Option<Mode> {
    match n {
        0 => Some(Mode::Visible),
        1 => Some(Mode::Collapsed),
        2 => Some(Mode::Hidden),
        _ => None,
    }
}

pub fn cut(n: u32, log: &mut Vec<u32>) {
    match mode(n) {
        Some(Mode::Visible) => {
            // Everything should remain as it was.
        }
        Some(Mode::Collapsed) => {
            log.push(1);
        }
        // Hidden is now the default.
        _ => {
            // Any insertions at the end of the tail list after this point
            // should be invisible.
            let mut left = log.len();
            while left > 3 {
                left -= 1;
            }
            log.push(left as u32);
        }
    }
}

pub fn tagged(tag: u8) -> u32 {
    match tag {
        73 /* "I" */ => 1,
        72 /* "H" */ => 2,
        _ /* "{" "[" */ => {
            let doubled = u32::from(tag) * 2;
            doubled + 1
        }
    }
}
`);
  expect(js).toContain("      break;\n    // Hidden is now the default.\n    default: {\n      // Any insertions");
  expect(js).not.toMatch(/\/\/ Hidden is now the default\.\n\s+switch/);
  expect(js).toContain('    default: /* "{" "[" */ {\n      const doubled');
  const cut = lib.cut as (n: number, log: number[]) => void;
  const log: number[] = [5, 6, 7, 8, 9];
  for (const n of [0, 1, 2, 3]) cut(n, log);
  expect(log).toEqual([5, 6, 7, 8, 9, 1, 3, 3]);
  expect((lib.tagged as (tag: number) => number)(73)).toBe(1);
  expect((lib.tagged as (tag: number) => number)(91)).toBe(183);
});

// React's Flight client's parseModelString: `if .. && let Ok(function) =
// js::eval(&code) { // eslint-disable-next-line no-eval\n return function;
// }` is `try { return (0, eval)(code); } catch {}`, the comment above the
// `return` it's written above, so ESLint's next line is the eval's, as
// upstream has it (rd525).
test("a comment above an Ok arm's return is above the call's return in the try", async () => {
  const { js, lib } = await compiled("comment-above-tried-return", `use js::Unknown;

pub fn parse(code: &str) -> Option<&'static Unknown> {
    if !code.is_empty()
        && let Ok(function) = js::eval(code)
    {
        // eslint-disable-next-line no-eval
        return function;
    }
    // Fallthrough to fallback case.
    None
}
`);
  expect(js).toContain("    try {\n      // eslint-disable-next-line no-eval\n      return (0, eval)(code);\n    } catch {}\n");
  const parse = lib.parse as (code: string) => unknown;
  expect([parse("1 + 2"), parse("("), parse("")]).toEqual([3, undefined, undefined]);
});

// React's Webpack bundler config's addChunkDebugInfo: `let href = match
// catch_unwind(|| { // $FlowFixMe[incompatible-type]\n url(..).href() })
// { Ok(h) => h, Err(_) => .. }` is `let href; try { // $FlowFixMe..\n href
// = ..; } catch { .. }`, the comment inside the `try` above the value it's
// written above, as upstream has it, not above `let href;` (rd545).
test("a comment in a catch_unwind closure is above its value in the try", async () => {
  const { js, lib } = await compiled("comment-in-tried-closure", `use std::panic::catch_unwind;

fn parse(s: &str) -> String {
    if s.is_empty() {
        panic!("empty");
    }
    s.to_uppercase()
}

pub fn href(s: &str, fallback: &str) -> String {
    let href = match catch_unwind(|| {
        // $FlowFixMe[incompatible-type]
        parse(s)
    }) {
        Ok(h) => h,
        Err(_) => fallback.to_string(),
    };
    href + "!"
}
`);
  expect(js).toContain("  let href;\n  try {\n    // $FlowFixMe[incompatible-type]\n    href = parse(s);\n  } catch {\n");
  const href = lib.href as (s: string, fallback: string) => unknown;
  expect([href("a", "f"), href("", "f")]).toEqual(["A!", "f!"]);
});

// React's Flight client's `then`: a comment above `let settled = match &mut
// *chunk.state.borrow_mut() { .. }`, whose JS's first statement printed is
// the `if` of its first arm, is above that `if`, as upstream's `// The
// status might have changed after initialization.` is above its `switch`;
// not before the binding of a later arm, which the scrutinee's span is,
// nor lost where that binding isn't printed (rd533).
test("a comment above a `let` of a `match` is above the chain it makes", async () => {
  const { js, lib } = await compiled("comment-above-let-match", `use std::cell::RefCell;

pub enum State {
    Done { value: u32, extra: u32 },
    Pending { value: Option<Vec<u32>>, reason: Option<Vec<u32>> },
    Blocked { value: Option<Vec<u32>>, reason: Option<Vec<u32>> },
    Failed { reason: u32 },
}

pub struct Chunk {
    pub state: RefCell<State>,
}

pub fn make(n: u32) -> Chunk {
    let state = match n {
        0 => State::Done { value: 7, extra: 0 },
        1 => State::Pending { value: None, reason: None },
        2 => State::Blocked { value: Some(vec![9]), reason: None },
        _ => State::Failed { reason: n },
    };
    Chunk { state: RefCell::new(state) }
}

pub fn settle(chunk: &Chunk, out: &mut Vec<u32>) {
    out.push(0);
    // The status might have changed after initialization.
    let settled = match &mut *chunk.state.borrow_mut() {
        State::Done { value, .. } => Ok(*value),
        State::Pending { value, reason } | State::Blocked { value, reason } => {
            value.get_or_insert_with(Vec::new).push(1);
            reason.get_or_insert_with(Vec::new).push(2);
            out.push(value.as_ref().map_or(0, |v| v.len() as u32));
            return;
        }
        State::Failed { reason } => Err(*reason),
    };
    match settled {
        Ok(value) => out.push(value),
        Err(reason) => out.push(reason + 100),
    }
}
`);
  expect(js).toMatch(/out\.push\(0\);\n {2}\/\/ The status might have changed after initialization\.\n {2}if \(/);
  const settle = lib.settle as (chunk: unknown, out: number[]) => void;
  const make = lib.make as (n: number) => unknown;
  const out: number[] = [];
  for (const n of [0, 1, 2, 5]) settle(make(n), out);
  expect(out).toEqual([0, 7, 0, 1, 0, 2, 0, 105]);
});

// React's Flight client's `let value = match js::eval(code) { //
// eslint-disable-next-line no-eval\n Ok(v) => v, .. }`, and a `catch_unwind`'s
// `Ok` arm of the same: a comment written above the `Ok` arm is above the
// statement in the `try` that holds the arm's value, which maps to the call
// it's made of; one above the `Err` arm stays in the `catch` (rd630).
test("a comment above a match's Ok arm is above its value in the try", async () => {
  const { js, lib } = await compiled("comment-above-tried-arm", `use std::panic::catch_unwind;

fn parse(s: &str) -> String {
    if s.is_empty() {
        panic!("empty");
    }
    s.to_uppercase()
}

pub fn href(s: &str, fallback: &str) -> String {
    let href = match catch_unwind(|| parse(s)) {
        // The parsed value.
        Ok(h) => h,
        // The fallback.
        Err(_) => fallback.to_string(),
    };
    href + "!"
}

pub fn evaluated(code: &str) -> bool {
    let value = match js::eval(code) {
        // eslint-disable-next-line no-eval
        Ok(v) => v,
        Err(_) => None,
    };
    value.is_some()
}
`);
  expect(js).toContain(
    "  let href;\n  try {\n    // The parsed value.\n    href = parse(s);\n  } catch {\n    // The fallback.\n    href = fallback;\n  }\n",
  );
  expect(js).toContain(
    "  let value;\n  try {\n    // eslint-disable-next-line no-eval\n    value = (0, eval)(code);\n  } catch {\n    value = undefined;\n  }\n",
  );
  const href = lib.href as (s: string, fallback: string) => unknown;
  const evaluated = lib.evaluated as (code: string) => unknown;
  expect([href("a", "f"), href("", "f"), evaluated("1 + 2"), evaluated("(")]).toEqual(["A!", "f!", true, false]);
});

// A comment above a later arm, `// c\n None => given`, of a `match` read as
// `n ?? given`: the arm's code is between it and the operand, so it's not
// the operand's, `n ?? // c\n given`, but the statement's (ADR 0444,
// amended); where the arm is the first, the whole arm before says so too.
test("a comment above a later arm read as an operand is above its statement", async () => {
  const { js, lib } = await compiled("comment-above-operand-arm", `use std::cell::Cell;

pub fn kept_else(n: Option<u32>, given: Option<u32>) -> Option<u32> {
    let current = Cell::new(None);
    current.set(match n {
        Some(n) => Some(n),
        // Else the one given.
        None => given,
    });
    current.get()
}
`);
  expect(js).toContain("  // Else the one given.\n  let current = n ?? given;\n");
  const kept = lib.keptElse as (n?: number, given?: number) => unknown;
  expect([kept(1, 2), kept(undefined, 2)]).toEqual([1, 2]);
});

// A comment above a `_` arm whose pattern's comment holds brackets, `_ /*
// "{" "}" */ => {`: the code between it and the `default`'s start is read
// as if that comment weren't there, so it's before the `default`, not lost
// or above the `switch` (rd525). The pattern's own was after the arm's
// first statement, `log.push(row); /* "{" "}" */` (d1).
test("a comment above a _ arm whose pattern's comment has brackets is before its default", async () => {
  const { js, lib } = await compiled("comment-above-bracketed-default", `pub fn process(tag: u8, row: &str) -> Vec<String> {
    let mut log = Vec::new();
    match tag {
        82 /* "R" */ => {
            log.push("r".to_string());
        }
        67 /* "C" */ => {
            log.push("stop".to_string());
        }
        125 /* "}" */ => {
            log.push("close".to_string());
        }
        // We assume anything else is JSON.
        _ /* "{" "}" */ => {
            log.push(row.to_string());
        }
    }
    log
}

pub fn counted(tags: &[u8]) -> usize {
    let mut n = 0;
    for &tag in tags {
        match tag {
            1 => n += 1,
            2 => n += 2,
            _ /* "{" */ => continue,
        }
        n += 10;
    }
    n
}
`);
  // And the pattern's comment after its `default:`, the arm's first
  // statement, an expression, on the next line, not after it (d1); a
  // `default:` of one statement that leaves, `continue;`, too, which
  // oxc writes on its line, not after the statement.
  expect(js).toContain('      break;\n    // We assume anything else is JSON.\n    default: /* "{" "}" */\n      log.push(row);\n');
  expect(js).toContain('      default: /* "{" */\n        continue;\n');
  const process = lib.process as (tag: number, row: string) => string[];
  expect([process(125, "x"), process(1, "{}")]).toEqual([["close"], ["{}"]]);
  expect((lib.counted as (tags: number[]) => number)([1, 3, 2])).toBe(23);
});

// React's ReactFiberCommitWork: a guarded arm that falls to a last `_`
// that opens a block, `_ => {`, the program's own comment above it,
// `// Fallthrough`, `// Intentional fallthrough to next branch`: that
// comment above `default:`, and no `// falls through` of rust-js's, where
// the arm ends in the case's `break` and where it returns (rd638).
test("a program's fall-through comment above a block's `_` is kept, none of rust-js's beside it", async () => {
  const { js, lib } = await compiled("fallthrough-to-a-block-default", `const VIEW: u32 = 30;
const HOST: u32 = 5;

pub fn commit(tag: u32, enabled: bool, log: &mut Vec<u32>) {
    match tag {
        HOST => log.push(5),
        VIEW if enabled => {
            if log.is_empty() {
                log.push(30);
            }
        }
        // Intentional fallthrough to next branch
        _ => {
            log.push(0);
        }
    }
}

pub fn returned(tag: u32, enabled: bool, log: &mut Vec<u32>) {
    match tag {
        HOST => log.push(5),
        VIEW if enabled => {
            log.push(30);
            return;
        }
        // Fallthrough
        _ => {
            log.push(0);
        }
    }
}
`);
  expect(js).toContain(`        break;
      }
    // Intentional fallthrough to next branch
    default:
      log.push(0);`);
  expect(js).toContain(`        return;
      }
    // Fallthrough
    default:
      log.push(0);`);
  expect(js).not.toContain("// falls through");
  const log: number[] = [];
  const run = lib as unknown as Record<string, (tag: number, enabled: boolean, log: number[]) => void>;
  run.commit(30, true, log);
  run.commit(30, false, log);
  run.returned(30, false, log);
  run.returned(5, true, log);
  expect(log).toEqual([30, 0, 0, 5]);
});

// React's ReactFizzNewContext port writes `// Not equal, so not both the
// root.` above `(None, None) => {}`, its last arm. MIR has no statement
// of an arm that does nothing, and the comment was lost (#508). It's the
// arm's, as one written in its `{}` is (ADR 0753): `} else { // Fine. }`,
// a `switch`'s `case` above which it's written, and `default: // ..`. A
// first arm's is above the `if` as before, and a guarded one's above its
// `case`.
test("a comment above an arm that does nothing is the arm's", async () => {
  const { js, lib } = await compiled("comment-above-empty-arm", `pub fn err_first(r: Result<(), u32>, log: &mut Vec<u32>) {
    match r {
        // What failed.
        Err(_) => log.push(3),
        // Fine.
        Ok(()) => {}
    }
    log.push(2);
}

pub fn ok_first(r: Result<(), u32>, log: &mut Vec<u32>) {
    match r {
        // Fine.
        Ok(()) => {}
        // What failed.
        Err(_) => log.push(3),
    }
    log.push(2);
}

pub fn three(n: u8, log: &mut Vec<u32>) {
    match n {
        0 => log.push(0),
        // One is fine.
        1 => {}
        _ => log.push(9),
    }
}

pub fn defaulted(n: u8, log: &mut Vec<u32>) {
    match n {
        0 => log.push(0),
        1 => log.push(1),
        // Others are fine.
        _ => {}
    }
}

pub fn none_first(o: Option<u32>, log: &mut Vec<u32>) {
    match o {
        // Nothing yet.
        None => {}
        Some(n) => log.push(n),
    }
}
`);
  expect(js).toContain(`  if (r.TAG === "Err") {
    // What failed.
    log.push(3);
  } else {
    // Fine.
  }
  log.push(2);`);
  expect(js).toContain(`  // Fine.
  if (r.TAG !== "Ok") {
    // What failed.
    log.push(3);
  }`);
  expect(js).toContain(`      break;
    // One is fine.
    case 1:
      break;
    default:`);
  expect(js).toContain(`    default:
      // Others are fine.
      break;
  }`);
  expect(js).toContain(`  // Nothing yet.
  if (o != null) {
    log.push(o);
  }
}`);
  const log: number[] = [];
  lib.errFirst({ TAG: "Err", _0: 1 } as never, log as never);
  lib.errFirst({ TAG: "Ok" } as never, log as never);
  lib.okFirst({ TAG: "Err", _0: 1 } as never, log as never);
  lib.three(1 as never, log as never);
  lib.three(2 as never, log as never);
  lib.defaulted(5 as never, log as never);
  lib.defaulted(1 as never, log as never);
  lib.noneFirst(undefined as never, log as never);
  lib.noneFirst(6 as never, log as never);
  expect(log).toEqual([3, 2, 2, 3, 2, 9, 1, 6]);
});

// In a loop, a `match` of `catch_unwind` whose first arm, `Ok(()) => {}`,
// does nothing put the comment above it in the `catch`, before the `Err`
// arm's code: MIR tested the `Err`, and a comment above MIR's `if` was
// taken for the arm it names (#509). It's the first arm's, where that arm
// is done, at the `try`'s end; and one in its `{}` no longer keeps the
// closure an arrow called in place, `(() => { .. })();`.
test("a comment above a loop's Ok arm of nothing is at its try's end", async () => {
  const { js, lib } = await compiled("comment-above-loop-ok-arm", `use std::panic::{AssertUnwindSafe, catch_unwind};

pub fn turns(xs: &[u32], log: &mut Vec<u32>) {
    for &x in xs {
        match catch_unwind(AssertUnwindSafe(|| {
            if x == 1 {
                return;
            }
            log.push(x);
        })) {
            // Next.
            Ok(()) => {}
            Err(_) => log.push(9),
        }
    }
}

pub fn inside(xs: &[u32], log: &mut Vec<u32>) {
    for &x in xs {
        match catch_unwind(AssertUnwindSafe(|| {
            if x == 1 {
                return;
            }
            log.push(x);
        })) {
            Ok(()) => {
                // Next.
            }
            Err(_) => log.push(9),
        }
    }
}
`);
  const tried = `    try {
      if (x === 1) {
        continue;
      }
      log.push(x);
      // Next.
    } catch {
      log.push(9);
    }`;
  expect(js).toContain(`export function turns(xs, log) {
  for (let i = 0; i < xs.length; i++) {
    const x = xs[i];
${tried}`);
  expect(js).toContain(`export function inside(xs, log) {
  for (let i = 0; i < xs.length; i++) {
    const x = xs[i];
${tried}`);
  const log: number[] = [];
  lib.turns([2, 1, 3] as never, log as never);
  lib.inside([1, 4] as never, log as never);
  expect(log).toEqual([2, 3, 4]);
});
