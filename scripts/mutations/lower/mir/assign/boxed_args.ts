// Mutations of src/lower/mir/assign/boxed_args.rs, what a call is given for each argument (split from assign.rs, #1300).
import type { Mutation } from "../../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "mir-borrowed-result-given-value",
    breaks:
      "a `&mut` whose borrow a call's result holds is given the number, not a handle on its place",
    file: "src/lower/mir/assign/boxed_args.rs",
    find: "                exprs.push(through(Expr::handle(place)));\n                continue;\n",
    replace: "                exprs.push(through(place));\n                continue;\n",
    tests: ["test/corpus.test.ts", "-t", "mut_number_handles"],
  },
  {
    name: "mir-object-unboxed",
    breaks:
      "an object given where a generic `&mut T` goes isn't boxed, so the callee reads `value` of it",
    file: "src/lower/mir/assign/boxed_args.rs",
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
    file: "src/lower/mir/assign/boxed_args.rs",
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
    file: "src/lower/mir/assign/boxed_args.rs",
    find: "!pointee.is_sized(self.tcx, self.typing_env) && !pointee.is_str())",
    replace: "!pointee.is_sized(self.tcx, self.typing_env))",
    tests: ["test/corpus.test.ts", "-t", "str_mut"],
  },
  {
    name: "function-given-boxed",
    breaks:
      "a function given to generic code for its `F: FnMut(&mut T)` of an object `T` is given as it is, and called with generic code's box of the object",
    file: "src/lower/mir/assign/boxed_args.rs",
    find: "                    let unboxed = callee.and_then(|(id, args)| self.unboxed_by_function(id, i, args, given));\n",
    replace:
      "                    let unboxed = callee.and_then(|(id, args)| self.unboxed_by_function(id, i, args, given)).filter(|_| false);\n",
    tests: ["test/corpus.test.ts", "-t", "^function_given_for_a_mut_param.rs$"],
  },
  {
    name: "mut-field-given-refused",
    breaks:
      "`inc(&mut es[0].n)` is refused, its object not named where it's borrowed",
    file: "src/lower/mir/assign/boxed_args.rs",
    find: "                js::ExprKind::Member(..) | js::ExprKind::Index(..) if !fixed_place(&place) && !one_value => {\n",
    replace:
      "                js::ExprKind::Member(..) | js::ExprKind::Index(..) if false => {\n",
    tests: ["test/corpus.test.ts", "-t", "^mut_field_given.rs$"],
  },
  {
    name: "boxed-param-regions-bound",
    breaks:
      "a call's parameter type is read with its signature's lifetimes still bound, so a `&mut &dyn Fn() -> u32` given a variable's box panics asking the `dyn`'s layout: the compiler crashes (#460)",
    file: "src/lower/mir/assign/boxed_args.rs",
    find: "            let param = (self.tcx)\n                .instantiate_bound_regions_with_erased(\n                    self.tcx\n                        .fn_sig(def_id)\n                        .instantiate(self.tcx, generic_args)\n                        .skip_normalization(),\n                )\n                .inputs()[i];\n",
    replace: "            let param = self.tcx.fn_sig(def_id).instantiate(self.tcx, generic_args).skip_binder().inputs()[i];\n",
    tests: ["test/corpus.test.ts", "-t", "^dyn_fn_reference_changed_through_mut\\.rs$"],
  },
  {
    name: "one-value-kept-mut-refused",
    breaks:
      "a `&mut ()` given where the call may keep it, `bad(&mut v).await`'s, is refused as a `&mut` of a number given back, where a box of its one value is all it needs (#1061)",
    file: "src/lower/mir/assign/boxed_args.rs",
    find: "                if one_value && !fixed_place(&place) {\n",
    replace: "                if false && one_value && !fixed_place(&place) {\n",
    tests: ["test/corpus.test.ts", "-t", "^mut_of_one_value_kept.rs$"],
  },
  {
    name: "boxed-function-arrow-carries-no-drop",
    breaks: "a boxed function std's compiled `Scan` holds through an arrow that gives it the state's box is never dropped, the arrow given `$dispose` as its drop carrying none (#1220)",
    file: "src/lower/mir/assign/boxed_args.rs",
    find: "                            match boxed_drop {\n",
    replace: "                            match None::<Expr> {\n",
    tests: ["test/iterator-forms.test.ts", "-t", "boxed_fn_through_a_call_drops"],
  },
  {
    name: "held-mut-boxed-as-held",
    breaks: "a `&mut` to a local `&mut` to an object, given as a generic `&mut W`, is boxed as the local holds it, the object, where generic code reads its in-place cell (#1300)",
    file: "src/lower/mir/assign/boxed_args.rs",
    find: "                self.form_differs(Form::Box, crossing).then_some(crossing)\n",
    replace: "                self.form_differs(Form::Box, crossing).then_some(crossing).filter(|_| false)\n",
    tests: ["test/corpus.test.ts", "-t", "^mut_of_a_held_mut_given_to_generic_code.rs$"],
  },
  {
    name: "held-mut-copied-back-as-a-cell",
    breaks: "what's copied back of a `&mut` to a local `&mut` to an object, given as a generic `&mut W`, is the cell generic code had, not the `&mut` it holds (#1300)",
    file: "src/lower/mir/assign/boxed_args.rs",
    find: "                    Some(crossing) => self.form_back(Form::Box, crossing, back),\n",
    replace: "                    Some(_) => back,\n",
    tests: ["test/corpus.test.ts", "-t", "^mut_of_a_held_mut_given_to_generic_code.rs$"],
  },
];
