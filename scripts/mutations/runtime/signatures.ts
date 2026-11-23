// Mutations of crates/rust-js-program/src/runtime/signatures.rs (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "signature-commas-in-brackets-split",
    breaks: "a helper's parameter list is split at a comma inside a default's brackets, `b = { c: 1, d: [2, 3] }`, so what follows is read as another parameter (#2334)",
    file: "crates/rust-js-program/src/runtime/signatures.rs",
    find: "            (None, ',') if depth == 1 => {\n",
    replace: "            (None, ',') => {\n",
    unitTests: "runtime::signatures",
  },
  {
    name: "signature-strings-unread",
    breaks: "a comma or `)` in a default's string ends a helper's parameter there (#2334)",
    file: "crates/rust-js-program/src/runtime/signatures.rs",
    find: "            (None, '\"' | '\\'' | '`') => quote = Some(c),\n",
    replace: "",
    unitTests: "runtime::signatures",
  },
  {
    name: "signature-rest-unseen",
    breaks: "a helper's rest parameter, `...items`, isn't known as its rest (#2334)",
    file: "crates/rust-js-program/src/runtime/signatures.rs",
    find: "        Some(text) => (Some(ParamKind::Rest), text),\n",
    replace: "        Some(text) => (None, text),\n",
    unitTests: "runtime::signatures",
  },
  {
    name: "signature-default-unseen",
    breaks: "a helper's parameter's default, `raisedAt = undefined`, isn't seen, so a location given only where its call asks is given never, and `total(std::iter::repeat(1))` panics `<unknown>` (#2334)",
    file: "crates/rust-js-program/src/runtime/signatures.rs",
    find: "    let defaulted = text[named.len()..].trim_start().starts_with('=');\n",
    replace: "    let defaulted = false;\n",
    tests: ["test/corpus.test.ts", "-t", "^repeat_counted_where_called.rs$"],
  },
];
