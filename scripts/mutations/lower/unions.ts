// Mutations of src/lower/unions.rs (ADR 0406).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "union-read-no-alike",
    breaks:
      "a union's field of the same type as the one it holds is rust-js's error, where it reads the same bytes",
    file: "src/lower/unions.rs",
    find: "            if other_ty == read_ty {\n                alike.push(Expr::str(other_key));\n",
    replace: "            if other_ty == read_ty {\n",
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
  {
    name: "union-no-decoders",
    breaks:
      "a union's read is given no decoder, so a field of another plain type is refused",
    file: "src/lower/unions.rs",
    find: "                decoders.push(Prop::Field(other_key, decoder));\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^union_bytes.rs$"],
  },
  {
    name: "union-partial-object-decoded",
    breaks:
      "a union's struct field smaller than the union is decoded, and a write into it lost or the union's other bytes dropped",
    file: "src/lower/unions.rs",
    find: "        let writable_whole = scalar || union_size.is_some_and(|size| self.fills(read_ty, size));\n",
    replace:
      "        let writable_whole = true || scalar || union_size.is_some_and(|size| self.fills(read_ty, size));\n",
    tests: ["test/runtime-package.test.ts", "-t", "union"],
  },
];
