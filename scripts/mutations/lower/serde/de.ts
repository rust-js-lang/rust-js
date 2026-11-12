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
    tests: ["test/serde-values.test.ts", "-t", "a repr enum serializes"],
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
    tests: ["test/serde-attributes.test.ts", "-t", "its own functions"],
  },
  {
    name: "tuple-struct-default-as-property",
    breaks: "a tuple struct with the container's `#[serde(default)]`, read from a shorter array, gives `defaults._1` of the array its default is, `undefined`, for each missing element (#1537)",
    file: "src/lower/serde/de.rs",
    find: "            Shape::Array(_) => self.project(defaults, ty, i),\n",
    replace: "            Shape::Array(_) => Expr::member(defaults, key.to_owned()),\n",
    tests: [
      "test/serde-attributes.test.ts",
      "-t",
      "container default gives each missing"
    ]
  },
  {
    name: "newtype-struct-reader-unasked",
    breaks: "a newtype struct's field that names its own reader is read as its type is, `\"3\"` refused (#1536)",
    file: "src/lower/serde/de.rs",
    find: "                let read = self.field_reader(field.did, &attrs, self.field_ty(field, args), span)?;\n",
    replace: "                let read = self.json_reader(self.field_ty(field, args), span)?;\n",
    tests: [
      "test/serde-attributes.test.ts",
      "-t",
      "own writer and reader are used"
    ]
  },
  {
    name: "flatten-reader-unasked",
    breaks: "a flattened field's own reader is dropped, its type's entries read (#1536)",
    file: "src/lower/serde/de.rs",
    find: "                flattened.push(self.field_reader(field.did, &attrs, field_ty, span)?);\n",
    replace: "                flattened.push(self.json_reader(field_ty, span)?);\n",
    tests: [
      "test/serde-attributes.test.ts",
      "-t",
      "own writer and reader are used"
    ]
  },
  {
    name: "newtype-variant-reader-unasked",
    breaks: "a newtype variant's own reader, or its field's, is dropped: `{\"A\":\"7\"}` refused (#1536)",
    file: "src/lower/serde/de.rs",
    find: "                    let read = match codec {\n                        Some(codec) => codec,\n",
    replace: "                    let read = match codec {\n                        Some(_) => self.json_reader(field_ty, span)?,\n",
    tests: [
      "test/serde-attributes.test.ts",
      "-t",
      "own writer and reader are used"
    ]
  },
  {
    name: "dropping-read-unrefused",
    breaks: "a read of what runs code when it's dropped is let through, and one that fails part-way drops nothing of what it had read, where serde_json's `?` drops each (#1552)",
    file: "src/lower/serde/de.rs",
    find: "        match self.drop_seen(ty, 0) {\n",
    replace: "        match false {\n",
    tests: ["test/diagnostics.test.ts", "-t", "runs code when it's dropped"],
  },
  {
    name: "quiet-drop-taken-as-seen",
    breaks: "a read of a type whose `Drop` does nothing is refused as one whose drop can be seen (#1552)",
    file: "src/lower/serde/de.rs",
    find: "                        .is_some_and(|d| !self.krate.quiet_fns.contains(&d.did))\n",
    replace: "                        .is_some()\n",
    tests: ["test/serde.test.ts", "-t", "a value with a destructor"],
  },
  {
    name: "std-container-drop-taken-as-seen",
    breaks: "a read of a std container of what drops quietly, a `Vec<Item>` whose `Drop` does nothing, is refused as one whose drop can be seen (#1552)",
    file: "src/lower/serde/de.rs",
    find: "if crate::lower::recognition::is_std_crate(tcx, adt.did()) => args.types().any(seen),",
    replace: "if crate::lower::recognition::is_std_crate(tcx, adt.did()) => true,",
    tests: ["test/serde.test.ts", "-t", "a value with a destructor"],
  },
];
