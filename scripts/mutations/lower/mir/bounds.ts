// Mutations of src/lower/mir/bounds.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "mir-known-bounds-checked",
    breaks: "`args[i]` of `0..args.len()` THIR's facts know is in bounds is checked, `if (!(i < args.length)) throw`",
    file: "src/lower/mir/bounds.rs",
    find: "        matches!(msg, mir::AssertKind::BoundsCheck { .. })\n",
    replace: "        false && matches!(msg, mir::AssertKind::BoundsCheck { .. })\n",
    tests: ["test/lowering-loops.test.ts", "-t", "early returns and index loops from MIR"],
  },
  {
    name: "mir-in-bounds-checked",
    breaks: "a call whose indexes the body's ranges show in bounds is checked, `$index(heap, leftIndex)` and `$swap`",
    file: "src/lower/mir/bounds.rs",
    find: "(self.thir_call(def_id, span)).is_some_and(|(fun, _)| self.frame.facts.in_bounds.contains(&fun))",
    replace: "(self.thir_call(def_id, span)).is_some_and(|_| false)",
    tests: ["test/lowering-numbers.test.ts", "-t", "integer ranges show"],
  },
  {
    name: "swap-remove-shifted",
    breaks: "`swap_remove`'s item is taken from the front, so a heap's `pop` gives its second item",
    file: "src/lower/mir/bounds.rs",
    find: "            _ => Expr::call(Expr::member(items, \"pop\"), Vec::new()),",
    replace: "            _ => Expr::call(Expr::member(items, \"shift\"), Vec::new()),",
    tests: ["test/lowering-numbers.test.ts", "-t", "integer ranges show"],
  },
  {
    name: "swap-remove-last-unswapped",
    breaks: "`swap_remove(i)` swaps `i` with itself, so a heap's `pop` gives its last item",
    file: "src/lower/mir/bounds.rs",
    find: "                let last = Expr::bin(Op::Sub, Expr::member(items.clone(), \"length\"), Expr::int(1));\n                (exprs.remove(0), last)",
    replace: "                let _ = Expr::bin(Op::Sub, Expr::member(items.clone(), \"length\"), Expr::int(1));\n                let i = exprs.remove(0);\n                (i.clone(), i)",
    tests: ["test/lowering-numbers.test.ts", "-t", "integer ranges show"],
  },
  {
    name: "mir-index-mut-a-value",
    breaks: "`v[i] += 1` of numbers through `index_mut` changes a copy of the element",
    file: "src/lower/mir/bounds.rs",
    find: "            return Ok(Value::Ref(Expr::index(items, index)));\n",
    replace: "            return Ok(Value::Expr(Expr::index(items, index)));\n",
    tests: ["test/corpus.test.ts","-t","compound_assignment_order"],
  },
  {
    "name": "mir-checked-index-copied",
    "breaks": "a parameter MIR checks an element's place at is copied first, `const index = i;`",
    "file": "src/lower/mir/bounds.rs",
    "find": "                || matches!(&index.kind, js::ExprKind::Var(v) if self.locals.mir.alike.contains(v))\n",
    "replace": "",
    "tests": ["test/lowering-numbers.test.ts", "-t", "checked element set is one statement"]
  },
  {
    name: "index-mut-known-checked",
    breaks: "an item's place through `index_mut` the ranges show in bounds is checked, `$index(heap, 0);` before `heap[0] = last`",
    file: "src/lower/mir/bounds.rs",
    find: "            if checked {\n",
    replace: "            if true || checked {\n",
    tests: ["test/lowering-options.test.ts", "-t", "nullable result is written"],
  },
];
