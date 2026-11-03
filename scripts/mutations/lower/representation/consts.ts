// Mutations of src/lower/representation/consts.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "nullable-const-field-undefined",
    breaks:
      "a constant's `#[rust_js::nullable]` field's `None` is left out, `{ count: 0 }`, React's `CacheContext` without its `_currentValue: null` (ADR 0586, amended)",
    file: "src/lower/representation/consts.rs",
    find: "                            let nullable = bindings::null_where_made(tcx, f)\n",
    replace:
      "                            let nullable = false && bindings::null_where_made(tcx, f)\n",
    tests: ["test/lowering-nullable-constants.test.ts"],
  },
  {
    name: "optional-const-field-null",
    breaks:
      "a constant's `#[rust_js::optional]` nullable field's `None` is `null`, not left out until it's set (ADR 0810)",
    file: "src/lower/representation/consts.rs",
    find: "                            let nullable = bindings::null_where_made(tcx, f)\n",
    replace:
      "                            let nullable = bindings::is_nullable(tcx, f)\n",
    tests: ["test/lowering-nullable-constants.test.ts"],
  },
  {
    name: "ref-cell-field-const-boxed",
    breaks:
      "a `const` of a struct whose `RefCell<Option<u8>>` field is the value it holds is `{ items: {} }`, the cell's box, which a read of the field takes for `Some` (ADR 0863, amended)",
    file: "src/lower/representation/consts.rs",
    find: "                    .any(|f| ref_cell(f) || bindings::named_by_symbol(tcx, f.did))\n",
    replace: "                    .any(|f| bindings::named_by_symbol(tcx, f.did))\n",
    tests: ["test/corpus.test.ts", "-t", "^ref_cell_field_const.rs$"],
  },
  {
    name: "tagged-constant-tag",
    breaks:
      'a constant of a discriminated union\'s variant is `{ TAG: "Fulfilled", value }`, its Rust name under `TAG`',
    file: "src/lower/representation/consts.rs",
    find: "                    crate::lower::bindings::tag_key(tcx, adt.did()),",
    replace: '                    "TAG".to_string(),',
    tests: ["test/lowering-structs-enums.test.ts", "-t", "discriminated union"],
  },
  {
    name: "named-unit-const-undefined",
    breaks:
      "a constant of a unit struct named `#[rust_js::name]` is `undefined`, not its string",
    file: "src/lower/representation/consts.rs",
    find: "                Some(CtorKind::Const) => Some(unit_name(tcx, adt.did()).map_or_else(Expr::undefined, Expr::str)),",
    replace:
      "                Some(CtorKind::Const) => Some(Expr::undefined()),",
    tests: ["test/lowering-structs-enums.test.ts", "-t", "named unit struct"],
  },
  {
    name: "constant-some-unboxed",
    breaks:
      "a constant `Some(None)` that rustc computed is `undefined`, and reads as `None`",
    file: "src/lower/representation/consts.rs",
    find: "                    Some(&inner) => const_js_by(tcx, inner, unit).map(crate::lower::std_types::option::some_literal),",
    replace:
      "                    Some(&inner) => const_js_by(tcx, inner, unit),",
    tests: ["test/corpus.test.ts", "-t", "nested_options"],
  },
  {
    name: "non-zero-const-one-level",
    breaks: "`NonZeroU8::MIN`, two levels of std's structs in, is an error",
    file: "src/lower/representation/consts.rs",
    find: "    while let ty::ValTreeKind::Branch(items) = &**value.valtree",
    replace: "    if let ty::ValTreeKind::Branch(items) = &**value.valtree",
    tests: ["test/corpus.test.ts", "-t", "nonzero"],
  },
  {
    name: "duration-const-seconds",
    breaks: "`Duration::MAX` has none of its nanoseconds",
    file: "src/lower/representation/consts.rs",
    find: "        return Some(num_literal(secs * 1_000_000_000 + nanos, Num::U128));",
    replace:
      "        return Some(num_literal(secs * 1_000_000_000 + 0 * nanos, Num::U128));",
    tests: ["test/corpus.test.ts", "-t", "duration"],
  },
  {
    name: "untagged-const-tagged",
    breaks: "a `const` of an untagged enum is a tagged object",
    file: "src/lower/representation/consts.rs",
    find: "            if crate::lower::bindings::is_untagged(tcx, adt.did()) {\n",
    replace: "            if false {\n",
    tests: ["test/corpus.test.ts", "-t", "untagged_enums"],
  },
  {
    name: "const-cell-field-boxed",
    breaks:
      "a constant's `Cell` field is `{ hits: { value: 0 } }`, so setting `COUNTER.hits` adds to an object",
    file: "src/lower/representation/consts.rs",
    find: "                    ty::Adt(adt, _) if is_std_def(tcx, adt.did(), StdItem::Cell) => Expr::held(js),\n",
    replace:
      "                    ty::Adt(adt, _) if is_std_def(tcx, adt.did(), StdItem::Cell) => js,\n",
    tests: ["test/corpus.test.ts", "-t", "marker_traits"],
  },
  {
    name: "const-range-from-refused",
    breaks: "a constant `(5..)` is refused, not `{ start: 5 }`",
    file: "src/lower/representation/consts.rs",
    find: "        ty::Adt(adt, _) if tcx.is_lang_item(adt.did(), LangItem::RangeFrom) => {\n",
    replace:
      "        ty::Adt(adt, _) if false && tcx.is_lang_item(adt.did(), LangItem::RangeFrom) => {\n",
    tests: ["test/corpus.test.ts", "-t", "range_values"],
  },
  {
    name: "const-range-to-refused",
    breaks: "a constant `(..8)` or `(..=7)` is refused, not `{ end: 8 }`",
    file: "src/lower/representation/consts.rs",
    find: "            if tcx.is_lang_item(adt.did(), LangItem::RangeTo)\n                || tcx.is_lang_item(adt.did(), LangItem::RangeToInclusive) =>\n",
    replace: "            if false =>\n",
    tests: ["test/corpus.test.ts", "-t", "range_values"],
  },
  {
    name: "discriminant-constant-refused",
    breaks: "a `Discriminant` constant is refused, not its number",
    file: "src/lower/representation/consts.rs",
    find: "    if crate::lower::recognition::is_discriminant_type(tcx, ty) {\n",
    replace: "    if false {\n",
    tests: ["test/corpus.test.ts", "-t", "^enum_discriminant.rs$"],
  },
  {
    name: "transparent-const-an-array",
    breaks:
      "a constant `#[repr(transparent)]` struct is `[field]`, `Uuid([9, 8, 7, 6])` unequal to the one made",
    file: "src/lower/representation/consts.rs",
    find: "        ty::Adt(adt, _) if let Some(at) = crate::lower::recognition::transparent_field(tcx, *adt) => {\n",
    replace:
      "        ty::Adt(adt, _) if let Some(at) = crate::lower::recognition::transparent_field(tcx, *adt).filter(|_| false) => {\n",
    tests: ["test/corpus.test.ts", "-t", "^transparent_transmute.rs$"],
  },
  {
    name: "registered-symbol-const-its-struct",
    breaks:
      'a `const` of the registry\'s symbol is its key\'s array, `["react.fragment"]`, not `Symbol.for("react.fragment")`',
    file: "src/lower/representation/consts.rs",
    find: "        return Some(symbol_for(const_js_by(tcx, key.try_to_value()?, unit)?));\n",
    replace:
      "        return Some(Expr::array(vec![const_js_by(tcx, key.try_to_value()?, unit)?]));\n",
    tests: [
      "test/lowering-symbols.test.ts",
      "-t",
      "a match of the registry's symbols",
    ],
  },
  {
    name: "const-of-a-js-value-its-markers",
    breaks:
      "a `const` of a binding's JS value, `HostType::new(\"aside\")`, is rustc's value of its markers, `[undefined, undefined]`, not its initializer's `\"aside\"` (ADR 0220, amended)",
    file: "src/lower/representation/consts.rs",
    find: "        // `\"Text\"` (ADR 0220, amended).\n        ty::Adt(adt, args) if marks_js_object(tcx, *adt, args) => None,\n",
    replace: "",
    tests: ["test/class-components.test.ts", "-t", "a tag that's a string"],
  },
  {
    name: "js-object-value-tree-said",
    breaks:
      "a static of react's `ElementType` is what rustc's memory says, `Icon: [undefined]`, not `Icon: IconBadge`",
    file: "src/lower/representation/consts.rs",
    find: "        // `\"Text\"` (ADR 0220, amended).\n        ty::Adt(adt, args) if marks_js_object(tcx, *adt, args) => None,\n",
    replace: "",
    tests: ["test/jsx-types.test.ts", "-t", "ElementType, rendered as a tag"],
  },
  {
    name: "field-symbol-constant-folded",
    breaks: "a constant struct a symbol names a field of is folded at compile time, its key named by text, which it hasn't",
    file: "src/lower/representation/consts.rs",
    find: "                    .any(|f| ref_cell(f) || bindings::named_by_symbol(tcx, f.did))",
    replace: "                    .any(ref_cell)",
    tests: [
      "test/field-symbol-names.test.ts",
      "-t",
      "a registry's symbol names a field"
    ]
  },
  {
    name: "keyed-const-unit-a-name",
    breaks: "a keyed enum's constant without fields is its name, a string (#504)",
    file: "src/lower/representation/consts.rs",
    find: "    let unit = |adt, variant: &ty::VariantDef| match keyed.contains(&adt) {\n",
    replace: "    let unit = |adt, variant: &ty::VariantDef| match false {\n",
    tests: ["test/corpus.test.ts", "-t", "^flattened_keyed_const.rs$"],
  },
  {
    name: "const-range-swapped",
    breaks: "a constant range's start is its end",
    file: "src/lower/representation/consts.rs",
    find: '                Prop::Field("start".into(), start?),\n',
    replace: '                Prop::Field("start".into(), end.clone()?),\n',
    tests: ["test/corpus.test.ts", "-t", "pretty_debug"],
  },
  {
    name: "value-cell-const-boxed",
    breaks:
      "a thread-local's `const { RefCell::new(Pair { .. }) }` is `{ value: { a, b } }`, which its borrows read as the struct it isn't",
    file: "src/lower/representation/consts.rs",
    find: "            if is_std_def(tcx, adt.did(), StdItem::RefCell)\n                && crate::lower::drops::holds_itself(tcx, args.type_at(0)) =>\n",
    replace: "            if false\n                && crate::lower::drops::holds_itself(tcx, args.type_at(0)) =>\n",
    tests: ["test/corpus.test.ts", "-t", "^refcell_const_object.rs$"],
  },
  {
    name: "keyed-const-keys-dropped",
    breaks: "a keyed enum's constant's `None` field is no key, so its variant isn't told (#504)",
    file: "src/lower/representation/consts.rs",
    find: "                    layout.kept = (props.iter())\n",
    replace: "                    layout.kept = (Vec::<Prop>::new().iter())\n",
    tests: ["test/corpus.test.ts", "-t", "^flattened_keyed_const.rs$"],
  },
];
