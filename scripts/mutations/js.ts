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
];
