// Mutations of src/lower/unions.rs (ADR 0406).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "union-read-no-alike",
    breaks: "a union's field of the same type as the one it holds is rust-js's error, where it reads the same bytes",
    file: "src/lower/unions.rs",
    find: "        list.extend(\n",
    replace: "        list.truncate(2);\n        Vec::<Expr>::new().extend(\n",
    tests: ["test/runtime-package.test.ts", "-t", "union"],
  },
  {
    name: "union-copy-shared",
    breaks: "a union's copy is the same object: writing one changes the other",
    file: "src/lower/unions.rs",
    find: "        let mut copied = Expr::object(vec![Prop::Spread(union.clone())]);\n",
    replace: "        let mut copied = union.clone();\n",
    tests: ["test/corpus.test.ts", "-t", "^unions.rs$"],
  },
  {
    name: "union-copy-field-shared",
    breaks: "a union's copy shares the struct it holds, which changes in place",
    file: "src/lower/unions.rs",
    find: "            if !self.contains_mutated(field_ty) {\n                continue;\n",
    replace: "            if true {\n                continue;\n",
    tests: ["test/corpus.test.ts", "-t", "^unions.rs$"],
  },
];
