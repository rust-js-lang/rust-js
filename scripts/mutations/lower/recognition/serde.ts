// Mutations of src/lower/recognition/serde.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "other-derives-skipped",
    breaks: "what another crate's derive writes, strum's or thiserror's, is skipped as std's derives' is",
    file: "src/lower/recognition/serde.rs",
    find: "        true => tcx.is_builtin_derived(id),",
    replace: "        true => tcx.is_automatically_derived(id),",
    tests: ["test/cargo-workspace.test.ts","-t","another crate's derive"],
  },
  {
    name: "foreign-std-derives-unknown",
    breaks: "core's derived `Debug` of `IntErrorKind` isn't known as derived, and `{:?}` of one is an error",
    file: "src/lower/recognition/serde.rs",
    find: "        false => tcx.is_automatically_derived(id),",
    replace: "        false => false,",
    tests: ["test/corpus.test.ts", "-t", "int_error_kind"],
  },
  {
    name: "serde-de-custom-unknown",
    breaks: "`de::Error::custom(msg)` isn't known, and is refused",
    file: "src/lower/recognition/serde.rs",
    find: "\"ser\" | \"de\") => SerdeOp::Custom,",
    replace: "\"ser\") => SerdeOp::Custom,",
    tests: ["test/serde.test.ts", "-t", "hand-written Deserialize"],
  },
  {
    name: "serde-hint-number-unknown",
    breaks: "`deserialize_u64` of a visitor is refused",
    file: "src/lower/recognition/serde.rs",
    find: "            | \"deserialize_u16\" | \"deserialize_u32\" | \"deserialize_u64\" | \"deserialize_f32\" | \"deserialize_f64\" => {\n",
    replace: "            | \"deserialize_u16\" | \"deserialize_u32\" | \"deserialize_f32\" | \"deserialize_f64\" => {\n",
    tests: ["test/serde.test.ts", "-t", "hand-written Visitor"],
  },
  {
    name: "serde-next-entry-unknown",
    breaks: "`map.next_entry()` of a visitor is refused",
    file: "src/lower/recognition/serde.rs",
    find: "        (\"MapAccess\", \"next_entry\") => SerdeOp::Next(SerdeNext::Entry),\n",
    replace: "",
    tests: ["test/serde.test.ts", "-t", "sequence and a map"],
  },
  {
    name: "serde-tuple-field-keyed",
    breaks: "a tuple struct's `serialize_field(&v)` is taken as a struct's, of a key and a value",
    file: "src/lower/recognition/serde.rs",
    find: "            SerdeCompound::Struct | SerdeCompound::StructVariant => SerdeOp::Field,\n",
    replace: "            _ => SerdeOp::Field,\n",
    tests: ["test/serde.test.ts", "-t", "collects and writes variants"],
  },
];
