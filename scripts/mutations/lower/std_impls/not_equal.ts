// Mutations of src/lower/std_impls/not_equal.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

const own = ["test/corpus.test.ts", "-t", "^own_ne_max_min_clamp.rs$"];

export const mutations: Mutation[] = [
  {
    name: "ne-of-a-param-is-not-eq",
    breaks: "a type parameter's `!=` is `!TPartialEq.eq(a, b)`, where Rust calls its `ne`, an impl's own (#962)",
    file: "src/lower/std_impls/not_equal.rs",
    find: "        if self.is_unknown(ty) {\n            return true;\n        }\n",
    replace: "        if self.is_unknown(ty) {\n            return false;\n        }\n",
    tests: own,
  },
  {
    name: "tuple-ne-not-its-parts",
    breaks: "a tuple's `!=` is `!(a == b)`, where std's asks each part's `!=`, an impl's own `ne` (#962)",
    file: "src/lower/std_impls/not_equal.rs",
    find: "            ty::Tuple(parts) => parts.iter().any(|part| self.own_ne_in(part, seen)),\n",
    replace: "            ty::Tuple(_) => false,\n",
    tests: own,
  },
  {
    name: "own-ne-unwritten",
    breaks: "an impl's own `ne` is taken to be std's `!eq`, so `!=` through a reference calls its `eq` (#962)",
    file: "src/lower/std_impls/not_equal.rs",
    find: "            ty::Adt(..) if let Some(imp) = self.partial_eq_impl(ty) => self.writes_ne(imp),\n",
    replace: "            ty::Adt(..) if let Some(imp) = self.partial_eq_impl(ty) => self.writes_ne(imp) && false,\n",
    tests: own,
  },
  {
    name: "rc-ne-not-its-value",
    breaks: "an `Rc`'s `!=` is `!(a == b)`, where std's asks its value's `!=`, an impl's own `ne` (#962)",
    file: "src/lower/std_impls/not_equal.rs",
    find: "                self.own_ne_in(pointee, seen)\n",
    replace: "                false\n",
    tests: own,
  },
  {
    name: "box-ne-not-its-value",
    breaks: "a `Box`'s `!=` is `!(a == b)`, where std's asks its value's `!=`, an impl's own `ne` (#962)",
    file: "src/lower/std_impls/not_equal.rs",
    find: "            ty::Adt(_, args) if ty.is_box() => self.own_ne_in(args.type_at(0), seen),\n",
    replace: "            ty::Adt(_, args) if ty.is_box() => false && self.own_ne_in(args.type_at(0), seen),\n",
    tests: own,
  },
  {
    name: "rc-ne-not-by-allocation-first",
    breaks: "an `Rc` of an `Eq` value's `!=` asks its value's `ne` of the one allocation, where `RcEqIdent` says they're equal first (#962)",
    file: "src/lower/std_impls/not_equal.rs",
    find: "                        Expr::bin(Op::And, Expr::unary(js::UnaryOp::Not, same), ne)\n",
    replace: "                        Expr::bin(Op::Or, Expr::bin(Op::And, same, Expr::bool(false)), ne)\n",
    tests: own,
  },
  {
    name: "tuple-ne-every-part",
    breaks: "a tuple's parts' `!=` are each `&&`, so it's `true` only where all of them differ (#962)",
    file: "src/lower/std_impls/not_equal.rs",
    find: "                        Some(any) => Expr::bin(Op::Or, any, ne),\n",
    replace: "                        Some(any) => Expr::bin(Op::And, any, ne),\n",
    tests: own,
  },
  {
    name: "ne-between-own-ne-unasked",
    breaks: "an impl of `PartialEq<B>` for `A` that writes its own `ne` isn't asked it: `[A] == [B]` asks `eq` of each two, where std's slice `==` asks `!=` (a wrong answer, #970)",
    file: "src/lower/std_impls/not_equal.rs",
    find: "        self.recognition().user_impl(tr).is_some_and(|imp| self.writes_ne(imp))\n",
    replace: "        false\n",
    tests: ["test/corpus.test.ts", "-t", "^ne_between_two_item_types.rs$"],
  },
  {
    name: "ne-between-parameters-known",
    breaks: "generic `[T] == [U]` of `T: PartialEq<U>` asks the dictionary's `eq` of each two, where std's slice `==` asks its `ne` (a wrong answer, #970)",
    file: "src/lower/std_impls/not_equal.rs",
    find: "        if self.is_unknown(ty) || self.is_unknown(rhs) {\n            return true;\n        }\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^ne_between_two_item_types.rs$"],
  },
  {
    name: "pin-ne-unknown",
    breaks: "`!=` of two `Pin`s whose target writes its own `ne` is refused, where std's calls that `ne` (#1446)",
    file: "src/lower/std_impls/not_equal.rs",
    find: "            ty::Adt(..) if self.recognition().pinned(ty).is_some() => {\n                let (x, target) = self.pin_target(a, ty, span, out)?;\n                let (y, _) = self.pin_target(b, ty, span, out)?;\n                self.ne_value(",
    replace: "            ty::Adt(..) if false => {\n                let (x, target) = self.pin_target(a, ty, span, out)?;\n                let (y, _) = self.pin_target(b, ty, span, out)?;\n                self.ne_value(",
    tests: own,
  },
];
