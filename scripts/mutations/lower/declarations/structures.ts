// Mutations of crates/rust-js-lower/src/declarations/structures.rs (ADR 0093): a
// struct's declaration, its fields, each by its JS name, an `Option`'s
// optional, a `#[repr(transparent)]` one its field's type.
import type { Mutation } from "../../../mutations";

const tests = ["test/declarations.test.ts"];

export const mutations: Mutation[] = [
  {
    name: "held-value-field-boxed",
    breaks: "a nullable field of what may be `undefined`, which holds its `Some` as the value itself (rd645), is declared with ADR 0051's box, `serverModuleMap: ServerManifest | SomeNone | null`, where the JS never has one (#842)",
    file: "crates/rust-js-lower/src/declarations/structures.rs",
    find: "                        true => out.ts(inner),\n",
    replace: "                        true => out.some_ts(inner),\n",
    tests: ["test/lowering-nested-options.test.ts", "-t", "a nullable field of what may be undefined holds the value itself"],
  },
  {
    name: "boxed-option-field-unboxed",
    breaks: "a struct's `Option<()>` field is declared its payload, `unit?: undefined`, not its box, `unit?: SomeNone | undefined` (#842)",
    file: "crates/rust-js-lower/src/declarations/structures.rs",
    find: "                        false => out.some_ts(inner),\n",
    replace: "                        false => out.ts(inner),\n",
    tests: ["test/declarations-boxed-options.test.ts"],
  },
  {
    name: "nullable-field-union-nested",
    breaks: "a nullable field's union nests what it holds, `(number | SomeNone) | null`, not one union (#842)",
    file: "crates/rust-js-lower/src/declarations/structures.rs",
    find: "        Some(\"union\") => ty[\"types\"].as_array().cloned().unwrap_or_default(),\n        _ => vec![ty],\n    };\n    types.push(keyword(\"null\"));\n",
    replace: "        _ => vec![ty],\n    };\n    types.push(keyword(\"null\"));\n",
    tests: ["test/declarations-boxed-options.test.ts"],
  },
  {
    name: "spread-declared-any-keys",
    breaks: "a flattened `js::Spread` is declared an index of `unknown`, so its copy's keys aren't the given's (ADR 1164)",
    file: "crates/rust-js-lower/src/declarations/structures.rs",
    find: "                    crate::recognition::is_js_spread(self.tcx, adt.did()) || self.flattened.enum_of_keys(*ty)\n",
    replace: "                    self.flattened.enum_of_keys(*ty)\n",
    tests: ["test/js-spread.test.ts", "-t", "JS's own spread"],
  },
  {
    name: "spread-omits-nothing",
    breaks: "a `js::Spread`'s copy omits none of the struct's own keys, so the given's `source` is its type, where the struct's own wins (ADR 1164)",
    file: "crates/rust-js-lower/src/declarations/structures.rs",
    find: "                        1 => own_keys[0].clone(),\n",
    replace: "                        1 => keyword(\"never\"),\n",
    tests: ["test/js-spread.test.ts", "-t", "JS's own spread"],
  },
  {
    name: "accessor-field-writable",
    breaks: "a `js::Accessor` field is declared writable, where its getter has no setter (ADR 1164)",
    file: "crates/rust-js-lower/src/declarations/structures.rs",
    find: "                    \"readonly\": true,\n                    \"type\": self.ts(args.type_at(0)),\n",
    replace: "                    \"readonly\": false,\n                    \"type\": self.ts(args.type_at(0)),\n",
    tests: ["test/js-accessor.test.ts", "-t", "a live own getter"],
  },
  {
    name: "distributive-declared-as-interface",
    breaks: "a `#[rust_js::distributive]` struct is one interface of its reason, so checking `reason` doesn't narrow `event`, where upstream's `ChangeDetails` is one member per reason (ADR 1164)",
    file: "crates/rust-js-lower/src/declarations/structures.rs",
    find: "        if is_distributive(self.tcx, def_id) {\n",
    replace: "        if false && is_distributive(self.tcx, def_id) {\n",
    tests: ["test/declarations-type-maps.test.ts", "-t", "its keys and its lookup"],
  },
  {
    name: "untagged-records-declared-extended",
    breaks: "a struct that flattens an untagged enum of records extends it, which an interface can't, not `{ key: string } & CompositionInput` (#1818)",
    file: "crates/rust-js-lower/src/declarations/structures.rs",
    find: "                    crate::recognition::is_js_spread(self.tcx, adt.did()) || self.flattened.enum_of_keys(*ty)\n",
    replace: "                    crate::recognition::is_js_spread(self.tcx, adt.did()) || self.flattened.keyed.contains(&adt.did())\n",
    tests: ["test/flattened-untagged.test.ts", "-t", "declared an intersection"],
  },  {
    name: "cell-option-field-required",
    breaks:
      "a plain `Cell<Option<f64>>` field is declared `byteSize: number | null | undefined`, its cell's `Option` not seen, where it's `byteSize?: number | undefined` as an `Option` field is (rd415)",
    file: "crates/rust-js-lower/src/declarations/structures.rs",
    find: "                    .or_else(|| self.cell_item(ty, Some(field)).and_then(|item| self.option(item))),\n",
    replace: "                    .or(None),\n",
    tests: ["test/declarations-cells.test.ts"],
  },
  {
    name: "nested-field-declared-undefined",
    breaks: "a nullable `Option<Option<T>>` field is declared `T | undefined`, where JS holds `null` too (#470)",
    file: "crates/rust-js-lower/src/declarations/structures.rs",
    find: "                    match nullable || nested || value || self.null_fields.contains(&field.did) {\n",
    replace: "                    match nullable || value || self.null_fields.contains(&field.did) {\n",
    tests: ["test/declarations.test.ts", "-t", "declarations take JS's null"],
  },
  {
    name: "bound-methods-undeclared",
    breaks:
      "a struct's methods its impl binds of the JS object, `isPersistent()`, aren't in its interface, narrower than upstream's type (rd144)",
    file: "crates/rust-js-lower/src/declarations/structures.rs",
    find: "        for (method, name) in crate::bindings::bound_methods(self.tcx, def_id) {\n",
    replace: "        for (method, name) in crate::bindings::bound_methods(self.tcx, def_id).into_iter().take(0) {\n",
    tests: ["test/declarations-bound-methods.test.ts"],
  },
  {
    name: "declared-option-required",
    breaks:
      "an `Option` field is a required prop to TypeScript, `text: string`, which a caller leaving it out is an error of",
    file: "crates/rust-js-lower/src/declarations/structures.rs",
    find: "                        false => (true, inner),\n",
    replace: "                        false => (false, inner),\n",
    tests,
  },
  {
    name: "declared-rest-dropped",
    breaks:
      "props a struct doesn't name, `...rest`, are an error to TypeScript, `aria-label` of a link",
    file: "crates/rust-js-lower/src/declarations/structures.rs",
    find: "            if is_rest(self.tcx, ty) || (is_flatten(self.tcx, field) && extends.is_empty() && intersected.is_none()) {",
    replace: "            if false && (is_rest(self.tcx, ty) || (is_flatten(self.tcx, field) && extends.is_empty() && intersected.is_none())) {",
    tests,
  },
  {
    name: "flattened-declared-nested",
    breaks:
      "a flattened field is declared a field, `anchor: Anchor`, not `extends Anchor`",
    file: "crates/rust-js-lower/src/declarations/structures.rs",
    find: "                    (false, 0) => vec![inner],",
    replace: "                    (false, 0) => Vec::new(),",
    tests,
  },
  {
    name: "flattened-extends-any",
    breaks:
      "a flattened field of another module's type of the crate's own, `pub(crate) struct Far`, is `extends any`, which TypeScript refuses",
    file: "crates/rust-js-lower/src/declarations/structures.rs",
    find: "                    (true, _) => Vec::new(),",
    replace: "                    (true, _) => vec![inner],",
    tests: ["test/declarations-flatten.test.ts"],
  },
  {
    name: "shadowed-names-not-omitted",
    breaks:
      'props extend what they shadow, not `Omit<Linked, "href" | "className">`',
    file: "crates/rust-js-lower/src/declarations/structures.rs",
    find: '                    (false, _) => vec![reference(\n                        "Omit",',
    replace:
      '                    (false, _) => vec![inner.clone(), reference(\n                        "Omit",',
    tests,
  },
  {
    name: "defaulted-field-required",
    breaks:
      "a field with a default is declared required, `count: number`, where a caller may leave it out",
    file: "crates/rust-js-lower/src/declarations/structures.rs",
    find: "                    field_default(self.tcx, field).is_some() || crate::representation::always_undefined(self.tcx, ty),\n",
    replace: "                    crate::representation::always_undefined(self.tcx, ty),\n",
    tests,
  },
  {
    name: "option-field-without-null",
    breaks:
      "a `#[rust_js::nullable]` field is declared `?: T`, so a TypeScript caller's `{ label: null }`, which rust-js reads as `None`, doesn't typecheck",
    file: "crates/rust-js-lower/src/declarations/structures.rs",
    find: "                        true => (!present, with_null(inner)),\n",
    replace: "                        true => (!present, inner),\n",
    tests: ["test/declarations.test.ts", "-t", "null for an Option"],
  },
  {
    name: "every-option-field-nullable",
    breaks:
      "every `Option` field is declared `?: T | null`, so a TypeScript caller reading one passes `null` where a default takes only `undefined`",
    file: "crates/rust-js-lower/src/declarations/structures.rs",
    find: "                    match nullable || nested || value || self.null_fields.contains(&field.did) {\n",
    replace: "                    match true || nullable {\n",
    tests: ["test/declarations.test.ts", "-t", "null for an Option"],
  },
  {
    name: "optional-field-refuses-undefined",
    breaks:
      "a field a caller may leave out is `?: T`, which refuses `undefined` under `exactOptionalPropertyTypes`, where Rust takes it as `None`",
    file: "crates/rust-js-lower/src/declarations/structures.rs",
    find: "            let ty = if optional { or_undefined(ty) } else { ty };\n",
    replace: "",
    tests: ["test/declarations.test.ts", "-t", "null for an Option"],
  },
  {
    name: "transparent-declared-as-struct",
    breaks:
      "a `#[repr(transparent)]` struct is declared an interface of its fields, where its JS is its field",
    file: "crates/rust-js-lower/src/declarations/structures.rs",
    find: "        if let Some(at) = crate::recognition::transparent_field(self.tcx, adt) {\n",
    replace: "        if let Some(at) = crate::recognition::transparent_field(self.tcx, adt).filter(|_| false) {\n",
    tests: [
      "test/declarations.test.ts",
      "-t",
      "transparent struct is declared",
    ],
  },
  {
    name: "nullable-cell-declared-undefined",
    breaks:
      "a nullable `Cell` field is declared `T | undefined`, where JS has its `None` `null`",
    file: "crates/rust-js-lower/src/declarations/structures.rs",
    find: "                true => crate::bindings::nullable_option(self.tcx, ty).and_then(|option| self.option(option)),\n",
    replace: "                true => self.option(ty),\n",
    tests: ["test/declarations-nullable.test.ts"],
  },
  {
    name: "once-field-declared-a-cell",
    breaks:
      "a `OnceCell` field is declared as one that's always there, where it's absent until it's set, `ioInfo?: string | undefined`",
    file: "crates/rust-js-lower/src/declarations/structures.rs",
    find: "                false => (self.option(ty).or_else(|| self.once_cell_item(ty)))\n",
    replace: "                false => (self.option(ty))\n",
    tests: [
      "test/declarations.test.ts",
      "-t",
      "declarations type what a module exports",
    ],
  },
  {
    name: "object-methods-undeclared",
    breaks:
      "an object of methods' interface declares its fields alone, so TypeScript has no `scope.containsNode(..)` (ADR 0943)",
    file: "crates/rust-js-lower/src/declarations/structures.rs",
    find: "            if !self.tcx.visibility(method).is_public() {\n",
    replace: "            if true {\n",
    tests: [
      "test/classes.test.ts",
      "-t",
      "an object's methods are its literal's own",
    ],
  },
  {
    name: "js-null-fields-unread",
    breaks:
      "a plain `Option` field a `js::null()` is written to is declared `?: T | undefined`, though it holds `null` (ADR 0935, amended)",
    file: "crates/rust-js-lower/src/declarations/structures.rs",
    find: "                    match nullable || nested || value || self.null_fields.contains(&field.did) {\n",
    replace: "                    match nullable || nested || value {\n",
    tests: ["test/lowering-nullable-constants.test.ts", "-t", "is declared"],
  },
  {
    name: "field-symbol-private-undeclared",
    breaks: "a module's own symbol a declared field is named by isn't declared, `[RESPONSE_SYMBOL]` of nothing TypeScript knows",
    file: "crates/rust-js-lower/src/declarations/structures.rs",
    find: "                self.unexported.insert(name.clone(), Some(declared));\n            }\n            return json!({ \"computed\": name });",
    replace: "            }\n            return json!({ \"computed\": name });",
    tests: [
      "test/field-symbol-names.test.ts",
      "-t",
      "a field a symbol names is that symbol's key"
    ]
  },
  {
    name: "field-symbol-declared-by-text",
    breaks: "a declared field a symbol names is declared by the symbol's text, a string key",
    file: "crates/rust-js-lower/src/declarations/structures.rs",
    find: "            return json!({ \"computed\": crate::bindings::js_name(self.tcx, item) });",
    replace: "            return json!(crate::bindings::js_name(self.tcx, item));",
    tests: [
      "test/field-symbol-names.test.ts",
      "-t",
      "a closure a symbol names is its method"
    ]
  },
  {
    name: "constructed-declared-optional",
    breaks: "a nullable field each constructor of its struct sets first is declared optional, `moduleLoading?: unknown | null | undefined`, though it's present (rd628)",
    file: "crates/rust-js-lower/src/declarations/structures.rs",
    find: "                        true => (!present, with_null(inner)),\n",
    replace: "                        true => (!present || true, with_null(inner)),\n",
    tests: ["test/js-constructors.test.ts", "-t", "a nullable field each constructor sets first"],
  },
  {
    name: "value-field-declared-optional",
    breaks:
      "a nullable field of what may be `undefined` is declared optional, `serverModuleMap?: ServerManifest | null | undefined` (rd645)",
    file: "crates/rust-js-lower/src/declarations/structures.rs",
    find: "                    let present = matches!(\n                        declared,\n                        crate::representation::NoneForm::Null | crate::representation::NoneForm::Value\n                    );\n",
    replace: "                    let present = matches!(declared, crate::representation::NoneForm::Null);\n",
    tests: ["test/lowering-nested-options.test.ts", "-t", "what may be undefined"],
  },
  {
    name: "value-field-declared-without-null",
    breaks:
      "a nullable field of what may be `undefined` is declared without `| null` (rd645)",
    file: "crates/rust-js-lower/src/declarations/structures.rs",
    find: "                    match nullable || nested || value || self.null_fields.contains(&field.did) {\n",
    replace: "                    match nullable || nested || self.null_fields.contains(&field.did) {\n",
    tests: ["test/lowering-nested-options.test.ts", "-t", "what may be undefined"],
  },
  {
    name: "flattened-param-extended",
    breaks: "a struct of a flattened type parameter is declared an interface extending it, which TypeScript refuses (#504)",
    file: "crates/rust-js-lower/src/declarations/structures.rs",
    find: "                ty::Param(_) => true,\n                ty::Adt(adt, _) => {\n",
    replace: "                ty::Param(_) => false,\n                ty::Adt(adt, _) => {\n",
    tests: ["test/flattened-keyed.test.ts", "-t", "intersection"],
  },
  {
    name: "js-given-nullable-field-declared-optional",
    breaks:
      "a nullable field of a struct JS may give is declared optional, `moduleLoading?: unknown | null | undefined`, where `nullable` says JS gives `null`, never `undefined`, as upstream's `moduleLoading: ModuleLoading` does (ADR 1072, amended)",
    file: "crates/rust-js-lower/src/declarations/structures.rs",
    find: "                    let present = matches!(\n                        declared,\n                        crate::representation::NoneForm::Null | crate::representation::NoneForm::Value\n                    );\n",
    replace: "                    let present = matches!(declared, crate::representation::NoneForm::Value);\n",
    tests: ["test/lowering-nullable-reads.test.ts", "-t", "only the crate makes its struct"],
  },
  {
    name: "struct-field-unfilled",
    breaks: "a struct's `dyn Fn` field is named by its types, though its literals name it",
    file: "crates/rust-js-lower/src/declarations/structures.rs",
    find: "            let outer = self.function_names.len();\n            self.fill_names(field.did, ty);\n",
    replace: "            let outer = self.function_names.len();\n",
    tests: ["test/declarations-function-types.test.ts"],
  },
  {
    name: "ungiven-object-method-declared",
    breaks: "an object of methods' method whose JS takes a fact, `scope.named(TName)`, is declared `named<T>(): string` (#559)",
    file: "crates/rust-js-lower/src/declarations/structures.rs",
    find: "            if self.leave_out(method, &name) {\n",
    replace: "            if false && self.leave_out(method, &name) {\n",
    tests: ["test/declarations-ungiven.test.ts"],
  },
  {
    name: "present-declared-optional",
    breaks: "a `#[rust_js::present]` field is declared optional, `point?: number`, where its key is there, `point: number | undefined` (ADR 1161)",
    file: "crates/rust-js-lower/src/declarations/structures.rs",
    find: "                        false if crate::bindings::is_present(self.tcx, field) => (false, or_undefined(inner)),\n",
    replace: "",
    tests: ["test/present-slots.test.ts", "-t", "a present field"],
  },
  {
    name: "sealed-tuple-undeclared",
    breaks: "a tuple struct of a private field, `Email(String)`, has no declaration of its own, so what names it names a type nothing declares (#2356, ADR 1187)",
    file: "crates/rust-js-lower/src/declarations/structures.rs",
    find: "            if sealed_tuple(self.tcx, def_id) {\n",
    replace: "            if false && sealed_tuple(self.tcx, def_id) {\n",
    tests: ["test/declarations-sealed.test.ts", "-t", "a tuple struct of a private field"],
  },
  {
    name: "sealed-tuple-brand-exported",
    breaks: "a sealed tuple struct's brand is exported, `export declare const $emailBrand`, so TypeScript can name its key and make one (#2356, ADR 1187)",
    file: "crates/rust-js-lower/src/declarations/structures.rs",
    find: "let declared = json!({ \"kind\": \"const\", \"name\": brand, \"exported\": false,",
    replace: "let declared = json!({ \"kind\": \"const\", \"name\": brand, \"exported\": true,",
    tests: ["test/declarations-sealed.test.ts", "-t", "a tuple struct of a private field"],
  },
  {
    name: "sealed-tuple-brand-a-rust-name",
    breaks: "a sealed tuple struct's brand is named as a Rust item's JS name can be, `nameBrand`, which a `pub fn name_brand` declares too, TypeScript's duplicate identifier (#2356, ADR 1187 amended)",
    file: "crates/rust-js-lower/src/declarations/structures.rs",
    find: "                let mut brand = format!(\"${name}Brand\");\n                brand[1..2].make_ascii_lowercase();\n",
    replace: "                let mut brand = format!(\"{name}Brand\");\n                brand[..1].make_ascii_lowercase();\n",
    tests: ["test/declarations-sealed.test.ts", "-t", "meets no export of its module"],
  },
  {
    name: "sealed-tuple-of-pub-fields",
    breaks: "a tuple struct whose fields are all `pub`, which any caller makes, `Open(pub String)`, is declared opaque too (#2356, ADR 1187)",
    file: "crates/rust-js-lower/src/declarations/structures.rs",
    find: "        && variant.fields.iter().any(|field| !field.vis.is_public())\n",
    replace: "",
    tests: ["test/declarations-sealed.test.ts", "-t", "is declared as before"],
  },
  {
    name: "sealed-tuple-generic",
    breaks: "a generic tuple struct, `Id<T>(u64, PhantomData<T>)`, is branded with no type parameter, so every `Id<T>` is one type (#2356, ADR 1187)",
    file: "crates/rust-js-lower/src/declarations/structures.rs",
    find: "        && tcx.generics_of(did).own_params.is_empty()\n",
    replace: "",
    tests: ["test/declarations-sealed.test.ts", "-t", "is declared as before"],
  },
  {
    name: "sealed-tuple-handle",
    breaks: "a handle of `PhantomData` alone, which JS gives, `Handle(PhantomData<()>)`, is declared a Rust-made value (#2356, ADR 1187)",
    file: "crates/rust-js-lower/src/declarations/structures.rs",
    find: "        !matches!(ty.kind(), ty::Adt(adt, _) if adt.is_phantom_data())\n",
    replace: "        !matches!(ty.kind(), ty::Never)\n",
    tests: ["test/declarations-sealed.test.ts", "-t", "is declared as before"],
  },
  {
    name: "sealed-tuple-binding-type",
    breaks: "a binding's type of a private field that holds a value, `#[rust_js::types = \"Map<string, number>\"] Scores(RefCell<..>)`, is declared a sealed tuple struct of its own, not JS's type it names (#2356, ADR 1187)",
    file: "crates/rust-js-lower/src/declarations/structures.rs",
    find: "        && super::written_types(tcx, did).is_none()\n",
    replace: "",
    tests: ["test/declarations-sealed.test.ts", "-t", "is declared as before"],
  },
];
