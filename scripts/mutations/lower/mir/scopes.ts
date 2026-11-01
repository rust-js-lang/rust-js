// Mutations of src/lower/mir/scopes.rs (ADR 1043).
import type { Mutation } from "../../../mutations";

const scopes = ["test/corpus.test.ts", "-t", "scope_struct"];

export const mutations: Mutation[] = [
  {
    name: "scope-functions-before-their-value",
    breaks: "a scope struct's methods' functions are declared before its value's `let`, not after it",
    file: "src/lower/mir/scopes.rs",
    find: "                list.splice(last + 1..last + 1, std::mem::take(&mut functions));",
    replace: "                list.splice(0..0, std::mem::take(&mut functions));",
    tests: scopes,
    snapshots: true,
  },
  {
    name: "scope-function-takes-self",
    breaks: "a scope struct's method's function takes the value it closes over as a parameter, which no call gives it",
    file: "src/lower/mir/scopes.rs",
    find: "                params.remove(0);\n",
    replace: "",
    tests: scopes,
  },
  {
    name: "scope-function-names-its-own",
    breaks: "a scope struct's method's function names its variables apart from none of its function's, a `counter` of its own hiding the one it closes over",
    file: "src/lower/mir/scopes.rs",
    find: "            names: self.names.clone(),\n",
    replace: "            names: Default::default(),\n",
    tests: scopes,
  },
];
