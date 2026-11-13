// Mutations of src/lower/serde.rs (ADR 0093).
import type { Mutation } from "../../mutations";

const examples = ["test/snapshots.test.ts", "-t", "wire|inbox|api"];
const renaming = ["test/serde-values.test.ts", "-t", "renaming rules of variants"];

export const mutations: Mutation[] = [
  {
    name: "serde-snake-unseparated",
    breaks:
      '`rename_all = "snake_case"` writes `TextMessage` as `textmessage`, not `text_message`',
    file: "src/lower/serde.rs",
    find: "                        snake.push('_');\n",
    replace: "",
    tests: renaming,
  },
  {
    name: "serde-uppercase-kept",
    breaks: '`rename_all = "UPPERCASE"` writes a variant as it\'s named',
    file: "src/lower/serde.rs",
    find: "            Rule::Upper => variant.to_ascii_uppercase(),\n",
    replace: "            Rule::Upper => variant.to_owned(),\n",
    tests: renaming,
  },
  {
    name: "serde-struct-tag-unwritten",
    breaks: 'a struct\'s `#[serde(tag = "kind")]` writes no `kind` key',
    file: "src/lower/serde.rs",
    find: '                .unwrap_or_else(|| self.tcx.item_name(adt.did()).to_string());\n            self.emit(json, "key", vec![Expr::str(tag.as_str())], out);\n',
    replace:
      "                .unwrap_or_else(|| self.tcx.item_name(adt.did()).to_string());\n",
    tests: renaming,
  },
  {
    name: "serde-camel-case-unknown",
    breaks:
      "`rename_all = \"camelCase\"` isn't known, and its fields are written as they're named",
    file: "src/lower/serde.rs",
    find: '            "camelCase" => Rule::Camel,\n',
    replace: "",
    tests: examples,
  },
  {
    name: "serde-variant-by-its-own-test",
    breaks:
      "a codec tests an enum's variant by `TAG` and its Rust name, not as a pattern does: a tagged one's, `{ kind: \"On\" }`, is none of them",
    file: "src/lower/serde.rs",
    find: "                    let test = self.variant_test(value.clone(), Ty::new_adt(self.tcx, adt, args), adt, index, false);\n",
    replace:
      '                    let _ = index;\n                    let test = Expr::bin(Op::Eq, Expr::member(value.clone(), "TAG"), Expr::str(variant.name.to_string()));\n',
    tests: ["test/serde-values.test.ts", "-t", "discriminated union"],
  },
  {
    name: "serde-with-unread",
    breaks:
      '`#[serde(with = "module")]` is refused as an attribute rust-js doesn\'t know',
    file: "src/lower/serde.rs",
    find: '                "with" => (attrs.serialize_with, attrs.deserialize_with) = (Some("with"), Some("with")),\n',
    replace: "",
    tests: ["test/serde-attributes.test.ts", "-t", "its own functions"],
  },
  {
    name: "serde-field-written-as-its-type",
    breaks: "a field with its own writer is written as its type is",
    file: "src/lower/serde.rs",
    find: "                self.write_field(codec, value.clone(), json, written, span, &mut body)?;\n",
    replace:
      "                self.write_field(None, value.clone(), json, written, span, &mut body)?;\n",
    tests: ["test/serde-attributes.test.ts", "-t", "its own functions"],
  },
  {
    name: "serde-path-desugared",
    breaks:
      "the function a field's attribute names is taken as what its `?` desugars to, and refused",
    file: "src/lower/serde.rs",
    find: "                    && expr.span.desugaring_kind().is_none()\n",
    replace: "",
    tests: ["test/serde-attributes.test.ts", "-t", "its own functions"],
  },
  {
    name: "serde-item-def-id-unfound",
    breaks:
      "an item's own `#[serde(..)]` is looked for among its fields' and variants' ids, where it isn't, so `rename_all` and `tag` are lost and the JSON is Rust's names, externally tagged (#450)",
    file: "src/lower/serde.rs",
    find: "                true => owner.def_id,\n",
    replace: "                true => *owner.node_id_to_def_id.get(&id)?,\n",
    tests: ["test/serde-attributes.test.ts", "-t", "under -C incremental"],
  },
  {
    name: "serde-field-def-id-unfound",
    breaks:
      "a field's or a variant's `#[serde(..)]` has no `DefId`, so `skip_serializing_if` and `rename` are lost (#450)",
    file: "src/lower/serde.rs",
    find: "                false => *owner.node_id_to_def_id.get(&id)?,\n",
    replace: "                false => return None,\n",
    tests: ["test/serde-attributes.test.ts", "-t", "under -C incremental"],
  },
  {
    name: "newtype-struct-writer-unasked",
    breaks: "a newtype struct's or a `transparent` one's field that names its own writer is written as its type is, `3` for `\"3\"` (#1536)",
    file: "src/lower/serde.rs",
    find: "                let codec = self.field_codec(field, &self.serde_attrs(field)?, true, span)?;\n",
    replace: "                let codec = None;\n",
    tests: [
      "test/serde-attributes.test.ts",
      "-t",
      "own writer and reader are used"
    ]
  },
  {
    name: "flatten-writer-unasked",
    breaks: "a flattened field's own writer is dropped: its type's entries are written, `{\"source\":7}` for `{\"wire\":7}` (#1536)",
    file: "src/lower/serde.rs",
    find: "                let writer = match codec {\n                    Some(codec) => codec,\n",
    replace: "                let writer = match codec {\n                    Some(_) => self.json_writer(*ty, span)?,\n",
    tests: [
      "test/serde-attributes.test.ts",
      "-t",
      "own writer and reader are used"
    ]
  },
  {
    name: "newtype-variant-writer-unasked",
    breaks: "a newtype variant's own writer, or its field's, is dropped: `{\"A\":7}` for `{\"A\":\"7\"}` (#1536)",
    file: "src/lower/serde.rs",
    find: "                    this.write_field(codec.clone(), fields[0].0.clone(), json, fields[0].1, span, out)\n",
    replace: "                    this.write_field(None, fields[0].0.clone(), json, fields[0].1, span, out)\n",
    tests: [
      "test/serde-attributes.test.ts",
      "-t",
      "own writer and reader are used"
    ]
  },
  {
    name: "variant-own-codec-unasked",
    breaks: "`#[serde(with)]` on a newtype variant is ignored, only its field's asked (#1536)",
    file: "src/lower/serde.rs",
    find: "                Some(_) => self.field_codec(variant.def_id, &attrs, serialize, span),\n",
    replace: "                Some(_) => Ok(None),\n",
    tests: [
      "test/serde-attributes.test.ts",
      "-t",
      "own writer and reader are used"
    ]
  },
  {
    name: "variant-own-codec-ignored-elsewhere",
    breaks: "`#[serde(with)]` on a variant of no field or of more is ignored, not refused (#1536)",
    file: "src/lower/serde.rs",
    find: "            _ => match own {\n",
    replace: "            _ => match None::<&str> {\n",
    tests: [
      "test/diagnostics.test.ts",
      "-t",
      "own writer|own reader"
    ]
  },
  {
    name: "internal-newtype-variant-writer-ignored",
    breaks: "an internally tagged newtype variant whose field names its own writer is written as its type is, not refused (#1536)",
    file: "src/lower/serde.rs",
    find: "        if codec.is_some() && matches!(tagging, Tagging::Internal(_)) {\n",
    replace: "        if false && codec.is_some() && matches!(tagging, Tagging::Internal(_)) {\n",
    tests: [
      "test/diagnostics.test.ts",
      "-t",
      "own writer|own reader"
    ]
  },
  {
    name: "skip-count-uncalled",
    breaks: "a crate's own skip predicate is called once, as a field's written, not first to count what's written: one that answers differently the second time writes `{\"n\":7}` for `{}` (#1543)",
    file: "src/lower/serde.rs",
    find: "            counts.push(StmtKind::Expr(test).at(js::Span::NONE));\n",
    replace: "            let _ = test;\n",
    tests: [
      "test/serde-attributes.test.ts",
      "-t",
      "skip predicate is called as"
    ]
  },
  {
    name: "skip-count-flattened",
    breaks: "a struct with a flattened field counts its fields' skip predicates, which serde's `serialize_map(None)` doesn't (#1543)",
    file: "src/lower/serde.rs",
    find: "        Ok(if flattened { Vec::new() } else { counts })\n",
    replace: "        Ok(counts)\n",
    tests: [
      "test/serde-attributes.test.ts",
      "-t",
      "skip predicate is called as"
    ]
  },
  {
    name: "skip-count-of-a-variant-unwritten",
    breaks: "a tuple or struct variant, other than adjacently tagged, doesn't count its skip predicates (#1543)",
    file: "src/lower/serde.rs",
    find: "        if !adjacent {\n            out.extend(counts.iter().cloned());\n",
    replace: "        if false {\n            out.extend(counts.iter().cloned());\n",
    tests: [
      "test/serde-attributes.test.ts",
      "-t",
      "skip predicate is called as"
    ]
  },
  {
    name: "skip-count-of-std-made",
    breaks: "std's skip predicates, which only ask what they're given, are counted too, a statement of no effect where the guide's `Order` has none (#1543)",
    file: "src/lower/serde.rs",
    find: "            if (self.resolved_skip(field.did)).is_some_and(|f| self.recognition().skip_predicate(f, *ty).is_some()) {\n",
    replace: "            if false {\n",
    tests: [
      "test/docs.test.ts"
    ],
    snapshots: true
  },
  {
    name: "tagged-optional-unasked",
    breaks: "an internally tagged newtype variant of an `Option` is written as what it holds, or as the tag alone, where serde returns `containing an optional` (#1540)",
    file: "src/lower/serde.rs",
    find: "        if self.tagged_optional(ty, span)? {\n",
    replace: "        if false && self.tagged_optional(ty, span)? {\n",
    tests: [
      "test/serde.test.ts",
      "-t",
      "TaggedSerializer writes it"
    ]
  },
  {
    name: "tagged-optional-not-through-a-box",
    breaks: "an `Option` a `Box` holds, an internally tagged newtype variant's payload, isn't serde's error (#1540)",
    file: "src/lower/serde.rs",
    find: "                self.tagged_optional(args.type_at(0), span)\n",
    replace: "                Ok(false)\n",
    tests: [
      "test/serde.test.ts",
      "-t",
      "TaggedSerializer writes it"
    ]
  },
  {
    name: "tagged-optional-not-through-a-newtype",
    breaks: "an `Option` a newtype or `transparent` struct holds, an internally tagged newtype variant's payload, isn't serde's error (#1540)",
    file: "src/lower/serde.rs",
    find: "                    return self.tagged_optional(field_ty, span);\n",
    replace: "                    return Ok(false);\n",
    tests: [
      "test/serde.test.ts",
      "-t",
      "TaggedSerializer writes it"
    ]
  },
  {
    name: "tagged-payload-of-a-parameter-written",
    breaks: "an internally tagged newtype variant of a type parameter is written, though its type can't say whether it's an `Option` (#1568)",
    file: "src/lower/serde.rs",
    find: "            _ if self.serde_evidence(ty, true).is_some() => refused(self, \"a type parameter\"),\n",
    replace: "            _ if self.serde_evidence(ty, true).is_some() => Ok(false),\n",
    tests: [
      "test/diagnostics.test.ts",
      "-t",
      "internally tagged newtype variant of a type parameter"
    ]
  },
  {
    name: "tagged-payload-by-hand-written",
    breaks: "an internally tagged newtype variant's payload written by hand is written by its own impl, whose `serialize_none` no writer tells from `()` (#1568)",
    file: "src/lower/serde.rs",
    find: "                if !super::recognition::serde_derived(self.tcx, self.tcx.parent(f)) || attrs.into {\n",
    replace: "                if attrs.into {\n",
    tests: [
      "test/diagnostics.test.ts",
      "-t",
      "internally tagged payload written by hand"
    ]
  },
  {
    name: "tagged-payload-through-into-written",
    breaks: "an internally tagged newtype variant's payload with `#[serde(into)]` is written, as its own fields, not what it converts into (#1568)",
    file: "src/lower/serde.rs",
    find: "                if !super::recognition::serde_derived(self.tcx, self.tcx.parent(f)) || attrs.into {\n",
    replace: "                if !super::recognition::serde_derived(self.tcx, self.tcx.parent(f)) {\n",
    tests: [
      "test/diagnostics.test.ts",
      "-t",
      "internally tagged payload written through into"
    ]
  },
  {
    name: "tagged-payload-field-writer-written",
    breaks: "an internally tagged newtype variant of a newtype struct whose field has its own writer is written as its type is (#1568)",
    file: "src/lower/serde.rs",
    find: "                    if field_attrs.serialize_with.is_some() {\n",
    replace: "                    if false {\n",
    tests: [
      "test/diagnostics.test.ts",
      "-t",
      "internally tagged newtype struct's field's own writer"
    ]
  },
  {
    name: "tagged-error-of-renamed-names",
    breaks: "an internally tagged newtype variant's error names its enum and variant as they're serialized, `Renamed::b`, where serde names them as they're written in Rust, `F::B` (#1540)",
    file: "src/lower/serde.rs",
    find: "                        let within = format!(\"{enum_name}::{}\", variant.name);\n",
    replace: "                        let within = format!(\"{}::{name}\", { let _ = enum_name; self.serde_attrs(self.tcx.parent(variant.def_id))?.rename.unwrap_or_default() });\n",
    tests: [
      "test/serde.test.ts",
      "-t",
      "TaggedSerializer writes it"
    ]
  },
];
