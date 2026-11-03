// Mutations of src/lower/mir/hints/constants.rs (ADR 0093).
import type { Mutation } from "../../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "transparent-constant-no-case",
    breaks:
      "a `match` of a `#[repr(transparent)]` struct's constants is no `switch`, an `if` chain of its tests (ADR 0590, amended)",
    file: "src/lower/mir/hints/constants.rs",
    find: "                    && let thir::PatKind::Constant { .. } = field.kind\n",
    replace: "                    && let thir::PatKind::Never = field.kind\n",
    tests: [
      "test/lowering-constants.test.ts",
      "-t",
      "transparent struct's constants",
    ],
  },
  {
    name: "switch-of-somes-unseen",
    breaks:
      "a `match` of a `Some`'s constants, `Some(\"s\")`, isn't a `switch`, but an `if` chain",
    file: "src/lower/mir/hints/constants.rs",
    find: "                && !self.boxed_payload(some.pattern.ty) =>\n",
    replace:
      "                && !self.boxed_payload(some.pattern.ty)\n                && false =>\n",
    tests: ["test/lowering-switch-ends.test.ts", "-t", "strings from MIR is the"],
  },
  {
    name: "switch-of-registered-symbols-unseen",
    breaks:
      "a `match` of the registry's symbols isn't a `switch`, but an `if` chain",
    file: "src/lower/mir/hints/constants.rs",
    find: "            thir::PatKind::Leaf { .. } if self.recognition().is_symbol_for(pattern.ty) => {\n",
    replace:
      "            thir::PatKind::Leaf { .. } if self.recognition().is_symbol_for(pattern.ty) && false => {\n",
    tests: [
      "test/lowering-symbols.test.ts",
      "-t",
      "a match of the registry's symbols",
    ],
  },
  {
    name: "untagged-constant-no-case",
    breaks:
      'a `match` of `js::classify(rel)`\'s `Kind::String("preload")` arms is no `switch (rel)`, an `if` chain of `rel === "preload"` as React\'s processLink printed (rd242)',
    file: "src/lower/mir/hints/constants.rs",
    find: "                && let thir::PatKind::Constant { .. } = peeled(&field.pattern).kind\n",
    replace:
      "                && let thir::PatKind::Never = peeled(&field.pattern).kind\n",
    tests: [
      "test/lowering-strict-switches.test.ts",
      "-t",
      "a match of an untagged enum's constants is a switch of the value",
    ],
  },
  {
    name: "binding-of-constants-unswitched",
    breaks:
      "a `match` of constants whose arm binds what it matched, `symbol @ (VIEW_TRANSITION | TRACING_MARKER)`, is an `if` chain, not its `switch` (rd351, getComponentNameFromType)",
    file: "src/lower/mir/hints/constants.rs",
    find: "            thir::PatKind::Binding {\n                subpattern: Some(subpattern),\n                ..\n            } => self.valued_constants(subpattern, present),\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^switch_arm_binding_constants.rs$"],
    snapshots: true,
  },
  {
    name: "bool-option-no-switch",
    breaks:
      "a `match` of an `Option<bool>`'s `Some(false)` and `Some(true)` is a `value != null` test and an `if`/`else`, not React's `switch (shouldError(..)) { case false: .. case true: .. }` (#545)",
    file: "src/lower/mir/hints/constants.rs",
    find: "        let (a, b) = (some(first)?, some(second)?);\n",
    replace: "        let (a, b) = (some(first)?, some(second)?);\n        return None;\n",
    tests: ["test/corpus.test.ts", "-t", "^switch_of_an_optional_bool.rs$"],
    snapshots: true,
  },
  {
    name: "bool-option-guarded-switch",
    breaks:
      "a guarded `Some(false) if on` of an `Option<bool>` is a case of `false` with no test of its guard, so a `false` the guard fails runs its arm, not the `_` (#545)",
    file: "src/lower/mir/hints/constants.rs",
    find: "        if guards.iter().any(Option::is_some) {\n",
    replace: "        if guards.iter().any(Option::is_some) && false {\n",
    tests: ["test/corpus.test.ts", "-t", "^switch_of_an_optional_bool.rs$"],
    snapshots: true,
  },
];
