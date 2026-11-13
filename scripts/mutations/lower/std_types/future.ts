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
    find: "                    && (self.is_callable(held) || self.is_unknown(held))\n",
    replace: "                    && false\n",
    tests: ["test/lowering-async.test.ts", "-t", "thenable"],
  },
  {
    name: "future-of-future-not-thenable",
    breaks:
      "an `Option` of a future an async fn gives is flattened by the promise, `None` where it's `Some`",
    file: "src/lower/std_types/future.rs",
    find: "            ty::Adt(..) if let Some(inner) = self.option_of(ty) => self.may_be_thenable(inner),\n",
    replace: "",
    tests: ["test/lowering-async.test.ts", "-t", "thenable"],
  },
  {
    name: "thenable-method-unsettled",
    breaks: "a class's or an object's value whose method is `then` is resolved as it is, and the promise adopts it: its `.await` never ends (#1071)",
    file: "src/lower/std_types/future.rs",
    find: '            ty::Adt(adt, _) if crate::lower::classes::has_method_named(self.tcx, adt.did(), "then") => true,\n',
    replace: "",
    tests: ["test/lowering-thenables.test.ts"],
  },
  {
    name: "thenable-field-by-rust-name",
    breaks: "a field whose JS name is `then`, by `#[rust_js::name]`, is taken for one that isn't, and its value is adopted where it's awaited (#1071)",
    file: "src/lower/std_types/future.rs",
    find: '                crate::lower::bindings::field_key(self.tcx, field).name() == Some("then")\n',
    replace: '                field.name.as_str() == "then"\n',
    tests: ["test/lowering-thenables.test.ts"],
  },
];
