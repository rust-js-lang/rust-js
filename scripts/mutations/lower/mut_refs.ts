// Mutations of src/lower/mut_refs.rs (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "closure-through-is-unadapted",
    breaks:
      "a closure given to generic code through a type parameter read as another, `f: impl Is<F>` of `F: Fn(*mut T)`, is given generic code's handle where it takes the array",
    file: "src/lower/mut_refs.rs",
    find: "            .find(|&f| fn_bound(f).is_some())?;",
    replace: "            .take(1)\n            .find(|&f| fn_bound(f).is_some())?;",
    tests: ["test/corpus.test.ts", "-t", "^generic_fn_item_takes_generic_forms\\.rs$"],
  },
  {
    name: "closure-given-handle-unadapted",
    breaks:
      "a closure given to generic code as an `F: Fn(*mut T)` is given generic code's handle of a slice's pointer where it takes the array",
    file: "src/lower/mut_refs.rs",
    find: "                self.form_of(crossing).map(|form| (form, crossing))",
    replace: "                self.form_of(crossing).filter(|&form| form == Form::Box).map(|form| (form, crossing))",
    tests: ["test/corpus.test.ts", "-t", "^generic_fn_item_takes_generic_forms\\.rs$"],
  },
  {
    name: "closure-result-handle-unadapted",
    breaks:
      "a closure given to generic code gives back a slice's pointer as the array where generic code reads its handle",
    file: "src/lower/mut_refs.rs",
    find: "        let back = output.and_then(|out| ",
    replace: "        let back = output.filter(|_| false).and_then(|out| ",
    tests: ["test/corpus.test.ts", "-t", "^generic_fn_item_takes_generic_forms\\.rs$"],
  },
  {
    name: "reified-slice-pointer-as-its-value",
    breaks: "a generic function's instance whose raw pointer is a slice's, as a `fn` pointer, is its value, given the array where it takes generic code's handle",
    file: "src/lower/mut_refs.rs",
    find: "            self.form_of((def_id, declared, self.instantiated(declared, args)))\n                .is_some()\n",
    replace: "            self.form_of((def_id, declared, self.instantiated(declared, args)))\n                .is_some_and(|form| form == Form::Box)\n",
    tests: ["test/corpus.test.ts", "-t", "^generic_fn_pointer_takes_its_types_forms\\.rs$"],
  },
  {
    name: "reified-trait-method-refused",
    breaks: "a trait's provided method as a value, `Tally::bump`, is taken as a generic function's instance whose forms differ, though its value is its impl's, in its callers' forms already: a `fn` pointer's const of its own, `bumpTallyFn`",
    file: "src/lower/mut_refs.rs",
    find: "        if self.tcx.trait_of_assoc(def_id).is_some()\n            || !(self.is_rust_fn(def_id)",
    replace: "        if !(self.is_rust_fn(def_id)",
    tests: ["test/corpus.test.ts", "-t", "^mut_method_pointer\\.rs$"],
    snapshots: true,
  },
  {
    name: "pointer-by-value-lets-out",
    breaks: "a raw pointer given by value counts as one a call may store, and `ptr::hash(p, state)`'s hasher, a box this function was given, is refused",
    file: "src/lower/mut_refs.rs",
    find: "            && (raw(output) || (inputs.iter().enumerate()).any(|(j, &ty)| j != i && stores(ty)))",
    replace: "            && (raw(output) || (inputs.iter().enumerate()).any(|(j, &ty)| j != i && raw(ty)))",
    tests: ["test/corpus.test.ts", "-t", "^reference_given_as_its_pointer\\.rs$"],
  },
  {
    name: "given-its-pointer-never",
    breaks:
      "a function that may let a reference out as a raw pointer is given a box of a copy, `ptr_from_mut(&mut n)`'s, and a write through the pointer after the call is lost",
    file: "src/lower/mut_refs.rs",
    find: "        self.lets_pointers_out(fn_id, i) && self.param_is_box(fn_id, i)",
    replace: "        false && self.lets_pointers_out(fn_id, i) && self.param_is_box(fn_id, i)",
    tests: ["test/corpus.test.ts", "-t", "^reference_given_as_its_pointer\\.rs$"],
  },
  {
    name: "lets-pointers-out-of-the-result-alone",
    breaks:
      "a pointer let out through another parameter, `store_in(&mut t, &mut out)`'s, is of a box of a copy",
    file: "src/lower/mut_refs.rs",
    find: "            && (raw(output) || (inputs.iter().enumerate()).any(|(j, &ty)| j != i && stores(ty)))",
    replace: "            && raw(output)",
    tests: ["test/corpus.test.ts", "-t", "^reference_given_as_its_pointer\\.rs$"],
  },
  {
    name: "closure-param-not-given-its-pointer",
    breaks:
      "a closure that lets its `&mut u32` out, `|r: &mut u32| -> *mut u32 { r }`, refuses the pointer its callers give",
    file: "src/lower/mut_refs.rs",
    find: "                true => self.sig_given_its_pointer((&inputs, output), index - first),",
    replace: "                true => false,",
    tests: ["test/corpus.test.ts", "-t", "^reference_given_as_its_pointer\\.rs$"],
  },
  {
    name: "raw-pointer-result-not-given-back",
    breaks:
      "a slice's pointer generic code gives back is its handle, not the array, and `same(p)[0] = 40` writes the handle",
    file: "src/lower/mut_refs.rs",
    find: "            Some(form) => self.form_back(form, crossing, value),\n",
    replace: "            Some(Form::Handle) => value,\n            Some(form) => self.form_back(form, crossing, value),\n",
    tests: ["test/corpus.test.ts", "-t", "^reference_given_as_its_pointer\\.rs$"],
  },
  {
    name: "given-function-box-unboxed",
    breaks:
      "a function given for an `F: FnMut(&mut T)` that takes generic code's box itself, `bumped::<Counter>`, is given as its value, its `fn` pointer adapted again, `(x0) => bumpedCounterFn(x0.value)`, not itself (#975, #1324)",
    file: "src/lower/mut_refs.rs",
    find: "        if !self.reified_takes_generic_forms(def_id, args) {\n            return false;\n        }\n        let sig",
    replace: "        if true {\n            return false;\n        }\n        let sig",
    tests: ["test/corpus.test.ts", "-t", "^generic_fn_value_takes_its_callers_forms\\.rs$"],
    snapshots: true,
  },
  {
    name: "into-iter-self-items-given",
    breaks:
      "`for x in &mut v` steps `v`'s numbers, not a handle on each: `into_iter`'s own `self`, bound by `IntoIterator`, counted as giving the `&mut`s it makes (ADR 0926)",
    file: "src/lower/mut_refs.rs",
    find: "                matches!(input.kind(), ty::Param(param) if !super::recognition::is_trait_self(*param))\n",
    replace: "                matches!(input.kind(), ty::Param(_))\n",
    tests: ["test/corpus.test.ts", "-t", "^(std_mut_items|mut_ref_loop).rs$"],
  },
  {
    name: "into-iterator-items-made",
    breaks:
      "`a.iter().zip(b)` of a `&mut [u32]` is refused, `a &mut u32 from zip used as a value`: the `&mut`s `b`'s `IntoIterator` gives counted as `zip`'s own (ADR 0926)",
    file: "src/lower/mut_refs.rs",
    find: "            let into = taken_into(i).then(|| self.iterable_item(t));\n",
    replace:
      "            let into = taken_into(i).then(|| self.iterable_item(t)).filter(|_| false);\n",
    tests: ["test/corpus.test.ts", "-t", "^zip_into_mut_slice.rs$"],
  },
  {
    name: "into-iterator-items-of-any-argument",
    breaks:
      "`b.iter_mut()` of a `&mut Vec<u32>` steps its numbers, not a handle on each: its receiver's `IntoIterator` items counted as given where `iter_mut` takes it as itself (ADR 0926)",
    file: "src/lower/mut_refs.rs",
    find: "            let into = taken_into(i).then(|| self.iterable_item(t));\n",
    replace:
      "            let into = Some(self.iterable_item(t)).filter(|_| i < usize::MAX);\n",
    tests: ["test/corpus.test.ts", "-t", "^zip_into_mut_slice.rs$"],
  },
  {
    name: "nested-mut-object-accepted",
    breaks:
      "`to_vec(c)` of an object gives a `Vec` of handles, which the caller takes for the objects",
    file: "src/lower/mut_refs.rs",
    find: "            && let Some(here) = self.nested_mut_object(def_id, generic_args)\n",
    replace: "            && let Some(here) = None::<Ty<'tcx>>\n",
    tests: ["test/diagnostics.test.ts", "-t", "generic &mut T"],
  },
  {
    name: "dispatched-nested-mut-unwalked",
    breaks:
      "generic code gives a bound trait's method a `&mut` to an object within what it takes that its entry doesn't adapt, `pick(vec![&mut w])`, through the dictionary, whose entry gives its impl generic code's box: `w` never changes (#682)",
    file: "src/lower/mut_refs.rs",
    find: "                todo.extend(self.dispatched_takes(def_id, generic_args, bound));\n",
    replace: "",
    tests: ["test/diagnostics.test.ts", "-t", "a generic &mut S to an object in a Vec"],
  },
  {
    name: "dispatched-nested-mut-of-own-receiver",
    breaks:
      "a trait method's own `&mut self`, which its impl's entry unboxes, is taken for a `&mut` within what it takes: every generic call of a `&mut self` method of an object's impl is refused (#682)",
    file: "src/lower/mut_refs.rs",
    find: "            for ty in (method_sig.inputs_and_output.iter()).filter_map(|ty| self.past_own(ty, param_env)) {\n",
    replace: "            for ty in method_sig.inputs_and_output.iter() {\n",
    tests: ["test/corpus.test.ts", "-t", "^declared_signatures.rs$"],
  },
  {
    name: "within-option-unadapted",
    breaks:
      "an `Option<&mut Self>` a bound trait's method takes is no shape its entry adapts: generic code's `p.pick(Some(q))` of an object's impl is refused again (#686)",
    file: "src/lower/mut_refs.rs",
    find: "                Within::Some(Box::new(within_shape(tcx, (some, given), (leaf, form), unboxed)))\n",
    replace: "                Within::AsItIs(declared)\n",
    tests: ["test/corpus.test.ts", "-t", "^nested_mut_self_through_a_dictionary.rs$"],
  },
  {
    name: "within-mut-given-generic-box",
    breaks:
      "a dictionary's entry gives an object's impl generic code's box of a `&mut Self` within an `Option` or a tuple, `clicksPick_pick(arg0.value, arg1)`: its writes go to the box (#682, #686)",
    file: "src/lower/mut_refs.rs",
    find: "                false if to_impl => Some(Expr::held(value.clone())),\n",
    replace: "                false if to_impl => None,\n",
    tests: ["test/corpus.test.ts", "-t", "^nested_mut_self_through_a_dictionary.rs$"],
  },
  {
    name: "within-mut-given-back-as-the-object",
    breaks:
      "a dictionary's entry gives generic code the object an impl's `Option<&mut Self>` holds, which generic code reads as a box, `r.value` (#686)",
    file: "src/lower/mut_refs.rs",
    find: "                false => Some(in_place_cell(value.clone())),\n",
    replace: "                false => None,\n",
    tests: ["test/corpus.test.ts", "-t", "^nested_mut_self_through_a_dictionary.rs$"],
  },
  {
    name: "dispatched-adapted-walked",
    breaks:
      "what a dictionary's entry adapts, an `Option<&mut Self>` a bound trait's method takes, is walked for a `&mut` to an object as what it gives as it is: the call is refused (#686)",
    file: "src/lower/mut_refs.rs",
    find: "                        (self.within(part, param_env, self.instantiated(part, generic_args))).given_as_it_is(&mut takes)\n",
    replace: "                        takes.push(part)\n",
    tests: ["test/corpus.test.ts", "-t", "^nested_mut_self_through_a_dictionary.rs$"],
  },
  {
    name: "nested-mut-object-fields-unread",
    breaks:
      "a generic struct's `r: Option<&mut T>` field, built of an object by the caller, is read as a cell",
    file: "src/lower/mut_refs.rs",
    find: "                                None => todo.push(ty),\n",
    replace: "                                None => {}\n",
    tests: ["test/diagnostics.test.ts", "-t", "in a struct"],
  },
  {
    name: "phantom-mut-is-a-mut",
    breaks:
      "anyhow's `Mut<'a, T>` is refused, `a &mut Frame inside a generic function's parameters`: its `PhantomData<&'a mut T>`, which holds no value, taken for a `&mut` generic code has (#516)",
    file: "src/lower/mut_refs.rs",
    find: "                    walk.skip_current_subtree();\n                    continue;\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^phantom_mut_marker.rs$"],
  },
  {
    name: "phantom-mut-skips-any-struct",
    breaks:
      "a generic struct's `r: &mut T` field, built of an object by the caller, is read as a cell: every struct's fields skipped as a `PhantomData`'s are",
    file: "src/lower/mut_refs.rs",
    find: "                    && marker.is_phantom_data()\n",
    replace: "                    && (marker.is_phantom_data() || !marker.is_box())\n",
    tests: ["test/diagnostics.test.ts", "-t", "in a struct"],
  },
  {
    name: "nested-mut-object-bounds-unread",
    breaks:
      "a closure's parameter holding a `&mut` to an object, `F: FnOnce(Vec<&mut T>)`, isn't refused, and the closure is given generic code's boxes",
    file: "src/lower/mut_refs.rs",
    find: "        for (clause, _) in self.tcx.clauses_of(def_id).instantiate_identity(self.tcx) {\n",
    replace:
      "        for (clause, _) in self.tcx.clauses_of(def_id).instantiate_identity(self.tcx).into_iter().filter(|_| false) {\n",
    tests: [
      "test/diagnostics.test.ts",
      "-t",
      "a Vec of generic &mut Ts to objects given to a closure",
    ],
  },
  {
    name: "generic-result-kept-as-cell",
    breaks:
      "`pick(true, &mut c, &mut d).n += 5` of an object `T` writes `n` on the handle, not on `c`",
    file: "src/lower/mut_refs.rs",
    find: "            Some(form) => self.form_back(form, crossing, value),\n",
    replace: "            Some(Form::Box) => value,\n            Some(form) => self.form_back(form, crossing, value),\n",
    tests: ["test/corpus.test.ts", "-t", "generic_mut_ref_kept"],
  },
  {
    name: "map-handles-unsorted",
    breaks:
      "a B-tree's `iter_mut()` and `values_mut()` go in insertion order, not its keys', of a `Map` JS gives in another order (the crate's own are kept in order, ADR 0404, amended)",
    file: "src/lower/mut_refs.rs",
    find: "            Std::Map(MapOp::Iter(_)) if self.is_sorted(receiver) => {",
    replace:
      "            Std::Map(MapOp::Iter(_)) if false && self.is_sorted(receiver) => {",
    tests: [
      "test/lowering-collections.test.ts",
      "-t",
      "a B-tree JS gives is changed through in its keys' order",
    ],
  },
  {
    name: "cow-mut-no-handle",
    breaks: "`to_mut()` of a `Cow<str>` is refused",
    file: "src/lower/mut_refs.rs",
    find: '            Std::Cow(CowOp::ToMut) => "$cowMut",\n',
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^cows"],
  },
  {
    name: "mut-items-box-unseen",
    breaks:
      "a `Box<[T; N]>`'s `&mut` items, `for n in &mut boxed`, aren't a sequence's",
    file: "src/lower/mut_refs.rs",
    find: "        let receiver = receiver.boxed_ty().unwrap_or(receiver);\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "boxed_array_iter"],
  },
  {
    name: "mut-items-pointee-unseen",
    breaks: "a `Box<[T; N]>`'s `into_iter()` through a `&mut` makes no handles",
    file: "src/lower/mut_refs.rs",
    find: '            Std::Same | Std::Pointee if sequence => "$mutItems",\n',
    replace: '            Std::Same if sequence => "$mutItems",\n',
    tests: ["test/corpus.test.ts", "-t", "boxed_array_iter"],
  },
  {
    name: "mir-box-not-copied-back",
    breaks:
      "a box given for a `&mut` number isn't copied back, so the caller never sees what the call wrote",
    file: "src/lower/mut_refs.rs",
    find: "        StmtKind::Assign(place, back).at(at)\n",
    replace: "        let _ = (place, back);\n        StmtKind::Expr(Expr::undefined()).at(at)\n",
    tests: ["test/corpus.test.ts", "-t", "mut_number_args"],
  },
  {
    name: "borrow-kept-by-parameter-boxed",
    breaks:
      "a `&mut` of a local given to a function that keeps it in another parameter is a box, copied back when the call returns, so writes through it after are lost",
    file: "src/lower/mut_refs.rs",
    find: "            .filter(|&(j, _)| j != i)\n",
    replace: "            .filter(|_| false)\n",
    tests: ["test/corpus.test.ts", "-t", "^mut_ref_escapes.rs$"],
  },
  {
    name: "borrow-kept-by-what-it-refers-to",
    breaks:
      "a `&mut` of a reference is taken as kept where what it refers to is, `pick(slot: &mut &'a [i32], other: &'a [i32])`'s, a handle where a box reads as the original",
    file: "src/lower/mut_refs.rs",
    find: "            ty::Ref(region, ..) => vec![region],\n",
    replace: "            ty::Ref(..) => regions(input),\n",
    tests: ["test/corpus.test.ts", "-t", "^mut_ref_to_ref.rs$"],
    snapshots: true,
  },
  {
    name: "nested-param-mut-allowed",
    breaks:
      "a `&mut` to an object given for a type parameter inside an `Option` generic code takes is allowed, though it has a box there and its caller the object",
    file: "src/lower/mut_refs.rs",
    find: "                if matches!(part.kind(), ty::Param(_) | ty::Ref(_, _, Mutability::Mut)) {\n",
    replace: "                if matches!(part.kind(), ty::Ref(_, _, Mutability::Mut)) {\n",
    tests: [
      "test/limitations.test.ts",
      "-t",
      "inside what generic code takes or gives, an .Option. or a field.s part",
    ],
  },
  {
    name: "clause-self-nested",
    breaks:
      "a bound's own type, `D: Total`'s `D`, counts as a value inside what generic code takes, so a `&mut` given for `D` is refused",
    file: "src/lower/mut_refs.rs",
    find: "                todo.extend(walked(bound.def_id(), args));\n                given.push(args.type_at(0));\n",
    replace:
      "                todo.extend(walked(bound.def_id(), args));\n                todo.extend(args.types().take(1));\n",
    tests: ["test/corpus.test.ts", "-t", "^mut_given_as_type_param.rs$"],
  },
  {
    name: "dictionary-result-unread",
    breaks:
      "a call through a dictionary of a `&mut Self::Out` of an object takes the box its entry gives as the object",
    file: "src/lower/mut_refs.rs",
    find: "        if !self.is_rust_fn(def_id) && self.tcx.trait_of_assoc(def_id).is_none() {\n",
    replace: "        if !self.is_rust_fn(def_id) {\n",
    tests: ["test/corpus.test.ts", "-t", "^mut_assoc_through_dictionary.rs$"],
  },
  {
    name: "bound-mut-self-walked",
    breaks:
      "a `for<'a> &'a mut C: IntoIterator` bound's own `&mut C`, which isn't a value, refuses a caller given a `&mut Vec` as one inside generic code's parameters",
    file: "src/lower/mut_refs.rs",
    find: "                ty::Ref(_, inner, Mutability::Mut) => todo.push(inner),\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^into_iter_of_ref_param.rs$"],
  },
  {
    name: "callee-fields-caller-terms",
    breaks:
      "a generic function's fields are read in its caller's terms, whose type parameters it doesn't have, and rustc panics",
    file: "src/lower/mut_refs.rs",
    find: "                            let ty = field_ty(field, args);\n",
    replace: "                            let ty = self.field_ty(field, args);\n",
    tests: ["test/corpus.test.ts", "-t", "^callee_field_own_terms.rs$"],
  },
  {
    name: "function-param-mut-refused",
    breaks:
      "a `&mut T` in a function's parameters through `F: Fn(&mut T)`, of a `T` that's an object here, is refused as a nested `&mut`, though the function given takes it out of the box",
    file: "src/lower/mut_refs.rs",
    find: "                            ty::Ref(_, pointee, Mutability::Mut) => pointee,\n",
    replace:
      "                            ty::Ref(_, pointee, Mutability::Mut) => Ty::new_mut_ref(self.tcx, self.tcx.lifetimes.re_erased, pointee),\n",
    tests: ["test/corpus.test.ts", "-t", "^function_given_for_a_mut_param.rs$"],
  },
  {
    name: "static-result-not-the-borrow",
    breaks:
      "a `&'static mut` result made of a `&'a mut` argument by `transmute` is taken for another place, so the argument's box is copied back before a write through the result, which is lost",
    file: "src/lower/mut_refs.rs",
    find: "        static_to_it\n            || (others.chain([sig.output()])).any(|ty| named(ty) || holds_it(ty))\n",
    replace: "        (others.chain([sig.output()])).any(|ty| named(ty) || holds_it(ty))\n",
    tests: ["test/corpus.test.ts", "-t", "^transparent_transmute.rs$"],
  },
  {
    name: "bound-lifetime-unread",
    breaks: "a borrow whose lifetime only another parameter's bound names, `kept: &mut impl GetMut<Vec<&'a mut u32>>`, is boxed and copied back, so a write through what was kept is lost (#973)",
    file: "src/lower/mut_refs.rs",
    find: "            || bounds.into_iter().any(named)\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^mut_kept_by_what_a_call_gives_back.rs$"],
  },
  {
    name: "type-parameter-given-back-unseen",
    breaks: "a borrow given for a type parameter the result names, `fn same<T>(t: T) -> T`, is boxed and copied back before the result is written through (#973)",
    file: "src/lower/mut_refs.rs",
    find: "            || (others.chain([sig.output()])).any(|ty| named(ty) || holds_it(ty))\n",
    replace: "            || (others.chain([sig.output()])).any(|ty| named(ty))\n",
    tests: ["test/corpus.test.ts", "-t", "^mut_kept_by_what_a_call_gives_back.rs$"],
  },
  {
    name: "static-result-of-a-type-parameter-unseen",
    breaks: "a `&'static mut String` result of a `&'a mut impl GetMut<String>` argument is taken for another place, so the argument's box is copied back before the result is written through (#975)",
    file: "src/lower/mut_refs.rs",
    find: "                Some(erased(pointee)) == lent_to || (unknown_to && mutability.is_mut())\n",
    replace: "                Some(erased(pointee)) == lent_to\n",
    tests: ["test/corpus.test.ts", "-t", "^mut_kept_by_what_a_call_gives_back.rs$"],
  },
  {
    name: "parameter-read-as-a-function-boxed",
    breaks: "a `&mut` to a closure or an iterator given for `impl Is<F>` of `F: FnMut()` or `I: Iterator` is generic code's box, which `f()` calls and `next()` steps: `f is not a function`, `[]` (#992)",
    file: "src/lower/mut_refs.rs",
    find: "            && self.read_as_stepped(self.tcx.param_env(def_id), declared)\n",
    replace: "            && false\n",
    tests: ["test/corpus.test.ts", "-t", "^mut_given_for_a_parameter_read_as_another.rs$"],
  },
  {
    name: "collection-for-an-into-iterator-unboxed",
    breaks: "a `&mut Vec` given for `impl Is<I>` of `I: IntoIterator` is the array, where its dictionary's `into_iter` takes its cell, `cell.value` undefined (#992)",
    file: "src/lower/mut_refs.rs",
    find: "                ty::Ref(_, pointee, Mutability::Mut) if self.is_callable(pointee) || self.implements_iterator(pointee))\n",
    replace: "                ty::Ref(..))\n",
    tests: ["test/corpus.test.ts", "-t", "^mut_given_for_a_parameter_read_as_another.rs$"],
  },
  {
    name: "library-trait-dispatch-unasked",
    breaks:
      "a consumer's call of a library's generic code that gives its trait's method a `Vec<&mut S>` of the consumer's objects isn't refused as the library's own call is, so its impl's entry is given boxes, `{ value: b }`, and writes them, not `b` (the `is_local()` audit)",
    file: "src/lower/mut_refs.rs",
    find: "        if !self.krate.foreign.compiled_by_rust_js(bound.def_id())\n",
    replace: "        if !bound.def_id().is_local()\n",
    tests: ["test/crates.test.ts", "-t", "Vec of &mut objects through generic code"],
  },
  {
    name: "boxed-field-still-refused",
    breaks: "a struct's `&mut T` field given a `&mut` to an object is refused across a generic function, `hold(&mut shape.at)` (#1405)",
    file: "src/lower/mut_refs.rs",
    find: "                                Some(_) => todo.extend(match *ty.kind() {\n",
    replace: "                                Some(_) => todo.extend(match *ty.kind() {\n                                    _ if true => Some(ty),\n",
    tests: ["test/corpus.test.ts", "-t", "^mut_ref_fields_of_a_type_parameter.rs$"],
  },
  {
    name: "crate-type-arguments-walked",
    breaks: "a crate's type's arguments are walked beside its fields, so `ParserI<&mut Parser>`'s holder is refused though its field holds generic code's box (#1405)",
    file: "src/lower/mut_refs.rs",
    find: "                {\n                    walk.skip_current_subtree();\n                    let here = self.instantiated(part, generic_args);\n",
    replace: "                {\n                    let here = self.instantiated(part, generic_args);\n",
    tests: ["test/corpus.test.ts", "-t", "^mut_ref_fields_of_a_type_parameter.rs$"],
  },
  {
    name: "kept-mut-not-its-pointer",
    breaks: "a raw pointer to what a `&mut` a function reborrows for its result refers to, httparse's `deinit_slice_mut`, is refused, though its callers give a handle on its place (#1283)",
    file: "src/lower/mut_refs.rs",
    find: "                        || (self.param_is_box(fn_id, index - first) && self.borrow_outlives_call(fn_id, index - first))\n",
    replace: "                        || false\n",
    tests: ["test/corpus.test.ts", "-t", "^pointer_to_a_kept_mut.rs$"],
  },
  {
    name: "struct-raw-pointer-unseen",
    breaks: "a constructor that keeps its `&mut` as a struct's `*mut T` field, lock_api's guard, is given a box of a copy, refused (#1283)",
    file: "src/lower/mut_refs.rs",
    find: "                ty::Adt(adt, args) if !self.is_std(adt.did()) && !seen.contains(&adt.did()) => {\n",
    replace: "                ty::Adt(adt, args) if false && !seen.contains(&adt.did()) => {\n",
    tests: ["test/corpus.test.ts", "-t", "^pointer_to_a_lent_slot.rs$"],
  },
  {
    name: "phantom-raw-pointer-seen",
    breaks: "a marker's `PhantomData<*const ()>` in a type is taken for a raw pointer, so a JSX component given a reference to it is refused as letting it out (#1283)",
    file: "src/lower/mut_refs.rs",
    find: "                ty::Adt(marker, _) if marker.is_phantom_data() => walk.skip_current_subtree(),\n",
    replace: "",
    tests: ["test/traits.test.ts", "-t", "copied JSX defaults select the implementation module"],
  },
];
