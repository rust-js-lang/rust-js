// Mutations of crates/rust-js-program/src/js/path.rs (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "field-path-not-of-this",
    breaks:
      "no path is read through `this`, so a class's method's read of its own `bool` field isn't known a `bool`: `!!this.open || this.below(x)`",
    file: "crates/rust-js-program/src/js/path.rs",
    find: "            ExprKind::This => Path::this(),\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^class_bool_methods.rs$"],
    snapshots: true,
  },];
