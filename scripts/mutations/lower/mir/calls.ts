// Mutations of src/lower/mir/calls.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

const given1840 = ["test/none-given.test.ts", "-t", "in Bun and in Chromium"];

export const mutations: Mutation[] = [
  {
    name: "closure-call-of-a-write-error-uncaught",
    breaks: "a call of a closure that may give back a write's error isn't caught, `$writeTry`, so `{:?}` of it throws (#1042)",
    file: "src/lower/mir/calls.rs",
    find: "                if writes.is_none() && self.is_write_result(output) && self.closure_may_fail(arg_tys[0]) =>\n",
    replace: "                if false =>\n",
    tests: ["test/corpus.test.ts", "-t", "^closure_giving_back_a_write_error.rs$"],
  },
  {
    name: "closure-arguments-unplaced",
    breaks:
      "a `dyn Fn` call's arguments have no place, so a comment before the first is after it, `d.two(href, /* two */ n)`, or, of one argument, before the statement (rd657)",
    file: "src/lower/mir/calls.rs",
    find: "        let written = self.closure_arguments_written(span);\n",
    replace: "        let written: Vec<Span> = (span, Vec::new()).1;\n",
    tests: ["test/lowering-comments-inside.test.ts", "-t", "a comment before a dyn Fn call's argument is before it"],
  },
  {
    name: "bool-assign-by-ref-refused",
    breaks:
      "a `bool`'s `op=` given a `&bool` from MIR, `*s ^= c`, is refused (#319)",
    file: "src/lower/mir/calls.rs",
    find: "            } else if crate::lower::recognition::has_number_arithmetic(ty) || ty.is_bool() {\n",
    replace:
      "            } else if crate::lower::recognition::has_number_arithmetic(ty) {\n",
    tests: ["test/corpus.test.ts", "-t", "^bool_ops_by_ref.rs$"],
  },
  {
    name: "held-call-named-at-call",
    breaks:
      "a binding's call is asked where it's read by the call's span, not its path's, so one named through the module that holds it is read in place",
    file: "src/lower/mir/calls.rs",
    find: "            false => (self.thir_call(def_id, span)).map_or(span, |(fun, _)| self.frame.thir[fun].span),\n",
    replace:
      "            false => (self.thir_call(def_id, span)).map_or(span, |_| span),\n",
    tests: ["test/lowering-bindings.test.ts", "-t", "named through the module"],
  },
  {
    name: "mir-binding-component-called",
    breaks:
      "a component a JS module exports is its binding's value, not the tag JSX imports",
    file: "src/lower/mir/calls.rs",
    find: "                        ty::FnDef(id, _) => cx.binding_component_of(id),\n",
    replace: "                        ty::FnDef(..) => None,\n",
    tests: [
      "test/jsx-children.test.ts",
      "test/jsx-evaluation-order.test.ts",
      "test/jsx-events.test.ts",
      "test/jsx-props.test.ts",
      "test/jsx-react-dev.test.ts",
      "test/jsx-types.test.ts",
      "test/jsx.test.ts",
    ],
  },
  {
    name: "mir-generic-result-boxed",
    breaks:
      "MIR's `pick(..).n += 5`, a generic `&mut T` of an object, reads the box's `n`",
    file: "src/lower/mir/calls.rs",
    find: "        let called = self.generic_result_values((def_id, generic_args), called, span)?;\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "generic_mut_ref_kept"],
  },
  {
    name: "mir-closure-tuple-refused",
    breaks:
      "MIR's `f(x)` of a generic `F: Fn`, its arguments a tuple made before, is refused",
    file: "src/lower/mir/calls.rs",
    find: "                if tuple.reads_same()\n",
    replace: "                if false && tuple.reads_same()\n",
    tests: ["test/corpus.test.ts", "-t", "generic_trait_methods"],
  },
  {
    name: "mir-assign-shift-amount-bigint",
    breaks: "`c <<= &3i64` of a `u32` shifts a number by a BigInt",
    file: "src/lower/mir/calls.rs",
    find: "            let rhs = crate::lower::std_types::number::shift_amount_of(op, rhs, ty, arg_tys[1]);\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "generic_bitwise_operators"],
  },
  {
    name: "own-assign-through-place-refused",
    breaks:
      "`a.0 += b` of the crate's own type through a place is refused, not its impl's",
    file: "src/lower/mir/calls.rs",
    find: "            && (self.is_lang_adt(ty, LangItem::String)\n                || crate::lower::representation::Num::of(ty).is_some()\n                || ty.is_bool())\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^generic_compound_assign.rs$"],
  },
  {
    name: "duration-assign-number",
    breaks:
      "a `Duration`'s `-=` is its number's `-`, which doesn't panic past zero as std's does",
    file: "src/lower/mir/calls.rs",
    find: "            } else if crate::lower::recognition::has_number_arithmetic(ty) || ty.is_bool() {",
    replace:
      "            } else if crate::lower::representation::Num::of(ty).is_some() || ty.is_bool() {",
    tests: ["test/corpus.test.ts", "-t", "^duration_sub_assign_panic.rs$"],
  },
  {
    name: "writer-callable-given-formatter",
    breaks:
      "a writer callable is given the `Formatter`'s text, which it doesn't take",
    file: "src/lower/mir/calls.rs",
    find: "        let given = |i: &usize| Some(*i) != writes;\n",
    replace: "        let given = |_: &usize| true;\n",
    tests: ["test/corpus.test.ts", "-t", "^writer_callables.rs$"],
  },
  {
    name: "writer-callable-no-options",
    breaks:
      "a writer callable isn't given the `Formatter`'s options: a closure's `f.pad(w)` pads nothing",
    file: "src/lower/mir/calls.rs",
    find: "        if writes.is_some() {\n            exprs.extend(self.given_to_writer(None));\n        }\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^writer_callables.rs$"],
  },
  {
    name: "assign-impl-on-number-skipped",
    breaks:
      "the crate's `impl AddAssign<Cents> for i32` is skipped, `t += Cents(5)` the number's `+` of the newtype's array",
    file: "src/lower/mir/calls.rs",
    find: "            && !self.resolves_to_crate(def_id, generic_args)?\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^assign_impl_on_number.rs$"],
  },
  {
    name: "registered-read-unchecked",
    breaks:
      "a value read as the registry's symbol whose `Some` the program tells is the value itself, so a string is its `Some`",
    file: "src/lower/mir/calls.rs",
    find: "                match read.is_some_and(|read| self.frame.facts.registered_reads.contains(&read)) {\n",
    replace: "                match true {\n",
    tests: [
      "test/lowering-symbols.test.ts",
      "-t",
      "a read of one it tells is checked",
    ],
  },
  {
    name: "registered-read-checked",
    breaks:
      "a value read as the registry's symbol only tested against them is checked, `switch ($symbolFor(type))`, not the value",
    file: "src/lower/mir/calls.rs",
    find: "                match read.is_some_and(|read| self.frame.facts.registered_reads.contains(&read)) {\n",
    replace: "                match false {\n",
    tests: [
      "test/lowering-symbols.test.ts",
      "-t",
      "a match of the registry's symbols",
    ],
  },
  {
    name: "throwing-fn-called-plain",
    breaks:
      "a throwing function's call is its value, `testStringCoercion(value).TAG`, not a throwing call",
    file: "src/lower/mir/calls.rs",
    find: "        Ok(Value::Expr(self.catching(def_id, generic_args, called)))\n",
    replace: "        Ok(Value::Expr(called))\n",
    tests: ["test/throwing-calls.test.ts", "-t", "is a throwing function"],
  },
  {
    name: "rest-variable-called-as-js",
    breaks:
      "a variable of a function whose last parameter is `#[rust_js::rest]` is called as the variable, the arrow that spreads it, `named([x])`, where the function reads as written, `total(x)`",
    file: "src/lower/mir/calls.rs",
    find: "            && !self.takes_rest(def_id)\n",
    replace: "",
    tests: [
      "test/rest-parameters.test.ts",
      "-t",
      "rest slice is JS's rest parameter",
    ],
  },
  {
    name: "rest-value-closure-call-unspread",
    breaks:
      "a call of a closure or a `dyn Fn` of rest arguments as a value gives them as one array, `count([1, 2])`",
    file: "src/lower/mir/calls.rs",
    find: "                .is_some_and(|&ty| crate::lower::bindings::is_rest_value(self.tcx, ty))\n",
    replace:
      "                .is_some_and(|&ty| crate::lower::bindings::is_rest_value(self.tcx, ty) && false)\n",
    tests: [
      "test/rest-parameters.test.ts",
      "-t",
      "spreads them, through any type",
    ],
  },
  {
    name: "symbol-for-held-not-called",
    breaks:
      "`js::SymbolFor` a variable holds is called as `Symbol.for(..)`, not as the variable, React's `symbolFor('selector.component')` (ADR 0964)",
    file: "src/lower/mir/calls.rs",
    find: "        if crate::lower::recognition::is_symbol_for_constructor(self.tcx, def_id) {\n            return true;\n        }\n",
    replace: "",
    tests: [
      "test/lowering-is-function.test.ts",
      "-t",
      "js::SymbolFor is Symbol.for",
    ],
  },
  {
    name: "object-binding-spread-a-field",
    breaks:
      '`js::object::spread(obj)` is `{ "...": obj }`, an object of one field, not `{ ...obj }` (ADR 0967)',
    file: "src/lower/mir/calls.rs",
    find: '                        "..." => self.spread_into(value, ty, span, out, &mut fields)?,\n',
    replace:
      '                        "..." => fields.push(Prop::Field(key, self.value_expr(value, span)?)),\n',
    tests: ["test/js-any-values.test.ts", "-t", "an object spread, a splice"],
  },
  {
    name: "object-spread-tuple-whole",
    breaks:
      "`js::object::spread((a, b))` is `{ ...[a, b] }`, an object of the tuple's indexes, not `{ ...a, ...b }` (ADR 0967, amended)",
    file: "src/lower/mir/calls.rs",
    find: "        let ty::Tuple(tys) = *ty.kind() else {\n",
    replace:
      "        let (true, ty::Tuple(tys)) = (false, *ty.kind()) else {\n",
    tests: [
      "test/js-collections.test.ts",
      "-t",
      "a Dict spread before more of it",
    ],
  },
  {
    name: "object-spread-parts-made-in-place",
    breaks:
      "`js::object::spread((base, changed(base)))` spreads `base` before `changed(base)` sets it, where Rust makes the tuple first (ADR 0967, amended)",
    file: "src/lower/mir/calls.rs",
    find: "        if parts.iter().skip(1).any(runs_code) {\n",
    replace: "        if false && parts.iter().skip(1).any(runs_code) {\n",
    tests: [
      "test/js-collections.test.ts",
      "-t",
      "a Dict spread before more of it",
    ],
  },
  {
    name: "object-spread-written-out-held",
    breaks:
      "React's `UIEventInterface` is `const spread = { view: 0, detail: 0 }; const UIEventInterface = { ...EventInterface, ...spread };`, though a `Dict` written out runs none of the program's code (ADR 0967, amended)",
    file: "src/lower/mir/calls.rs",
    find: "        let runs_code = |part: &Expr| part.has_effects() && !part.facts.has(js::Known::RunsNothing);\n",
    replace: "        let runs_code = |part: &Expr| part.has_effects();\n",
    tests: [
      "test/js-collections.test.ts",
      "-t",
      "a Dict spread before more of it",
    ],
  },
  {
    name: "crate-once-callback-kept",
    breaks:
      "React's runWithFiberInDEV(fiber, commitCallbacks, ..) is given `(queue, instance) => commitCallbacks(queue, instance)`, a closure of the crate's own function's `FnOnce` kept",
    file: "src/lower/mir/calls.rs",
    find: "                    && let Some(callee) = crate::lower::passed_on::passed_on(expr, taker, self.forwarding())\n",
    replace:
      "                    && let Some(callee) = crate::lower::passed_on::passed_on(expr, taker, self.forwarding()).filter(|_| false)\n",
    tests: ["test/lowering-passed-on.test.ts", "-t", "crate's FnOnce"],
  },
  {
    name: "js-given-call-unrouted",
    breaks:
      "a call of a function JS calls calls its own JS whatever it gives, so `relay(())`, which needs the box, is `None`",
    file: "src/lower/mir/calls.rs",
    find: "        let copy = self.called_copy((def_id, generic_args), span)?;",
    replace:
      "        let copy = self.called_copy((def_id, generic_args), span).map(|_| None)?;",
    tests: ["test/corpus.test.ts", "-t", "js_given_copies"],
  },
  {
    name: "add-assign-text-unjoined",
    breaks:
      '`s += &js::concat_text(n)` of a `String` is `s += "" + n`, its `String` not known text (ADR 0639, amended)',
    file: "src/lower/mir/calls.rs",
    find: "                Expr::text_joined(place.clone(), rhs)\n",
    replace: "                Expr::bin(Op::Add, place.clone(), rhs)\n",
    tests: ["test/bindings-any-values.test.ts", "-t", "any value is read as JS reads it"],
  },
  {
    name: "mir-assign-op-call-place-twice",
    breaks:
      "`*first(&mut v) += &five` is `first(v).value = (first(v).value + five) | 0`: the call that gives the place runs twice, a wrong answer",
    file: "src/lower/mir/calls.rs",
    find: "            let place = match !crate::lower::held::fixed_place(place) && place.has_effects() {\n",
    replace: "            let place = match false {\n",
    tests: ["test/corpus.test.ts", "-t", "^assign_op_through_a_call.rs$"],
  },
  {
    name: "unbound-member-called-on",
    breaks:
      "`(s.f)()` of a `dyn Fn` is `s.f()`, which gives a with_this function `s` as its `this`, where Rust gives it `None` (#402)",
    file: "src/lower/mir/calls.rs",
    find: "            (PlaceElem::Field(index, _), ty::Adt(adt, _)) if adt.is_struct() => {\n",
    replace: "            (PlaceElem::Field(index, _), ty::Adt(adt, _)) if adt.is_struct() && false => {\n",
    tests: [
      "test/js-functions.test.ts",
      "-t",
      "a JS function the program's own data holds is called with no this",
    ],
  },
  {
    name: "unbound-item-called-on",
    breaks:
      "`fs[0]()` of a `dyn Fn` an array holds gives it the array as its `this` (#402)",
    file: "src/lower/mir/calls.rs",
    find: "            | (PlaceElem::Index(_) | PlaceElem::ConstantIndex { .. } | PlaceElem::Subslice { .. }, _) => Holder::Item,\n",
    replace: "            | (PlaceElem::Index(_) | PlaceElem::ConstantIndex { .. } | PlaceElem::Subslice { .. }, _) => Holder::Unknown,\n",
    tests: [
      "test/js-functions.test.ts",
      "-t",
      "a JS function the program's own data holds is called with no this",
    ],
  },
  {
    name: "dyn-fn-call-bound",
    breaks:
      "a `dyn Fn`'s or a generic `F`'s call through a field is `p[0]()`, giving it `p` as its `this` (#402)",
    file: "src/lower/mir/calls.rs",
    find: "                let call = self.called_unbound(state, (call, Called::Function(arg_tys[0], arg_tys[1], output)))?;\n",
    replace: "",
    tests: [
      "test/js-functions.test.ts",
      "-t",
      "a dyn Fn a field or an item holds is called with no this",
    ],
  },
  {
    name: "crate-closure-call-unbound",
    breaks:
      "a closure of the crate's a tuple holds is held first to be called, `const f = p[0]; f()`, though it never reads its `this` (#402)",
    file: "src/lower/mir/calls.rs",
    find: "            Some((state, output, out)) if !own => {\n",
    replace: "            Some((state, output, out)) => {\n",
    tests: [
      "test/js-functions.test.ts",
      "-t",
      "a dyn Fn a field or an item holds is called with no this",
    ],
  },
  {
    name: "this-read-anywhere",
    breaks:
      "a program that calls no binding, gives JS no function value, holds each call through a field first, `const f = s.f; f()`, though nothing it calls reads its `this` (#402)",
    file: "src/lower/mir/calls.rs",
    find: "        if !pointers.may_read_this(self.tcx, called, holder) {\n",
    replace: "        if false && !pointers.may_read_this(self.tcx, called, holder) {\n",
    tests: ["test/corpus.test.ts", "-t", "^refcell_listeners_momentary.rs$"],
    snapshots: true,
  },

  {
    name: "cell-get-field-unseen",
    breaks:
      "a `Cell<fn()>` field's value called, `(hooks.flush.get())()`, isn't seen as its field's, so one the crate fills with its own is held first (#427)",
    file: "src/lower/mir/calls.rs",
    find: "                        == Some(crate::lower::recognition::CellMethod::Get)\n",
    replace:
      "                        == Some(crate::lower::recognition::CellMethod::Other)\n",
    tests: [
      "test/lowering-passed-on.test.ts",
      "-t",
      "may hold a host function is held first",
    ],
  },
  {
    name: "host-fn-temporary-unheld",
    breaks:
      "a host function's field read MIR holds in a temporary, `_3 = CONFIG.schedule`, is read in place by tidy, `CONFIG.schedule(..)`, its `this` the config (#427)",
    file: "src/lower/mir/calls.rs",
    find: "        Ok(Value::Expr(Expr::call(Expr::unbound(*callee), args)))\n    }\n",
    replace: "        if let js::ExprKind::Var(_) = callee.kind {\n            return Ok(Value::Expr(Expr::call(*callee, args)));\n        }\n        Ok(Value::Expr(Expr::call(Expr::unbound(*callee), args)))\n    }\n",
    tests: [
      "test/lowering-passed-on.test.ts",
      "-t",
      "may hold a host function is held first",
    ],
  },
  {
    name: "spread-of-none-written",
    breaks:
      "`js::object::spread(None)`, a new object of nothing, is `{ ...undefined }`, not `{}` (rd210)",
    file: "src/lower/mir/calls.rs",
    find: "            if !matches!(value.kind, js::ExprKind::Undefined | js::ExprKind::Null) {\n                fields.push(Prop::Spread(value));\n            }\n",
    replace: "            fields.push(Prop::Spread(value));\n",
    tests: ["test/js-any-values.test.ts", "-t", "an object spread, a splice"],
  },
  {
    name: "through-flattened-unmarked",
    breaks:
      "a namespace's method's result through a value that only `.flatten()` reads is flattened again, `$someValue(renderer.flushSync(f))`, a `Some(None)` it gives read as `None` (rd311)",
    file: "src/lower/mir/calls.rs",
    find: "            state.flattened.locals.insert(local);\n",
    replace: "            let _ = local;\n",
    tests: ["test/namespaces.test.ts", "-t", "exact where its result is flattened"],
  },
  {
    name: "through-flattened-js-alike",
    breaks:
      "a copy that takes fewer of its type parameters as JS gives them than its function's own JS isn't taken as leaving out only a `Some`, so the crate's flattened call is refused (rd311)",
    file: "src/lower/mir/calls.rs",
    find: "        copied.js.is_subset(&own.js)\n",
    replace: "        copied.js == own.js\n",
    tests: ["test/namespaces.test.ts", "-t", "unlike its own JS"],
  },
  {
    name: "flattened-by-name-boxed",
    breaks:
      "a namespace's method by its name whose result is flattened is unboxed again, `$someValue(flushSync(f))`, where its library's JS gives it flattened (rd324)",
    file: "src/lower/mir/calls.rs",
    find: "        if let Some((_, Some((local, p)))) = namespace_call\n",
    replace: "        if let Some((false, Some((local, p)))) = namespace_call\n",
    tests: ["test/namespaces.test.ts", "-t", "by its name is its library"],
  },
  {
    name: "fn-value-none-given",
    breaks: "a trailing `None` given a `dyn Fn` is an argument, `cb(undefined)`, the function value's parameters unread",
    file: "src/lower/mir/calls.rs",
    find: "            false => crate::lower::calls::given_to_js_but(&mut exprs, absent),\n",
    replace: "            false => crate::lower::calls::given_to_js(&mut exprs),\n",
    tests: ["test/js-functions.test.ts", "-t", "a trailing None given a function value is no argument"],
  },
  {
    name: "own-call-none-given",
    breaks: "the crate's own function's trailing `None` is given, `shown(undefined)`, where a binding's and a function value's leave it out (ADR 0443)",
    file: "src/lower/mir/calls.rs",
    find: "            crate::lower::calls::own_absent(&mut exprs, |i, arg| {\n",
    replace: "            crate::lower::calls::own_absent(&mut exprs, |i, arg| false && {\n",
    tests: ["test/corpus.test.ts", "-t", "^option_arguments.rs$"],
    snapshots: true,
  },
  {
    name: "own-closure-none-given",
    breaks: "a closure's own trailing `None` is given, `cb(undefined)` of `cb(None)`, alone",
    file: "src/lower/mir/calls.rs",
    find: "            true => crate::lower::calls::own_absent(&mut exprs, absent),\n",
    replace: "            true => {\n                let _ = absent;\n            }\n",
    tests: ["test/corpus.test.ts", "-t", "^option_arguments.rs$"],
    snapshots: true,
  },
  {
    name: "scope-call-gives-self",
    breaks: "a call of a scope struct's method's function gives it the value it closes over, first, where it takes its other arguments",
    file: "src/lower/mir/calls.rs",
    find: "        args.remove(0);\n        let callee = match self.krate.never_unwinding.contains(&def_id) {",
    replace: "        let callee = match self.krate.never_unwinding.contains(&def_id) {",
    tests: ["test/corpus.test.ts", "-t", "scope_struct"],
  },
  {
    name: "scope-call-may-throw",
    breaks: "a call of a scope struct's method that never unwinds is wrapped in a `try` that drops on unwinding",
    file: "src/lower/mir/calls.rs",
    find: "            true => Expr::var(name).never_throws(),\n",
    replace: "            true => Expr::var(name),\n",
    tests: ["test/corpus.test.ts", "-t", "^refused_drops_and_displays.rs$"],
    snapshots: true,
  },

  {
    name: "arguments-own-call-left-out",
    breaks: "a call of the crate's function that reads `js::arguments()` leaves out its trailing `undefined`s, `new LinearGradient()` of five `None`s (rd332)",
    file: "src/lower/mir/calls.rs",
    find: "        if self.reads_arguments(def_id) {\n",
    replace: "        if false {\n",
    tests: ["test/arguments.test.ts"],
  },
  {
    name: "with-this-call-left-out",
    breaks: "a call of a `dyn Fn` that may be a `with_this` function reading `js::arguments()` leaves out its trailing `undefined` (rd509)",
    file: "src/lower/mir/calls.rs",
    find: "                    .may_call(self.tcx, self.krate.foreign, arg_tys[1]) =>\n",
    replace: "                    .may_call(self.tcx, self.krate.foreign, arg_tys[1]) && false =>\n",
    tests: ["test/arguments.test.ts"],
  },
  {
    name: "rc-field-unseen",
    breaks:
      "an `Rc<dyn Fn>` field's call, `(o.g)(1)`, isn't seen as its field's, through the `Rc`'s deref, so a JS object's function is called with no `this`, `(0, o.g)(1)` (rd579)",
    file: "src/lower/mir/calls.rs",
    find: "                                ty.is_box() || crate::lower::recognition::rc_pointee(self.tcx, ty).is_some()\n",
    replace: "                                false\n",
    tests: [
      "test/js-functions.test.ts",
      "-t",
      "a JS function the program's own data holds is called with no this",
    ],
  },
  {
    name: "variable-called-unbound",
    breaks:
      "a variable of the program's a call through a field gives no `this` is called `(0, ping)()`, so a closure that only calls it, `|_| ping()`, is no longer the function it calls, `addEventListener(\"load\", ping)` (rd579)",
    file: "src/lower/mir/calls.rs",
    find: "            && !super::tidy::is_temporary(name)\n        {\n            return Ok(Value::Expr(Expr::call(*callee, args)));\n",
    replace: "            && false\n        {\n            return Ok(Value::Expr(Expr::call(*callee, args)));\n",
    tests: ["test/lowering-passed-on.test.ts", "-t", "a closure that ignores what it's given is the function it calls"],
  },
  {
    name: "this-unread-unmarked",
    breaks: "a call of a field's function that reads no `this` isn't marked so, and alien-signals' `function link(..) { system.link(..); }` stays beside its system (#533)",
    file: "src/lower/mir/calls.rs",
    find: "                facts: pointers.call_facts(holder),\n",
    replace: "                facts: js::Facts::NONE,\n",
    tests: ["test/corpus.test.ts", "-t", "^thread_local_field_forwarders.rs$"],
    snapshots: true,
  },
  {
    name: "held-getter-read-as-import",
    breaks:
      "a getter's function a variable holds, `js::import!(version).await`'s, is called as the getter, read through an import nothing made, which panics rustc (#581)",
    file: "src/lower/mir/calls.rs",
    find: "                    crate::lower::bindings::JsForm::Call(_) | crate::lower::bindings::JsForm::Get(_)\n",
    replace: "                    crate::lower::bindings::JsForm::Call(_)\n",
    tests: ["test/bindings.test.ts", "-t", "js::import! of a getter reads its export"],
  },
  {
    name: "this-namespace-call-bound",
    breaks: "a call of a function that reads `js::this()` through its module's namespace is `Inner.onSomething()`, which gives it the namespace where Rust gives no `this` (rd683)",
    file: "src/lower/mir/calls.rs",
    find: "        if self.reads_this(def_id)\n",
    replace: "        if false\n",
    tests: ["test/this.test.ts"],
  },
  {
    name: "unkept-lent-vec-tracked",
    breaks:
      "a `Vec` a consumer lends a library's function whose library keeps no capacity, and which it can't compile itself, keeps the capacity it had: its `capacity()` its length, 6, where std's is 8, silently (#1180's (5))",
    file: "src/lower/mir/calls.rs",
    find: "                if ty.is_mutable_ptr() || !ty.is_freeze(self.tcx, self.typing_env) {\n",
    replace: "                if false {\n",
    tests: ["test/crates-compiled-here.test.ts", "-t", "keeps no capacity has its functions"],
  },
  {
    name: "unkept-result-tracked",
    breaks:
      "what a library's function whose library keeps no capacity, which the crate can't compile itself, gives back is taken as its `Vec`s' capacities known: their lengths, 5 where std's is 8, silently (#1180's (5))",
    file: "src/lower/mir/calls.rs",
    find: "        let called = match unkept && !output.is_scalar() && !output.is_unit() {\n",
    replace: "        let called = match false {\n",
    tests: ["test/crates-compiled-here.test.ts", "-t", "keeps no capacity has its functions"],
  },
  {
    name: "callback-write-result-thrown",
    breaks: "a `map(|w| writeln!(out, ..))` closure throws its error past what steps it, not an item (#1410)",
    file: "src/lower/mir/calls.rs",
    find: "        if !self.is_write_result(output) || !self.closure_may_fail(ty) {\n            return f;\n        }\n",
    replace: "        if true {\n            return f;\n        }\n",
    tests: ["test/corpus.test.ts", "-t", "^write_result_items.rs$"],
  },
  {
    name: "dyn-callback-output-unread",
    breaks: "a `Box<dyn FnMut(&&str) -> io::Result<()>>` given to `map` throws its error past what steps it, not an item (#1410)",
    file: "src/lower/mir/calls.rs",
    find: "            ty::Dynamic(bounds, ..) => match bounds.projection_bounds().find_map(|p| p.skip_binder().term.as_type()) {\n",
    replace: "            ty::Dynamic(bounds, ..) => match bounds.projection_bounds().find_map(|p| p.skip_binder().term.as_type()).filter(|_| false) {\n",
    tests: ["test/corpus.test.ts", "-t", "^write_result_items.rs$"],
  },
  {
    name: "callback-arguments-given-as-they-are",
    breaks: "a `dyn Fn` JS gave is called with JS's `null` as it is (#1840)",
    file: "src/lower/mir/calls.rs",
    find: "            self.ones_given_to_a_callback(state, (arg_tys[0], arg_tys[1], *output), (&tys, &present), &mut exprs);\n",
    replace: "",
    tests: given1840,
  },
  {
    name: "callback-call-of-each-count-unmade",
    breaks: "a `dyn Fn` JS gave, called of a trailing `None` known where it runs, is given `undefined`, `f(a, undefined)`, which a JS function counting what it's given tells from `f(a)` (#1876)",
    file: "src/lower/mir/calls.rs",
    find: "        let laddered = !own\n",
    replace: "        let laddered = false\n",
    tests: ["test/none-given.test.ts", "-t", "a function value's trailing None"],
  },
  {
    name: "callback-somes-unseen",
    breaks: "a `dyn Fn` JS gave, given `Some(b)`, tests what's there, `b != null ? f(a, b) : f(a)` (#1876)",
    file: "src/lower/mir/calls.rs",
    find: "            Some((state, _, _)) if laddered || !own => self.presences_in_a_tuple(state),\n",
    replace: "            Some((state, _, _)) if false => self.presences_in_a_tuple(state),\n",
    tests: ["test/none-given.test.ts", "-t", "a function value's trailing None"],
  },
];
