// Mutations of src/lower/classes.rs (ADR 0093): a JS class (ADR 0761).
import type { Mutation } from "../../mutations";

const file = "src/lower/classes.rs";
const base = ["test/classes.test.ts", "-t", "React's Component is a constructor function"];
const refusals = ["test/classes.test.ts", "-t", "what a class is refused"];

export const mutations: Mutation[] = [
  {
    name: "class-receiver-left-to-tidy",
    breaks: "a class's method's `self` stays a parameter through tidy, so it's no `this`, its `let point` read as another variable",
    file,
    find: "    if member(tcx, function) != Some(Member::Method) {\n",
    replace: "    if member(tcx, function) == Some(Member::Method) {\n",
    tests: ["test/corpus.test.ts", "-t", "^class_local_named_as_self.rs$"],
  },
  {
    name: "class-unmarked",
    breaks: "a struct marked `#[rust_js::class]` is no class: its impl an object of functions, `Component.new(..)`, which `class X extends Component` can't run",
    file,
    find: "    tcx.def_kind(adt) == DefKind::Struct && marked(tcx, adt)\n",
    replace: "    tcx.def_kind(adt) == DefKind::Struct && marked(tcx, adt) && false\n",
    tests: base,
  },
  {
    name: "class-method-a-static",
    breaks: "a class's `&self` method is the constructor's own, `Component.setState`, so `this.setState` of an instance is `undefined`",
    file,
    find: "        DefKind::AssocFn if tcx.associated_item(def_id).is_method() => Member::Method,\n",
    replace: "",
    tests: base,
  },
  {
    name: "class-self-not-this",
    breaks: "a method's `self` stays its parameter, read where JS gives its first argument, not `this`",
    file,
    find: "            e.kind = ExprKind::This;\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "class_methods"],
  },
  {
    name: "class-constructor-returns",
    breaks: "a constructor's early `return { .. }` sets `this`'s fields and goes on, so the rest overwrites them",
    file,
    find: "            if Some(i + 1) != last {\n",
    replace: "            if false {\n",
    tests: ["test/corpus.test.ts", "-t", "class_methods"],
  },
  {
    name: "class-const-screaming",
    breaks: "a class's `const IS_REACT_COMPONENT` is the prototype's `IS_REACT_COMPONENT`, not `isReactComponent`, which React reads",
    file,
    find: "            true => crate::lower::camel_case(&named.to_lowercase()),\n",
    replace: "            true => named,\n",
    tests: base,
  },
  {
    name: "class-trait-allowed",
    breaks: "a class's `#[derive(Clone)]` compiles, and its clone is a plain object, no instance of its prototype",
    file,
    find: "                    && !tcx.associated_item_def_ids(id).is_empty()\n",
    replace: "                    && false\n",
    tests: refusals,
  },
  {
    name: "class-literal-allowed",
    breaks: "a class's struct literal outside its constructor compiles, a plain object without its prototype's methods",
    file,
    find: "                && is_class(tcx, adt.adt_def.did())\n                && !(in_constructor",
    replace: "                && false\n                && !(in_constructor",
    tests: refusals,
  },
  {
    name: "class-by-value-allowed",
    breaks: "a class's method of `self` by value compiles, its receiver no reference",
    file,
    find: "                            if !matches!(receiver.kind(), ty::Ref(_, inner, _) if matches!(inner.kind(), ty::Adt(..))) {\n",
    replace: "                            if false {\n",
    tests: refusals,
  },
  {
    name: "class-field-method-clash-allowed",
    breaks: "a class's method named as its field compiles, and `this.count` hides `count()`",
    file,
    find: "                    if matches!(member(tcx, item), Some(Member::Method | Member::Const)) && fields.contains(&named) {\n",
    replace: "                    if false {\n",
    tests: refusals,
  },
  {
    name: "class-generic-allowed",
    breaks: "a generic class compiles, one prototype for each of its types",
    file,
    find: "                if tcx.generics_of(id).own_params.iter().any(|p| p.kind.is_ty_or_const()) {\n",
    replace: "                if false {\n",
    tests: refusals,
  },
  {
    name: "class-unmade-allowed",
    breaks: "a class of no `fn new` compiles, no constructor function to make it",
    file,
    find: "                if constructor_of(tcx, id).is_none() {\n",
    replace: "                if false {\n",
    tests: refusals,
  },
  {
    name: "class-impl-unmarked-allowed",
    breaks: "a class's impl not marked `#[rust_js::class]` compiles, the attribute on one half of the class",
    file,
    find: "                    (true, false) => refuse(span, \"struct's impl is marked `#[rust_js::class]` too\"),\n",
    replace: "",
    tests: refusals,
  },
];
