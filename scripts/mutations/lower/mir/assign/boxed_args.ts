// Mutations of crates/rust-js-mir/src/assign/boxed_args.rs, what a call is given for each argument (split from assign.rs, #1300).
import type { Mutation } from "../../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "placed-argument-bare",
    breaks:
      "a fieldless enum's iterator given as an argument to generic code that holds it in a place is given bare, so its own `&mut self` step throws (#2255)",
    file: "crates/rust-js-mir/src/assign/boxed_args.rs",
    find: "                if self.form_differs(Form::Placed, crossing) {\n",
    replace: "                if false {\n",
    tests: ["test/corpus.test.ts", "-t", "^non_object_iterator_given_to_generic_code\\.rs$"],
  },
  {
    name: "boxes-not-read-through",
    breaks:
      "a closure given boxes where it takes what's in them is called by an arrow that unboxes them again, `(x0, x1) => ((pair) => { .. })(x0.value, x1)` (#2008)",
    file: "crates/rust-js-mir/src/assign/boxed_args.rs",
    find: "                        && let Some(arrow) = self.read_through_boxes(&f, forms)\n",
    replace: "                        && let Some(arrow) = self.read_through_boxes(&f, forms).filter(|_| false)\n",
    tests: ["test/corpus.test.ts", "-t", "^(scan_state_through_its_box|scan_object_state).rs$"],
    snapshots: true,
  },
  {
    name: "boxes-read-through-of-as-many-only",
    breaks:
      "a closure that takes fewer than generic code gives, `(pair) => ..` of `|pair, _|`, isn't read through its box, but called by the arrow that unboxes it (#2008)",
    file: "crates/rust-js-mir/src/assign/boxed_args.rs",
    find: "        if names.len() > forms.len() {\n",
    replace: "        if names.len() != forms.len() {\n",
    tests: ["test/corpus.test.ts", "-t", "^scan_object_state.rs$"],
    snapshots: true,
  },
  {
    name: "fn-box-called-and-given-back-allowed",
    breaks: "a `&mut` to a boxed function given for an `F` that's called and given back compiles, given as its box, whose call throws `TypeError: f is not a function` (#1909)",
    file: "crates/rust-js-mir/src/assign/boxed_args.rs",
    find: "                && self.called_box_escapes(id, declared, instantiated)\n",
    replace: "                && false && self.called_box_escapes(id, declared, instantiated)\n",
    tests: ["test/corpus.test.ts", "-t", "^fn_box_called_and_given_back_refused.rs$"],
  },
  {
    name: "librarys-mut-kept-from-its-own-dictionary",
    breaks: "`f.write_str(..)` of a library's own `impl fmt::Write` is refused, its `&mut` taken for one std's dictionary would read as a box, where the library's dictionary takes the crate's forms (#1487)",
    file: "crates/rust-js-mir/src/assign/boxed_args.rs",
    find: "        Ok(values.iter().any(|v| matches!(v, Value::Ref(_))) || (!of_dyn && !compiled && given.iter().any(a_mut)))\n",
    replace: "        Ok(values.iter().any(|v| matches!(v, Value::Ref(_))) || (!of_dyn && given.iter().any(a_mut)))\n",
    tests: ["test/crates.test.ts", "-t", "library_own_methods_through_its_dictionary"],
  },
  {
    name: "known-receivers-mut-given-to-stds-dictionary",
    breaks: "a `Hasher`'s `write` of a type known where it's called, its recogniser left out, goes by the trait call generic code makes, to a dictionary that reads its `&mut self` as a box: JS that runs otherwise, where it's refused (#1487)",
    file: "crates/rust-js-mir/src/assign/boxed_args.rs",
    find: "        Ok(values.iter().any(|v| matches!(v, Value::Ref(_))) || (!of_dyn && !compiled && given.iter().any(a_mut)))\n",
    replace: "        Ok(values.iter().any(|v| matches!(v, Value::Ref(_))))\n",
    tests: ["test/tidy-stages.test.ts", "-t", "is refused without its recogniser"],
  },
  {
    name: "mir-borrowed-result-given-value",
    breaks:
      "a `&mut` whose borrow a call's result holds is given the number, not a handle on its place",
    file: "crates/rust-js-mir/src/assign/boxed_args.rs",
    find: "                exprs.push(through(self, Expr::handle(place))?);\n                continue;\n",
    replace: "                exprs.push(through(self, place)?);\n                continue;\n",
    tests: ["test/corpus.test.ts", "-t", "mut_number_handles"],
  },
  {
    name: "mir-object-unboxed",
    breaks:
      "an object given where a generic `&mut T` goes isn't boxed, so the callee reads `value` of it",
    file: "crates/rust-js-mir/src/assign/boxed_args.rs",
    find: "                if self.form_differs(Form::Box, crossing) {\n",
    replace: "                if false && self.form_differs(Form::Box, crossing) {\n",
    tests: [
      "test/corpus.test.ts",
      "-t",
      "generic_traits|mut_ref_as_generic|generic_mut_ref",
    ],
  },
  {
    name: "mir-unit-place-refused",
    breaks:
      "`pass(&mut unit)` of a `()`, and a unit struct's `&mut` in what a call gave, `f()[0].touch()`, are refused as places that aren't fixed, though no write changes one",
    file: "crates/rust-js-mir/src/assign/boxed_args.rs",
    find: "            if !fixed_place(&place) && !one_value {\n",
    replace: "            if !fixed_place(&place) {\n",
    tests: [
      "test/corpus.test.ts",
      "-t",
      "^(mut_ref_as_generic|option_as_mut_generic).rs$",
    ],
  },
  {
    name: "mir-str-box-not-copied-back",
    breaks:
      "MIR's `shout(&mut s)` of a `&mut str` boxes `s` and doesn't copy it back",
    file: "crates/rust-js-mir/src/assign/boxed_args.rs",
    find: "!pointee.is_sized(self.tcx, self.typing_env) && !pointee.is_str())",
    replace: "!pointee.is_sized(self.tcx, self.typing_env))",
    tests: ["test/corpus.test.ts", "-t", "str_mut"],
  },
  {
    name: "function-given-boxed",
    breaks:
      "a function given to generic code for its `F: FnMut(&mut T)` of an object `T` is given as it is, and called with generic code's box of the object",
    file: "crates/rust-js-mir/src/assign/boxed_args.rs",
    find: "                    let mut unboxed = callee.and_then(|(id, args)| self.unboxed_by_function(id, i, args));\n",
    replace:
      "                    let mut unboxed = callee.and_then(|(id, args)| self.unboxed_by_function(id, i, args)).filter(|_| false);\n",
    tests: ["test/corpus.test.ts", "-t", "^function_given_for_a_mut_param.rs$"],
  },
  {
    name: "mut-field-given-refused",
    breaks:
      "`inc(&mut es[0].n)` is refused, its object not named where it's borrowed",
    file: "crates/rust-js-mir/src/assign/boxed_args.rs",
    find: "                js::ExprKind::Member(..) | js::ExprKind::Index(..) if !fixed_place(&place) && !one_value => {\n",
    replace:
      "                js::ExprKind::Member(..) | js::ExprKind::Index(..) if false => {\n",
    tests: ["test/corpus.test.ts", "-t", "^mut_field_given.rs$"],
  },
  {
    name: "boxed-param-regions-bound",
    breaks:
      "a call's parameter type is read with its signature's lifetimes still bound, so a `&mut &dyn Fn() -> u32` given a variable's box panics asking the `dyn`'s layout: the compiler crashes (#460)",
    file: "crates/rust-js-mir/src/assign/boxed_args.rs",
    find: "            let param = (self.tcx)\n                .instantiate_bound_regions_with_erased(\n                    self.tcx\n                        .fn_sig(def_id)\n                        .instantiate(self.tcx, generic_args)\n                        .skip_normalization(),\n                )\n                .inputs()[i];\n",
    replace: "            let param = self.tcx.fn_sig(def_id).instantiate(self.tcx, generic_args).skip_binder().inputs()[i];\n",
    tests: ["test/corpus.test.ts", "-t", "^dyn_fn_reference_changed_through_mut\\.rs$"],
  },
  {
    name: "one-value-kept-mut-refused",
    breaks:
      "a `&mut ()` given where the call may keep it, `bad(&mut v).await`'s, is refused as a `&mut` of a number given back, where a box of its one value is all it needs (#1061)",
    file: "crates/rust-js-mir/src/assign/boxed_args.rs",
    find: "                if one_value && !fixed_place(&place) {\n",
    replace: "                if false && one_value && !fixed_place(&place) {\n",
    tests: ["test/corpus.test.ts", "-t", "^mut_of_one_value_kept.rs$"],
  },
  {
    name: "boxed-function-arrow-carries-no-drop",
    breaks: "a boxed function std's compiled `Scan` holds through an arrow that gives it the state's box is never dropped, the arrow given `$dispose` as its drop carrying none (#1220)",
    file: "crates/rust-js-mir/src/assign/boxed_args.rs",
    find: "                            match boxed_drop {\n",
    replace: "                            match None::<Expr> {\n",
    tests: ["test/iterator-forms.test.ts", "-t", "boxed_fn_through_a_call_drops"],
  },
  {
    name: "held-mut-boxed-as-held",
    breaks: "a `&mut` to a local `&mut` to an object, given as a generic `&mut W`, is boxed as the local holds it, the object, where generic code reads its in-place cell (#1300)",
    file: "crates/rust-js-mir/src/assign/boxed_args.rs",
    find: "                self.form_differs(Form::HeldBox, crossing).then_some(crossing)\n",
    replace: "                self.form_differs(Form::HeldBox, crossing).then_some(crossing).filter(|_| false)\n",
    tests: ["test/corpus.test.ts", "-t", "^mut_of_a_held_mut_given_to_generic_code.rs$"],
  },
  {
    name: "held-mut-copied-back-as-a-cell",
    breaks: "what's copied back of a `&mut` to a local `&mut` to an object, given as a generic `&mut W`, is the cell generic code had, not the `&mut` it holds (#1300)",
    file: "crates/rust-js-mir/src/assign/boxed_args.rs",
    find: "                    Some(crossing) => self.form_back(Form::Box, crossing, back),\n",
    replace: "                    Some(_) => back,\n",
    tests: ["test/corpus.test.ts", "-t", "^mut_of_a_held_mut_given_to_generic_code.rs$"],
  },
  {
    name: "held-mut-value-given-as-it-is",
    breaks:
      "a `&mut` to a `&mut` to an object held as a value, a closure's parameter, is given to generic code as its caller's box, where generic code reads its box of the object's cell (#1303)",
    file: "crates/rust-js-mir/src/assign/boxed_args.rs",
    find: "                if !matches!(value, Value::Ref(_)) && self.form_differs(Form::HeldBox, crossing) {",
    replace: "                if false && self.form_differs(Form::HeldBox, crossing) {",
    tests: ["test/corpus.test.ts", "-t", "^mut_of_a_held_mut_given_back_by_generic_code.rs$"],
  },
  {
    name: "tagged-argument-as-it-is",
    breaks: "a write's result given as an argument where generic code has a `Result` of its type parameters is given as it is (#1408)",
    file: "crates/rust-js-mir/src/assign/boxed_args.rs",
    find: "                if self.form_differs(Form::Tagged, crossing) {\n",
    replace: "                if false {\n",
    tests: ["test/corpus.test.ts", "-t", "^write_result_given_to_generic_code.rs$"],
  },
  {
    name: "valued-callback-throws",
    breaks: "a closure given to std's `Map` struct that gives a write's result throws its error, not an item (#1410)",
    file: "crates/rust-js-mir/src/assign/boxed_args.rs",
    find: "                                    self.fmt_try(vec![StmtKind::Return(Some(called)).at(js::Span::NONE)])\n",
    replace: "                                    called\n",
    tests: ["test/iterator-forms.test.ts", "-t", "^write_result_items with"],
  },
  {
    name: "generic-read-to-string-fresh-untold",
    breaks: "generic code's `read_to_string` of a `String` proved fresh-empty isn't told so, and a crate's reader it's given is refused as it runs (#1297)",
    file: "crates/rust-js-mir/src/assign/boxed_args.rs",
    find: "            exprs.push(Expr::bool(true));\n",
    replace: "",
    tests: [
      "test/corpus.test.ts",
      "-t",
      "^read_of_a_crates_own_reader.rs$"
    ]
  },
  {
    name: "mut-given-pointer-kept-unseen",
    breaks: "a `&mut` given to a function that keeps a raw pointer of it is a box copied back, so the pointer's later write, `*kept = 0`, reaches the box and not the caller's `n`, which prints `4` where Rust's is `u32::MAX` (#1781)",
    file: "crates/rust-js-mir/src/assign/boxed_args.rs",
    find: "            if self.borrow_outlives_call(def_id, i)\n                || pointer_may_keep(\n",
    replace: "            if self.borrow_outlives_call(def_id, i)\n                || false && pointer_may_keep(\n",
    tests: ["test/corpus.test.ts", "-t", "^pointer_kept_of_a_mut_given.rs$"],
  },
  {
    name: "pointer-kept-of-generic-code-unseen",
    breaks: "a raw pointer generic code keeps of a `&mut impl GetMut` it's given, of the `u32` it gets, isn't one to what the call gives it, so its caller's box copied back misses the pointer's write (#1781)",
    file: "crates/rust-js-mir/src/assign/boxed_args.rs",
    find: "                        .instantiate(self.tcx, generic_args)\n                        .skip_normalization(),\n                )\n                .inputs()[i];\n",
    replace: "                        .instantiate_identity()\n                        .skip_normalization(),\n                )\n                .inputs()[i];\n",
    tests: ["test/corpus.test.ts", "-t", "^pointer_kept_of_a_mut_given.rs$"],
  },
];
