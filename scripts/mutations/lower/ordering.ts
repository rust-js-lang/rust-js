// Mutations of src/lower/ordering.rs (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "cmp-without-cells",
    breaks: "`refs.sort()` of a `Vec<&mut i32>` orders the cells, not what they point at",
    file: "src/lower/ordering.rs",
    find: "        let (a, _) = self.through_refs(a, ty);\n        let (b, ty) = self.through_refs(b, ty);\n",
    replace: "        let ty = ty.peel_refs();\n",
    tests: ["test/corpus.test.ts", "-t", "mut_ref_compare"],
  },
  {
    name: "wrapping-ordered-reversed",
    breaks: "`Wrapping`s order the other way round, as `Reverse`'s",
    file: "src/lower/ordering.rs",
    find: "                self.cmp_value(inside(a), inside(b), args.type_at(0), partial, span, out)",
    replace: "                self.cmp_value(inside(b), inside(a), args.type_at(0), partial, span, out)",
    tests: ["test/corpus.test.ts", "-t", "wrapping_type"],
  },
  {
    name: "text-literal-any-order",
    breaks: "a string compared with any literal is JS's `<`, which orders 🦀 before a full-width `！`",
    file: "src/lower/ordering.rs",
    find: "(c as u32) < 0xd800",
    replace: "(c as u32) < 0x110000",
    tests: ["test/corpus.test.ts", "-t", "code_point_order"],
  },
  {
    name: "text-operator-by-units",
    breaks: "`a < b` of two strings is JS's `<`, by UTF-16 units",
    file: "src/lower/ordering.rs",
    find: "            return Ok(Some(self.text_compare(op, a, b)));",
    replace: "            return Ok(Some(Expr::bin(op, a, b)));",
    tests: ["test/corpus.test.ts", "-t", "code_point_order"],
  },
  {
    name: "partial-parts-joined-by-or",
    breaks: "a derived `partial_cmp` of `f64` parts joins them with `||`, so a `NaN` part, unordered, goes on to the next part, not `None`",
    file: "src/lower/ordering.rs",
    find: "                let total = !partial || parts.iter().all(|&(_, _, t)| self.is_total(t));",
    replace: "                let total = true || !partial || parts.iter().all(|&(_, _, t)| self.is_total(t));",
    tests: ["test/compiler.test.ts", "-t", "PartialOrd and Ord compare with \\$cmp, a hand-written cmp, or the parts in turn"],
  },
  {
    name: "pin-cmp-unknown",
    breaks: "`<` of pinned `Box`es is an error",
    file: "src/lower/ordering.rs",
    find: "            ty::Adt(_, args) if ty.is_box() || self.is_rc(ty) || self.recognition().pinned(ty).is_some() => {",
    replace: "            ty::Adt(_, args) if ty.is_box() || self.is_rc(ty) => {",
    tests: ["test/corpus.test.ts", "-t", "pin_box"],
  },
  {
    "name": "enum-order-refused",
    "breaks": "a derived order of an enum with fields is refused",
    "file": "src/lower/ordering.rs",
    "find": "                if adt.is_enum() && adt.did().is_local() && !bindings::is_untagged(self.tcx, adt.did()) =>",
    "replace": "                if false =>",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "a derived order of an enum with fields"
    ]
  },
  {
    "name": "enum-order-variants-only",
    "breaks": "a derived order of an enum with fields compares its variants alone, `Rect(2, 3)` equal to `Rect(1, 9)`",
    "file": "src/lower/ordering.rs",
    "find": "                Ok(Expr::bin(Op::Or, order, same))",
    "replace": "                let _ = same;\n                Ok(order)",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "a derived order of an enum with fields"
    ]
  },
  {
    name: "cow-cmp-refused",
    breaks: "ordering `Cow`s is refused, where it's what they borrow",
    file: "src/lower/ordering.rs",
    find: "            ty::Adt(..) if let Some((x, borrowed)) = self.cow_deref(&a, ty) => {\n                let y",
    replace: "            ty::Adt(..) if let Some((x, borrowed)) = self.cow_deref(&a, ty).filter(|_| false) => {\n                let y",
    tests: ["test/corpus.test.ts", "-t", "^cow_compare.rs$"],
  },
  {
    name: "path-cmp-refused",
    breaks: "ordering paths is refused, where it's their components'",
    file: "src/lower/ordering.rs",
    find: "        if self.recognition().is_path(ty) {\n            return Ok(Expr::call(Expr::helper(\"$pathCmp\"), vec![a, b]));\n",
    replace: "        if false {\n            return Ok(Expr::call(Expr::helper(\"$pathCmp\"), vec![a, b]));\n",
    tests: ["test/corpus.test.ts", "-t", "^path_compare.rs$"],
  },
  {
    name: "unit-struct-unordered",
    breaks: "a struct of no fields, `struct Zst;`, isn't compared, and a map keyed by one is an error",
    file: "src/lower/ordering.rs",
    find: "            _ if self.one_value(ty) && !a.has_effects() && !b.has_effects() => Ok(Expr::int(0)),\n",
    replace: "            _ if false && self.one_value(ty) && !a.has_effects() && !b.has_effects() => Ok(Expr::int(0)),\n",
    tests: ["test/corpus.test.ts", "-t", "^unit_struct_order.rs$"],
  },
  {
    name: "unit-unordered",
    breaks: "`()`s aren't compared, and a heap of them is an error",
    file: "src/lower/ordering.rs",
    find: "        ty.is_unit()\n            || matches!",
    replace: "        false\n            || matches!",
    tests: ["test/corpus.test.ts", "-t", "^collection_capacity.rs$"],
  },
];
