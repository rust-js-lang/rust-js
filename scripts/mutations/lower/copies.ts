// Mutations of src/lower/copies.rs (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "flattened-copy-nested",
    breaks: "a copy of a struct with a flattened field, changed in place, reads and makes a key of the field's name, `range: { ...range.range }`, which the object hasn't, and an inherited getter of that name throws (#1847)",
    file: "src/lower/copies.rs",
    find: "        match self.own_object_fields(ty) {\n            Some(own) => own.into_iter().map(|(name, t, def)| (name, t, Some(def))).collect(),\n",
    replace: "        match self.own_object_fields(ty).filter(|_| false) {\n            Some(own) => own.into_iter().map(|(name, t, def)| (name, t, Some(def))).collect(),\n",
    tests: ["test/lowering.test.ts", "-t", "reads its own keys alone"],
  },


  {
    name: "closure-copy-shares-its-captures",
    breaks: "a copy of a `Copy` closure that changes what it captured, which the crate clones apart, is the same JS function, so the copy counts on from the original's count (#1211)",
    file: "src/lower/copies.rs",
    find: "                if self.own_captures_copied(ty) == Some(true) {\n                    return once(place, &|f| Expr::call_helper(\"$clone\", vec![f]));\n                }\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^(closure_cloned_in_a_holder|copied_closure_bound).rs$"],
  },
  {
    name: "closure-copy-unchanged-in-place",
    breaks: "a holder of a closure that changes what it captured, `Some(counter())`, `(f, 1)`, is no type changed in place, so its copy and its clone are the same object, sharing the closure (#1211)",
    file: "src/lower/copies.rs",
    find: "            || self.own_captures_copied(ty) == Some(true)\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^closure_cloned_in_a_holder.rs$"],
  },
  {
    name: "holder-copy-of-closure-unrefused",
    breaks: "a copy of what holds a closure that changes what it captured, which the crate never clones, a tuple's, is the same JS function, sharing its count, where a copy of the closure itself is refused (#1211)",
    file: "src/lower/copies.rs",
    find: "            ty::Tuple(parts) => parts.iter().any(|t| self.holds_by_value(t, found, depth + 1)),\n",
    replace: "            ty::Tuple(_) => false,\n",
    tests: ["test/corpus.test.ts", "-t", "^copied_closure_in_a_holder.rs$"],
  },
  {
    name: "closure-copy-refused-where-cloned-apart",
    breaks: "a copy of a closure the crate clones apart, one a `Copy` bound is given, is refused, though its copy is its clone",
    file: "src/lower/copies.rs",
    find: "            .then(|| self.krate.closure_facts.clones_apart(id))\n",
    replace: "            .then(|| false)\n",
    tests: ["test/corpus.test.ts", "-t", "^copied_closure_bound.rs$"],
  },
  {
    name: "struct-copy-of-closure-unrefused",
    breaks: "a copy of a struct holding a closure that changes what it captured, which the crate never clones, is the same JS function, sharing its count (#1211)",
    file: "src/lower/copies.rs",
    find: "            ty::Adt(adt, args) => adt\n                .all_fields()\n                .any(|f| self.holds_by_value(",
    replace: "            ty::Adt(adt, args) if false => adt\n                .all_fields()\n                .any(|f| self.holds_by_value(",
    tests: ["test/corpus.test.ts", "-t", "^copied_closure_in_a_struct.rs$"],
  },
  {
    name: "copied-closure-shared",
    breaks: "a copy of a `Copy` closure that captures something, a reference to which is made a `&dyn Fn` where the crate reads its address, is the same JS function, so `ptr::eq(&c1, &c2)` is `true` where native Rust's is `false` (#1090)",
    file: "src/lower/copies.rs",
    find: "            && self.krate.counted.copied_apart(self.tcx, ty)\n",
    replace: "            && false\n",
    tests: ["test/corpus.test.ts", "-t", "^copied_closures_compared.rs$"],
  },
  {
    name: "array-items-unmutated",
    breaks:
      "an array of `Copy` structs changed in place isn't copied: a copy's item, and a closure's by-value one, change the array they came from",
    file: "src/lower/copies.rs",
    find: "                    ty::Array(item, _) => self.contains_mutated(*item),\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "copy_mutation"],
  },
  {
    name: "generic-instance-bounds-unasked",
    breaks:
      "a `pub` generic function's `Map<I, F>`, its `I` an iterator of `u32`s, is taken for another function's cloned `Map` of a slice's `&i32`s, so its `items.map(f).rfold(..)` is refused as cloning a `Map<I, F>` where `I` isn't bound by `Clone` (#955, #1109)",
    file: "src/lower/copies.rs",
    find: "    if !parameters_given(ty, general, &mut given) || given.is_empty() {\n",
    replace: "    if !parameters_given(ty, general, &mut given) || !given.is_empty() {\n",
    tests: ["test/corpus.test.ts", "-t", "^generic_map_of_pub_code_beside_a_cloned_map.rs$"],
  },

  {
    name: "generic-instance-unmatched",
    breaks:
      "a generic function's clone of `Holder<T>` shares it, though its caller changes the `Holder<Numbers>` in place",
    file: "src/lower/copies.rs",
    find: "        _ if unknown(general) || unknown(ty) => true,",
    replace: "        _ if unknown(general) => true,",
    tests: ["test/corpus.test.ts", "-t", "copy_mutation"],
  },
  {
    name: "copy-on-read",
    breaks: "a `Copy` value read from a place is that place, not a copy",
    file: "src/lower/copies.rs",
    find: "        if self.contains_mutated(ty) && self.is_copy(ty) {",
    replace:
      "        if false && self.contains_mutated(ty) && self.is_copy(ty) {",
    tests: ["test/corpus.test.ts", "-t", "copy_mutation"],
  },
  {
    name: "library-type-shared",
    breaks:
      "a library's type is read without a copy by a crate that never changes it itself, so the copy it kept of a `Shape` its holder has is the object the library then grows in place",
    file: "src/lower/copies.rs",
    find: "                (foreign.in_library(adt.did()) && foreign.changed_by_a_library(ty))\n",
    replace: "                false\n",
    tests: [
      "test/crates.test.ts",
      "-t",
      "(two crates: copy_enum_library_changed|three crates: library_type_changed_by_another_library)",
    ],
  },
  {
    name: "library-vec-shared",
    breaks:
      "a component's `Vec` of its props, given to a library's function that pushes into it, isn't copied first, so React's second render is given the pushed one (#1871)",
    file: "src/lower/copies.rs",
    find: "            || self.krate.foreign.changed_by_a_library(ty)\n            || self",
    replace: "            || self",
    tests: ["test/react-kept.test.ts", "-t", "a library changes in place"],
  },
  {
    name: "library-tuple-shared",
    breaks:
      "a crate's copy of a tuple, which a library changes through a `&mut`, is the tuple, so both change",
    file: "src/lower/copies.rs",
    find: "            ty::Tuple(_) | ty::Array(..) => self.krate.compiled_once || foreign.changed_by_a_library(ty),\n",
    replace: "            ty::Tuple(_) | ty::Array(..) => false,\n",
    tests: ["test/crates.test.ts", "-t", "two crates: tuple"],
  },
  {
    name: "library-copy-enum-shared",
    breaks: "a copy of a library's `Copy` enum with fields is the same object",
    file: "src/lower/copies.rs",
    find: "            && (self.krate.foreign.compiled_by_rust_js(adt.did()) || self.is_std_type(ty, StdItem::Result)))\n",
    replace:
      "            && (adt.did().is_local() || self.is_std_type(ty, StdItem::Result)))\n",
    tests: ["test/crates.test.ts", "-t", "two crates: copy_enum"],
  },
  {
    name: "functions-can-change",
    breaks:
      "a static of a function, a closure or a `dyn` is taken to change in place, and refused",
    file: "src/lower/copies.rs",
    find: "            ty::FnPtr(..) | ty::FnDef(..) | ty::Closure(..) | ty::Dynamic(..) => true,",
    replace:
      "            ty::FnPtr(..) | ty::FnDef(..) | ty::Closure(..) | ty::Dynamic(..) => false,",
    tests: ["test/corpus.test.ts", "-t", "static_fns"],
  },
  {
    name: "copy-into-box",
    breaks:
      "a copy of an `Option<Option<[i32; 2]>>` that's `Some(None)` copies the box as an array",
    file: "src/lower/copies.rs",
    find: "                    let boxed = item != inner;",
    replace: "                    let boxed = false;",
    tests: ["test/corpus.test.ts", "-t", "nested_options"],
    snapshots: true,
  },
  {
    name: "crossing-mixed-enum-not-copied",
    breaks:
      "an enum with a fieldless variant another crate may change isn't copied where it's read, so the library's change in place of what holds one shows in the copy its consumer kept",
    file: "src/lower/copies.rs",
    find: "        shared && self.may_be_object(ty)",
    replace: "        shared && self.is_object(ty)",
    tests: [
      "test/crates.test.ts",
      "-t",
      "two crates: copy_enum_library_changed",
    ],
  },
  {
    name: "untagged-copy-shared",
    breaks:
      "a copy of a `Copy` untagged enum is a tagged one's, `{ ...spot, _0: { ...spot._0 } }`, a payload it hasn't",
    file: "src/lower/copies.rs",
    find: "                if self.untagged(ty).is_some() {\n                    let itself = self.untagged_changed_itself(ty);\n",
    replace:
      "                if false {\n                    let itself = self.untagged_changed_itself(ty);\n",
    tests: ["test/corpus.test.ts", "-t", "untagged_enums"],
    snapshots: true,
  },
  {
    name: "copy-js-object-unshared",
    breaks:
      "a static holding react's `ElementType` is refused: `statics of type Variant`",
    file: "src/lower/copies.rs",
    find: "                is_copy(tcx, ty::TypingEnv::fully_monomorphized(), ty)\n",
    replace: "                false\n",
    tests: ["test/jsx-types.test.ts", "-t", "ElementType, rendered as a tag"],
  },
  {
    name: "fn-param-copied",
    breaks:
      "props holding an `F: Fn() + Copy` are kept whole, `param.next`, as if the function needed a copy of its own",
    file: "src/lower/copies.rs",
    find: "        if self.is_unknown(ty) && self.is_callable(ty) {\n            return self.krate.closure_facts.functions_clone_apart();",
    replace:
      "        if false {\n            return self.krate.closure_facts.functions_clone_apart();",
    tests: ["test/jsx-props.test.ts", "-t", "Fn type parameter"],
  },
  {
    name: "unchanging-closure-uncopied",
    breaks:
      "a closure that changes nothing it captured is taken for one that does, and its copy rejected",
    file: "src/lower/copies.rs",
    find: "        (super::analysis::writes_what_it_took(self.tcx, id, args) && self.is_copy(ty))\n",
    replace: "        (self.is_copy(ty))\n",
    tests: ["test/corpus.test.ts", "-t", "moved_closure"],
  },
  {
    name: "mutated-type-any-instance",
    breaks:
      "a type's arguments aren't compared: a mutated `Pair<u32>` makes every `Pair<..>` copied where it's read, `Pair<bool>` too, which nothing changes",
    file: "src/lower/copies.rs",
    find: "(Some(arg), Some(general)) => instance_of_in(tcx, typing_env, arg, general),",
    replace: "(Some(_), Some(_)) => true,",
    tests: [
      "test/lowering-functions.test.ts",
      "-t",
      "copies are made for the mutated instantiations of a generic type only",
    ],
  },
  {
    name: "maybe-uninit-unshareable",
    breaks: "a constant `MaybeUninit` is refused",
    file: "src/lower/copies.rs",
    find: "            ty::Adt(adt, args) if super::recognition::is_std_def(tcx, adt.did(), StdItem::MaybeUninit) => {",
    replace: "            ty::Adt(adt, args) if false => {",
    tests: ["test/corpus.test.ts", "-t", "slice_uninit"],
  },
  {
    name: "string-unshareable",
    breaks:
      "a `static` `String` is refused, as if something could change it in place",
    file: "src/lower/copies.rs",
    find: "            ty::Adt(adt, _) if tcx.is_lang_item(adt.did(), LangItem::String) => true,\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "const_block_values"],
  },
  {
    name: "local-key-unshared",
    breaks:
      "a constant holding a thread-local by reference, a memo component, is refused",
    file: "src/lower/copies.rs",
    find: "            ty::Adt(adt, _) if super::recognition::is_std_def(tcx, adt.did(), StdItem::LocalKey) => true,",
    replace:
      "            ty::Adt(adt, _) if super::recognition::is_std_def(tcx, adt.did(), StdItem::LocalKey) => false,",
    tests: [
      "test/jsx-evaluation-order.test.ts",
      "-t",
      "thread-local by reference as its variable",
    ],
  },
  {
    name: "vec-const-use-shared",
    breaks:
      "each use of a `Vec` constant is its one JS array, so changing one changes every other",
    file: "src/lower/copies.rs",
    find: "        self.contains_mutated(ty) || (!self.is_copy(ty) && !shareable(self.tcx, ty))\n",
    replace: "        self.contains_mutated(ty)\n",
    tests: ["test/corpus.test.ts", "-t", "^const_block_vec.rs$"],
  },
  {
    name: "const-use-uncloned",
    breaks:
      "a use of a `Vec` constant is copied as a `Copy` value is, which is the array itself",
    file: "src/lower/copies.rs",
    find: "        if self.is_copy(ty) {\n            return Ok(self.copy(place, ty));\n",
    replace:
      "        if true {\n            return Ok(self.copy(place, ty));\n",
    tests: ["test/corpus.test.ts", "-t", "^const_block_vec.rs$"],
  },
  {
    name: "union-copy-unmade",
    breaks: "a union read where one of it may change is shared, not copied",
    file: "src/lower/copies.rs",
    find: "                if self.is_union(ty) {\n                    return once(place, &|u| self.union_copy(u, ty));\n",
    replace:
      "                if false {\n                    return once(place, &|u| self.union_copy(u, ty));\n",
    tests: ["test/corpus.test.ts", "-t", "^unions.rs$"],
  },
  {
    name: "union-holding-mutated-unseen",
    breaks:
      "a union holding what changes in place, unchanged itself, isn't copied",
    file: "src/lower/copies.rs",
    find: "                    ty::Adt(..) if self.is_union(ty) => self.union_holds_mutated(ty),\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^unions.rs$"],
  },
  {
    name: "type-id-static-unshared",
    breaks:
      "a `static` of a `TypeId`, its type's key, a string, isn't shareable, and is refused",
    file: "src/lower/copies.rs",
    find: "            ty::Adt(adt, _) if tcx.is_lang_item(adt.did(), LangItem::TypeId) => true,\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^type_id_statics.rs$"],
  },
  {
    name: "library-struct-unshareable",
    breaks:
      "a static of a library's struct of functions, React's `Dispatcher`, isn't made as its initializer says, and is refused",
    file: "src/lower/copies.rs",
    find: "            ty::Adt(adt, args) if !super::recognition::is_std_crate(tcx, adt.did()) && !adt.is_union() => adt",
    replace: "            ty::Adt(adt, args) if adt.did().is_local() && !adt.is_union() => adt",
    tests: ["test/crates.test.ts", "-t", "two crates: static_of_a_library_struct"],
  },
  {
    name: "std-struct-shareable",
    breaks:
      "std's struct is shared as its fields read, so a constant `BTreeMap` is one JS map, which `extend` of one use grows for the next",
    file: "src/lower/copies.rs",
    find: "            ty::Adt(adt, args) if !super::recognition::is_std_crate(tcx, adt.did()) && !adt.is_union() => adt",
    replace: "            ty::Adt(adt, args) if !adt.is_union() => adt",
    tests: ["test/corpus.test.ts", "-t", "^std_map_const.rs$"],
  },
  {
    name: "raw-pointer-not-shareable",
    breaks:
      "a static holding a raw pointer isn't made as its initializer says, and is refused",
    file: "src/lower/copies.rs",
    find: "            _ if super::pointers::raw_pointee(tcx, ty).is_some() => true,\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^pointer_constants.rs$"],
  },
  {
    name: "fresh-array-copied",
    breaks:
      "an array made here, a literal or a `.slice(..)`, is copied as it's bound, `[1, 2].slice()`, where nothing else holds it",
    file: "src/lower/copies.rs",
    find: "        if fresh && items_plain {",
    replace: "        if false && fresh && items_plain {",
    tests: ["test/corpus.test.ts", "-t", "^(slice_patterns|str_mut).rs$"],
    snapshots: true,
  },
  {
    name: "copy-not-given-called",
    breaks:
      "a copy a caller gives none of is called, and JS's `createContext(defaultValue)` throws on `TCopy.copy`",
    file: "src/lower/copies.rs",
    find: "                Expr::cond(dictionary.clone(), copy, place)\n",
    replace: "                copy\n",
    tests: [
      "test/crates-compiled-here.test.ts",
      "-t",
      "Copy bound takes a copy only where one is given",
    ],
  },
  {
    name: "library-copies-for-consumers",
    breaks:
      "a library copies its public `Copy` type where its consumer may change one, which its program doesn't, so its JS changes when another crate uses it (ADR 0914, amended)",
    file: "src/lower/copies.rs",
    find: "                    || (self.krate.compiled_once && super::library::reachable(self.tcx, adt.did()))\n",
    replace:
      "                    || (self.krate.library && super::library::reachable(self.tcx, adt.did()))\n",
    tests: ["test/crates-compiled-here.test.ts", "-t", "program's, alone or a dependency"],
  },
  {
    name: "own-copy-option-tested",
    breaks:
      "a copy of an `Option` of a value that's its own copy tests it, `value == null ? value : value` (ADR 0051, amended)",
    file: "src/lower/copies.rs",
    find: "                    if !boxed && self.copy_parts(probe.clone(), item, stored).kind == probe.kind {\n",
    replace: "                    if false {\n",
    tests: ["test/corpus.test.ts", "-t", "^unboxed_type_params.rs$"],
    snapshots: true,
  },
  {
    name: "untagged-arrays-copy-shared",
    breaks:
      "a copy of an untagged enum of arrays changed in place is the array itself, which `*place = ..` then changes too (rd490)",
    file: "src/lower/copies.rs",
    find: "            && self.mutated_itself(ty)\n",
    replace: "            && false\n",
    tests: ["test/corpus.test.ts", "-t", "untagged_told_by_length"],
  },
  {
    name: "untagged-arrays-slice-unread",
    breaks:
      "a copy of an untagged enum of arrays tests each variant to copy its items, where its `slice()` copies any (rd490)",
    file: "src/lower/copies.rs",
    find: "        super::untagged::arrays_in_place(self.tcx, adt, args)\n            && adt.variants().iter().all(|v| {\n",
    replace: "        false\n            && adt.variants().iter().all(|v| {\n",
    tests: ["test/corpus.test.ts", "-t", "untagged_told_by_length"],
    snapshots: true,
  },
  {
    name: "items-changed-as-vecs-unasked",
    breaks:
      "a boxed slice's clone is the same array where the `Vec` its `into_vec()` makes is changed in place (#1290)",
    file: "src/lower/copies.rs",
    find: "            || self.krate.changed_vecs.iter().any(|&changed| vec_of(changed))\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^clone_changed_as_a_vec_or_a_boxed_slice.rs$"],
  },
  {
    name: "items-changed-as-slices-unasked",
    breaks:
      "a `Vec`'s clone is the same array where the boxed slice its `into_boxed_slice()` makes is changed in place (#1290)",
    file: "src/lower/copies.rs",
    find: "            || self.contains_mutated(Ty::new_slice(self.tcx, item))\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^clone_changed_as_a_vec_or_a_boxed_slice.rs$"],
  },
  {
    name: "read-out-not-apart",
    breaks: "`ptr::read` of a `Vec` and a write there after: what was read is the `Vec` in place, and shows the write, `[8, 9]` where Rust's is `[7]` (#1351)",
    file: "src/lower/copies.rs",
    find: "        Expr::call_helper(\"$apart\", vec![place])\n    }\n",
    replace: "        self.copy_if_needed(place, ty)\n    }\n",
    tests: ["test/corpus.test.ts", "-t", "^whole_values_moved_through_pointers.rs$"],
  },
  {
    name: "read-out-apart-of-text",
    breaks: "`ptr::read` of a `String`, which no write changes in place, is `$apart(text)`, a copy of nothing (#1351)",
    file: "src/lower/copies.rs",
    find: "        if self.is_copy(ty) || !self.mut_ref_form(ty).written_in_place() {\n",
    replace: "        if self.is_copy(ty) {\n",
    tests: ["test/corpus.test.ts", "-t", "^whole_values_moved_through_pointers.rs$"],
    snapshots: true,
  },
  {
    name: "bits-copy-shares",
    breaks: "a `Copy` item's copy std makes of its bits, `copy_from_slice`'s, `partition`'s and the rest, shares an item something changes in place, so a change of a copy changes its source",
    file: "src/lower/copies.rs",
    find: "        let copied = self.copy_if_needed(Expr::var(&item_name), item);\n",
    replace: "        let copied = Expr::var(&item_name);\n",
    tests: ["test/corpus.test.ts", "-t", "^copied_of_a_copy_with_its_own_clone.rs$"],
  },
  {
    name: "moved-inline-fields-shared",
    breaks: "a move of a struct whose address is taken shares the struct inline in it, which a write over a pointer into it then changes in the value moved (#1535)",
    file: "src/lower/copies.rs",
    find: "            own(t)\n                && self.is_object(t)\n",
    replace: "            false\n                && self.is_object(t)\n",
    tests: ["test/corpus.test.ts", "-t", "^address_of_a_local_moved_out.rs$"],
  },
  {
    name: "moved-std-type-spread",
    breaks: "a move of std's type whose address is taken, a `HashMap`, is spread as its Rust fields, which its JS `Map` hasn't, not refused (#1551)",
    file: "src/lower/copies.rs",
    find: "            ty::Adt(adt, _) => !self.recognition().in_sysroot(adt.did()),\n",
    replace: "            ty::Adt(..) => true,\n",
    tests: ["test/diagnostics.test.ts", "-t", "a HashMap whose address is taken, moved"],
  },
  {
    name: "moved-array-items-shared",
    breaks: "an array moved out of a place a pointer is into shares its items, which a write over that place then changes (#1553)",
    file: "src/lower/copies.rs",
    find: "                ty::Array(item, _) if inline(item) => {\n",
    replace: "                ty::Array(item, _) if false && inline(item) => {\n",
    tests: ["test/corpus.test.ts", "-t", "^address_of_an_array_item_kept_when_written_whole.rs$"],
  },
  {
    name: "write-result-copied-as-tagged",
    breaks: "a copy of a write's result where one is held in a `RefCell` is a tagged object's, `{ ...undefined }`, so `Ok(())` reads as an `Err` (#1757)",
    file: "src/lower/copies.rs",
    find: "                if self.is_write_result(ty) {\n                    return place;\n",
    replace: "                if false {\n                    return place;\n",
    tests: ["test/corpus.test.ts", "-t", "^(write_result_copied_where_one_is_held_in_a_cell)\\.rs$"],
  },
  {
    name: "given-uncopied-though-lent",
    breaks: "a caller's value it lends `&mut` to a closure given the call too is given uncopied, so the callee sees the closure change it, `incidence_at(c, || c.y += 1)` (#1801)",
    file: "src/lower/copies.rs",
    find: "                Rvalue::Ref(_, kind, lent) => kind.mutability().is_mut() && lent.local == place.local,\n",
    replace: "                Rvalue::Ref(_, kind, lent) => false && kind.mutability().is_mut() && lent.local == place.local,\n",
    tests: ["test/corpus.test.ts", "-t", "^copy_given_to_a_function_that_only_reads_it.rs$"],
  },
  {
    name: "given-uncopied-though-pointed",
    breaks: "a caller's value a raw pointer is taken to is given uncopied, so the callee sees a write through the pointer change it (#1801)",
    file: "src/lower/copies.rs",
    find: "                Rvalue::RawPtr(_, lent) => lent.local == place.local,\n",
    replace: "                Rvalue::RawPtr(_, lent) => false && lent.local == place.local,\n",
    tests: ["test/corpus.test.ts", "-t", "^copy_given_to_a_function_that_only_reads_it.rs$"],
  },
  {
    name: "given-uncopied-through-a-reference",
    breaks: "a value read through a reference, `*r` of an `r: &mut`, is given uncopied, so the callee sees a write through `r` change it (#1801)",
    file: "src/lower/copies.rs",
    find: "        let own = !place.is_indirect();\n",
    replace: "        let own = true;\n",
    tests: ["test/corpus.test.ts", "-t", "^copy_given_to_a_function_that_only_reads_it.rs$"],
  },
  {
    name: "given-uncopied-never",
    breaks: "no argument is given uncopied, so `lamp_at(c)` copies `c` though `incidence_at` only reads it (#1801)",
    file: "src/lower/copies.rs",
    find: "        only_read && own && !lent\n",
    replace: "        false && only_read && own && !lent\n",
    tests: ["test/corpus.test.ts", "-t", "^copy_given_to_a_function_that_only_reads_it.rs$"],
    snapshots: true,
  },
  {
    name: "variant-groups-never-merged",
    breaks: "variants of the same fields are each copied, cloned and compared apart, each with its own test, so a nested enum's copy is written once a variant, as 4 to the power of its depth, and a `Result<T, T>`'s clone tests `Ok` and `Err` for the same `{ ...value, _0: .. }` (#1821)",
    file: "src/lower/copies.rs",
    find: "            match (groups.copied.iter_mut()).find(|g| {\n                g.fields == fields\n",
    replace: "            match (groups.copied.iter_mut()).find(|g| {\n                false && g.fields == fields\n",
    snapshots: true,
    tests: ["test/corpus.test.ts", "-t", "^(enum_variants_of_the_same_fields_copied_and_compared_once|clone_from_of_a_result_given_to_generic_code).rs$"],
  },
  {
    name: "variants-tested-by-the-group",
    breaks: "where one group's are all the variants copied, its test is still of them, `e.TAG === \"First\" || e.TAG === \"Second\" || ..`, not of the fewer others, `e === \"Empty\" ? e : ..`, nor none at all, `{ ...value, _0: .. }` of a `Result<T, T>` (#1821)",
    file: "src/lower/copies.rs",
    find: "            && others.len() < indices.len()\n",
    replace: "            && others.len() > usize::MAX - 1\n",
    snapshots: true,
    tests: ["test/corpus.test.ts", "-t", "^(enum_variants_of_the_same_fields_copied_and_compared_once|clone_from_of_a_result_given_to_generic_code).rs$"],
  },
  {
    name: "library-vec-items-shared",
    breaks: "a clone of a boxed slice whose `Vec` a library changes in place, `mid::pushed(boxed.clone().into_vec())`, is the array, so the library's push changes the crate's own (#1871)",
    file: "src/lower/copies.rs",
    find: "            || self.krate.foreign.changes_a_vec_of(item)\n",
    replace: "",
    tests: ["test/crates.test.ts", "-t", "three crates: library_type_changed_by_another_library"],
  },
  {
    name: "stored-objects-shallow",
    breaks: "a copy of what a library stores copies its top object alone, `{ ...PAIR }`, so `mid`'s change of `pair.first` changes `PAIR`'s (#1907)",
    file: "src/lower/copies.rs",
    find: "        self.contains_mutated(ty) || (stored && self.holds_plain_object(ty, 0))\n",
    replace: "        self.contains_mutated(ty) || (stored && false)\n",
    tests: ["test/crates.test.ts", "-t", "three crates: library_value_changed_by_another_library"],
  },
  {
    name: "copy-evidence-of-a-copy-bound-alone",
    breaks: "a `T: Copied` whose `Copy` is a supertrait's isn't copied, `let copy = value`, so what changes `copy` changes `value` too (#1925)",
    file: "src/lower/copies.rs",
    find: "        self.evidence_for(ty::TraitRef::new(self.tcx, copy, [ty]))\n",
    replace: "        self.given_evidence(|tr| tr.self_ty() == ty && tr.def_id == copy)\n",
    tests: ["test/corpus.test.ts", "-t", "^copy_through_a_supertrait.rs$"],
  },
  {
    name: "copied-source-of-an-option-unread",
    breaks: "an `Option`'s copy, `x == null ? x : { ...x }`, says nothing of what it copies, so its temporary is `value$1`, not `at` (#1923)",
    file: "src/lower/copies.rs",
    find: "                    && copied_source(copied).is_some_and(|inner| inner.kind == source.kind) =>\n",
    replace: "                    && false =>\n",
    tests: ["test/library-kept-copies.test.ts", "-t", "literal, new box and constant"],
  },
  {
    name: "within-bounds-of-what-is-left",
    breaks:
      "a bound left with another function's type parameters is taken to hold, so a trait default's `Map` of its own closure, `|held: U| Held { held }`, is taken for a cloned `Map<I, F>` of `F: FnMut(i32) -> i32` and refused, its items dropping (#1895)",
    file: "src/lower/copies.rs",
    find: "        let bound = bound.fold_with(&mut giving).fold_with(&mut any);\n",
    replace: "        let bound = bound.fold_with(&mut giving);\n",
    tests: ["test/corpus.test.ts", "-t", "^map_of_another_closure_not_cloned_apart.rs$|^generic_map_past_a_take_cloned_beside_another_bound.rs$"],
  },
  {
    name: "identity-map-shared-though-written",
    breaks: "a `Copy` array mapped by the identity in JS, `a.map(String::from)`, is the same array as its result where one is written after, `d[0] = String::from(\"z\")`, which then writes the array mapped too (#1882)",
    file: "src/lower/copies.rs",
    find: "        let told_apart = (self.contains_mutated(from) || self.contains_mutated(to)) && self.is_copy(from);\n",
    replace: "        let told_apart = self.contains_mutated(from) && self.is_copy(from);\n",
    tests: ["test/corpus.test.ts", "-t", "^array_map_of_an_identity.rs$"],
  },
  {
    name: "elided-unrecorded",
    breaks: "a library records none of the copies it leaves out, so a consumer that changes a `Spot` its `first_spot` hands out uses the library's JS, which hands out the slice's own (#2112's change D)",
    file: "src/lower/copies.rs",
    find: "        if self.krate.library && !self.krate.compiled_once && self.may_be_object(ty) {\n",
    replace: "        if false && self.krate.library && !self.krate.compiled_once && self.may_be_object(ty) {\n",
    tests: ["test/crates.test.ts", "-t", "two crates: library_elided_copies_said"],
  },
  {
    name: "elided-of-changed",
    breaks:
      "a library says it leaves out the copies of a type its own crate changes, whose JS copies it, so a consumer that changes it too compiles its own of the function, where it is imported, `Dep.firstNotch(notches)` (rd606, #2112's change D)",
    file: "src/lower/copies.rs",
    find: "            if !mutated {\n                found.push(ty);\n            }\n",
    replace: "            {\n                found.push(ty);\n            }\n",
    tests: [
      "test/crates-compiled-here.test.ts",
      "-t",
      "a library's generic function is its program's",
    ],
  },
  {
    name: "elided-walk-parts-unrecorded",
    breaks: "a copy decision records only the type it asked of, not the parts its walk found unchanged, so a consumer that changes a part of what a library's function left a copy of out uses the library's JS (#2204)",
    file: "src/lower/copies.rs",
    find: "        for &unchanged in found.iter() {\n",
    replace: "        for &unchanged in found.iter().filter(|&&t| t == ty) {\n",
    tests: ["test/crates.test.ts", "test/crates-compiled-here.test.ts", "test/cross-crate-copies.test.ts"],
  },
  {
    name: "changed-in-place-records",
    breaks: "asking whether a type may be changed in place records it in a library's contract as a copy left out, so #2201's ask at a move makes a consumer that changes it compile the library's function itself (#2204)",
    file: "src/lower/copies.rs",
    find: "        self.walks().unchanged.borrow_mut().push(Vec::new());\n        let mutated = self.contains_mutated(ty);\n        self.walks().unchanged.borrow_mut().pop();\n",
    replace: "        let mutated = self.contains_mutated(ty);\n",
    tests: ["test/cross-crate-copies.test.ts", "-t", "a library's contract lists no copy its function only asked about"],
  },
  {
    name: "placed-field-copy-shared",
    breaks:
      "a copy of the crate's `Copy` iterator whose field is held in a place of its own shares the place: stepping `p` steps its copy `r` (#2255)",
    file: "src/lower/copies.rs",
    find: "                    if def.is_some_and(|def| self.placed_field(def, t)) {\n                        let read = self.held(Expr::member(place.clone(), name.clone()), def, t);\n                        let field = self.holding(self.copy_parts(read, t, stored), def, t);\n",
    replace: "                    if false {\n                        let read = self.held(Expr::member(place.clone(), name.clone()), def, t);\n                        let field = self.holding(self.copy_parts(read, t, stored), def, t);\n",
    tests: ["test/corpus.test.ts", "-t", "^iterator_field_in_its_place_copied_cloned_compared\\.rs$"],
  },
  {
    name: "placed-variant-copy-shared",
    breaks:
      "a copy of the crate's `Copy` enum whose variant's field is held in a place of its own, `Step::On(Tri)`, shares the place: stepping `on` steps `kept` (#2255)",
    file: "src/lower/copies.rs",
    find: "                                if self.placed_field(def, t) {\n                                    let read = self.held(Expr::member(e.clone(), name.clone()), Some(def), t);\n",
    replace: "                                if false {\n                                    let read = self.held(Expr::member(e.clone(), name.clone()), Some(def), t);\n",
    tests: ["test/corpus.test.ts", "-t", "^iterator_field_in_its_place_copied_cloned_compared\\.rs$"],
  },
  {
    name: "variants-grouped-of-other-placed-fields",
    breaks:
      "variants of the same fields' types, one held in a place of its own and one not, `A(I)` and `B(Tri)` of `Two<Tri>`, are grouped as one, so the group's first's definitions copy the other's field as a place, `{ value: e._0.value }` of a bare string (#2255)",
    file: "src/lower/copies.rs",
    find: "                    && placed(g.first, &g.fields) == placed(variant, &fields)\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^iterator_field_in_its_place_copied_cloned_compared\\.rs$"],
  },
  {
    name: "placed-part-copy-shared",
    breaks:
      "a copy of a `Copy` tuple struct whose field is held in a place of its own shares the place: stepping `w` steps `v` (#2255)",
    file: "src/lower/copies.rs",
    find: "                            .is_some_and(|def| self.placed_field(def, t))\n                        {\n                            self.holding_part(self.copy_parts(",
    replace: "                            .is_some_and(|def| self.placed_field(def, t) && false)\n                        {\n                            self.holding_part(self.copy_parts(",
    tests: ["test/corpus.test.ts", "-t", "^iterator_field_in_its_place_copied_cloned_compared\\.rs$"],
  },
];
