// Mutations of src/lower/patterns.rs (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "numbered-gives-own-name",
    breaks:
      'a `match` giving each numbered variant its name is taken as the value itself, so `name(0)` is `0`, not `"Empty"`',
    file: "src/lower/patterns.rs",
    find: "        if !super::representation::is_string_enum(tcx, *adt) {\n",
    replace:
      "        if !adt.is_enum() || adt.variants().iter().any(|variant| !variant.fields.is_empty()) {\n",
    tests: [
      "test/lowering-structs-enums.test.ts",
      "-t",
      "a repr enum's variant is its number",
    ],
  },
  {
    name: "rest-props-as-field",
    breaks:
      "`LinkProps { href, rest }` is `{ href, rest }`, where JS's props have no `rest`, not `...rest`",
    file: "src/lower/patterns.rs",
    find: "                    match super::bindings::is_rest_field(self.tcx, pat.ty, i) {",
    replace:
      "                    match false && super::bindings::is_rest_field(self.tcx, pat.ty, i) {",
    tests: ["test/jsx-props.test.ts", "-t", "doesn't name as ...rest"],
  },
  {
    name: "unbound-field-in-rest",
    breaks:
      "a field the pattern leaves, `..`, is in the rest, `...props` holding `className`",
    file: "src/lower/patterns.rs",
    find: "or the rest would hold it (ADR 0205).\n                if rest.is_some() {",
    replace:
      "or the rest would hold it (ADR 0205).\n                if false && rest.is_some() {",
    tests: ["test/jsx-props.test.ts", "-t", "chain of flattened"],
  },
  {
    name: "prop-default-not-written",
    breaks:
      "a props field's default isn't written where JS takes them apart, `{ size }`, so a JS caller's missing one is undefined",
    file: "src/lower/patterns.rs",
    find: "                            let default = self.prop_default(pat.ty, i, pat.span);",
    replace: "                            let default = None;",
    tests: ["test/jsx-props.test.ts", "-t", "a props field"],
  },
  {
    name: "prop-default-string-ignored",
    breaks:
      'the literal `#[rust_js::default = ..]` says is its type\'s default instead: `"_self"` is `""`, `true` is `false`',
    file: "src/lower/patterns.rs",
    find: "                self.literal(&lit, false, field_ty, span).ok()?\n",
    replace: "                self.default_value(field_ty, span).ok()?\n",
    tests: ["test/jsx-props.test.ts", "-t", "a props field"],
  },
  {
    name: "prop-default-made",
    breaks:
      "a default that's made, `new Map()`, is written where the linker doesn't see it",
    file: "src/lower/patterns.rs",
    find: "        if !is_literal(&default) {",
    replace: "        if false {",
    tests: ["test/jsx-props.test.ts", "-t", "a props field"],
  },
  {
    name: "default-of-another-type",
    breaks:
      "a props field's default that isn't of its type, `= \"3\"` of a `u32`, is written as it is",
    file: "src/lower/patterns.rs",
    find: "                if !of_type {\n",
    replace: "                if false {\n",
    tests: ["test/jsx-props.test.ts", "-t", "literal default it says"],
  },
  {
    name: "default-bool-refused",
    breaks:
      "`#[rust_js::default = true]` of a `bool` is refused, where JS takes `isPageAnchor = true`",
    file: "src/lower/patterns.rs",
    find: "                    LitKind::Bool(_) => field_ty.is_bool(),\n",
    replace: "                    LitKind::Bool(_) => false,\n",
    tests: ["test/jsx-props.test.ts", "-t", "literal default it says"],
  },
  {
    name: "default-number-refused",
    breaks: "`#[rust_js::default = 3]` of a `u32` is refused",
    file: "src/lower/patterns.rs",
    find: "                    LitKind::Int(..) => Num::of(field_ty).is_some(),\n",
    replace: "                    LitKind::Int(..) => false,\n",
    tests: ["test/jsx-props.test.ts", "-t", "literal default it says"],
  },
  {
    name: "rest-keys-unknown",
    breaks:
      "a props pattern's rest isn't known as one, so an update of it, `..props`, gives each of its fields, the props' own `href` among them, and is refused",
    file: "src/lower/patterns.rs",
    find: "                    self.locals.rests.insert(rest.clone(), taken);\n",
    replace:
      "                    self.locals.rests.insert(String::new(), taken);\n",
    tests: [
      "test/jsx-props.test.ts",
      "-t",
      "flattened props with a field of them set",
    ],
  },
  {
    name: "default-const-unread",
    breaks:
      "a props field's `#[rust_js::default(NAME)]` is its type's Default, not the const it names",
    file: "src/lower/patterns.rs",
    find: "        if let Some(name) = bindings::field_default_const(self.tcx, field) {",
    replace:
      "        if let Some(name) = bindings::field_default_const(self.tcx, field).filter(|_| false) {",
    tests: ["test/jsx-props.test.ts", "-t", "default may be a const"],
  },
  {
    name: "other-name-call-dropped",
    breaks:
      'a function giving a variant another name, `Section::Blog => "news"`, is taken for its argument',
    file: "src/lower/patterns.rs",
    find: "if text.as_str() == super::bindings::variant_name(tcx, adt.variant(variant_index)))))",
    replace:
      "if true || text.as_str() == super::bindings::variant_name(tcx, adt.variant(variant_index)))))",
    tests: ["test/lowering-structs-enums.test.ts", "-t", "enum's own names"],
  },
  {
    name: "cell-field-destructured",
    breaks:
      "a parameter's struct of a `Cell` field taken apart is JS's destructuring, `{ count }`, the number, which `count.value = ..` can't set",
    file: "src/lower/patterns.rs",
    find: "                _ if matches!(self.shape(pat.ty), Shape::Object(_)) && self.is_std_type(field.ty, StdItem::Cell) => {\n                    None\n",
    replace: "                _ if false => {\n                    None\n",
    tests: [
      "test/lowering-matches.test.ts",
      "-t",
      "struct taken apart through a reference",
    ],
  },
  {
    name: "noinline-own-name-called",
    breaks:
      "a function giving each variant its name, marked `#[inline(never)]`, is its argument, uncalled",
    file: "src/lower/patterns.rs",
    find: " || super::recognition::is_noinline(tcx, def_id) {\n",
    replace: " {\n",
    tests: ["test/emission.test.ts", "-t", "inline\\(never\\)"],
  },
  {
    name: "newtype-from-called",
    breaks:
      "a transparent struct's `From` of its field is called, `eventPriorityFromI32_from(lane)`, not `lane`",
    file: "src/lower/patterns.rs",
    find: "        if converts {\n",
    replace: "        if false {\n",
    tests: ["test/corpus.test.ts", "-t", "^newtype_conversions.rs$"],
    snapshots: true,
  },
  {
    name: "newtype-from-of-anything",
    breaks:
      "a transparent struct's `From` that makes it of more than its parameter, `Doubled(lane * 2)`, is taken as the value it's given",
    file: "src/lower/patterns.rs",
    find: "[field] if field.name.as_usize() == at && given(field.expr))\n",
    replace: "[field] if field.name.as_usize() == at)\n",
    tests: ["test/corpus.test.ts", "-t", "^newtype_conversions.rs$"],
  },
];
