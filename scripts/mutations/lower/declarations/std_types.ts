// Mutations of crates/rust-js-lower/src/declarations/std_types.rs (ADR 0093): the std
// types a crate's declarations name of `@rust-js/std`, opaque (#1752, ADR
// 1149).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "some-none-undeclared",
    breaks: "`@rust-js/std`'s declarations leave out `SomeNone`, which a crate's declare an `Option<()>` of, so TypeScript can't resolve it (#842)",
    file: "crates/rust-js-lower/src/declarations/std_types.rs",
    find: "export interface {SOME_NONE} {{",
    replace: "export interface {SOME_NONE}Gone {{",
    tests: ["test/std-package.test.ts"],
  },
  {
    name: "std-opaque-unbranded",
    breaks: "`@rust-js/std`'s `LazyCell<T>` is declared of its value, `{ value?: T }`, which TypeScript lets a caller make by hand, `counted({ value: 1 })`",
    file: "crates/rust-js-lower/src/declarations/std_types.rs",
    find: "export interface {}{params} {{\\n  readonly [{brand}]: {held};\\n{keys}}}\\n",
    replace: "export interface {}{params} {{\\n  readonly value?: {held};\\n{keys}}}\\n",
    tests: ["test/std-package.test.ts", "-t", "types are the catalog"],
  },
  {
    name: "std-opaque-of-its-item-alone",
    breaks: "a `LazyLock<Point>` is declared `@rust-js/std`'s `LazyCell<Point>`, the first of its std item's types, its name unasked",
    file: "crates/rust-js-lower/src/declarations/std_types.rs",
    find: "Of::Cell { item, lock: its } => is_std_def(tcx, did, item) && its == lock,",
    replace: "Of::Cell { item, .. } => is_std_def(tcx, did, item),",
    tests: ["test/declarations-lazy.test.ts"],
  },
  {
    name: "json-error-opaque-unmatched",
    breaks: "a `serde_json::Error` a crate's function gives or takes is declared `any`, `_0: any`, where it's `@rust-js/std`'s opaque `JsonError` (#2373)",
    file: "crates/rust-js-lower/src/declarations/std_types.rs",
    find: "        Of::JsonError => is_json_error_def(tcx, did),\n",
    replace: "        Of::JsonError => false,\n",
    tests: ["test/declarations-json-error.test.ts"],
  },
  {
    name: "json-error-keys-undeclared",
    breaks: "`@rust-js/std`'s `JsonError` declares no `message`, `line` or `column`, so TypeScript can't read what serde_json's `line()` and `column()` give (#2373)",
    file: "crates/rust-js-lower/src/declarations/std_types.rs",
    find: "            Of::JsonError => (JSON_ERROR_KEYS.iter())\n                .map(|(key, ty)| format!(\"  readonly {key}: {ty};\\n\"))\n                .collect(),\n",
    replace: "            Of::JsonError => String::new(),\n",
    tests: ["test/std-package.test.ts", "-t", "types are the catalog"],
  },
  {
    name: "json-error-holds-a-type",
    breaks: "`@rust-js/std`'s `JsonError` is taken to hold a type, as a `LazyLock<T>` does, so a crate's declaration asks a `serde_json::Error` of its type parameter, which it hasn't (#2373)",
    file: "crates/rust-js-lower/src/declarations/std_types.rs",
    find: "        matches!(self.of, Of::Cell { .. })\n",
    replace: "        matches!(self.of, Of::Cell { .. } | Of::JsonError)\n",
    tests: ["test/declarations-json-error.test.ts", "test/std-package.test.ts", "-t", "JsonError|types are the catalog"],
  },
];
