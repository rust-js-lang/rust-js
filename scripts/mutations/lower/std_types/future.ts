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
  {
    name: "then-field-not-thenable",
    breaks:
      "a struct with a function named `then` an async fn gives is adopted as a thenable, and its `.await` never resolves",
    file: "src/lower/std_types/future.rs",
    find: '                field.name.as_str() == "then" && (self.is_callable(held) || self.is_unknown(held))\n',
    replace:
      '                false && field.name.as_str() == "then" && (self.is_callable(held) || self.is_unknown(held))\n',
    tests: ["test/lowering-functions.test.ts", "-t", "thenable"],
  },
  {
    name: "future-of-future-not-thenable",
    breaks:
      "an `Option` of a future an async fn gives is flattened by the promise, `None` where it's `Some`",
    file: "src/lower/std_types/future.rs",
    find: "            ty::Adt(..) if let Some(inner) = self.option_of(ty) => self.may_be_thenable(inner),\n",
    replace: "",
    tests: ["test/lowering-functions.test.ts", "-t", "thenable"],
  },
];
