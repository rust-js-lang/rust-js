// Mutations of src/lower/serde/de.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "numbered-read-as-name",
    breaks:
      'a numbered enum read from JSON is its name, `"Entries"`, so `==` of it to `Kind::Entries`, `3`, is false',
    file: "src/lower/serde/de.rs",
    find: "                return self.unit_variant(adt.did(), variant);\n",
    replace: "                return Expr::str(name);\n",
    tests: ["test/serde.test.ts", "-t", "a repr enum serializes"],
  },
  {
    name: "json-value-set",
    breaks:
      "a `HashSet<Point>` read from JSON is a plain `Set`, which keeps equal points twice",
    file: "src/lower/serde/de.rs",
    find: '                if !matches!(&class.kind, js::ExprKind::Var(name) if name == "Set") {',
    replace: "                if false {",
    tests: ["test/serde.test.ts", "-t", "set of structs"],
  },
  {
    name: "generic-codec-readers-unpassed",
    breaks:
      "a generic type's derived `deserialize` is given no readers of its type arguments, so it reads its items with `undefined` and throws",
    file: "src/lower/serde/de.rs",
    find: "                if readers.is_empty() {\n                    return Ok(callee);\n                }",
    replace:
      "                if !readers.is_empty() {\n                    return Ok(callee);\n                }",
    tests: [
      "test/serde.test.ts",
      "-t",
      "a generic type's derived codec of a value with a destructor",
    ],
  },
  {
    name: "serde-field-read-as-its-type",
    breaks: "a field with its own reader is read as its type is",
    file: "src/lower/serde/de.rs",
    find: "        match self.field_codec(field, attrs, false, span)? {\n",
    replace: "        match None::<Expr> {\n",
    tests: ["test/serde.test.ts", "-t", "its own functions"],
  },
];
