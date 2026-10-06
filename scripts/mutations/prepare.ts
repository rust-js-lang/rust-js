// Mutations of src/prepare.rs (ADR 0093).
import type { Mutation } from "../mutations";

export const mutations: Mutation[] = [
  {
    name: "handler-of-one-call-blocked",
    breaks: "a handler of one call is a block, `() => { setCount(1); }`, where React ignores what it returns",
    file: "src/prepare.rs",
    find: "                            *body = vec![StmtKind::Return(Some(e.clone())).at(*span)];\n",
    replace: "",
    tests: ["test/jsx.test.ts"],
  },
];
