// Mutations of src/lower/mir/tidy/flags.rs (ADR 0093).
import type { Mutation } from "../../../../mutations";

const leaving = [
  "test/corpus.test.ts",
  "-t",
  "drop_operand_leaves|drop_operand_panic|closure_drops",
];

export const mutations: Mutation[] = [
  {
    name: "joined-flag-unnamed",
    breaks:
      "a drop flag tested with its pattern's test, `if (d && flag)`, keeps its temporary's name, `_13$0$live`",
    file: "src/lower/mir/tidy/flags.rs",
    find: "        ExprKind::Binary(js::Op::And, a, b) => tests_flag(a, flag) || tests_flag(b, flag),\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^question_drops.rs$"],
    snapshots: true,
  },
  {
    name: "flags-known-test-kept",
    breaks:
      "a drop's flag known `false`, as the value was just moved, is still tested, in a `try` of its own",
    file: "src/lower/mir/tidy/flags.rs",
    find: "        if let Some(holds) = tested\n",
    replace: "        if let Some(holds) = tested.filter(|_| false)\n",
    tests: leaving,
    snapshots: true,
  },
  {
    name: "flags-caught-as-ended",
    breaks:
      "a `catch` takes a flag as the `try` left it, though the panic came before the flag was set: a value moved after it isn't dropped",
    file: "src/lower/mir/tidy/flags.rs",
    find: "                let mut caught: Known = before\n                    .into_iter()\n",
    replace:
      "                let _ = before;\n                let mut caught: Known = known\n                    .clone()\n                    .into_iter()\n",
    tests: ["test/corpus.test.ts", "-t", "drop_update_panics"],
  },
  {
    name: "flags-loop-entry-kept",
    breaks:
      "a loop's body takes a flag as it was before the loop, though an earlier time round set it",
    file: "src/lower/mir/tidy/flags.rs",
    find: "                let mut entry = known.clone();\n                forget(&mut entry, body, flags);\n                settle(body, &mut entry, flags);\n",
    replace:
      "                let mut entry = known.clone();\n                settle(body, &mut entry, flags);\n",
    tests: ["test/corpus.test.ts", "-t", "drop_flag_loops"],
  },
  {
    name: "flags-temporary-unnamed",
    breaks:
      "a temporary's drop flag is named for MIR's local, `_14$live`, not for what it says is there",
    file: "src/lower/mir/tidy/flags.rs",
    find: "        if !super::is_temporary(base) {\n",
    replace: "        if true || !super::is_temporary(base) {\n",
    tests: [
      "test/corpus.test.ts",
      "-t",
      "option_combinator_drops|scope_temporaries",
    ],
    snapshots: true,
  },
  {
    name: "flags-panic-try-kept",
    breaks:
      "a panic's own `throw` is in a `try` whose `catch` drops and throws it on",
    file: "src/lower/mir/tidy/flags.rs",
    find: "                    && made_of_values(panic)\n",
    replace:
      "                    && made_of_values(panic)\n                    && false\n",
    tests: [
      "test/lowering.test.ts",
      "-t",
      "a value moved before anything can leave",
    ],
  },
];
