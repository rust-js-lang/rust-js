// Mutations of src/lower/mir/bounds.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "mir-known-bounds-checked",
    breaks: "`args[i]` of `0..args.len()` THIR's facts know is in bounds is checked, `if (!(i < args.length)) throw`",
    file: "src/lower/mir/bounds.rs",
    find: "        matches!(msg, mir::AssertKind::BoundsCheck { .. })\n",
    replace: "        false && matches!(msg, mir::AssertKind::BoundsCheck { .. })\n",
    tests: ["test/lowering.test.ts", "-t", "early returns and index loops from MIR"],
  },
  {
    name: "mir-in-bounds-checked",
    breaks: "a call whose indexes the body's ranges show in bounds is checked, `$index(heap, leftIndex)` and `$swap`",
    file: "src/lower/mir/bounds.rs",
    find: "                .is_some_and(|(fun, _)| self.body_facts.in_bounds.contains(&fun))",
    replace: "                .is_some_and(|_| false)",
    tests: ["test/lowering.test.ts", "-t", "integer ranges show"],
  },
  {
    name: "swap-remove-shifted",
    breaks: "`swap_remove`'s item is taken from the front, so a heap's `pop` gives its second item",
    file: "src/lower/mir/bounds.rs",
    find: "            _ => Expr::call(Expr::member(items, \"pop\"), Vec::new()),",
    replace: "            _ => Expr::call(Expr::member(items, \"shift\"), Vec::new()),",
    tests: ["test/lowering.test.ts", "-t", "integer ranges show"],
  },
  {
    name: "swap-remove-last-unswapped",
    breaks: "`swap_remove(i)` swaps `i` with itself, so a heap's `pop` gives its last item",
    file: "src/lower/mir/bounds.rs",
    find: "                let last = Expr::bin(Op::Sub, Expr::member(items.clone(), \"length\"), Expr::int(1));\n                (exprs.remove(0), last)",
    replace: "                let _ = Expr::bin(Op::Sub, Expr::member(items.clone(), \"length\"), Expr::int(1));\n                let i = exprs.remove(0);\n                (i.clone(), i)",
    tests: ["test/lowering.test.ts", "-t", "integer ranges show"],
  },
];
