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

// React's getOwnerStackByFiberInDev, ported: two guarded arms, the first's
// failed guard going on to the next arm, which names its constant, the
// last's to nothing; and a guard MIR tests part of with the arm's
// constant, `tag === FORWARD_REF && !owned`, and part in an `if` inside.
// Upstream's `switch`, no label (ADRs 0636, 0905).
test("two guarded arms, one falling to the next, one to nothing, are one switch", async () => {
  const dir = fixture("switch-two-guards");
  writeFileSync(
    join(dir, "lib.rs"),
    `pub const HOST_COMPONENT: u8 = 5;
pub const FUNCTION_COMPONENT: u8 = 0;
pub const FORWARD_REF: u8 = 11;
pub const VIEW_TRANSITION_COMPONENT: u8 = 30;

pub fn owner_stack(tag: u8, owned: bool, view_transitions: bool) -> String {
    let mut info = String::new();
    match tag {
        HOST_COMPONENT => {
            info += "host";
        }
        VIEW_TRANSITION_COMPONENT if view_transitions => {
            info += "ViewTransition";
        }
        VIEW_TRANSITION_COMPONENT | FUNCTION_COMPONENT => {
            if !owned && info.is_empty() {
                info += "function";
            }
        }
        FORWARD_REF if !owned && info.is_empty() => {
            info += "forwardRef";
        }
        _ => {}
    }
    let mut owner = Some(2);
    while let Some(o) = owner {
        info += "/";
        owner = if o > 0 { Some(o - 1) } else { None };
    }
    info
}
`,
  );
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withJs()]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain(`  let info = "";
  switch (tag) {
    case HOST_COMPONENT:
      info += "host";
      break;
    case VIEW_TRANSITION_COMPONENT:
      if (viewTransitions) {
        info += "ViewTransition";
        break;
      }
    // Fallthrough
    case FUNCTION_COMPONENT:
      if (!owned && !info) {
        info += "function";
      }
      break;
    case FORWARD_REF:
      if (!owned && !info) {
        info += "forwardRef";
      }
  }
  for (let owner = 2;`);
  expect(js).not.toMatch(/\bbb\d+:/);
  const lib = await import(join(dir, "lib.js"));
  // Upstream's JS, as React writes it.
  const upstream = (tag: number, owned: boolean, viewTransitions: boolean) => {
    let info = "";
    switch (tag) {
      case 5:
        info += "host";
        break;
      case 30:
        if (viewTransitions) {
          info += "ViewTransition";
          break;
        }
      // Fallthrough
      case 0:
        if (!owned && info === "") {
          info += "function";
        }
        break;
      case 11:
        if (!owned && info === "") {
          info += "forwardRef";
        }
        break;
    }
    return `${info}///`;
  };
  for (const tag of [0, 5, 11, 30, 99]) {
    for (const owned of [false, true]) {
      for (const viewTransitions of [false, true]) {
        const got = lib.ownerStack(tag, owned, viewTransitions);
        expect([tag, owned, viewTransitions, got]).toEqual([tag, owned, viewTransitions, upstream(tag, owned, viewTransitions)]);
      }
    }
  }
});

// React's describeFiber, ported: a `match` that's its function's value,
// whose guarded arm just before a last `_` that does something goes on to
// it where the guard fails, as upstream's case falls through to its
// `default` (ADR 0636, amended).
test("a guarded arm before a last _ that does something falls through to the default", async () => {
  const dir = fixture("switch-guard-to-default");
  writeFileSync(
    join(dir, "lib.rs"),
    `pub const HOST_COMPONENT: u8 = 5;
pub const LAZY_COMPONENT: u8 = 16;
pub const VIEW_TRANSITION_COMPONENT: u8 = 30;

pub fn describe(tag: u8, view_transitions: bool) -> String {
    match tag {
        HOST_COMPONENT => "host".to_string(),
        LAZY_COMPONENT => "Lazy".to_string(),
        VIEW_TRANSITION_COMPONENT if view_transitions => "ViewTransition".to_string(),
        // Anything else.
        _ => format!("<{tag}>"),
    }
}

/// An arm between: no case falls to the \`default\`, an \`if\` chain.
pub fn between(tag: u8, view_transitions: bool) -> u32 {
    match tag {
        HOST_COMPONENT => 1,
        VIEW_TRANSITION_COMPONENT if view_transitions => 2,
        LAZY_COMPONENT => 3,
        _ => 4,
    }
}
`,
  );
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withJs()]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain(`  switch (tag) {
    case HOST_COMPONENT:
      return "host";
    case LAZY_COMPONENT:
      return "Lazy";
    case VIEW_TRANSITION_COMPONENT:
      if (viewTransitions) {
        return "ViewTransition";
      }
    // Anything else.
    // Fallthrough
    default:
      return \`<\${tag}>\`;
  }
`);
  expect(js).toMatch(/export function between\(tag, viewTransitions\) \{\n  if \(/);
  const lib = await import(join(dir, "lib.js"));
  for (const tag of [0, 5, 16, 30]) {
    for (const on of [false, true]) {
      const upstream = tag === 5 ? "host" : tag === 16 ? "Lazy" : tag === 30 && on ? "ViewTransition" : `<${tag}>`;
      expect([tag, on, lib.describe(tag, on)]).toEqual([tag, on, upstream]);
      const between = tag === 5 ? 1 : tag === 30 && on ? 2 : tag === 16 ? 3 : 4;
      expect([tag, on, lib.between(tag, on)]).toEqual([tag, on, between]);
    }
  }
});

// ADR 0905, amended: React's createFiberFromTypeAndProps' `getTag`, four
// guarded arms of its flags, each falling to the next, of the registry's
// symbols, which MIR tests by their `==`, one guard's `if` and the next
// arm's in its `else`, `if (scope) { .. } else if (tracing) { .. }`: one
// `switch`, each case falling through, as upstream's.
test("four guarded arms of symbols, each falling to the next, are one switch", async () => {
  const dir = fixture("switch-four-falling");
  writeFileSync(
    join(dir, "lib.rs"),
    `use js::{SymbolFor, Unknown};

const REACT_FRAGMENT_TYPE: SymbolFor = SymbolFor("react.fragment");
const REACT_LEGACY_HIDDEN_TYPE: SymbolFor = SymbolFor("react.legacy_hidden");
const REACT_VIEW_TRANSITION_TYPE: SymbolFor = SymbolFor("react.view_transition");
const REACT_SCOPE_TYPE: SymbolFor = SymbolFor("react.scope");
const REACT_TRACING_MARKER_TYPE: SymbolFor = SymbolFor("react.tracing_marker");

const ENABLE_LEGACY_HIDDEN: bool = false;
const ENABLE_VIEW_TRANSITION: bool = true;
const ENABLE_SCOPE_API: bool = false;
const ENABLE_TRANSITION_TRACING: bool = false;

pub fn tag_of(r#type: Option<&Unknown>) -> &'static str {
    let mut tag = "function";
    if js::type_of(r#type) == "function" {
        tag = "class";
    } else if js::type_of(r#type) == "string" {
        tag = "host";
    } else {
    'get_tag: {
        match SymbolFor::from_value(r#type) {
            Some(REACT_FRAGMENT_TYPE) => return "fragment",
            Some(REACT_LEGACY_HIDDEN_TYPE) if ENABLE_LEGACY_HIDDEN => return "legacy hidden",
            // falls through
            Some(REACT_LEGACY_HIDDEN_TYPE | REACT_VIEW_TRANSITION_TYPE) if ENABLE_VIEW_TRANSITION => {
                return "view transition";
            }
            // falls through
            Some(REACT_LEGACY_HIDDEN_TYPE | REACT_VIEW_TRANSITION_TYPE | REACT_SCOPE_TYPE)
                if ENABLE_SCOPE_API =>
            {
                return "scope";
            }
            // falls through
            Some(
                REACT_LEGACY_HIDDEN_TYPE
                | REACT_VIEW_TRANSITION_TYPE
                | REACT_SCOPE_TYPE
                | REACT_TRACING_MARKER_TYPE,
            ) if ENABLE_TRANSITION_TRACING => return "tracing marker",
            _ => {
                if js::type_of(r#type) == "object"
                    && let Some(object) = r#type
                {
                    match SymbolFor::from_value(js::get(object, "$$typeof")) {
                        Some(REACT_SCOPE_TYPE) => {
                            tag = "scope object";
                            break 'get_tag;
                        }
                        Some(REACT_FRAGMENT_TYPE) => {
                            tag = "fragment object";
                            break 'get_tag;
                        }
                        Some(REACT_TRACING_MARKER_TYPE) => {
                            tag = "marker object";
                            break 'get_tag;
                        }
                        _ => {}
                    }
                }
                tag = "throw";
            }
        }
    }
    }
    tag
}
`,
  );
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withJs()]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain(`      case REACT_SCOPE_TYPE:
        if (ENABLE_SCOPE_API) {
          return "scope";
        }
      // falls through
      case REACT_TRACING_MARKER_TYPE:
        if (ENABLE_TRANSITION_TRACING) {
          return "tracing marker";
        }
`);
  const lib = await import(join(dir, "lib.js"));
  const of = (name: string) => Symbol.for(`react.${name}`);
  expect(["fragment", "legacy_hidden", "view_transition", "scope", "tracing_marker"].map((n) => lib.tagOf(of(n))))
    .toEqual(["fragment", "view transition", "view transition", "throw", "throw"]);
  expect([lib.tagOf({ $$typeof: of("scope") }), lib.tagOf({ $$typeof: of("fragment") }), lib.tagOf({}), lib.tagOf(null), lib.tagOf(3)])
    .toEqual(["scope object", "fragment object", "throw", "throw", "throw"]);
  expect([lib.tagOf(() => {}), lib.tagOf("div")]).toEqual(["class", "host"]);
});
