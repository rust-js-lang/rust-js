// Mutations of src/lower/serde/value.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

// `dynamic.report`, against native Rust's, and its JS.
const report = ["test/compiler.test.ts", "-t", "dynamic.report matches native Rust"];

export const mutations: Mutation[] = [
  {
    name: "json-is-null-negated",
    breaks: "`is_null()` of a `Value` is true of what isn't `Null`",
    file: "src/lower/serde/value.rs",
    find: "            JsonMethod::IsNull => Expr::bin(Op::Eq, receiver.clone(), Expr::str(\"Null\")),",
    replace: "            JsonMethod::IsNull => Expr::bin(Op::Ne, receiver.clone(), Expr::str(\"Null\")),",
    tests: report,
  },
  {
    name: "json-display-never-pretty",
    breaks: "`{:#}` of a `Value` is its compact JSON, not its pretty",
    file: "src/lower/serde/value.rs",
    find: "Expr::call(Expr::helper(\"$jsonValueText\"), vec![value, Expr::bool(pretty)])",
    replace: "Expr::call(Expr::helper(\"$jsonValueText\"), vec![value, Expr::bool(false)])",
    tests: report,
  },
  {
    name: "json-equal-sides-swapped",
    breaks: "`x == value` compares as `value == x` whichever side the `Value` is, and evaluates its sides out of order",
    file: "src/lower/serde/value.rs",
    find: "                let eq = if value_first {\n                    self.json_value_eq(a, b, other, span)?\n                } else {\n                    self.json_value_eq(b, a, other, span)?\n                };",
    replace: "                let eq = if !value_first {\n                    self.json_value_eq(a, b, other, span)?\n                } else {\n                    self.json_value_eq(b, a, other, span)?\n                };",
    tests: ["test/serde.test.ts", "-t", "Value comparisons preserve operand order"],
  },
  {
    name: "json-default-not-null",
    breaks: "`Value::default()` is an empty string, not `Null`",
    file: "src/lower/serde/value.rs",
    find: "            JsonCall::Default => Ok(Expr::str(\"Null\")),",
    replace: "            JsonCall::Default => Ok(Expr::str(\"\")),",
    tests: report,
  },
  {
    name: "json-option-none-dropped",
    breaks: "`Value::from` of an `Option` maps its value, and `None` isn't `Null`",
    file: "src/lower/serde/value.rs",
    find: "                Expr::call(Expr::helper(\"$jsonOption\"), vec![value, arrow])",
    replace: "                Expr::call(Expr::member(value, \"map\"), vec![arrow])",
    tests: ["test/serde.test.ts", "-t", "nested Value conversions"],
  },
  {
    name: "json-get-mut-unhandled",
    breaks: "`get_mut` of a `Value` is `get`'s, of its cell, which has no entries",
    file: "src/lower/serde/value.rs",
    find: "                call(\"$jsonGetMut\", vec![key])\n",
    replace: "                call(\"$jsonGet\", vec![key])\n",
    tests: ["test/serde.test.ts", "-t", "changes to and from Null"],
  },
  {
    name: "json-as-mut-of-cell",
    breaks: "`as_array_mut()` of a `&mut Value` asks its cell, not what it holds",
    file: "src/lower/serde/value.rs",
    find: "                vec![Expr::held(receiver.clone()), Expr::str(tag)],\n",
    replace: "                vec![receiver.clone(), Expr::str(tag)],\n",
    tests: ["test/serde.test.ts", "-t", "changes to and from Null"],
  },
];
