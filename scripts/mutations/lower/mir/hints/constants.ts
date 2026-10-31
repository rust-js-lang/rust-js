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
    tests: ["test/lowering-matches.test.ts", "-t", "strings from MIR is the"],
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
];
