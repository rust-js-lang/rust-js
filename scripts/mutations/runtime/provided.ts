// Mutations of crates/rust-js-program/src/runtime/provided.js (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "provided-always-the-default",
    breaks: "`$provided` always runs the trait's default, never the impl's own, so a reader of the crate's that writes `read_line` is read by std's (#2414)",
    file: "crates/rust-js-program/src/runtime/provided.js",
    find: "  return written === undefined ? provided.apply(undefined, byDefault) : written.apply(undefined, entry);\n",
    replace: "  return provided.apply(undefined, byDefault);\n",
    tests: ["test/corpus.test.ts", "-t", "^provided_methods_a_type_writes_are_its_own.rs$"],
  },
];
