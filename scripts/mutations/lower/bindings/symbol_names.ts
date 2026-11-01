// Mutations of src/lower/bindings/symbol_names.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "field-symbol-any-use",
    breaks: "a field is named by the symbol any use beside its struct names, its struct's other field's: two fields of two symbols are one key",
    file: "src/lower/bindings/symbol_names.rs",
    find: "                    && adt.non_enum_variant().fields[index].did == self.field\n",
    replace: "",
    tests: [
      "test/field-symbol-names.test.ts",
      "-t",
      "each field is its own symbol's"
    ]
  },
  {
    name: "field-symbol-use-unread",
    breaks: "no field's symbol is read of the use rustc resolved: each symbol name is refused, as of a struct no use is written beside",
    file: "src/lower/bindings/symbol_names.rs",
    find: "                    self.found = Some((item, symbol.span));",
    replace: "                    self.found = None;",
    tests: [
      "test/field-symbol-names.test.ts",
      "-t",
      "a field a symbol names is that symbol's key"
    ]
  },
  {
    name: "field-symbol-item-allowed",
    breaks: "a symbol name on a function says nothing of what to write, as a field's alone may be named by one",
    file: "src/lower/bindings/symbol_names.rs",
    find: "            if tcx.def_kind(def_id) != DefKind::Field {",
    replace: "            if false {",
    tests: [
      "test/field-symbol-names.test.ts",
      "-t",
      "a symbol name where it can't be one is refused"
    ]
  },
  {
    name: "field-symbol-list-allowed",
    breaks: "a name of two symbols isn't refused for what it is, one path",
    file: "src/lower/bindings/symbol_names.rs",
    find: "            if !matches!(list.as_slice(), [one] if one.meta_item().is_some_and(|item| item.is_word())) {",
    replace: "            if false {",
    tests: [
      "test/field-symbol-names.test.ts",
      "-t",
      "a symbol name where it can't be one is refused"
    ]
  },
  {
    name: "field-symbol-variant-allowed",
    breaks: "an enum variant's field a symbol names isn't refused, its tagged object keyed by text alone",
    file: "src/lower/bindings/symbol_names.rs",
    find: "            if tcx.def_kind(parent) != DefKind::Struct {",
    replace: "            if false {",
    tests: [
      "test/field-symbol-names.test.ts",
      "-t",
      "a symbol name where it can't be one is refused"
    ]
  },
  {
    name: "field-symbol-unwritten-use-allowed",
    breaks: "a generic struct's or a function's struct's symbol name, which no use beside it resolves, isn't refused: its field is named as if by text",
    file: "src/lower/bindings/symbol_names.rs",
    find: "            if symbol_name(tcx, field).is_none() {",
    replace: "            if false {",
    tests: [
      "test/field-symbol-names.test.ts",
      "-t",
      "a symbol name where it can't be one is refused"
    ]
  },
  {
    name: "field-symbol-class-allowed",
    breaks: "a class's field a symbol names isn't refused, its constructor's `this.x` of text",
    file: "src/lower/bindings/symbol_names.rs",
    find: "            let refused = if crate::lower::classes::is_class(tcx, parent)\n                || crate::lower::classes::is_object(tcx, parent)\n            {",
    replace: "            let refused = if false {",
    tests: [
      "test/field-symbol-names.test.ts",
      "-t",
      "a symbol name where it can't be one is refused"
    ]
  },
  {
    name: "field-symbol-flatten-allowed",
    breaks: "a flattened field a symbol names isn't refused, though its fields are its parent's",
    file: "src/lower/bindings/symbol_names.rs",
    find: "            } else if super::is_flatten(tcx, field) {",
    replace: "            } else if false {",
    tests: [
      "test/field-symbol-names.test.ts",
      "-t",
      "a symbol name where it can't be one is refused"
    ]
  },
  {
    name: "field-symbol-test-member-allowed",
    breaks: "an untagged enum's `#[rust_js::test]` member a symbol names isn't refused, though its test reads it by name",
    file: "src/lower/bindings/symbol_names.rs",
    find: "            } else if super::is_test_field(tcx, def_id) {",
    replace: "            } else if false {",
    tests: [
      "test/field-symbol-names.test.ts",
      "-t",
      "a symbol name where it can't be one is refused"
    ]
  },
  {
    name: "field-symbol-props-allowed",
    breaks: "a component's props field a symbol names isn't refused, though JSX's attributes are text",
    file: "src/lower/bindings/symbol_names.rs",
    find: "            } else if props.contains(&parent) {",
    replace: "            } else if false {",
    tests: [
      "test/field-symbol-names.test.ts",
      "-t",
      "a symbol name where it can't be one is refused"
    ]
  },
  {
    name: "field-symbol-derives-allowed",
    breaks: "a struct that derives `Clone`, `PartialEq`, `Debug` or `Hash`, whose JS reads each field by name, may have a field a symbol names",
    file: "src/lower/bindings/symbol_names.rs",
    find: "        if !reads(trait_id) {",
    replace: "        if true {",
    tests: [
      "test/field-symbol-names.test.ts",
      "-t",
      "a symbol name where it can't be one is refused"
    ]
  },
  {
    name: "field-symbol-not-unique",
    breaks: "a symbol a field is named by is declared a `symbol`, of which TypeScript takes no computed key",
    file: "src/lower/bindings/symbol_names.rs",
    find: "        .any(|(named, _)| named == item)",
    replace: "        .any(|(named, _)| named == item && false)",
    tests: [
      "test/field-symbol-names.test.ts",
      "-t",
      "a registry's symbol names a field"
    ]
  },
];
