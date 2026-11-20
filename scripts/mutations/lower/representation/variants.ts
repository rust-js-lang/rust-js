// Mutations of src/lower/representation/variants.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "numbered-variant-tested-as-number",
    breaks:
      "a test of a numbered enum's variant is of its number, `status === 0`, and a `switch`'s `case 0:`, where it's its `const`, `Pending`",
    file: "src/lower/representation/variants.rs",
    find: "            true => Expr::bin(Op::Eq, subject, self.unit_variant(adt.did(), def)),\n",
    replace:
      "            true => Expr::bin(Op::Eq, subject, bindings::variant_tag(self.tcx, def)),\n",
    tests: [
      "test/lowering-repr-enums.test.ts",
      "-t",
      "a repr enum's variants are its module's consts",
    ],
  },
  {
    name: "mir-untagged-variant-tagged",
    breaks: "an untagged enum's variant is tested by a tag it has none of",
    file: "src/lower/representation/variants.rs",
    find: "        if bindings::is_untagged(self.tcx, adt.did()) {\n            return self.untagged_variant_test(ty, &[def], &[], &subject, present);\n",
    replace:
      "        if false && bindings::is_untagged(self.tcx, adt.did()) {\n            return self.untagged_variant_test(ty, &[def], &[], &subject, present);\n",
    tests: ["test/corpus.test.ts", "-t", "untagged_enums"],
  },
  {
    name: "mir-otherwise-tag-of-its-own",
    breaks:
      "a discriminated union's `otherwise` is tested by a tag of its own, which no object has",
    file: "src/lower/representation/variants.rs",
    find: "            let key = bindings::tag_key(self.tcx, adt.did());\n            let tests = (adt.variants().iter())\n                .filter(|other| other.def_id != def.def_id)\n",
    replace:
      "            let key = bindings::tag_key(self.tcx, adt.did());\n            let tests = (adt.variants().iter())\n                .filter(|other| other.def_id == def.def_id)\n",
    tests: [
      "test/lowering-structs-enums.test.ts",
      "-t",
      "otherwise variant is any other",
    ],
  },
  {
    name: "write-result-variant-by-tag",
    breaks:
      "a `match` of an `io::Result<()>` reads its `TAG`, which `undefined` hasn't",
    file: "src/lower/representation/variants.rs",
    find: "        if self.is_write_result(ty) {\n            let op = if variant.as_u32() == 0 { Op::Eq } else { Op::Ne };\n",
    replace:
      "        if false {\n            let op = if variant.as_u32() == 0 { Op::Eq } else { Op::Ne };\n",
    tests: ["test/corpus.test.ts", "-t", "^io_unit_result.rs$"],
  },
  {
    name: "unit-literal-tag-tested",
    breaks:
      'a unit variant written in place is tested for a tag, `"RootDir".TAG === "Normal"`',
    file: "src/lower/representation/variants.rs",
    find: "            false if matches!(subject.kind, js::ExprKind::Str(_)) => Expr::bool(false),\n",
    replace: "",
    snapshots: true,
    tests: ["test/corpus.test.ts", "-t", "^path_components.rs$"],
  },
  {
    name: "variant-alternative-unplaced",
    breaks:
      "an enum's or-pattern's alternative isn't where it's written, so a comment before it is before the `if` (ADR 0444, amended)",
    file: "src/lower/representation/variants.rs",
    find: "                    Some(&span) if test.span.is_none() => Expr { span, ..test },\n",
    replace: "                    Some(_) if false => test,\n",
    tests: [
      "test/lowering-comments.test.ts",
      "-t",
      "a comment before an or-pattern's alternative",
    ],
  },
  {
    name: "keyed-otherwise-of-all",
    breaks: "a test of a keyed enum's variants, one without fields among them, leaves out the others' keys and its own alike: `matches!(k, Effect | Scope)` holds of neither (#504)",
    file: "src/lower/representation/variants.rs",
    find: "                .filter(|v| !variants.contains(v))\n",
    replace: "                .filter(|_| true)\n",
    tests: ["test/corpus.test.ts", "-t", "^flattened_keyed_enum.rs$"],
  },
  {
    name: "tagged-eq-tag",
    breaks:
      "`==` of a discriminated union's fields reads `TAG`, which it hasn't",
    file: "src/lower/representation/variants.rs",
    find: "                Expr::member(subject, bindings::tag_key(self.tcx, adt.did())),\n",
    replace: '                Expr::member(subject, "TAG"),\n',
    tests: ["test/lowering-structs-enums.test.ts", "-t", "discriminated union"],
  },
];
