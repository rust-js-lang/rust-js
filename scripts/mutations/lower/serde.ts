// Mutations of src/lower/serde.rs (ADR 0093).
import type { Mutation } from "../../mutations";

const examples = ["test/snapshots.test.ts", "-t", "wire|inbox|api"];
const renaming = ["test/serde.test.ts", "-t", "renaming rules of variants"];

export const mutations: Mutation[] = [
  {
    name: "serde-snake-unseparated",
    breaks: "`rename_all = \"snake_case\"` writes `TextMessage` as `textmessage`, not `text_message`",
    file: "src/lower/serde.rs",
    find: "                        snake.push('_');\n",
    replace: "",
    tests: renaming,
  },
  {
    name: "serde-uppercase-kept",
    breaks: "`rename_all = \"UPPERCASE\"` writes a variant as it's named",
    file: "src/lower/serde.rs",
    find: "            Rule::Upper => variant.to_ascii_uppercase(),\n",
    replace: "            Rule::Upper => variant.to_owned(),\n",
    tests: renaming,
  },
  {
    name: "serde-struct-tag-unwritten",
    breaks: "a struct's `#[serde(tag = \"kind\")]` writes no `kind` key",
    file: "src/lower/serde.rs",
    find: "                .unwrap_or_else(|| self.tcx.item_name(adt.did()).to_string());\n            self.emit(json, \"key\", vec![Expr::str(tag.as_str())], out);\n",
    replace: "                .unwrap_or_else(|| self.tcx.item_name(adt.did()).to_string());\n",
    tests: renaming,
  },
  {
    name: "serde-camel-case-unknown",
    breaks: "`rename_all = \"camelCase\"` isn't known, and its fields are written as they're named",
    file: "src/lower/serde.rs",
    find: "            \"camelCase\" => Rule::Camel,\n",
    replace: "",
    tests: examples,
  },
  {
    name: "serde-variant-by-its-own-test",
    breaks: "a codec tests an enum's variant by `TAG` and its Rust name, not as a pattern does: a tagged one's, `{ kind: \"On\" }`, is none of them",
    file: "src/lower/serde.rs",
    find: "                    let test = self.variant_test(value.clone(), Ty::new_adt(self.tcx, adt, args), adt, index, false);\n",
    replace: "                    let _ = index;\n                    let test = Expr::bin(Op::Eq, Expr::member(value.clone(), \"TAG\"), Expr::str(variant.name.to_string()));\n",
    tests: ["test/serde.test.ts", "-t", "discriminated union"],
  },
  {
    name: "hand-written-codec-unadapted",
    breaks: "a hand-written impl's function, which gives a `Result`, is called as a codec, and what's written is the `Result`",
    file: "src/lower/serde.rs",
    find: "            false => Expr::call(Expr::helper(\"$jsonCodec\"), vec![callee]),\n",
    replace: "            false => callee,\n",
    tests: ["test/serde.test.ts", "-t", "hand-written"],
  },
  {
    name: "serde-with-unread",
    breaks: "`#[serde(with = \"module\")]` is refused as an attribute rust-js doesn't know",
    file: "src/lower/serde.rs",
    find: "                \"with\" => (attrs.serialize_with, attrs.deserialize_with) = (Some(\"with\"), Some(\"with\")),\n",
    replace: "",
    tests: ["test/serde.test.ts", "-t", "its own functions"],
  },
  {
    name: "serde-field-written-as-its-type",
    breaks: "a field with its own writer is written as its type is",
    file: "src/lower/serde.rs",
    find: "                self.write_field(codec, value.clone(), json, written, span, &mut body)?;\n",
    replace: "                self.write_field(None, value.clone(), json, written, span, &mut body)?;\n",
    tests: ["test/serde.test.ts", "-t", "its own functions"],
  },
  {
    name: "serde-path-desugared",
    breaks: "the function a field's attribute names is taken as what its `?` desugars to, and refused",
    file: "src/lower/serde.rs",
    find: "                    && expr.span.desugaring_kind().is_none()\n",
    replace: "",
    tests: ["test/serde.test.ts", "-t", "its own functions"],
  },
];
