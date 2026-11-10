// Mutations of src/lower/std_impls.rs (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "generic-eq-unrouted",
    breaks: "`a == b` of a `T: Ord` finds no `PartialEq` through its `Ord`, rustc's supertrait route, where Rust calls its `eq`, a derived one by its fields (#944)",
    file: "src/lower/std_impls.rs",
    find: "            // Of a `T: Ord` or `T: PartialOrd` alone, its `PartialEq`, a\n            // supertrait's, `TOrd.PartialOrd().PartialEq()`: never `cmp`,\n            // which Rust doesn't tie to it (#944).\n            if let Some(dictionary) = self.evidence_for(tr) {\n",
    replace: "            if let Some(dictionary) = self.evidence_for(tr).filter(|_| !self.has_evidence(ty::TraitRef::new(self.tcx, self.ord_trait(), [ty]))) {\n",
    tests: ["test/corpus.test.ts", "-t", "^generic_compare_by_impls.rs$"],
  },
  {
    name: "text-pattern-constant-unnamed",
    breaks:
      "a `match` of `&str` constants tests their text, `case \"fulfilled\":`, not the constant the program wrote, `case INITIALIZED:` (ADR 0590, amended)",
    file: "src/lower/std_impls.rs",
    find: "            if let Some(named) = self.named_text(&b, span)? {\n",
    replace: "            if let Some(named) = None::<Expr> {\n",
    tests: ["test/corpus.test.ts", "-t", "^str_const_patterns.rs$"],
    snapshots: true,
  },
  {
    name: "eq-items-fixed-names",
    breaks:
      "`==` of two `Vec`s of other item types names its arrow's `x` and `i` as they are, hiding a variable so named and a nested one's outer index (#307)",
    file: "src/lower/std_impls.rs",
    find: '        let (x, i) = (self.fresh("x"), self.fresh("i"));\n',
    replace: '        let (x, i) = ("x".to_string(), "i".to_string());\n',
    tests: ["test/corpus.test.ts", "-t", "^eq_items_of_other_types.rs$"],
  },
  {
    name: "numbered-eq-structural",
    breaks:
      "`==` of two numbered enums is `$eq(a, b)`, where they're numbers JS's `===` compares",
    file: "src/lower/std_impls.rs",
    find: " || super::bindings::numbered(self.tcx, *adt))\n",
    replace: ")\n",
    tests: ["test/corpus.test.ts", "-t", "^repr_enum_numbers.rs$"],
    snapshots: true,
  },
  {
    name: "untagged-undefined-loosely-null",
    breaks:
      "an untagged enum's `undefined` is `==` its `null`, as an `Option`'s `None` is",
    file: "src/lower/std_impls.rs",
    find: "            let none = !nullish && [&a, &b]",
    replace: "            let none = [&a, &b]",
    tests: ["test/corpus.test.ts", "-t", "untagged_null_undefined"],
  },
  {
    name: "unknown-eq-refused",
    breaks: "`a == b` of two `js::Unknown`s is refused",
    file: "src/lower/std_impls.rs",
    find: "        let primitive = any\n",
    replace: "        let primitive = false\n",
    tests: ["test/lowering-bindings.test.ts", "-t", "js::Unknown's == is ==="],
  },
  {
    name: "unknown-option-eq-loose",
    breaks: '`Option<&Unknown>`\'s `==` is `==`, which makes `"1"` of `1`',
    file: "src/lower/std_impls.rs",
    find: "                    true => js::nullish_equal(a, b),\n",
    replace: "                    true => Expr::bin(Op::LooseEq, a, b),\n",
    tests: ["test/lowering-bindings.test.ts", "-t", "js::Unknown's == is ==="],
  },
  {
    name: "untagged-unit-eq-any",
    breaks: "a variant without fields equals any value of the enum",
    file: "src/lower/std_impls.rs",
    find: "                        let both = self.untagged_variant_test(ty, &[variant], &[], &b, false);",
    replace: "                        let both = Expr::bool(true);",
    tests: [
      "test/lowering-structs-enums.test.ts",
      "-t",
      "variant without fields is its name",
    ],
  },
  {
    name: "eq-without-cells",
    breaks:
      "`o == Some(&mut 7)` of an `Option<&mut i32>` compares the cells, not what they point at",
    file: "src/lower/std_impls.rs",
    find: "        let (mut a, _) = self.through_refs(a, ty);\n        let (mut b, mut ty) = self.through_refs(b, ty);\n",
    replace: "        let (mut a, mut b, mut ty) = (a, b, ty.peel_refs());\n",
    tests: ["test/corpus.test.ts", "-t", "mut_ref_compare"],
  },
  {
    name: "primitive-eq-on-cells",
    breaks: "`==` of what holds `&mut`s to numbers is JS's `===` of the cells",
    file: "src/lower/std_impls.rs",
    find: "    pub(super) fn js_eq(&self, ty: Ty<'tcx>) -> Option<JsEq> {\n        // A `&mut` to a number is a cell, an object (ADR 0099).\n        if self.has_cell_layer(ty) {",
    replace:
      "    pub(super) fn js_eq(&self, ty: Ty<'tcx>) -> Option<JsEq> {\n        // A `&mut` to a number is a cell, an object (ADR 0099).\n        if false && self.has_cell_layer(ty) {",
    tests: ["test/corpus.test.ts", "-t", "mut_ref_compare"],
  },
  {
    name: "range-eq-unknown",
    breaks: "`(1..4) == (1..4)` is refused",
    file: "src/lower/std_impls.rs",
    find: "        if self.range_kind(ty).is_some() {\n            return self.range_eq(a, b, ty, span, out);\n",
    replace:
      "        if false && self.range_kind(ty).is_some() {\n            return self.range_eq(a, b, ty, span, out);\n",
    tests: ["test/corpus.test.ts", "-t", "range_values"],
  },
  {
    name: "wrapping-eq-refused",
    breaks: "`==` of `std::num::Wrapping`s is an error",
    file: "src/lower/std_impls.rs",
    find: "            || self.recognition().wrapping_of(ty).is_some()\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "wrapping_type"],
  },
  {
    name: "given-eq-of-parametrized-ignored",
    breaks:
      "`==` of a `Wrapping<T>` asks `T`'s `PartialEq`, not the one its bound gives",
    file: "src/lower/std_impls.rs",
    find: "        if ty.has_param() && !self.is_unknown(ty) {",
    replace: "        if false {",
    tests: ["test/corpus.test.ts", "-t", "trait_default_bounds"],
  },
  {
    name: "untagged-eq-tagged",
    breaks: "`==` of an untagged enum compares tags it hasn't",
    file: "src/lower/std_impls.rs",
    find: "            ty::Adt(adt, args) if self.untagged(ty).is_some() => {\n                let mut value = Expr::bool(false);\n",
    replace:
      "            ty::Adt(adt, args) if false => {\n                let mut value = Expr::bool(false);\n",
    tests: ["test/corpus.test.ts", "-t", "untagged_enums"],
  },
  {
    name: "inner-eq-unseen",
    breaks:
      "a derived `==` of a struct holding a type with a hand-written `eq` compares field by field with `$eq`, never calling that `eq`",
    file: "src/lower/std_impls.rs",
    find: "            ty::Adt(adt, args) => adt\n                .all_fields()\n                .any(|f| self.custom_eq_in(self.field_ty(f, args), seen)),",
    replace: "            ty::Adt(..) => false,",
    tests: [
      "test/compiler.test.ts",
      "-t",
      "== calls a hand-written eq wherever it's inside, and generics take a dictionary",
    ],
  },
  {
    name: "eq-cell-field-unhandled",
    breaks:
      "a derived `==` beside a hand-written one compares a `Cell` field's `value` of the number it holds, so any two are equal",
    file: "src/lower/std_impls.rs",
    find: "                            let x = self.held(Expr::member(a.clone(), name.clone()), def, t);\n",
    replace:
      "                            let x = Expr::member(a.clone(), name.clone());\n",
    tests: [
      "test/lowering-cells.test.ts",
      "-t",
      "cell in a field is the property",
    ],
  },
  {
    name: "rc-eq-counts",
    breaks: "two `Rc`s of equal values compare their counts too",
    file: "src/lower/std_impls.rs",
    find: "            ty::Adt(..) if self.counted_rc(ty).is_some() => true,\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "rc_counted_uses"],
  },
  {
    name: "refcell-eq-unheld",
    breaks:
      "`==` of `RefCell`s of what may ask runs with them free to borrow mutably",
    file: "src/lower/std_impls.rs",
    find: "            ty::Adt(_, args) if std(StdItem::RefCell) && !self.recognition().std_alone(args.type_at(0)) => {\n                let itself = self.is_value_cell(ty);\n",
    replace:
      "            ty::Adt(_, args) if false => {\n                let itself = self.is_value_cell(ty);\n",
    tests: ["test/corpus.test.ts", "-t", "refcell_held_while_asked"],
  },
  {
    name: "eq-of-box-as-constant",
    breaks: "`==` of `Box`es of a unit struct with its own `eq` skips it",
    file: "src/lower/std_impls.rs",
    find: "                ty::Adt(_, args) if ty.is_box() || self.is_rc(ty) && self.counted_rc(ty).is_none() => {\n                    ty = args.type_at(0);",
    replace:
      "                ty::Adt(_, args) if false => {\n                    ty = args.type_at(0);",
    tests: ["test/corpus.test.ts", "-t", "pin_box"],
  },
  {
    name: "eq-of-pin-as-constant",
    breaks: "`==` of `Pin`s of a unit struct with its own `eq` skips it",
    file: "src/lower/std_impls.rs",
    find: "                ty::Adt(_, args) if self.recognition().pinned(ty).is_some() => {\n                    let pointer",
    replace:
      "                ty::Adt(_, args) if false => {\n                    let pointer",
    tests: ["test/corpus.test.ts", "-t", "pin_box"],
  },
  {
    name: "type-id-eq-deep",
    breaks: "`==` of `TypeId`s isn't `===`",
    file: "src/lower/std_impls.rs",
    find: "            || self.recognition().is_type_id(ty)\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "dyn_any_generic"],
    snapshots: true,
  },
  {
    name: "tagged-enum-eq-generic",
    breaks:
      "a tagged enum's fieldless variants are compared by the generic `$eq`, not by their tag",
    file: "src/lower/std_impls.rs",
    find: "            && let Some(key) = super::bindings::declared_tag(self.tcx, adt.did())\n        {\n            return Ok(Expr::bin(Op::Eq, Expr::member(a, &key), Expr::member(b, &key)));\n",
    replace:
      "            && let Some(key) = super::bindings::declared_tag(self.tcx, adt.did())\n            && false\n        {\n            return Ok(Expr::bin(Op::Eq, Expr::member(a, &key), Expr::member(b, &key)));\n",
    tests: ["test/corpus.test.ts", "-t", "tagged_enum_values"],
    snapshots: true,
  },
  {
    name: "own-eq-enum-by-identity",
    breaks:
      "a fieldless enum with an `eq` of the crate's is compared by `===`, `contains` by `includes`, not by its `eq`",
    file: "src/lower/std_impls.rs",
    find: "                && !self.has_user_impl(self.partial_eq_trait(), ty));\n",
    replace: ");\n",
    tests: ["test/corpus.test.ts", "-t", "^enum_own_eq.rs$"],
  },
  {
    name: "type-id-key-by-value",
    breaks: "a `TypeId` key is found by value, a `$KeyMap`, not as a string",
    file: "src/lower/std_impls.rs",
    find: "            || self.recognition().is_type_id(ty)\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^dyn_any.rs"],
    snapshots: true,
  },
  {
    name: "float-same-value",
    breaks:
      "a float is found by `includes`, `[NaN].includes(NaN)`, where Rust's `contains` doesn't find `NaN`",
    file: "src/lower/std_impls.rs",
    find: "            return Some(match Num::of(ty).is_some_and(Num::float) || any {\n",
    replace: "            return Some(match any {\n",
    tests: ["test/corpus.test.ts", "-t", "^nan_contains.rs$"],
  },
  {
    name: "discriminant-not-primitive",
    breaks: "a `Discriminant` isn't compared as the number it is",
    file: "src/lower/std_impls.rs",
    find: "            || self.recognition().is_discriminant(ty)\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^enum_discriminant.rs$"],
  },
  {
    name: "fn-pointer-eq-refused",
    breaks: "`==` of two function pointers is refused, not JS's identity",
    file: "src/lower/std_impls.rs",
    find: "            || matches!(ty.kind(), ty::FnPtr(..))\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^fn_pointer_eq.rs$"],
  },
  {
    name: "cow-eq-as-enum",
    breaks: 'two `Cow`s compare their variants, `Borrowed("a") != Owned("a")`',
    file: "src/lower/std_impls.rs",
    find: '                ty::Adt(..) if let Some((x, borrowed)) = self.cow_deref(&a, ty) => {\n                    (a, b, ty) = (x, self.cow_deref(&b, ty).expect("a `Cow`").0, borrowed);\n                }\n',
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^cow_compare.rs$"],
  },
  {
    name: "cow-eq-structural",
    breaks:
      "what holds a `Cow`, an `Option` of one, compares it with `$eq`, its variant too",
    file: "src/lower/std_impls.rs",
    find: "            ty::Adt(..) if self.recognition().is_cow(ty) => true,\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^cow_compare.rs$"],
  },
  {
    name: "path-eq-as-text",
    breaks: "`==` of paths is refused, where it's their components'",
    file: "src/lower/std_impls.rs",
    find: '        if self.recognition().is_path(ty) {\n            return Ok(Expr::call(Expr::helper("$pathEq"), vec![a, b]));\n',
    replace:
      '        if false {\n            return Ok(Expr::call(Expr::helper("$pathEq"), vec![a, b]));\n',
    tests: ["test/corpus.test.ts", "-t", "^path_compare.rs$"],
  },
  {
    name: "pointer-eq-refused",
    breaks: "`p == q` of raw pointers is refused",
    file: "src/lower/std_impls.rs",
    find: "        if self.compared_by_address(ty)\n            && let Some((pointee, _)) = super::pointers::raw_pointee(self.tcx, ty)\n        {\n            return self.pointers_equal(a, b, pointee, span);\n",
    replace:
      "        if false\n            && let Some((pointee, _)) = super::pointers::raw_pointee(self.tcx, ty)\n        {\n            return self.pointers_equal(a, b, pointee, span);\n",
    tests: ["test/corpus.test.ts", "-t", "^raw_pointers.rs$"],
  },
  {
    name: "pointer-held-compared-by-value",
    breaks:
      "what holds a raw pointer, an `Option` of one, compares it with `$eq`, the handles' fields, not their identity",
    file: "src/lower/std_impls.rs",
    find: "            _ if self.compared_by_address(ty) => true,\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^raw_pointers.rs$"],
  },
  {
    name: "eq-two-types-as-one",
    breaks:
      "`==` of two types' items asks `PartialEq` of the left one's alone, and is refused",
    file: "src/lower/std_impls.rs",
    find: "        if lhs != other {\n",
    replace: "        if false {\n",
    tests: ["test/corpus.test.ts", "-t", "^eq_two_types.rs$"],
  },
  {
    name: "eq-items-repeated",
    breaks: "a side of `==` of items that's a call is called for each item",
    file: "src/lower/std_impls.rs",
    find: '        let b = if b.reads_same() { b } else { self.spill("other", b, out) };\n',
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^eq_two_types.rs$"],
    snapshots: true,
  },
  {
    name: "boxed-some-compared-as-object",
    breaks:
      "`Some(())`, an object of its own, compares by which object it is, so `o == Some(())` is never true",
    file: "src/lower/std_impls.rs",
    find: "            if (self.js_eq(inner).is_some() || self.eq_by_address(inner)) && !self.boxed_payload(inner) {\n",
    replace:
      "            if self.js_eq(inner).is_some() || self.eq_by_address(inner) {\n",
    tests: ["test/corpus.test.ts", "-t", "^option_boxed_eq.rs$"],
  },
  {
    name: "option-unknown-strict",
    breaks:
      "`==` of two `Option<&Unknown>`s is `a === b`, so a `null` `None` isn't an `undefined` one, as std's `Option` says it is",
    file: "src/lower/std_impls.rs",
    find: "                    true => js::nullish_equal(a, b),\n",
    replace: "                    true => Expr::bin(Op::Eq, a, b),\n",
    tests: ["test/corpus.test.ts", "-t", "^unknown_option_equality.rs$"],
  },
  {
    name: "unknown-compared-by-contents",
    breaks:
      "a struct, a slice or a tuple holding a value of any shape compares it by `$eq`, by what's in it, so two objects alike are equal, where `===` says they aren't",
    file: "src/lower/std_impls.rs",
    find: "        self.any_shape(ty) || self.equal_only_to_itself(ty) || self.untagged(ty).is_some_and(of_one)\n",
    replace:
      "        self.equal_only_to_itself(ty) || self.untagged(ty).is_some_and(of_one)\n",
    tests: ["test/corpus.test.ts", "-t", "^unknown_option_equality.rs$"],
  },
  {
    name: "map-eq-refused",
    breaks: "`==` of two maps is refused",
    file: "src/lower/std_impls.rs",
    find: "        if self.is_map(ty)\n            && let ty::Adt(_, args) = ty.kind()\n        {\n",
    replace:
      "        if false\n            && let ty::Adt(_, args) = ty.kind()\n        {\n",
    tests: ["test/corpus.test.ts", "-t", "^map_eq.rs$"],
  },
  {
    name: "set-eq-as-map",
    breaks: "`==` of two sets compares them as maps, a key as an entry",
    file: "src/lower/std_impls.rs",
    find: '            if self.is_set(ty) {\n                return Ok(Expr::call_helper("$setEq", vec![a, b]));\n            }\n',
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^map_eq.rs$"],
  },
  {
    name: "map-eq-helper-wrapped",
    breaks:
      "a map's values compared by `$eq` are `(x, y) => $eq(x, y)`, not `$eq`",
    file: "src/lower/std_impls.rs",
    find: "                    list.push((**callee).clone());\n",
    replace:
      "                    made.push(StmtKind::Return(Some(same.clone())).at(js::Span::NONE));\n                    list.push(Expr::arrow(vec![x.clone().into(), y.clone().into()], made));\n",
    tests: ["test/corpus.test.ts", "-t", "^map_eq.rs$"],
    snapshots: true,
  },
  {
    name: "eq-of-other-type-derived",
    breaks:
      "`==` of a type with another through references or items, its own `PartialEq<Other>`, compares the type with itself field by field, `$eq(a, b)`, so it's false",
    file: "src/lower/std_impls.rs",
    find: "            if self.is_user_impl(tr) {\n                let eq = self.tcx.associated_item_def_ids(self.partial_eq_trait())[0];",
    replace:
      "            if false && self.is_user_impl(tr) {\n                let eq = self.tcx.associated_item_def_ids(self.partial_eq_trait())[0];",
    tests: ["test/corpus.test.ts", "-t", "^partial_eq_other_type.rs$"],
  },
  {
    name: "registered-symbol-key-tested",
    breaks:
      'a pattern of the registry\'s symbol tests its key, `Symbol.keyFor(ty) === "react.fragment"`, which throws of a value not a symbol',
    file: "src/lower/std_impls.rs",
    find: "                if let Some(symbol) = super::representation::symbol_of_key(key) {\n",
    replace:
      "                if let Some(symbol) = super::representation::symbol_of_key(key).filter(|_| false) {\n",
    tests: [
      "test/lowering-symbols.test.ts",
      "-t",
      "a match of the registry's symbols",
    ],
  },
  {
    name: "registered-symbol-equal-by-key",
    breaks:
      "`==` of two registered symbols compares them as structs, not by JS's `===`",
    file: "src/lower/std_impls.rs",
    find: "            || self.recognition().is_symbol_for(ty)\n            || self.is_string_like(ty)\n",
    replace: "            || self.is_string_like(ty)\n",
    tests: ["test/lowering-symbols.test.ts", "-t", "Symbol.for its key"],
  },
  {
    name: "manually-drop-eq-refused",
    breaks: "`==` of two `ManuallyDrop`s is refused, where it's their values'",
    file: "src/lower/std_impls.rs",
    find: "                ty::Adt(..) if let Some(held) = self.recognition().std_wrapped(ty) => ty = held,\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^manually_drop_values.rs$"],
  },
  {
    name: "std-enum-eq-refused",
    breaks: "`==` of std's `Bound` and `Component`, derived, is refused",
    file: "src/lower/std_impls.rs",
    find: "            || self.std_enum_derives_eq(ty);",
    replace: ";",
    tests: ["test/corpus.test.ts", "-t", "^path_components.rs$"],
  },
  {
    name: "os-text-not-js-eq",
    breaks:
      "an `OsStr` isn't a key a map finds by value, so a set of `Component`s is refused",
    file: "src/lower/std_impls.rs",
    find: "            || self.recognition().is_os_text(ty)\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^path_components.rs$"],
  },
  {
    name: "eq-of-values-borrows",
    breaks:
      "what a `RefCell` of an object holds, compared through the guard that holds it, is borrowed to compare and found borrowed",
    file: "src/lower/std_impls.rs",
    find: "            if self.recognition().is_borrowed_cell(ty) {\n                given.push(Expr::bool(true));\n            }\n",
    replace: "            given.push(Expr::bool(true));\n",
    tests: ["test/corpus.test.ts", "-t", "^refcell_eq_through_guard.rs$"],
    snapshots: true,
  },
  {
    name: "eq-of-cells-unborrowed",
    breaks:
      "`==` of two `RefCell`s, one mutably borrowed, runs on where Rust panics",
    file: "src/lower/std_impls.rs",
    find: "            if self.recognition().is_borrowed_cell(ty) {\n                given.push(Expr::bool(true));\n            }\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^refcell_eq_while_mut.rs$"],
  },
  {
    name: "identity-eq-unseen",
    breaks:
      "a type equal only to itself, by `ptr::eq`, keys no `Set`, refused as a key",
    file: "src/lower/std_impls.rs",
    find: "            || self.equal_only_to_itself(ty)\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^identity_keys.rs$"],
  },
  {
    name: "library-identity-unread",
    breaks:
      "a library's type equal only to itself isn't, to its consumer: a `HashSet<&'static Fiber>` of the library's is refused, as React's reconciler's was",
    file: "src/lower/std_impls.rs",
    find: "            true => recognition.foreign.equal_only_to_itself(adt.did()),\n",
    replace: "            true => false,\n",
    tests: [
      "test/crates.test.ts",
      "-t",
      "equal only to itself keys its consumer",
    ],
  },
  {
    name: "rc-identity-unread",
    breaks:
      "an `Rc` of a type equal only to itself isn't, so React's root's `HashSet<Rc<Fiber>>` is refused, \"values of type `Rc<Fiber>`\"",
    file: "src/lower/std_impls.rs",
    find: "    if let Some(pointee) = super::recognition::rc_pointee(recognition.tcx, ty) {\n",
    replace:
      "    if let Some(pointee) = super::recognition::rc_pointee(recognition.tcx, ty).filter(|_| false) {\n",
    tests: ["test/corpus.test.ts", "-t", "^rc_identity_set(_counted)?.rs$"],
  },
  {
    name: "identity-eq-value-called",
    breaks:
      "`==` of two values whose `PartialEq` is `ptr::eq` but has no `Hash` calls its `eq`, `updatePartialEq_eq(a, b)`, not `a === b`",
    file: "src/lower/std_impls.rs",
    find: "        if self.js_eq(ty).is_some() || self.eq_by_address(ty) {\n",
    replace: "        if self.js_eq(ty).is_some() {\n",
    tests: ["test/lowering-dictionaries.test.ts", "-t", "identity PartialEq"],
  },
  {
    name: "identity-eq-option-called",
    breaks:
      "`a != b` of two `Option<Rc<Update>>` whose `PartialEq` is `ptr::eq` calls its `eq` on two `Some`s, not `a != b`",
    file: "src/lower/std_impls.rs",
    find: "            if (self.js_eq(inner).is_some() || self.eq_by_address(inner)) && !self.boxed_payload(inner) {\n",
    replace:
      "            if self.js_eq(inner).is_some() && !self.boxed_payload(inner) {\n",
    tests: ["test/lowering-dictionaries.test.ts", "-t", "identity PartialEq"],
  },
  {
    name: "identity-eq-needs-hash",
    breaks:
      "a `PartialEq` of `ptr::eq` is identity only with a `Hash` of its address too, so React's `Update`'s `a != b` calls its `eq`",
    file: "src/lower/std_impls.rs",
    find: "    if !hashed {\n        return (recognition.own_eq(adt.did())).is_some_and(|eq| calls(eq, super::recognition::Std::PtrEq));\n    }\n",
    replace: "",
    tests: ["test/lowering-dictionaries.test.ts", "-t", "identity PartialEq"],
  },
  {
    name: "untagged-eq-structural",
    breaks:
      "an untagged enum of primitives, React's `ReactKey`, is compared by `$eq`, not JS's `===`",
    file: "src/lower/std_impls.rs",
    find: "        self.untagged_js_eq(ty)\n",
    replace: "        None\n",
    snapshots: true,
    tests: ["test/corpus.test.ts", "-t", "^untagged_eq_by_value.rs$"],
  },
  {
    name: "untagged-eq-of-a-float-same-value",
    breaks:
      "an untagged enum of a float is found by `includes`, whose `NaN` is found where Rust's `==` finds none",
    file: "src/lower/std_impls.rs",
    find: "                    if self.js_eq(payload)? == JsEq::Strict {\n                        eq = JsEq::Strict;\n",
    replace: "                    if self.js_eq(payload)? == JsEq::Strict {\n",
    tests: ["test/corpus.test.ts", "-t", "^untagged_eq_by_value.rs$"],
  },
  {
    name: "untagged-option-eq-loose",
    breaks:
      'an `Option` of an untagged enum of a string and a number is compared by `==`, which makes `"7"` of `7`',
    file: "src/lower/std_impls.rs",
    find: "self.any_shape(inner) || self.untagged(inner).is_some()",
    replace: "self.any_shape(inner)",
    tests: ["test/corpus.test.ts", "-t", "^untagged_eq_by_value.rs$"],
  },
  {
    name: "untagged-eq-of-a-payload-compared-otherwise",
    breaks:
      "an untagged enum of a `Vec` is compared by `===`, two lists alike unequal",
    file: "src/lower/std_impls.rs",
    find: "                    if self.js_eq(payload)? == JsEq::Strict {\n",
    replace:
      "                    if self.js_eq(payload).unwrap_or(JsEq::SameValue) == JsEq::Strict {\n",
    tests: ["test/corpus.test.ts", "-t", "^untagged_eq_by_value.rs$"],
  },
  {
    name: "contains-strict-some",
    breaks:
      "`contains` of floats is `some((each) => each === x)`, not `indexOf(x) >= 0`, React's `nextInstanceRoots.indexOf(target) < 0` (ADR 0966)",
    file: "src/lower/std_impls.rs",
    find: "        if body.is_empty() && strict {\n",
    replace: "        if body.is_empty() && strict && false {\n",
    tests: ["test/corpus.test.ts", "-t", "^float_partial_cmp_tests.rs$"],
    snapshots: true,
  },
  {
    name: "contains-strict-includes",
    breaks:
      "`contains` of floats is `includes(x)`, which finds `NaN`, where Rust's `==` doesn't (ADR 0966)",
    file: "src/lower/std_impls.rs",
    find: '                    Expr::call(Expr::member(items, "indexOf"), vec![x]),\n                    Expr::int(0),\n',
    replace:
      '                    Expr::call(Expr::member(items, "includes"), vec![x]),\n                    Expr::bool(true),\n',
    tests: ["test/corpus.test.ts", "-t", "^float_partial_cmp_tests.rs$"],
  },
  {
    name: "identity-inside-compared-by-contents",
    breaks:
      "`==` of an `Option<&Option<Rc<Fiber>>>`, a tuple, a `Vec` or a derived struct holding a type equal only to itself is `$eq`, which walks the fibers' fields: two alike are equal, and a cycle overflows the stack, React's `processDispatchQueueItemsInOrder` (rd104)",
    file: "src/lower/std_impls.rs",
    find: "        if self.js_eq(ty).is_some() {\n            return self.eq_of_objects(ty);\n        }\n",
    replace:
      "        if self.js_eq(ty).is_some() {\n            return false;\n        }\n",
    tests: ["test/corpus.test.ts", "-t", "^eq_of_identities_inside.rs$"],
  },
  {
    name: "identity-inside-unseen",
    breaks:
      "a type equal only to itself inside what `==` compares is no object `===` compares by itself, so `$eq` walks it (rd104)",
    file: "src/lower/std_impls.rs",
    find: "        self.any_shape(ty) || self.equal_only_to_itself(ty) || self.untagged(ty).is_some_and(of_one)\n",
    replace:
      "        self.any_shape(ty) || self.untagged(ty).is_some_and(of_one)\n",
    tests: ["test/corpus.test.ts", "-t", "^eq_of_identities_inside.rs$"],
  },
  {
    name: "identity-inside-untagged-unseen",
    breaks:
      "an untagged enum of an `Rc<Fiber>` in a derived struct is compared by `$eq`, which walks the fiber (rd104)",
    file: "src/lower/std_impls.rs",
    find: "        self.any_shape(ty) || self.equal_only_to_itself(ty) || self.untagged(ty).is_some_and(of_one)\n",
    replace: "        self.any_shape(ty) || self.equal_only_to_itself(ty)\n",
    tests: ["test/corpus.test.ts", "-t", "^eq_of_identities_inside.rs$"],
  },
  {
    name: "library-eq-by-address-unread",
    breaks:
      "a consumer asks a library's `Update`, whose `PartialEq` alone is `ptr::eq`, whether it's equal only to itself, which needs its `Hash` too, and calls its `eq` (ADR 0864, amended)",
    file: "src/lower/std_impls.rs",
    find: "            false => recognition.foreign.eq_by_address(adt.did()) ||",
    replace:
      "            false => recognition.foreign.equal_only_to_itself(adt.did()) ||",
    tests: ["test/crates.test.ts", "-t", "never \\$eq"],
  },
  {
    name: "webapi-eq-binding-unread",
    breaks:
      "`==` of two webapi classes, whose `eq` is the binding of `===`, is refused as of any JS object, and `v.contains(&node)` with it (ADR 1016)",
    file: "src/lower/std_impls.rs",
    find: " || recognition.eq_bound_to_strict_equality(adt.did()),\n",
    replace: ",\n",
    tests: ["test/webapi-bindings.test.ts", "-t", "equal only to itself"],
  },
  {
    name: "eq-by-address-of-references-unseen",
    breaks:
      "`a != b` of two `Option<&Node>`s asks whether `==` of a reference is its address's, which isn't a type of its own, and compares `a == null || b == null ? a == b : a === b` (ADR 1016)",
    file: "src/lower/std_impls.rs",
    find: "        !self.has_cell_layer(ty) && eq_by_address(&self.recognition(), self.krate.bodies, ty.peel_refs())\n",
    replace:
      "        !self.has_cell_layer(ty) && eq_by_address(&self.recognition(), self.krate.bodies, ty)\n",
    tests: ["test/webapi-bindings.test.ts", "-t", "equal only to itself"],
  },
  {
    name: "contains-of-identity-index-of",
    breaks:
      "`v.contains(&node)` of what `===` compares and is never `NaN`, an object equal only to itself, is `v.indexOf(node) >= 0`, not `v.includes(node)` (ADR 1016)",
    file: "src/lower/std_impls.rs",
    find: '                None => Expr::call(Expr::member(items, "includes"), vec![x]),\n',
    replace:
      '                None => Expr::bin(Op::Ge, Expr::call(Expr::member(items, "indexOf"), vec![x]), Expr::int(0)),\n',
    tests: ["test/webapi-bindings.test.ts", "-t", "equal only to itself"],
  },
  {
    name: "option-eq-one-none-loose",
    breaks: "two `Option`s a getter gives, each `T | null`, compare by `==`, `node.firstChild == node.lastChild`, where upstream writes `===` (rd117)",
    file: "src/lower/std_impls.rs",
    find: "                    false if a.facts.same_none(b.facts) => Expr::bin(Op::Eq, a, b),\n",
    replace: "",
    tests: ["test/webapi-bindings.test.ts", "-t", "equal only to itself"],
  },
  {
    name: "recursive-eq-in-place",
    breaks:
      "a derived `==` of a type inside itself with a custom part, regex-syntax's `Hir`, is expanded in place, the one inside it again, until rust-js's stack overflows (#511)",
    file: "src/lower/std_impls.rs",
    find: "        if self.is_recursive(ty) {\n            let ty::Adt(adt, _) = ty.kind() else {\n                unreachable!(\"a recursive type is an ADT\");\n            };\n            let base = format!(\"eq{}\"",
    replace: "        if false && self.is_recursive(ty) {\n            let ty::Adt(adt, _) = ty.kind() else {\n                unreachable!(\"a recursive type is an ADT\");\n            };\n            let base = format!(\"eq{}\"",
    tests: ["test/corpus.test.ts", "-t", "^recursive_eq.rs$"],
  },
  {
    name: "variant-eq-boxes-cells",
    breaks: "a derived `==` of an enum whose variant's field compares as it's written reads its `RefCell` field that holds its value as `{ value }` (#503)",
    file: "src/lower/std_impls.rs",
    find: "                            self.held(Expr::member(a.clone(), field.clone()), Some(def), t),\n                            self.held(Expr::member(b.clone(), field), Some(def), t),\n",
    replace: "                            self.held(Expr::member(a.clone(), field.clone()), Some(def).filter(|_| false), t),\n                            self.held(Expr::member(b.clone(), field), Some(def).filter(|_| false), t),\n",
    tests: ["test/corpus.test.ts", "-t", "^refcell_variant_derives.rs$"],
  },
  {
    name: "rc-eq-by-value-alone",
    breaks: "`==` of two `Rc`s of one allocation of an `Eq` value calls its `eq`, where std's `RcEqIdent` answers by address (#910)",
    file: "src/lower/std_impls.rs",
    find: "            && self.custom_eq(pointee)\n            && !self.eq_is_identity(pointee)\n",
    replace: "            && false\n            && !self.eq_is_identity(pointee)\n",
    tests: ["test/corpus.test.ts", "-t", "^rc_eq_by_address_first.rs$"],
  },
  {
    name: "rc-eq-identity-twice",
    breaks: "`==` of two `Rc`s of a value compared by identity asks `===` twice, `a === b || a === b` (#910)",
    file: "src/lower/std_impls.rs",
    find: "            && !self.eq_is_identity(pointee)\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^eq_of_identities_inside.rs$"],
    snapshots: true,
  },
  {
    name: "rc-eq-counted-boxes-compared",
    breaks: "`==` of two counted `Rc`s compares their boxes as the values, not what they hold (#910)",
    file: "src/lower/std_impls.rs",
    find: "                true => (\n                    self.counted_pointee(a.clone(), pointee),\n                    self.counted_pointee(b.clone(), pointee),\n                ),\n",
    replace: "                true => (a.clone(), b.clone()),\n",
    tests: ["test/corpus.test.ts", "-t", "^rc_eq_by_address_first.rs$"],
  },
  {
    name: "slice-eq-by-eq",
    breaks: "std's slice `==` asks each two `==`, where it asks `!=`, an impl's own `ne` (#962)",
    file: "src/lower/std_impls.rs",
    find: "        let same = match by_ne && self.own_ne_between(item, other_item) {\n",
    replace: "        let same = match false && by_ne && self.own_ne_between(item, other_item) {\n",
    tests: ["test/corpus.test.ts", "-t", "^own_ne_max_min_clamp.rs$"],
  },
  {
    name: "list-eq-by-ne",
    breaks: "a `LinkedList`'s `==` asks each two `!=`, where its iterators' `eq` asks `==` (#962)",
    file: "src/lower/std_impls.rs",
    find: "                let by_ne = !std(StdItem::LinkedList);\n",
    replace: "                let by_ne = true;\n",
    tests: ["test/corpus.test.ts", "-t", "^own_ne_max_min_clamp.rs$"],
  },
  {
    name: "eq-items-ne-of-one-type",
    breaks: "a slice's `==` asks items' `!=` only of one item type, so `[A] == [B]` asks `eq` where std asks `A`'s own `ne` (a wrong answer, #970)",
    file: "src/lower/std_impls.rs",
    find: "        let same = match by_ne && self.own_ne_between(item, other_item) {\n",
    replace: "        let same = match by_ne && item.peel_refs() == other_item.peel_refs() && self.own_ne(item) {\n",
    tests: ["test/corpus.test.ts", "-t", "^ne_between_two_item_types.rs$"],
  },
];
