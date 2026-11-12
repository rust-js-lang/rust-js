// Mutations of src/lower/traits/dictionaries.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "deque-a-generic-reader",
    breaks: "a `VecDeque<u8>` given to generic code as its reader is given `$dequeReader`, which has no `read` or `fill_buf`, where it's refused (#1302)",
    file: "src/lower/traits/dictionaries.rs",
    find: "            Some(ByteSource::Deque) => Err(self.unsupported(\n                span,\n                &format!(\n                    \"a `VecDeque<u8>` as generic code's `io::{}`, whose `read` and `fill_buf` read its front slice alone (#1302)\",\n                    self.tcx.item_name(tr.def_id)\n                ),\n            )),\n",
    replace: '            Some(ByteSource::Deque) => Ok(Expr::helper("$dequeReader")),\n',
    tests: ["test/diagnostics.test.ts", "-t", "VecDeque given to generic code"],
  },
  {
    name: "derived-order-dictionary-inline",
    breaks: "a derived type's `PartialOrd` and `Ord` dictionaries are made in place at each call, `$partialOrd({ .. })` again and again, not its module's `const` (#944)",
    file: "src/lower/traits/dictionaries.rs",
    find: "                let base = self.dictionary_name(tr);\n                return self.module_dictionary((tr.def_id, ty, &base), make);\n",
    replace: "                return make(self);\n",
    tests: ["test/lowering-dictionaries.test.ts", "-t", "an Ord dictionary holds its PartialOrd"],
  },
  {
    name: "std-ord-dictionary-without-supertraits",
    breaks: "an `Ord` dictionary std's derive makes has no `PartialOrd()`, which generic `<` reads (#944)",
    file: "src/lower/traits/dictionaries.rs",
    find: "                    let mut props = cx.supertrait_entries(tr, span)?;\n",
    replace: "                    let mut props = Vec::new();\n",
    tests: ["test/corpus.test.ts", "-t", "^generic_compare_by_impls.rs$"],
  },
  {
    name: "generic-dictionary-repeats-unnamed",
    breaks:
      "a generic impl's dictionary a body asks twice or more, `holderTouch(TTouch)`, is written each time, not read once at its top (ADR 0979, amended, #691)",
    file: "src/lower/traits/dictionaries.rs",
    find: "        let named = if is_impl_call(&made) {\n",
    replace: "        let named = if false {\n",
    tests: ["test/corpus.test.ts", "-t", "^(generic_dictionary_declared_once|copied_default_through_its_entry).rs$"],
    snapshots: true,
  },
  {
    name: "impl-nested-mut-unchecked",
    breaks:
      "a generic impl's dictionary of an object's, `impl<T: Pick> Wrap for Holder<T>` of a `Holder<Clicks>`, is made though its `T::pick(Some(&mut ..))` gives `Clicks`'s entry generic code's box: what it writes is lost (#685)",
    file: "src/lower/traits/dictionaries.rs",
    find: "        if let Some(here) = self.impl_nested_mut_object(impl_id, impl_args) {\n",
    replace: "        if let Some(here) = None::<Ty<'tcx>> {\n",
    tests: ["test/diagnostics.test.ts", "-t", "a generic impl's &mut T"],
  },
  {
    name: "dictionary-repeats-unnamed",
    breaks:
      "a dictionary its body writes twice or more is written each time, not declared once at its top (ADR 0979)",
    file: "src/lower/traits/dictionaries.rs",
    find: "            self.keep_dictionary(&made, name, shared);\n",
    replace: "",
    tests: [
      "test/corpus.test.ts",
      "-t",
      "^(std_inspect|nested_item_drops|held_peekable|writer_fn_pointer).rs$",
    ],
    snapshots: true,
  },
  {
    name: "dictionary-of-steps-declared",
    breaks:
      "a dictionary of helpers alone, `{ next: $next }`, is declared at its body's top, where inline it reads as well (ADR 0979)",
    file: "src/lower/traits/dictionaries.rs",
    find: "            && holds_a_function(&made)\n",
    replace: "",
    tests: [
      "test/corpus.test.ts",
      "-t",
      "^(std_inspect|nested_item_drops|held_peekable|writer_fn_pointer).rs$",
    ],
    snapshots: true,
  },
  {
    name: "identity-copy-param-refused",
    breaks:
      "a type parameter whose `Copy` bound takes no copy function, each value its callers give its own copy, is refused a `Copy` dictionary, `Copy without representation evidence`, where std's compiled `Copied` asks one (ADR 0938)",
    file: "src/lower/traits/dictionaries.rs",
    find: "                && !self.krate.given.copied.contains(&(item, param.index))\n            {\n                return Ok(Expr::undefined());\n",
    replace:
      "                && false\n            {\n                return Ok(Expr::undefined());\n",
    tests: ["test/iterator-forms.test.ts", "-t", "^heap_pop_of_a_param with"],
  },
  {
    name: "mut-ref-extension-of-evidence-unseen",
    breaks:
      "a `&mut R`'s `DoubleEndedIterator`, of a type parameter given one, isn't of the dictionary `R` was given: `self.inner.by_ref().map(f).rfold(..)` is refused, `the DoubleEndedIterator of a R` (ADR 0937)",
    file: "src/lower/traits/dictionaries.rs",
    find: "                && self.is_unknown(*inner)\n            {\n                let of = self.dictionary(ty::TraitRef::new(self.tcx, tr.def_id, [*inner]), span)?;\n                return Ok(self.mut_ref_extension_dictionary(tr.def_id, of));\n",
    replace:
      "                && false\n            {\n                let of = self.dictionary(ty::TraitRef::new(self.tcx, tr.def_id, [*inner]), span)?;\n                return Ok(self.mut_ref_extension_dictionary(tr.def_id, of));\n",
    tests: ["test/iterator-forms.test.ts", "-t", "^held_enumerate with"],
  },
  {
    name: "evidence-unnormalized",
    breaks:
      "`shown(&mut Words ..)` is given the dictionary of `<Words as Source>::Item: Debug`, not `String: Debug`, and it's refused",
    file: "src/lower/traits/dictionaries.rs",
    find: "            .try_normalize_erasing_regions(self.typing_env, ty::Unnormalized::new_wip(tr))\n            .unwrap_or(tr);\n",
    replace:
      "            .try_normalize_erasing_regions(self.typing_env, ty::Unnormalized::new_wip(tr))\n            .map(|_| tr)\n            .unwrap_or(tr);\n",
    tests: ["test/corpus.test.ts", "-t", "associated_types"],
  },
  {
    name: "object-dictionary-unselected",
    breaks: "a `dyn` given to generic code has no dictionary, and is refused",
    file: "src/lower/traits/dictionaries.rs",
    find: "        if let Ok(ImplSource::Builtin(BuiltinImplSource::Object(_), _)) = selected {",
    replace:
      "        if let Ok(ImplSource::Builtin(BuiltinImplSource::Object(_), _)) = selected && false {",
    tests: ["test/corpus.test.ts", "-t", "dyn_generic"],
  },
  {
    name: "object-mut-receiver-unboxed",
    breaks:
      "a `&mut self` method of a `dyn` in generic code takes generic code's box for the pair, and JS throws",
    file: "src/lower/traits/dictionaries.rs",
    find: "            let pair = match mutable {\n                true => Expr::held(object.clone()),",
    replace:
      "            let pair = match mutable {\n                true => object.clone(),",
    tests: ["test/corpus.test.ts", "-t", "dyn_generic"],
  },
  {
    name: "object-supertraits-missing",
    breaks:
      "a `dyn`'s dictionary in generic code has no supertraits, and a supertrait's method throws",
    file: "src/lower/traits/dictionaries.rs",
    find: "            .ok_or_else(|| self.no_evidence(span, tr))?;\n        let mut props = self.supertrait_entries(tr, span)?;\n",
    replace:
      "            .ok_or_else(|| self.no_evidence(span, tr))?;\n        let mut props = Vec::new();\n",
    tests: ["test/corpus.test.ts", "-t", "dyn_generic"],
  },
  {
    name: "to-string-dictionary-unmade",
    breaks: "a `T: ToString` is given no dictionary for a concrete type",
    file: "src/lower/traits/dictionaries.rs",
    find: "            if to_string {\n",
    replace: "            if false && to_string {\n",
    tests: ["test/corpus.test.ts", "-t", "evidence_paths"],
  },
  {
    name: "std-as-ref-dictionary-missing",
    breaks:
      '`shout("hi")` of an `S: AsRef<str>` has no dictionary for a `&str`\'s',
    file: "src/lower/traits/dictionaries.rs",
    find: "            && self.recognition().as_ref_is_itself(ty, tr.args.type_at(1))\n        {\n            // Through a counted",
    replace: "            && false\n        {\n            // Through a counted",
    tests: ["test/corpus.test.ts", "-t", "generic_as_ref"],
  },
  {
    name: "wrapping-dictionary-unknown",
    breaks: "a `T: Mul` given a `Wrapping<u8>` has no dictionary",
    file: "src/lower/traits/dictionaries.rs",
    find: "            && let Some(newtype) = self.recognition().int_newtype(ty.peel_refs())\n",
    replace:
      "            && let Some(newtype) = self.recognition().int_newtype(ty.peel_refs()).filter(|_| false)\n",
    tests: ["test/corpus.test.ts", "-t", "wrapping_type"],
  },
  {
    name: "dyn-item-drops-left-out",
    breaks:
      "a `dyn`'s dictionary has no drop for the associated type its type names, so generic code given one drops nothing",
    file: "src/lower/traits/dictionaries.rs",
    find: "        }\n        props.extend(self.item_drops(tr, false, span)?);\n        Ok(Expr::object(props))\n",
    replace: "        }\n        Ok(Expr::object(props))\n",
    tests: ["test/corpus.test.ts", "-t", "associated_type_drops"],
  },
  {
    name: "string-writer-no-dictionary",
    breaks:
      "a `String` or a `Formatter` given to generic code that writes to any writer has no dictionary, and it's refused",
    file: "src/lower/traits/dictionaries.rs",
    find: "if text(ty) {",
    replace: "if false && text(ty) {",
    tests: ["test/corpus.test.ts", "-t", "generic_writers"],
  },
  {
    name: "std-radix-dictionary-missing",
    breaks:
      "`impl<T: LowerHex> LowerHex for Bits<T>` of a `u8` has no `u8: LowerHex` to call",
    file: "src/lower/traits/dictionaries.rs",
    find: "            && let Some(radix) = self.radix_trait(tr.def_id)\n",
    replace:
      "            && let Some(radix) = self.radix_trait(tr.def_id).filter(|_| false)\n",
    tests: ["test/corpus.test.ts", "-t", "user_fmt_traits"],
  },
  {
    name: "marker-dyn-no-evidence",
    breaks:
      "a `T: Marker` made a `dyn` has no dictionary, as no bound passed one, and doesn't compile",
    file: "src/lower/traits/dictionaries.rs",
    find: "        if tr.self_ty().has_param() && is_marker(self.tcx, self.krate.foreign, tr.def_id) {",
    replace:
      "        if false && tr.self_ty().has_param() && is_marker(self.tcx, self.krate.foreign, tr.def_id) {",
    tests: ["test/traits.test.ts", "-t", "nothing in it passes no dictionary"],
  },
  {
    name: "marker-dyn-no-drop",
    breaks:
      "a `T: Marker` made a `dyn` isn't dropped: its dictionary has no `$drop`",
    file: "src/lower/traits/dictionaries.rs",
    find: "            let mut props = self.supertrait_entries(tr, span)?;\n            props.extend(self.dictionary_drop(tr.self_ty(), span)?);\n            return Ok(Expr::object(props));",
    replace:
      "            let props = self.supertrait_entries(tr, span)?;\n            return Ok(Expr::object(props));",
    tests: ["test/traits.test.ts", "-t", "nothing in it passes no dictionary"],
  },
  {
    name: "std-deref-dictionary-missing",
    breaks:
      "std's `Deref` of a `String` or a `Box` given to generic code has no dictionary",
    file: "src/lower/traits/dictionaries.rs",
    find: "        if self.tcx.is_lang_item(tr.def_id, LangItem::Deref) && !self.has_user_impl(tr.def_id, ty) {\n",
    replace: "        if false {\n",
    tests: ["test/corpus.test.ts", "-t", "^generic_deref.rs$"],
  },
  {
    name: "number-assign-dictionary-missing",
    breaks:
      "std's `AddAssign` of numbers, given to generic code, has no dictionary",
    file: "src/lower/traits/dictionaries.rs",
    find: "            && (self.primitive_operands(tr) || (op == BinOp::Add && tr.args.types().all(text)))\n",
    replace: "            && (op == BinOp::Add && tr.args.types().all(text))\n",
    tests: ["test/corpus.test.ts", "-t", "^generic_compound_assign.rs$"],
  },
  {
    name: "text-assign-dictionary-missing",
    breaks:
      "std's `AddAssign<&str>` of a `String`, given to generic code, has no dictionary",
    file: "src/lower/traits/dictionaries.rs",
    find: "            && (self.primitive_operands(tr) || (op == BinOp::Add && tr.args.types().all(text)))\n",
    replace: "            && self.primitive_operands(tr)\n",
    tests: ["test/corpus.test.ts", "-t", "^generic_compound_assign.rs$"],
  },
  {
    name: "slice-index-inclusive-as-exclusive",
    breaks: "`..=end`'s dictionary is `..end`'s, one item short",
    file: "src/lower/traits/dictionaries.rs",
    find: '                    Some((Unbounded, Field("Included", _))) => Some("toInclusive"),\n',
    replace:
      '                    Some((Unbounded, Field("Included", _))) => Some("to"),\n',
    tests: ["test/corpus.test.ts", "-t", "^slice_index_generic.rs$"],
  },
  {
    name: "any-dictionary-without-drop",
    breaks:
      "`Any`'s dictionary has no `$drop`, so a `Box<dyn Any>` of a value with a destructor never drops it",
    file: "src/lower/traits/dictionaries.rs",
    find: "                type_id,\n            )];\n            props.extend(self.dictionary_drop(tr.self_ty(), span)?);\n",
    replace: "                type_id,\n            )];\n",
    tests: ["test/corpus.test.ts", "-t", "^dyn_std_destructors.rs$"],
  },
  {
    name: "extend-dictionary-none",
    breaks:
      "a std collection's `Extend` has no dictionary, and generic code that extends one is refused",
    file: "src/lower/traits/dictionaries.rs",
    find: "            && let Some(dictionary) = self.extend_dictionary(ty, tr.args.type_at(1), span)?\n",
    replace: "            && let Some(dictionary) = None::<Expr>\n",
    tests: ["test/corpus.test.ts", "-t", "^generic_extend.rs$"],
  },
  {
    name: "from-iter-dictionary-none",
    breaks:
      "a std collection's `FromIterator` has no dictionary, and generic code that collects into one is refused",
    file: "src/lower/traits/dictionaries.rs",
    find: "        if is_std_def(self.tcx, tr.def_id, StdItem::FromIterator) && !self.is_user_impl(tr) {\n",
    replace: "        if false {\n",
    tests: ["test/corpus.test.ts", "-t", "^generic_collect.rs$"],
  },
  {
    name: "mut-writer-unboxed",
    breaks:
      "a `&mut` to a writer taken by value is given its writer's dictionary, which reads the box as the writer, and what it writes is lost",
    file: "src/lower/traits/dictionaries.rs",
    find: '            return Ok(Expr::call_helper("$mutWriter", vec![writer]));\n',
    replace: "            return Ok(writer);\n",
    tests: [
      "test/corpus.test.ts",
      "-t",
      "^(generic_writers|mut_given_as_type_param).rs$",
    ],
  },
  {
    name: "mut-deref-unboxed",
    breaks:
      "std's `Deref` of a `&mut` in a dictionary is the box, not what's in it",
    file: "src/lower/traits/dictionaries.rs",
    find: '                let value = Expr::held(Expr::var("x"));\n',
    replace: '                let value = Expr::var("x");\n',
    tests: ["test/corpus.test.ts", "-t", "^mut_given_as_type_param.rs$"],
  },
  {
    name: "as-mut-dictionary-missing",
    breaks:
      "std's `AsMut` of a `String` or `Vec` has no dictionary, and is refused",
    file: "src/lower/traits/dictionaries.rs",
    find: "        if is_std_def(self.tcx, tr.def_id, StdItem::AsMut) && !self.has_user_impl(tr.def_id, ty) {\n",
    replace: "        if false {\n",
    tests: ["test/corpus.test.ts", "-t", "^as_mut_generic.rs$"],
  },
  {
    name: "as-mut-through-mut-unread",
    breaks:
      "std's `AsMut` of a `&mut String` gives its box, not the cell of its text",
    file: "src/lower/traits/dictionaries.rs",
    find: "                cell = Expr::held(cell);\n                at = inner;\n",
    replace: "                at = inner;\n",
    tests: ["test/corpus.test.ts", "-t", "^as_mut_generic.rs$"],
  },
  {
    name: "deref-mut-dictionary-missing",
    breaks:
      "std's `DerefMut` of a `Box`, a `Vec`, a `String`, a `&mut` or a guard has no dictionary, and generic code over a `P: DerefMut` is refused",
    file: "src/lower/traits/dictionaries.rs",
    find: "        if self.tcx.is_lang_item(tr.def_id, LangItem::DerefMut) && !self.has_user_impl(tr.def_id, ty) {\n",
    replace: "        if false {\n",
    tests: ["test/corpus.test.ts", "-t", "^generic_deref_mut.rs$"],
  },
  {
    name: "deref-mut-of-mut-ref-unread",
    breaks:
      "std's `DerefMut` of a `&mut` gives the box around it, not the one it is",
    file: "src/lower/traits/dictionaries.rs",
    find: '                _ if matches!(ty.kind(), ty::Ref(_, _, Mutability::Mut)) => Some(Expr::held(Expr::var("cell"))),\n',
    replace:
      '                _ if matches!(ty.kind(), ty::Ref(_, _, Mutability::Mut)) => Some(Expr::var("cell")),\n',
    tests: ["test/corpus.test.ts", "-t", "^generic_deref_mut.rs$"],
  },
  {
    name: "deref-mut-of-guard-unread",
    breaks:
      "std's `DerefMut` of a guard gives the box around it, not the guard, which holds its value",
    file: "src/lower/traits/dictionaries.rs",
    find: '                Some(Std::GuardValue { .. }) => Some(Expr::held(Expr::var("cell"))),\n',
    replace:
      '                Some(Std::GuardValue { .. }) => Some(Expr::var("cell")),\n',
    tests: ["test/corpus.test.ts", "-t", "^generic_deref_mut.rs$"],
  },
  {
    name: "deref-mut-of-box-refused",
    breaks:
      "std's `DerefMut` of a `Box`, which MIR never calls, has no dictionary",
    file: "src/lower/traits/dictionaries.rs",
    find: '                _ if ty.is_box() => Some(Expr::var("cell")),\n',
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^generic_deref_mut.rs$"],
  },
  {
    name: "deref-mut-without-deref",
    breaks:
      "std's `DerefMut` dictionary has no `Deref`, which generic code over a `P: DerefMut` reads `p.len()` through",
    file: "src/lower/traits/dictionaries.rs",
    find: "                let mut props = self.supertrait_entries(tr, span)?;\n                props.push(Prop::Field(bindings::fn_name(self.tcx, deref_mut), f));\n",
    replace:
      "                let mut props = Vec::new();\n                props.push(Prop::Field(bindings::fn_name(self.tcx, deref_mut), f));\n",
    tests: ["test/corpus.test.ts", "-t", "^generic_deref_mut.rs$"],
  },
  {
    name: "into-iter-std-dictionary-missing",
    breaks:
      "std's `IntoIterator` of a `&mut Vec`, a `&[T]`, a `&mut Option` or a `&mut BTreeMap` has no dictionary",
    file: "src/lower/traits/dictionaries.rs",
    find: "        if self.ref_into_iter_of_collection(tr) {\n",
    replace: "        if false && self.ref_into_iter_of_collection(tr) {\n",
    tests: ["test/corpus.test.ts", "-t", "^into_iter_of_ref_param.rs$"],
  },
  {
    name: "std-error-dictionary-not-found",
    breaks:
      "`e.source()` of a concrete `io::Error` asks for evidence of its `Error`, which nothing gives",
    file: "src/lower/traits/dictionaries.rs",
    find: "        if is_std_def(self.tcx, tr.def_id, StdItem::Error)\n            && let Some(dictionary) = self.std_error_dictionary(tr.self_ty())\n",
    replace:
      "        if false\n            && let Some(dictionary) = self.std_error_dictionary(tr.self_ty())\n",
    tests: ["test/corpus.test.ts", "-t", "^io_error_values.rs$"],
  },
  {
    name: "text-as-ref-bytes-refused",
    breaks: "text given where `T: AsRef<[u8]>` goes has no dictionary",
    file: "src/lower/traits/dictionaries.rs",
    find: "            && matches!(tr.args.type_at(1).kind(), ty::Slice(item) if *item == self.tcx.types.u8)\n",
    replace: "            && false\n",
    tests: ["test/corpus.test.ts", "-t", "^as_ref_bytes.rs$"],
  },
  {
    name: "std-product-dictionary-sums",
    breaks: "std's `Product` dictionary of a number sums",
    file: "src/lower/traits/dictionaries.rs",
    find: '            let made = self.accumulated_numbers(items, num, name == "sum");\n',
    replace:
      "            let made = self.accumulated_numbers(items, num, true);\n",
    tests: ["test/corpus.test.ts", "-t", "^generic_accumulations.rs$"],
  },
  {
    name: "std-borrow-mut-dictionary-missing",
    breaks: "std's `BorrowMut` of a `Vec` has no `borrow_mut`",
    file: "src/lower/traits/dictionaries.rs",
    find: '                bindings::trait_method_name(self.tcx, tr.def_id, "borrow_mut"),\n',
    replace: '                "borrowed".into(),\n',
    tests: ["test/corpus.test.ts", "-t", "^borrow_mut_supertrait.rs$"],
  },
  {
    name: "std-into-iterator-steps-missing",
    breaks:
      "std's `IntoIterator` dictionary of a type known here carries no `Iterator` of what it makes, so generic code holding that as it is can't step it",
    file: "src/lower/traits/dictionaries.rs",
    find: "            if is_std_def(self.tcx, bound.def_id, StdItem::Iterator) {\n",
    replace: "                if false {\n",
    tests: ["test/corpus.test.ts", "-t", "^fused_iterators.rs$"],
  },
  {
    name: "std-from-str-dictionary-missing",
    breaks: "`read::<i32>(s)` of a `T: FromStr` has no dictionary for `i32`'s",
    file: "src/lower/traits/dictionaries.rs",
    find: "        if crate::lower::recognition::is_from_str(self.tcx, tr.def_id) && parsed_by_std {",
    replace:
      "        if crate::lower::recognition::is_from_str(self.tcx, tr.def_id) && parsed_by_std && false {",
    tests: ["test/corpus.test.ts", "-t", "generic_from_str"],
  },
  {
    name: "into-iterator-of-bound-iterator-refused",
    breaks:
      "an iterator only bounds know, `J::IntoIter`, given where an `IntoIterator` is held as it is, has no evidence",
    file: "src/lower/traits/dictionaries.rs",
    find: "            && (!crate::lower::recognition::known_by_bounds(ty) || self.recognition().iterates_as_itself(ty))\n",
    replace: "            && !crate::lower::recognition::known_by_bounds(ty)\n",
    tests: ["test/corpus.test.ts", "-t", "^into_iter_of_a_bound.rs$"],
  },
  {
    name: "dictionary-const-called",
    breaks:
      "a dictionary that's its module's `const` is called, `circleShape()`",
    file: "src/lower/traits/dictionaries.rs",
    find: "            false => callee,\n",
    replace: "            false => Expr::call(callee, Vec::new()),\n",
    tests: [
      "test/lowering-dictionaries.test.ts",
      "-t",
      "a dictionary that takes nothing",
    ],
  },
  {
    name: "io-writer-mut-as-its-writer",
    breaks:
      "an `io::Write` given by `&mut`, `W = &mut Vec<u8>`, gets the `Vec`'s dictionary, which reads the box it's in as the `Vec`, and throws",
    file: "src/lower/traits/dictionaries.rs",
    find: "        if (is_std_def(self.tcx, tr.def_id, StdItem::FmtWrite) || is_std_def(self.tcx, tr.def_id, StdItem::IoWrite))",
    replace: "        if is_std_def(self.tcx, tr.def_id, StdItem::FmtWrite)",
    tests: ["test/corpus.test.ts", "-t", "^io_write_mut_generic.rs$"],
  },
  {
    name: "other-type-compare-synthesized",
    breaks:
      "a `PartialOrd<u32>` dictionary of a struct compares it with itself, not by its impl",
    file: "src/lower/traits/dictionaries.rs",
    find: "            && !of_other\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^generic_other_type_compare.rs$"],
  },
  {
    name: "reference-dictionary-unforwarded",
    breaks:
      "a reference's dictionary through std's forwarding impl is its referent's",
    file: "src/lower/traits/dictionaries.rs",
    find: "        if let Some(referent) = self.forwarded_to_referent(tr, typing_env) {",
    replace: "        if let Some(referent) = None::<ty::TraitRef<'tcx>> {",
    tests: ["test/corpus.test.ts", "-t", "^references_given_to_generics.rs$"],
  },
  {
    name: "component-as-ref-dictionary-refused",
    breaks: "a `Component` given to a `P: AsRef<Path>` has no dictionary",
    file: "src/lower/traits/dictionaries.rs",
    find: "            && self.recognition().as_ref_is_component_text(ty, tr.args.type_at(1))\n",
    replace: "            && false\n",
    tests: ["test/corpus.test.ts", "-t", "^path_components.rs$"],
  },
  {
    name: "copy-of-itself-given",
    breaks:
      "a value that's its own copy is given `{ copy: (value) => value }`, `createContext(7, ..)`, where JS gives none",
    file: "src/lower/traits/dictionaries.rs",
    find: "            if js::same_path(&copy, &value) && clone.is_none() {\n",
    replace: "            if false {\n",
    tests: [
      "test/crates-compiled-here.test.ts",
      "-t",
      "Copy bound takes a copy only where one is given",
    ],
  },
  {
    name: "ref-into-iter-steps-missing",
    breaks:
      "a reference's `IntoIterator` dictionary carries no `Iterator` of what it makes, so a held flatten of `&[T; N]` items can't step them",
    file: "src/lower/traits/dictionaries.rs",
    find: "        let mut props = vec![Prop::Field(bindings::fn_name(self.tcx, into_iter), f)];\n        props.extend(self.made_steps(tr, false, span)?);\n        Ok(Expr::object(props))\n",
    replace: "        let props = vec![Prop::Field(bindings::fn_name(self.tcx, into_iter), f)];\n        Ok(Expr::object(props))\n",
    tests: ["test/corpus.test.ts", "-t", "^std_flatten.rs$"],
  },
  {
    name: "mut-ref-dictionary-unpeeled",
    breaks:
      "a `&mut` of a range or of the crate's iterator is given a JS iterator's dictionary, so std's `Map` or `Fuse` holding it steps it by `it.next`, which neither has (ADR 0912)",
    file: "src/lower/traits/dictionaries.rs",
    find: "            if let ty::Ref(_, inner, ty::Mutability::Mut) = tr.self_ty().kind() {\n                let of",
    replace:
      "            if false && let ty::Ref(_, inner, ty::Mutability::Mut) = tr.self_ty().kind() {\n                let of",
    tests: ["test/corpus.test.ts", "-t", "^held_adapters_of_by_ref.rs$"],
  },
  {
    name: "control-flow-try-never-short-circuits",
    breaks:
      "a `ControlFlow`'s `Try` is given as a `NeverShortCircuit`'s, so std's `Take::try_fold` never stops at its count (ADR 0913)",
    file: "src/lower/traits/dictionaries.rs",
    find: '                Some(TryKind::ControlFlow) => Ok(Expr::helper("$controlFlowTry")),\n',
    replace:
      '                Some(TryKind::ControlFlow) => Ok(Expr::helper("$neverShortCircuitTry")),\n',
    tests: [
      "test/std-package.test.ts",
      "-t",
      "the committed @rust-js/std is std's bodies",
    ],
  },
  {
    name: "made-steps-of-a-js-iterator",
    breaks:
      "what an `I: Iterator` makes, itself, is stepped as a JS iterator in its `IntoIterator` dictionary, not by `I`'s own, so std's `chain` of two ranges given as they are throws (ADR 0920)",
    file: "src/lower/traits/dictionaries.rs",
    find: "                let dictionary = match self.iterator_evidence(of, bound.def_id) {\n",
    replace: "                let dictionary = match None::<Expr> {\n",
    tests: ["test/corpus.test.ts", "-t", "^lazy_iterator_clone.rs$"],
  },
  {
    name: "unexported-impl-evidence-refused",
    breaks:
      "a library's impl its JS doesn't make has no evidence where its consumer gives it to generic code, and is refused (ADR 0914, amended)",
    file: "src/lower/traits/dictionaries.rs",
    find: "            .filter(|&(id, _)| self.krate.trait_impls.contains(&id) || self.krate.foreign.compiled(id))\n",
    replace: "            .filter(|&(id, _)| self.krate.trait_impls.contains(&id))\n",
    tests: ["test/crates-compiled-here.test.ts", "-t", "program's, alone or a dependency"],
  },
  {
    name: "class-display-dictionary-impl",
    breaks:
      "a `T: Display` of a class is its impl's dictionary, which isn't emitted (#358)",
    file: "src/lower/traits/dictionaries.rs",
    find: "            && (!self.has_user_impl(tr.def_id, ty) || shown)\n",
    replace: "            && !self.has_user_impl(tr.def_id, ty)\n",
    tests: ["test/classes.test.ts", "-t", "Display is its prototype"],
  },
  {
    name: "js-object-deref-dictionary-unmade",
    breaks:
      "a generic function bounded by `Deref<Target = Base>`, given a JS class whose `Deref` is to the class it extends, panics: its dictionary looks for the impl's function, which is never lowered (#403)",
    file: "src/lower/traits/dictionaries.rs",
    find: "            && crate::lower::recognition::is_js_object_deref(self.tcx, imp)\n",
    replace: "            && false\n",
    tests: [
      "test/js-class-deref.test.ts",
      "-t",
      "a JS class's Deref to the class it extends is no JS",
    ],
  },
  {
    name: "ref-iterator-into-iter-by-collection",
    breaks: "a `&mut` of the crate's iterator given where std asks `IntoIterator` is taken for a collection's, and has no evidence, so std's compiled `flatten()` of `&mut`s is refused (#742)",
    file: "src/lower/traits/dictionaries.rs",
    find: "            && !self.recognition().iterates_as_itself(ty)\n            && !self.has_user_impl(tr.def_id, ty)\n",
    replace: "            && !self.has_user_impl(tr.def_id, ty)\n",
    tests: ["test/iterator-forms.test.ts", "-t", "^mut_ref_iterators_into_iter with std"],
  },
  {
    name: "ref-iterator-not-itself",
    breaks: "a `&mut` of the crate's iterator isn't taken as it is where std's impl for every iterator takes it, so it has no evidence (#742)",
    file: "src/lower/traits/dictionaries.rs",
    find: "            && (!matches!(ty.kind(), ty::Ref(..)) || self.recognition().iterates_as_itself(ty))\n",
    replace: "            && !matches!(ty.kind(), ty::Ref(..))\n",
    tests: ["test/iterator-forms.test.ts", "-t", "^mut_ref_iterators_into_iter with std"],
  },
  {
    name: "user-from-iterator-as-collection",
    breaks:
      "the crate's own `impl FromIterator<u32> for Bag` is taken for a std collection's, its dictionary an entry of `Array.from(items)` that gives the impl no fact of what it's given, where it's the impl's own (#794)",
    file: "src/lower/traits/dictionaries.rs",
    find: "        if is_std_def(self.tcx, tr.def_id, StdItem::FromIterator) && !self.is_user_impl(tr) {\n",
    replace: "        if is_std_def(self.tcx, tr.def_id, StdItem::FromIterator) && !self.has_user_impl(tr.def_id, ty) {\n",
    tests: ["test/corpus.test.ts", "-t", "^generic_collect.rs$"],
    snapshots: true,
  },
  {
    name: "std-clone-dictionary-without-clone-from",
    breaks: "a `Vec<D>`'s `Clone` dictionary has no `cloneFrom`, so a generic `clone_from` of it clones every item before the old ones drop, not std's in place (#928)",
    file: "src/lower/traits/dictionaries.rs",
    find: "                    props.push(Prop::Field(name, clone_from));\n",
    replace: "                    let _ = (name, clone_from);\n",
    tests: ["test/corpus.test.ts", "-t", "^clone_from_generic_std_own.rs$"],
  },
  {
    name: "std-eq-dictionary-without-own-ne",
    breaks: "a tuple's `PartialEq` dictionary has std's `!eq` for `ne`, where std's asks each part's `!=`, an impl's own (#962)",
    file: "src/lower/traits/dictionaries.rs",
    find: "                if self.own_ne(ty) {\n",
    replace: "                if self.own_ne(ty) && false {\n",
    tests: ["test/corpus.test.ts", "-t", "^own_ne_max_min_clamp.rs$"],
  },
  {
    name: "quiet-eq-dictionary-made",
    breaks: "a quiet type's `PartialEq` dictionary is made where it's given, not std's shared `$strictlyEqual` (#962)",
    file: "src/lower/traits/dictionaries.rs",
    find: "                if self.eq_is_strict(ty) {\n",
    replace: "                if self.eq_is_strict(ty) && false {\n",
    tests: ["test/lowering-dictionaries.test.ts", "-t", "hold ne, max, min and clamp"],
  },
  {
    name: "super-item-dictionary-uncarried",
    breaks: "a consumer's dictionary of a library's trait carries none of `IteratorItemSize`, so the library's default reads `undefined` (#990)",
    file: "src/lower/traits/dictionaries.rs",
    find: "            false => self.carried_super(tr.def_id),\n",
    replace: "            false => Vec::new(),\n",
    tests: ["test/crates.test.ts", "-t", "two crates: library_default_item_layouts"],
  },
  {
    name: "library-impl-dispatch-unasked",
    breaks:
      "a consumer's dictionary of a library's generic impl, made of its own type, that gives a bound trait's method a `Vec<&mut T>` isn't refused as the library's own is (#685, #686, the `is_local()` audit)",
    file: "src/lower/traits/dictionaries.rs",
    find: "        if let Some(here) = self.impl_nested_mut_object(impl_id, impl_args) {\n",
    replace: "        if impl_id.is_local() && let Some(here) = self.impl_nested_mut_object(impl_id, impl_args) {\n",
    tests: ["test/crates.test.ts", "-t", "dictionary of a library's generic impl giving"],
  },
  {
    name: "generic-text-extend-joined-nothing",
    breaks:
      "a `String` extended in generic code, through `Extend`'s dictionary, is given nothing, where each piece is appended as it's stepped (#1127)",
    file: "src/lower/traits/dictionaries.rs",
    find: "                let stepped = crate::lower::std_types::text::extend_text(items.clone(), &piece, push);\n",
    replace: "                let stepped = crate::lower::std_types::text::extend_text(Expr::array(Vec::new()), &piece, push);\n",
    tests: ["test/corpus.test.ts", "-t", "^(iterator_consumed_by_extend_unzip_partition_eq|generic_extend).rs$"],
  },
  {
    name: "from-iter-dictionary-unclosed",
    breaks: "generic code's `collect` into a std collection closes nothing of what it's given where collecting it panics, where std drops the iterator its collect holds as it unwinds (#1141, #1239)",
    file: "src/lower/traits/dictionaries.rs",
    find: "        let made = Expr::call_helper(\n            \"$collecting\",\n            vec![\n                Expr::var(\"items\"),\n                Expr::arrow(Vec::new(), vec![StmtKind::Return(Some(made)).at(js::Span::NONE)]),\n            ],\n        );\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^consumer_closes_what_it_steps_on_a_panic.rs$"],
  },
  {
    name: "as-ref-bytes-of-text-its-text",
    breaks: "std's `AsRef<[u8]>` of text gives the text, not its UTF-8 bytes",
    file: "src/lower/traits/dictionaries.rs",
    find: "            let bytes = Expr::call(Expr::helper(\"$utf8Bytes\"), vec![Expr::var(\"value\")]);\n",
    replace: "            let bytes = Expr::var(\"value\");\n",
    tests: ["test/corpus.test.ts", "-t", "^as_ref_bytes.rs$"],
  },
  {
    name: "dictionary-grown-dropped",
    breaks: "generic code's `extend` through its dictionary, in a crate that keeps capacities, isn't grown as std's (#1180)",
    file: "src/lower/traits/dictionaries.rs",
    find: "        let items = self.dictionary_grown(collection.clone(), items, (ty, item), span, &mut body)?;\n",
    replace: "        let _ = self.dictionary_grown(collection.clone(), items.clone(), (ty, item), span, &mut body)?;\n",
    tests: ["test/corpus.test.ts", "-t", "^vec_capacity_grown.rs$"],
  },
  {
    name: "from-iter-entry-folds-to-an-array",
    breaks: "a std collection's `from_iter` given generic code is its own fold into an array, `$foldToArray(items)`, not the collect generic code's own `Vec` collect is, so `collect::<B>()` of `B = Vec<u32>` asks no hint where std's asks it as it grows (#1239)",
    file: "src/lower/traits/dictionaries.rs",
    find: "            cx.collected_into((Expr::var(\"items\"), None), output, Some(item), Some(param), span)\n",
    replace: "            cx.collected_into((Expr::call_helper(\"$foldToArray\", vec![Expr::var(\"items\")]), None), output, Some(item), None, span)\n",
    tests: ["test/corpus.test.ts", "-t", "^collect_into_a_type_parameter_is_stds\\.rs$"],
  },
  {
    name: "ref-into-iter-bound-taken-for-a-collections",
    breaks:
      "in the build of std's iterator structs, std's `zip` given a `&I` whose `IntoIterator` is a bound of its caller's asks a std collection's reference's dictionary of it, refused, where its caller's bound gives it (#1264)",
    file: "src/lower/traits/dictionaries.rs",
    find: "            && self.evidence_for(tr).is_none()\n",
    replace: "",
    tests: ["test/iterator-forms.test.ts", "-t", "^into_iterator_of_a_reference_given_to_generic_code with"],
  },
  {
    name: "mut-reader-as-its-reader",
    breaks: "a `&mut` to a reader taken by value, `R = &mut c`, gets the reader's own dictionary, which reads the box it's in as the reader",
    file: "src/lower/traits/dictionaries.rs",
    find: "            return Ok(Expr::call_helper(\"$mutReader\", vec![reader]));\n",
    replace: "            return Ok(reader);\n",
    tests: ["test/corpus.test.ts", "-t", "^io_buf_read.rs$"],
  },

  {
    name: "impl-trait-item-uncarried",
    breaks: "an impl's dictionary carries no drop of what a trait method gives behind an `impl Trait`, so generic code holding one, where a type may have a destructor, is refused (#1296)",
    file: "src/lower/traits/dictionaries.rs",
    find: "                    && (!item.is_impl_trait_in_trait() || super::words::one_impl_trait_of(self.tcx, item.def_id))\n",
    replace: "                    && !item.is_impl_trait_in_trait()\n",
    tests: ["test/corpus.test.ts", "-t", "^impl_trait_of_a_trait_method_dropped_by_its_dictionary.rs$"],
  },
  {
    name: "std-reader-read-no-defaults",
    breaks: "a `Chain`'s or a `Take`'s `Read` dictionary is its impl's alone, without std's defaults, so its `read_exact` isn't there (#1156)",
    file: "src/lower/traits/dictionaries.rs",
    find: "                Ok(Expr::call_helper(\"$readerDefaults\", vec![made, read_buf]))\n",
    replace: "                Ok(made)\n",
    tests: ["test/corpus.test.ts", "-t", "^io_read_chain_take_bytes.rs$"],
  },
  {
    name: "buf-reader-dictionary-unmade",
    breaks: "a `BufReader`'s dictionary is its inner's, given as it is, not std's bodies over its buffer (#1156)",
    file: "src/lower/traits/dictionaries.rs",
    find: "Ok(Expr::call_helper(\"$bufReader\", vec![reader]))",
    replace: "Ok(reader)",
    tests: ["test/corpus.test.ts", "-t", "^io_buf_reader.rs$"],
  },
  {
    name: "buf-reader-seek-asks-read",
    breaks: "a `BufReader`'s `Seek` asks its inner's `Read`, not its `Seek`, which a type parameter bound by `Seek` alone has none of (#1156)",
    file: "src/lower/traits/dictionaries.rs",
    find: "                    true => StdItem::IoSeek,\n",
    replace: "                    true => StdItem::IoRead,\n",
    tests: ["test/corpus.test.ts", "-t", "^io_buf_reader.rs$"],
  },
  {
    name: "slice-writer-through-mut-writer",
    breaks: "`write!` of a `&mut [u8]` takes it for a `&mut W` of a writer `W`, `$mutWriter($sliceWriter)`, one `value` too deep, a `TypeError` (#1282)",
    file: "src/lower/traits/dictionaries.rs",
    find: "            && !(is_std_def(self.tcx, tr.def_id, StdItem::IoWrite) && crate::lower::recognition::is_byte_slice(inner))\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^mut_ref_of_a_mut_slice.rs$"],
  },
  {
    name: "borrow-of-a-mut-unboxed",
    breaks: "std's `Borrow` of a `&mut` given for a type parameter gives generic code's box, not what it's to: `peek(&mut parser)` reads `undefined` (#1414)",
    file: "src/lower/traits/dictionaries.rs",
    find: "in_box(Expr::var(\"value\")))).at(js::Span::NONE)],\n            );\n            return Ok(Expr::object(vec![Prop::Field(\"borrow\".into(), same)]));",
    replace: "Expr::var(\"value\"))).at(js::Span::NONE)],\n            );\n            return Ok(Expr::object(vec![Prop::Field(\"borrow\".into(), same)]));",
    tests: ["test/corpus.test.ts", "-t", "^mut_ref_given_for_a_bound.rs$"],
  },
  {
    name: "borrow-mut-of-a-mut-unboxed",
    breaks: "std's `BorrowMut` of a `&mut` given for a type parameter gives back the box it was given, one `value` short: `bump(&mut parser)` writes the box, the parser left as it was (#1414)",
    file: "src/lower/traits/dictionaries.rs",
    find: "in_box(Expr::var(\"value\")))).at(js::Span::NONE)],\n            );\n            let mut props",
    replace: "Expr::var(\"value\"))).at(js::Span::NONE)],\n            );\n            let mut props",
    tests: ["test/corpus.test.ts", "-t", "^mut_ref_given_for_a_bound.rs$"],
  },
  {
    name: "borrow-mut-of-a-mut-number-refused",
    breaks: "std's `BorrowMut<u32>` of a `&mut u32` given for a type parameter is refused (#1414)",
    file: "src/lower/traits/dictionaries.rs",
    find: "            && (self.is_object(tr.args.type_at(1)) || ty.is_ref())\n",
    replace: "            && self.is_object(tr.args.type_at(1))\n",
    tests: ["test/corpus.test.ts", "-t", "^mut_ref_given_for_a_bound.rs$"],
  },
  {
    name: "forwarded-mut-not-through-box",
    breaks: "std's impl that passes a trait on from a `&mut T` to `T` is given `T`'s dictionary as it is, which reads generic code's box as the object: `show(&mut parser)` shows `undefined` (#1414)",
    file: "src/lower/traits/dictionaries.rs",
    find: "        if self.in_generic_box(tr.self_ty())\n",
    replace: "        if false && self.in_generic_box(tr.self_ty())\n",
    tests: ["test/corpus.test.ts", "-t", "^mut_ref_given_for_a_bound.rs$"],
  },
  {
    name: "forwarded-mut-iterator-through-box",
    breaks: "an iterator trait of a `&mut I`, which generic code is given as it is, is read through a box, `self.by_ref().take(1)` refused (#1414)",
    file: "src/lower/traits/dictionaries.rs",
    find: "            && !rustc_middle::ty::elaborate::supertraits(self.tcx, ty::Binder::dummy(tr))\n                .any(|tr| crate::lower::recognition::is_iterator_trait(self.tcx, tr.def_id()))\n",
    replace: "",
    tests: ["test/iterator-forms.test.ts", "-t", "^match_moves_a_part_or_the_whole with"],
  },
  {
    name: "cursor-of-a-mut-through-box",
    breaks: "a `Cursor<&mut Vec<u8>>` made where its type is known, which holds the vector itself, is given the `&mut`'s `AsRef`, which reads a box: its read sees no bytes (#1414)",
    file: "src/lower/traits/dictionaries.rs",
    find: "                        if self.in_generic_box(inner) && !self.mut_ref_form(object).is_cell() =>\n",
    replace: "                        if false =>\n",
    tests: ["test/corpus.test.ts", "-t", "^(mut_ref_given_for_a_bound|io_cursor).rs$"],
  },
  {
    name: "std-index-dictionary-missing",
    breaks:
      "std's `Index<usize>` of a slice, an array, a `Vec` or a `VecDeque` given to an `S: Index<usize>` has no dictionary, \"implementation evidence\", refused (#1420)",
    file: "src/lower/traits/dictionaries.rs",
    find: "            && (ty.is_slice() || ty.is_array() || self.is_vec_like(ty))\n",
    replace: "            && false\n",
    tests: ["test/corpus.test.ts", "-t", "^index_through_a_generic_bound.rs$"],
  },
  {
    name: "std-index-dictionary-unchecked",
    breaks:
      "std's `Index<usize>` dictionary reads `items[i]` unchecked, so an index past the end gives `undefined` where Rust panics, \"index out of bounds: the len is 3 but the index is 5\" (#1420)",
    file: "src/lower/traits/dictionaries.rs",
    find: "            let item = self.index_checked(Expr::var(\"items\"), Expr::var(\"i\"), ty);\n",
    replace: "            let item = Expr::index(Expr::var(\"items\"), Expr::var(\"i\"));\n",
    tests: ["test/corpus.test.ts", "-t", "^index_through_a_generic_bound.rs$"],
  },
  {
    name: "wrapping-dictionary-rhs-unread",
    breaks: "generic code's operator given a `Wrapping` takes its right side's `Wrapping` as a number, `[x]` where `x` goes (#1439)",
    file: "src/lower/traits/dictionaries.rs",
    find: "                Ok(_) => (vec![\"a\".into(), \"b\".into()], Some((Expr::var(\"b\"), tr.args.type_at(1)))),\n",
    replace: "                Ok(_) => (vec![\"a\".into(), \"b\".into()], Some((Expr::var(\"b\"), self.tcx.types.u8))),\n",
    tests: ["test/corpus.test.ts", "-t", "^wrapping_ops_by_its_number_and_past_its_range.rs$"],
  },
  {
    name: "newtype-sum-refused",
    breaks: "the `sum()` of `Wrapping`s or `Saturating`s has no dictionary, refused (#1445)",
    file: "src/lower/traits/dictionaries.rs",
    find: "            && let Some(newtype) = self.recognition().int_newtype(ty)\n            && let Some(num) = Num::of(newtype.0)\n",
    replace: "            && let Some(newtype) = self.recognition().int_newtype(ty).filter(|_| false)\n            && let Some(num) = Num::of(newtype.0)\n",
    tests: ["test/corpus.test.ts", "-t", "^newtype_sums_and_products.rs$"],
  },
  {
    name: "newtype-product-adds",
    breaks: "a `Wrapping`'s `product()` adds, from its one (#1445)",
    file: "src/lower/traits/dictionaries.rs",
    find: "                false => (rustc_middle::mir::BinOp::Mul, 1),\n",
    replace: "                false => (rustc_middle::mir::BinOp::Add, 1),\n",
    tests: ["test/corpus.test.ts", "-t", "^newtype_sums_and_products.rs$"],
  },
  {
    name: "newtype-sum-from-one",
    breaks: "a `Wrapping`'s `sum()` starts from one, an empty one's `1` (#1445)",
    file: "src/lower/traits/dictionaries.rs",
    find: "                true => (rustc_middle::mir::BinOp::Add, 0),\n",
    replace: "                true => (rustc_middle::mir::BinOp::Add, 1),\n",
    tests: ["test/corpus.test.ts", "-t", "^newtype_sums_and_products.rs$"],
  },
  {
    name: "copy-dictionary-holds-no-clone",
    breaks: "a `Copy` dictionary of a type with a hand-written `Clone` holds no `Clone()`, so a library's generic code bounded by `T: Copy` alone copies where std's calls that `clone` (#1451)",
    file: "src/lower/traits/dictionaries.rs",
    find: "                true => Some(self.dictionary(ty::TraitRef::new(self.tcx, self.clone_trait(), [ty]), span)?),\n",
    replace: "                true => None,\n",
    tests: ["test/crates.test.ts", "-t", "library_clones_through_a_copy_bound"],
  },
  {
    name: "copy-dictionary-dropped-with-its-clone",
    breaks: "a `Copy` dictionary whose copy is the value is given as `undefined` though it holds a `Clone()` its generic code calls (#1451)",
    file: "src/lower/traits/dictionaries.rs",
    find: "            if js::same_path(&copy, &value) && clone.is_none() {\n",
    replace: "            if js::same_path(&copy, &value) {\n",
    tests: ["test/crates.test.ts", "-t", "library_clones_through_a_copy_bound"],
  },
  {
    name: "as-ref-of-a-counted-rc-is-the-rc",
    breaks: "generic code's `T: AsRef<str>` of a counted `Rc<str>` is given the `Rc`, `{ value, strong, weak }`, not what it points at, and `t.as_ref().to_uppercase()` throws (#1503)",
    file: "src/lower/traits/dictionaries.rs",
    find: "            let inside = self.through_boxes_to(Expr::var(\"value\"), ty.peel_refs(), tr.args.type_at(1));\n",
    replace: "            let inside = Expr::var(\"value\");\n",
    tests: [
      "test/corpus.test.ts",
      "-t",
      "^generic_borrow_of_a_counted_rc.rs$"
    ]
  },
  {
    name: "borrow-of-a-counted-rc-is-the-rc",
    breaks: "generic code's `K: Borrow<str>` of a counted `Rc<str>` is given the `Rc`, not what it points at, and `k.borrow().len()` is 0 (#1503)",
    file: "src/lower/traits/dictionaries.rs",
    find: "        let in_box = |value: Expr| {\n            self.through_boxes_to(self.held_in_generic_box(ty)(value), ty.peel_refs(), tr.args.type_at(1))\n        };\n",
    replace: "        let in_box = self.held_in_generic_box(ty);\n",
    tests: [
      "test/corpus.test.ts",
      "-t",
      "^generic_borrow_of_a_counted_rc.rs$"
    ]
  },
  {
    name: "into-iterator-dictionary-drops-nothing",
    breaks: "std's `IntoIterator` dictionary of a type known here carries no drops of its associated types, so std's compiled `Flatten` never drops an inner iterator it replaces (#1510)",
    file: "src/lower/traits/dictionaries.rs",
    find: "        steps.extend(self.item_drops(tr, as_it_is, span)?);\n",
    replace: "",
    tests: ["test/corpus.test.ts", "test/iterator-forms.test.ts", "-t", "^flatten_nth_drops_each_inner_it_runs_out.rs$|^flatten_nth_drops_each_inner_it_runs_out with"],
  },
  {
    name: "into-iterator-as-it-is-dropped-as-made",
    breaks: "std's `IntoIterator` dictionary that gives what it makes as it is drops it as a JS iterator made of it, its `of`, which the value itself hasn't (#1510)",
    file: "src/lower/traits/dictionaries.rs",
    find: "            let steps = self.made_steps(tr, true, span)?;\n",
    replace: "            let steps = self.made_steps(tr, false, span)?;\n",
    tests: ["test/corpus.test.ts", "test/iterator-forms.test.ts", "-t", "^flatten_nth_drops_each_inner_it_runs_out.rs$|^flatten_nth_drops_each_inner_it_runs_out with"],
  },
  {
    name: "own-io-impl-no-dictionary",
    breaks: "a crate's own `BufRead` or `Seek` given to generic code has no dictionary, and is refused (#1489)",
    file: "src/lower/traits/dictionaries.rs",
    find: "            None => (self.impl_dictionary(tr, span)?).ok_or_else(|| self.no_evidence(span, tr)),\n",
    replace: "            None => Err(self.no_evidence(span, tr)),\n",
    tests: [
      "test/corpus.test.ts",
      "-t",
      "^readers_of_the_crates_own_and_boxed.rs$"
    ]
  },
  {
    name: "extend-dictionary-iterator-undropped",
    breaks: "a std collection's `Extend` dictionary never drops the iterator it was given at its end, `drop Up 3` lost (#1511)",
    file: "src/lower/traits/dictionaries.rs",
    find: "                body.push(StmtKind::Expr(dropped()).at(js::Span::NONE));\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^extend_through_a_dictionary_drops_its_iterator\\.rs$"],
  },
  {
    name: "extend-dictionary-not-by-fold",
    breaks: "a list's `Extend` dictionary steps what it's given by `next`, never reaching the crate's own `fold` std's `for_each` calls (#1511)",
    file: "src/lower/traits/dictionaries.rs",
    find: "        let by_fold = self.extends_by_fold(ty);\n",
    replace: "        let by_fold = false;\n",
    tests: ["test/corpus.test.ts", "-t", "^extend_through_a_dictionary_drops_its_iterator\\.rs$"],
  },
  {
    name: "extend-dictionary-own-fold-dropped-again",
    breaks: "a list's `Extend` dictionary drops the iterator its own `fold` took and dropped, twice (#1511)",
    file: "src/lower/traits/dictionaries.rs",
    find: "                        Expr::member(it.clone(), \"ownFold\"),\n                        Expr::undefined(),\n",
    replace: "                        Expr::member(it.clone(), \"ownFold\"),\n                        Expr::var(\"dropIter\"),\n",
    tests: ["test/corpus.test.ts", "-t", "^extend_through_a_dictionary_drops_its_iterator\\.rs$"],
  },
  {
    name: "std-debug-dictionary-drops-nothing",
    breaks: "a `Box<dyn Debug>` of a `Vec<Point>`, std's `Debug` dictionary, drops none of its points (#1578)",
    file: "src/lower/traits/dictionaries.rs",
    find: "                let mut props = vec![Prop::Field(\"fmt\".into(), fmt)];\n                // Its type's drop, as an impl's dictionary has it: a `Box<dyn\n                // Debug>` of a `Vec<Point>` drops its points (#1578).\n                props.extend(self.dictionary_drop(ty, span)?);\n",
    replace: "                let mut props = vec![Prop::Field(\"fmt\".into(), fmt)];\n                // Its type's drop, as an impl's dictionary has it: a `Box<dyn\n                // Debug>` of a `Vec<Point>` drops its points (#1578).\n",
    tests: [
      "test/corpus.test.ts",
      "-t",
      "^std_dictionaries_drop_their_values.rs$"
    ]
  },
  {
    name: "std-display-dictionary-drops-nothing",
    breaks: "a `Box<dyn Display>` of an `Rc<Point>` or a `Box<Point>`, std's `Display` dictionary, never drops its point (#1578)",
    file: "src/lower/traits/dictionaries.rs",
    find: "                let mut props = vec![Prop::Field(\"fmt\".into(), fmt)];\n                props.extend(self.dictionary_drop(ty, span)?);\n                return Ok(Expr::object(props));\n            }\n            // `ToString` is std's",
    replace: "                let props = vec![Prop::Field(\"fmt\".into(), fmt)];\n                return Ok(Expr::object(props));\n            }\n            // `ToString` is std's",
    tests: [
      "test/corpus.test.ts",
      "-t",
      "^std_dictionaries_drop_their_values.rs$"
    ]
  },
  {
    name: "to-string-dictionary-drops-nothing",
    breaks: "a `Box<dyn ToString>` of a `Point`, the blanket `ToString` dictionary, never drops it (#1578)",
    file: "src/lower/traits/dictionaries.rs",
    find: "                )];\n                props.extend(self.dictionary_drop(ty, span)?);\n                return Ok(Expr::object(props));\n            }\n            // `$partialEq`'s",
    replace: "                )];\n                let _ = &mut props;\n                return Ok(Expr::object(props));\n            }\n            // `$partialEq`'s",
    tests: [
      "test/corpus.test.ts",
      "-t",
      "^std_dictionaries_drop_their_values.rs$"
    ]
  },
];
