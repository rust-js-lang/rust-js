// Mutations of src/lower/declarations/std_types.rs (ADR 0093): the std
// types a crate's declarations name of `@rust-js/std`, opaque (#1752, ADR
// 1149).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "std-opaque-unbranded",
    breaks: "`@rust-js/std`'s `LazyCell<T>` is declared of its value, `{ value?: T }`, which TypeScript lets a caller make by hand, `counted({ value: 1 })`",
    file: "src/lower/declarations/std_types.rs",
    find: "export interface {}<T> {{\\n  readonly [{brand}]: T;\\n}}\\n",
    replace: "export interface {}<T> {{\\n  readonly value?: T;\\n}}\\n",
    tests: ["test/std-package.test.ts", "-t", "types are the catalog"],
  },
  {
    name: "std-opaque-of-its-item-alone",
    breaks: "a `LazyLock<Point>` is declared `@rust-js/std`'s `LazyCell<Point>`, the first of its std item's types, its name unasked",
    file: "src/lower/declarations/std_types.rs",
    find: "is_std_def(tcx, did, opaque.item) && opaque.lock == lock)",
    replace: "is_std_def(tcx, did, opaque.item))",
    tests: ["test/declarations-lazy.test.ts"],
  },
];
