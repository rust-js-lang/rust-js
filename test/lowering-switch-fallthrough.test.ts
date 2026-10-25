// A `match` of constants that repeats a constant under guards, each
// guarded arm's constants among the next arm's, is a `switch` whose guarded
// cases fall through to the next (ADR 0905): React's
// `getComponentNameFromFiber`. Each test is a program of its own, compiled
// and run, against what upstream's JS does.

import { beforeAll, expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { buildReact, compiler, fixture, run, target } from "./support";

beforeAll(buildReact, 600_000);

const withJs = () => ["--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];

test("a guarded arm whose constants the next arm names falls through to its case", async () => {
  const dir = fixture("switch-fallthrough");
  writeFileSync(
    join(dir, "lib.rs"),
    `pub const ACTIVITY_COMPONENT: u32 = 1;
pub const VIEW_TRANSITION_COMPONENT: u32 = 2;
pub const INCOMPLETE_CLASS_COMPONENT: u32 = 3;
pub const INCOMPLETE_FUNCTION_COMPONENT: u32 = 4;
pub const CLASS_COMPONENT: u32 = 5;
pub const FUNCTION_COMPONENT: u32 = 6;
pub const LEGACY_HIDDEN_COMPONENT: u32 = 7;

#[cfg_attr(rust_js, rust_js::nullable)]
pub fn name(tag: u32, ty: &str, enable_view_transition: bool, disable_legacy_mode: bool) -> Option<String> {
    match tag {
        ACTIVITY_COMPONENT => return Some("Activity".to_string()),
        VIEW_TRANSITION_COMPONENT if enable_view_transition => {
            return Some("ViewTransition".to_string());
        }
        // The display name for these tags come from the user-provided type:
        VIEW_TRANSITION_COMPONENT | INCOMPLETE_CLASS_COMPONENT | INCOMPLETE_FUNCTION_COMPONENT
            if disable_legacy_mode => {}
        // A function's name, or the tag's.
        VIEW_TRANSITION_COMPONENT
        | INCOMPLETE_CLASS_COMPONENT
        | INCOMPLETE_FUNCTION_COMPONENT
        | CLASS_COMPONENT
        | FUNCTION_COMPONENT => {
            if ty.starts_with('<') {
                return Some(ty.to_string());
            }
        }
        LEGACY_HIDDEN_COMPONENT => {
            if enable_view_transition {
                return Some("LegacyHidden".to_string());
            }
        }
        _ => {}
    }
    None
}

/// What follows the \`match\` is more than its value: the arm the guarded
/// arms fall to is its case still, no block MIR made around it.
pub fn counted(tag: u32, on: bool, log: &mut Vec<u32>) {
    // Each tag's, a comment before the match, not an arm.
    match tag {
        ACTIVITY_COMPONENT => log.push(1),
        VIEW_TRANSITION_COMPONENT if on => log.push(2),
        // A long log stops here.
        VIEW_TRANSITION_COMPONENT | INCOMPLETE_CLASS_COMPONENT if log.len() > 2 => {}
        VIEW_TRANSITION_COMPONENT | INCOMPLETE_CLASS_COMPONENT | CLASS_COMPONENT => {
            log.push(3);
            log.push(4);
        }
        _ => {}
    }
    log.push(0);
}

/// A comment before a one-line arm fallen to is before its case too.
pub fn short(n: u32, on: bool) -> u32 {
    match n {
        1 if on => 10,
        // Two's, or one's off.
        2 | 1 => 20,
        3 => 30,
        _ => 0,
    }
}

/// The constant the guard's arm names isn't the next arm's first: Rust
/// tries \`E\` between, so no case falls to \`1 | 2\`'s.
pub fn apart(n: u32, on: bool) -> u32 {
    match n {
        1 if on => 10,
        5 => 50,
        1 | 2 => 20,
        3 => 30,
        _ => 0,
    }
}
`,
  );
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withJs()]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  // Upstream's, but for its \`typeof type\`'s test, the comment written
  // before the arm fallen to before its \`// Fallthrough\` (ADR 0905, amended).
  expect(js).toContain(`  switch (tag) {
    case ACTIVITY_COMPONENT:
      return "Activity";
    case VIEW_TRANSITION_COMPONENT:
      if (enableViewTransition) {
        return "ViewTransition";
      }
    // The display name for these tags come from the user-provided type:
    // Fallthrough
    case INCOMPLETE_CLASS_COMPONENT:
    case INCOMPLETE_FUNCTION_COMPONENT:
      if (disableLegacyMode) {
        break;
      }
    // A function's name, or the tag's.
    // Fallthrough
    case CLASS_COMPONENT:
    case FUNCTION_COMPONENT:
      if (ty.startsWith("<")) {
        return ty;
      }
      break;
    case LEGACY_HIDDEN_COMPONENT:
      if (enableViewTransition) {
        return "LegacyHidden";
      }
  }
  return null;
`);
  expect(js).toContain("  // Each tag's, a comment before the match, not an arm.\n  switch (tag) {\n");
  expect(js).toContain("      }\n    // Two's, or one's off.\n    // Fallthrough\n    case 2:\n      return 20;\n");
  expect(js).toContain(`    // A long log stops here.
    // Fallthrough
    case INCOMPLETE_CLASS_COMPONENT:
      if (log.length > 2) {
        break;
      }
    // Fallthrough
    case CLASS_COMPONENT:
      log.push(3);
      log.push(4);
  }
  log.push(0);
`);
  expect(js).not.toMatch(/\bbb\d+:/);
  expect(js).toMatch(/export function apart\(n, on\) \{\n  if \(/);
  const lib = await import(join(dir, "lib.js"));
  expect([lib.short(1, true), lib.short(1, false), lib.short(2, true), lib.short(3, false), lib.short(9, true)]).toEqual([10, 20, 20, 30, 0]);
  // Upstream's JS, as React writes it.
  const upstream = (tag: number, ty: string, enableViewTransition: boolean, disableLegacyMode: boolean) => {
    switch (tag) {
      case 1:
        return "Activity";
      case 2:
        if (enableViewTransition) {
          return "ViewTransition";
        }
      // Fallthrough
      case 3:
      case 4:
        if (disableLegacyMode) {
          break;
        }
      // Fallthrough
      case 5:
      case 6:
        if (ty.startsWith("<")) {
          return ty;
        }
        break;
      case 7:
        if (enableViewTransition) {
          return "LegacyHidden";
        }
        break;
    }
    return null;
  };
  for (let tag = 0; tag <= 8; tag++) {
    for (const ty of ["<div>", "div"]) {
      for (const [a, b] of [
        [false, false],
        [false, true],
        [true, false],
        [true, true],
      ] as const) {
        expect([tag, ty, a, b, lib.name(tag, ty, a, b)]).toEqual([tag, ty, a, b, upstream(tag, ty, a, b)]);
      }
    }
  }
  const counted = (tag: number, on: boolean, log: number[]) => {
    lib.counted(tag, on, log);
    return log;
  };
  expect([0, 1, 2, 3, 5].flatMap((tag) => [counted(tag, true, []), counted(tag, false, []), counted(tag, false, [9, 9, 9])])).toEqual([
    [0],
    [0],
    [9, 9, 9, 0],
    [1, 0],
    [1, 0],
    [9, 9, 9, 1, 0],
    [2, 0],
    [3, 4, 0],
    [9, 9, 9, 0],
    [3, 4, 0],
    [3, 4, 0],
    [9, 9, 9, 0],
    [3, 4, 0],
    [3, 4, 0],
    [9, 9, 9, 3, 4, 0],
  ]);
  expect([0, 1, 2, 3, 5].flatMap((n) => [lib.apart(n, true), lib.apart(n, false)])).toEqual([0, 0, 10, 20, 20, 20, 30, 30, 50, 50]);
});
