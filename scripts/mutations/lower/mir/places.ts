// Mutations of crates/rust-js-mir/src/places.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [  {
    name: "nested-field-payload-boxed",
    breaks: "what a nested field's `Some` holds is read through the box, `$someValue(candidate.line)` (#470)",
    file: "crates/rust-js-mir/src/places.rs",
    find: "                && self.held_nested(state, place, at - 1)\n",
    replace: "                && at == 1\n                && self.held_nested(state, place, at - 1)\n",
    tests: ["test/lowering-nested-options.test.ts", "-t", "a dictionary's nullable Option of an Option"],
  },
  {
    name: "mir-residual-err-lost",
    breaks: "`?` of an `Err` returns nothing, not the `Err`",
    file: "crates/rust-js-mir/src/places.rs",
    find: "                    Some(_) => Expr::js_undefined(),\n                    None => tried,\n",
    replace:
      "                    Some(_) => Expr::js_undefined(),\n                    None => Expr::js_undefined(),\n",
    tests: ["test/corpus.test.ts", "-t", "question_early_returns"],
  },
  {
    name: "mir-handle-deref-unread",
    breaks:
      "`*h.count` of a handle a struct holds is the handle, not the number it's to",
    file: "crates/rust-js-mir/src/places.rs",
    find: "                if !std::mem::take(&mut placed) {\n                    value = Expr::held(value);\n                }\n",
    replace: "                let _ = std::mem::take(&mut placed);\n",
    tests: ["test/corpus.test.ts", "-t", "mut_number_handles"],
  },
  {
    name: "mir-cell-deref-skipped",
    breaks:
      "a reborrow through a handle on a `Box<dyn Trait>` is the handle, not the pair it holds",
    file: "crates/rust-js-mir/src/places.rs",
    find: "                            && !matches!(ty.ty.kind(), ty::Ref(_, inner, ty::Mutability::Mut) if self.mut_ref_form(*inner).is_cell()))\n",
    replace: "                            && true)\n",
    tests: ["test/corpus.test.ts", "-t", "dyn_mut"],
  },
  {
    name: "mir-union-field-read",
    breaks:
      "a union's field read is its object's member, `undefined` where it holds another",
    file: "crates/rust-js-mir/src/places.rs",
    find: "            (ty::Adt(adt, _), _) if adt.is_union() => self.union_read(base, ty.ty, field, span),\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "union_const"],
  },
  {
    name: "mir-index-from-end-front",
    breaks: "`[.., last]` of a slice reads its item counted from the front",
    file: "crates/rust-js-mir/src/places.rs",
    find: '                    None => Expr::bin(Op::Sub, Expr::member(base.clone(), "length"), Expr::int(offset.into())),\n',
    replace: "                    None => Expr::int(offset.into()),\n",
    tests: ["test/corpus.test.ts", "-t", "slice_patterns"],
  },
  {
    name: "mir-subslice-whole",
    breaks: "`[first, rest @ ..]`'s rest is the whole slice",
    file: "crates/rust-js-mir/src/places.rs",
    find: "                self.slice_part(base, start, range.next())\n",
    replace: "                { let _ = (start, range); base }\n",
    tests: ["test/corpus.test.ts", "-t", "slice_patterns"],
  },
  {
    name: "mir-mut-subslice-refused",
    breaks: "a `&mut` to a slice pattern's rest is refused, not a view",
    file: "crates/rust-js-mir/src/places.rs",
    find: '        Ok(Some(self.view_call("$view", list)))\n',
    replace: "        Ok(None)\n",
    tests: ["test/corpus.test.ts", "-t", "^slice_pattern_mut_rest.rs$"],
  },
  {
    name: "mir-named-local-in-place",
    breaks:
      "a temporary a branch's subject read by its name is written where it's read, so its name is never declared",
    file: "crates/rust-js-mir/src/places.rs",
    find: "                    state.named.insert(place.local);\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "nested_type_chain"],
  },
  {
    name: "mir-prop-default-unread",
    breaks:
      "a props field with a default, left out, reads `undefined` from MIR",
    file: "crates/rust-js-mir/src/places.rs",
    find: "            Some(default) => Expr::cond(Expr::bin(js::Op::Eq, read.clone(), Expr::js_undefined()), default, read),\n",
    replace: "            Some(_) => read,\n",
    tests: [
      "test/lowering-bindings.test.ts",
      "-t",
      "props field's default is read",
    ],
  },
  {
    name: "mir-bounds-write-read",
    breaks:
      "an element written, `xs[i] = v`, is checked as one read, `$index(xs, i) = v`",
    file: "crates/rust-js-mir/src/places.rs",
    find: "match write && at + 1 == last {",
    replace: "match false {",
    tests: ["test/lowering-numbers.test.ts", "-t", "nothing proves in bounds"],
  },
  {
    name: "mir-default-base-fields-held",
    breaks:
      "a field of an update's default base is read of an object held for it, `{ name: value.name, .. }`, not its default",
    file: "crates/rust-js-mir/src/places.rs",
    find: "        if let Some(defaults) = state.defaults.get(&place.local)\n",
    replace:
      "        if let Some(defaults) = state.defaults.get(&place.local).filter(|_| false)\n",
    tests: [
      "test/lowering-structs-enums.test.ts",
      "-t",
      "an update's default base is the defaults it reads",
    ],
  },
  {
    name: "mir-rest-field-read",
    breaks:
      "a `Rest` of props read as a field of its own, `p.rest`, is `undefined`, not refused",
    file: "crates/rust-js-mir/src/places.rs",
    find: "            PlaceElem::Field(_, field_ty) if bindings::is_rest(self.tcx, field_ty) => {",
    replace:
      "            PlaceElem::Field(_, field_ty) if false && bindings::is_rest(self.tcx, field_ty) => {",
    tests: [
      "test/jsx-children.test.ts",
      "test/jsx-evaluation-order.test.ts",
      "test/jsx-events.test.ts",
      "test/jsx-props.test.ts",
      "test/jsx-react-dev.test.ts",
      "test/jsx-types.test.ts",
      "test/jsx.test.ts",
    ],
  },
  {
    name: "mir-made-array-unsliced",
    breaks:
      "MIR refuses a slice pattern's rest of an array made where it's taken apart",
    file: "crates/rust-js-mir/src/places.rs",
    find: "            PlaceElem::ConstantIndex { from_end: true, .. } | PlaceElem::Subslice { .. } => {\n",
    replace:
      "            PlaceElem::ConstantIndex { from_end: true, .. } | PlaceElem::Subslice { .. } if false => {\n",
    tests: ["test/corpus.test.ts", "-t", "slice_pattern_of_made_array"],
  },
  {
    name: "pointed-local-read-whole",
    breaks: "a local a pointer's taken to reads as its cell, not its value",
    file: "crates/rust-js-mir/src/places.rs",
    find: "                        true => Expr::held(Expr::var(name)),\n",
    replace: "                        true => Expr::var(name),\n",
    tests: ["test/corpus.test.ts", "-t", "^raw_pointers.rs$"],
  },
  {
    name: "pointer-deref-unread",
    breaks: "`*p` of a pointer is the handle, not its place",
    file: "crates/rust-js-mir/src/places.rs",
    find: "            PlaceElem::Deref if self.is_handle_pointer(ty.ty) => self.pointee_place(base),\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^raw_pointers.rs$"],
  },
  {
    name: "write-result-field-of-err",
    breaks:
      "an `io::Result<()>`'s `Err`'s error isn't the result itself, but read as its `Ok`'s, `undefined`",
    file: "crates/rust-js-mir/src/places.rs",
    find: "self.variant_is_payload(*adt, adt.variant(variant)) || self.is_write_result(ty.ty) && variant.as_u32() != 0",
    replace: "self.variant_is_payload(*adt, adt.variant(variant))",
    tests: ["test/corpus.test.ts", "-t", "^io_unit_result.rs$"],
  },
  {
    name: "nested-option-payload-unboxed",
    breaks:
      "what a held `Option` of an `Option`'s `Some` holds is read through `$someValue`, a box it never is (ADR 1017)",
    file: "crates/rust-js-mir/src/places.rs",
    find: "            if at >= 1\n",
    replace: "            if at == usize::MAX\n",
    tests: ["test/corpus.test.ts", "-t", "^nested_option_held_as_payload.rs$"],
    snapshots: true,
  },
  {
    name: "cut-of-the-second-variant",
    breaks:
      "a `try_from`'s `Ok`, its first variant, isn't read as its cut, so its number is undeclared (#366)",
    file: "crates/rust-js-mir/src/places.rs",
    find: "            && variant == of\n",
    replace: "            && variant.as_u32() == 1\n",
    tests: ["test/corpus.test.ts", "-t", "^try_from_int_tested.rs$"],
  },
  {
    name: "one-variant-place-of-any-key",
    breaks: "fields of two variants at other keys, `Circle`'s `r` and `Ring`'s `_0`, are one JS place, a `Ring`'s read as `shape.r` (rd430)",
    file: "crates/rust-js-mir/src/places.rs",
    find: "            && variant_field(self.tcx, variant_a, a.as_usize()) == variant_field(self.tcx, variant_b, b.as_usize())\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "or_pattern_one_place"],
  },
  {
    name: "local-reference-not-through-a-box",
    breaks: "a place reached through a box of a reference written again, `&mut (**at).n` before `*at = b`, reads the new reference's (#980)",
    file: "crates/rust-js-mir/src/places.rs",
    find: "                Some(held) => ty = held,\n                None => return false,",
    replace: "                Some(_) => return false,\n                None => return false,",
    tests: ["test/corpus.test.ts", "-t", "^box_read_before_its_write.rs$"],
  },
  {
    name: "boxed-field-not-read-through",
    breaks: "a write through a struct's field held as generic code's box writes the box, `slot.at.x = ..`, not the object (#1405)",
    file: "crates/rust-js-mir/src/places.rs",
    find: "                        .is_some())\n",
    replace: "                        .is_some_and(|_| false))\n",
    tests: ["test/corpus.test.ts", "-t", "^mut_ref_fields_of_a_type_parameter.rs$"],
  },
  {
    name: "boxed-field-alias-not-followed",
    breaks: "`let r = &mut slot.at; *r = &mut d` writes the object into the field held as generic code's box, and `r.x` the box (#1405)",
    file: "crates/rust-js-mir/src/places.rs",
    find: "            && state.refs.contains_key(&place.local)\n",
    replace: "            && false\n",
    tests: ["test/corpus.test.ts", "-t", "^mut_ref_fields_of_a_type_parameter.rs$"],
  },
  {
    name: "dyn-place-through-a-call-named",
    breaks: "a `&mut dyn` of what a call lends, `get_mut()`'s, is given a handle on a MIR temporary JS never names, `$ptr(_31, ..)`, and throws a ReferenceError (#1453)",
    file: "crates/rust-js-mir/src/places.rs",
    find: "        if !rust_js_lower::pointers::within_items(self.tcx, state.body, &place) {\n            return Ok(made);\n        }\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^address_of_a_field_given_to_generic_code.rs$"],
  },
  {
    name: "dyn-of-a-whole-local-pointed",
    breaks: "a `&mut dyn` made of a whole local, `fmt::write(&mut s, ..)`'s `String`, in a crate that asks an address, is given a pointer to the variable, which JS holds as it is, and refused: \"a raw pointer to this place\" (#1893)",
    file: "crates/rust-js-mir/src/places.rs",
    find: "        if matches!(target.kind, js::ExprKind::Var(_)) {\n            return Ok(made);\n        }\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^dyn_of_a_whole_local_beside_an_address.rs$"],
  },
  {
    name: "placed-field-place-unkept",
    breaks:
      "a field held in a place of its own isn't kept as the cell it is, so a `&mut` of it is a copy into a new place and back, `const it = { value: pair.a.value }`, not the place, `pair.a` (#2255)",
    file: "crates/rust-js-mir/src/places.rs",
    find: "            if in_its_place\n",
    replace: "            if in_its_place && false\n",
    tests: ["test/corpus.test.ts", "-t", "^non_object_iterator_given_to_generic_code\\.rs$"],
    snapshots: true,
  },
  {
    name: "otherwise-field-read-as-its-key",
    breaks: "a match binding a discriminated union's `otherwise` payload, `Other(note) => note.text`, reads `._0` of the object itself, `undefined`, and throws a `TypeError`, a value made by the program and one serde read alike (#2396)",
    file: "crates/rust-js-mir/src/places.rs",
    find: "        self.variant_is_payload(*adt, adt.variant(variant)) || self.is_write_result(ty.ty) && variant.as_u32() != 0\n",
    replace: "        bindings::is_untagged(self.tcx, adt.did()) || self.is_write_result(ty.ty) && variant.as_u32() != 0\n",
    tests: ["test/corpus.test.ts", "-t", "^an_otherwise_variant_bound_by_its_match\\.rs$"],
  },
  {
    name: "held-ok-payload-unread",
    breaks: "an `Ok` made of a flattened state's held handle, `try_borrow_mut()`'s, has its payload read as `_6._0`, so `Cell::from_mut(&mut *state)` lends a handle on its `value` that assigns the variable holding the class's object (#2429)",
    file: "crates/rust-js-mir/src/places.rs",
    find: "            && let Some(handle) = self.locals.mir.made.held_ok(&state.locals.names[place.local])\n",
    replace: "            && let Some(handle) = None::<&Expr>\n",
    tests: ["test/corpus.test.ts", "-t", "^flattened_state_lent_through_what_holds_it.rs$"],
  },
];
