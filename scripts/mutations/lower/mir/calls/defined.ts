// Mutations of src/lower/mir/calls/defined.rs (ADR 0093): `js::is_defined`
// and `js::is_function` (ADRs 0597, 0942), and the paths of bindings they and
// `js::method` read.
import type { Mutation } from "../../../../mutations";

const isFunctionValue = [
  "test/lowering-is-function.test.ts",
  "-t",
  "value of a function's type",
];

export const mutations: Mutation[] = [
  {
    name: "defined-member-refused",
    breaks:
      "`js::is_defined(Clock::now)` of a method of a global's type is refused",
    file: "src/lower/mir/calls/defined.rs",
    find: "bindings::is_method(tcx, f) => self.member_of_global(f),\n",
    replace: "bindings::is_method(tcx, f) => None,\n",
    tests: [
      "test/lowering-bindings.test.ts",
      "test/lowering.test.ts",
      "-t",
      "js::is_defined",
    ],
  },
  {
    name: "defined-path-last-only",
    breaks:
      "`js::is_defined(tool_hook)` is `typeof rustJsTools.hook`, which throws where the host lacks `rustJsTools`",
    file: "src/lower/mir/calls/defined.rs",
    find: "            .reduce(|a, b| Expr::bin(Op::And, a, b).or_at(at)))\n",
    replace: "            .last())\n",
    tests: [
      "test/lowering-bindings.test.ts",
      "test/lowering.test.ts",
      "-t",
      "js::is_defined",
    ],
  },
  {
    name: "defined-member-of-either",
    breaks: "a method of a type two globals hold asks of the first",
    file: "src/lower/mir/calls/defined.rs",
    find: "        let [global] = statics.as_slice() else { return None };\n",
    replace:
      "        let [global, ..] = statics.as_slice() else { return None };\n",
    tests: [
      "test/lowering-bindings.test.ts",
      "test/lowering.test.ts",
      "-t",
      "js::is_defined",
    ],
  },
  {
    name: "defined-of-own-function",
    breaks:
      "`js::is_defined(helper)` of the crate's own function is a `typeof` of it, which is always there",
    file: "src/lower/mir/calls/defined.rs",
    find: "            ty::FnDef(f, _) if bindings::is_binding(tcx, f) => bindings::js_path(tcx, f),\n",
    replace: "            ty::FnDef(f, _) => bindings::js_path(tcx, f),\n",
    tests: [
      "test/lowering-bindings.test.ts",
      "test/lowering.test.ts",
      "-t",
      "js::is_defined",
    ],
  },
  {
    name: "defined-of-any-name",
    breaks: "`js::is_defined(|| 1)` is a `typeof` of no global, not refused",
    file: "src/lower/mir/calls/defined.rs",
    find: "            crate::names::is_identifier(first).then(|| {\n",
    replace: "            true.then(|| {\n",
    tests: [
      "test/lowering-bindings.test.ts",
      "test/lowering.test.ts",
      "-t",
      "js::is_defined",
    ],
  },
  {
    name: "defined-static-unfollowed",
    breaks:
      "`js::is_defined(&js::symbol::iterator)`, a reference to a static, is refused as no global's, not as the static's other spelling",
    file: "src/lower/mir/calls/defined.rs",
    find: "                if matches!(place.projection.as_slice(), [PlaceElem::Deref]) =>\n",
    replace:
      "                if false && matches!(place.projection.as_slice(), [PlaceElem::Deref]) =>\n",
    tests: [
      "test/lowering-bindings.test.ts",
      "test/lowering.test.ts",
      "-t",
      "js::is_defined",
    ],
  },
  {
    name: "defined-static-value-unread",
    breaks:
      "`js::is_defined(js::symbol::to_string_tag)`, a static given as itself, is refused",
    file: "src/lower/mir/calls/defined.rs",
    find: "                [PlaceElem::Deref] => return self.static_given(body, &Operand::Copy(Place::from(place.local))),\n",
    replace: "",
    tests: [
      "test/lowering-bindings.test.ts",
      "test/lowering.test.ts",
      "-t",
      "js::is_defined",
    ],
  },
  {
    name: "defined-static-reference-accepted",
    breaks:
      "`js::is_defined(&js::symbol::iterator)` is accepted beside the static itself, two spellings",
    file: "src/lower/mir/calls/defined.rs",
    find: "                Some(s) if refs(given) > refs(tcx.type_of(s).instantiate_identity().skip_normalization()) => {\n",
    replace:
      "                Some(s) if false && refs(given) > refs(tcx.type_of(s).instantiate_identity().skip_normalization()) => {\n",
    tests: [
      "test/lowering-bindings.test.ts",
      "test/lowering.test.ts",
      "-t",
      "js::is_defined",
    ],
  },
  {
    name: "defined-of-own-static",
    breaks:
      "`js::is_defined(LIMIT)` of the crate's own static is a `typeof` of it, which is there",
    file: "src/lower/mir/calls/defined.rs",
    find: "            _ => match self.static_given(body, &arg.node).filter(|&s| tcx.is_foreign_item(s)) {\n",
    replace: "            _ => match self.static_given(body, &arg.node) {\n",
    tests: [
      "test/lowering-bindings.test.ts",
      "test/lowering.test.ts",
      "-t",
      "js::is_defined",
    ],
  },
  {
    name: "defined-path-unread",
    breaks:
      "`js::is_defined((Page::doc, Doc::make))`, a path of bindings, is refused as no global's binding",
    file: "src/lower/mir/calls/defined.rs",
    find: "            && let ty::Tuple(steps) = *arg.node.ty(&body.local_decls, tcx).kind()\n            && !steps.is_empty()\n",
    replace:
      "            && let ty::Tuple(steps) = *arg.node.ty(&body.local_decls, tcx).kind()\n            && steps.is_empty()\n",
    tests: [
      "test/lowering-bindings.test.ts",
      "-t",
      "js::is_defined of a path of bindings",
    ],
  },
  {
    name: "defined-path-step-unchecked",
    breaks:
      "a path whose step is no method of what the one before gives, `(Doc::make, Page::doc)`, is asked as if it were",
    file: "src/lower/mir/calls/defined.rs",
    find: "                    if !bindings::is_method(tcx, f) || (of != Some(before.peel_refs()) && !any && !extends) {\n",
    replace: "                    if !bindings::is_method(tcx, f) {\n",
    tests: [
      "test/lowering-bindings.test.ts",
      "-t",
      "js::is_defined of a path of bindings",
    ],
  },
  {
    name: "math-defined-refused",
    breaks:
      "`js::is_defined(u32::leading_zeros)` is refused as no global's binding, where it's `typeof Math.clz32 !== \"undefined\"`",
    file: "src/lower/mir/calls/defined.rs",
    find: "            && let Some(function) = self.math_function(f, f_args)\n",
    replace:
      "            && let Some(function) = self.math_function(f, f_args).filter(|_| false)\n",
    tests: [
      "test/lowering-math.test.ts",
      "-t",
      "a std function that's Math's is its function, and is_defined asks of it",
    ],
  },
  {
    name: "defined-of-a-windows-constructor-refused",
    breaks:
      "`js::is_defined(Window::new_error_event)` is refused, so React's `reportGlobalError` can't ask `typeof window.ErrorEvent` (ADR 0891)",
    file: "src/lower/mir/calls/defined.rs",
    find: "        bindings::JsForm::Call(name) | bindings::JsForm::Get(name) | bindings::JsForm::New(name) => Some(name),\n",
    replace:
      "        bindings::JsForm::Call(name) | bindings::JsForm::Get(name) => Some(name),\n",
    tests: [
      "test/webapi-bindings.test.ts",
      "-t",
      "a window's constructor is the window's",
    ],
  },
  {
    name: "defined-path-read-past-a-constructor",
    breaks:
      "`js::is_defined((Window::new_error_event, ErrorEvent::message))` asks `window.ErrorEvent.message`, a property of the class, not of what it makes (ADR 0891)",
    file: "src/lower/mir/calls/defined.rs",
    find: "            if !bindings::is_binding(tcx, f) || (constructs && k + 1 < steps.len()) {\n",
    replace:
      "            if !bindings::is_binding(tcx, f) || (constructs && k + 1 < steps.len() && false) {\n",
    tests: [
      "test/webapi-bindings.test.ts",
      "-t",
      "a window's constructor is the window's",
    ],
  },
  {
    name: "is-function-asks-defined",
    breaks:
      '`js::is_function(callable)` is `typeof rustJsCallable !== "undefined"`, true of a number (ADR 0942)',
    file: "src/lower/mir/calls/defined.rs",
    find: "                Some(kind) if k + 1 == parts.len() => tests.push(Expr::bin(Op::Eq, typeof_, Expr::str(kind))),\n",
    replace: "                Some(_) if false => {}\n",
    tests: ["test/lowering-is-function.test.ts"],
  },
  {
    name: "is-function-asks-each-part",
    breaks:
      "`js::is_function(Performance::now)` asks its last part alone, `typeof performance.now === \"function\"`, which throws a `ReferenceError` where the host hasn't `performance`, where Rust is told `false` (rd320, ADR 0942 amended)",
    file: "src/lower/mir/calls/defined.rs",
    find: "                _ if k + 1 == parts.len() || from_value || !(k == 0 && every_host_has(first)) => {\n",
    replace:
      "                _ if asked.type_of().is_some() => {}\n                _ if k + 1 == parts.len() || from_value || !(k == 0 && every_host_has(first)) => {\n",
    tests: [
      "test/lowering-is-function.test.ts",
      "-t",
      "is whether a binding's global is a function",
    ],
  },
  {
    name: "fn-pointer-step-unnamed",
    breaks:
      "`js::is_function((style, CSSStyleDeclaration::set_property as fn(..)))`, a method no turbofish names, is refused as a step of no method (rd154)",
    file: "src/lower/mir/calls/defined.rs",
    find: "                ty::FnPtr(..) => cast_from(k).unwrap_or(step),\n",
    replace:
      "                ty::FnPtr(..) if false => cast_from(k).unwrap_or(step),\n",
    tests: ["test/lowering-is-function.test.ts", "-t", "path from a value"],
  },
  {
    name: "fn-pointer-step-copy-unfollowed",
    breaks:
      "a step's `fn` pointer MIR copies, `_5 = copy _6`, isn't followed to its cast, so hideInstance's test is refused (rd154)",
    file: "src/lower/mir/calls/defined.rs",
    find: "                    Rvalue::Use(copied, _) => item = copied,\n",
    replace: "                    Rvalue::Use(..) => return None,\n",
    tests: ["test/lowering-is-function.test.ts", "-t", "path from a value"],
  },
  {
    name: "is-number-of-a-function-asked",
    breaks:
      '`js::is_number(callable)`, a function\'s binding, is `typeof rustJsCallable === "number"`, always false, not refused (rd195)',
    file: "src/lower/mir/calls/defined.rs",
    find: '            Asked::Number => (number(read) || payload.is_some_and(number), "a number"),\n',
    replace: '            Asked::Number => (true, "a number"),\n',
    tests: [
      "test/lowering-is-function.test.ts",
      "-t",
      "is_number of what's no number",
    ],
  },
  {
    name: "is-number-of-an-option-refused",
    breaks:
      "`js::is_number(RATIO)` of an `Option<f64>` static is refused, though it may be a number (rd195)",
    file: "src/lower/mir/calls/defined.rs",
    find: '            Asked::Number => (number(read) || payload.is_some_and(number), "a number"),\n',
    replace: '            Asked::Number => (number(read), "a number"),\n',
    tests: ["test/lowering-is-function.test.ts", "-t", "is_number is whether"],
  },
  {
    name: "is-number-of-a-getter-refused",
    breaks:
      "`js::is_number(width)` of a getter's binding asks of the function, no number, so it's refused (rd195)",
    file: "src/lower/mir/calls/defined.rs",
    find: "        bindings::JsForm::Get(_) => tcx\n            .fn_sig(f)\n",
    replace:
      "        bindings::JsForm::Get(_) if false => tcx\n            .fn_sig(f)\n",
    tests: ["test/lowering-is-function.test.ts", "-t", "is_number is whether"],
  },
  {
    name: "every-host-has-asked",
    breaks:
      "a path through ECMAScript's `Object` asks `typeof Object !== \"undefined\"`, where React writes `typeof Object.preventExtensions === 'function'` (ADR 0942)",
    file: "src/lower/mir/calls/defined.rs",
    find: "                _ if k + 1 == parts.len() || from_value || !(k == 0 && every_host_has(first)) => {\n",
    replace: "                _ if true => {\n",
    tests: [
      "test/lowering-is-function.test.ts",
      "test/lowering-math.test.ts",
      "-t",
      "is_defined asks|is_function is whether",
    ],
  },
  {
    name: "every-host-has-unasked-whole",
    breaks:
      "`js::is_defined(js::date::new)` asks nothing of `Date`, so it's a call of the binding, not a test (ADR 0942)",
    file: "src/lower/mir/calls/defined.rs",
    find: "                _ if k + 1 == parts.len() || from_value || !(k == 0 && every_host_has(first)) => {\n",
    replace: "                _ if from_value || !(k == 0 && every_host_has(first)) => {\n",
    tests: [
      "test/lowering-is-function.test.ts",
      "-t",
      "is_function is whether",
    ],
  },
  {
    name: "symbol-for-not-a-global",
    breaks:
      "`js::is_function(js::SymbolFor)` is refused, as of no global's binding, so React's `typeof Symbol === 'function' && Symbol.for` can't be written (ADR 0964)",
    file: "src/lower/mir/calls/defined.rs",
    find: "            ty::FnDef(f, _) if crate::lower::recognition::is_symbol_for_constructor(tcx, f) => {\n",
    replace:
      "            ty::FnDef(f, _) if false && crate::lower::recognition::is_symbol_for_constructor(tcx, f) => {\n",
    tests: [
      "test/lowering-is-function.test.ts",
      "-t",
      "js::SymbolFor is Symbol.for",
    ],
  },
  {
    name: "defined-test-unplaced",
    breaks:
      "`js::is_function(..)`'s test has no place of its own, so a comment written before it is lost, React's `// In DEV we also rely on performance.measure`",
    file: "src/lower/mir/calls/defined.rs",
    find: "            .map(|test| test.or_at(at))\n",
    replace: "",
    tests: [
      "test/lowering-is-function.test.ts",
      "-t",
      "a thread-local bool of another",
    ],
  },
  {
    name: "is-function-of-no-value",
    breaks:
      "SchedulerMock's `js::is_function(&callback)` is refused as of no global's binding, so nothing can write `typeof callback === 'function'`",
    file: "src/lower/mir/calls/defined.rs",
    find: "        if self.asks_of_value(body, def_id, args) {\n",
    replace: "        if false && self.asks_of_value(body, def_id, args) {\n",
    tests: isFunctionValue,
  },
  {
    name: "is-function-value-of-an-option-refused",
    breaks:
      "`js::is_function(&Option<Callback>)` is refused: an `Option` of a function isn't one JS may give",
    file: "src/lower/mir/calls/defined.rs",
    find: "            return self.option_of(payload.peel_refs()).is_none() && self.function_given(payload);\n",
    replace: "            return false;\n",
    tests: isFunctionValue,
  },
  {
    name: "is-function-value-of-a-transparent-struct-refused",
    breaks:
      "SchedulerMock's `Callback(Box<dyn FnMut ..>)`, a transparent struct of a function, isn't one JS may give, so its `js::is_function` is refused",
    file: "src/lower/mir/calls/defined.rs",
    find: "            ty::Adt(adt, args) => crate::lower::recognition::transparent_field(tcx, adt).is_some_and(|i| {\n",
    replace:
      "            ty::Adt(adt, args) => crate::lower::recognition::transparent_field(tcx, adt).is_some_and(|i| false && {\n",
    tests: isFunctionValue,
  },
  {
    name: "is-function-value-of-the-crates-function",
    breaks:
      '`js::is_function(helper)` of the crate\'s function, or of a closure, which only Rust makes, is `typeof helper === "function"`, always true, not refused',
    file: "src/lower/mir/calls/defined.rs",
    find: "            ty::Closure(..) | ty::FnDef(..) => false,\n",
    replace: "            ty::Closure(..) | ty::FnDef(..) => true,\n",
    tests: ["test/lowering-is-function.test.ts", "-t", "refused"],
  },
  {
    name: "value-path-refused",
    breaks:
      "ReactDOMSelection's `js::is_function((selection, Selection::extend))` is refused as a path whose first step is no binding, so `typeof selection.extend` can't be written",
    file: "src/lower/mir/calls/defined.rs",
    find: "    (steps.len() > 1 && crate::lower::fn_def(first).is_none()).then_some(first)\n",
    replace:
      "    (false && steps.len() > 1 && crate::lower::fn_def(first).is_none()).then_some(first)\n",
    tests: ["test/lowering-is-function.test.ts", "-t", "path from a value"],
  },
  {
    name: "value-path-member-unread",
    breaks:
      '`js::is_function((selection, Selection::extend))` is `typeof selection === "function"`, the value asked, not its member',
    file: "src/lower/mir/calls/defined.rs",
    find: "            if k > 0 || from_value {\n",
    replace: "            if k > 0 {\n",
    tests: ["test/lowering-is-function.test.ts", "-t", "path from a value"],
  },
  {
    name: "method-path-any-value-refused",
    breaks:
      "`js::method((js::class::<HTMLElement>().prototype(), HTMLElement::focus))`, a path from any value, is refused as a step of no method of it (rd181)",
    file: "src/lower/mir/calls/defined.rs",
    find: "(of != Some(before.peel_refs()) && !any && !extends)",
    replace: "(of != Some(before.peel_refs()) && !extends)",
    tests: ["test/lowering-is-function.test.ts", "-t", "js::method"],
  },
  {
    name: "path-step-superclass-refused",
    breaks:
      "`js::is_function((style, CSSStyleDeclaration::set_property as fn(..)))` of an element's `CSSStyleProperties`, which extends it, is refused (rd232)",
    file: "src/lower/mir/calls/defined.rs",
    find: "&& !any && !extends)",
    replace: "&& !any)",
    tests: ["test/lowering-is-function.test.ts", "-t", "path from a value"],
  },
  {
    name: "path-value-held-refused",
    breaks:
      "`js::method((element, HTMLElement::focus))` while a guard with a drop is live, its tuple held before the guard's `try`, is refused (rd233)",
    file: "src/lower/mir/calls/defined.rs",
    find: "            js::ExprKind::Var(_) => {\n",
    replace: "            js::ExprKind::Var(_) if false => {\n",
    tests: ["test/lowering-is-function.test.ts", "-t", "js::method"],
  },
  {
    name: "is-function-of-a-class-refused",
    breaks:
      "`js::is_function(js::class::<Promise<()>>)`, React's `typeof Promise === 'function'`, is refused (rd235)",
    file: "src/lower/mir/calls/defined.rs",
    find: "                    .and_then(|ty| self.recognition().class_value(ty))\n            }\n",
    replace: "                    .and_then(|_| None)\n            }\n",
    tests: [
      "test/lowering-is-function.test.ts",
      "-t",
      "a class held as a value",
    ],
  },
  {
    name: "asked-quoted-key-named",
    breaks:
      "`js::is_number` of `window['$RN']` asks `typeof window.$RN`, which Closure may rename",
    file: "src/lower/mir/calls/defined.rs",
    find: "                read = part.of(read);\n",
    replace:
      "                read = match part {\n                    bindings::Step::Named(name) | bindings::Step::Quoted(name) => Expr::member(read, name),\n                };\n",
    tests: ["test/lowering-binding-paths.test.ts"],
  },
  {
    name: "asked-object-any-kind",
    breaks:
      "`js::is_object` of a number's binding, always false, compiles, not refused",
    file: "src/lower/mir/calls/defined.rs",
    find: '                (object(payload.unwrap_or(read)), "an object")\n',
    replace:
      '                (true || object(payload.unwrap_or(read)), "an object")\n',
    tests: [
      "test/lowering-is-function.test.ts",
      "-t",
      "js::is_object is whether",
    ],
  },
];
