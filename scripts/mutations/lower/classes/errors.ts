// Mutations of src/lower/classes/errors.rs (ADR 0093): a class that extends
// JS's `Error`, its `message` and `cause` given `super` (#1808, ADR 1158).
import type { Mutation } from "../../../mutations";

const own = ["test/error-classes.test.ts", "-t", "JS's own subclass of Error"];
const refused = ["test/error-classes.test.ts", "-t", "refused of another shape"];

export const mutations: Mutation[] = [
  {
    name: "extends-error-unrecognized",
    breaks: "a class that implements `js::ErrorClass` is constructed as a component is, `super(data)`, its `message` the data object's text, `[object Object]`",
    file: "src/lower/classes/errors.rs",
    find: "    (superclass_impls(tcx, adt).first()).is_some_and(|&imp| is_js_error_class(tcx, tcx.impl_trait_id(imp)))\n",
    replace: "    (superclass_impls(tcx, adt).first()).is_some_and(|&imp| false && is_js_error_class(tcx, tcx.impl_trait_id(imp)))\n",
    tests: own,
  },
  {
    name: "error-message-set-after",
    breaks: "an `Error`'s `message` is set after `super()`, `this.message = data.message`, an enumerable property `JSON.stringify` and `Object.keys` list, where upstream's class has none",
    file: "src/lower/classes/errors.rs",
    find: "            ExprKind::Member(_, key) if key == MESSAGE => message = Some(value),\n",
    replace: "            ExprKind::Member(_, key) if false && key == MESSAGE => message = Some(value),\n",
    tests: own,
  },
  {
    name: "error-cause-of-no-field-given",
    breaks: "a class of no `cause` field is given `{ cause: undefined }`, an own `cause` that `new Error(message)` doesn't make",
    file: "src/lower/classes/errors.rs",
    find: "    if has_field(tcx, adt, CAUSE) {\n",
    replace: "    if true || has_field(tcx, adt, CAUSE) {\n",
    tests: own,
  },
  {
    name: "error-cause-field-unset-left-out",
    breaks: "a `cause` field of `None` is left out of `super`'s options, `{}`, so the instance has no `cause` where Rust's struct has the field",
    file: "src/lower/classes/errors.rs",
    find: "            layout.kept.push(CAUSE.to_owned());\n",
    replace: "            let _ = layout;\n",
    tests: own,
  },
  {
    name: "error-field-before-message-unheld",
    breaks: "a field written before `message` whose value runs code is set after `super`, so it runs after `message`'s value, where Rust runs it first: `code` 2 and `message` `x1`",
    file: "src/lower/classes/errors.rs",
    find: "            ExprKind::Member(_, key) if value.has_effects() => {\n",
    replace: "            ExprKind::Member(_, key) if false && value.has_effects() => {\n",
    tests: own,
  },
  {
    name: "error-message-any-type",
    breaks: "an `Error`'s `message` of a number compiles, made a string by `Error`'s constructor where Rust's field is a `u32`",
    file: "src/lower/classes/errors.rs",
    find: "        if !text {\n",
    replace: "        if false && !text {\n",
    tests: refused,
  },
  {
    name: "error-class-unconstructed-allowed",
    breaks: "a class that extends `Error` and writes no constructor compiles, which nothing constructs",
    file: "src/lower/classes/errors.rs",
    find: "    if constructor_of(tcx, adt).is_none() {\n",
    replace: "    if false && constructor_of(tcx, adt).is_none() {\n",
    tests: refused,
  },
];
