// Mutations of crates/rust-js-program/src/runtime/provided.js (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "provided-always-the-default",
    breaks: "`$provided` always runs the trait's default, never the impl's own, so a reader of the crate's that writes `read_line` is read by std's (#2414)",
    file: "crates/rust-js-program/src/runtime/provided.js",
    find: "  return written === undefined ? byDefault() : written.apply(undefined, entry);\n",
    replace: "  return byDefault();\n",
    tests: ["test/corpus.test.ts", "-t", "^provided_methods_a_type_writes_are_its_own.rs$"],
  },
  {
    name: "provided-default-made-first",
    breaks: "`$provided` runs the default before it asks for the impl's own, so a reader's default `read_line` reads a line the crate's own `read_line` then follows (#2414)",
    file: "crates/rust-js-program/src/runtime/provided.js",
    find: "  const written = dictionary[key];\n  return written === undefined ? byDefault() : written.apply(undefined, entry);\n",
    replace: "  const written = dictionary[key];\n  const made = byDefault();\n  return written === undefined ? made : written.apply(undefined, entry);\n",
    tests: ["test/corpus.test.ts", "-t", "^provided_methods_a_type_writes_are_its_own.rs$"],
  },
];
