// Mutations of src/lower/traits/impls.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "made-item-iterator-own-dictionary",
    breaks:
      "the `DoubleEndedIterator` entry of an associated type the dictionary made a JS iterator of is the crate's own `next_back`, given that JS iterator: `undefined from the back` (#972)",
    file: "src/lower/traits/impls.rs",
    find: "                        .then(|| self.made_iterator_dictionary(bound.def_id, ty))\n",
    replace: "                        .then(|| None)\n",
    tests: ["test/corpus.test.ts", "-t", "^associated_iterator_given_as_it_is.rs$"],
  },
  {
    name: "made-item-iterator-param-as-it-is",
    breaks:
      "an impl's own type parameter given as it is, its associated type's, goes to generic code as it is, which steps it as a JS iterator: `it.next is not a function` (#972)",
    file: "src/lower/traits/impls.rs",
    find: "                || self.iterator_given_as_it_is(ty)\n                || crate::lower::recognition::is_compiled_std_iterator(self.tcx, ty))\n",
    replace: "                || crate::lower::recognition::is_compiled_std_iterator(self.tcx, ty))\n",
    tests: ["test/corpus.test.ts", "-t", "^associated_iterator_given_as_it_is.rs$"],
  },
  {
    name: "made-item-iterator-backless",
    breaks:
      "what the dictionary makes of an associated type the trait bounds by `DoubleEndedIterator` knows no back: `it.back is not a function` (#972)",
    file: "src/lower/traits/impls.rs",
    find: "                (true, (true, false)) => self.as_double_ended(call, output, span).map(Some),\n",
    replace: "                (true, (true, false)) => self.as_js_iterator(call, output, span).map(Some),\n",
    tests: ["test/corpus.test.ts", "-t", "^associated_iterator_dropped.rs$"],
  },
  {
    name: "own-operator-left-out-of-dictionary",
    breaks: "a `PartialOrd`'s own `lt` isn't in its dictionary, so generic `<` and std's helpers run std's (#944)",
    file: "src/lower/traits/impls.rs",
    find: "                && !own_operator\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^generic_compare_by_impls.rs$"],
  },
  {
    name: "entry-nested-mut-unadapted",
    breaks:
      "a dictionary's entry gives its impl what generic code gives within a parameter as it is, `Option<&mut Self>`'s box, where an object's impl takes the object (#686)",
    file: "src/lower/traits/impls.rs",
    find: "                    *value = made;\n",
    replace: "                    let _ = made;\n",
    tests: ["test/corpus.test.ts", "-t", "^nested_mut_self_through_a_dictionary.rs$"],
  },
  {
    name: "dyn-dictionary-without-drop",
    breaks:
      "a dictionary doesn't carry its type's drop, so a `dyn` of a value with a destructor never drops it",
    file: "src/lower/traits/impls.rs",
    find: "        props.extend(self.dictionary_drop(tr.self_ty(), span)?);\n        props.extend(self.item_drops(tr, false, span)?);\n",
    replace: "        props.extend(self.item_drops(tr, false, span)?);\n",
    tests: ["test/corpus.test.ts", "-t", "dyn_drops"],
  },
  {
    name: "impl-dictionary-cache-undropped",
    breaks:
      "a generic impl's dictionary is cached by its dictionaries only, not the drops it's given",
    file: "src/lower/traits/impls.rs",
    find: "                    .chain(self.given_drops().iter().map(|name| Expr::var(name)))\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "drop_impl_dictionary"],
  },
  {
    name: "impl-dictionary-drops-unforwarded",
    breaks:
      "a method through a generic impl's dictionary is given its dictionaries, but not its drops",
    file: "src/lower/traits/impls.rs",
    find: "                true => self.evidence_args_with(method, instance.args, location, span),\n",
    replace:
      "                true => (|| -> R<Vec<Expr>> {\n                    let mut evidence: Vec<Expr> = self.takes_caller_location(method).then_some(location).into_iter().collect();\n                    for param in const_params(self.tcx, method) {\n                        evidence.push(self.const_arg(instance.args.const_at(param.index as usize), span)?);\n                    }\n                    for bound in crate::lower::traits::bounds(self.tcx, self.krate.foreign, self.krate.given, method) {\n                        let bound = ty::EarlyBinder::bind(self.tcx, bound)\n                            .instantiate(self.tcx, instance.args)\n                            .skip_normalization();\n                        evidence.push(self.dictionary(bound, span)?);\n                    }\n                    Ok(evidence)\n                })(),\n",
    tests: ["test/corpus.test.ts", "-t", "drop_impl_dictionary"],
  },
  {
    name: "dictionary-mut-self-boxed",
    breaks:
      "an object impl's `&mut self` method in its dictionary is given the box generic code calls it with",
    file: "src/lower/traits/impls.rs",
    find: "                let given = usize::from(self.param_is_box(item, i)) + usize::from(i == 0 && self_held);\n",
    replace: "                let given = usize::from(i == 0 && self_held);\n",
    tests: ["test/corpus.test.ts", "-t", "generic_mut_ref"],
  },
  {
    name: "item-bounds-unwritten",
    breaks:
      "an impl's dictionary has no `LabelDisplay`, and generic code reads it",
    file: "src/lower/traits/impls.rs",
    find: "        let mut shared = Vec::new();\n        let mut props = self.supertrait_entries(tr, span)?;\n        props.extend(self.item_bound_entries(tr, span)?);\n",
    replace:
      "        let mut shared = Vec::new();\n        let mut props = self.supertrait_entries(tr, span)?;\n",
    tests: ["test/corpus.test.ts", "-t", "associated_types"],
  },
  {
    name: "trait-const-shared",
    breaks:
      "`T::ZERO` of a type changed in place is one value for every use, so a `bump` of one changes the next",
    file: "src/lower/traits/impls.rs",
    find: "                props.push(match self.contains_mutated(ty) {\n",
    replace: "                props.push(match false {\n",
    tests: ["test/corpus.test.ts", "-t", "associated_consts"],
  },
  {
    name: "trait-consts-all-evaluated",
    breaks:
      "every trait constant is in each dictionary, so `Wrap`'s `PER_SIDE`, which nothing reads, divides by zero as rustc evaluates it",
    file: "src/lower/traits/impls.rs",
    find: "                if !self.krate.library && !self.krate.generic_consts.contains(&item.def_id) {\n",
    replace: "                if false {\n",
    tests: ["test/corpus.test.ts", "-t", "associated_consts"],
  },
  {
    name: "const-dictionary-weak-cache",
    breaks:
      "`ringShow(2)` caches its dictionary in a `WeakMap`, which throws on the number `2`",
    file: "src/lower/traits/impls.rs",
    find: '                false => "Map",\n',
    replace: '                false => "WeakMap",\n',
    tests: ["test/corpus.test.ts", "-t", "const_generics"],
  },
  {
    name: "error-without-source",
    breaks:
      "an `Error` dictionary has no `source`, which a `dyn Error` calls, and JS throws",
    file: "src/lower/traits/impls.rs",
    find: "            let source = self.is_error_source(item.def_id);",
    replace: "            let source = false;",
    tests: ["test/corpus.test.ts", "-t", "dyn_display"],
  },
  {
    name: "dictionary-const-not-taken",
    breaks:
      "a dictionary's entry for `repeat<const N>` doesn't take `N` from its caller",
    file: "src/lower/traits/impls.rs",
    find: "                self.given.const_params.push((index, Expr::var(&name)));\n                params.push(name);",
    replace:
      "                self.given.const_params.push((index, Expr::var(&name)));",
    tests: ["test/corpus.test.ts", "-t", "const_generic_traits"],
  },
  {
    name: "dictionary-entry-own-drops-unlent",
    breaks:
      "a dictionary's entry for a generic method takes its caller's drops but doesn't pass them on",
    file: "src/lower/traits/impls.rs",
    find: "                replaced.push(self.lend_drop_param(name.clone(), ty));",
    replace: "                let _ = (index, ty, &mut replaced);",
    tests: ["test/corpus.test.ts", "-t", "generic_method_drops"],
  },
  {
    name: "blanket-unsized-impl-unresolved",
    breaks:
      "a blanket impl over a `?Sized` type, `impl<Q: ?Sized> Equivalent<K> for Q`, is an error",
    file: "src/lower/traits/impls.rs",
    find: "                .or_else(own)\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "generic_borrow"],
  },
  {
    name: "generic-impl-const-trait-item",
    breaks:
      "a generic impl's constant is looked up as its trait's, which has no body here",
    file: "src/lower/traits/impls.rs",
    find: "                let own = self.tcx.impl_item_implementor_ids(id).get(&item.def_id).copied();",
    replace: "                let own = Some(item.def_id);",
    tests: ["test/corpus.test.ts", "-t", "generic_impl_consts"],
  },
  {
    name: "impl-item-drops-left-out",
    breaks:
      "an impl's dictionary has no drop for its associated types, so generic code holding one drops nothing",
    file: "src/lower/traits/impls.rs",
    find: "        props.extend(self.dictionary_drop(tr.self_ty(), span)?);\n        props.extend(self.item_drops(tr, false, span)?);\n",
    replace: "        props.extend(self.dictionary_drop(tr.self_ty(), span)?);\n",
    tests: ["test/corpus.test.ts", "-t", "associated_type_drops"],
  },
  {
    name: "writer-defaults-missing",
    breaks:
      "a writer of the crate's that keeps std's `write_char` has no `write_char` in its dictionary, and it's refused",
    file: "src/lower/traits/impls.rs",
    find: "if is_writer_default(self.tcx, item.def_id) && !self.krate.fns.contains_key(&method) {",
    replace:
      "if false && is_writer_default(self.tcx, item.def_id) && !self.krate.fns.contains_key(&method) {",
    tests: ["test/corpus.test.ts", "-t", "generic_writers"],
  },
  {
    name: "writer-entry-repeated",
    breaks:
      "a writer's dictionary repeats its `write_str` arrow for `write_char` and `write_fmt`: right, but not the JS a person writes",
    file: "src/lower/traits/impls.rs",
    find: "        if value.is_named() {",
    replace: "        if true {",
    tests: ["test/corpus.test.ts", "-t", "generic_writers"],
    snapshots: true,
  },
  {
    name: "library-default-copied",
    breaks:
      "a consumer's impl copies a library's default, whose body it can't read",
    file: "src/lower/traits/impls.rs",
    find: "            let library_default = self.krate.foreign.in_library(method);\n",
    replace:
      "            let library_default = false && self.krate.foreign.in_library(method);\n",
    tests: ["test/crates.test.ts", "-t", "defaults are what"],
  },
  {
    name: "dictionary-uncached",
    breaks:
      "a generic impl's dictionary is made anew on each call, so two of the same impl given the same aren't the same object",
    file: "src/lower/traits/impls.rs",
    find: "                    vec![Expr::var(cache), keys, make],\n",
    replace:
      '                    vec![Expr::new_(Expr::var("WeakMap"), Vec::new()), keys, make],\n',
    tests: ["test/traits.test.ts", "-t", "dictionaries are explicit"],
  },
  {
    name: "binding-in-dictionary",
    breaks:
      "an impl of a trait of bindings is refused: its methods aren't the crate's",
    file: "src/lower/traits/impls.rs",
    find: "            if self.tcx.def_kind(item.def_id) != DefKind::AssocFn || bindings::is_binding(self.tcx, item.def_id) {\n",
    replace:
      "            if self.tcx.def_kind(item.def_id) != DefKind::AssocFn {\n",
    tests: [
      "test/bindings.test.ts",
      "-t",
      "binding method is read of the value",
    ],
  },
  {
    name: "library-default-const-computed",
    breaks:
      "a library's trait default constant is computed, and `360 / Self::A` of 0 is rustc's error, where no consumer reads it",
    file: "src/lower/traits/impls.rs",
    find: "                    (true, None) => None,\n",
    replace: "",
    tests: [
      "test/lowering-structs-enums.test.ts",
      "-t",
      "default constant of Self",
    ],
  },
  {
    name: "default-const-of-self-refused",
    breaks:
      "a generic impl's trait default constant of `Self`, `Self::A + 1`, is refused, where it's its initializer copied in",
    file: "src/lower/traits/impls.rs",
    find: "                        None => self.default_method(id, item.def_id, tr.args)?,",
    replace:
      '                        None => return Err(self.unsupported(span, "a generic impl\'s default constant of its parameters")),',
    tests: [
      "test/lowering-structs-enums.test.ts",
      "-t",
      "default constant of Self",
    ],
  },
  {
    name: "default-const-literal-getter",
    breaks:
      'a literal default, `"k"`, is a getter that gives it back, where it\'s the field',
    file: "src/lower/traits/impls.rs",
    find: "                            ] if value.is_constant() => Some(value.clone()),",
    replace: "                            ] if false => Some(value.clone()),",
    tests: [
      "test/lowering-structs-enums.test.ts",
      "-t",
      "default constant of Self",
    ],
  },
  {
    name: "dictionary-iterator-unmade",
    breaks:
      "an impl's range for an associated type bound by `Iterator` is given to generic code as a range, which `for .. of` can't step",
    file: "src/lower/traits/impls.rs",
    find: "            && (self.range_kind(ty).is_some()\n                || self.is_user_iterator(ty)\n",
    replace: "            && (false\n                || self.is_user_iterator(ty)\n",
    tests: ["test/corpus.test.ts", "-t", "generic_associated_types"],
  },
  {
    name: "impl-cache-ignores-facts",
    breaks:
      "an impl's dictionary of one size is cached for every size, a `Wrap<u64>`'s footprint counted in a `Wrap<u8>`'s bytes",
    file: "src/lower/traits/impls.rs",
    find: "                    .chain(self.given.type_facts.iter().map(|(_, _, value)| value.clone()))\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^impl_type_facts.rs$"],
  },
  {
    name: "impl-cache-weak-of-facts",
    breaks:
      "an impl's dictionaries keyed by a type's fact, a number, are a `WeakMap`'s, which refuses it",
    file: "src/lower/traits/impls.rs",
    find: "                && (!self.given.evidence.is_empty() || self.given.type_facts.is_empty());\n",
    replace: ";\n",
    tests: ["test/corpus.test.ts", "-t", "^impl_type_facts.rs$"],
  },
  {
    name: "impl-cache-strong-of-dictionaries-and-facts",
    breaks:
      "an impl's dictionaries keyed by dictionaries, then a type's fact, are a `Map`'s, which holds every dictionary it's given for good (#931)",
    file: "src/lower/traits/impls.rs",
    find: "                && (!self.given.evidence.is_empty() || self.given.type_facts.is_empty());\n",
    replace: "                && self.given.type_facts.is_empty();\n",
    tests: ["test/lowering-std-unwinds.test.ts", "-t", "generic code asks"],
  },
  {
    name: "own-clone-from-not-in-dictionary",
    breaks:
      "an impl's own `clone_from` isn't in its `Clone` dictionary, so a generic `clone_from_slice` clones each item instead",
    file: "src/lower/traits/impls.rs",
    find: "                && !own_clone_from\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^clone_from_slice_generic.rs$"],
  },
  {
    name: "io-write-default-refused",
    breaks:
      "a writer of bytes of the crate's has no `write_all` or `write_fmt`, std's defaults, so it's refused where a `W: Write` goes",
    file: "src/lower/traits/impls.rs",
    find: "            if let Some(all) = io_write_default(self.tcx, item.def_id)\n",
    replace:
      "            if let Some(all) = io_write_default(self.tcx, item.def_id).filter(|_| false)\n",
    tests: ["test/corpus.test.ts", "-t", "^io_write_impl.rs$"],
  },
  {
    name: "io-write-all-as-fmt",
    breaks: "a writer of the crate's `write_all` takes its bytes as text",
    file: "src/lower/traits/impls.rs",
    find: '                    true => "$ioWriteAll",\n',
    replace: '                    true => "$ioWriteFmt",\n',
    tests: ["test/corpus.test.ts", "-t", "^io_write_impl.rs$"],
  },
  {
    name: "dictionary-item-facts-missing",
    breaks:
      "an impl's dictionary has no `ItemSize`, so generic code reads `undefined`",
    file: "src/lower/traits/impls.rs",
    find: "        props.extend(self.item_facts(tr, span)?);\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^associated_type_facts.rs$"],
  },
  {
    name: "impossible-method-in-dictionary",
    breaks:
      "a default whose `where` clauses an impl can't meet is copied into its dictionary, and refused",
    file: "src/lower/traits/impls.rs",
    find: "            if self.tcx.instantiate_and_check_impossible_clauses((item.def_id, args)) {\n",
    replace: "            if false {\n",
    snapshots: true,
    tests: ["test/corpus.test.ts", "-t", "^default_method_where.rs$"],
  },
  {
    name: "dyn-shim-location-undefined",
    breaks:
      "an impl tracking its caller where its declaration doesn't, called through a `dyn`, is given no location, not its definition's",
    file: "src/lower/traits/impls.rs",
    find: "                        defaulted = Some((name.clone(), at_definition));\n",
    replace:
      "                        defaulted = Some((name.clone(), at_definition)).filter(|_| false);\n",
    tests: ["test/corpus.test.ts", "-t", "^track_caller_traits.rs$"],
  },
  {
    name: "error-defaults-unwritten",
    breaks:
      "an `Error` dictionary's `description` and `cause` are its impl's, which std's aren't, and are refused",
    file: "src/lower/traits/impls.rs",
    find: "            if let Some(cause) = error_default(self.tcx, item.def_id)\n",
    replace: "            if let Some(cause) = None::<bool>\n",
    tests: ["test/corpus.test.ts", "-t", "^error_description_cause.rs$"],
  },
  {
    name: "error-defaults-override-own",
    breaks: "an impl's own `description` and `cause` are replaced by std's",
    file: "src/lower/traits/impls.rs",
    find: "            if let Some(cause) = error_default(self.tcx, item.def_id)\n                && !self.krate.fns.contains_key(&method)\n",
    replace:
      "            if let Some(cause) = error_default(self.tcx, item.def_id)\n",
    tests: ["test/corpus.test.ts", "-t", "^error_description_cause.rs$"],
  },
  {
    name: "mut-result-of-impl-unboxed",
    breaks:
      "an impl's `&mut Vec` given where the trait's signature says a generic `&mut Self::Out`, a box, is the array, and its caller's `.value` of it `undefined`",
    file: "src/lower/traits/impls.rs",
    find: "        let result_cell = self.declares_box(item, output)\n",
    replace: "        let result_cell = false && self.declares_box(item, output)\n",
    tests: ["test/corpus.test.ts", "-t", "^mut_assoc_through_dictionary.rs$"],
  },
  {
    name: "into-iter-given-box-unread",
    breaks:
      "std's `IntoIterator` of a `&mut Vec` reads the box generic code gives as the array",
    file: "src/lower/traits/impls.rs",
    find: "        matches!(*self_ty.kind(), ty::Ref(_, pointee, Mutability::Mut) if !self.mut_ref_form(pointee).is_cell())\n",
    replace: "        false\n",
    tests: ["test/corpus.test.ts", "-t", "^into_iter_of_ref_param.rs$"],
  },
  {
    name: "self-held-not-unboxed",
    breaks:
      "an impl for `&mut Money` through a dictionary is given generic code's box of a box, and reads the box as the object",
    file: "src/lower/traits/impls.rs",
    find: "                let given = usize::from(self.param_is_box(item, i)) + usize::from(i == 0 && self_held);\n",
    replace:
      "                let given = usize::from(self.param_is_box(item, i));\n",
    tests: [
      "test/corpus.test.ts",
      "-t",
      "^(mut_ref_to_object_impl|into_iter_of_ref_param).rs$",
    ],
  },
  {
    name: "fmt-result-not-generic",
    breaks:
      "an impl's `fmt::Result` given where generic code has a `Result<(), Self::Error>` is `undefined`, whose `TAG` it reads",
    file: "src/lower/traits/impls.rs",
    find: "            let call = match self.generic_fmt_result(declared_output, args) {\n",
    replace:
      "            let call = match false && self.generic_fmt_result(declared_output, args) {\n",
    tests: ["test/corpus.test.ts", "-t", "^formatter_sink.rs$"],
  },
  {
    name: "hasher-dictionary-unfilled",
    breaks:
      "a crate's hasher's dictionary has none of the provided methods its impl doesn't write",
    file: "src/lower/traits/impls.rs",
    find: "        let object = match is_hasher(self.tcx, tr.def_id) {\n",
    replace: "        let object = match false {\n",
    tests: ["test/corpus.test.ts", "-t", "^hash_hashers.rs$"],
  },
  {
    name: "entry-own-facts-unlent",
    breaks:
      "a dictionary's entry passes no facts its caller gives of the method's own type parameters",
    file: "src/lower/traits/impls.rs",
    find: "            params.extend(fact_names);\n",
    replace: "            let _ = fact_names;\n",
    tests: ["test/corpus.test.ts", "-t", "^own_type_facts.rs$"],
  },
  {
    name: "blanket-unsized-default-refused",
    breaks:
      "a blanket impl over `?Sized` types that keeps its trait's default methods, `Itertools`'s, is refused",
    file: "src/lower/traits/impls.rs",
    find: "                    .has_value()\n                    .then(|| ty::Instance::new_raw(item.def_id, args)),",
    replace:
      "                    .has_value()\n                    .then(|| ty::Instance::new_raw(item.def_id, args))\n                    .filter(|_| false),",
    tests: ["test/corpus.test.ts", "-t", "^blanket_unsized_impl.rs$"],
  },
  {
    name: "dictionary-coded-const-getter",
    breaks:
      "a dictionary's entry of an impl's function constant lowers its initializer again, where it's the module's constant it is (ADR 0410)",
    file: "src/lower/traits/impls.rs",
    find: "                        Some(own) if crate::lower::analysis::impl_const(self.tcx, own) => Expr::arrow(\n",
    replace:
      "                        Some(own) if false && crate::lower::analysis::impl_const(self.tcx, own) => Expr::arrow(\n",
    tests: ["test/corpus.test.ts", "-t", "^impl_fn_consts.rs$"],
    snapshots: true,
  },
  {
    name: "dictionary-const-a-function",
    breaks:
      "a dictionary that takes nothing is a function, which its readers take for the object",
    file: "src/lower/traits/impls.rs",
    find: "        if !self.takes_evidence(id) {\n",
    replace: "        if false {\n",
    tests: [
      "test/lowering-dictionaries.test.ts",
      "-t",
      "a dictionary that takes nothing",
    ],
  },
  {
    name: "instance-dictionary-exported",
    breaks:
      "a dictionary a consumer compiles of its library's impl is exported, as if its own (ADR 0914, amended)",
    file: "src/lower/traits/impls.rs",
    find: "            export: self.tcx.visibility(tr.def_id).is_public() && !self.krate.foreign.is_instance(id),\n",
    replace:
      "            export: self.tcx.visibility(tr.def_id).is_public(),\n",
    tests: ["test/crates-compiled-here.test.ts", "-t", "program's, alone or a dependency"],
  },
  {
    name: "entry-wrapper-kept",
    breaks:
      "a dictionary's entry that passes its first parameters on to its impl's method, which declares just those, wraps it, `(arg0, TIntoIterInPlace) => bagFromIteratorU32_fromIter(arg0)`, where it's the method itself (#801)",
    file: "src/lower/traits/impls.rs",
    find: "                Some(own)\n            });\n",
    replace: "                Some(own).filter(|_| false)\n            });\n",
    tests: ["test/corpus.test.ts", "-t", "^generic_collect.rs$"],
    snapshots: true,
  },
  {
    name: "entry-takes-any-box-apart",
    breaks: "a blanket impl's method, of a generic `Self` that writes its box's `value`, is taken for one that takes the box apart, so a `&mut dyn` of it whose `*self = ..` replaces the whole leaves the caller's object as it was (a wrong answer, #831, #968)",
    file: "src/lower/traits/impls.rs",
    find: "        Some(forms.unboxed.get(i).is_some_and(|&unboxed| unboxed > 0))\n",
    replace: "        Some(forms.unboxed.get(i).is_some())\n",
    tests: ["test/corpus.test.ts", "-t", "^dyn_of_a_generic_impl_written_whole.rs$"],
  },
  {
    name: "entry-of-own-type-as-method",
    breaks:
      "a library's impl's method whose trait's signature names `Self`'s associated type is called through its dictionary's entry, which gives generic code's form of it, a JS iterator of `Upto(2).numbers()`'s `Range` (#1032)",
    file: "src/lower/traits/impls.rs",
    find: "            && !names_own_type\n",
    replace: "",
    tests: ["test/crates.test.ts", "-t", "two crates: library_singles_across_crates"],
  },
  {
    name: "library-entry-unboxes-unknown",
    breaks:
      "a library's blanket impl has no entry to its consumer, so `&mut c as &mut dyn Reset` is a box whose `*self = T::default()` leaves `c` as it was (#1032)",
    file: "src/lower/traits/impls.rs",
    find: "            .filter(|(imp, _)| self.krate.foreign.compiled_by_rust_js(*imp))?;\n",
    replace: "            .filter(|(imp, _)| imp.is_local())?;\n",
    tests: ["test/crates.test.ts", "-t", "two crates: library_singles_across_crates"],
  },
  {
    name: "own-fact-given-in-trait-terms",
    breaks: "a consumer's generic impl's entry for a library's default gives a map's class of `<Self as Iterator>::Item`, which the default, asking `<I as Iterator>::Item`, doesn't find, so it's refused (#990)",
    file: "src/lower/traits/impls.rs",
    find: "                let of = self.drop_slot_of(item.def_id, index, args);\n",
    replace: "                let of = drop_slot(self.tcx, self.slots(), item.def_id, index);\n",
    tests: ["test/crates.test.ts", "-t", "two crates: library_default_map_keys"],
  },
  {
    name: "library-class-default-through-its-library",
    breaks:
      "a consumer's class's default of a library's trait of classes is a call of the library's default given a dictionary, `render_render(this, arg1, leafRender)`, of a dictionary nothing makes, not its prototype's method (#1073)",
    file: "src/lower/traits/impls.rs",
    find: "            if (!library_default || crate::lower::classes::is_class_trait(self.tcx, tr.def_id))\n",
    replace: "            if !library_default\n",
    tests: ["test/library-classes.test.ts", "-t", "on the prototype"],
  },
  {
    name: "made-js-iterator-of-std-struct-unseen",
    breaks: "generic code takes std's struct of an adapter an impl gives for an associated iterator for a JS iterator that knows its back, stepping it as an empty one (#1292)",
    file: "src/lower/traits/impls.rs",
    find: "                || crate::lower::recognition::is_compiled_std_iterator(self.tcx, ty))\n",
    replace: "                || false)\n",
    tests: ["test/corpus.test.ts", "-t", "^associated_iterator_of_stds_struct_from_its_back.rs$"],
  },
  {
    name: "entry-output-unnormalized",
    breaks: "an impl's entry asks of its method's output as written, `<Tens as Source>::Iter`, not std's struct the impl makes, and makes no JS iterator of it (#1292)",
    file: "src/lower/traits/impls.rs",
    find: "                .unwrap_or(output);\n            let env = tcx.param_env(item.def_id);\n",
    replace: "                .map(|_| output)\n                .unwrap_or(output);\n            let env = tcx.param_env(item.def_id);\n",
    tests: ["test/corpus.test.ts", "-t", "^associated_iterator_of_stds_struct_from_its_back.rs$"],
  },
  {
    name: "dictionary-output-of-parameters-normalized",
    breaks: "what an impl's method gives is normalized in the dictionary's environment, which doesn't hold the method's own type or const parameters, and `fn foo<const N: usize>(&self) -> Self::Assoc<N>` crashes rustc: \"cannot find `N/#1` in param-env\" (#1304, #1308)",
    file: "src/lower/traits/impls.rs",
    find: "                .try_normalize_erasing_regions(method_env, ty::Unnormalized::new_wip(output))\n",
    replace: "                .try_normalize_erasing_regions(self.typing_env, ty::Unnormalized::new_wip(output))\n",
    tests: ["test/corpus.test.ts", "-t", "^const_generic_assoc_type_of_a_dictionary.rs$"],
  },
  {
    name: "entry-output-of-own-generics-unnormalized",
    breaks: "what an impl's method with generics of its own gives, `items<U>`'s `Self::Iter` or `hidden<const M>`'s `impl Trait`, is left as written, so generic code takes std's struct for a JS iterator that knows its back: `it.back is not a function` (#1308)",
    file: "src/lower/traits/impls.rs",
    find: "            let output = tcx\n                .try_normalize_erasing_regions(method_env, ty::Unnormalized::new_wip(output))\n                .unwrap_or(output);\n",
    replace: "            let output = match tcx.generics_of(method).own_requires_monomorphization() {\n                true => output,\n                false => tcx\n                    .try_normalize_erasing_regions(method_env, ty::Unnormalized::new_wip(output))\n                    .unwrap_or(output),\n            };\n",
    tests: ["test/corpus.test.ts", "-t", "^associated_iterator_of_a_method_with_its_own_generics.rs$"],
  },
  {
    name: "entry-form-in-the-dictionarys-env",
    breaks: "an impl's entry picks the form of what its method gives in the dictionary's environment, without the method's own bounds: `tagged<U: Copy>`'s closure copying a `U` isn't an `FnMut` there, so its `Map` isn't made to know its back (#1308)",
    file: "src/lower/traits/impls.rs",
    find: "            let dictionary_env = std::mem::replace(&mut self.typing_env, method_env);\n",
    replace: "            let dictionary_env = self.typing_env;\n",
    tests: ["test/corpus.test.ts", "-t", "^associated_iterator_of_a_method_with_its_own_generics.rs$"],
  },
  {
    name: "entry-facts-without-the-methods-bounds",
    breaks: "a dictionary's entry makes the facts its impl's method asks of its own type parameter in the dictionary's environment, without the method's bounds, so a crate's `from_iter` that collects a `Vec` of what it's given doesn't know its items and is refused (#1239)",
    file: "src/lower/traits/impls.rs",
    find: "            let outer_env = std::mem::replace(&mut self.typing_env, own_env);\n",
    replace: "            let outer_env = self.typing_env;\n",
    tests: ["test/corpus.test.ts", "-t", "^(collect_fallible_into|from_iter_entry_drops_its_items).rs$"],
  },
  {
    name: "entry-facts-under-the-impls-bounds",
    breaks: "a dictionary's entry makes its impl's method's facts under the impl's bounds too, by which its impl's evidence is taken for a bound's and refused (#1239)",
    file: "src/lower/traits/impls.rs",
    find: "                ty::ClauseKind::Trait(bound) => own(bound.self_ty()),\n",
    replace: "                ty::ClauseKind::Trait(_) => true,\n",
    tests: ["test/crates.test.ts", "-t", "generic_impl_const_of_instance"],
  },
  {
    name: "std-reader-dictionary-takes-std-defaults",
    breaks: "a `Chain`'s or a `Take`'s dictionary holds std's provided `read_exact` and the rest it doesn't write, whose bodies reach memory, where `$readerDefaults` holds them, and `@rust-js/std` can't be built (#1156)",
    file: "src/lower/traits/impls.rs",
    find: "                && crate::lower::recognition::in_io_dictionary(self.tcx, item.def_id)\n                && !self.krate.fns.contains_key(&method)\n",
    replace: "                && false\n",
    tests: ["test/std-package.test.ts", "-t", "the committed @rust-js/std is std"],
  },
  {
    name: "error-dictionary-no-type-id",
    breaks: "a crate's `Error` dictionary carries no `typeId`, so a `dyn Error`'s downcast to its type finds `undefined`, `None`, where Rust finds it",
    file: "src/lower/traits/impls.rs",
    find: "                if !crate::lower::generic_facts::error_impl_has_type_id(self.tcx, id) {\n",
    replace: "                if true {\n",
    tests: ["test/corpus.test.ts", "-t", "^dyn_error_downcast.rs$"],
  },
  {
    name: "dictionary-fact-given-twice-refused",
    breaks:
      "a default compiled over `Self` that asks `<Self as Iterator>::Item`'s size, of a library's impl the crate compiles itself whose dictionary has it, is refused, \"a trait method's type parameter's fact here\", where its caller gives it in its place (#1409)",
    file: "src/lower/traits/impls.rs",
    find: "                if !(self.given.type_facts.iter()).any(|&(at, given, _)| at == of && given == fact) {\n",
    replace: "                if (self.given.type_facts.iter()).any(|&(at, given, _)| at == of && given == fact) {\n                    return Err(self.unsupported(span, \"a trait method's type parameter's fact here\"));\n                }\n                {\n",
    tests: ["test/crates.test.ts", "-t", "^two crates: library_instance_impl_asks_its_defaults_facts$"],
  },
  {
    name: "own-reader-impl-no-defaults",
    breaks: "a crate's reader's dictionary is its impl's alone, without std's provided methods over its `read` (#1297)",
    file: "src/lower/traits/impls.rs",
    find: "            false if self.own_reader_impl(tr) => Expr::call(Expr::helper(\"$readerDefaults\"), vec![Expr::object(props)]),\n",
    replace: "",
    tests: [
      "test/corpus.test.ts",
      "-t",
      "^read_of_a_crates_own_reader.rs$"
    ]
  },
  {
    name: "own-seek-impl-no-defaults",
    breaks: "a crate's own `Seek`'s dictionary is its impl's alone, without std's `stream_position` and `seek_relative` over its `seek` (#1489)",
    file: "src/lower/traits/impls.rs",
    find: "            false if self.own_seek_impl(tr) => Expr::call(Expr::helper(\"$seekDefaults\"), vec![Expr::object(props)]),\n",
    replace: "",
    tests: [
      "test/corpus.test.ts",
      "-t",
      "^readers_of_the_crates_own_and_boxed.rs$"
    ]
  },
  {
    name: "own-seek-provided-copied",
    breaks: "a crate's own `Seek`'s provided `stream_position` is copied as a default of its trait, which is refused (#1489)",
    file: "src/lower/traits/impls.rs",
    find: "                || self.own_seek_impl(tr))\n",
    replace: "                || false)\n",
    tests: [
      "test/corpus.test.ts",
      "-t",
      "^readers_of_the_crates_own_and_boxed.rs$"
    ]
  },
  {
    name: "own-write-fmt-entry-given-text",
    breaks: "a writer's own `write_fmt` reached through generic code's dictionary is given its text, which its body calls as a function (#1695)",
    file: "src/lower/traits/impls.rs",
    find: "            let value = match crate::lower::recognition::is_write_fmt(self.tcx, item.def_id) {\n",
    replace: "            let value = match false {\n",
    tests: ["test/corpus.test.ts", "-t", "^writer_with_its_own_write_fmt.rs$"],
  },
];
