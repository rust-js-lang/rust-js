// Mutations of src/lower/std_impls/defaults.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "default-str-unknown",
    breaks: "`Default` of a `&str` is refused",
    file: "src/lower/std_impls/defaults.rs",
    find: '            ty::Ref(_, inner, _) if inner.is_str() => Expr::str(""),',
    replace:
      '            ty::Ref(_, inner, _) if inner.is_str() && false => Expr::str(""),',
    tests: ["test/corpus.test.ts", "-t", "std_odds"],
  },
  {
    name: "default-array-empty",
    breaks: "`Default` of a `[u16; 3]` is no items",
    file: "src/lower/std_impls/defaults.rs",
    find: "                Expr::array((0..len).map(|_| self.default_value(*item, span)).collect::<R<_>>()?)",
    replace:
      "                Expr::array((0..len).skip(len as usize).map(|_| self.default_value(*item, span)).collect::<R<_>>()?)",
    tests: ["test/corpus.test.ts", "-t", "std_odds"],
  },
  {
    name: "default-boxed-slice-unknown",
    breaks: "`Default` of a `Box<[bool]>` is refused",
    file: "src/lower/std_impls/defaults.rs",
    find: "            ty::Slice(_) => Expr::array(Vec::new()),",
    replace: "            ty::Slice(_) if false => Expr::array(Vec::new()),",
    tests: ["test/corpus.test.ts", "-t", "std_odds"],
  },
  {
    name: "element-default-null",
    breaks:
      "an \`Element\`'s default, children not given, is \`null\`, written as a child",
    file: "src/lower/std_impls/defaults.rs",
    find: "            return Ok(Expr::undefined());\n        }\n        // react's `Rest`: no props,",
    replace:
      "            return Ok(Expr::null());\n        }\n        // react's `Rest`: no props,",
    tests: [
      "test/jsx-props.test.ts",
      "-t",
      "named props and the rest from a base",
    ],
  },
  {
    name: "rest-default-object",
    breaks:
      "a `Rest`'s default is `{}`, which a Rust caller's struct update writes as a `rest` prop",
    file: "src/lower/std_impls/defaults.rs",
    find: "        if bindings::is_rest(self.tcx, ty) {\n            return Ok(Expr::undefined());",
    replace:
      "        if bindings::is_rest(self.tcx, ty) {\n            return Ok(Expr::object(Vec::new()));",
    tests: ["test/jsx-props.test.ts", "-t", "doesn't name as ...rest"],
  },
  {
    name: "default-cell-field-boxed",
    breaks:
      "a derived `Default`'s `Cell` field is `{ value: 0 }`, where it's `0`",
    file: "src/lower/std_impls/defaults.rs",
    find: "                                let value = self.holding(value, self.struct_field_def(ty, i), t);\n",
    replace: "",
    tests: [
      "test/lowering-cells.test.ts",
      "-t",
      "cell in a field is the property",
    ],
  },
  {
    name: "flattened-default-nested",
    breaks:
      "a derived `Default` of props with a flattened field nests it, `{ anchor: {} }`",
    file: "src/lower/std_impls/defaults.rs",
    find: "                    self.flattened_object(made, ty)\n",
    replace: "                    made\n",
    tests: [
      "test/jsx-props.test.ts",
      "-t",
      "JSX gives a flattened struct's fields as a component's own props",
    ],
  },
  {
    name: "lazy-default-made",
    breaks: "`LazyCell::default()` is made already, not on first use",
    file: "src/lower/std_impls/defaults.rs",
    find: '                let init = Expr::arrow(vec![], vec![StmtKind::Return(Some(made)).at(js::Span::NONE)]);\n                Expr::object(vec![Prop::Field("init".into(), init)])\n',
    replace:
      '                Expr::object(vec![Prop::Field("value".into(), made)])\n',
    tests: ["test/corpus.test.ts", "-t", "lazy_cells"],
  },
  {
    name: "default-bound-of-parameters-unread",
    breaks:
      "`Default::default()` of `[T; N]`, which a where clause gives a dictionary of, is refused, as only a type parameter's bound is read",
    file: "src/lower/std_impls/defaults.rs",
    find: "        // its dictionary's.\n        let tr = ty::TraitRef::new(self.tcx, default, [ty]);\n        if let Some(dictionary) = self.evidence_for(tr) {\n",
    replace:
      "        // its dictionary's.\n        let tr = ty::TraitRef::new(self.tcx, default, [ty]);\n        if let Some(dictionary) = self.evidence_for(tr).filter(|_| self.is_unknown(ty)) {\n",
    tests: ["test/corpus.test.ts", "-t", "^array_defaults.rs$"],
  },
  {
    name: "value-cell-default-boxed",
    breaks:
      "a `RefCell` of an object's `default` is a `{ value }`, read as the object it isn't",
    file: "src/lower/std_impls/defaults.rs",
    find: "            ty::Adt(_, args) if std(StdItem::RefCell) && self.is_value_cell(ty) => {\n                self.default_value(args.type_at(0), span)?\n            }\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^refcell_objects.rs$"],
  },
  {
    name: "transparent-default-unsupported",
    breaks:
      'a derived `Default` of a transparent struct, a `RefCell`\'s `take` of one, is refused, "`Default` of `Lines`" (ADR 0584)',
    file: "src/lower/std_impls/defaults.rs",
    find: "            ty::Adt(adt, args) if let Some(at) = recognition::transparent_field(self.tcx, *adt) => {\n                let field",
    replace:
      "            ty::Adt(adt, args) if false && let Some(at) = recognition::transparent_field(self.tcx, *adt) => {\n                let field",
    tests: ["test/corpus.test.ts", "-t", "^refcell_transparent.rs$"],
  },
  {
    name: "field-symbol-default-by-name",
    breaks: "a derived `Default`'s field a symbol names is keyed by the symbol item's name",
    file: "src/lower/std_impls/defaults.rs",
    find: "                                    Some((item, at)) => Prop::Keyed(self.field_symbol(item, at), value),",
    replace: "                                    Some((item, _)) => Prop::Field(self.tcx.item_name(item).to_string(), value),",
    tests: [
      "test/field-symbol-names.test.ts",
      "-t",
      "each field is its own symbol's"
    ]
  },
  {
    name: "range-default-refused",
    breaks:
      "`Default` of a `Range` or a `RangeFull`, a struct as any is, is refused as another std type's, \"`Default` of `std::ops::Range<i32>`\": `mem::take(&mut range)`, a derived `Default` holding one, one given where `T: Default` goes (#639)",
    file: "src/lower/std_impls/defaults.rs",
    find: "            ty::Adt(adt, _) if self.is_std(adt.did()) && self.range_kind(ty).is_none() => {\n",
    replace: "            ty::Adt(adt, _) if self.is_std(adt.did()) => {\n",
    tests: ["test/corpus.test.ts", "-t", "^range_defaults.rs$"],
  },
];
