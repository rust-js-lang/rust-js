// Mutations of src/lower/passed_on.rs (ADR 0093).
import type { Mutation } from "../../mutations";

const once = [
  "test/lowering-passed-on.test.ts",
  "-t",
  "FnOnce callback that only passes its call on",
];
const kept = [
  "test/lowering-passed-on.test.ts",
  "-t",
  "only where the function would see",
];
const handler = [
  "test/jsx-events.test.ts",
  "-t",
  "calls a function with nothing",
];
const callback = [
  "test/lowering-functions.test.ts",
  "-t",
  "only passes its arguments on",
];
const ownCallbacks = ["test/corpus.test.ts", "-t", "^own_callback_params.rs$"];
const ownListener = [
  "test/lowering-passed-on.test.ts",
  "-t",
  "each call fills with the crate's function",
];
const restCallee = [
  "test/lowering-passed-on.test.ts",
  "-t",
  "of rest arguments that passes them all on",
];

export const mutations: Mutation[] = [
  {
    name: "own-unit-statement-kept",
    breaks:
      "a closure lent as a `&dyn Fn` whose statement only passes on to the crate's function of `()`, `{ preload(Some(href), ..); }`, stays an arrow, where upstream's `L: preload` is the function (rd660)",
    file: "src/lower/passed_on.rs",
    find: "            output.is_unit() || output.is_never()\n",
    replace: "            let _ = output;\n            false\n",
    tests: ["test/lowering-passed-on.test.ts", "-t", "whose statement only passes on to the crate's function"],
  },
  {
    name: "own-statement-of-a-value",
    breaks:
      "a closure that discards the crate's function's value, `{ counted(n); }`, is that function, so a JS caller sees what it gives (rd660)",
    file: "src/lower/passed_on.rs",
    find: "            output.is_unit() || output.is_never()\n",
    replace: "            let _ = output;\n            true\n",
    tests: ["test/lowering-passed-on.test.ts", "-t", "whose statement only passes on to the crate's function"],
  },
  {
    name: "statement-bound",
    breaks:
      "a closure's statement of the crate's function of what it took, then its parameter, is the function bound, `succeed.bind(null, queue)`, where upstream's `.then((s) => { onActionSuccess(queue, node, s); })` keeps its arrow (rd660)",
    file: "src/lower/passed_on.rs",
    find: "        && !fact.statement\n",
    replace: "",
    tests: ["test/lowering-passed-on.test.ts", "-t", "whose statement only passes on to the crate's function"],
  },
  {
    name: "passed-on-array-method-kept",
    breaks:
      "`xs.map((x) => double(x))` keeps its arrow: `map` isn't an array method whose callback may be the function (ADR 0279)",
    file: "src/lower/passed_on.rs",
    find: '        ["map", "filter", "some", "every", "find", "forEach"]\n',
    replace: '        ["filter", "some", "every", "find", "forEach"]\n',
    tests: callback,
  },
  {
    name: "passed-on-handler-kept",
    breaks:
      "`onClick={() => onClear()}`, not `onClick={onClear}`: no prop is an event's (ADR 0198)",
    file: "src/lower/passed_on.rs",
    find: "        js::is_handler_name(name).then_some(Taker::Handler)\n",
    replace:
      "        (js::is_handler_name(name) && false).then_some(Taker::Handler)\n",
    tests: handler,
  },
  {
    name: "once-callback-kept",
    breaks:
      "a binding's `FnOnce` callback that only passes its call on stays an arrow, `p.then(() => ping())`, not `p.then(ping)`",
    file: "src/lower/passed_on.rs",
    find: "            true => Taker::OnceCallback,\n",
    replace: "            true => Taker::Closure,\n",
    tests: once,
  },
  {
    name: "once-callback-of-any-fn-bound",
    breaks:
      "a binding's `impl Fn` parameter, a listener's, is the function the closure passes on to, `listenEvery(count)`, which JS may keep and compare",
    file: "src/lower/passed_on.rs",
    find: "            ty::Param(_) => crate::lower::recognition::function_kind(tcx, def_id, ty) == Some(ty::ClosureKind::FnOnce),\n",
    replace:
      "            ty::Param(_) => crate::lower::recognition::function_kind(tcx, def_id, ty).is_some(),\n",
    tests: once,
  },
  {
    name: "once-callback-of-boxed-fn",
    breaks:
      "a binding's `Box<dyn Fn>` parameter, a listener's, is a once-callback, so `|x| shown(x)` given to it is `shown`, which JS may keep and compare, and give more",
    file: "src/lower/passed_on.rs",
    find: "                ty::Dynamic(traits, ..) => traits.principal_def_id().is_some_and(only_once),\n",
    replace: "                ty::Dynamic(..) => true,\n",
    tests: ["test/corpus.test.ts", "-t", "^once_callback_passed_on.rs$"],
    snapshots: true,
  },
  {
    name: "once-callback-of-js-function",
    breaks:
      "a once-callback that calls a generic `impl Fn` or a `dyn Fn`, which may be a JS caller's function, is it: `p.then(f)` gives `f` the value it may read",
    file: "src/lower/passed_on.rs",
    find: "            Taker::OnceCallback => callee == Callee::Own,\n",
    replace: "            Taker::OnceCallback => true,\n",
    tests: kept,
  },
  {
    name: "array-method-of-dyn-fn",
    breaks:
      "`xs.map((x) => f(x))` of a `dyn Fn` is `xs.map(f)`, which gives a JS caller's function the index and the array",
    file: "src/lower/passed_on.rs",
    find: "            Taker::ArrayMethod => matches!(callee, Callee::Own | Callee::Generic),\n",
    replace: "            Taker::ArrayMethod => true,\n",
    tests: kept,
  },
  {
    name: "passed-on-without-fact",
    breaks:
      "an arrow MIR made no fact of, one rust-js writes itself, `(x) => f(x)`, is the function it calls, `f`, which then reads the index and array JS's `map` gives it too",
    file: "src/lower/passed_on.rs",
    find: "            let fact = (forwarding.iter()).find(|fact| !f.span.is_none() && fact.span == f.span)?;\n",
    replace:
      "            let fact = (forwarding.iter()).find(|fact| !f.span.is_none() && fact.span == f.span).copied().unwrap_or(Forwarding { span: f.span, callee: Callee::Own, adapts: true, item: false, binds: false, ignores: false, eta: false, fixed: false, statement: false });\n",
    tests: ["test/corpus.test.ts", "-t", "^array_map.rs$"],
  },
  {
    name: "optional-handler-kept",
    breaks:
      "a component's optional handler passed on is `(e) => { if (onClick != null) { onClick(e); } }`, not `onClick` (ADR 0198)",
    file: "src/lower/passed_on.rs",
    find: "    if taker == Taker::Handler\n        && let Some(handler) = optional_handler(f)\n",
    replace:
      "    if taker == Taker::Made\n        && let Some(handler) = optional_handler(f)\n",
    tests: ["test/jsx-events.test.ts", "-t", "optional event handler"],
  },
  {
    name: "made-member-kept",
    breaks:
      "a binding of a namespace's function taken as a `fn` stays an arrow, `(...data) => console.log(...data)`, not `console.log`",
    file: "src/lower/passed_on.rs",
    find: "        Taker::Made => named || matches!(callee.kind, js::ExprKind::Member(..)),\n",
    replace: "        Taker::Made => named,\n",
    tests: ["test/js-object.test.ts", "-t", "console is a value"],
  },
  {
    name: "generic-callback-kept",
    breaks:
      "an `impl Fn` called with what it's given is `(x, i) => f(x, i)`, not `f`",
    file: "src/lower/passed_on.rs",
    find: "            ty::Param(_) => Callee::Generic,\n",
    replace: "                    ty::Param(_) => return None,\n",
    tests: callback,
  },
  {
    name: "boxed-dyn-fn-not-rust",
    breaks:
      "a `Box<dyn Fn()>` prop's call isn't a Rust function's, `() => onClear()` kept",
    file: "src/lower/passed_on.rs",
    find: "            ty::Adt(adt, args) => adt.is_box() && matches!(args.type_at(0).kind(), ty::Dynamic(..)),\n",
    replace: "                    ty::Adt(adt, args) => adt.is_box() && matches!(args.type_at(0).kind(), ty::Never),\n",
    tests: handler,
  },
  {
    name: "binding-callee-passed-on",
    breaks:
      "`onClick={() => alert()}` is `onClick={alert}`, which shows the event, and `xs.map((x) => parseFloat(x))` `xs.map(parseFloat)`, which reads the index",
    file: "src/lower/passed_on.rs",
    find: "                callee: Callee::Host,\n",
    replace: "                callee: Callee::Own,\n",
    tests: [
      "test/jsx-events.test.ts",
      "test/lowering-functions.test.ts",
      "-t",
      "calls a function with nothing|only passes its arguments",
    ],
  },
  {
    name: "rest-callee-passed-on",
    breaks:
      "`xs.map((x) => many(x))` of a function of rest arguments is `xs.map(many)`, which counts the index and the array too",
    file: "src/lower/passed_on.rs",
    find: "\n                && (!self.takes_rest(def_id) || self.gathers_rest(body.def_id)))",
    replace: ")",
    tests: ["test/corpus.test.ts", "-t", "^once_callback_passed_on.rs$"],
  },
  {
    name: "passed-handler-wrapped",
    breaks:
      "an optional handler passed on to an element is wrapped, `(e) => { if (onClick != null) { onClick(e); } }`",
    file: "src/lower/passed_on.rs",
    find: "    (given && called && handler != param).then(|| tested.clone())",
    replace:
      "    (false && given && called && handler != param).then(|| tested.clone())",
    tests: [
      "test/jsx-events.test.ts",
      "-t",
      "optional event handler on as it is",
    ],
  },
  {
    name: "helper-called-by-name",
    breaks:
      "a std helper as a value is it by name, not the arrow that names what it's given by its type",
    file: "src/lower/passed_on.rs",
    find: "            js::ExprKind::Var(_) => callee.helper_name().is_none(),\n",
    replace: "            js::ExprKind::Var(_) => true,\n",
    snapshots: true,
    tests: ["test/corpus.test.ts", "-t", "^str_circumfix.rs$"],
  },
  {
    name: "called-arrow-wrapped",
    breaks:
      "an arrow called with exactly its parameters is wrapped in another, not itself",
    file: "src/lower/passed_on.rs",
    find: "                taken.len() == arity && taken.iter().all(|p| matches!(p, js::Pattern::Name(_)))\n",
    replace: "                false\n",
    tests: ["test/corpus.test.ts", "-t", "^writer_fn_pointer.rs$"],
    snapshots: true,
  },
  {
    name: "library-callee-not-rust",
    breaks:
      "a once-callback that passes its call on to a library's function, shared's `noop`, keeps its arrow, `then(() => { noop(); }, ..)`",
    file: "src/lower/passed_on.rs",
    find: "(def_id.is_local() || self.krate.foreign.is_library(def_id.krate))",
    replace: "def_id.is_local()",
    tests: [
      "test/cross-crate-items.test.ts",
      "-t",
      "once-callback that passes its call on",
    ],
  },
  {
    name: "library-rest-callee-passed-on",
    breaks:
      "a once-callback that gives one argument to a library's function of rest arguments is the function, which reads all JS gives it",
    file: "src/lower/passed_on.rs",
    find: "\n                && (!self.takes_rest(def_id) || self.gathers_rest(body.def_id)))",
    replace:
      "\n                && (def_id.is_local() && !self.takes_rest(def_id) || self.gathers_rest(body.def_id)))",
    tests: [
      "test/cross-crate-items.test.ts",
      "-t",
      "once-callback that passes its call on",
    ],
  },
  {
    name: "fn-pointer-statement-kept",
    breaks:
      "a closure as a `fn` whose call never returns, `|_, _| throw_invalid_hook_error()`, keeps its arrow, as its JS is a statement (ADR 0605, amended)",
    file: "src/lower/passed_on.rs",
    find: "                Taker::Handler | Taker::OnceCallback | Taker::FnPointer | Taker::Closure\n",
    replace: "                Taker::Handler | Taker::OnceCallback\n",
    tests: [
      "test/lowering-passed-on.test.ts",
      "-t",
      "a closure as a fn that only calls",
    ],
  },
  {
    name: "fn-pointer-wrapper-collapsed",
    breaks:
      "a closure as a `fn` of a function Rust could name there, a DEV dispatcher's wrapper `|context| read_context(context)`, is the function, not the function of its own the program wrote (ADR 0605, amended)",
    file: "src/lower/passed_on.rs",
    find: "            if taker == Taker::FnPointer && !fact.adapts {\n",
    replace: "            if false {\n",
    tests: [
      "test/lowering-passed-on.test.ts",
      "-t",
      "a closure as a fn that only calls",
    ],
  },
  {
    name: "fn-pointer-always-adapts",
    breaks:
      "every function a closure calls is taken to have a signature other than the closure's, so a wrapper of one Rust could name is the function (ADR 0605, amended)",
    file: "src/lower/passed_on.rs",
    find: "        inputs.len() != called.inputs().len()\n",
    replace: "        true || inputs.len() != called.inputs().len()\n",
    tests: [
      "test/lowering-passed-on.test.ts",
      "-t",
      "a closure as a fn that only calls",
    ],
  },
  {
    name: "bound-any-variable",
    breaks:
      "a closure that passes on what it borrows, `(cb, o) => startTransition(fiber, false, cb, o)` of `&fiber`, is bound to it as a moved value is (ADR 0605, amended)",
    file: "src/lower/passed_on.rs",
    find: "    let taken = |e: &Expr| matches!(&e.kind, js::ExprKind::Var(v) if moved.contains(v));\n",
    replace:
      '    let taken = |e: &Expr| matches!(&e.kind, js::ExprKind::Var(v) if moved.contains(v) || v != "");\n',
    tests: ["test/lowering-passed-on.test.ts", "-t", "is the function bound"],
  },
  {
    name: "bound-of-parameters-alone",
    breaks:
      "alien-signals' `computed` is `() => computedOper(this$)`, whose `name` is `\"\"`, where upstream's `computedOper.bind(..)` is `bound computedOper`: a move closure of no parameters of its own that passes on what it took stays its arrow (#502, ADR 0605, amended)",
    file: "src/lower/passed_on.rs",
    find: "    (callee.is_named() && bound.iter().all(value))",
    replace: "    (callee.is_named() && !params.is_empty() && bound.iter().all(value))",
    tests: ["test/lowering-passed-on.test.ts", "-t", "is the function bound"],
  },
  {
    name: "bound-a-closure",
    breaks:
      "a closure that passes on to a closure, `(cb, o) => start(true, cb, o)`, is `start.bind(null, true)` (ADR 0605, amended)",
    file: "src/lower/passed_on.rs",
    find: "    fact.item\n        && fact.callee == Callee::Own\n",
    replace: "    fact.callee == Callee::Own\n",
    tests: ["test/lowering-passed-on.test.ts", "-t", "is the function bound"],
  },
  {
    name: "bound-of-nothing-captured",
    breaks:
      'a closure that captures nothing, `(x) => noisy("take", x)`, is `noisy.bind(null, "take")`, which an arrow called in place leaves `noisy.bind(null, "find")(x)` (ADR 0605, amended)',
    file: "src/lower/passed_on.rs",
    find: "bound.iter().any(taken) && ",
    replace: "",
    tests: ["test/lowering-passed-on.test.ts", "-t", "is the function bound"],
  },
  {
    name: "bound-never",
    breaks:
      "React's `startTransition.bind(null, fiber, queue, true, false)` stays `(callback, options) => startTransition(fiber, queue, true, false, callback, options)` (ADR 0605, amended)",
    file: "src/lower/passed_on.rs",
    find: "    (forwarding.iter()).find(|fact| fact.binds && ",
    replace: "    (forwarding.iter()).find(|fact| false && fact.binds && ",
    tests: ["test/lowering-passed-on.test.ts", "-t", "is the function bound"],
  },
  {
    name: "rest-callee-never-passed-on",
    breaks:
      "ReactFiberConfigWithNoPersistence's `export const cloneInstance = shim;` is `(...args) => { shim(...args); }`: a closure of rest arguments that passes them all on to a rest function stays its arrow (ADR 0605, amended)",
    file: "src/lower/passed_on.rs",
    find: "(!self.takes_rest(def_id) || self.gathers_rest(body.def_id))",
    replace: "!self.takes_rest(def_id)",
    tests: restCallee,
  },
  {
    name: "rest-gathered-of-any-last-parameter",
    breaks:
      "a closure's last parameter of any type gathers JS's arguments: `xs.map((x) => total(x))` of a rest function is `xs.map(total)`",
    file: "src/lower/passed_on.rs",
    find: "ty::Tuple(items) => items.last().is_some_and(rest),",
    replace: "ty::Tuple(items) => !items.is_empty(),",
    tests: restCallee,
  },
  {
    name: "namespace-fn-value-arrowed",
    breaks:
      "a library's function given as a value through its module's namespace, `&dep::noop` as a `&dyn Fn`, is `() => Dep.noop()`, not `Dep.noop` as its named import was (ADR 1018, amended)",
    file: "src/lower/passed_on.rs",
    find: "            js::ExprKind::Member(..) => callee.is_named(),\n",
    replace: "",
    tests: [
      "test/crates.test.ts",
      "-t",
      "returned past a guard's drop is returned after its finally",
    ],
  },
  {
    name: "host-callee-not-a-fn",
    breaks:
      "a closure as a `fn` that only calls a host function, `|cb| { js::set_timeout(cb, None); }`, stays its arrow, where React holds `setTimeout` itself (rd158)",
    file: "src/lower/passed_on.rs",
    find: "            Taker::FnPointer => matches!(callee, Callee::Own | Callee::Host),\n",
    replace: "            Taker::FnPointer => callee == Callee::Own,\n",
    tests: ["test/lowering-passed-on.test.ts", "-t", "one host function"],
  },
  {
    name: "host-statement-unread",
    breaks:
      "a closure of one statement, `{ js::set_timeout(cb, None); }`, passes its call on to nothing, an arrow (rd158)",
    file: "src/lower/passed_on.rs",
    find: "                    statement = true;\n",
    replace:
      "                    statement = true;\n                    return None;\n",
    tests: ["test/lowering-passed-on.test.ts", "-t", "one host function"],
  },
  {
    name: "own-statement-passed-on",
    breaks:
      "a closure that discards the crate's function's value, `|a| { first(a); }`, is that function, so a JS caller sees what it gives (rd158)",
    file: "src/lower/passed_on.rs",
    find: "        if statement && !super::bindings::is_binding(self.tcx, def_id) && !gives_nothing() {\n",
    replace:
      "        if false && statement && !super::bindings::is_binding(self.tcx, def_id) && !gives_nothing() {\n",
    tests: [
      "test/lowering-passed-on.test.ts",
      "-t",
      "only calls the crate's function",
    ],
  },
  {
    name: "host-callee-a-handler",
    breaks:
      "an event handler that only calls a binding's function with the event is that function, which may read what React gives it more (rd158)",
    file: "src/lower/passed_on.rs",
    find: "            Taker::Handler => callee != Callee::Host,\n",
    replace: "            Taker::Handler => true,\n",
    tests: ["test/jsx-events.test.ts"],
  },

  {
    name: "ignoring-not-taken",
    breaks:
      "a callback taker takes no closure that ignores what it's given, a listener's arrow kept (rd212)",
    file: "src/lower/passed_on.rs",
    find: "            if !taker.takes(fact.callee) && !fact.ignores && !fact.eta {\n",
    replace: "            if !taker.takes(fact.callee) && !fact.eta {\n",
    tests: ["test/lowering-passed-on.test.ts", "-t", "ignores what it's given"],
  },
  {
    name: "ignoring-item-given",
    breaks:
      "a closure that calls a function of the crate's with nothing isn't known to ignore what it's given (rd212)",
    file: "src/lower/passed_on.rs",
    find: "                ignores: args.is_empty(),\n",
    replace: "                ignores: false,\n",
    tests: ["test/lowering-passed-on.test.ts", "-t", "ignores what it's given"],
  },
  {
    name: "bound-own-dyn",
    breaks:
      "a bound function of the crate's is taken for a `dyn Fn` a JS function may be, `move |_| ping()` kept an arrow (rd196)",
    file: "src/lower/passed_on.rs",
    find: "            _ if dyn_fn && captured.is_some_and(|var| self.bound_own(var)) => Callee::Own,\n",
    replace: "",
    tests: ["test/lowering-passed-on.test.ts", "-t", "ignores what it's given"],
  },
  {
    name: "bound-host-own",
    breaks:
      "a binding's JS function bound is taken for the crate's, given to a listener that reads what it's given (rd196)",
    file: "src/lower/passed_on.rs",
    find: "                    && !super::bindings::is_binding(tcx, f)\n                    && !self.takes_rest(f)\n",
    replace: "                    && !self.takes_rest(f)\n",
    tests: ["test/lowering-passed-on.test.ts", "-t", "ignores what it's given"],
  },
  {
    name: "bound-with-this-not-own",
    breaks:
      "a `with_this` function bound isn't taken for one that ignores what it's given, `onUnsuspend.bind(state)` kept in an arrow (rd196)",
    file: "src/lower/passed_on.rs",
    find: "                ty::FnDef(f, _) => matches!(super::bindings::js_form(tcx, f), super::bindings::JsForm::WithThis),\n",
    replace: "                ty::FnDef(..) => false,\n",
    tests: ["test/lowering-passed-on.test.ts", "-t", "ignores what it's given"],
  },
  {
    name: "passed-on-closure-of-a-left-out-none",
    breaks:
      "a closure that calls a closure with a trailing `None`, `|n| tagged(n, None)`, is that closure, `[3, 4].map(tagged)`, which JS's `map` gives the index for the `None`: a wrong answer",
    file: "src/lower/passed_on.rs",
    find: "                    ExprKind::Tuple { ref fields } if leaves_out(fields) => return None,\n",
    replace:
      "                    ExprKind::Tuple { ref fields } if false && leaves_out(fields) => return None,\n",
    tests: ["test/corpus.test.ts", "-t", "^option_arguments.rs$"],
  },
  {
    name: "passed-on-function-of-a-left-out-none",
    breaks:
      "a closure that calls the crate's function with a trailing `None`, `|n| plus(n, None)`, is that function, `[1, 2].map(plus)`, which JS's `map` gives the index for the `None`: a wrong answer",
    file: "src/lower/passed_on.rs",
    find: "                && !leaves_out(args)\n",
    replace: "                && !(false && leaves_out(args))\n",
    tests: ["test/corpus.test.ts", "-t", "^option_arguments.rs$"],
  },
  {
    name: "forwarded-function-self-passed",
    breaks:
      "a namespace's method that only passes its parameters on, past its `self`, is no function it calls: a function of its own beside its module's `pub use`-like re-export (rd292b)",
    file: "src/lower/passed_on.rs",
    find: "    let (target, args) = super::body_queries::forwards_to(thir, 1)?;\n",
    replace: "    let (target, args) = super::body_queries::forwards_to(thir, 0)?;\n",
    tests: ["test/namespaces.test.ts", "-t", "only passes its parameters on"],
  },
  {
    name: "forwarded-function-of-its-own-module",
    breaks:
      "a namespace's method that passes its parameters on to its own module's function that it doesn't export by the method's name, which may be private, is that function re-exported from itself, `export { doubleIt as double } from \"./reconciler.js\"` (rd292b, #566)",
    file: "src/lower/passed_on.rs",
    find: "        || own_unexported\n",
    replace: "",
    tests: ["test/namespaces.test.ts", "-t", "only passes its parameters on"],
  },
  {
    name: "forwarded-function-other-generics",
    breaks:
      "a namespace's method that passes its parameters on to a function it gives other generic arguments, `size_of::<u64>`, is that function, which then takes the fact the method knows (rd292b)",
    file: "src/lower/passed_on.rs",
    find: "    if kept(args) != kept(own) {\n",
    replace: "    if false && kept(args) != kept(own) {\n",
    tests: ["test/namespaces.test.ts", "-t", "only passes its parameters on"],
  },
  {
    name: "forwarded-function-other-parameters",
    breaks:
      "a namespace's method of a `&mut` the function takes as a `&` is that function, given the method's handle (rd292b)",
    file: "src/lower/passed_on.rs",
    find: "    (mine.inputs().get(1..) == Some(theirs.inputs()) && mine.output() == theirs.output()).then_some((target, args))\n",
    replace: "    (mine.output() == theirs.output()).then_some((target, args))\n",
    tests: ["test/namespaces.test.ts", "-t", "only passes its parameters on"],
  },
  {
    name: "eta-namespace-unseen",
    breaks:
      "a closure that passes all it's given to a namespace's function through a value, `move |f, a| renderer.batched_updates(f, a)`, keeps its arrow, not `renderer.batchedUpdates` (the user's decision)",
    file: "src/lower/passed_on.rs",
    find: "                eta: super::classes::member(self.tcx, def_id) == Some(super::classes::Member::Namespace)\n",
    replace: "                eta: false\n",
    tests: ["test/namespaces.test.ts", "-t", "passes all it's given"],
  },
  {
    name: "eta-member-unnamed",
    breaks:
      "a closure that passes all it's given to a namespace's function through a value is no function of a name, so its arrow is kept (the user's decision)",
    file: "src/lower/passed_on.rs",
    find: "        || eta\n",
    replace: "        || false\n",
    tests: ["test/namespaces.test.ts", "-t", "passes all it's given"],
  },
  {
    name: "fixed-local-function-unheld",
    breaks:
      "a listener that only calls a local function JS makes, `handleNavigateComplete`, is a function of its own, `function onSuccess() { handleNavigateComplete(); }` (rd284b)",
    file: "src/lower/passed_on.rs",
    find: "                    fixed: dyn_fn && callee == Callee::Own,\n",
    replace: "                    fixed: false,\n",
    tests: ["test/webapi-events.test.ts", "-t", "event methods and reusable listeners"],
  },
  {
    name: "with-this-reading-this-held",
    breaks:
      "a listener that calls a `with_this` function that reads its `this` is that function, whose `this` is then the target, not `undefined` (rd284b)",
    file: "src/lower/passed_on.rs",
    find: "            matches!(tcx.hir_body(closure.body).params.first(), Some(param) if matches!(param.pat.kind, hir::PatKind::Wild))\n",
    replace: "            matches!(tcx.hir_body(closure.body).params.first(), Some(_))\n",
    tests: ["test/webapi-events.test.ts", "-t", "event methods and reusable listeners"],
  },
  {
    name: "with-this-never-own",
    breaks:
      "a function `function::with_this` makes of a closure that ignores its `this` isn't taken as one that reads none, so a listener calling it stays a function of its own (rd284b)",
    file: "src/lower/passed_on.rs",
    find: "                    ignores_this(self.tcx, target)\n",
    replace: "                    false\n",
    tests: ["test/webapi-events.test.ts", "-t", "event methods and reusable listeners"],
  },
  {
    name: "flattened-forwarding-unseen",
    breaks:
      "a closure that only passes its argument on to a namespace's method by its name, its result flattened, `|f| Reconciler.flush_sync(f).flatten()`, is an arrow of its own, `(f) => flushSync(f)`, not the function, as one through a value is (rd324, the user's decision)",
    file: "src/lower/passed_on.rs",
    find: "            && super::analysis::flattened_by_name(&self.recognition(), &mir.body, called).is_some()\n",
    replace: "            && super::analysis::flattened_by_name(&self.recognition(), &mir.body, called).is_some() && false\n",
    tests: ["test/namespaces.test.ts", "-t", "by its name is its library"],
  },
  {
    name: "arguments-forwarded",
    breaks: "a closure that only passes on what it's given to a function reading `js::arguments()` is that function, which then counts what its JS caller gives it (rd332)",
    file: "src/lower/passed_on.rs",
    find: "                && !self.reads_arguments(def_id)\n",
    replace: "",
    tests: ["test/arguments.test.ts"],
  },
  {
    name: "forwarder-never",
    breaks:
      "a function that only passes its parameters on to a function at one instantiation stays a function, `function decodeAction(body, m) { return decodeAction$1(body, m); }`, not that function re-exported (rd575)",
    file: "src/lower/passed_on.rs",
    find: "    let (target, args) = super::body_queries::forwards_to(thir, 0)?;\n",
    replace: "    let (target, args) = super::body_queries::forwards_to(thir, 0).filter(|_| false)?;\n",
    tests: ["test/declarations-libraries.test.ts", "test/lowering-forwarding.test.ts", "-t", "is its re-export"],
  },
  {
    name: "forwarder-other-signature",
    breaks:
      "a function that gives nothing, `fn discards(n: u32) { kept(n); }`, is the function it calls, which gives a value, re-exported (rd575)",
    file: "src/lower/passed_on.rs",
    find: "    (mine.inputs_and_output == theirs.inputs_and_output).then_some((target, args))\n",
    replace: "    Some((target, args))\n",
    tests: ["test/declarations-libraries.test.ts", "test/lowering-forwarding.test.ts", "-t", "is its re-export"],
  },
  {
    name: "counted-closure-ungated",
    breaks:
      "a closure made an `Rc`'s keeps its arrow where the crate compares no uncounted `Rc<dyn Fn>`, `Rc::new(move |a, b| renderer.discrete_updates(a, b))` not `renderer.discreteUpdates` (rd313)",
    file: "src/lower/passed_on.rs",
    find: "        if !self.krate.counted.compared_uncounted {\n            return false;\n        }\n",
    replace: "",
    tests: ["test/namespaces.test.ts", "-t", "through a value is that function"],
  },
  {
    name: "own-callback-params-none",
    breaks:
      "a `dyn Fn` parameter each of the crate's calls fills with its own function isn't one, `on(() => { onEnd(); })` where React Flight's Node client gives `stream.on('end', onEnd)` (rd616)",
    file: "src/lower/passed_on.rs",
    find: "        self.krate.own_callbacks.get_or_init(|| self.find_own_callbacks())\n",
    replace: "        self.krate.own_callbacks.get_or_init(HashSet::new)\n",
    tests: ownCallbacks,
    snapshots: true,
  },
  {
    name: "own-callback-param-changed",
    breaks:
      "a `mut` parameter, which its body may set to what a JS caller gave, is taken for what its calls give it (rd616)",
    file: "src/lower/passed_on.rs",
    find: "                let hir::PatKind::Binding(hir::BindingMode::NONE, var, _, None) = param.pat.kind else {\n",
    replace: "                let hir::PatKind::Binding(_, var, _, None) = param.pat.kind else {\n",
    tests: ownCallbacks,
    snapshots: true,
  },
  {
    name: "own-callback-unfilled-kept",
    breaks:
      "a parameter one call fills with another that a JS caller fills is taken for the crate's function, `relay`'s `on(onEnd)` of an exported function's callback (rd616)",
    file: "src/lower/passed_on.rs",
    find: "            if unfilled.is_empty() {\n",
    replace: "            if true {\n",
    tests: ownCallbacks,
    snapshots: true,
  },
  {
    name: "own-target-reads-arguments",
    breaks:
      "a function of the crate's that reads `js::arguments()` is taken for one that reads nothing it's given more, a listener's, which then counts the event (rd616)",
    file: "src/lower/passed_on.rs",
    find: "                    && !self.reads_arguments(f);\n",
    replace: ";\n",
    tests: ownListener,
  },
  {
    name: "with-this-reading-arguments-own",
    breaks:
      "a `with_this` function that ignores its `this` but reads `js::arguments()` is taken for one that reads nothing it's given more, a listener's, which then counts the event (rd616)",
    file: "src/lower/passed_on.rs",
    find: "                        && !made_closure(target).is_some_and(|c| self.krate.reads_arguments.made.contains(&c))\n",
    replace: "",
    tests: ownListener,
  },
  {
    name: "own-callback-block-unseen",
    breaks:
      "a function made as a block's value, `let handle_end: Rc<dyn Fn()> = { ..; Rc::new(move || ..) }`, isn't seen to be the crate's (rd616)",
    file: "src/lower/passed_on.rs",
    find: "            hir::ExprKind::Block(block, _) => block.expr.is_some_and(inner),\n",
    replace: "            hir::ExprKind::Block(..) => false,\n",
    tests: ownCallbacks,
    snapshots: true,
  },
  {
    name: "own-callback-borrow-unseen",
    breaks:
      "a borrowed closure, `listen_end(&|| close(..))` of a `&dyn Fn()`, isn't seen to be the crate's (rd616)",
    file: "src/lower/passed_on.rs",
    find: "            hir::ExprKind::AddrOf(_, _, of) => inner(of),\n",
    replace: "            hir::ExprKind::AddrOf(..) => false,\n",
    tests: ownCallbacks,
    snapshots: true,
  },
  {
    name: "own-callback-param-given-on-unseen",
    breaks:
      "a parameter given on to another, `ended_by(on_end)`, isn't seen to be what its own calls fill it with (rd616)",
    file: "src/lower/passed_on.rs",
    find: "                hir::def::Res::Local(var) => params.contains(&var) || let_value(self.tcx, var).is_some_and(inner),\n",
    replace: "                hir::def::Res::Local(var) => let_value(self.tcx, var).is_some_and(inner),\n",
    tests: ownCallbacks,
    snapshots: true,
  },
  {
    name: "own-callback-through-rc-unseen",
    breaks:
      "a closure that calls an `Rc<dyn Fn()>` it captured, through the `Rc`'s `Deref`, isn't seen to call that variable, `addEventListener(\"end\", () => { onEnd(); })` (rd616)",
    file: "src/lower/passed_on.rs",
    find: "            } if args.len() == 1 && deref(fun) => e = args[0],\n",
    replace: "            } if false && deref(fun) => e = args[0],\n",
    tests: ownCallbacks,
    snapshots: true,
  },
  {
    name: "own-callback-boxed-unseen",
    breaks: "a boxed closure, `later(Box::new(|| ..))`, isn't seen to be the crate's (rd616)",
    file: "src/lower/passed_on.rs",
    find: "        super::recognition::is_std_def(self.tcx, f, StdItem::BoxNew)\n            ||",
    replace: "        false\n            ||",
    tests: ownCallbacks,
    snapshots: true,
  },
  {
    name: "own-callback-counted-unseen",
    breaks: "a closure made an `Rc`'s, `Rc::new(|| close(..))`, isn't seen to be the crate's (rd616)",
    file: "src/lower/passed_on.rs",
    find: "                Some(HandleStep::RcNew | HandleStep::RcClone)\n",
    replace: "                Some(HandleStep::RcClone)\n",
    tests: ownCallbacks,
    snapshots: true,
  },
  {
    name: "own-callback-shared-unseen",
    breaks: "an `Rc`'s clone, `Rc::clone(&handle_end)`, isn't seen to be what it's a clone of (rd616)",
    file: "src/lower/passed_on.rs",
    find: "                Some(HandleStep::RcNew | HandleStep::RcClone)\n",
    replace: "                Some(HandleStep::RcNew)\n",
    tests: ownCallbacks,
    snapshots: true,
  },
  {
    name: "forwarder-generic",
    breaks:
      "a generic function that passes its parameters on is the function it calls, re-exported, declared of what that's given, not of its own type parameters (rd575)",
    file: "src/lower/passed_on.rs",
    find: "    if tcx.def_kind(function) != DefKind::Fn || generic {\n",
    replace: "    if tcx.def_kind(function) != DefKind::Fn {\n",
    tests: ["test/declarations-libraries.test.ts", "test/lowering-forwarding.test.ts", "-t", "is its re-export"],
  },
  {
    name: "forwarder-of-its-own-module",
    breaks:
      "a function that calls its own module's function with its parameters is re-exported from its own module (rd575)",
    file: "src/lower/passed_on.rs",
    find: "        || (target.is_local() && module(target) == module(function))\n",
    replace: "",
    tests: ["test/declarations-libraries.test.ts", "test/lowering-forwarding.test.ts", "-t", "is its re-export"],
  },
  {
    name: "forwarded-call-commented",
    breaks:
      "a function that only passes its parameters on, with a comment in its body, is re-exported, its comment left where no statement is (rd575)",
    file: "src/lower/passed_on.rs",
    find: "    let commented = written.is_none_or(|text| text.contains(\"//\") || text.contains(\"/*\"));\n",
    replace: "    let commented = written.is_none();\n",
    tests: ["test/declarations-libraries.test.ts", "test/lowering-forwarding.test.ts", "-t", "is its re-export"],
  },
  {
    name: "forwarded-call-out-of-line",
    breaks:
      "a function kept out of line, `#[inline(never)]`, is re-exported, losing its `@noinline` (rd575, ADR 0649)",
    file: "src/lower/passed_on.rs",
    find: "    if !function.export || function.noinline || commented {\n",
    replace: "    if !function.export || commented {\n",
    tests: ["test/declarations-libraries.test.ts", "test/lowering-forwarding.test.ts", "-t", "is its re-export"],
  },
  {
    name: "let-value-of-mut",
    breaks:
      "a `let mut` written again, `error = listener`, is taken for what its initializer is, so a field written it is taken for one of the crate's own closures, `rewritten.error()`, which gives the caller's JS function `rewritten` as its `this` (rd655)",
    file: "src/lower/passed_on.rs",
    find: "    let hir::PatKind::Binding(hir::BindingMode(hir::ByRef::No, rustc_hir::Mutability::Not), _, _, None) = pat.kind\n",
    replace: "    let hir::PatKind::Binding(hir::BindingMode(hir::ByRef::No, _), _, _, None) = pat.kind\n",
    tests: [
      "test/js-functions.test.ts",
      "-t",
      "a field written a variable of the crate's own closure is called through its object",
    ],
  },
  {
    name: "scope-method-closure-kept",
    breaks:
      "a closure that passes all it's given to a scope struct's method keeps its arrow, `propagate: (node) => propagate(node)` where upstream returns `{ propagate }` (#500)",
    file: "src/lower/passed_on.rs",
    find: "\n                    || self.krate.scopes.of_method(def_id).is_some() && !self.krate.counted.points_at_functions,",
    replace: ",",
    tests: ["test/corpus.test.ts", "-t", "scope_struct"],
    snapshots: true,
  },
  {
    name: "scope-method-closure-identity-unasked",
    breaks:
      "a closure that passes on to a scope struct's method is that method where the crate compares closures' addresses, so two of one method are one, `ptr::addr_eq` true where native gives false (#500, the user's decision)",
    file: "src/lower/passed_on.rs",
    find: "self.krate.scopes.of_method(def_id).is_some() && !self.krate.counted.points_at_functions,",
    replace: "self.krate.scopes.of_method(def_id).is_some(),",
    tests: ["test/corpus.test.ts", "-t", "^scope_method_closure_addr_eq.rs$"],
  },
  {
    name: "counted-closure-ungated-of-a-scope-method",
    breaks:
      "a closure made an `Rc`'s that passes on to a scope struct's method is that method where the crate compares uncounted `Rc<dyn Fn>`s, so two `Rc`s of one method are `Rc::ptr_eq`, true where native gives false (#500, the user's decision)",
    file: "src/lower/passed_on.rs",
    find: "        if !self.krate.counted.compared_uncounted {\n            return false;\n        }\n",
    replace: "        if true {\n            return false;\n        }\n",
    tests: ["test/corpus.test.ts", "-t", "^scope_method_closure_ptr_eq.rs$"],
  },
  {
    name: "thunk-handed-on-bound",
    breaks:
      "Fizz's `scheduleWork(() => performWork(request))` is `scheduleWork(performWork.bind(null, request))`, and its `createDrainHandler`'s `return () => startFlowing(request, destination)` is bound: a thunk handed on as a callback, which JS writes in place, is bound (#502, ADR 0605, amended)",
    file: "src/lower/passed_on.rs",
    find: "        && !kept\n",
    replace: "",
    tests: ["test/lowering-passed-on.test.ts", "-t", "of rest arguments, that passes on what it captured"],
  },
  {
    name: "thunk-made-an-rc-handed-on",
    breaks:
      "React's Flight client's `startReadingFromStream(response, stream, close.bind(null, response))` keeps its arrow: a thunk an `Rc` is made of, a function its holders share, is taken for a callback handed on (#502, ADR 0605, amended)",
    file: "src/lower/passed_on.rs",
    find: "                    && !fn_def(thir[fun].ty).is_some_and(|(id, _)| super::recognition::is_rc_new(self.tcx, id))\n",
    replace: "",
    tests: ["test/lowering-passed-on.test.ts", "-t", "of rest arguments, that passes on what it captured"],
  },
  {
    name: "thunk-call-argument-bound",
    breaks:
      "`schedule(move || perform(&this))` and a returned `Box::new(move || perform(&this))` are `perform.bind(null, this$)`: a thunk given to a call is bound (#502, ADR 0605, amended)",
    file: "src/lower/passed_on.rs",
    find: "                args.iter().any(|&a| closure(a))\n",
    replace: "                false && args.iter().any(|&a| closure(a))\n",
    tests: ["test/lowering-passed-on.test.ts", "-t", "of rest arguments, that passes on what it captured"],
  },
  {
    name: "thunk-lent-bound",
    breaks:
      "`lend(&move || perform(&this))`, a thunk lent as a `&dyn Fn()`, is `lend(perform.bind(null, this$))` (#502, ADR 0605, amended)",
    file: "src/lower/passed_on.rs",
    find: "            ExprKind::Borrow { arg, .. } => closure(arg),\n",
    replace: "",
    tests: ["test/lowering-passed-on.test.ts", "-t", "of rest arguments, that passes on what it captured"],
  },
  {
    name: "thunk-of-a-block-bound",
    breaks:
      "createReactNoop's root's `getChildren() { return getChildren(container); }`, of `Box::new({ let container = ..; move || get_children(..) })`, is `getChildren: getChildren.bind(null, container)`: a thunk that's a block's value, boxed, isn't seen handed on (#502, ADR 0605, amended)",
    file: "src/lower/passed_on.rs",
    find: "                ExprKind::Block { block } if let Some(value) = thir[block].expr => a = value,\n",
    replace: "",
    tests: ["test/lowering-passed-on.test.ts", "-t", "of rest arguments, that passes on what it captured"],
  },
  {
    name: "thunk-wrapped-bound",
    breaks:
      "`guarded(AssertUnwindSafe(move || computed_oper(&this)))`, a thunk wrapped for the call it's given to, is `guarded(computedOper.bind(null, this$))` (#502, ADR 0605, amended)",
    file: "src/lower/passed_on.rs",
    find: "                ExprKind::Adt(ref adt) if let [only] = &adt.fields[..] => a = only.expr,\n",
    replace: "",
    tests: ["test/lowering-passed-on.test.ts", "-t", "of rest arguments, that passes on what it captured"],
  },
  {
    name: "own-module-forwarding-unexported",
    breaks:
      "a namespace's method that only passes its parameters on to its own module's function is a function of its own, `close$1`, and refused as named apart (#566)",
    file: "src/lower/passed_on.rs",
    find: "        || own_unexported\n",
    replace: "        || module(target) == module(method)\n",
    tests: ["test/method-forwarding.test.ts", "-t", "a namespace's method that only passes its parameters on to its module's function"],
  },
  {
    name: "forwarded-result-unasked",
    breaks:
      "a namespace's method of `()` whose body is `f(x);` of a function of another result is that function, whose result its JS callers then see (#566)",
    file: "src/lower/passed_on.rs",
    find: "    (mine.inputs().get(1..) == Some(theirs.inputs()) && mine.output() == theirs.output()).then_some((target, args))\n",
    replace: "    (mine.inputs().get(1..) == Some(theirs.inputs())).then_some((target, args))\n",
    tests: ["test/method-forwarding.test.ts", "-t", "a prototype's method that only passes its parameters on"],
  },
  {
    name: "prototype-forwarding-unasked",
    breaks:
      "a prototype's method that only passes its parameters on is never the function it calls, `X.prototype.m = function (..) { f(..); }` (rd715)",
    file: "src/lower/passed_on.rs",
    find: "    if super::classes::member(tcx, method) != Some(super::classes::Member::Namespace) && !prototype {\n",
    replace: "    if super::classes::member(tcx, method) != Some(super::classes::Member::Namespace) {\n",
    tests: ["test/method-forwarding.test.ts", "-t", "a prototype's method that only passes its parameters on"],
  },
  {
    name: "prototype-forwarding-elsewhere",
    breaks:
      "a prototype's method that only passes its parameters on to another module's function is that function, read where its module loads, which a cycle may not have loaded (rd715)",
    file: "src/lower/passed_on.rs",
    find: "    let elsewhere = prototype && module(target) != module(method);\n",
    replace: "    let elsewhere = false;\n",
    tests: ["test/method-forwarding.test.ts", "-t", "a declaration its module's load can read"],
  },
  {
    name: "converting-unseen",
    breaks:
      "a closure that only converts what it passes on to a function value it captured keeps its arrow, alien-signals' `getter: (previous) => getter(previous)` (ADR 0605, amended)",
    file: "src/lower/passed_on.rs",
    find: "            eta: true,\n",
    replace: "            eta: false,\n",
    tests: ["test/lowering-passed-on.test.ts", "-t", "only converts what it passes on"],
  },
  {
    name: "converting-identity-unasked",
    breaks:
      "a converting closure is its function where the crate compares two boxes of them by `ptr::addr_eq`, which JS says are one, where Rust says two (ADR 0605, amended)",
    file: "src/lower/passed_on.rs",
    find: "        if self.krate.counted.points_at_functions || !self.tcx.is_closure_like(body.def_id) {\n",
    replace: "        if !self.tcx.is_closure_like(body.def_id) {\n",
    tests: ["test/corpus.test.ts", "-t", "^closure_converting_identity_read.rs$"],
  },
  {
    name: "converting-of-a-js-function-given-on",
    breaks:
      "`xs.iter().map(|x| f(x))` of a `dyn Fn(&u32)`, a reborrow its arrow converts, is `xs.map(f)`, which gives a JS function the index and the array (ADR 0279, ADR 0605, amended)",
    file: "src/lower/passed_on.rs",
    find: "        if callee != Callee::Own && !self.boxed_into_a_field(made_at) {\n",
    replace: "        if false {\n",
    tests: ["test/lowering-passed-on.test.ts", "-t", "only where the function would see and give what the closure does"],
  },
  {
    name: "converting-left-out",
    breaks:
      "a closure whose call leaves its last `None` out, `|n| tagged(n, None)`, is the function, so `[3, 4].map(tagged)` gives it the index for its tag (ADR 0605, amended)",
    file: "src/lower/passed_on.rs",
    find: "            ExprKind::Tuple { ref fields } if fields.last().is_some_and(|&last| left_out(self.tcx, thir, last)) => {\n                return None;\n            }\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^option_arguments.rs$"],
  },
  {
    name: "converting-unit-discarding",
    breaks:
      "a converting closure of `()` that discards what its call gives is the function, which gives it to a JS caller (ADR 0605, amended)",
    file: "src/lower/passed_on.rs",
    find: "        if sig.output().is_unit() && !call.1.ty.is_unit() {\n",
    replace: "        if false {\n",
    tests: ["test/lowering-passed-on.test.ts", "-t", "only converts what it passes on"],
  },
];
