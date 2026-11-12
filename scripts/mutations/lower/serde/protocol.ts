// Mutations of src/lower/serde/protocol.rs (ADR 0459).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "serde-field-keyless",
    breaks:
      "a hand-written struct's `serialize_field` writes its value with no key",
    file: "src/lower/serde/protocol.rs",
    find: '            SerdeOp::Field => {\n                out.push(step(&json, "key", vec![values.remove(0)]));\n',
    replace:
      "            SerdeOp::Field => {\n                values.remove(0);\n",
    tests: ["test/serde.test.ts", "-t", "hand-written Serialize"],
  },
  {
    name: "serde-compound-unclosed",
    breaks: "a hand-written compound's `end()` writes no closing bracket",
    file: "src/lower/serde/protocol.rs",
    find: '                    SerdeCompound::Map | SerdeCompound::Struct | SerdeCompound::StructVariant => "endObject",\n                };\n                out.push(step(&json, method, Vec::new()));\n                // A variant\'s',
    replace:
      '                    SerdeCompound::Map | SerdeCompound::Struct | SerdeCompound::StructVariant => "endObject",\n                };\n                let _ = method;\n                // A variant\'s',
    tests: ["test/serde.test.ts", "-t", "hand-written Serialize"],
  },
  {
    name: "serde-compound-box-unread",
    breaks:
      "a compound's `&mut self`, a box in generic code, is taken as the writer, and JS throws",
    file: "src/lower/serde/protocol.rs",
    find: "                Expr::held(values.remove(0))\n",
    replace: "                values.remove(0)\n",
    tests: ["test/serde.test.ts", "-t", "hand-written Serialize"],
  },
  {
    name: "serde-f32-as-f64",
    breaks: "`serialize_f32(0.1)` writes its `f64` digits, 0.10000000149011612",
    file: "src/lower/serde/protocol.rs",
    find: "                    SerdeScalar::Float { single: true } => given.push(Expr::bool(true)),\n",
    replace: "",
    tests: ["test/serde.test.ts", "-t", "hand-written Serialize"],
  },
  {
    name: "serde-deserialize-unmade",
    breaks:
      "`T::deserialize(d)` in a hand-written impl is what's read, not a `Result` of it, and `?` of it throws",
    file: "src/lower/serde/protocol.rs",
    find: '            return Ok(Written::Value(Expr::call(\n                Expr::helper("$jsonRead"),\n                vec![values.remove(0), reader],\n            )));\n',
    replace: "            return Ok(Written::Value(Expr::call(reader, vec![values.remove(0)])));\n",
    tests: ["test/serde.test.ts", "-t", "hand-written Deserialize"],
  },
  {
    name: "serde-visit-ignores-hint",
    breaks:
      "`deserialize_u64` of a visitor given a string calls its `visit_str`, where serde_json says it's an invalid type",
    file: "src/lower/serde/protocol.rs",
    find: '                (VisitorMethod::Str, SerdeHint::Any | SerdeHint::Str) => "str",\n',
    replace: '                (VisitorMethod::Str, _) => "str",\n',
    tests: ["test/serde.test.ts", "-t", "hand-written Visitor"],
  },
  {
    name: "serde-visit-borrowed-as-copied",
    breaks:
      "a borrowed string is given to a visitor's `visit_str`, not its `visit_borrowed_str`",
    file: "src/lower/serde/protocol.rs",
    find: '                (VisitorMethod::BorrowedStr, SerdeHint::Any | SerdeHint::Str) => "borrowedStr",\n',
    replace: "",
    tests: ["test/serde.test.ts", "-t", "hand-written Visitor"],
  },
  {
    name: "serde-invalid-value-as-type",
    breaks: '`invalid_value` says "invalid type"',
    file: "src/lower/serde/protocol.rs",
    find: '                    "invalid value: "\n',
    replace: '                    "invalid type: "\n',
    tests: ["test/serde.test.ts", "-t", "hand-written Visitor"],
  },
  {
    name: "serde-expected-unvisited",
    breaks: "what a visitor's error says it expected, `&self`, is refused",
    file: "src/lower/serde/protocol.rs",
    find: "            Some((_, f)) => Ok(Expr::call(self.fn_ref(f), vec![value])),\n",
    replace:
      '            Some(_) => Err(self.unsupported(span, "what\'s expected")),\n',
    tests: ["test/serde.test.ts", "-t", "hand-written Visitor"],
  },
  {
    name: "serde-struct-hint-map-only",
    breaks:
      "`deserialize_struct` of a sequence isn't given to the visitor's `visit_seq`, where serde_json gives it",
    file: "src/lower/serde/protocol.rs",
    find: '                (VisitorMethod::Seq, SerdeHint::Any | SerdeHint::Seq | SerdeHint::Struct) => "seq",\n',
    replace:
      '                (VisitorMethod::Seq, SerdeHint::Any | SerdeHint::Seq) => "seq",\n',
    tests: ["test/serde.test.ts", "-t", "sequence and a map"],
  },
  {
    name: "serde-next-unboxed-access",
    breaks:
      "`seq.next_element()`'s `&mut self`, a box in generic code, is taken as the access, and JS throws",
    file: "src/lower/serde/protocol.rs",
    find: "            let access = Expr::held(values.remove(0));\n",
    replace: "            let access = values.remove(0);\n",
    tests: ["test/serde.test.ts", "-t", "sequence and a map"],
  },
  {
    name: "serde-hint-errors-as-any",
    breaks:
      "`deserialize_map` of a sequence errs where it's read, not where it's peeked at, as serde_json does",
    file: "src/lower/serde/protocol.rs",
    find: '            _ => "$jsonVisitHint",\n',
    replace: '            _ => "$jsonVisit",\n',
    tests: ["test/serde.test.ts", "-t", "sequence and a map"],
  },
  {
    name: "serde-variant-unnamed",
    breaks:
      "`serialize_struct_variant` writes the variant's fields without the object of its name",
    file: "src/lower/serde/protocol.rs",
    find: '                    out.push(step(&json, "key", vec![values.remove(2)]));\n',
    replace: "                    values.remove(2);\n",
    tests: ["test/serde.test.ts", "-t", "collects and writes variants"],
  },
  {
    name: "serde-collect-map-keys-unwritten",
    breaks:
      "`collect_map` writes its keys as they are, a number where JSON's is text",
    file: "src/lower/serde/protocol.rs",
    find: "                let text = self.json_key(Expr::var(&key), own(1), span)?;\n",
    replace: "                let text = Expr::var(&key);\n",
    tests: ["test/serde.test.ts", "-t", "collects and writes variants"],
  },
  {
    name: "serde-visit-none-given-a-value",
    breaks:
      "a visitor's `visit_none` is called with a value, which its function takes as nothing, so it's the same",
    file: "src/lower/serde/protocol.rs",
    find: "                VisitorMethod::Unit | VisitorMethod::None => Vec::new(),\n",
    replace: "",
    tests: ["test/serde.test.ts", "-t", "reads an option"],
  },
  {
    name: "inspected-protocol-result-thrown",
    breaks: "a protocol call whose `Result` an impl looks at, `match seq.serialize_element(&x)`, is written as one given on by `?`, its error thrown past the `Err` arm (#1545)",
    file: "src/lower/serde/protocol.rs",
    find: "        if reads || used == ResultUse::Propagated {\n",
    replace: "        if true {\n",
    tests: ["test/serde-protocol.test.ts"],
  },
  {
    name: "inspected-protocol-writer-unrewound",
    breaks: "a protocol call whose error an impl catches leaves its writer where it failed, so a `Value` built after it nests in the failed one (#1545)",
    file: "src/lower/serde/protocol.rs",
    find: "Expr::arrow(Vec::new(), steps), writer]",
    replace: "Expr::arrow(Vec::new(), steps)]",
    tests: ["test/serde-protocol.test.ts"],
  },
  {
    name: "serde-expecting-asked-eagerly",
    breaks:
      "a hand-written visitor's `expecting` is called as its read starts, so a read that succeeds runs its side effects and its panic, which serde never asks it for (#1544)",
    file: "src/lower/serde/protocol.rs",
    find: "                    let call = Expr::call(self.fn_ref(def_id), vec![visitor.clone()]);\n                    expected = Some(Expr::arrow(\n                        Vec::new(),\n                        vec![StmtKind::Return(Some(call)).at(crate::js::Span::NONE)],\n                    ));\n",
    replace: "                    let call = Expr::call(self.fn_ref(def_id), vec![visitor.clone()]);\n                    expected = Some(Expr::arrow(\n                        Vec::new(),\n                        vec![StmtKind::Return(Some(call.clone())).at(crate::js::Span::NONE)],\n                    ));\n                    expected = Some(Expr::sequence(vec![call, expected.take().expect(\"made\")]));\n",
    tests: ["test/serde-protocol.test.ts", "-t", "expecting is asked only where an error says it"],
  },
  {
    name: "serde-visit-string-unwritten",
    breaks: "a hand-written visitor's `visit_string` isn't among its methods, so a string given whole goes to its `visit_str`, or is refused where it writes no other (#1546)",
    file: "src/lower/serde/protocol.rs",
    find: "                (VisitorMethod::String, SerdeHint::Any | SerdeHint::Str) => \"string\",\n",
    replace: "",
    tests: ["test/serde-protocol.test.ts", "-t", "a string is given to a visitor as serde_json gives it"],
  },
  {
    name: "serde-wide-int-ungiven",
    breaks: "a 128-bit integer's writer isn't told its width, so a `Value` holds `u128::MAX` and a flattened one says `got an integer` (#1547)",
    file: "src/lower/serde/protocol.rs",
    find: "                    SerdeScalar::Int { wide: Some(wide) } => given.push(Expr::str(wide)),\n",
    replace: "",
    tests: ["test/serde-protocol.test.ts", "-t", "a 128-bit integer is written by each serializer"],
  },
];
