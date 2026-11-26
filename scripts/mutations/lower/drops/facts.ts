// Mutations of crates/rust-js-lower/src/drops/facts.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "count-every-cell-keeps-momentary",
    breaks: "`--count-every-cell` keeps a borrow checked only where it's made, never counted (#2491)",
    file: "crates/rust-js-lower/src/drops/facts.rs",
    find: "    if crate::session::caches(cx.recognition.tcx).count_every_cell {\n",
    replace: "    if false {\n",
    tests: ["test/tidy-stages.test.ts", "-t", "with every cell counted"],
    env: { TIDY_VARIANTS: "cells" },
  },
  {
    name: "drop-let-value-context",
    breaks:
      "a `let`'s value is taken as its block's, so a closure that holds a value with a destructor, made for a `let`, is refused",
    file: "crates/rust-js-lower/src/drops/facts.rs",
    find: "            if self.lets.contains_key(&child) {\n                return (None, child);\n            }\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "closure_drops"],
  },
  {
    name: "closure-made-anywhere",
    breaks:
      "a closure that holds a value with a destructor is made where its drop can't see what it took",
    file: "crates/rust-js-lower/src/drops/facts.rs",
    find: "                    if !placed && !carried && !once {",
    replace: "                    if false && !placed && !carried && !once {",
    tests: ["test/traits.test.ts", "-t", "closure made in a struct"],
  },
  {
    name: "fn-closure-given-away",
    breaks:
      "an `Fn` closure that holds a value with a destructor is given to a call that consumes it, and never drops it",
    file: "crates/rust-js-lower/src/drops/facts.rs",
    find: "        matches!(ty.kind(), ty::Closure(_, args) if OnceDrop::of_closure(args) == OnceDrop::Whole)",
    replace: "        false",
    tests: [
      "test/bindings.test.ts",
      "-t",
      "listener holding a value with a destructor",
    ],
  },
  {
    name: "closure-assigned",
    breaks:
      "a closure that holds a value with a destructor is assigned where its drop can't see what it took",
    file: "crates/rust-js-lower/src/drops/facts.rs",
    find: '                self.problem(expr.span, "assigning a closure that holds a value with a destructor");',
    replace: "                let _ = expr;",
    tests: ["test/traits.test.ts", "-t", "closure assigned"],
  },
  {
    name: "std-given-closure-refused",
    breaks:
      "a closure called by `&` given by value to std, which drops it as it returns, is refused",
    file: "crates/rust-js-lower/src/drops/facts.rs",
    find: "                || crate::recognition::is_std_crate(tcx, def_id)\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^given_function_drops.rs$"],
  },
  {
    name: "returned-closure-made-refused",
    breaks:
      "a closure its maker gives back, holding a value with a destructor, is refused where it's made",
    file: "crates/rust-js-lower/src/drops/facts.rs",
    find: "                    if !placed && !carried && !once {\n",
    replace: "                    if !placed && !once {\n",
    tests: ["test/corpus.test.ts", "-t", "^returned_closure_drops.rs$"],
  },
  {
    name: "transparent-made-here",
    breaks:
      "a closure that holds a value with a destructor, in an `AssertUnwindSafe` given to `catch_unwind`, is refused as made where it's not kept",
    file: "crates/rust-js-lower/src/drops/facts.rs",
    find: "                ExprKind::Adt(..) if self.cx.recognition.transparent(self.thir[expr].ty).is_some() => child = expr,\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^catch_unwind_assert_drops.rs$"],
  },
  {
    name: "closure-block-value-refused",
    breaks:
      "a closure holding an `Rc` made as a block's value for a `let` is refused",
    file: "crates/rust-js-lower/src/drops/facts.rs",
    find: "            Some((block, ExprKind::Block { block: b })) if self.thir[*b].expr == Some(child) => self.placed(block),\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^closure_block_value.rs$"],
  },
  {
    name: "closure-block-given-to-a-call-refused",
    breaks:
      "a closure holding a `D` made as a block's value given straight to a call, `fold(0, { let g = D(90); move |a, x| .. })`, is refused, a block placed only for a `let` (#759)",
    file: "crates/rust-js-lower/src/drops/facts.rs",
    find: "            Some((block, ExprKind::Block { block: b })) if self.thir[*b].expr == Some(child) => self.placed(block),\n",
    replace:
      "            Some((block, ExprKind::Block { block: b })) if self.thir[*b].expr == Some(child) => {\n                let (parent, child) = self.context_at(block);\n                parent.is_none() && self.lets.contains_key(&child)\n            }\n",
    tests: ["test/corpus.test.ts", "-t", "^closure_block_given_to_a_call.rs$"],
  },
  {
    name: "called-once-closure-refused",
    breaks:
      "a closure rustc infers `Fn` that holds a value with a destructor, given where it's made to a binding that calls it once, is refused as one given away without being called once (ADR 1020)",
    file: "crates/rust-js-lower/src/drops/facts.rs",
    find: "                            && !matches!(*ty.kind(), ty::Closure(id, _)\n                                if self.cx.closure_facts.body_is_its_call_once(id, true))\n",
    replace: "",
    tests: [
      "test/corpus.test.ts",
      "-t",
      "^called_once_closure_drops_after.rs$",
    ],
  },
  {
    name: "replaced-with-never-momentary",
    breaks:
      "a `replace_with` whose function can't ask of its borrow is never momentary, so a flattened state's is refused as borrowed where another borrow may meet it (#687)",
    file: "crates/rust-js-lower/src/drops/facts.rs",
    find: "        self.replaced_with(id);\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^flattened_state_replaced_with.rs$"],
  },
  {
    name: "instance-given-closures-another-crates",
    breaks:
      "a library's function the crate compiles itself, given closures that hold a value with a destructor, is taken for another crate's JS, and the call is refused (#1051)",
    file: "crates/rust-js-lower/src/drops/facts.rs",
    find: "                || self.cx.recognition.foreign.is_instance(def_id))\n",
    replace: "                || false && self.cx.recognition.foreign.is_instance(def_id))\n",
    tests: ["test/crates-pairs-1.test.ts", "-t", "two crates: closure_made_by_an_inlined_function"],
  },
  {
    name: "returned-once-closure-refused",
    breaks: "an effect's cleanup that holds a value with a destructor, `move || drop(listening)`, is refused as a closure made where JS sees nothing of it, though React calls it once (#1881)",
    file: "crates/rust-js-lower/src/drops/facts.rs",
    find: "                    if !placed && !carried && !once {\n",
    replace: "                    if !placed && !carried && !false {\n",
    tests: ["test/effect-cleanup-drops.test.ts", "-t", "drops what it holds as React calls it"],
  },
  {
    name: "closure-moved-into-a-once-type-unchecked",
    breaks: "a closure that holds a value with a destructor, called by `&` through a struct and then moved with it to a function that calls it by `FnOnce`, compiles, its captures never dropped (#2031, #2600)",
    file: "crates/rust-js-lower/src/drops/facts.rs",
    find: "                        && self.cx.closure_facts.called_by_value(closure)\n",
    replace: "                        && false && self.cx.closure_facts.called_by_value(closure)\n",
    tests: ["test/corpus.test.ts", "-t", "^fn_closure_in_a_struct_called_by_ref_then_by_value_refused.rs$"],
  },
  {
    name: "held-fn-dyn-unrefused",
    breaks: "a closure made a `dyn Fn` that a call by value calls where it's held in what's given, an `Option<F>` of an `F: FnOnce`, is given as it is, so what it holds is never dropped (#2050)",
    file: "crates/rust-js-lower/src/drops/facts.rs",
    find: "                        && self.cx.closure_facts.held_fn_dyn(c.closure_id.to_def_id())\n",
    replace: "                        && false && self.cx.closure_facts.held_fn_dyn(c.closure_id.to_def_id())\n",
    tests: ["test/corpus.test.ts", "-t", "^boxed_fn_in_an_option_called_once_refused.rs$"],
  },
  {
    name: "guard-replaced-in-a-closure-unrefused",
    breaks: "a `RefCell` guard a closure captures by reference and writes whole, `|| state = other`, compiles, and the other cell's guard is never released, its next borrow panicking where Rust's runs (#2475 1b)",
    file: "crates/rust-js-lower/src/drops/facts.rs",
    find: ".any(|captured| captured.get_root_variable() == var_hir_id.0 && captured.is_by_ref())",
    replace: ".any(|captured| captured.get_root_variable() == var_hir_id.0 && captured.is_by_ref() && false)",
    tests: ["test/corpus.test.ts", "-t", "^(guard_replaced_by_a_closure_refused|flattened_guard_replaced_by_a_closure_refused)\\.rs$"],
  },
  {
    name: "cell-form-flip-unasked",
    breaks: "a `RefCell` borrow the proof proves quiet is counted, as if no proof were asked (#2490's flip)",
    file: "crates/rust-js-lower/src/drops/facts.rs",
    find: "        if !forms.counted(body_owner, thir[fun].span) {\n",
    replace: "        if false && !forms.counted(body_owner, thir[fun].span) {\n",
    tests: ["test/corpus.test.ts", "-t", "^subscriptions.rs$"],
    snapshots: true,
  },
  {
    name: "cell-form-seen-unasked",
    breaks: "every borrow is taken as held only while code rust-js sees runs, so one held while a library's code runs, which borrows its cell unchecked, isn't refused (ADR 0863)",
    file: "crates/rust-js-lower/src/drops/facts.rs",
    find: "        if forms.seen(body_owner, thir[fun].span) {\n",
    replace: "        if true {\n",
    tests: ["test/crates.test.ts", "-t", "a library's RefCells are the values"],
  },
];
