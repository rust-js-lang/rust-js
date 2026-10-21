// Mutations of src/lower/std_types/future.rs (ADR 0571).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "future-ready-rejected",
    breaks: "`future::ready(v)` is a promise rejected with `v`, not resolved",
    file: "src/lower/std_types/future.rs",
    find: `Expr::member(Expr::var("Promise"), "resolve")`,
    replace: `Expr::member(Expr::var("Promise"), "reject")`,
    tests: ["test/compiler.test.ts", "-t", "async code"],
  },
];
