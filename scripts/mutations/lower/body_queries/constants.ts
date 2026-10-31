// Mutations of src/lower/body_queries/constants.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "transparent-constant-unnamed",
    breaks:
      "a `match` of a `#[repr(transparent)]` struct's constants tests their numbers, `eventPriority === 2`, not `DiscreteEventPriority` (ADR 0590, amended)",
    file: "src/lower/body_queries/constants.rs",
    find: "        PatKind::Leaf { .. } if let Some(field) = transparent_subpattern(tcx, pat) => pattern_value(tcx, field),\n",
    replace: "",
    tests: [
      "test/lowering-constants.test.ts",
      "-t",
      "transparent struct's constants",
    ],
  },
  {
    name: "transparent-constant-field-walked",
    breaks:
      "a `#[repr(transparent)]` struct's constant's field, its number, is a pattern written another way of its value, so the constant goes unnamed (ADR 0590, amended)",
    file: "src/lower/body_queries/constants.rs",
    find: "        tested.push((value, path, named));\n        return;\n",
    replace: "        tested.push((value, path, named));\n",
    tests: [
      "test/lowering-constants.test.ts",
      "-t",
      "transparent struct's constants",
    ],
  },
  {
    name: "constant-written-two-ways-named",
    breaks:
      "a value one pattern names by a `const` and another writes as a literal is tested by the `const`'s name in both",
    file: "src/lower/body_queries/constants.rs",
    find: "            .map(|&(_, _, named)| named);\n        let first = named.next()??;\n        named.all(|named| named == Some(first)).then_some(first)\n",
    replace:
      "            .filter_map(|&(_, _, named)| named);\n        named.next()\n",
    tests: ["test/lowering-constants.test.ts", "-t", "constants of one value"],
  },
  {
    name: "same-valued-constants-of-the-outer-match",
    breaks:
      "a value two constants name, 5, is named in an inner `match` by the outer's constant, `tag === SuspendedOnInstanceAndReadyToContinue`, not `case HostComponent:` (ADR 0590, amended)",
    file: "src/lower/body_queries/constants.rs",
    find: "            .min_by_key(|(written, ..)| written.hi() - written.lo())?;\n",
    replace:
      "            .max_by_key(|(written, ..)| written.hi() - written.lo())?;\n",
    tests: ["test/lowering-constants.test.ts", "-t", "constants of one value"],
  },
  {
    name: "same-valued-constants-of-any-field",
    breaks:
      "a value two constants name in two fields of a tuple's `match`, `(SUSPENDED_ON_INSTANCE_AND_READY_TO_CONTINUE, HOST_TEXT)` and `(_, HOST_COMPONENT)`, is its number, `reason === 5` (ADR 0590, amended)",
    file: "src/lower/body_queries/constants.rs",
    find: "places.iter().any(|(ty, steps)| alike(*ty, *matched) && steps == path)",
    replace: "places.iter().any(|(ty, _)| alike(*ty, *matched))",
    tests: ["test/lowering-constants.test.ts", "-t", "constants of one value"],
  },
  {
    name: "same-valued-constants-of-any-part",
    breaks:
      "a field of a tuple inside what's matched is taken for one of what's matched, so `(X, (Y, _))` of one value names neither, `[tag, other][0] === 5` (ADR 0590, amended)",
    file: "src/lower/body_queries/constants.rs",
    find: "places.iter().any(|(ty, steps)| alike(*ty, *matched) && steps == path)",
    replace: "places.iter().any(|(_, steps)| steps == path)",
    tests: ["test/lowering-constants.test.ts", "-t", "constants of one value"],
  },
  {
    name: "same-valued-constants-of-another-expansion",
    breaks:
      "a `matches!`'s test is where core writes it, not where it's called, so two in one body are taken for one, and the second's constant is the first's, `tag === SuspendedOnInstanceAndReadyToContinue` (ADR 0590, amended)",
    file: "src/lower/body_queries/constants.rs",
    find: "        let at = at.source_callsite();\n",
    replace: "",
    tests: ["test/lowering-constants.test.ts", "-t", "constants of one value"],
  },
  {
    name: "if-let-constants-unnamed",
    breaks:
      "an `if let Some(HOST_COMPONENT) = tag` tests its number, `tag === 5`: no `match` is around it (ADR 0590, amended)",
    file: "src/lower/body_queries/constants.rs",
    find: "            ExprKind::Let { ref pat, .. } => found.matched.extend(of(expr.span, &[pat])),\n",
    replace: "",
    tests: ["test/lowering-constants.test.ts", "-t", "constants of one value"],
  },
  {
    name: "let-else-constants-unnamed",
    breaks:
      "a `let Some(HOST_TEXT) = tag else { .. }` tests its number, `tag !== 6` (ADR 0590, amended)",
    file: "src/lower/body_queries/constants.rs",
    find: "            found.matched.extend(of(span, &[pattern]));\n",
    replace: "",
    tests: ["test/lowering-constants.test.ts", "-t", "constants of one value"],
  },
  {
    name: "registered-symbol-pattern-unnamed",
    breaks:
      'a pattern of the registry\'s symbol tests it by its value, `ty === Symbol.for("react.fragment")`, not by the `const` that names it',
    file: "src/lower/body_queries/constants.rs",
    find: "                [key] => pattern_text(&key.pattern).map(PatternValue::Key),\n",
    replace: "                [_] => None,\n",
    tests: [
      "test/lowering-symbols.test.ts",
      "-t",
      "a match of the registry's symbols",
    ],
  },
  {
    name: "pattern-trait-const-named",
    breaks:
      "a pattern of a trait's constant, `<u8 as Level>::HIGH`, is named with no type, and rustc crashes resolving it",
    file: "src/lower/body_queries/constants.rs",
    find: "            .filter(|&d| tcx.generics_of(d).count() == 0);\n",
    replace: ";\n",
    tests: ["test/corpus.test.ts", "-t", "^match_trait_const\\.rs$"],
  },
  {
    name: "text-pattern-untold",
    breaks:
      'a string pattern has no value, so a guarded arm that falls to the next string arm, `"keypress" if .. => return`, isn\'t known to fall and its `match` is no `switch`: `bb22: { if (name === "keypress") { .. } else if (name !== "keydown") { .. } ctor = 1; }` (rd112)',
    file: "src/lower/body_queries/constants.rs",
    find: "        _ => pattern_text(pat).map(PatternValue::Text),\n",
    replace: "        _ => None,\n",
    tests: ["test/corpus.test.ts", "-t", "^switch_text_guard_falls.rs$"],
    snapshots: true,
  },
];
