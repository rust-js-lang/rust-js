// Mutations of src/lower/mir/rvalues.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "parameter-given-its-pointer-boxed",
    breaks:
      "`&raw mut *r` of a parameter its callers give the pointer to its place for is refused as a box of a copy",
    file: "src/lower/mir/rvalues.rs",
    find: "                    if base.projection.is_empty() && self.given_its_pointer_here(state.body, base.local) {",
    replace: "                    if false && self.given_its_pointer_here(state.body, base.local) {",
    tests: ["test/corpus.test.ts", "-t", "^reference_given_as_its_pointer\\.rs$"],
  },
  {
    name: "stored-at-reference-unrecorded",
    breaks: "a reference made to an object's place, `&(other.0).0`, doesn't record where it's stored, so a field's address through it is its own allocation's, `4294963208` where native Rust's is `8` (#1113)",
    file: "src/lower/mir/rvalues.rs",
    find: "        if let Rvalue::Ref(_, _, place) | Rvalue::RawPtr(mir::RawPtrKind::Mut | mir::RawPtrKind::Const, place) = rvalue\n",
    replace: "        if let Rvalue::RawPtr(mir::RawPtrKind::Mut | mir::RawPtrKind::Const, place) = rvalue\n",
    tests: ["test/corpus.test.ts", "-t", "^address_of_a_field_where_its_object_is_stored.rs$"],
  },
  {
    name: "rooted-item-of-no-offset-unrooted",
    breaks: "an array's item a tuple holds, `(wrapped.0)[1]`, whose own place says no offset, is given no root, so it's 4096 from its field's where native Rust's is 0 (#1110)",
    file: "src/lower/mir/rvalues.rs",
    find: 'if callee.helper_name() == Some("$ptr") && matches!(given.len(), 2 | 3) =>',
    replace: 'if callee.helper_name() == Some("$ptr") && given.len() == 3 =>',
    tests: ["test/corpus.test.ts", "-t", "^address_of_a_nested_field_through_a_deref.rs$"],
    snapshots: true,
  },
  {
    name: "item-written-at-its-own-index-only",
    breaks:
      "an array's item given another reference, `cur[0] = &mut b`, isn't seen to change what a place through `cur[i]` is, so a handle or a `Cell`'s receiver made of it reads `cur[0]` again where it's used (#1075)",
    file: "src/lower/mir/rvalues.rs",
    find: "    path.len() >= written.len() && (path.iter().zip(written)).all(|(a, b)| a == b || (item(a) && item(b)))\n",
    replace: "    path.len() >= written.len() && (path.iter().zip(written)).all(|(a, b)| a == b)\n",
    tests: ["test/corpus.test.ts", "-t", "^reference_an_item_holds_set_again.rs$"],
  },
  {
    name: "item-reference-written-unseen",
    breaks:
      "a `&mut` held in an aggregate, of a place through a reference an array's item holds, isn't made where it's borrowed, so it reads `cur[0]` again after `cur[0] = &mut b` (#1075)",
    file: "src/lower/mir/rvalues.rs",
    find: "            written.local == local && !written.projection.is_empty() && within(holder, written.projection)\n",
    replace: "            written.local == local && !written.projection.is_empty() && holder.starts_with(written.projection.as_slice())\n",
    tests: ["test/corpus.test.ts", "-t", "^reference_an_item_holds_set_again.rs$"],
  },
  {
    name: "nested-field-tested-loose",
    breaks: "a nested field's `None` is tested `== null`, so `{ value: null }` is `None` (#470)",
    file: "src/lower/mir/rvalues.rs",
    find: "            Rvalue::Discriminant(place) if self.held_nested(state, *place, place.projection.len()) => {\n",
    replace: "            Rvalue::Discriminant(place) if place.projection.is_empty() && self.held_nested(state, *place, 0) => {\n",
    tests: ["test/lowering-nested-options.test.ts", "-t", "a dictionary's nullable Option of an Option"],
  },
  {
    name: "nested-field-lent-mut",
    breaks: "a `&mut` to a nested field is a handle that writes the box in it (#470)",
    file: "src/lower/mir/rvalues.rs",
    find: "                    if matches!(kind, BorrowKind::Mut { .. }) {\n                        return Err(self.unsupported(\n",
    replace: "                    if matches!(kind, BorrowKind::Mut { .. }) && false {\n                        return Err(self.unsupported(\n",
    tests: ["test/diagnostics.test.ts", "-t", "nullable Option of an Option"],
  },
  {
    name: "held-fn-pointer-in-place",
    breaks:
      "a binding taken as a `fn` through the module that holds it is read in place, `Object.assign(target, ...sources)`",
    file: "src/lower/mir/rvalues.rs",
    find: "                                Operand::Constant(c) => c.span,\n",
    replace:
      "                                Operand::Constant(_) => rustc_span::DUMMY_SP,\n",
    tests: ["test/lowering-bindings.test.ts", "-t", "named through the module"],
  },
  {
    name: "binding-reified-as-arrow",
    breaks:
      "a binding taken as a `fn` is its arrow, which reads the global at each call",
    file: "src/lower/mir/rvalues.rs",
    find: "                            && bindings::is_binding(tcx, def_id)\n                        {\n",
    replace:
      "                            && false\n                        {\n",
    tests: ["test/lowering-functions.test.ts", "-t", "a binding taken as a fn"],
  },
  {
    name: "mir-shared-temporary-a-place",
    breaks:
      "a shared reference to a temporary's read of a static is read where it's used, after what changes the static",
    file: "src/lower/mir/rvalues.rs",
    find: "                            if e.reads_same() && movable(state, &Value::Expr(e.clone())) =>\n",
    replace: "                            if e.reads_same() =>\n",
    tests: [
      "test/lowering-matches.test.ts",
      "-t",
      "a nested pattern of a captured item from MIR",
    ],
  },
  {
    name: "mir-unsize-unchecked",
    breaks:
      "a `Box` of a struct whose last field is a `dyn`, unsized, is compiled, with no `impl` for its field",
    file: "src/lower/mir/rvalues.rs",
    find: "                            self.check_unsize(*to, span)?;\n",
    replace: "",
    tests: [
      "test/diagnostics.test.ts",
      "-t",
      "a raw pointer to a struct of a dyn last field",
    ],
  },
  {
    name: "mir-rc-dyn-unpaired",
    breaks:
      "`Rc<dyn Any>` made from MIR is the value, not its pair with its dictionary",
    file: "src/lower/mir/rvalues.rs",
    find: "let to_dyn = matches!(self.pointee(*to).kind(), ty::Dynamic(..))",
    replace:
      "let to_dyn = to.builtin_deref(true).is_some_and(|p| matches!(p.kind(), ty::Dynamic(..)))",
    tests: ["test/corpus.test.ts", "-t", "dyn_any_generic"],
  },
  {
    name: "mir-mut-subslice-copied",
    breaks: "a `&mut` to a slice pattern's rest writes a copy, not the slice",
    file: "src/lower/mir/rvalues.rs",
    find: "                        .any(|elem| matches!(elem, PlaceElem::Subslice { .. }))\n",
    replace:
      "                        .any(|elem| false && matches!(elem, PlaceElem::Subslice { .. }))\n",
    tests: ["test/corpus.test.ts", "-t", "^slice_pattern_mut_rest.rs$"],
  },
  {
    name: "mir-whole-cast-wrapped",
    breaks:
      "a float THIR knows whole and in range, cast, is `$f64ToInt(index, ..)` from MIR, not `index`",
    file: "src/lower/mir/rvalues.rs",
    find: "                } else if self.kept_cast_at(span) {",
    replace: "                } else if false {",
    tests: [
      "test/lowering-numbers.test.ts",
      "-t",
      "a whole float's cast is the float from MIR too",
    ],
  },
  {
    name: "mir-range-bound-first",
    breaks: "a range pattern's test from MIR reads its bound first, `2 <= k`",
    file: "src/lower/mir/rvalues.rs",
    find: "&& state.facts.tested().iter().any(|pattern| pattern.contains(span))",
    replace: "&& false",
    tests: [
      "test/lowering-matches.test.ts",
      "-t",
      "a range pattern's tests from MIR",
    ],
  },
  {
    name: "mir-enum-cast-refused",
    breaks:
      "`ordering as i32` from MIR, and an enum's `as`, is a test of each variant, `cmp === -1 ? -1 : ..`, not its discriminant",
    file: "src/lower/mir/rvalues.rs",
    find: "                    Value::Discriminant(value, enum_ty, _) => {\n                        return Ok(Value::Expr(self.cast(value, enum_ty, *to, span)?));\n                    }\n",
    replace: "",
    tests: [
      "test/lowering-numbers.test.ts",
      "-t",
      "an enum cast to a number from MIR",
    ],
  },
  {
    name: "mir-reborrow-cell-folded",
    breaks:
      "MIR's `&mut *s` of `leak()`'s cell is its text, `\"cd\"`, not the cell",
    file: "src/lower/mir/rvalues.rs",
    find: '                                    let kind = js::ExprKind::Member(Box::new(cell), "value".into());\n                                    Expr {\n                                        kind,\n                                        ..Expr::undefined()\n                                    }\n',
    replace:
      '                                    Expr::member(cell, "value")\n',
    tests: ["test/corpus.test.ts", "-t", "leaked_string_written"],
  },
  {
    name: "mir-through-alias-handle",
    breaks:
      "MIR's `&mut outer[0]` through `outer`, a `&mut` to `list`, indexes a handle on `list`",
    file: "src/lower/mir/rvalues.rs",
    find: "                        (_, Value::Ref(e)) if !place.projection.is_empty() && !self.mut_ref_form(pointee).is_cell() => {\n",
    replace: "                        (_, Value::Ref(e)) if false => {\n",
    tests: ["test/corpus.test.ts", "-t", "^ref_mut_of_mut_payload.rs$"],
  },
  {
    name: "mir-borrow-root-unheld",
    breaks:
      "MIR's `&mut list.value` of a `list` moved on later reads `list` where it's used",
    file: "src/lower/mir/rvalues.rs",
    find: "                    let target = match self.local_reference_written(state, place) {\n",
    replace: "                    let target = match false {\n",
    tests: ["test/corpus.test.ts", "-t", "generic_mut_ref_kept"],
  },
  {
    name: "mir-dyn-drops-lost",
    breaks:
      "MIR's `Rc<dyn Iterator>` of a value with a destructor, a JS iterator, never drops it",
    file: "src/lower/mir/rvalues.rs",
    find: "                        if self\n                            .drop_query()\n                            .dyn_drop_lost(",
    replace:
      "                        if false\n                            && self\n                            .drop_query()\n                            .dyn_drop_lost(",
    tests: ["test/lowering.test.ts", "-t", "MIR refuses an Rc<dyn Iterator>"],
  },
  {
    name: "mir-named-divisor-unknown",
    breaks: "MIR's division by a named constant that can't panic is `$div`",
    file: "src/lower/mir/rvalues.rs",
    find: "                    _ => self.binary(*op, l, r, known, ty, span)?,\n",
    replace:
      "                    _ => self.binary(*op, l, r, None.or(known.filter(|_| false)), ty, span)?,\n",
    tests: [
      "test/lowering-numbers.test.ts",
      "-t",
      "a division by a named constant",
    ],
  },
  {
    name: "mir-raw-ptr-object-refused",
    breaks: "MIR's pointer to a JS object, `ptr::eq`'s, is refused",
    file: "src/lower/mir/rvalues.rs",
    find: "            Rvalue::RawPtr(_, place) if self.is_js_object(place.ty(decls, tcx).ty) => {\n",
    replace:
      "            Rvalue::RawPtr(_, place) if false && self.is_js_object(place.ty(decls, tcx).ty) => {\n",
    tests: ["test/lowering-bindings.test.ts", "-t", "ptr::eq of JS objects"],
  },
  {
    name: "mir-cell-handle-root-followed",
    breaks:
      "MIR's handle on a `Cell` field reads the field of what its reference is set to after",
    file: "src/lower/mir/rvalues.rs",
    find: "                    && (self.is_std_type(pointee, StdItem::Cell)\n                        || self\n                            .place_field(state, *place)\n                            .is_some_and(|field| self.property_cell(Some(field), pointee).is_some()))\n",
    replace: "                    && false\n",
    tests: ["test/lowering-cells.test.ts", "-t", "a Cell a let takes apart"],
  },
  {
    name: "raw-pointer-through-mut-unrefused",
    breaks:
      "a raw pointer to what a `&mut` refers to is the handle's place, which another pointer to it isn't",
    file: "src/lower/mir/rvalues.rs",
    find: "                    if js::same_path(&target, &Expr::held(reference)) {\n",
    replace:
      "                    if false && js::same_path(&target, &Expr::held(reference)) {\n",
    tests: ["test/diagnostics.test.ts", "-t", "a pointer through a &mut"],
  },
  {
    name: "dyn-cell-cast-unsized",
    breaks: "a coercion to `RefCell<dyn T>` keeps the cell without its `impl`",
    file: "src/lower/mir/rvalues.rs",
    find: "                    || self.dyn_cell_trait(*to).is_some()\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^dyn_ref_cells.rs$"],
  },
  {
    name: "dyn-tail-cast-unsized",
    breaks:
      "a coercion to `&Fat<dyn T>` keeps the struct without its last field's `impl`",
    file: "src/lower/mir/rvalues.rs",
    find: "                    || self.dyn_tail(operand.ty(decls, tcx), *to).is_some();\n",
    replace: ";\n",
    tests: ["test/corpus.test.ts", "-t", "^dyn_struct_tails.rs$"],
  },
  {
    name: "dyn-field-ref-plain",
    breaks:
      "a reference to a struct's `dyn` last field is its value, with no `impl` to call through",
    file: "src/lower/mir/rvalues.rs",
    find: "                    && matches!(self.unwrapped(place.ty(decls, tcx).ty).kind(), ty::Dynamic(..))\n                {\n                    let base_ty",
    replace:
      "                    && false\n                {\n                    let base_ty",
    tests: ["test/corpus.test.ts", "-t", "^dyn_struct_tails.rs$"],
  },
  {
    name: "dyn-field-mut-ref-copied",
    breaks:
      "a `&mut` to a struct's `dyn` last field, a number, is a pair of its value, whose method's write is lost",
    file: "src/lower/mir/rvalues.rs",
    find: "                        BorrowKind::Mut { .. } => Expr::pair(value, dictionary),\n",
    replace:
      "                        BorrowKind::Mut { .. } if false => Expr::pair(value, dictionary),\n",
    tests: ["test/corpus.test.ts", "-t", "^dyn_struct_tails.rs$"],
  },
  {
    name: "union-field-borrow-unrefused",
    breaks:
      "a `&mut` to a union's field is taken, whose write would change which it holds unsaid",
    file: "src/lower/mir/rvalues.rs",
    find: "                    && self.is_union(base.ty(decls, tcx).ty)\n",
    replace: "                    && false\n",
    tests: ["test/diagnostics.test.ts", "-t", "a &mut to a union's field"],
  },
  {
    name: "reborrowed-pointer-new-handle",
    breaks:
      "`&raw const *p` of a pointer is a handle on its `value`, not the pointer: two pointers to one place differ",
    file: "src/lower/mir/rvalues.rs",
    find: "                    && self.is_handle_pointer(decls[place.local].ty)\n",
    replace: "                    && false\n",
    tests: ["test/corpus.test.ts", "-t", "^raw_pointers.rs$"],
    // Its JS, `$ptr(p, "value", 0)` for `p`; the runtime gives the same
    // place of it since #1479, so the snapshot holds it.
    snapshots: true,
  },
  {
    name: "pointer-of-reference-refused",
    breaks:
      "`&x as *const _` through a reference to a whole local is refused, where it's a pointer to that local",
    file: "src/lower/mir/rvalues.rs",
    find: "                    && let Some(&local) = state.facts.pointed().refs.get(&place.local)\n",
    replace: "                    && let Some(&local) = None::<&Local>\n",
    tests: ["test/corpus.test.ts", "-t", "^raw_pointers.rs$"],
  },
  {
    name: "array-pointer-whole",
    breaks: "`&arr as *const T` points to the array, not its first item",
    file: "src/lower/mir/rvalues.rs",
    find: "                    PointerCoercion::ArrayToPointer if self.is_handle_pointer(*to) => {\n",
    replace:
      "                    PointerCoercion::ArrayToPointer if false => {\n",
    tests: ["test/corpus.test.ts", "-t", "^raw_pointers.rs$"],
  },
  {
    name: "slice-item-pointer-unchecked",
    breaks:
      "a pointer into a subslice's copy is taken, its place not the array's",
    file: "src/lower/mir/rvalues.rs",
    find: "                self.check_slice_item_pointer((state.body, &state.facts.pointed().copies), *place, span)?;\n",
    replace: "",
    tests: [
      "test/diagnostics.test.ts",
      "-t",
      "a pointer into a subslice's copy",
    ],
  },
  {
    name: "raw-const-of-shared-a-place",
    breaks:
      "a `*const` of what a shared reference refers to is taken as a place's, refused where JS names none, arrayvec's `elt as *const _`",
    file: "src/lower/mir/rvalues.rs",
    find: "                    && state.facts.pointed().read_only.contains(&place.local)\n",
    replace: "                    && false\n",
    tests: ["test/corpus.test.ts", "-t", "^slice_pointers.rs$"],
  },
  {
    name: "mir-fake-raw-refused",
    breaks:
      "a bounds check's borrow for a subslice's length is taken as a pointer into the copy JS holds of it, and refused",
    file: "src/lower/mir/rvalues.rs",
    find: "            Rvalue::RawPtr(mir::RawPtrKind::FakeForPtrMetadata, place) => self.mir_place(state, *place, span, out)?,\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^slice_patterns.rs$"],
  },
  {
    name: "crate-future-made-dyn",
    breaks:
      "a future of the crate's made a `dyn Future`, a promise, is the value itself",
    file: "src/lower/mir/rvalues.rs",
    find: "                    return Err(self.unlowered_future_awaited(future, span));\n",
    replace: "                    let _ = future;\n",
    tests: ["test/diagnostics.test.ts", "-t", "as a dyn Future"],
  },
  {
    name: "dyn-double-ended-made-forgetting-its-back",
    breaks:
      "a range made a `dyn DoubleEndedIterator` is a JS iterator with no back",
    file: "src/lower/mir/rvalues.rs",
    find: "                                true => self.as_double_ended(value.clone(), iterator, span)?,\n",
    replace:
      "                                true => self.as_js_iterator(value.clone(), iterator, span)?,\n",
    tests: ["test/corpus.test.ts", "-t", "^generic_double_ended.rs$"],
  },
  {
    name: "reborrowed-cell-place-unkept",
    breaks:
      "a reborrow of a cell a place holds, `&mut **u`, is a new handle on its `value`, not the cell, `return o.value`",
    file: "src/lower/mir/rvalues.rs",
    find: "                        self.locals.mir.cells.push((**cell).clone());\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^ref_mut_of_mut_payload.rs$"],
    snapshots: true,
  },
  {
    name: "raw-pointer-unsized-as-handle",
    breaks:
      "a raw pointer unsized to a `dyn` is a pair of the handle, not of its place's value",
    file: "src/lower/mir/rvalues.rs",
    find: "                            let place = match self.has_no_place(self.pointee(from)) {\n                                true => Expr::undefined(),\n                                false => self.pointee_place(value),\n                            };\n",
    replace: "                            let place = value;\n",
    tests: ["test/corpus.test.ts", "-t", "^dyn_pointers.rs$"],
  },
  {
    name: "discriminant-compared-as-a-value",
    breaks:
      "an inlined `is_some()`'s discriminant compared with a variant's, in a std body, is read as a value, not as the variant's test",
    file: "src/lower/mir/rvalues.rs",
    find: "                if let BinOp::Eq | BinOp::Ne = op\n",
    replace:
      "                if let BinOp::Eq | BinOp::Ne = op\n                    && false\n",
    tests: ["test/std-package.test.ts"],
  },
  {
    name: "dyn-field-through-wrapper",
    breaks:
      "a borrow of a struct's `ManuallyDrop<dyn T>` last field isn't its value and the struct's `impl`",
    file: "src/lower/mir/rvalues.rs",
    find: "                    && matches!(self.unwrapped(place.ty(decls, tcx).ty).kind(), ty::Dynamic(..))\n",
    replace:
      "                    && matches!(place.ty(decls, tcx).ty.kind(), ty::Dynamic(..))\n",
    tests: ["test/corpus.test.ts", "-t", "manually_drop_unsized"],
  },
  {
    name: "mir-shift-amount-bigint",
    breaks: "`22i32 >> by` of an `i64` `by` shifts a number by a BigInt",
    file: "src/lower/mir/rvalues.rs",
    find: "crate::lower::std_types::number::shift_amount_of(*op, self.value_expr(r, span)?, ty, r_ty)",
    replace: "{ let _ = r_ty; self.value_expr(r, span)? }",
    tests: ["test/corpus.test.ts", "-t", "wrapping"],
  },
  {
    name: "slice-pointer-cast-numeric",
    breaks:
      "a cast of a slice's raw pointer is taken for a number's, and refused",
    file: "src/lower/mir/rvalues.rs",
    find: "                if crate::lower::pointers::raw_pointee(tcx, operand.ty(decls, tcx)).is_some()\n                    || crate::lower::pointers::raw_pointee(tcx, *to).is_some() =>\n",
    replace:
      "                if self.is_handle_pointer(operand.ty(decls, tcx)) || self.is_handle_pointer(*to) =>\n",
    tests: ["test/corpus.test.ts", "-t", "^uninit_slice_cast.rs$"],
  },
  {
    name: "ptr-eq-by-identity",
    breaks:
      "`p == q` of raw pointers is `===`, so two pointers with no place at one address differ",
    file: "src/lower/mir/rvalues.rs",
    find: "                if self.compared_by_address(ty)\n                    && let Some((pointee, _)) = crate::lower::pointers::raw_pointee(tcx, ty)\n                {\n                    let same",
    replace:
      "                if false\n                    && let Some((pointee, _)) = crate::lower::pointers::raw_pointee(tcx, ty)\n                {\n                    let same",
    tests: ["test/corpus.test.ts", "-t", "^pointer_addresses.rs$"],
  },
  {
    name: "pointer-of-slot-reference-slot",
    breaks:
      "a pointer to what a reference read from a slot refers to is a handle on that slot, `$ptr(refs, 0)`, not on the object, so `ptr::eq` of it and the object's own slot is false",
    file: "src/lower/mir/rvalues.rs",
    find: "                    && (!base.projection.is_empty() || self.lent_from_a_holder(state.body, base.local))\n",
    replace: "                    && self.lent_from_a_holder(state.body, base.local)\n",
    tests: ["test/corpus.test.ts", "-t", "^ptr_eq_through_refs.rs$"],
  },
  {
    name: "registered-read-some-unknown",
    breaks:
      "a value read as the registry's symbol, only tested against them, is tested for its `Some`, `if (ty != null) { switch (ty) { .. } }`",
    file: "src/lower/mir/rvalues.rs",
    find: "                if place.projection.is_empty() && state.facts.options().registered.contains(&place.local) =>\n",
    replace:
      "                if place.projection.is_empty() && state.facts.options().registered.contains(&place.local) && false =>\n",
    tests: [
      "test/lowering-symbols.test.ts",
      "-t",
      "a match of the registry's symbols",
    ],
  },
  {
    name: "place-stepped-in-bytes",
    breaks:
      "a place in units stepped past a literal by its length in bytes, `at + 2` past an `é`, one unit",
    file: "src/lower/mir/rvalues.rs",
    find: "                            (Operand::Constant(_), Some(units)) => Expr::int(units as i128),",
    replace:
      "                            (Operand::Constant(_), Some(_)) => self.value_expr(part, span)?,",
    tests: ["test/lowering-string-places.test.ts", "-t", "stepped past"],
  },
  {
    name: "dyn-iterator-as-made",
    breaks:
      "a `dyn Iterator` of the crate's iterator keeps its own `fold`, which std's `Box` doesn't give it",
    file: "src/lower/mir/rvalues.rs",
    find: "                            let it = crate::lower::iterators::behind_a_dyn(it);\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^own_fold_and_nth.rs$"],
  },
  {
    name: "dyn-debug-pair-unheld",
    breaks:
      "a pair upcast to a dyn Debug where it's made by a call is held once",
    file: "src/lower/mir/rvalues.rs",
    find: "                            let value = match self.is_pair_dyn(from) && !value.parts_read_same() {",
    replace: "                            let value = match false {",
    tests: ["test/corpus.test.ts", "-t", "^dyn_shapes_shared.rs$"],
  },
  {
    name: "thrown-tested-none",
    breaks:
      "a downcast of an unwind's payload to `&JsError` is `None` where what was thrown is falsy",
    file: "src/lower/mir/rvalues.rs",
    find: "                    && let Some(&discriminant) = state.facts.options().thrown.get(&place.local) =>\n",
    replace:
      "                    && let Some(&discriminant) = state.facts.options().thrown.get(&place.local).filter(|_| false) =>\n",
    tests: ["test/lowering-unwinds.test.ts", "-t", "every unwind"],
  },
  {
    name: "cell-handle-place-reread",
    breaks:
      "a handle on `get(&c).value` calls `get` each time it's read or written",
    file: "src/lower/mir/rvalues.rs",
    find: "                    && !fixed_place(on)\n",
    replace: "                    && false\n",
    tests: ["test/corpus.test.ts", "-t", "^cell_reached_through_a_call.rs$"],
  },
  {
    name: "cell-read-before-written-held",
    breaks:
      "a `Cell` read by `get` through a reference set again after, `node.s.get()` in a loop that moves `node` on, is `const node$1 = node; node$1.s`, not `node.s`",
    file: "src/lower/mir/rvalues.rs",
    find: "                    && !(self.is_std_type(pointee, StdItem::Cell)\n",
    replace: "                    && !(false\n",
    tests: ["test/corpus.test.ts", "-t", "^cache_node_walk.rs$"],
    snapshots: true,
  },
  {
    name: "cell-kept-by-any-call",
    breaks:
      "a `Cell` given to a function that keeps it, `hold(&node.v)`, then `node = a`, reads `a`'s, not the node it was made of",
    file: "src/lower/mir/rvalues.rs",
    find: "            && matches!(\n                self.recognition().classify(def_id, generic_args),\n",
    replace:
      "            && !matches!(\n                self.recognition().classify(def_id, generic_args),\n",
    tests: ["test/corpus.test.ts", "-t", "^given_last_unwinds.rs$"],
  },
  {
    name: "written-none-of-two-writes",
    breaks:
      "a temporary written `Some(..)` or `None`, `S { title: if c { Some(t) } else { None } }`'s, is taken for the literal's `None`: left out where it's `Some` (ADR 0810, amended)",
    file: "src/lower/mir/rvalues.rs",
    find: "        if !place.projection.is_empty() || locals.writes[place.local] != 1 {\n",
    replace: "        if !place.projection.is_empty() {\n",
    tests: [
      "test/lowering-nullable.test.ts",
      "-t",
      "an optional nullable field",
    ],
  },
  {
    name: "written-none-cell-unasked",
    breaks:
      "an optional nullable field's `Cell::new(None)`, a temporary MIR makes where it has drop glue, isn't the literal's `None`, so it's there, `queue: null`, not left out (ADR 0810, amended)",
    file: "src/lower/mir/rvalues.rs",
    find: "                }) && matches!(&args[..], [arg] if self.written_none(body, locals, &arg.node));\n",
    replace: "                }) && false;\n",
    tests: [
      "test/lowering-nullable-constants.test.ts",
      "-t",
      "an optional nullable Cell of what drops is left out where it's made",
    ],
  },
  {
    name: "math-pointer-arrow",
    breaks:
      "`u32::leading_zeros` taken as a `fn` is the arrow of its call, `(n) => Math.clz32(n)`, not `Math.clz32`, which the program's polyfill captures",
    file: "src/lower/mir/rvalues.rs",
    find: "                            && let Some(function) = self.math_function(def_id, args)\n                        {\n                            return Ok(Value::Expr(function));\n",
    replace:
      "                            && let Some(function) = self.math_function(def_id, args).filter(|_| false)\n                        {\n                            return Ok(Value::Expr(function));\n",
    tests: [
      "test/lowering-math.test.ts",
      "-t",
      "a std function that's Math's is its function, and is_defined asks of it",
    ],
  },
  {
    name: "closure-fn-pointer-kept-arrow",
    breaks:
      "a closure as a `fn` that only calls the crate's function is an arrow of it, React's `ContextOnlyDispatcher`'s `useCallback() { throwInvalidHookError(); }` (ADR 0605, amended)",
    file: "src/lower/mir/rvalues.rs",
    find: "                        if matches!(coercion, PointerCoercion::ClosureFnPointer(_))\n",
    replace: "                        if false\n",
    tests: [
      "test/lowering-passed-on.test.ts",
      "-t",
      "a closure as a fn that only calls",
    ],
  },
  {
    name: "owned-dyn-closure-passed-on",
    breaks:
      "a closure an `Rc` owns as a `dyn Fn` is the function it adapts, so two `Rc`s of two closures are `Rc::ptr_eq` (ADR 0605, amended)",
    file: "src/lower/mir/rvalues.rs",
    find: "                        if from.is_ref()\n                            && matches!(self.pointee(from).kind(), ty::Closure(..))\n",
    replace:
      "                        if true\n                            && matches!(self.pointee(from).kind(), ty::Closure(..))\n",
    tests: ["test/lowering-passed-on.test.ts", "-t", "lent as a dyn Fn"],
  },
  {
    name: "lent-dyn-closure-kept-arrow",
    breaks:
      "a closure lent as a `&dyn Fn` that only adapts the crate's function stays an arrow, a new component each render (ADR 0605, amended)",
    file: "src/lower/mir/rvalues.rs",
    find: "                            && matches!(self.pointee(from).kind(), ty::Closure(..))\n",
    replace: "                            && false\n",
    tests: ["test/lowering-passed-on.test.ts", "-t", "lent as a dyn Fn"],
  },
  {
    name: "nested-option-test-loose",
    breaks:
      "a held `Option` of an `Option` is tested `!= null`, so its `Some(None)`, `null`, is `None` (ADR 1017)",
    file: "src/lower/mir/rvalues.rs",
    find: "                return Ok(Value::Tested(Expr::bin(Op::Ne, held, none)));\n",
    replace:
      "                return Ok(Value::Tested(Expr::bin(Op::LooseNe, held, Expr::null())));\n",
    tests: ["test/corpus.test.ts", "-t", "^nested_option_held_as_payload.rs$"],
  },
  {
    name: "reborrow-argument-unplaced",
    breaks:
      "an argument given by reference, `&*form`, has no place in the source map, so a block comment before it is printed after it, `.r(form /* requestFormReset */)` (rd205)",
    file: "src/lower/mir/rvalues.rs",
    find: "                            match reborrow && given_to_call(state) {\n",
    replace:
      "                            match reborrow && false && given_to_call(state) {\n",
    tests: [
      "test/lowering-comments-inside.test.ts",
      "-t",
      "argument given by reference",
    ],
  },
  {
    name: "binding-lent-as-dyn-arrow",
    breaks:
      "a binding's function lent as a `&dyn Fn` is a new arrow each time, `() => defaultIndicator()`, so React's registerDefaultIndicator sees another indicator per root (rd289)",
    file: "src/lower/mir/rvalues.rs",
    find: "                            && bindings::is_binding(self.tcx, def_id)\n",
    replace:
      "                            && bindings::is_binding(self.tcx, def_id) && false\n",
    tests: ["test/js-functions.test.ts", "-t", "lent as a dyn Fn"],
  },
  {
    name: "constant-pattern-test-constant-first",
    breaks:
      "a float's constant pattern, tested by MIR constant first, reads `1 === a`, not `a === 1`",
    file: "src/lower/mir/rvalues.rs",
    find: "                    BinOp::Eq | BinOp::Ne => Some(*op),\n",
    replace: "",
    tests: [
      "test/corpus.test.ts",
      "-t",
      "^(default_value_exits|presence_said_by_chain).rs$",
    ],
    snapshots: true,
  },
  {
    name: "held-discriminant-undefined",
    breaks:
      "a `match` of a nullable field of what may be `undefined` tests `!== undefined`, its `Some` a `None` (rd645)",
    file: "src/lower/mir/rvalues.rs",
    find: "                let none = self.held_none(place.ty(decls, tcx).ty);\n",
    replace: "                let none = Expr::undefined();\n",
    tests: ["test/corpus.test.ts", "-t", "^nullable_field_of_undefined.rs$"],
  },
  {
    name: "nullable-get-mut-reborrow-as-given",
    breaks:
      "a reborrow of a nullable `RefCell` field's `get_mut()` is its place alone, so a callee's box is copied back as it is, `fiber.target = slot.value` (#541)",
    file: "src/lower/mir/rvalues.rs",
    find: "                                js::ExprKind::Handle(_, js::Storage::NullForNone) => {\n",
    replace: "                                js::ExprKind::Handle(_, js::Storage::NullForNone) if false => {\n",
    tests: ["test/lowering-nullable-borrows.test.ts", "-t", "written through a borrow"],
  },
  {
    name: "nullable-handle-unrecorded",
    breaks:
      "a handle a variable holds on a nullable place isn't known to be that place's, so a copy back from a callee that may throw is made in a `finally`, as if the place outlived the body (#541)",
    file: "src/lower/mir/rvalues.rs",
    find: "            self.locals.mir.made.handles.insert(name.clone(), on);\n",
    replace: "            _ = (name, on);\n",
    tests: ["test/corpus.test.ts", "-t", "^nullable_written_through_borrows.rs$"],
    snapshots: true,
  },
  {
    name: "given-to-call-own-block-only",
    breaks:
      "a reborrow given to a call a later block makes, `f(href, nonce.map(..))`'s `href`, has no span, so a comment before it is before the next argument (rd710)",
    file: "src/lower/mir/rvalues.rs",
    find: "    state.body.basic_blocks.iter().any(|block| {\n",
    replace: "    std::iter::once(data).any(|block| {\n",
    tests: ["test/corpus.test.ts", "-t", "^call_argument_comments_across_lines.rs$"],
    snapshots: true,
  },
  {
    name: "kept-handle-index-read-later",
    breaks: "a `&mut c[k]` kept past `k = 2` reads `c[k]` when it's used, the item at the new index (#988)",
    file: "src/lower/mir/rvalues.rs",
    find: "                PlaceElem::Index(index) => written_again(state, index) || index_copied_from_set(state, index),\n",
    replace: "                PlaceElem::Index(index) => written_again(state, index),\n",
    tests: ["test/corpus.test.ts", "-t", "^kept_handle_of_a_place_set_again.rs$"],
  },
  {
    name: "kept-handle-field-reference-read-later",
    breaks: "a `&mut (*h.node).count` kept past `h.node = &mut other` reads `h.node.count` when it's used, the other node's (#988)",
    file: "src/lower/mir/rvalues.rs",
    find: "                            || reference_written(body, place.local, &place.projection[..at]))\n",
    replace: "                            || false)\n",
    tests: ["test/corpus.test.ts", "-t", "^kept_handle_of_a_place_set_again.rs$"],
  },
  {
    name: "cell-through-a-field-reference-read-later",
    breaks:
      "`node.0.v.set({ node.0 = b; .. })` sets `b`'s, the `Cell` reached through the field read after the write, not the node it was (#1034)",
    file: "src/lower/mir/rvalues.rs",
    find: "                        self.reached_through_written(state, place),\n",
    replace: "                        false,\n",
    tests: ["test/corpus.test.ts", "-t", "^reference_a_field_holds_set_again.rs$"],
  },
  {
    name: "field-reference-holder-set-whole-unseen",
    breaks:
      "`node.0.v.set({ node = Plain(b); .. })` sets `b`'s, the holder of the reference set again whole taken for one never written (#1034)",
    file: "src/lower/mir/rvalues.rs",
    find: "                        && ((own && written_again(state, place.local))\n",
    replace: "                        && ((own && false)\n",
    tests: ["test/corpus.test.ts", "-t", "^reference_a_field_holds_set_again.rs$"],
  },
  {
    name: "field-reference-box-set-again-unseen",
    breaks:
      "`held.0.v.set({ held.0 = Box::new(c); .. })` of a `Box<&Node>` field sets `c`'s, a write of what holds the reference not counted as its (#1034)",
    file: "src/lower/mir/rvalues.rs",
    find: "            written.local == local && !written.projection.is_empty() && within(holder, written.projection)\n",
    replace: "            written.local == local && !written.projection.is_empty() && holder == written.projection.as_slice()\n",
    tests: ["test/corpus.test.ts", "-t", "^reference_a_field_holds_set_again.rs$"],
  },
  {
    name: "dyn-debug-made-a-pair-where-made",
    breaks: "a `&dyn Debug` is made the pair where it's made, so a derived `Debug`'s fields, which show it there, interpolate the pair (#1335)",
    file: "src/lower/mir/rvalues.rs",
    find: "                            return Ok(Value::DynDebug(from, *to, value));\n",
    replace: "                            return Ok(Value::Expr(self.dyn_debug_pair((from, *to), value, span)?));\n",
    tests: ["test/corpus.test.ts", "-t", "^(dyn_debug_shown_where_rust_calls_fmt|pretty_debug|nullable_field_of_undefined)\\.rs$"],
  },
  {
    name: "boxed-field-shared-borrow-as-box",
    breaks: "a shared borrow of a struct's field held as generic code's box, `&slot.at`, is the box, shown as `undefined` fields (#1405)",
    file: "src/lower/mir/rvalues.rs",
    find: "                    return Ok(Value::Expr(self.form_back(Form::Box, crossing, read)));\n",
    replace: "                    return Ok(Value::Expr({ let _ = crossing; read }));\n",
    tests: ["test/corpus.test.ts", "-t", "^mut_ref_fields_of_a_type_parameter.rs$"],
  },
  {
    name: "boxed-field-mut-borrow-allowed",
    breaks: "a `&mut` to a struct's field held as generic code's box, `repoint(&mut s.at, q)`, is given the box's object, and the write of the whole is lost (#1405)",
    file: "src/lower/mir/rvalues.rs",
    find: "                    if let BorrowKind::Mut { .. } = kind {\n",
    replace: "                    if let BorrowKind::Fake(_) = kind {\n",
    tests: ["test/diagnostics.test.ts", "-t", "a &mut to a field held as generic code"],
  },
  {
    name: "iterator-dyn-carries-no-drop",
    breaks: "an iterator boxed as a `dyn Iterator` carries no drop, so its `Box`'s drop drops nothing of what it owns (#1420)",
    file: "src/lower/mir/rvalues.rs",
    find: "                                && self.drops(iterator) != crate::lower::drops::Drops::Nothing;\n",
    replace: "                                && false;\n",
    tests: ["test/corpus.test.ts", "-t", "^dyn_iterator_drops.rs$"],
  },
  {
    name: "iter-mut-item-pointer-refused",
    breaks: "a raw pointer to an item a slice's `iter_mut()` lends, byteorder's `n as *const f32`, is refused (#1283)",
    file: "src/lower/mir/rvalues.rs",
    find: "                    if base.projection.is_empty() && self.lent_by_slice_iter_mut(state.body, base.local) {\n",
    replace: "                    if false {\n",
    tests: ["test/corpus.test.ts", "-t", "^pointer_to_a_lent_slot.rs$"],
  },
  {
    name: "mir-transmute-cast-as-number",
    breaks: "a `transmute` rustc's MIR makes a cast, of a struct to its bytes, is cast as a number, and refused (#1420)",
    file: "src/lower/mir/rvalues.rs",
    find: "                    self.transmute(v, (from, *to), span, out)?\n",
    replace: "                    self.cast(v, from, *to, span)?\n",
    tests: ["test/crates.test.ts", "-t", "instance_converts_and_transmutes"],
  },
  {
    name: "placed-lend-pointer-refused",
    breaks: "a raw pointer made of a `&mut` std lent as a handle on its place is refused as a box of a copy's, or wraps the handle (#1474)",
    file: "src/lower/mir/rvalues.rs",
    find: "                    && self.placed_by_lend(state.body, place.local)\n",
    replace: "                    && false\n",
    tests: ["test/corpus.test.ts","-t","^address_of_a_mut_std_lends_from_its_storage.rs$"],
    snapshots: true,
  },
  {
    name: "call-lent-cell-pointer-let-through",
    breaks: "a raw pointer to what a call lends as a cell made at each call, `o.as_mut()`'s, is let through, two pointers to one place differing (#1497)",
    file: "src/lower/mir/rvalues.rs",
    find: "                        && let js::ExprKind::Call(callee, _) = &of.kind\n",
    replace: "                        && let js::ExprKind::Call(callee, _) = &of.kind\n                        && false\n",
    tests: ["test/diagnostics.test.ts", "-t", "a raw pointer to what Option::as_mut lends"],
  },
  {
    name: "captured-mut-ref-no-handle",
    breaks: "a shared reference to a `&mut` a closure captured by name is the place itself, which what reads through it reads `.value` of (#1514)",
    file: "src/lower/mir/rvalues.rs",
    find: "                    && self.held_as_its_place(state, place)\n",
    replace: "                    && false\n",
    tests: ["test/corpus.test.ts", "-t", "^borrow_of_a_captured_mut.rs$"],
  },
  {
    name: "reborrowed-field-offset-unsaid",
    breaks: "a raw pointer made of `*r`, `r = &mut s.n` made once, says no offset, so generic code given it refuses its address at run time (#1471)",
    file: "src/lower/mir/rvalues.rs",
    find: "                    [PlaceElem::Deref] => crate::lower::pointers::made_once_of(state.body, place.local),\n",
    replace: "                    [PlaceElem::Deref] => None,\n",
    tests: ["test/corpus.test.ts", "-t", "^address_of_a_field_given_to_generic_code.rs$"],
  },
];
