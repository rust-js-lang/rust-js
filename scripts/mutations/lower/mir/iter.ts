// Mutations of src/lower/mir/iter.rs (ADR 0364).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "mir-enumerate-swapped",
    breaks: "`enumerate` gives `(item, index)`, not `(index, item)`",
    file: "src/lower/mir/iter.rs",
    find: "                let pair = Expr::array(vec![Expr::var(&i), Expr::var(&x)]);\n",
    replace: "                let pair = Expr::array(vec![Expr::var(&x), Expr::var(&i)]);\n",
    tests: ["test/mir.test.ts", "-t", "labeled_blocks"],
  },
  {
    name: "mir-sum-from-one",
    breaks: "`sum` of an iterator starts from one",
    file: "src/lower/mir/iter.rs",
    find: "                    \"sum\" => (Op::Add, 0),\n",
    replace: "                    \"sum\" => (Op::Add, 1),\n",
    tests: ["test/mir.test.ts", "-t", "assert_failure"],
  },
  {
    name: "mir-max-is-min",
    breaks: "`max()` of an iterator from MIR is its least item",
    file: "src/lower/mir/iter.rs",
    find: "                self.extreme_of(name == \"max\", items, item, span)?\n",
    replace: "                self.extreme_of(name == \"min\", items, item, span)?\n",
    tests: ["test/mir.test.ts","-t","enum_order|opaque_iterators|generic_as_ref|returned_references"],
  },
];
