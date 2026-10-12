// Mutations of src/lower/places.rs (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "constant-index-checked",
    breaks: "an array's constant index below its length, `HEADERS[0]`, is checked, `$index(HEADERS, 0)`, as an index that may be past the end is",
    file: "src/lower/places.rs",
    find: "        (0..i128::from(len)).contains(&i)",
    replace: "        false && (0..i128::from(len)).contains(&i)",
    tests: ["test/compiler.test.ts", "-t", "the report's JS: function values, case mapping, and plain array reads"],
  },
  {
    name: "through-same-unseen",
    breaks: "`v[1].make_ascii_uppercase()` of a `String` isn't its place",
    file: "src/lower/places.rs",
    find: "        {\n            place = inner;\n        }\n        place\n",
    replace: "        {\n            break;\n        }\n        place\n",
    tests: ["test/corpus.test.ts","-t","str_mut"],
  },
  {
    name: "leaked-unboxed",
    breaks: "a leaked `String`'s `&mut str` is the string, not a box of it",
    file: "src/lower/places.rs",
    find: "        match self.is_cell(output) {\n            true => Expr::object(",
    replace: "        match false {\n            true => Expr::object(",
    tests: ["test/corpus.test.ts","-t","leaked_string_written"],
  },
];
