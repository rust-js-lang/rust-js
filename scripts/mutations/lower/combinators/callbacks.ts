// Mutations of crates/rust-js-lower/src/combinators/callbacks.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

const rd632 = ["test/lowering-nullable-reads.test.ts", "-t", "returns null or a value"];

export const mutations: Mutation[] = [
  {
    name: "unrun-function-undropped",
    breaks:
      "a function an `Option`'s or a `Result`'s method leaves unrun, `map`'s of a `None`, holds what it's never dropped",
    file: "crates/rust-js-lower/src/combinators/callbacks.rs",
    find: "            out.push(StmtKind::If(test, dropped, None).at(js_span));\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^unrun_function_drops.rs$"],
  },
  {
    name: "unrun-none-as-some",
    breaks:
      "a function `map` leaves unrun of a `None` is dropped where it ran, of a `Some`, twice",
    file: "crates/rust-js-lower/src/combinators/callbacks.rs",
    find: "                Unused::None => Expr::bin(Op::LooseEq, subject, Expr::null()),\n",
    replace:
      "                Unused::None => Expr::bin(Op::LooseNe, subject, Expr::null()),\n",
    tests: ["test/corpus.test.ts", "-t", "^unrun_function_drops.rs$"],
  },
  {
    name: "unrun-ok-as-err",
    breaks:
      "a function `map_err` leaves unrun of an `Ok` is dropped where it ran, of an `Err`",
    file: "crates/rust-js-lower/src/combinators/callbacks.rs",
    find: "                Unused::Ok => result::is(subject, true),\n",
    replace: "                Unused::Ok => result::is(subject, false),\n",
    tests: ["test/corpus.test.ts", "-t", "^unrun_function_drops.rs$"],
  },
  {
    name: "unrun-param-ungiven",
    breaks:
      "a generic function's `F` it gives `unwrap_or_else` is taken as given no drop, so what it holds is never dropped",
    file: "crates/rust-js-lower/src/combinators/callbacks.rs",
    find: "        if self.drop_query().drops_but_params(ty) == Drops::Nothing\n            && self.given_drop(ty).is_none()\n",
    replace:
      "        if self.drop_query().drops_but_params(ty) == Drops::Nothing\n",
    tests: ["test/corpus.test.ts", "-t", "^unused_value_drops.rs$"],
  },
  {
    name: "then-unrun-where-run",
    breaks: "`then(f)`'s function is dropped where it ran, of `true`, twice",
    file: "crates/rust-js-lower/src/combinators/callbacks.rs",
    find: "                Unused::False => Expr::unary(js::UnaryOp::Not, subject),\n",
    replace: "                Unused::False => subject,\n",
    tests: ["test/corpus.test.ts", "-t", "^unrun_function_drops.rs$"],
  },
  {
    name: "given-function-undropped",
    breaks:
      "a closure `all`, `fold` or `sort_by` is given by value, called by `&mut`, is never dropped, nor what it holds",
    file: "crates/rust-js-lower/src/combinators/callbacks.rs",
    find: "                out.extend(dropped);\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^given_function_drops.rs$"],
  },
  {
    name: "fn-mut-as-unrun",
    breaks:
      "a function called by `&mut` given to std is taken for one its body drops, and left undropped",
    file: "crates/rust-js-lower/src/combinators/callbacks.rs",
    find: "            ty::ClosureKind::Fn | ty::ClosureKind::FnMut => Some(None),\n",
    replace:
      "            ty::ClosureKind::Fn | ty::ClosureKind::FnMut => None,\n",
    tests: ["test/corpus.test.ts", "-t", "^given_function_drops.rs$"],
  },
  {
    name: "held-function-dropped",
    breaks:
      "a function a lazy adapter holds, `map(f)`'s, is dropped as `map` returns, before it's run",
    file: "crates/rust-js-lower/src/combinators/callbacks.rs",
    find: "            if when.is_none() && kept {\n",
    replace: "            if false && when.is_none() && kept {\n",
    tests: [
      "test/diagnostics.test.ts",
      "-t",
      "a function with a destructor from_fn keeps",
    ],
  },
  {
    name: "held-function-kept-refused",
    breaks:
      "a function a lazy adapter keeps, `map(f)`'s, is refused where it's given",
    file: "crates/rust-js-lower/src/combinators/callbacks.rs",
    find: "            if when.is_none() && self.recognition().held_function(output) == Some(ty) {\n",
    replace:
      "            if false && when.is_none() && self.recognition().held_function(output) == Some(ty) {\n",
    tests: ["test/corpus.test.ts", "-t", "^held_function_drops.rs$"],
  },
  {
    name: "closure-of-given-params-ungiven",
    breaks:
      "a closure that took a type parameter's value, given its drop, is taken for one its callers give none",
    file: "crates/rust-js-lower/src/combinators/callbacks.rs",
    find: "            && !self.holds_given_drops(ty)\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^dyn_fn_generic_closure_drops.rs$"],
  },
  {
    name: "unused-value-unhandled",
    breaks:
      "a value std leaves unused, `unwrap_or`'s default, isn't dropped where it's left, and is refused",
    file: "crates/rust-js-lower/src/combinators/callbacks.rs",
    find: "            return unused().filter(|_| self.drops_function(ty));\n",
    replace: "            return None;\n",
    tests: ["test/corpus.test.ts", "-t", "^unused_value_drops.rs$"],
  },
  {
    name: "given-value-drops-unseen",
    breaks:
      "an `Option<U>` a std call leaves unused, of a `U` whose drop is given, is refused",
    file: "crates/rust-js-lower/src/combinators/callbacks.rs",
    find: "            && !self.holds_given_drops(ty)\n",
    replace:
      "            && !(matches!(ty.kind(), ty::Closure(..)) && self.holds_given_drops(ty))\n",
    tests: ["test/corpus.test.ts", "-t", "^and_value_drops.rs$"],
  },
  {
    name: "kept-dropped-anywhere-refused",
    breaks:
      "`Box::new(f)` of a function that drops wherever it's dropped is refused",
    file: "crates/rust-js-lower/src/combinators/callbacks.rs",
    find: "            if when.is_none() && kept && dropped_anywhere {\n",
    replace:
      "            if when.is_none() && kept && false && dropped_anywhere {\n",
    tests: ["test/corpus.test.ts", "-t", "^dyn_fn_drops.rs$"],
  },
  {
    name: "dropped-function-dropped-again",
    breaks:
      "`drop(f)` of a function that holds a value with a destructor drops it twice",
    file: "crates/rust-js-lower/src/combinators/callbacks.rs",
    find: "            Some(crate::recognition::Std::Drop | crate::recognition::Std::Forget)\n",
    replace: "            Some(crate::recognition::Std::Forget)\n",
    tests: ["test/corpus.test.ts", "-t", "^closure_block_value.rs$"],
  },
  {
    name: "boxed-for-js-refused",
    breaks:
      "`Box::new` of a closure handed to JS, which carries no drop, is refused as a function kept by what it's given to (ADR 0873)",
    file: "crates/rust-js-lower/src/combinators/callbacks.rs",
    find: "                ty::Closure(id, _) => facts.carries_its_drop(self.tcx, *id) || facts.handed_to_js(*id),\n",
    replace:
      "                ty::Closure(id, _) => facts.carries_its_drop(self.tcx, *id),\n",
    tests: ["test/corpus.test.ts", "-t", "^dyn_fn_handed_to_js.rs$"],
  },
  {
    name: "scan-kept-function-refused",
    breaks: "a `scan()`'s function with a destructor is refused, though the scan closes to drop it (#760)",
    file: "crates/rust-js-lower/src/combinators/callbacks.rs",
    find: "                && kept\n                && (self.recognition().closed_on_drop(output))\n",
    replace: "                && kept\n                && false && (self.recognition().closed_on_drop(output))\n",
    tests: ["test/corpus.test.ts", "-t", "^scan_drops_its_function_and_state.rs$"],
  },
  {
    name: "boxed-function-kind-unread",
    breaks: "a `Box<dyn FnMut>` given by value to `any`, `all`, `fold` or `for_each` is never dropped, its kind, which its type doesn't say, not read from its callee's `Fn` bound (#1220)",
    file: "crates/rust-js-lower/src/combinators/callbacks.rs",
    find: "        let kind = crate::recognition::function_kind(self.tcx, self.frame.owner, ty).or(called);\n",
    replace: "        let kind = crate::recognition::function_kind(self.tcx, self.frame.owner, ty);\n",
    tests: ["test/corpus.test.ts", "-t", "^boxed_fn_given_to_consumers_drops.rs$"],
  },
  {
    name: "boxed-function-unrun-undropped",
    breaks: "a boxed `FnOnce` that `or_insert_with` leaves unrun, its key there, is never dropped, its kind, which its type doesn't say, not taken as its taker's (#1220)",
    file: "crates/rust-js-lower/src/combinators/callbacks.rs",
    find: "        (kind == Some(ty::ClosureKind::FnOnce) || kind.is_none() && self.boxed_function_kind(ty).is_some())\n",
    replace: "        (kind == Some(ty::ClosureKind::FnOnce))\n",
    tests: ["test/corpus.test.ts", "-t", "^boxed_fn_through_a_call_drops.rs$"],
  },
  {
    name: "boxed-fn-called-as-its-taker-calls",
    breaks: "a `Box<dyn Fn()>` given to `catch_unwind`, whose `F: FnOnce` calls it once, is taken to drop itself as its call ends, which a `dyn Fn`'s call doesn't, so it's never dropped (#1220)",
    file: "crates/rust-js-lower/src/combinators/callbacks.rs",
    find: "            self.boxed_function_kind(tys[at])\n",
    replace: "            crate::recognition::function_kind(self.tcx, callee, *inputs.get(at)?)\n",
    tests: ["test/corpus.test.ts", "-t", "^boxed_fn_through_a_call_drops.rs$"],
  },
  {
    name: "closure-returned-callee-unknown",
    breaks: "the call a closure gives back isn't known, so `and_then(|labels| labels.item(0))` of a binding that types its result `T | null` gives `null` where what it's of is `None`, not JS's `labels?.item(0)` (#2112 C S5)",
    file: "crates/rust-js-lower/src/combinators/callbacks.rs",
    find: "        let mut at = mir::RETURN_PLACE;\n",
    replace: "        let mut at = mir::RETURN_PLACE;\n        if !body.basic_blocks.is_empty() {\n            return None;\n        }\n",
    tests: ["test/snapshots.test.ts", "-t", "^web_forms: the generated JS is its snapshot$"],
    snapshots: true,
  },
  {
    name: "nullable-result-not-nulled",
    breaks:
      "a nullable getter's result kept in a `let`, `prevTransition`, is written to a nullable setter `?? null`, as if it may be `undefined` (ADR 0841, amended)",
    file: "crates/rust-js-lower/src/combinators/callbacks.rs",
    find: "        crate::calls::gives_null_for_none(self.tcx, def_id)\n            || (",
    replace: "        false\n            || (",
    tests: [
      "test/lowering-nullable.test.ts",
      "-t",
      "result is written to a nullable place",
    ],
  },
  {
    name: "returns-nulled-unasked",
    breaks: "a call of a function found to return `null` or a value isn't taken as giving so, so the nullable parameter it's given is tested `!= null` (rd632)",
    file: "crates/rust-js-lower/src/combinators/callbacks.rs",
    find: "                && (self.settled.nulled).returns_nulled(&mut self.nulled_seen.borrow_mut(), self.item, def_id))\n",
    replace: "                && false)\n",
    tests: rd632,
  },
  {
    name: "found-index-never-index-of",
    breaks:
      "a `position` by `===` is `children.findIndex((c) => c === child)`, not `children.indexOf(child)` as a person writes it",
    file: "crates/rust-js-lower/src/combinators/callbacks.rs",
    find: "            Some(x) => Expr::builtin(items, \"indexOf\", vec![x]),\n",
    replace: "            Some(_) => Expr::call(Expr::member(items, \"findIndex\"), vec![f]),\n",
    tests: ["test/corpus.test.ts", "-t", "^position_index_of.rs$"],
    snapshots: true,
  },
];
