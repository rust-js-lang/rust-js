// Mutations of src/lower/mir/nulled.rs (ADR 0093): what's `null` or a
// value already, moved from rvalues.rs's with rd632.
import type { Mutation } from "../../../mutations";

const rd632 = ["test/lowering-nullable-reads.test.ts", "-t", "returns null or a value"];
const corpus632 = ["test/corpus.test.ts", "-t", "^nulled_returns_given_on.rs$"];
const bound632 = ["test/lowering-nullable-reads.test.ts", "-t", "a bind of each parameter"];

export const mutations: Mutation[] = [
  {
    name: "given-on-param-unsaid",
    breaks: "which own nullable parameter is given on isn't said, so it's `null` or a value at that call whatever its own calls give (rd624)",
    file: "src/lower/mir/nulled.rs",
    find: "                .then(|| place.local.as_usize() - 1);\n",
    replace: "                .then(|| place.local.as_usize() - 1)\n                .filter(|_| false);\n",
    tests: ["test/corpus.test.ts", "-t", "^nullable_param_passed_on.rs$"],
  },
  {
    name: "nullable-local-read-unnulled",
    breaks:
      "a nullable thread-local read whole or taken, `KEY.with_borrow(Clone::clone)`, `KEY.take()`, kept in a guard's field is restored `?? null`, as if it may be `undefined` (rd523)",
    file: "src/lower/mir/nulled.rs",
    find: "        if state.facts.quiet_cells().nullable.contains(&block) {\n",
    replace: "        if false {\n",
    tests: ["test/corpus.test.ts", "-t", "^drop_guard_restores_nullable_fields.rs$"],
    snapshots: true,
  },
  {
    name: "nullable-result-not-nulled",
    breaks:
      "a nullable getter's result kept in a `let`, `prevTransition`, is written to a nullable setter `?? null`, as if it may be `undefined` (ADR 0841, amended)",
    file: "src/lower/mir/nulled.rs",
    find: "        crate::lower::calls::gives_null_for_none(self.tcx, def_id)\n            || (",
    replace: "        false\n            || (",
    tests: [
      "test/lowering-nullable.test.ts",
      "-t",
      "result is written to a nullable place",
    ],
  },
  {
    name: "mir-nullable-field-coalesced",
    breaks:
      "a `#[rust_js::nullable]` field moved into another, `p.code`, is `p.code ?? null`",
    file: "src/lower/mir/nulled.rs",
    find: "                Some(field) if bindings::null_where_made(self.tcx, field) => Given::Nulled,\n",
    replace:
      "                Some(field) if false && bindings::null_where_made(self.tcx, field) => Given::Nulled,\n",
    tests: [
      "test/lowering-options.test.ts",
      "-t",
      "a nullable field's None is null",
    ],
  },
  {
    name: "optional-field-read-nulled",
    breaks:
      "an optional nullable field's value, `undefined` where it's absent, is given to a nullable field as it is, not `null` (ADR 0810)",
    file: "src/lower/mir/nulled.rs",
    find: "                Some(field) if bindings::null_where_made(self.tcx, field) => Given::Nulled,\n",
    replace:
      "                Some(field) if bindings::is_nullable(self.tcx, field) => Given::Nulled,\n",
    tests: [
      "test/lowering-nullable.test.ts",
      "-t",
      "an optional nullable field",
    ],
  },
  {
    name: "optional-read-coalesced",
    breaks:
      "an optional nullable field's value given another optional nullable field is `abort.name ?? null`, `null` where upstream's copy is `undefined` (ADR 0810, amended)",
    file: "src/lower/mir/nulled.rs",
    find: "                Some(field) if bindings::is_optional(self.tcx, field) => Given::Optional,\n",
    replace:
      "                Some(field) if bindings::is_optional(self.tcx, field) => Given::Unknown,\n",
    tests: [
      "test/lowering-optional-fields.test.ts",
      "-t",
      "an optional nullable field given another's value",
    ],
  },
  {
    name: "mir-nullable-moved-through-unseen",
    breaks:
      "a field's value moved through a temporary, as MIR moves it, is `?? null` again",
    file: "src/lower/mir/nulled.rs",
    find: "                    Rvalue::Use(operand, _) => self.stored_in(state, operand, seen),\n",
    replace: "                    Rvalue::Use(..) => (Given::Unknown, None),\n",
    tests: [
      "test/lowering-options.test.ts",
      "-t",
      "a nullable field's None is null",
    ],
  },
  {
    name: "mir-nullable-some-coalesced",
    breaks:
      "a `Some(..)` given to a `#[rust_js::nullable]` field is `?? null` too",
    file: "src/lower/mir/nulled.rs",
    find: "                        if fields.len() == 1\n",
    replace: "                        if fields.len() == 0\n",
    tests: [
      "test/lowering-nullable.test.ts",
      "-t",
      "a nullable field is written what's never undefined as it is",
    ],
  },
  {
    name: "mir-null-none-coalesced",
    breaks:
      "a nullable field of an `Option` made `null` where it's `None` is `?? null` again",
    file: "src/lower/mir/nulled.rs",
    find: "        if state.facts.options().null_nones.contains(&place.local) {\n",
    replace: "        if false {\n",
    tests: [
      "test/lowering-options.test.ts",
      "-t",
      "a nullable field of Some or None by a branch",
    ],
  },
  {
    name: "mir-nullable-clone-renulled",
    breaks:
      "a clone of a nullable field given to one from MIR is `m.rule_id ?? null`",
    file: "src/lower/mir/nulled.rs",
    find: "        if itself\n",
    replace: "        if false\n",
    tests: ["test/lowering-options.test.ts", "-t", "nullable field's clone"],
  },
  {
    name: "own-nullable-param-maybe-undefined",
    breaks:
      "a `#[rust_js::nullable(..)]` parameter given on is `nearest ?? null`, though it's `Fiber | null` as declared (ADR 0861, amended)",
    file: "src/lower/mir/nulled.rs",
    find: "        if self.own_nullable_param(state, place) {\n            let param = (place.projection.is_empty() && !self.tcx.is_closure_like(state.body.source.def_id()))\n                .then(|| place.local.as_usize() - 1);\n            return (Given::Nulled, param);\n        }\n",
    replace: "",
    tests: [
      "test/lowering-nullable-reads.test.ts",
      "-t",
      "a nullable parameter, a nullable thread-local",
    ],
  },
  {
    name: "stored-given-of-a-parameter",
    breaks:
      "what a parameter assigned once stores is asked of that assignment alone, though it holds its caller's value too, so `reassigned(undefined, false)` tests `reason !== null` (rd632; `x = x;` is followed once, `seen`, since rustc's nll/issue-50461-used-mut-from-moves overflowed the stack)",
    file: "src/lower/mir/nulled.rs",
    find: "        if local.as_usize() <= state.body.arg_count || seen.contains(&local) {\n",
    replace: "        if seen.contains(&local) {\n",
    tests: rd632,
  },
  {
    name: "cell-new-of-some-unknown",
    breaks:
      "a `Cell::new(Some(reducer))` of a nullable field is taken to maybe be `undefined`, `lastRenderedReducer: reducer ?? null` (ADR 0841, amended)",
    file: "src/lower/mir/nulled.rs",
    find: "            return self.stored_in(state, &arg.node, seen);\n        }\n        // A clone of such,",
    replace:
      "            return (Given::Unknown, None);\n        }\n        // A clone of such,",
    tests: ["test/lowering-nullable-reads.test.ts", "-t", "stored as they are"],
  },
  {
    name: "optional-cell-get-unknown",
    breaks:
      "an optional nullable cell field's `get()` given another is `?? null`, `d._debugThenableState ?? null` (ADR 0810, amended)",
    file: "src/lower/mir/nulled.rs",
    find: "                true => Given::Optional,\n                false if self.krate.constructed.contains(&field.did) => Given::Nulled,\n",
    replace: "                true => Given::Unknown,\n                false if self.krate.constructed.contains(&field.did) => Given::Nulled,\n",
    tests: [
      "test/lowering-optional-fields.test.ts",
      "-t",
      "optional nullable cell field given",
    ],
  },
  {
    name: "identity-map-stored-unknown",
    breaks:
      "a nullable parameter's `map(js::unknown)`, its own value, is stored `?? null`, not as given, `this.target = nativeEventTarget;` (rd94)",
    file: "src/lower/mir/nulled.rs",
    find: "        if known == Some(crate::lower::Std::OptionMap)\n",
    replace:
      "        if known == Some(crate::lower::Std::OptionMap) && false\n",
    tests: ["test/js-constructors.test.ts", "-t", "identity map"],
  },
  {
    name: "shared-ref-given-unknown",
    breaks:
      "a shared reference to a nullable field is read as of what may be `undefined`, `previousInstance = instance ?? null` (ADR 1017)",
    file: "src/lower/mir/nulled.rs",
    find: "                    Rvalue::Ref(_, BorrowKind::Shared, of) => self.stored_in(state, &Operand::Copy(*of), seen),\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^nested_option_held_as_payload.rs$"],
    snapshots: true,
  },
  {
    name: "reborrow-given-unknown",
    breaks:
      "a reborrow of a reference to a nullable field, `&*instance`, is read as of what may be `undefined` (ADR 1017)",
    file: "src/lower/mir/nulled.rs",
    find: "            return self.stored_in(state, &Operand::Copy(Place::from(place.local)), seen);\n",
    replace: "            return (Given::Unknown, None);\n",
    tests: ["test/corpus.test.ts", "-t", "^nested_option_held_as_payload.rs$"],
    snapshots: true,
  },
  {
    name: "constructed-get-unknown",
    breaks: "a nullable field each constructor of its struct sets first, read for another nullable cell, is given `?? null`: `twice.both = response.moduleLoading ?? null` (rd628)",
    file: "src/lower/mir/nulled.rs",
    find: "                false if self.krate.constructed.contains(&field.did) => Given::Nulled,\n",
    replace: "",
    tests: ["test/js-constructors.test.ts", "-t", "a nullable field each constructor sets first"],
  },
  {
    name: "returns-nulled-unfound",
    breaks: "a function of the crate's whose every return is `null` or a value, `getEventTarget`'s `target.parentNode` or `target`, isn't found so, so the nullable parameter its result is given is tested `!= null` (rd632)",
    file: "src/lower/mir/nulled.rs",
    find: "            self.krate.nulled_params.returned_nulled(id);\n",
    replace: "",
    tests: rd632,
  },
  {
    name: "returns-nulled-unasked",
    breaks: "a call of a function found to return `null` or a value isn't taken as giving so, so the nullable parameter it's given is tested `!= null` (rd632)",
    file: "src/lower/mir/nulled.rs",
    find: "                && self.krate.nulled_params.returns_nulled(self.item, def_id))\n",
    replace: "                && false)\n",
    tests: rd632,
  },
  {
    name: "returns-own-param-nulled",
    breaks: "a function that returns its own nullable parameter, given what may be `undefined`, is taken as returning `null` or a value, so `dispatchGivenBack(givenBack(text))` tests `undefined !== null` and reads its `id` (rd632)",
    file: "src/lower/mir/nulled.rs",
    find: "                    self.written_given(state, mir::RETURN_PLACE, write, &mut Vec::new()) == (Given::Nulled, None)\n",
    replace: "                    self.written_given(state, mir::RETURN_PLACE, write, &mut Vec::new()).0 == Given::Nulled\n",
    tests: corpus632,
  },
  {
    name: "writes-nulled-any",
    breaks: "a variable written more than once is `null` or a value where one of its writes is, so `if profiling { reason() } else { None }`'s `undefined` is tested `!== null` (rd632)",
    file: "src/lower/mir/nulled.rs",
    find: "                    .all(|write| self.written_given(state, local, write, &mut seen.clone()) == (Given::Nulled, None))\n",
    replace: "                    .any(|write| self.written_given(state, local, write, &mut seen.clone()) == (Given::Nulled, None))\n",
    tests: corpus632,
  },
  {
    name: "writes-nulled-unfollowed",
    breaks: "a variable written more than once, each write `null` or a value, isn't taken as so, `if profiling { reason() } else { js::null() }`'s, tested `!= null` (rd632)",
    file: "src/lower/mir/nulled.rs",
    find: "            true => (Given::Nulled, None),\n            false => (Given::Unknown, None),\n",
    replace: "            true => (Given::Unknown, None),\n            false => (Given::Unknown, None),\n",
    tests: rd632,
  },
  {
    name: "as-deref-unfollowed",
    breaks: "a shared `as_deref()` of an `Option` that's `null` or a value isn't taken as so, `parentFragmentInstances.as_deref()`'s parameter tested `== null` (rd632)",
    file: "src/lower/mir/nulled.rs",
    find: "        if known == Some(crate::lower::Std::Pointee)\n",
    replace: "        if known == Some(crate::lower::Std::Pointee) && false\n",
    tests: rd632,
  },
  {
    name: "bound-tuple-unnulled",
    breaks: "a `function::bind` of a function with a nullable parameter, each bound, records each item as `null` or a value, what JS gave too, so `given(root, undefined)`'s bound function tests `undefined !== null` (rd632)",
    file: "src/lower/mir/nulled.rs",
    find: "                    (Given::Nulled, param) => (true, param),\n                    _ => (false, None),\n",
    replace: "                    (Given::Nulled, param) => (true, param),\n                    _ => (true, None),\n",
    tests: bound632,
  },
  {
    name: "bound-call-unrecorded",
    breaks: "a `function::bind` of a function with a nullable parameter, each bound, isn't seen as its call, so the parameter is never found `null` or a value, `complete(root, reason)` tested `!= null` (rd632)",
    file: "src/lower/mir/nulled.rs",
    find: "        self.krate.nulled_params.record(caller, f, &nulled, &given_on);\n",
    replace: "",
    tests: bound632,
  },
  {
    name: "alike-as-deref-unsaid",
    breaks: "a `let`'s `as_deref()` given a nullable parameter of the crate's own isn't said to be, so the `let` is never `null` for `None` and `measure(previous)` stays `!= null` (rd632)",
    file: "src/lower/mir/nulled.rs",
    find: "                    reads.push((of, toward.contains(&destination.local)));\n",
    replace: "                    reads.push((of, false));\n",
    tests: ["test/lowering-nullable-reads.test.ts", "-t", "read by a nullable parameter and alike"],
  },
  {
    name: "alike-is-some-unread",
    breaks: "a `let`'s `is_some()`, which reads `null` and `undefined` alike, isn't counted, so the `let` is never `null` for `None` (rd632)",
    file: "src/lower/mir/nulled.rs",
    find: "                Some(Std::IsSome | Std::IsNone) if locals.reads[arg.local] == 1 && lent.contains_key(&arg.local) => {\n",
    replace: "                Some(Std::IsSome | Std::IsNone) if false => {\n",
    tests: ["test/lowering-nullable-reads.test.ts", "-t", "read by a nullable parameter and alike"],
  },
  {
    name: "alike-map-or-unread",
    breaks: "a `let`'s `map_or`, which reads `null` and `undefined` alike, isn't counted, so the `let` is never `null` for `None` (rd632)",
    file: "src/lower/mir/nulled.rs",
    find: "                Some(Std::Comb(crate::lower::combinators::Comb::MapOr)) if option(arg.local) => {\n",
    replace: "                Some(Std::Comb(crate::lower::combinators::Comb::MapOr)) if false => {\n",
    tests: ["test/lowering-nullable-reads.test.ts", "-t", "read by a nullable parameter and alike"],
  },
  {
    name: "cast-of-a-nulled-unfollowed",
    breaks: "a `js::cast` of an `Option` that's `null` or a value isn't taken as so, so hydrateRoot's `options ?? null` is `options != null ? (options ?? null) : null` (rd632)",
    file: "src/lower/mir/nulled.rs",
    find: "            && matches!(bindings::js_form(self.tcx, def_id), bindings::JsForm::This)\n",
    replace: "            && matches!(bindings::js_form(self.tcx, def_id), bindings::JsForm::This) && false\n",
    tests: ["test/lowering-nullable-reads.test.ts", "-t", "a cast of an Option is what it's given"],
  },
  {
    name: "strict-through-a-borrow-unseen",
    breaks:
      "a `RefCell` field's `borrow()` isn't followed to the field, so `clones.as_ref()` is tested `== null` (rd647)",
    file: "src/lower/mir/nulled.rs",
    find: "                    }) => place.projection.is_empty(),\n",
    replace: "                    }) => false,\n",
    tests: ["test/corpus.test.ts", "-t", "^nullable_field_tested_strictly.rs$"],
    snapshots: true,
  },
  {
    name: "strict-through-a-guard-deref-unseen",
    breaks:
      "a guard's `Deref` isn't followed to its `RefCell` field, so `clones.as_ref()` is tested `== null` (rd647)",
    file: "src/lower/mir/nulled.rs",
    find: "                    _ => deref && !place.projection.is_empty(),\n",
    replace: "                    _ => false,\n",
    tests: ["test/corpus.test.ts", "-t", "^nullable_field_tested_strictly.rs$"],
    snapshots: true,
  },
  {
    name: "strict-cell-get-unseen",
    breaks:
      "a nullable `Cell` field's `get()` is tested `!= null` (rd647)",
    file: "src/lower/mir/nulled.rs",
    find: "                Some(Std::Pointee | Std::CellGet)\n",
    replace: "                Some(Std::Pointee)\n",
    tests: ["test/corpus.test.ts", "-t", "^nullable_field_tested_strictly.rs$"],
    snapshots: true,
  },
  {
    name: "caught-ok-unknown",
    breaks: "what `catch_unwind(f)` gives is taken to be maybe `undefined`, so a nullable function returns `activeElement ?? null` (rd16)",
    file: "src/lower/mir/nulled.rs",
    find: "            && let Some(caught) = self.caught_function(state, place.local)\n",
    replace: "            && let Some(caught) = self.caught_function(state, place.local).filter(|_| false)\n",
    tests: ["test/lowering-nullable-reads.test.ts", "-t", "catch_unwind gives"],
  },
  {
    name: "caught-closure-returns-unnoted",
    breaks: "a closure's returns aren't noted, so `catch_unwind(f)` of one gives `f() ?? null` (rd16)",
    file: "src/lower/mir/nulled.rs",
    find: "        if (id != self.item && !self.tcx.is_closure_like(id))\n",
    replace: "        if id != self.item\n",
    tests: ["test/lowering-nullable-reads.test.ts", "-t", "catch_unwind gives"],
  },
  {
    name: "caught-in-assert-unwind-safe-unseen",
    breaks: "a closure `catch_unwind` is given in an `AssertUnwindSafe` isn't seen, so getActiveElement's keeps its `?? null` (rd16)",
    file: "src/lower/mir/nulled.rs",
    find: "            ty::Adt(adt, args) if crate::lower::recognition::is_assert_unwind_safe(self.tcx, adt.did()) => {\n",
    replace: "            ty::Adt(adt, args) if false && crate::lower::recognition::is_assert_unwind_safe(self.tcx, adt.did()) => {\n",
    tests: ["test/lowering-nullable-reads.test.ts", "-t", "catch_unwind gives"],
  },
  {
    name: "caught-by-any-call",
    breaks: "the `Ok` of any call given a function is taken for what that function returns, though `attempt(found)` gives `Ok(maybe())`, `undefined` (rd16)",
    file: "src/lower/mir/nulled.rs",
    find: "        if crate::lower::recognition::unwind_op(self.tcx, def_id)\n            != Some(crate::lower::std_types::unwind::UnwindOp::Catch)\n        {\n",
    replace: "        if false {\n",
    tests: ["test/lowering-nullable-reads.test.ts", "-t", "catch_unwind gives"],
  },
  {
    name: "or-fallback-unchecked",
    breaks: "`a.or(b)` is taken for `null` or a value where `a` is alone, so `found().or(maybe())` gives `undefined` (rd16)",
    file: "src/lower/mir/nulled.rs",
    find: "                crate::lower::combinators::Comb::Or => self.stored_in(state, &fallback.node, seen).0 == Given::Nulled,\n",
    replace: "                crate::lower::combinators::Comb::Or => true,\n",
    tests: ["test/lowering-nullable-reads.test.ts", "-t", "catch_unwind gives"],
  },
  {
    name: "or-else-fallback-unchecked",
    breaks: "`a.or_else(f)` is taken for `null` or a value where `a` is alone, so `found().or_else(maybe)` gives `undefined` (rd16)",
    file: "src/lower/mir/nulled.rs",
    find: "                    ty::Closure(f, _) | ty::FnDef(f, _) => self.returns_null_or_value(*f),\n",
    replace: "                    ty::Closure(..) | ty::FnDef(..) => true,\n",
    tests: ["test/lowering-nullable-reads.test.ts", "-t", "catch_unwind gives"],
  },
  {
    name: "or-else-unknown",
    breaks: "`a.or_else(f)` of two that are each `null` or a value is taken to be maybe `undefined`, so `catch_unwind(|| found().or_else(found))` keeps `?? null` (rd16)",
    file: "src/lower/mir/nulled.rs",
    find: "            comb @ (crate::lower::combinators::Comb::Or | crate::lower::combinators::Comb::OrElse),\n",
    replace: "            comb @ crate::lower::combinators::Comb::Or,\n",
    tests: ["test/lowering-nullable-reads.test.ts", "-t", "catch_unwind gives"],
  },
];
