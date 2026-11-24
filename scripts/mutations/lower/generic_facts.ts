// Mutations of crates/rust-js-lower/src/generic_facts.rs (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "fn-bound-least-taken",
    breaks: "of a parameter's `Fn` bounds, `F: Fn`'s with its supertraits, the least is taken, `FnOnce`, so a function value it calls through `Fn::call` names `call_once`'s line (#2335)",
    file: "crates/rust-js-lower/src/generic_facts.rs",
    find: "    [ty::ClosureKind::Fn, ty::ClosureKind::FnMut, ty::ClosureKind::FnOnce]\n",
    replace: "    [ty::ClosureKind::FnOnce, ty::ClosureKind::FnMut, ty::ClosureKind::Fn]\n",
    tests: ["test/corpus.test.ts", "-t", "^std_function_value_panic_fn.rs$"],
  },
  {
    name: "placed-form-unasked",
    breaks:
      "a fieldless enum's iterator given by value to generic code is given bare, so std's `take` steps the string and its own `&mut self` step throws, \"Cannot create property 'value' on string\" (#2255)",
    file: "crates/rust-js-lower/src/generic_facts.rs",
    find: "        let form = [Form::Handle, Form::Box, Form::HeldBox, Form::Tagged, Form::Placed]",
    replace: "        let form = [Form::Handle, Form::Box, Form::HeldBox, Form::Tagged]",
    tests: ["test/corpus.test.ts", "-t", "^non_object_iterator_given_to_generic_code\\.rs$"],
  },
  {
    name: "placed-into-iter-unplaced",
    breaks:
      "what an `IntoIterator` of generic code's makes of a fieldless enum's iterator, `zip(Tri::C)`'s `U::IntoIter`, isn't in a place, so std's `Zip` steps the bare string (#2255)",
    file: "crates/rust-js-lower/src/generic_facts.rs",
    find: "        (matches!(declared.kind(), ty::Param(_)) || made_by_into_iter)\n",
    replace: "        matches!(declared.kind(), ty::Param(_))\n",
    tests: ["test/corpus.test.ts", "-t", "^non_object_iterator_given_to_generic_code\\.rs$"],
  },
  {
    name: "placed-impl-param-unheld",
    breaks:
      "std's derived `Clone` of `Take<I>`, whose `I` its `Iterator` impl holds in a place, holds it as it is, so `cycle()` of a `take()` of a fieldless enum's iterator is refused (#2255)",
    file: "crates/rust-js-lower/src/generic_facts.rs",
    find: "                || self.held_in_place_by_its_impl(def_id, declared))\n",
    replace: "                || false)\n",
    tests: ["test/corpus.test.ts", "-t", "^non_object_iterator_given_to_generic_code\\.rs$"],
  },
  {
    name: "held-box-given-back-as-it-is",
    breaks:
      "what generic code gives back of a `&mut W` whose `W` is a `&mut` to an object, its box of the object's cell, is read as the caller's box of the object: `keep(&mut by).write_all(..)` of `by: &mut Vec<u8>` throws `push is not a function` (#1303)",
    file: "crates/rust-js-lower/src/generic_facts.rs",
    find: '            Form::HeldBox => Expr::call_helper("$heldObjectOf", vec![value]),',
    replace: "            Form::HeldBox => value,",
    tests: ["test/corpus.test.ts", "-t", "^mut_of_a_held_mut_given_back_by_generic_code.rs$"],
  },
  {
    name: "held-box-of-no-box",
    breaks:
      "a `&mut W` whose `W` is given a `&mut` to an object is taken as no box of its box, so what's given is the caller's box and what's given back is read as it (#1300, #1303)",
    file: "crates/rust-js-lower/src/generic_facts.rs",
    find: "                    self.form_differs(Form::Box, (def_id, declared, instantiated))\n",
    replace: "                    false\n",
    tests: ["test/corpus.test.ts", "-t", "^mut_of_a_held_mut_given_(to|back_by)_generic_code.rs$"],
  },
  {
    name: "box-form-unasked",
    breaks:
      "generic code's box of an object's `&mut` isn't a form that differs: a library's function given as an `F` reads its box as the object, and a `fn` pointer gives the instance the object",
    file: "crates/rust-js-lower/src/generic_facts.rs",
    find: "        let form = [Form::Handle, Form::Box, Form::HeldBox, Form::Tagged, Form::Placed]",
    replace: "        let form = [Form::Handle, Form::HeldBox, Form::Tagged, Form::Placed]",
    tests: ["test/crates-pairs-3.test.ts", "-t", "library_generic_fn_item_forms"],
  },
  {
    name: "reified-object-mut-unboxed",
    breaks:
      "a `fn` pointer of an object's `&mut T` gives the instance the object where it takes generic code's box, `view::<Vec<u8>>` reading `r.value` of the vector",
    file: "crates/rust-js-lower/src/generic_facts.rs",
    find: "                self.in_place(InPlaceWriter::CellOf, written, vec![value], span)?\n",
    replace: "                value\n",
    tests: ["test/corpus.test.ts", "-t", "^generic_fn_pointer_takes_its_types_forms\\.rs$"],
  },
  {
    name: "shared-number-let-out-allowed",
    breaks:
      "a `&u32` given to generic code that may let it out as a raw pointer compiles, its pointer `$ptrTo(5)` thrown at run time",
    file: "crates/rust-js-lower/src/generic_facts.rs",
    find: "            .then_some(referent)",
    replace: "            .then_some(referent)\n            .filter(|_| false)",
    tests: ["test/corpus.test.ts", "-t", "^shared_number_given_to_let_a_pointer_out_refused\\.rs$"],
  },
  {
    name: "generic-shared-ref-let-out-refused",
    breaks:
      "generic code's own `&T` given on to code that may let it out as a raw pointer is refused, rust_decimal's `&Self`, where only its callers know `T`",
    file: "crates/rust-js-lower/src/generic_facts.rs",
    find: "        (!self.is_unknown(referent) && !self.is_object(referent) && !self.pointee_is_itself(referent))",
    replace: "        (!self.is_object(referent) && !self.pointee_is_itself(referent))",
    tests: ["test/corpus.test.ts", "-t", "^reference_given_as_its_pointer\\.rs$"],
  },
  {
    name: "raw-pointer-given-unadapted",
    breaks:
      "a slice's pointer, the array, given for generic code's `*mut T` isn't a handle, and its `value` is read of the array",
    file: "crates/rust-js-lower/src/generic_facts.rs",
    find: "            Form::Handle => Expr::call_helper(\"$ptrTo\", vec![value]),",
    replace: "            Form::Handle => value,",
    tests: ["test/corpus.test.ts", "-t", "^reference_given_as_its_pointer\\.rs$"],
  },
  {
    name: "raw-pointer-form-of-any-pointee",
    breaks:
      "a raw pointer of a concrete pointee is adapted too, an object's handle wrapped again",
    file: "crates/rust-js-lower/src/generic_facts.rs",
    find: "                matches!(pointee(declared).map(|p| *p.kind()), Some(ty::Param(_)))\n                    && pointee(instantiated).is_some_and(|p| self.pointee_is_itself(p)",
    replace: "                pointee(declared).is_some()\n                    && pointee(instantiated).is_some_and(|p| self.pointee_is_itself(p)",
    tests: ["test/corpus.test.ts", "-t", "^reference_given_as_its_pointer\\.rs$"],
  },
  {
    name: "param-given-mut-raw",
    breaks:
      "a `&mut` to an object given as a type parameter's value is the object, which an impl for `&mut T` reads as a box, and JS throws",
    file: "crates/rust-js-lower/src/generic_facts.rs",
    find: "            ty::Param(_) => self.mut_ref_form_in(declared, param_env).is_generic_box(),\n",
    replace: "            ty::Param(_) => false,\n",
    tests: ["test/corpus.test.ts", "-t", "^mut_given_as_type_param.rs$"],
  },
  {
    name: "boxed-through-shared-unpeeled",
    breaks:
      "an impl for `&mut T`'s `&self`, called where its type is known, is given the object, not a box of it",
    file: "crates/rust-js-lower/src/generic_facts.rs",
    find: "                    (declared, instantiated) = (d, i);\n",
    replace: "                    let _ = (d, i);\n                    break;\n",
    tests: ["test/corpus.test.ts", "-t", "^mut_given_as_type_param.rs$"],
  },
  {
    name: "fn-bound-mut-boxed",
    breaks:
      "a `&mut F` of an `F: FnMut()` is generic code's box too, so a closure given for it is `$cellOf(f)`, which generic code calls",
    file: "crates/rust-js-lower/src/generic_facts.rs",
    find: "            ty::Ref(..) => self.is_generic_box_in(declared, param_env),\n",
    replace: "            ty::Ref(_, pointee, _) => matches!(pointee.kind(), ty::Param(_)),\n",
    tests: ["test/corpus.test.ts", "-t", "^closure_mut_ref\\.rs$"],
  },
  {
    name: "held-box-form-unasked",
    breaks:
      "a `&mut W` of a `W` given a `&mut` to an object isn't a form a function value is asked: `stamped` given to std's `map`, or as a `fn` pointer, is given its callers' box where it takes a box of its box (#1322)",
    file: "crates/rust-js-lower/src/generic_facts.rs",
    find: "        let form = [Form::Handle, Form::Box, Form::HeldBox, Form::Tagged, Form::Placed]",
    replace: "        let form = [Form::Handle, Form::Box, Form::Tagged, Form::Placed]",
    tests: ["test/corpus.test.ts", "-t", "^generic_fn_value_takes_its_callers_forms\\.rs$"],
  },
  {
    name: "tagged-form-unasked",
    breaks: "a write's result given to generic code that has a `Result` of its type parameters keeps its own form, and generic code reads `undefined.TAG` (#1408)",
    file: "crates/rust-js-lower/src/generic_facts.rs",
    find: "        let form = [Form::Handle, Form::Box, Form::HeldBox, Form::Tagged, Form::Placed]\n",
    replace: "        let form = [Form::Handle, Form::Box, Form::HeldBox, Form::Placed]\n",
    tests: ["test/corpus.test.ts", "-t", "^write_result_given_to_generic_code.rs$"],
  },
  {
    name: "write-tagged-as-it-is",
    breaks: "a write's result given where generic code has a `Result` is given as it is, `undefined`, which generic code reads as no `Result` (#1408)",
    file: "crates/rust-js-lower/src/generic_facts.rs",
    find: "            Form::Tagged => Expr::call_helper(\"$writeTagged\", vec![value]),\n",
    replace: "            Form::Tagged => value,\n",
    tests: ["test/corpus.test.ts", "-t", "^write_result_given_to_generic_code.rs$"],
  },
  {
    name: "write-of-as-it-is",
    breaks: "generic code's `Result` given back where its caller has a write's result is `{ TAG }`, which the caller reads as an `Err` (#1408)",
    file: "crates/rust-js-lower/src/generic_facts.rs",
    find: "            Form::Tagged => Expr::call_helper(\"$writeOf\", vec![value]),\n",
    replace: "            Form::Tagged => value,\n",
    tests: ["test/corpus.test.ts", "-t", "^write_result_given_to_generic_code.rs$|^formatter_sink.rs$"],
  },
  {
    name: "held-write-result-allowed",
    breaks: "a write's result inside a `Vec<Result<T, E>>` given to generic code is given as it is, whose items generic code reads as `{ TAG }` (#1416)",
    file: "crates/rust-js-lower/src/generic_facts.rs",
    find: "        if self.tagged_where(declared, instantiated) {\n            return Some(instantiated);\n        }\n",
    replace: "",
    tests: ["test/diagnostics.test.ts", "-t", "write's result inside generic code"],
  },
  {
    name: "valued-form-unasked",
    breaks: "a closure that gives a write's result to std's `Map` struct, which holds it as its `B`, throws its error past what steps it, not an item (#1410)",
    file: "crates/rust-js-lower/src/generic_facts.rs",
    find: "            .or_else(|| self.form_differs(Form::Valued, crossing).then_some(Form::Valued))\n",
    replace: "",
    tests: ["test/iterator-forms.test.ts", "-t", "^write_result_items with"],
  },
  {
    name: "write-result-through-bound-allowed",
    breaks: "a write's result given for an `impl Get<Result<T, E>>` is given as it is, which generic code reads as `{ TAG }` through the bound (#1416)",
    file: "crates/rust-js-lower/src/generic_facts.rs",
    find: "        if !matches!(declared.kind(), ty::Param(_)) || !self.is_write_result(instantiated) {\n",
    replace: "        if true {\n",
    tests: ["test/diagnostics.test.ts", "-t", "write's result read as a Result through a bound"],
  },
  {
    name: "dyn-pair-boxed-again",
    breaks: "a `&mut dyn Hasher` given for a type parameter, which generic code holds as the pair, is read through a box too: a hash through it throws (#1414)",
    file: "crates/rust-js-lower/src/generic_facts.rs",
    find: " && !pointee.is_trait(),\n",
    replace: ",\n",
    tests: ["test/corpus.test.ts", "-t", "^hash_hashers.rs$"],
  },
  {
    name: "field-crossing-unasked",
    breaks: "a struct's `&mut T` field made where `T` is known holds the object, where its generic methods read generic code's box: `slot.set(..)` writes `undefined` (#1405)",
    file: "crates/rust-js-lower/src/generic_facts.rs",
    find: "        (self.field_crossing_of(adt, def, instantiated)).filter(|&crossing| self.form_differs(Form::Box, crossing))\n",
    replace: "        (self.field_crossing_of(adt, def, instantiated)).filter(|&crossing| self.form_differs(Form::Box, crossing) && false)\n",
    tests: ["test/corpus.test.ts", "-t", "^mut_ref_fields_of_a_type_parameter.rs$"],
  },
  {
    name: "own-parameter-mut-unboxed",
    breaks: "generic code's own `&mut T` given to std's impl that passes `AsRef` on to `T` gives `T`'s dictionary the box, not what it holds, so `held.as_ref().len()` is 0",
    file: "crates/rust-js-lower/src/generic_facts.rs",
    find: "            true => self.is_cell(ty),\n",
    replace: "            true => false,\n",
    tests: ["test/corpus.test.ts", "-t", "^mut_ref_to_its_own_type_parameter_through_std_impls.rs$"],
  },
  {
    name: "bare-parameter-writes-unseen",
    breaks: "a `&mut` given where a function declares a bare type parameter, `Pin::new(&mut b)`, isn't taken to be written whole, so `let mut b = a` shares `a` and `set` refuses (#1665)",
    file: "crates/rust-js-lower/src/generic_facts.rs",
    find: "        ty::Param(_) => true,\n        _ => false,\n",
    replace: "        _ => false,\n",
    tests: ["test/corpus.test.ts", "-t", "^copy_written_whole_through_a_pin_is_its_own.rs$"],
  },
  {
    name: "reified-writer-takes-its-formatter",
    breaks: "a generic writer function's pointer whose forms differ from its callers' takes its `Formatter`, which its JS doesn't (#1710)",
    file: "crates/rust-js-lower/src/generic_facts.rs",
    find: "            .filter(|&(at, _)| Some(at) != formatter)\n",
    replace: "            .filter(|_| true)\n",
    tests: ["test/corpus.test.ts", "-t", "^writer_fn_pointer_at_an_instance.rs$"],
  },
  {
    name: "reified-writer-pointer-options-dropped",
    breaks: "a generic writer function's instance whose forms differ, as a `fn` pointer, takes no options (#1758)",
    file: "crates/rust-js-lower/src/generic_facts.rs",
    find: "        let options = formatter.is_some() && self.takes_options(def_id);\n",
    replace: "        let options = false;\n",
    tests: ["test/corpus.test.ts", "-t", "^(writer_fn_pointer_of_an_instance_given_options)\\.rs$"],
  },
  {
    name: "placed-field-of-own-type-unheld",
    breaks:
      "a type parameter of the crate's own iterator that its `Iterator` impl holds in a place, `I` of `Both<I, J>`, isn't held so by the type, so `then_all`'s `Both<Self, J::IntoIter>` given `Never` is refused (#2255)",
    file: "crates/rust-js-lower/src/generic_facts.rs",
    find: "            DefKind::Impl { .. } | DefKind::Struct | DefKind::Enum => def_id,\n",
    replace: "            DefKind::Impl { .. } => def_id,\n",
    tests: ["test/corpus.test.ts", "-t", "^trait_default_takes_an_iterator_as_its_callers_give_it\\.rs$"],
  },
  {
    name: "placed-field-walked-as-held-otherwise",
    breaks:
      "a field of the crate's own type held in a place of its own is walked as held as it is inside what crosses into generic code, so a `Both<Never, Three>` given back by a trait default is refused (#2255)",
    file: "crates/rust-js-lower/src/generic_facts.rs",
    find: "                    let held = of(dargs).has_param() && self.placed_field(field, of(iargs));\n",
    replace: "                    let held = false;\n",
    tests: ["test/corpus.test.ts", "-t", "^trait_default_takes_an_iterator_as_its_callers_give_it\\.rs$"],
  },
  {
    name: "placed-field-known-walked-as-generic",
    breaks:
      "a field of the crate's own type held in a place of its own, inside what's given a function that knows the type, `first_of(Wrap(t, _): Wrap<Tri>)`, is walked as a type parameter's that function holds otherwise, and the call is refused (#2255)",
    file: "crates/rust-js-lower/src/generic_facts.rs",
    find: "                    let held = of(dargs).has_param() && self.placed_field(field, of(iargs));\n",
    replace: "                    let held = self.placed_field(field, of(iargs));\n",
    tests: ["test/corpus.test.ts", "-t", "^iterator_field_in_its_place_copied_cloned_compared\\.rs$"],
  },
  {
    name: "placed-tuple-parts-unasked",
    breaks:
      "a tuple of a fieldless enum's iterator and a unit struct's given by value to generic code that holds each in a place is given as its caller holds it, so `firsts((Light::Green, Sevens))`'s `next` stores into the bare string (#2255)",
    file: "crates/rust-js-lower/src/generic_facts.rs",
    find: "        if let (ty::Tuple(d), ty::Tuple(i)) = (*declared.kind(), *instantiated.kind()) {\n",
    replace: "        if let (ty::Tuple(d), ty::Tuple(i)) = (*declared.kind(), *instantiated.kind())\n            && false\n        {\n",
    tests: ["test/corpus.test.ts", "-t", "^tuple_of_placed_iterators_crosses_part_by_part\\.rs$"],
  },
  {
    name: "placed-tuple-written-as-a-call",
    breaks:
      "a tuple written where it's given to generic code crosses by `$inPlace([\"Green\", undefined], [1, 1])` where its parts are written in their places, `[{ value: \"Green\" }, {}]` (#2255)",
    file: "crates/rust-js-lower/src/generic_facts.rs",
    find: "                if items.len() == parts.len()\n",
    replace: "                if false\n",
    snapshots: true,
    tests: ["test/corpus.test.ts", "-t", "^tuple_of_placed_iterators_crosses_part_by_part\\.rs$"],
  },
  {
    name: "placed-tuple-by-value-refused",
    breaks:
      "a tuple of iterators taken by value, which crosses part by part, is refused as one held otherwise inside what crosses (#2255)",
    file: "crates/rust-js-lower/src/generic_facts.rs",
    find: "            ty::Tuple(_) => None,\n",
    replace: "            ty::Tuple(_) => Some(false),\n",
    tests: ["test/corpus.test.ts", "-t", "^tuple_of_placed_iterators_crosses_part_by_part\\.rs$"],
  },
  {
    name: "placed-tuple-held-not-inherited",
    breaks:
      "a tuple's parts are walked as held as they are whatever holds the tuple, so one taken by value and one an opaque library struct holds are refused (#2255)",
    file: "crates/rust-js-lower/src/generic_facts.rs",
    find: "(d, i, held)).collect(),\n",
    replace: "(d, i, Some(false))).collect(),\n",
    tests: ["test/corpus.test.ts", "-t", "^tuple_of_placed_iterators_crosses_part_by_part\\.rs$"],
  },
  {
    name: "placed-library-struct-walked-by-fields",
    breaks:
      "a library's struct whose fields only its own generic code reads, itertools' `IntoChunks<I>` holding a `RefCell<GroupInner<.., I, ..>>`, is walked by its fields, refusing every `Itertools` entry given a fieldless enum's iterator (#2255)",
    file: "crates/rust-js-lower/src/generic_facts.rs",
    find: "                        && d.is_struct()\n",
    replace: "                        && false\n",
    tests: ["test/crates-pairs-3.test.ts", "-t", "two crates: library_holder_of_a_placed_iterator_built_by_its_generic_code"],
  },
  {
    name: "placed-library-public-field-unrefused",
    breaks:
      "a library's struct with a public field, which its consumer reads by its shape, is walked as one only its own code reads, so `open.inner` reads generic code's place, never refused (#2255)",
    file: "crates/rust-js-lower/src/generic_facts.rs",
    find: "                        && !(d.all_fields()).any(|field| field.vis.is_accessible_from(CRATE_DEF_ID, self.tcx)) =>\n",
    replace: "                        =>\n",
    tests: ["test/crates-iterators.test.ts", "-t", "public field of an iterator its generic code holds in a place"],
  },
  {
    name: "placed-library-arguments-held-otherwise",
    breaks:
      "a library struct's type arguments, which its own generic code holds as generic code holds them, are walked as held as they are, refusing `Chunks<Self>` of a fieldless enum's iterator (#2255)",
    file: "crates/rust-js-lower/src/generic_facts.rs",
    find: "(d, i, None)).collect()\n",
    replace: "(d, i, Some(false))).collect()\n",
    tests: ["test/crates-pairs-3.test.ts", "-t", "two crates: library_holder_of_a_placed_iterator_built_by_its_generic_code"],
  },
  {
    name: "reified-pointer-params-unnamed",
    breaks: "a generic function's `fn` pointer names its parameters `arg0`, not as the function does, `(n) => bumpedPoint($cellOf(n))` (#1328)",
    file: "crates/rust-js-lower/src/generic_facts.rs",
    find: "            .map(|(at, name)| name.unwrap_or_else(|| format!(\"arg{at}\")))\n",
    replace: "            .map(|(at, name)| name.filter(|_| false).unwrap_or_else(|| format!(\"arg{at}\")))\n",
    tests: ["test/corpus.test.ts", "-t", "^generic_fn_value_takes_its_callers_forms.rs$"],
    snapshots: true,
  },
  {
    name: "ref-held-box-unread",
    breaks:
      "a reference generic code holds in its box, `c: &mut C` given `&mut range` or `&mut v`, is given its dictionary as the box itself, which its `into_iter` steps (#2336, ADR 0454)",
    file: "crates/rust-js-lower/src/generic_facts.rs",
    find: "            RefHeld::InBox => box_back(value),\n",
    replace: "            RefHeld::InBox => value,\n",
    tests: ["test/corpus.test.ts", "-t", "^mut_ref_iterator_through_a_reference_bound.rs$"],
  },
  {
    name: "ref-held-as-it-is-taken-for-js",
    breaks:
      "a `T: IntoIterator` given as it is, given `&mut range`, has its dictionary step the range as the JS iterator a type parameter given one is (#2336)",
    file: "crates/rust-js-lower/src/generic_facts.rs",
    find: "            ty::Param(p) if given_as_it_is(p) => RefHeld::AsItIs,\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^mut_ref_iterator_through_a_reference_bound.rs$"],
  },
  {
    name: "named-into-iter-unboxed",
    breaks:
      "an `IntoIterator` of a `&mut Vec` or a `&mut range` reached through `impl Is<I>` is taken to arrive as its own crossing says, not as that one's value: `bump(&mut v)` writes nothing, the range is stepped as a JS iterator (#966, #2336)",
    file: "crates/rust-js-lower/src/generic_facts.rs",
    find: "                self.ref_held_by_a_parameter(instantiated)\n",
    replace: "                unconverted\n",
    tests: ["test/corpus.test.ts", "-t", "^into_iterator_named_by_another_bound.rs$|^mut_ref_iterator_through_a_reference_bound.rs$"],
  },
  {
    name: "into-iter-param-boxed",
    breaks:
      "a `U: IntoIterator` given a `&mut [T]` is given a dictionary that takes the box a `&mut C` is: std's `zip` reads `cell.value` of the array, `undefined is not iterable` (ADR 0976)",
    file: "crates/rust-js-lower/src/generic_facts.rs",
    find: "            ty::Param(p) if given_as_it_is(p) => RefHeld::AsItIs,\n",
    replace: "            ty::Param(p) if given_as_it_is(p) => RefHeld::InBox,\n",
    tests: ["test/iterator-forms.test.ts", "-t", "^zip_into_mut_slice with"],
  },
  {
    name: "iterator-taken-of-a-parent",
    breaks:
      "a function is said to take an iterator by its impl's or its trait's parameters, not its own, so the debug build's check of what std's code reaches of the crate's `extend` passes it (#2376)",
    file: "crates/rust-js-lower/src/generic_facts.rs",
    find: "if param.index >= parents)",
    replace: "if param.index < parents)",
    tests: ["test/lowering-generic-iterators.test.ts", "-t", "unanswered"],
  },
  {
    name: "guard-given-to-a-whole-write-unrefused",
    breaks: "a `RefCell` guard's `&mut` given to generic code's `&mut T`, which may write it whole, isn't refused: `put(&mut g, h)` writes `h`'s cell's contents into `g`'s, in place, and never gives `h`'s borrow back (#2475)",
    file: "crates/rust-js-lower/src/generic_facts.rs",
    find: "            self.refuse_guard_written_whole((declared, caller), others, span)?;\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^refcell_(field_)?guard_given_to_generic_put_refused.rs$"],
  },
  {
    name: "guard-given-alone-refused",
    breaks: "a `RefCell` guard's `&mut` given to generic code given no other guard to write it with is refused, `bump(&mut cell.borrow_mut())` of `fn bump<P: DerefMut>(p: &mut P)`, which writes only what it guards (#2475)",
    file: "crates/rust-js-lower/src/generic_facts.rs",
    find: ".filter(|&(other, _)| other != at)",
    replace: ".filter(|&(other, _)| other != at || other == at)",
    tests: ["test/corpus.test.ts", "-t", "^generic_deref_mut.rs$"],
  },
];
