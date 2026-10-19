// Mutations of src/lower/serde/protocol.rs (ADR 0459).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "serde-field-keyless",
    breaks: "a hand-written struct's `serialize_field` writes its value with no key",
    file: "src/lower/serde/protocol.rs",
    find: "            SerdeOp::Field => {\n                out.push(step(&json, \"key\", vec![values.remove(0)]));\n",
    replace: "            SerdeOp::Field => {\n                values.remove(0);\n",
    tests: ["test/serde.test.ts", "-t", "hand-written Serialize"],
  },
  {
    name: "serde-compound-unclosed",
    breaks: "a hand-written compound's `end()` writes no closing bracket",
    file: "src/lower/serde/protocol.rs",
    find: "                    SerdeCompound::Map | SerdeCompound::Struct => \"endObject\",\n                };\n                out.push(step(&json, method, Vec::new()));\n",
    replace: "                    SerdeCompound::Map | SerdeCompound::Struct => \"endObject\",\n                };\n                let _ = method;\n",
    tests: ["test/serde.test.ts", "-t", "hand-written Serialize"],
  },
  {
    name: "serde-compound-box-unread",
    breaks: "a compound's `&mut self`, a box in generic code, is taken as the writer, and JS throws",
    file: "src/lower/serde/protocol.rs",
    find: "                Expr::held(values.remove(0))\n",
    replace: "                values.remove(0)\n",
    tests: ["test/serde.test.ts", "-t", "hand-written Serialize"],
  },
  {
    name: "serde-f32-as-f64",
    breaks: "`serialize_f32(0.1)` writes its `f64` digits, 0.10000000149011612",
    file: "src/lower/serde/protocol.rs",
    find: "                    given.push(Expr::bool(true));\n",
    replace: "",
    tests: ["test/serde.test.ts", "-t", "hand-written Serialize"],
  },
  {
    name: "serde-deserialize-unmade",
    breaks: "`T::deserialize(d)` in a hand-written impl is what's read, not a `Result` of it, and `?` of it throws",
    file: "src/lower/serde/protocol.rs",
    find: "            return Ok(Expr::call(Expr::helper(\"$jsonRead\"), vec![values.remove(0), reader]));\n",
    replace: "            return Ok(Expr::call(reader, vec![values.remove(0)]));\n",
    tests: ["test/serde.test.ts", "-t", "hand-written Deserialize"],
  },
];
