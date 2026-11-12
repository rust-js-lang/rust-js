// Mutations of src/lower/std_impls/defaults.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "random-state-default-refused",
    breaks: "a `RandomState`'s `Default`, `HashSet::with_hasher(Default::default())`'s and generic code's, is refused",
    file: "src/lower/std_impls/defaults.rs",
    find: "            ty::Adt(..) if std(StdItem::RandomState) => Expr::undefined(),\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^random_state_holds_nothing.rs$"],
  },
  {
    name: "build-hasher-default-refused",
    breaks: "`BuildHasherDefault::<DefaultHasher>::default()`, which holds nothing, is refused, so uuid's `test_compare` is (#1155)",
    file: "src/lower/std_impls/defaults.rs",
    find: "            ty::Adt(..) if std(StdItem::BuildHasherDefault) => Expr::undefined(),\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^build_hasher_default.rs$"],
  },
  {
    name: "default-never-panics",
    breaks: "a hand-written `Default` is taken to never panic, so a derived one's fields made before it leak as it does (#902)",
    file: "src/lower/std_impls/defaults.rs",
    find: "            _ if let Some(imp) = self.recognition().user_impl(tr) => self.impl_may_unwind(imp),\n",
    replace: "            _ if let Some(imp) = self.recognition().user_impl(tr) => false && self.impl_may_unwind(imp),\n",
    tests: ["test/corpus.test.ts", "-t", "^default_unwinds_what_it_made.rs$"],
  },
  {
    name: "default-of-struct-never-panics",
    breaks: "a derived `Default` of a struct is taken to never panic, as its fields' may, so a struct of them made in turn leaks its earlier ones (#902)",
    file: "src/lower/std_impls/defaults.rs",
    find: "                (adt.all_fields()).any(|f| self.default_may_panic_in(self.field_ty(f, args), seen))\n",
    replace: "                adt.all_fields().any(|_| false)\n",
    tests: ["test/corpus.test.ts", "-t", "^default_unwinds_what_it_made.rs$"],
  },
  {
    name: "default-of-vec-may-panic",
    breaks: "an empty collection's default, which makes no item, is taken to panic as its item's may, a guard where none is needed (#902)",
    file: "src/lower/std_impls/defaults.rs",
    find: "            _ => false,\n        };\n        seen.pop();\n",
    replace: "            ty::Adt(_, args) => args.types().any(|arg| self.default_may_panic_in(arg, seen)),\n            _ => false,\n        };\n        seen.pop();\n",
    tests: ["test/lowering-made-in-turn.test.ts"],
  },
  {
    name: "array-default-unguarded",
    breaks: "an array's `Default` whose item's `default()` panics leaks the items it made (#902)",
    file: "src/lower/std_impls/defaults.rs",
    find: "                let parts = vec![(*item, self.default_may_panic(*item)); len as usize];\n",
    replace: "                let parts = vec![(*item, false); len as usize];\n",
    tests: ["test/corpus.test.ts", "-t", "^default_unwinds_what_it_made.rs$"],
  },
  {
    name: "tuple-default-unguarded",
    breaks: "a tuple's `Default` whose part's `default()` panics leaks the parts it made (#902)",
    file: "src/lower/std_impls/defaults.rs",
    find: "                    let parts: Vec<_> = (tys.iter()).map(|&t| (t, self.default_may_panic(t))).collect();\n",
    replace: "                    let parts: Vec<_> = (tys.iter()).map(|&t| (t, false)).collect();\n",
    tests: ["test/corpus.test.ts", "-t", "^default_unwinds_what_it_made.rs$"],
  },
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
    find: "                Self::in_turn(Expr::array(items), made)\n            }\n            _ if ty.is_unit()",
    replace: "                Self::in_turn(Expr::array(items.into_iter().skip(len as usize).collect()), made)\n            }\n            _ if ty.is_unit()",
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
    find: "                            let value = self.holding(value, self.struct_field_def(ty, i), t);\n",
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
    find: "                    Self::in_turn(self.flattened_object(Expr::object(props), ty), made)\n",
    replace: "                    Self::in_turn(Expr::object(props), made)\n",
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
    find: "                                Some((item, at)) => Prop::Keyed(self.field_symbol(item, at), value),",
    replace: "                                Some((item, _)) => Prop::Field(self.tcx.item_name(item).to_string(), value),",
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
    find: "                if self.is_std(adt.did())\n                    && self.range_kind(ty).is_none()\n",
    replace: "                if self.is_std(adt.did())\n",
    tests: ["test/corpus.test.ts", "-t", "^range_defaults.rs$"],
  },
  {
    name: "c-string-default-unknown",
    breaks: "`CString::default()` is refused",
    file: "src/lower/std_impls/defaults.rs",
    find: "            _ if crate::lower::recognition::is_c_string_like(self.tcx, ty.peel_refs()) => {\n",
    replace: "            _ if false => {\n",
    tests: ["test/corpus.test.ts", "-t", "^c_strings.rs$"],
  },
  {
    name: "unsafe-cell-default-unboxed",
    breaks: "an `UnsafeCell`'s `Default` is the default of what it holds, not `{ value }`, so a write through its `get()` reads `undefined.value` (#1420)",
    file: "src/lower/std_impls/defaults.rs",
    find: "                if std(StdItem::Cell) || std(StdItem::UnsafeCell) || std(StdItem::RefCell) || std(StdItem::Atomic) =>\n",
    replace: "                if std(StdItem::Cell) || std(StdItem::RefCell) || std(StdItem::Atomic) =>\n",
    tests: ["test/corpus.test.ts", "-t", "^unsafe_cell_values.rs$"],
  },
  {
    name: "std-fields-struct-default-refused",
    breaks: "the derived `Default` of a `Saturating`, a `Wrapping` or a `Reverse` is refused, where it's `[0]` (#1434)",
    file: "src/lower/std_impls/defaults.rs",
    find: "                    && !(recognition::struct_is_its_fields(self.tcx, adt.did())\n",
    replace: "                    && !(false && recognition::struct_is_its_fields(self.tcx, adt.did())\n",
    tests: ["test/corpus.test.ts", "-t", "^saturating_ops_at_every_width.rs$"],
  },
];
