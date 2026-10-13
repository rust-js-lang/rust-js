// Mutations of src/lower/places.rs (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "leaked-unboxed",
    breaks: "a leaked `String`'s `&mut str` is the string, not a box of it",
    file: "src/lower/places.rs",
    find: "        match self.is_cell(output) {\n            true => Expr::object(",
    replace: "        match false {\n            true => Expr::object(",
    tests: ["test/corpus.test.ts","-t","leaked_string_written"],
  },
];
