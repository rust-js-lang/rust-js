// Mutations of src/lower/combinators/values.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "inspect-holds-nothing",
    breaks: "`inspect`'s value is left undropped as its closure panics (#903)",
    file: "src/lower/combinators/values.rs",
    find: "self.call_may_unwind(f_ty));\n",
    replace: "false);\n",
    tests: ["test/corpus.test.ts", "-t", "^inspect_unwinds_its_value.rs$"],
  },
  {
    name: "vec-repeat-rows-shared",
    breaks:
      "`vec![vec![0; n]; n]` is one row `n` times, `new Array(n).fill(row)`, so a write to one row is a write to all",
    file: "src/lower/combinators/values.rs",
    find: "        let fills = !self.needs_clone(item_ty) && drop.is_none();\n",
    replace:
      "        let fills = true || !self.needs_clone(item_ty) && drop.is_none();\n",
    tests: [
      "test/compiler.test.ts",
      "-t",
      "numbers are Math's, operators call their impl, and vec!\\[x; n\\] fills",
    ],
  },
  {
    name: "option-xor-both",
    breaks: "`Some(a).xor(Some(b))` is `Some(a)`",
    file: "src/lower/combinators/values.rs",
    find: "                Expr::cond(a_some.clone(), Expr::cond(b_none, a, Expr::js_undefined()), b)\n",
    replace: "                Expr::cond(a_some.clone(), a, b)\n",
    tests: ["test/corpus.test.ts", "-t", "option_result_methods"],
  },
  {
    name: "given-up-undropped",
    breaks: "`is_ok_and` of an `Err` with a destructor leaves it undropped",
    file: "src/lower/combinators/values.rs",
    find: "        dropped.extend(given_up_dropped);\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "combinators_owned_drops"],
  },
  {
    name: "as-deref-counted-whole",
    breaks: "`as_deref()` of a counted `Rc` gives the `Rc`",
    file: "src/lower/combinators/values.rs",
    find: "            return Ok(self.counted_pointee(value, pointee));",
    replace: "            return Ok(value);",
    tests: ["test/corpus.test.ts", "-t", "as_deref_counted"],
  },
  {
    name: "as-deref-own-skipped",
    breaks: "`as_deref()` of a type with its own `Deref` is refused",
    file: "src/lower/combinators/values.rs",
    find: "        if self.has_user_impl(deref, ty) {",
    replace: "        if false {",
    tests: ["test/corpus.test.ts", "-t", "^as_deref\\.rs"],
  },
  {
    name: "result-or-err-undropped",
    breaks: "`Err(e).or(r)` of a `Result` leaves `e` undropped",
    file: "src/lower/combinators/values.rs",
    find: "        if self.drops(given_up.1) == Drops::Nothing {\n            return Ok(Expr::cond(test, kept, otherwise));\n",
    replace:
      "        if true {\n            return Ok(Expr::cond(test, kept, otherwise));\n",
    tests: ["test/corpus.test.ts", "-t", "^unused_value_drops.rs$"],
  },
  {
    name: "xor-both-undropped",
    breaks: "`Some(a).xor(Some(b))` leaves `a` and `b` undropped",
    file: "src/lower/combinators/values.rs",
    find: "            out.push(StmtKind::If(both, dropped, None).at(js_span));\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^unpaired_option_drops.rs$"],
  },
  {
    name: "zip-lone-undropped",
    breaks: "`Some(a).zip(None)` leaves `a` undropped",
    file: "src/lower/combinators/values.rs",
    find: "            out.push(StmtKind::If(only_a, dropped_a, None).at(js_span));\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^unpaired_option_drops.rs$"],
  },
  {
    name: "zip-other-lone-undropped",
    breaks: "`None.zip(Some(b))` leaves `b` undropped",
    file: "src/lower/combinators/values.rs",
    find: "            out.push(StmtKind::If(only_b, dropped_b, None).at(js_span));\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^unpaired_option_drops.rs$"],
  },
  {
    name: "xor-cleanup-skipped-on-panic",
    breaks:
      "where `xor`'s first dropped value's destructor panics, the second is never dropped",
    file: "src/lower/combinators/values.rs",
    find: "            let dropped =\n                crate::lower::drops::each_dropped(vec![dropped_a, dropped_b], js_span, &mut self.catch_names());\n",
    replace: "            let dropped = [dropped_a, dropped_b].concat();\n",
    tests: ["test/corpus.test.ts", "-t", "^xor_drop_panics.rs$"],
  },
  {
    name: "or-dropped-otherwise-unguarded",
    breaks:
      "the variant a call gives up isn't dropped as what it gives otherwise unwinds, `unwrap_or_default`'s `Err` as its `T::default()` panics (#903)",
    file: "src/lower/combinators/values.rs",
    find: "        let mut dropped = match otherwise.may_throw() {\n",
    replace: "        let mut dropped = match false {\n",
    tests: ["test/corpus.test.ts", "-t", "^inspect_unwinds_its_value.rs$"],
  },
  {
    name: "or-dropped-before-otherwise",
    breaks:
      "the variant a call gives up is dropped before what it gives otherwise is made, a `Default`'s",
    file: "src/lower/combinators/values.rs",
    find: "        };\n        dropped.extend(given_up_dropped);\n",
    replace: "        };\n        dropped.splice(0..0, given_up_dropped);\n",
    tests: ["test/corpus.test.ts", "-t", "^default_and_pop_if_drops.rs$"],
  },
  {
    name: "repeat-drop-shared",
    breaks:
      "`vec![x; n]` of a value with a destructor fills one value into each slot, dropped once each",
    file: "src/lower/combinators/values.rs",
    find: "        let fills = !self.needs_clone(item_ty) && drop.is_none();\n",
    replace: "        let fills = !self.needs_clone(item_ty);\n",
    tests: ["test/corpus.test.ts", "-t", "^vec_repeat_drops.rs$"],
  },
  {
    name: "from-elem-item-made-after-room",
    breaks:
      "`vec![made(); n]` checks the room before `made()` runs, which a panic then skips",
    file: "src/lower/combinators/values.rs",
    find: '            if !item.reads_same() {\n                item = self.spill(Role::Item, item, out);\n            }\n            if !n.reads_same() {',
    replace:
      '            if false {\n                item = self.spill(Role::Item, item, out);\n            }\n            if !n.reads_same() {',
    tests: ["test/corpus.test.ts", "-t", "^capacity_overflow.rs$"],
  },
  {
    name: "from-elem-room-unchecked",
    breaks: "`vec![x; n]` past `isize::MAX` bytes is no panic",
    file: "src/lower/combinators/values.rs",
    find: "            self.room_for_count((n.clone(), known), size.clone(), span, out);\n",
    replace: "",
    tests: ["test/runtime-package.test.ts", "-t", "room past"],
  },
  {
    name: "repeat-dropping-of-no-size",
    breaks:
      "`vec![x; n]` of items with a destructor checks their room by their count alone",
    file: "src/lower/combinators/values.rs",
    find: "            let mut list = vec![item, n, size, Expr::arrow(vec![name.into()], body)];\n",
    replace:
      "            let mut list = vec![item, n, Expr::int(0), Expr::arrow(vec![name.into()], body)];\n",
    tests: ["test/runtime-package.test.ts", "-t", "room past"],
  },
  {
    name: "as-deref-of-a-guard-refused",
    breaks:
      "`as_deref()` of an `Option<Ref<u32>>`, which clippy's `option_as_ref_deref` asks for, is refused",
    file: "src/lower/combinators/values.rs",
    find: "        if crate::lower::recognition::is_cell_guard(self.tcx, ty) {\n            return Ok(self.guard_value(value, ty, false));\n        }\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^option_of_a_ref_as_deref.rs$"],
  },
  {
    name: "result-copied-of-mut-ref-gives-handle",
    breaks: "`copied()` of a `Result<&mut i32, E>` gives the handle on as it is, not its `value` (#1311)",
    file: "src/lower/combinators/values.rs",
    find: "        (comb == Comb::AsDeref || !self.mut_ref_form(pointee).read_is_itself()).then_some(pointee)",
    replace: "        (comb == Comb::AsDeref).then_some(pointee)",
    tests: ["test/corpus.test.ts", "-t", "^option_of_a_mut_ref_read_by_value.rs$"],
  },
];
