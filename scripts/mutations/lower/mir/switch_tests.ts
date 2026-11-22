// Mutations of crates/rust-js-mir/src/switch_tests.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

const given1840 = ["test/none-given.test.ts", "-t", "in Bun and in Chromium"];

export const mutations: Mutation[] = [
  {
    name: "untagged-arms-refusal-unsaid",
    breaks:
      "a `match`'s refusal of an untagged enum's variants of one kind doesn't say it's rust-js's, so it's taken for a crash (#939)",
    file: "crates/rust-js-mir/src/switch_tests.rs",
    find: "                            \"rust-js: `{}` and `{other}` hold values of one kind,",
    replace: "                            \"`{}` and `{other}` hold values of one kind,",
    tests: ["test/diagnostics.test.ts", "-t", "variants of one kind, matched"],
  },
  {
    name: "mir-branch-continue-is-break",
    breaks:
      "`?` goes on with an `Err` or a `None`, and returns early with what's inside",
    file: "crates/rust-js-mir/src/switch_tests.rs",
    find: "                    0 => failed.negated_test(),\n                    _ => failed,\n",
    replace:
      "                    0 => failed,\n                    _ => failed.negated_test(),\n",
    tests: ["test/corpus.test.ts", "-t", "question_early_returns"],
  },
  {
    name: "mir-variant-test-not-bool",
    breaks:
      "a variant's test function isn't known to give a `bool`, `!!(isValidElement(child) && ..)`",
    file: "crates/rust-js-mir/src/switch_tests.rs",
    find: "                    self.locals.mir.paths.entry(path).or_insert(true);\n",
    replace: "",
    tests: [
      "test/jsx-children.test.ts",
      "test/jsx-evaluation-order.test.ts",
      "test/jsx-events.test.ts",
      "test/jsx-props.test.ts",
      "test/jsx-react-dev.test.ts",
      "test/jsx-types.test.ts",
      "test/jsx.test.ts",
    ],
  },
  {
    name: "mir-untagged-alike-matched",
    breaks:
      "MIR's match of an untagged enum's variants of one kind takes the first for both",
    file: "crates/rust-js-mir/src/switch_tests.rs",
    find: "                if bindings::is_untagged(self.tcx, adt.did())\n                    && let Some((other, classes)) = self.untagged_alike(*enum_ty, adt.variant(variant))\n",
    replace:
      "                if false\n                    && let Some((other, classes)) = self.untagged_alike(*enum_ty, adt.variant(variant))\n",
    tests: [
      "test/lowering.test.ts",
      "-t",
      "MIR refuses an untagged enum's variants",
    ],
  },
  {
    name: "constant-patterns-numbered",
    breaks:
      "a `match` of the crate's constants tests their numbers, `priorityLevel === 1`",
    file: "crates/rust-js-mir/src/switch_tests.rs",
    find: "                let value_of = rust_js_lower::body_queries::PatternValue::Bits(ty, value);\n",
    replace:
      "                let value_of = rust_js_lower::body_queries::PatternValue::Bits(ty, value + 1000);\n",
    tests: [
      "test/lowering-matches.test.ts",
      "-t",
      "a match of constants cases each",
    ],
  },
  {
    name: "option-bool-pattern-truth",
    breaks:
      "an `Option<bool>`'s pattern `Some(true)` is its truth, `if (flag)`, not `flag === true` (ADR 0298, the user's decision)",
    file: "crates/rust-js-mir/src/switch_tests.rs",
    find: "                if ty.is_bool() && self.option_bool_payload(tested) {\n",
    replace: "                if false && ty.is_bool() && self.option_bool_payload(tested) {\n",
    tests: ["test/corpus.test.ts", "-t", "^option_bool_exact.rs$"],
    snapshots: true,
  },
  {
    name: "option-bool-false-unwritten",
    breaks:
      "an `Option<bool>`'s `matches!(o, Some(false))` is `o != null && o !== true`, not `o === false` (ADR 0298, the user's decision)",
    file: "crates/rust-js-mir/src/switch_tests.rs",
    find: "                        value != 0 || (self.frame.facts.constant_patterns).tests(self.tcx, &value_of, span, tested);\n",
    replace: "                        value != 0;\n",
    tests: ["test/corpus.test.ts", "-t", "^option_bool_exact.rs$"],
    snapshots: true,
  },
];
