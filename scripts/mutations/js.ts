// Mutations of src/js.rs.
import type { Mutation } from "../mutations";

export const mutations: Mutation[] = [
  {
    name: "load-statements-unread",
    breaks: "the helpers what a module runs when it's loaded reads, `$index`, aren't imported",
    file: "src/js.rs",
    find: "            Item::Statements(stmts) => visit_stmts(stmts, read),\n",
    replace: "            Item::Statements(_) => {}\n",
    tests: ["test/modules.test.ts", "-t", "js::on_load!"],
  },
  {
    name: "option-eq-one-side-none",
    breaks: "a getter's `T | null` compared with the crate's `Option`, maybe `undefined`, by `===`, so `a.parentNode === b` is false of `null` and `None` (rd117)",
    file: "src/js.rs",
    find: "        none == other.of_presence() && (none == Facts::of(Known::NoneNull) || none == Facts::of(Known::NoneUndefined))\n",
    replace: "        none == Facts::of(Known::NoneNull) || none == Facts::of(Known::NoneUndefined)\n",
    tests: ["test/webapi-bindings.test.ts", "-t", "equal only to itself"],
  },
  {
    name: "evaluation-facts-forgotten",
    breaks: "an operator of any value made again, `+m` or `m != 1` of `!(m == 1)`, forgets that JS may throw of it, so unread it goes (#462)",
    file: "src/js.rs",
    find: "        Facts(self.0 & Facts::of(Known::Getter).0)\n",
    replace: "        Facts::NONE\n",
    tests: ["test/lowering-unwind-arms.test.ts", "-t", "a caught value nothing reads that may throw"],
  },
];
