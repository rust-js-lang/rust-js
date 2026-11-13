// Mutations of src/prepare/declared.rs, a function a `const` binds declared (ADRs 0882, 0972).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "module-function-mutable-declared",
    breaks: "a thread-local its module sets, a `let`, is declared a function, which the module's assignment would replace as it does a `let`, but which is hoisted",
    file: "src/prepare/declared.rs",
    find: "            && !constant.mutable\n            && matches!(&constant.value.kind, ExprKind::Function(function) if function.name.is_empty())\n",
    replace: "            && matches!(&constant.value.kind, ExprKind::Function(function) if function.name.is_empty())\n",
    tests: ["test/js-functions.test.ts", "-t", "a with_this thread-local is a function declaration"],
  },
  {
    name: "valued-arrow-declared",
    breaks: "a function held as a value, an `Rc`'s or a `Box`'s of a closure, is declared, `function commitHook(n) { .. }` (ADR 0882, amended)",
    file: "src/prepare/declared.rs",
    find: "        if value.facts.has(js::Known::Valued) {\n            continue;\n        }\n",
    replace: "",
    tests: ["test/lowering-arrows.test.ts", "-t", "a let of an Rc or a Box"],
  },
  {
    name: "let-closure-of-this-declared",
    breaks: "a closure that reads a class's `this` is a function, whose `this` is its own (ADR 0882, amended)",
    file: "src/prepare/declared.rs",
    find: "        let mut own = js::reads_this(body);\n",
    replace: "        let mut own = false;\n",
    tests: ["test/lowering-arrows.test.ts", "-t", "function declared"],
  },
  {
    name: "let-closure-of-a-value-declared",
    breaks: "`const double = (n) => n * 2` is `function double(n) { return n * 2; }` (ADR 0882, amended)",
    file: "src/prepare/declared.rs",
    find: "        if own || value_alone || shadows {\n",
    replace: "        if own || shadows {\n",
    tests: ["test/lowering-arrows.test.ts", "-t", "function declared"],
  },
  {
    name: "let-closure-async-unmarked",
    breaks: "an async closure a `let` binds is a function that isn't `async`, whose `await` JS refuses (ADR 0882, amended)",
    file: "src/prepare/declared.rs",
    find: "            is_async: matches!(value.kind, ExprKind::AsyncArrow(..)),\n",
    replace: "            is_async: false,\n",
    tests: ["test/lowering-arrows.test.ts", "-t", "function declared"],
  },
  {
    name: "function-stored-unnamed",
    breaks: "a `with_this` function stored as a property stays anonymous, `update.callback = function () { .. }`, not React's `function callback()` (ADR 0972)",
    file: "src/prepare/declared.rs",
    find: "            named_as(value, property);\n",
    replace: "",
    tests: ["test/js-functions.test.ts", "-t", "stored as a property is named"]
  },
  {
    name: "function-named-as-what-it-reads",
    breaks: "a function stored as `update.callback` is named `callback` where it reads a variable of that name, which its own name then is: `function callback() { return callback(); }` calls itself (a wrong answer, ADR 0972)",
    file: "src/prepare/declared.rs",
    find: "        && js::mentions_in(&function.body, property) == 0\n",
    replace: "",
    tests: ["test/js-functions.test.ts", "-t", "stored as a property is named"]
  },
  {
    name: "function-named-a-kept-word",
    breaks: "a function stored as `update.default` is named `function default()`, a `SyntaxError` (ADR 0972)",
    file: "src/prepare/declared.rs",
    find: "        && !crate::names::kept_word(property)\n",
    replace: "",
    tests: ["test/js-functions.test.ts", "-t", "stored as a property is named"]
  },
  {
    name: "function-named-no-name",
    breaks: "a function stored as `on[\"data-on\"]` is named `function data-on()`, a `SyntaxError` (ADR 0972)",
    file: "src/prepare/declared.rs",
    find: "        && crate::names::is_identifier(property)\n",
    replace: "",
    tests: ["test/js-functions.test.ts", "-t", "stored as a property is named"]
  },
  {
    name: "function-of-a-let-not-declared",
    breaks: "a `let` of a `with_this` function stays `const inner = function () { .. }`, not `function inner() { .. }` (ADR 0972)",
    file: "src/prepare/declared.rs",
    find: "            && matches!(&value.kind, ExprKind::Function(function) if function.name.is_empty())\n",
    replace: "            && matches!(&value.kind, ExprKind::Function(function) if function.name.is_empty() && false)\n",
    tests: ["test/js-functions.test.ts", "-t", "a function nested in a with_this function"]
  },
  {
    name: "self-shadowing-closure-declared",
    breaks: "a closure that declares its own name is declared, `function key(event) { const key = event.key; .. }`, which React Compiler refuses (#1702)",
    file: "src/prepare/declared.rs",
    find: "        if own || value_alone || shadows {\n",
    replace: "        if own || value_alone {\n",
    tests: ["test/lowering-arrows.test.ts", "-t", "closure of statements is a function declared"],
  },
];
