// Mutations of src/lower/pointers.rs (ADR 0407).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "from-place-lent-reread",
    breaks:
      "std's `ptr::from_ref(&k)` of a number lent as its place's pointer is lowered as `FromRef`, which refuses a shared number's pointer as of a copy",
    file: "src/lower/pointers.rs",
    find: "            PointerOp::FromPlace if lent => arg(),",
    replace: "            PointerOp::FromPlace if lent && false => arg(),",
    tests: ["test/corpus.test.ts", "-t", "^reference_given_as_its_pointer\\.rs$"],
  },
  {
    name: "write-bytes-of-items-not-bytes",
    breaks: "`ptr::write_bytes(dst, byte, n)` of a wider type fills `n` bytes, not `n` items' bytes, so `[7u32; 3]` filled from its second with `0x01`, 2 items, is `[7, 257, 7]` (#1133)",
    file: "src/lower/pointers.rs",
    find: "                    _ => Expr::bin(Op::Mul, count, size),\n",
    replace: "                    _ => count,\n",
    tests: ["test/corpus.test.ts", "-t", "^ptr_write_bytes.rs$"],
  },
  {
    name: "tuple-field-pointer-no-offset",
    breaks: "a tuple's field's pointer, `$ptr(t, 1)`, is given no byte offset, so its address is its index times its size, `0 1 2` of a `(u8, u32, u16)` rustc lays out `4 0 6` (#1081)",
    file: "src/lower/pointers.rs",
    find: "            return Ok(handle(*object, *index, offset));\n",
    replace: "            return Ok(handle(*object, *index, Offset::Unsaid));\n",
    tests: ["test/corpus.test.ts", "-t", "^address_of_a_tuple_field.rs$"],
  },
  {
    name: "tuple-field-of-a-generic-tuple-an-item",
    breaks: "a generic tuple's field's pointer, whose layout its caller says, is given no `null` offset, so its address is its index times a size where it's refused (#1081)",
    file: "src/lower/pointers.rs",
    find: "                Offset::LaidOutByCaller => args.push(Expr::null()),\n                Offset::Unsaid => {}\n",
    replace: "                Offset::LaidOutByCaller | Offset::Unsaid => {}\n",
    tests: ["test/runtime-package.test.ts", "-t", "a tuple's field's pointer"],
  },
  {
    name: "rc-as-ptr-of-a-held-rc-its-slot",
    breaks: "`Rc::as_ptr(&first.0)` of an `Rc` a tuple struct holds points at the array item holding it, `$ptr(first, 0)`, so two clones' pointers differ (#1019)",
    file: "src/lower/pointers.rs",
    find: "                    false if self.is_object(pointee) && !self.mut_ref_form(pointee).is_cell() => {\n",
    replace: "                    false if false => {\n",
    tests: ["test/corpus.test.ts", "-t", "^rc_as_ptr_of_a_held_rc.rs$"],
  },
  {
    name: "function-pointer-an-address",
    breaks:
      "a pointer to a fn item, `&raw const *&noop`, is an address, not the function, so `std::ptr::eq(handler, &noop)` is `handler === undefined`",
    file: "src/lower/pointers.rs",
    find: '        if self.is_function_of_no_size(pointee) {\n            return Ok(Expr::call(Expr::helper("$ptrTo"), vec![place]));',
    replace:
      '        if false {\n            return Ok(Expr::call(Expr::helper("$ptrTo"), vec![place]));',
    tests: ["test/corpus.test.ts", "-t", "^fn_compared_by_its_pointer.rs$"],
  },
  {
    name: "function-pointer-made-dyn-undefined",
    breaks:
      "a fn item's pointer made a `*const dyn Fn` is `undefined`, as a unit's is, not the function",
    file: "src/lower/pointers.rs",
    find: "        self.has_one_value(pointee) && !self.is_function_of_no_size(pointee)\n",
    replace: "        self.has_one_value(pointee)\n",
    tests: ["test/corpus.test.ts", "-t", "^fn_compared_by_its_pointer.rs$"],
  },
  {
    name: "closure-pointer-an-address",
    breaks:
      "a pointer to a closure that captures nothing is an address, so `std::ptr::eq(handler, &quiet)` is `handler === undefined`",
    file: "src/lower/pointers.rs",
    find: "        matches!(pointee.kind(), ty::FnDef(..) | ty::Closure(..)) && self.has_one_value(pointee)\n",
    replace:
      "        matches!(pointee.kind(), ty::FnDef(..)) && self.has_one_value(pointee)\n",
    tests: ["test/corpus.test.ts", "-t", "^fn_compared_by_its_pointer.rs$"],
  },
  {
    name: "pointer-pointee-object-by-place",
    breaks:
      "an object's pointer is its place's, `$ptr(a, \"value\")`, not the object's: two to it differ",
    file: "src/lower/pointers.rs",
    find: "        if (self.is_object(pointee) || function) && !self.mut_ref_form(pointee).is_cell() {\n            return self.object_pointer(",
    replace: "        if false {\n            return self.object_pointer(",
    tests: ["test/corpus.test.ts", "-t", "^raw_pointers.rs$"],
  },
  {
    name: "pointer-of-constant-refused",
    breaks: "a pointer to a constant, `&3`, is refused",
    file: "src/lower/pointers.rs",
    find: "            _ if place.is_constant() => Ok(",
    replace: "            _ if false => Ok(",
    tests: ["test/corpus.test.ts", "-t", "^raw_pointers.rs$"],
  },
  {
    name: "pointee-ptr-unsimplified",
    breaks: "`*$ptr(o, k)` is the handle's `value`, not `o[k]`",
    file: "src/lower/pointers.rs",
    find: '                (Some("$ptr"), [object, key] | [object, key, _]) => {\n',
    replace:
      '                (Some("$ptrX"), [object, key] | [object, key, _]) => {\n',
    tests: ["test/corpus.test.ts", "-t", "^static_mut.rs$"],
    snapshots: true,
  },
  {
    name: "pointed-objects-kept",
    breaks: "a local holding an object a pointer's taken to is a cell too",
    file: "src/lower/pointers.rs",
    find: "            !self.is_object(ty) || self.mut_ref_form(ty).is_cell()\n",
    replace: "            true\n",
    tests: ["test/corpus.test.ts", "-t", "^raw_pointers.rs$"],
    snapshots: true,
  },
  {
    name: "pointer-sub-adds",
    breaks: "`p.sub(n)` steps forward",
    file: "src/lower/pointers.rs",
    find: '                helper("$ptrAdd", vec![pointer, Expr::unary(js::UnaryOp::Neg, arg())])\n',
    replace: '                helper("$ptrAdd", vec![pointer, arg()])\n',
    tests: ["test/corpus.test.ts", "-t", "^raw_pointers.rs$"],
  },
  {
    name: "pointer-as-ref-unboxed",
    breaks: "`p.as_ref()` of `null` is `Some`",
    file: "src/lower/pointers.rs",
    find: "                Expr::cond(Expr::bin(Op::Eq, pointer, Expr::null()), Expr::undefined(), some)\n",
    replace: "                some\n",
    tests: ["test/corpus.test.ts", "-t", "^raw_pointers.rs$"],
  },
  {
    name: "pointer-as-mut-reads",
    breaks:
      "`p.as_mut()` of a number is its value, not a handle on it, so writing it is lost",
    file: "src/lower/pointers.rs",
    find: "                let referred = match op == PointerOp::AsMut && self.mut_ref_form(pointee).is_cell() {\n",
    replace: "                let referred = match false {\n",
    tests: ["test/corpus.test.ts", "-t", "^raw_pointers.rs$"],
  },
  {
    name: "pointer-cast-any",
    breaks: "a pointer cast to another type's is taken, read as that type",
    file: "src/lower/pointers.rs",
    find: '            _ => Err(refuse(\n                self.tcx,\n                span,\n                Refused::Memory("a pointer read as another type\'s"),\n            )),\n',
    replace: "            _ => Ok(pointer),\n",
    tests: [
      "test/diagnostics.test.ts",
      "-t",
      "a struct's pointer as a later field's",
    ],
  },
  {
    name: "pointer-null-of-address",
    breaks: "`3 as *const T` is `null`",
    file: "src/lower/pointers.rs",
    find: "            mir::CastKind::PointerWithExposedProvenance if self.is_handle_pointer(to) => match value.as_int() {\n                Some(0) => Ok(Some(Expr::null())),\n",
    replace:
      "            mir::CastKind::PointerWithExposedProvenance if self.is_handle_pointer(to) => match value.as_int() {\n                Some(_) => Ok(Some(Expr::null())),\n",
    tests: ["test/corpus.test.ts", "-t", "^pointer_addresses.rs$"],
  },
  {
    name: "pointer-address-not-taken",
    breaks: "`p as usize` isn't its address but a cast of the handle, `NaN`",
    file: "src/lower/pointers.rs",
    find: "            mir::CastKind::PointerExposeProvenance if self.is_handle_pointer(from) => {\n",
    replace:
      "            mir::CastKind::PointerExposeProvenance if false => {\n",
    tests: ["test/corpus.test.ts", "-t", "^pointer_addresses.rs$"],
  },
  {
    name: "into-raw-number-shared",
    breaks: "`Box::into_raw` of a number is the number, which `*p` can't write",
    file: "src/lower/pointers.rs",
    find: "                    false => Expr::object(vec![js::Prop::Field(js::HELD.into(), value)]),\n                }\n            }\n            PointerOp::FromRaw",
    replace:
      "                    false => value,\n                }\n            }\n            PointerOp::FromRaw",
    tests: ["test/corpus.test.ts", "-t", "^raw_pointers.rs$"],
  },
  {
    name: "copies-of-subslices-unseen",
    breaks: "a reference to a subslice, a copy, is taken for the array's",
    file: "src/lower/pointers.rs",
    find: "                        .any(|elem| matches!(elem, PlaceElem::Subslice { .. }))\n",
    replace: "                        .any(|_| false)\n",
    tests: [
      "test/diagnostics.test.ts",
      "-t",
      "a pointer into a subslice's copy",
    ],
  },
  {
    name: "copies-of-constants-unseen",
    breaks: "a named constant's pointer is taken, each use a value of its own",
    file: "src/lower/pointers.rs",
    find: "                && uv.promoted.is_none()\n                && place.projection.is_empty()\n            {\n                copies.insert",
    replace:
      "                && false\n                && place.projection.is_empty()\n            {\n                copies.insert",
    tests: ["test/diagnostics.test.ts", "-t", "a pointer to a constant"],
  },
  {
    name: "guard-binding-aliased",
    breaks:
      "a `ref mut` binding read through a match guard is the place it refers to, its `&mut` lost",
    file: "src/lower/pointers.rs",
    find: "                borrowed.insert(local);\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^raw_pointers.rs$"],
  },
  {
    name: "reference-copies-unfollowed",
    breaks: "`let d: &u32 = &b` through a temporary isn't a reference to `b`",
    file: "src/lower/pointers.rs",
    find: "                        refs.get(&from.local).copied().flatten()\n                    }\n                    _ => None,",
    replace:
      "                        None\n                    }\n                    _ => None,",
    tests: ["test/corpus.test.ts", "-t", "^raw_pointers.rs$"],
  },
  {
    name: "non-null-not-a-pointer",
    breaks:
      "a `NonNull` isn't its pointer, so its values and its methods are refused",
    file: "src/lower/pointers.rs",
    find: "if super::recognition::is_std_def(tcx, adt.did(), super::recognition::StdItem::NonNull)",
    replace:
      "if false && super::recognition::is_std_def(tcx, adt.did(), super::recognition::StdItem::NonNull)",
    tests: ["test/corpus.test.ts", "-t", "^non_null.rs$"],
  },
  {
    name: "non-null-as-mut-of-its-value",
    breaks:
      "a `NonNull`'s `as_mut()` of a number is the number, not the handle on its place, `handle.value.value`",
    file: "src/lower/pointers.rs",
    find: "                match op == PointerOp::PlaceMut && self.mut_ref_form(pointee).is_cell() {\n",
    replace: "                match false {\n",
    tests: ["test/corpus.test.ts", "-t", "^non_null.rs$"],
  },
  {
    name: "non-null-from-object-unwrapped",
    breaks:
      "`NonNull::from(&mut object)` is the object, not a handle on it, so `as_ref()` reads its `value` field",
    file: "src/lower/pointers.rs",
    find: '                    (true, _) => helper("$ptrTo", vec![reference]),\n',
    replace: "                    (true, _) => reference,\n",
    tests: ["test/corpus.test.ts", "-t", "^non_null.rs$"],
  },
  {
    name: "recast-first-item-refused",
    breaks:
      "an array's pointer cast to its item's, arrayvec's `self.xs.as_ptr() as *const T`, is read through the array's bytes, not `$ptr(xs, 0)`",
    file: "src/lower/pointers.rs",
    find: "            (ty::Array(item, _), _) if self.same_place(*item, b) => Ok(self.first_item_pointer(pointer)),\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^array_vec_raw.rs$"],
    snapshots: true,
  },
  {
    name: "recast-uninit-not-its-value",
    breaks:
      "a `MaybeUninit<T>`'s pointer cast to a `T`'s is refused, though JS holds it as its value",
    file: "src/lower/pointers.rs",
    find: "        let value = |t: Ty<'tcx>| self.recognition().uninit_of(t).unwrap_or(t);\n        let (a, b) = (value(a), value(b));\n",
    replace: "        let value = |t: Ty<'tcx>| t;\n        let (a, b) = (value(a), value(b));\n",
    tests: [
      "test/corpus.test.ts",
      "-t",
      "^(array_vec_raw|bytes_through_pointers).rs$",
    ],
    snapshots: true,
  },
  {
    name: "recast-arrays-apart",
    breaks:
      "an array of `T`s' pointer cast to one of `MaybeUninit<T>`s, arrayvec's, is refused",
    file: "src/lower/pointers.rs",
    find: "            (ty::Array(x, n), ty::Array(y, m)) => n == m && self.same_place(*x, *y),\n",
    replace:
      "            (ty::Array(x, n), ty::Array(y, m)) => false && n == m && self.same_place(*x, *y),\n",
    tests: ["test/corpus.test.ts", "-t", "^array_vec_raw.rs$"],
  },
  {
    name: "recast-whole-refused",
    breaks:
      "a first item's pointer cast to the whole array's, arrayvec's `into_inner`, is refused",
    file: "src/lower/pointers.rs",
    find: "            (_, ty::Array(item, length)) if self.same_place(a, *item) && !is_bytes(a) => {\n",
    replace:
      "            (_, ty::Array(item, length)) if false && self.same_place(a, *item) && !is_bytes(a) => {\n",
    tests: ["test/corpus.test.ts", "-t", "^array_vec_raw.rs$"],
  },
  {
    name: "recast-generic-refused",
    breaks:
      "a cast in generic code, arrayvec's `ptr.cast::<u8>()` of a type with no size, is refused for every type",
    file: "src/lower/pointers.rs",
    find: '            _ if a.has_param() || b.has_param() => Ok(helper("$ptrRecast", vec![pointer])),\n',
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^maybe_uninit_pointer.rs$"],
  },
  {
    name: "copy-from-backwards",
    breaks: "`dst.copy_from(src, n)` copies from `dst` to `src`",
    file: "src/lower/pointers.rs",
    find: "                let (src, dst) = if dst_first { (b, a) } else { (a, b) };\n",
    replace: "                let (src, dst) = (a, b);\n",
    tests: ["test/corpus.test.ts", "-t", "^array_vec_raw.rs$"],
  },
  {
    name: "copy-shares-copies",
    breaks:
      "a `Copy` value JS holds as an object is shared by both places `ptr::copy` copies it between",
    file: "src/lower/pointers.rs",
    find: "                if let Some(item) = pointee.filter(|&t| self.contains_mutated(t) && self.is_copy(t)) {\n",
    replace:
      "                if let Some(item) = pointee.filter(|_| false) {\n",
    tests: ["test/corpus.test.ts", "-t", "^array_vec_raw.rs$"],
  },
  {
    name: "slice-mut-copied",
    breaks:
      "`slice::from_raw_parts_mut` is a copy, so sorting it sorts nothing",
    file: "src/lower/pointers.rs",
    find: '                let name = if mutable { "$ptrSliceMut" } else { "$ptrSlice" };\n',
    replace:
      '                let name = if mutable { "$ptrSlice" } else { "$ptrSlice" };\n',
    tests: ["test/corpus.test.ts", "-t", "^array_vec_raw.rs$"],
  },
  {
    name: "drop-in-place-skipped",
    breaks: "`ptr::drop_in_place(p)` drops nothing, so a destructor never runs",
    file: "src/lower/pointers.rs",
    find: '                self.drop_value(place, pointee.expect("a pointer"), span, out)?;\n',
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^array_vec_raw.rs$"],
  },
  {
    name: "slice-pointer-a-handle",
    breaks:
      "a pointer to a slice is a handle on a place, which a slice isn't: its `len()` and reads go through `.value`",
    file: "src/lower/pointers.rs",
    find: "        raw_pointee(self.tcx, ty)\n            .is_some_and(|(pointee, _)| !self.is_js_object(pointee) && !self.pointee_is_itself(pointee))\n",
    replace:
      "        raw_pointee(self.tcx, ty).is_some_and(|(pointee, _)| !self.is_js_object(pointee))\n",
    tests: ["test/corpus.test.ts", "-t", "^slice_pointers.rs$"],
  },
  {
    name: "slice-pointer-not-the-slice",
    breaks:
      "a pointer to a slice, `items as *mut [T]`, is refused, as a place JS has no name for",
    file: "src/lower/pointers.rs",
    find: "        if self.pointee_is_itself(pointee) {\n            return Ok(place);\n",
    replace: "        if false {\n            return Ok(place);\n",
    tests: ["test/corpus.test.ts", "-t", "^slice_pointers.rs$"],
  },
  {
    name: "slice-pointer-len-the-pointer",
    breaks: "a slice pointer's `len()` is the pointer, not its length",
    file: "src/lower/pointers.rs",
    find: '            PointerOp::SliceLen => Expr::member(arg(), "length"),\n',
    replace: "            PointerOp::SliceLen => arg(),\n",
    tests: ["test/corpus.test.ts", "-t", "^slice_pointers.rs$"],
  },
  {
    name: "slice-pointer-cast-unread",
    breaks:
      "a slice pointer cast to its item's is the slice, not its first item's handle, so writes through it go nowhere",
    file: "src/lower/pointers.rs",
    find: "                ) && self.same_place(item, b) =>\n",
    replace: "                ) && false =>\n",
    tests: ["test/corpus.test.ts", "-t", "^slice_pointers.rs$"],
  },
  {
    name: "read-only-pointer-of-value-refused",
    breaks:
      "a `*const` of a shared reference to a value with no place JS names, arrayvec's `elt as *const _`, is refused, though a cell of it reads the same",
    file: "src/lower/pointers.rs",
    find: "            false => Ok(Expr::object(vec![js::Prop::Field(js::HELD.into(), place)])),\n",
    replace:
      "            false => self.pointer_to(place, pointee, Offset::Unsaid, span),\n",
    tests: ["test/corpus.test.ts", "-t", "^slice_pointers.rs$"],
  },
  {
    name: "read-only-pointer-compared",
    breaks:
      "a `*const` of a shared reference that's compared, `ptr::eq(a, b)` of numbers, is a cell of its own, so it's never equal to another of the same place",
    file: "src/lower/pointers.rs",
    find: "        uses.only && uses.stores == 1 && uses.copies.into_iter().all(|copy| self.only_read_through(body, copy))\n",
    replace: "        true\n",
    tests: ["test/diagnostics.test.ts", "-t", "ptr::eq of a number"],
  },
  {
    name: "read-only-pointer-fake-read-a-use",
    breaks:
      "a pointer's `let` binding, a fake read of it, counts as a use other than a read, so arrayvec's `ptr::read(elt as *const _)` is refused",
    file: "src/lower/pointers.rs",
    find: "                    PlaceContext::NonMutatingUse(NonMutatingUseContext::Inspect) => true,\n",
    replace:
      "                    PlaceContext::NonMutatingUse(NonMutatingUseContext::Inspect) => false,\n",
    tests: ["test/corpus.test.ts", "-t", "^slice_pointers.rs$"],
  },
  {
    name: "read-only-pointer-copy-a-use",
    breaks:
      "a pointer copied whole to a local is a use other than a read, so arrayvec's `ptr::read(elt as *const _)` is refused",
    file: "src/lower/pointers.rs",
    find: "                                self.copies.push(assign.0.local);\n                                true\n",
    replace:
      "                                self.copies.push(assign.0.local);\n                                false\n",
    tests: ["test/corpus.test.ts", "-t", "^slice_pointers.rs$"],
  },
  {
    name: "read-only-pointer-copy-unchecked",
    breaks:
      "a pointer copied whole to a local is read only whatever the copy's uses, so `ptr::eq` of the copy is a cell's, never equal",
    file: "src/lower/pointers.rs",
    find: "        uses.only && uses.stores == 1 && uses.copies.into_iter().all(|copy| self.only_read_through(body, copy))\n",
    replace: "        uses.only && uses.stores == 1\n",
    tests: [
      "test/diagnostics.test.ts",
      "-t",
      "ptr::eq of a number's pointer, copied",
    ],
  },
  {
    name: "read-only-pointer-returned",
    breaks:
      "a pointer to what a `&mut` refers to, returned, is taken as only read, a cell of the value, which the caller's writes through it don't reach",
    file: "src/lower/pointers.rs",
    find: "            let read = place.projection.is_empty() && place.local != mir::RETURN_PLACE;\n",
    replace: "            let read = place.projection.is_empty();\n",
    tests: ["test/corpus.test.ts", "-t", "^pointer_to_a_captured_variable_returned_refused\\.rs$"],
  },
  {
    name: "text-pointer-compared",
    breaks:
      "text's `as_ptr()` compared with another is taken, where each is a copy's, never equal",
    file: "src/lower/pointers.rs",
    find: "            || destination.projection.is_empty() && self.kept_within_its_copy(body, destination.local)\n",
    replace: "            || true\n",
    tests: ["test/diagnostics.test.ts", "-t", "text's pointer compared"],
  },
  {
    name: "copy-source-not-a-read",
    breaks:
      "what `ptr::copy` copies from isn't taken as a read through it, so a pointer to what a reference refers to, copied from, is refused",
    file: "src/lower/pointers.rs",
    find: "                    Some(Std::Pointer(PointerOp::Copy { dst_first })) => at == Some(usize::from(dst_first)),\n",
    replace:
      "                    Some(Std::Pointer(PointerOp::Copy { .. })) => false,\n",
    tests: ["test/corpus.test.ts", "-t", "^pointer_of_a_reference_copied_from.rs$"],
  },
  {
    name: "slice-pointer-place-read",
    breaks:
      "what a slice pointer points to, `*p`, `as_mut()`, `drop_in_place`, is read as a handle's `.value`, which a slice hasn't",
    file: "src/lower/pointers.rs",
    find: "        match self.pointee_is_itself(pointee) {\n            true => pointer,\n",
    replace: "        match false {\n            true => pointer,\n",
    tests: ["test/corpus.test.ts", "-t", "^slice_pointers.rs$"],
  },
  {
    name: "to-bytes-refused",
    breaks:
      "a pointer cast to bytes, an allocation's given to `realloc` or `dealloc`, is refused",
    file: "src/lower/pointers.rs",
    find: '            (_, _) if is_bytes(b) => Ok(match self.plain_codec(a, span)? {\n                Some((size, encode, decode)) => helper("$ptrBytes", vec![pointer, size, encode, decode]),\n                None => helper("$ptrBytes", vec![pointer]),\n            }),\n',
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^allocator.rs$"],
  },
  {
    name: "from-bytes-refused",
    breaks: "an allocation's pointer to bytes cast to its items' is refused",
    file: "src/lower/pointers.rs",
    find: '            (_, _) if is_bytes(a) => Ok(helper("$ptrTyped", vec![pointer])),\n',
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^allocator.rs$"],
  },
  {
    name: "generic-shared-pointer-refused",
    breaks:
      "a pointer of a shared reference to a type parameter's value, smallvec's `&MaybeUninit<A>`, is refused",
    file: "src/lower/pointers.rs",
    find: "                    (false, false) if self.is_unknown(self.recognition().uninit_of(referent).unwrap_or(referent)) => {\n",
    replace:
      "                    (false, false) if false && self.is_unknown(self.recognition().uninit_of(referent).unwrap_or(referent)) => {\n",
    tests: ["test/corpus.test.ts", "-t", "^maybe_uninit_pointer.rs$"],
  },
  {
    name: "array-item-pointer-recast",
    breaks:
      "smallvec's `*const A` as `*const A::Item`, a whole array's pointer as its first item's, is rust-js's error where it's made",
    file: "src/lower/pointers.rs",
    find: '            _ if self.is_array_item_of(b, a) => Ok(helper("$ptrFirst", vec![pointer])),\n',
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^maybe_uninit_pointer.rs$"],
  },
  {
    name: "uninit-slice-cast-refused",
    breaks:
      "a `*mut [MaybeUninit<T>]` cast to a `*mut [T]` is refused, where it's the slice itself",
    file: "src/lower/pointers.rs",
    find: "                    && let (ty::Slice(a), ty::Slice(b)) = (a.kind(), b.kind())\n                    && self.same_place(*a, *b) =>\n",
    replace:
      "                    && let (ty::Slice(a), ty::Slice(b)) = (a.kind(), b.kind())\n                    && false && self.same_place(*a, *b) =>\n",
    tests: ["test/corpus.test.ts", "-t", "^uninit_slice_cast.rs$"],
  },
  {
    name: "field-offset-not-written",
    breaks:
      "a field's handle is made without its byte offset, so its address is refused where it's asked",
    file: "src/lower/pointers.rs",
    find: "                Ok(handle(*object, Expr::str(key), offset))\n",
    replace: "                Ok(handle(*object, Expr::str(key), Offset::Unsaid))\n",
    tests: ["test/corpus.test.ts", "-t", "^pointer_addresses.rs$"],
  },
  {
    name: "dangling-at-zero",
    breaks: "a dangling pointer is at 0, `null`, not at its type's alignment",
    file: "src/lower/pointers.rs",
    find: '                let align = self.type_fact_value(pointee, TypeFact::Align, span)?;\n                helper("$ptrFrom", self.at_address(align, pointee, span)?)\n',
    replace:
      '                helper("$ptrFrom", self.at_address(Expr::int(0), pointee, span)?)\n',
    tests: ["test/corpus.test.ts", "-t", "^pointer_addresses.rs$"],
  },
  {
    name: "zst-pointer-placed",
    breaks:
      "a pointer to a zero-sized value is a place of its own, not the dangling address",
    file: "src/lower/pointers.rs",
    find: "        if !self.has_no_place(pointee) {\n            return Ok(None);\n        }",
    replace: "        if true {\n            return Ok(None);\n        }",
    tests: ["test/corpus.test.ts", "-t", "^pointer_addresses.rs$"],
  },
  {
    name: "fn-pointer-cast-numeric",
    breaks:
      "a function cast to a pointer goes through the numeric cast, refused",
    file: "src/lower/pointers.rs",
    find: "            mir::CastKind::FnPtrToPtr if self.is_handle_pointer(to) => Ok(Some(value)),\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^pointer_addresses.rs$"],
  },
  {
    name: "type-param-pointer-refused",
    breaks:
      "a pointer to a type parameter's value is refused, where its caller gives an object",
    file: "src/lower/pointers.rs",
    find: "        if (self.is_unknown(pointee) || outer) && !matches!(place.kind, js::ExprKind::Member(..)) {",
    replace: "        if outer && !matches!(place.kind, js::ExprKind::Member(..)) {",
    tests: ["test/corpus.test.ts", "-t", "^pointer_addresses.rs$"],
  },
  {
    name: "item-pointer-to-object",
    breaks:
      "a pointer to an array's item that's an object is `$ptrTo` of it, its own allocation",
    file: "src/lower/pointers.rs",
    find: "        if let js::ExprKind::Index(object, index) = place.kind {\n            return Ok(handle(*object, *index, offset));\n        }\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^(pointer_addresses|pointers_into_a_cell_compared).rs$"],
  },
  {
    name: "is-aligned-ignores-align",
    breaks:
      "`is_aligned()` asks the alignment against the address, so a pointer at a base, or a byte off, is never aligned",
    file: "src/lower/pointers.rs",
    find: "                Expr::bin(Op::Eq, Expr::bin(Op::Rem, addr, align), Expr::int(0))",
    replace:
      "                Expr::bin(Op::Eq, Expr::bin(Op::Rem, align, addr), Expr::int(0))",
    tests: ["test/corpus.test.ts", "-t", "^pointer_addresses.rs$"],
  },
  {
    name: "recast-bytes-opaque",
    breaks: "plain data cast to bytes is a view that refuses every read",
    file: "src/lower/pointers.rs",
    find: '                Some((size, encode, decode)) => helper("$ptrBytes", vec![pointer, size, encode, decode]),',
    replace: '                Some(_) => helper("$ptrBytes", vec![pointer]),',
    tests: ["test/corpus.test.ts", "-t", "^bytes_through_pointers.rs$"],
  },
  {
    name: "retyped-unsigned",
    breaks: "bytes read as an `i8` are read unsigned",
    file: "src/lower/pointers.rs",
    find: "                let signed = Expr::bool(b.is_signed());",
    replace: "                let signed = Expr::bool(false);",
    tests: ["test/corpus.test.ts", "-t", "^bytes_through_pointers.rs$"],
  },
  {
    name: "dyn-pointer-a-handle",
    breaks: "a `*const dyn Trait` is a handle on the pair, not the pair itself",
    file: "src/lower/pointers.rs",
    find: "        pointee.is_slice() || matches!(pointee.kind(), ty::Dynamic(..)) && self.is_pair_dyn(pointee)\n",
    replace: "        pointee.is_slice()\n",
    tests: ["test/corpus.test.ts", "-t", "^dyn_pointers.rs$"],
  },
  {
    name: "dyn-pointer-thin-refused",
    breaks: "a `*const dyn Trait` made thin, `z as *const A`, is refused",
    file: "src/lower/pointers.rs",
    find: "                Ok(Some(self.pointer_to(\n                    Expr::member(value, js::HELD),\n                    b,\n                    Offset::Unsaid,\n                    span,\n                )?))\n",
    replace:
      '                Err(refuse(self.tcx, span, Refused::Memory("a pair\'s pointer made thin")))\n',
    tests: ["test/corpus.test.ts", "-t", "^dyn_pointers.rs$"],
  },
  {
    name: "box-dyn-into-raw-a-handle",
    breaks:
      "`Box::into_raw` of a `Box<dyn Trait>` is a handle on the pair, not the pair",
    file: "src/lower/pointers.rs",
    find: "                if self.pointee_is_itself(held) {\n                    return Ok(Some(value));\n                }\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^dyn_pointers.rs$"],
  },
  {
    name: "box-dyn-from-raw-its-value",
    breaks:
      "`Box::from_raw` of a `*mut dyn Trait` is the pair's value, not the pair",
    file: "src/lower/pointers.rs",
    find: '            PointerOp::FromRaw => self.pointed_place(arg(), pointee.expect("a pointer")),\n',
    replace: "            PointerOp::FromRaw => self.pointee_place(arg()),\n",
    tests: ["test/corpus.test.ts", "-t", "^dyn_pointers.rs$"],
  },
  {
    name: "first-field-pointer-refused",
    breaks:
      "a struct's pointer cast to its first field's type is refused, where rustc lays the field at offset 0",
    file: "src/lower/pointers.rs",
    find: "                if adt.is_struct()\n                    && let Some(field) = self.field_at_zero(a, args, b) =>\n",
    replace:
      "                if false\n                    && let Some(field) = self.field_at_zero(a, args, b) =>\n",
    tests: ["test/corpus.test.ts", "-t", "^pointer_relayout.rs$"],
  },
  {
    name: "prefix-cast-refused",
    breaks:
      "a `#[repr(C)]` struct's pointer cast to another layout of the struct, `ErrorImpl<()>` as `ErrorImpl<E>`, is refused where it runs",
    file: "src/lower/pointers.rs",
    find: "            (ty::Adt(x, _), ty::Adt(y, _)) if x.did() == y.did() && x.is_struct() && x.repr().c() => Ok(pointer),\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^pointer_relayout.rs$"],
  },
  {
    name: "same-place-minds-lifetimes",
    breaks:
      "a `&'static V` field isn't a `&V` in JS, so a struct's pointer cast to its reference field's type is refused",
    file: "src/lower/pointers.rs",
    find: "            self.tcx\n                .erase_and_anonymize_regions(self.recognition().uninit_of(t).unwrap_or(t))\n",
    replace: "            self.recognition().uninit_of(t).unwrap_or(t)\n",
    tests: ["test/corpus.test.ts", "-t", "^pointer_relayout.rs$"],
  },
  {
    name: "field-at-zero-any-offset",
    breaks:
      "a struct's pointer is cast to any field's type, not the one at offset 0",
    file: "src/lower/pointers.rs",
    find: "            let placed = layout.fields.offset(index.index()).bytes() == 0;\n",
    replace: "            let placed = true;\n",
    tests: [
      "test/diagnostics.test.ts",
      "-t",
      "a struct's pointer as a later field's",
    ],
  },
  {
    name: "zero-sized-value-not-given",
    breaks:
      "a pointer at an address to a value of no size isn't given its JS value, so `b\"\"`'s reads `undefined`, not `[]`",
    file: "src/lower/pointers.rs",
    find: "            before.push(zero);\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^zero_sized_pointers.rs$"],
    snapshots: true,
  },
  {
    name: "dyn-pointer-compared-as-objects",
    breaks:
      "two `*const dyn` pointers to one value are compared as the two pair objects JS made, so `==` is false",
    file: "src/lower/pointers.rs",
    find: ".is_some_and(|(pointee, _)| self.is_pair_dyn_pointee(pointee) || pointee.is_slice())",
    replace: ".is_some_and(|(pointee, _)| pointee.is_slice())",
    tests: ["test/corpus.test.ts", "-t", "^dyn_pointers.rs$"],
  },
  {
    name: "pointers-of-objects-by-handle",
    breaks:
      "`==` of two pointers to objects is `$ptrEq($ptrTo(a), $ptrTo(b), size)`, not `a === b`, which says the same",
    file: "src/lower/pointers.rs",
    find: "        (Some(x), Some(y)) => (x, y),\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^raw_pointers.rs$"],
    snapshots: true,
  },
  {
    name: "dyn-fn-pointer-refused",
    breaks:
      "`std::ptr::eq` of two `&dyn Fn`s, an event handler read twice, is refused as a raw pointer to a place JS can't name",
    file: "src/lower/pointers.rs",
    find: "        let function = matches!(pointee.kind(), ty::Dynamic(..)) && self.is_callable(pointee);\n",
    replace: "        let function = false;\n",
    tests: ["test/bindings.test.ts", "-t", "a function JS gives is a dyn Fn"],
  },
  {
    name: "rc-as-ptr-of-counted-box",
    breaks:
      "a counted `Rc`'s `as_ptr` points at the `Rc` itself, not its value's place, which JS can't point at",
    file: "src/lower/pointers.rs",
    find: "                    true => self.pointer_to(self.counted_pointee(rc, pointee), pointee, Offset::Unsaid, span)?,\n",
    replace: "                    true => self.pointer_to(rc, pointee, Offset::Unsaid, span)?,\n",
    tests: ["test/corpus.test.ts", "-t", "^rc_as_ptr.rs$"],
  },
  {
    name: "field-pointers-equal-of-any-field",
    breaks:
      "two pointers to fields of one object are equal whichever fields they are, `ptr::eq(&p.x, &p.y)` true",
    file: "src/lower/pointers.rs",
    find: "            if key != other || at != other_at {\n",
    replace: "            if false {\n",
    tests: ["test/corpus.test.ts", "-t", "^rc_as_ptr.rs$"],
  },
  {
    name: "item-pointer-unchecked",
    breaks:
      "a handle on an item `$index` checked drops the check, so `ptr::eq(&items[0], r)` of no items is `false` where Rust panics",
    file: "src/lower/pointers.rs",
    find: '            let at = Expr::call(Expr::helper("$at"), args.to_vec());\n',
    replace: "            let at = args[1].clone();\n",
    tests: ["test/corpus.test.ts", "-t", "^ptr_eq_items_and_counts.rs$"],
  },
  {
    name: "item-and-object-by-handles",
    breaks:
      "`ptr::eq` of an item that's an object and an object is the handles' `$ptrEq($ptr(items, 0), $ptrTo(r), 8)`, not `items[0] === r`",
    file: "src/lower/pointers.rs",
    find: "        (None, Some(y)) if items => (item_of(a)?, y),\n        (Some(x), None) if items => (x, item_of(b)?),\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^ptr_eq_items_and_counts.rs$"],
    snapshots: true,
  },
  {
    name: "item-and-object-of-any-pointee",
    breaks:
      "`ptr::eq` of a type parameter's item and a reference is `items[0] === r`, `1 === 1` of two numbers' places",
    file: "src/lower/pointers.rs",
    find: "        let items = self.mut_ref_form(pointee).is_the_object();\n",
    replace: "        let items = true;\n",
    tests: [
      "test/lowering-collections.test.ts",
      "-t",
      "ptr::eq of a type parameter's item is by its place",
    ],
  },
  {
    name: "item-and-object-unchecked",
    breaks:
      "`ptr::eq` of an item whose index is checked and an object is `items[$at(items, 0)] === r`, not `$index(items, 0) === r`",
    file: "src/lower/pointers.rs",
    find: '                if check.helper_name() == Some("$at") && given.first().is_some_and(|of| js::same_path(of, items)) =>\n',
    replace: "                if false =>\n",
    tests: ["test/corpus.test.ts", "-t", "^ptr_eq_items_and_counts.rs$"],
    snapshots: true,
  },
  {
    name: "same-objects-not-in-place",
    breaks:
      "two handles compared, one of them spilled where MIR made it, `const _16 = $ptrTo(refs[0])`, are `$ptrEq` once it's read in place, not `refs[0] === points[0]`",
    file: "src/lower/pointers.rs",
    find: "        && let Some(same) = same_object(a, b, true)\n",
    replace:
      "        && let Some(same) = same_object(a, b, true).filter(|_| false)\n",
    tests: ["test/corpus.test.ts", "-t", "^ptr_eq_through_refs.rs$"],
    snapshots: true,
  },
  {
    name: "lent-bytes-not-copies",
    breaks: "a reference text's `as_bytes()` lends isn't a copy's, so `ptr::eq(s.as_bytes(), s.as_bytes())` compares two arrays, `false` where Rust's is `true` (#1183)",
    file: "src/lower/pointers.rs",
    find: "    let mut copies = lent_copies.clone();\n",
    replace: "    let mut copies = IndexSet::default();\n",
    tests: ["test/diagnostics.test.ts", "-t", "text's bytes compared by ptr::eq"],
  },
  {
    name: "lent-bytes-copied-reference-not-followed",
    breaks: "a copy of a reference to text's bytes, `let x = s.as_bytes()`, isn't one, so its `as_ptr()` is compared with another's (#1183)",
    file: "src/lower/pointers.rs",
    find: "                Rvalue::Use(mir::Operand::Copy(of) | mir::Operand::Move(of), _) => lent_copies.contains(&of.local),\n",
    replace: "",
    tests: ["test/diagnostics.test.ts", "-t", "text's bytes' pointers compared"],
  },
  {
    name: "slice-pointers-compared-as-objects",
    breaks: "`ptr::eq` of two slices compares which arrays JS holds them as, `false` of two parts of one range (#1185)",
    file: "src/lower/pointers.rs",
    find: ".is_some_and(|(pointee, _)| self.is_pair_dyn_pointee(pointee) || pointee.is_slice())",
    replace: ".is_some_and(|(pointee, _)| self.is_pair_dyn_pointee(pointee))",
    tests: ["test/corpus.test.ts", "-t", "^slice_parts_by_address.rs$"],
  },
  {
    name: "slice-pointers-equal-as-items",
    breaks: "two slices' pointers are compared as the pointers to their first items, so two parts of one start and different lengths are equal",
    file: "src/lower/pointers.rs",
    find: "            return Ok(Expr::call(Expr::helper(\"$slicePtrEq\"), vec![a, b, size]));\n",
    replace: "            return Ok(Expr::call(Expr::helper(\"$ptrEq\"), vec![Expr::call(Expr::helper(\"$ptr\"), vec![a, Expr::int(0)]), Expr::call(Expr::helper(\"$ptr\"), vec![b, Expr::int(0)]), size]));\n",
    tests: ["test/corpus.test.ts", "-t", "^slice_parts_by_address.rs$"],
  },
  {
    name: "signed-bytes-are-bytes",
    breaks: "a pointer to `i8`s is taken for one to bytes, so an `i8` array read through a `*const u8` keeps its sign, `-21` where native Rust's is `235` (#1253)",
    file: "src/lower/pointers.rs",
    find: "        let is_bytes = |t: Ty<'tcx>| matches!(t.kind(), ty::Uint(ty::UintTy::U8));\n",
    replace: "        let is_bytes = |t: Ty<'tcx>| matches!(t.kind(), ty::Uint(ty::UintTy::U8) | ty::Int(ty::IntTy::I8));\n",
    tests: ["test/corpus.test.ts", "-t", "^pointer_cast_to_the_other_sign.rs$"],
  },
  {
    name: "items-range-end-at-start",
    breaks: "a slice's `as_ptr_range()` ends where it starts, so its `offset_from` is 0 and a cursor over it ends at once (#1256)",
    file: "src/lower/pointers.rs",
    find: '                let end = helper("$ptr", vec![items.clone(), Expr::member(items.clone(), "length")]);\n',
    replace: '                let end = helper("$ptr", vec![items.clone(), Expr::int(0)]);\n',
    tests: ["test/corpus.test.ts", "-t", "^slice_pointer_range_walked_by_a_cursor.rs$"],
  },
  {
    name: "items-range-unspilled",
    breaks: "a slice's `as_ptr_range()` of an expression reads it twice, a `$view` of a `Vec`'s part made twice, two places, so a cursor's ends are of two views and it never meets its end (#1256)",
    file: "src/lower/pointers.rs",
    find: '                let items = match items.reads_same() {\n                    true => items,\n                    false => self.spill("items", items, out),\n                };\n                let end',
    replace: "                let end",
    tests: ["test/corpus.test.ts", "-t", "^slice_pointer_range_walked_by_a_cursor.rs$"],
    snapshots: true,
  },
  {
    name: "swap-each-of-one",
    breaks: "`ptr::swap_nonoverlapping(a, b, n)` swaps only the first item of each (#1256)",
    file: "src/lower/pointers.rs",
    find: '            PointerOp::SwapEach => helper("$ptrSwap", vec![arg(), arg(), arg()]),\n',
    replace: '            PointerOp::SwapEach => helper("$ptrSwap", vec![arg(), arg()]),\n',
    tests: ["test/corpus.test.ts", "-t", "^items_swapped_nonoverlapping.rs$"],
  },
  {
    name: "byte-offset-from-of-one-size",
    breaks: "`p.byte_offset_from(q)` asks `q`'s address by `p`'s pointee's size, so a byte pointer 3 bytes into a word measured from the word's is placed by a byte's stride (#1256)",
    file: "src/lower/pointers.rs",
    find: '                    helper("$addr", vec![q, self.pointee_size(origin, span)?]),\n',
    replace: '                    helper("$addr", vec![q, self.pointee_size(pointee, span)?]),\n',
    tests: ["test/corpus.test.ts", "-t", "^byte_offset_from_is_an_address_difference.rs$"],
  },
  {
    name: "byte-offset-from-reversed",
    breaks: "`p.byte_offset_from(q)` is how far `q` is past `p`, its sign the other's (#1256)",
    file: "src/lower/pointers.rs",
    find: "                Expr::bin(Op::Sub, p, q)\n",
    replace: "                Expr::bin(Op::Sub, q, p)\n",
    tests: ["test/corpus.test.ts", "-t", "^byte_offset_from_is_an_address_difference.rs$"],
  },
  {
    name: "pointer-read-uncopied",
    breaks: "`p.read()` of a `[u8; 2]` is the array in place, so a later `(*p)[0] = 9` shows in what was read, `[9, 4]` where native Rust's is `[3, 4]` (#1265)",
    file: "src/lower/pointers.rs",
    find: '            PointerOp::Read => self.copy_if_needed(self.pointee_place(arg()), pointee.expect("a pointer")),\n',
    replace: "            PointerOp::Read => self.pointee_place(arg()),\n",
    tests: ["test/corpus.test.ts", "-t", "^pointer_read_is_a_copy.rs$"],
  },
];
