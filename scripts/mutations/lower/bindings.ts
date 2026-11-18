// Mutations of src/lower/bindings.rs (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "unit-struct-boolean-unread",
    breaks: "a unit struct named `#[rust_js::name = true]` is `undefined`, so `js::True`'s value isn't `true` (ADR 1164)",
    file: "src/lower/bindings.rs",
    find: "        .or_else(|| given_bool(tcx, def_id).map(UnitValue::Bool))\n",
    replace: "",
    tests: ["test/declarations-type-maps.test.ts", "-t", "a const type parameter"],
  },
  {
    name: "bound-methods-private-declared",
    breaks:
      "a private binding of a struct's impl is declared its interface's method, a JS caller's to call",
    file: "src/lower/bindings.rs",
    find: "        .filter(|&item| tcx.visibility(item).is_public() && is_binding(tcx, item))\n",
    replace: "        .filter(|&item| is_binding(tcx, item))\n",
    tests: ["test/declarations-bound-methods.test.ts"],
  },
  {
    name: "numbered-enum-named",
    breaks:
      "a `#[repr(u8)]` enum's variant is its name, `\"Empty\"`, where it's its number, `0`, as React's `EMPTY_ARRAY` is",
    file: "src/lower/bindings.rs",
    find: "    narrow\n        && adt.is_enum()\n",
    replace: "    false\n        && narrow\n        && adt.is_enum()\n",
    tests: [
      "test/lowering-repr-enums.test.ts",
      "-t",
      "a repr enum's variant is its number",
    ],
  },
  {
    name: "numbered-enum-wide",
    breaks:
      "a `#[repr(u64)]` enum's variant is a JS number, where a 64-bit integer is a BigInt, so a caller's name of one is shown as nothing",
    file: "src/lower/bindings.rs",
    find: "        Some(IntegerType::Fixed(int, _)) => int.size().bits() <= 32,\n",
    replace:
      "        Some(IntegerType::Fixed(int, _)) => int.size().bits() <= 64,\n",
    tests: [
      "test/lowering-repr-enums.test.ts",
      "-t",
      "a repr enum's variant is its number",
    ],
  },
  {
    name: "numbered-given-name-a-string",
    breaks:
      "a `#[repr(u8)]` enum's variant named `#[rust_js::name = \"UNTERMINATED\"]` is the string `'UNTERMINATED'`, where it's its number, its module's `const UNTERMINATED = 0;`, as React's cache's",
    file: "src/lower/bindings.rs",
    find: "        && (adt.variants().iter()).all(|v| v.fields.is_empty() && given_bool(tcx, v.def_id).is_none())\n",
    replace:
      "        && (adt.variants().iter()).all(|v| v.fields.is_empty() && given_name(tcx, v.def_id).is_none() && given_bool(tcx, v.def_id).is_none())\n",
    tests: [
      "test/lowering-repr-enums.test.ts",
      "-t",
      "a repr enum's variants are its module's consts",
    ],
  },
  {
    name: "any-length-a-strings",
    breaks:
      "an array's `get length` is taken for a JS string's, below `2^32 - 1`, so two steps below it lose their wrap",
    file: "src/lower/bindings.rs",
    find: "            .is_some_and(|this| this.peel_refs().is_str())\n",
    replace: "            .is_some()\n",
    tests: ["test/lowering-string-lengths.test.ts", "-t", "below a JS string"],
  },
  {
    name: "primitive-thread-local-camel",
    breaks: "`REACT_ELEMENT_TYPE` of a symbol is `reactElementType`",
    file: "src/lower/bindings.rs",
    find: "        DefKind::Const if thread_local_value(tcx, def_id).is_some_and(|held| primitive(tcx, held)) => name.to_string(),\n",
    replace: "        DefKind::Const if thread_local_value(tcx, def_id).is_some_and(|held| primitive(tcx, held) && false) => name.to_string(),\n",
    tests: [
      "test/lowering-names.test.ts",
      "-t",
      "a thread-local of a primitive is named",
    ],
  },
  {
    name: "symbol-not-primitive",
    breaks: "a symbol isn't taken for a primitive, `reactElementType`",
    file: "src/lower/bindings.rs",
    find: '                || written_types(tcx, adt.did()).as_ref().and_then(Written::keyword) == Some("symbol"))\n',
    replace: "                || false)\n",
    tests: [
      "test/lowering-names.test.ts",
      "-t",
      "a thread-local of a primitive is named",
    ],
  },
  {
    name: "directive-owner-stops-at-coroutine",
    breaks:
      "an `async fn`'s directive is its body's coroutine's, which no function writes",
    file: "src/lower/bindings.rs",
    find: "    while matches!(\n        tcx.coroutine_kind(owner),",
    replace: "    while false && matches!(\n        tcx.coroutine_kind(owner),",
    tests: ["test/modules.test.ts", "-t", "in a body is the body"],
  },
  {
    name: "tagged-unit-variant-string",
    breaks:
      'a discriminated union\'s variant without fields is its bare name, `"pending"`, not `{ status: "pending" }`',
    file: "src/lower/bindings.rs",
    find: "        Some(key) => js::Expr::object(vec![js::Prop::Field(key, name)]),",
    replace: "        Some(_) => name,",
    tests: ["test/lowering-structs-enums.test.ts", "-t", "discriminated union"],
  },
  {
    name: "event-method-receiver-lost",
    breaks: "an event's metadata-backed method loses its self receiver",
    file: "src/lower/bindings.rs",
    find: "        DefKind::AssocFn => tcx.associated_item(def_id).is_method() || named_this(),",
    replace: "        DefKind::AssocFn => named_this(),",
    tests: ["test/webapi-events.test.ts", "-t", "emit direct JavaScript"],
  },
  {
    name: "assoc-this-not-method",
    breaks:
      "a type's own function whose first parameter is `this`, `Mouse::widen(this)`, isn't a method: rust-js refuses it",
    file: "src/lower/bindings.rs",
    find: "        DefKind::AssocFn => tcx.associated_item(def_id).is_method() || named_this(),",
    replace:
      "        DefKind::AssocFn => tcx.associated_item(def_id).is_method(),",
    tests: ["test/jsx-events.test.ts", "-t", "tag's element"],
  },
  {
    name: "route-marks-written",
    breaks:
      "\`js::directive!\`'s and \`js::export_default!\`'s \`const _\` is written to the JS",
    file: "src/lower/bindings.rs",
    find: '    ["import", "directive", "export_default", "on_load", "names_field"]\n',
    replace: '    ["import", "on_load", "names_field"]\n',
    tests: ["test/modules.test.ts", "-t", "make a module a Next.js route"],
  },
  {
    name: "flatten-not-a-rest",
    breaks:
      "a flattened field is a field, `{ size, children, anchor }`, not `...anchor`",
    file: "src/lower/bindings.rs",
    find: "        .is_some_and(|field| is_flatten(tcx, field) || is_rest(tcx, field.ty(tcx, args).skip_normalization()))",
    replace:
      "        .is_some_and(|field| is_rest(tcx, field.ty(tcx, args).skip_normalization()))",
    tests: ["test/jsx-props.test.ts", "-t", "flattened struct"],
  },
  {
    name: "on-load-const-written",
    breaks:
      "`js::on_load!`'s `const _`, which holds its function, is written, `const _ = undefined`",
    file: "src/lower/bindings.rs",
    find: '["import", "directive", "export_default", "on_load", "names_field"]',
    replace: '["import", "directive", "export_default", "names_field"]',
    tests: ["test/modules.test.ts", "-t", "js::on_load!"],
  },
  {
    name: "optional-component-takes-dictionary",
    breaks:
      "a generic component of an `Option<JSX::Element>` takes a dictionary, `Shown(props, CDefault)`, which React never gives",
    file: "src/lower/bindings.rs",
    find: "        ty::TyKind::Adt(adt, args) if tcx.is_lang_item(adt.did(), LangItem::Option) => args.type_at(0),\n",
    replace: "",
    tests: ["test/lowering-bindings.test.ts", "-t", "optional element"],
  },
  {
    name: "given-variant-name-ignored",
    breaks:
      'a variant\'s `#[rust_js::name = "not-equal"]` is ignored: it\'s its Rust name, `"NotEqual"`, which JS never gives',
    file: "src/lower/bindings.rs",
    find: "    (given_name(tcx, variant.def_id))\n        .or_else(|| given_bool(tcx, variant.def_id).map(|b| b.to_string()))\n        .unwrap_or_else(|| default_name(tcx, variant.def_id))",
    replace: "    default_name(tcx, variant.def_id)",
    tests: [
      "test/js-builtins.test.ts",
      "-t",
      "Atomics are JS's, of a view of a shared buffer",
    ],
  },
  {
    name: "given-field-name-ignored",
    breaks:
      'a field\'s `#[rust_js::name = "react-dom"]` is ignored: its key is its Rust name, `react_dom`',
    file: "src/lower/bindings.rs",
    find: "    let key = name_in_js(tcx, field.did);\n",
    replace: "    let key = default_name(tcx, field.did);\n",
    tests: [
      "test/js-builtins.test.ts",
      "-t",
      "a struct whose JS is its JSON is stringified as it is",
    ],
  },
  {
    name: "named-callback-unread",
    breaks:
      "no hook names its function, so `useEffect(startTicking, [])` follows `const startTicking = () => ..`",
    file: "src/lower/bindings.rs",
    find: '    let path = [Symbol::intern("rust_js"), Symbol::intern("named_callback")];\n    tcx.get_attrs_by_path(def_id, &path).next().is_some()',
    replace:
      '    let path = [Symbol::intern("rust_js"), Symbol::intern("named_callback")];\n    tcx.get_attrs_by_path(def_id, &path).next().is_some() && false',
    tests: [
      "test/jsx-children.test.ts",
      "-t",
      "a closure a let names for a hook",
    ],
  },
  {
    name: "skips-falsy-unread",
    breaks:
      "no binding skips what's falsy, so classnames is given `test ? value : undefined`",
    file: "src/lower/bindings.rs",
    find: '    let path = [Symbol::intern("rust_js"), Symbol::intern("skips_falsy")];\n    tcx.get_attrs_by_path(def_id, &path).next().is_some()',
    replace:
      '    let path = [Symbol::intern("rust_js"), Symbol::intern("skips_falsy")];\n    tcx.get_attrs_by_path(def_id, &path).next().is_some() && false',
    tests: ["test/bindings.test.ts", "-t", "skips what's falsy"],
  },
  {
    name: "getter-export-unimported",
    breaks:
      "a getter of a module's export, `get node:path#delimiter`, isn't imported",
    file: "src/lower/bindings.rs",
    find: "            JsForm::Call(name) | JsForm::New(name) | JsForm::Get(name) if !is_method(tcx, def_id) => Some(name),\n",
    replace:
      "            JsForm::Call(name) | JsForm::New(name) if !is_method(tcx, def_id) => Some(name),\n",
    tests: ["test/bindings-require.test.ts", "-t", "a required binding"],
  },
  {
    name: "bool-tag-string",
    breaks:
      'a discriminated union\'s `#[rust_js::name = true]` tag is the string `"true"`',
    file: "src/lower/bindings.rs",
    find: "        Some(b) => js::Expr::bool(b),",
    replace: "        Some(b) => js::Expr::str(b.to_string()),",
    tests: [
      "test/lowering-structs-enums.test.ts",
      "-t",
      "tag may be a boolean",
    ],
  },
  {
    name: "trait-method-keyed-as-rust",
    breaks:
      "a std trait's method in std's dictionary is keyed as Rust spells it, `as_ref`, where a call through it asks `asRef`",
    file: "src/lower/bindings.rs",
    find: "    fn_name(tcx, super::recognition::trait_method(tcx, trait_id, name))\n",
    replace:
      "    let _ = super::recognition::trait_method(tcx, trait_id, name);\n    name.to_string()\n",
    tests: [
      "test/corpus.test.ts",
      "-t",
      "^(generic_as_ref|hash_values|generic_from_str).rs$",
    ],
  },
  {
    name: "repeated-name-allowed",
    breaks:
      "a `#[rust_js::name]` that repeats its item's camelCase name is let through, a second way to write it",
    file: "src/lower/bindings.rs",
    find: "            && given == default_name(tcx, def)\n",
    replace:
      "            && given == default_name(tcx, def)\n            && false\n",
    tests: [
      "test/bindings.test.ts",
      "-t",
      "a generator's camelCase is the compiler's",
    ],
  },
  {
    name: "thread-local-screaming",
    breaks:
      "a thread-local keeps its `SCREAMING_SNAKE` name, `let TASK_ID_COUNTER`, where JS names a variable camelCase",
    file: "src/lower/bindings.rs",
    find: "        DefKind::Const if thread_local_value(tcx, def_id).is_some() => variable_name(tcx, def_id, name),\n",
    replace:
      "        DefKind::Const if false && thread_local_value(tcx, def_id).is_some() => variable_name(tcx, def_id, name),\n",
    tests: [
      "test/lowering-names.test.ts",
      "-t",
      "thread-local's SCREAMING_SNAKE",
    ],
  },
  {
    name: "thread-local-component-lowercase",
    breaks:
      "a context's thread-local is `theme`, which JSX can't take as a component's name",
    file: "src/lower/bindings.rs",
    find: "        true => super::upper_first(&variable),\n",
    replace: "        true => variable,\n",
    tests: [
      "test/jsx-types.test.ts",
      "-t",
      "JSX context providers take any node",
    ],
  },
  {
    name: "thread-local-name-taken",
    breaks:
      "a thread-local named as another item of its module is renamed, `cache$1`, not kept as written",
    file: "src/lower/bindings.rs",
    find: "        true => name.to_string(),\n        false => named,\n",
    replace: "        true => named.clone(),\n        false => named,\n",
    tests: [
      "test/lowering-cells.test.ts",
      "-t",
      "an insert in a thread-local's closure",
    ],
  },
  {
    name: "thread-local-name-kept-for-private",
    breaks:
      "an exported thread-local keeps its Rust name beside a private function of its camelCase name, `EMPTY_CONTEXT_OBJECT`, which JS's importer of `emptyContextObject` doesn't read (ADR 0922)",
    file: "src/lower/bindings.rs",
    find: "            && (!exported || exported_by_name(tcx, other))\n",
    replace: "",
    tests: [
      "test/lowering-module-variables.test.ts",
      "-t",
      "an export keeps its name beside a private item of it",
    ],
  },
  {
    name: "private-item-exported-by-name",
    breaks:
      "a private item is taken for an export, so an exported thread-local beside a private function of its name keeps its Rust name (ADR 0922)",
    file: "src/lower/bindings.rs",
    find: "        && tcx.def_kind(tcx.parent(def_id)) == DefKind::Mod\n        && tcx.visibility(def_id).is_public()\n",
    replace: "        && tcx.def_kind(tcx.parent(def_id)) == DefKind::Mod\n",
    tests: [
      "test/lowering-module-variables.test.ts",
      "-t",
      "an export keeps its name beside a private item of it",
    ],
  },
  {
    name: "bound-global-renamed-lost",
    breaks: "`pub use js::object::keys as names` is `keys`, not `names`",
    file: "src/lower/bindings.rs",
    find: "        false => alias_name(tcx, use_item, ident),\n",
    replace: "        false => own(),\n",
    tests: [
      "test/lowering-bindings.test.ts",
      "-t",
      "a global binding a module names",
    ],
  },
  {
    name: "default-constant-a-mark",
    breaks:
      '`js::export_default! { const V: &str = "19.3.0"; }` is taken for the naming form\'s `const _`, and nothing is written',
    file: "src/lower/bindings.rs",
    find: "    if super::default_exports::held_by_default(tcx, def_id) {\n        return false;\n    }\n",
    replace: "",
    tests: [
      "test/modules.test.ts",
      "-t",
      "the constant's value where the macro holds it",
    ],
  },
  {
    name: "setter-value-optional",
    breaks:
      "a setter's `None` is taken for an optional argument left out, so `Error.prepareStackTrace = undefined` is refused",
    file: "src/lower/bindings.rs",
    find: "    if matches!(js_form(tcx, def_id), JsForm::Set(_)) {\n",
    replace: "    if false {\n",
    tests: ["test/node-bindings.test.ts", "-t", "stack trace hook is saved"],
  },
  {
    name: "rest-marked-anywhere",
    breaks:
      "`#[rust_js::rest]` is read of no parameter, and a function marked so takes the array whole",
    file: "src/lower/bindings.rs",
    find: "        .filter(move |(_, param)| tcx.hir_attrs(param.hir_id).iter().any(|attr| attr.path_matches(&path)))\n",
    replace:
      "        .filter(move |(_, param)| tcx.hir_attrs(param.hir_id).iter().all(|attr| attr.path_matches(&path)) && false)\n",
    tests: [
      "test/rest-parameters.test.ts",
      "-t",
      "rest slice is JS's rest parameter",
    ],
  },
  {
    name: "std-field-key-shared",
    breaks:
      "`Enumerate`'s field `count` shares its key with the `count` method `$stdIterator` gives it, which overwrites it (ADR 0912)",
    file: "src/lower/bindings.rs",
    find: '        true => format!("{key}$"),\n',
    replace: "        true => key,\n",
    tests: ["test/std-package.test.ts"],
  },
  {
    name: "use-name-attribute-dropped",
    breaks: "a `#[rust_js::name]` on a `pub use` that keeps its item's name is ignored: react-art's `export const PATH = Mode.Path;` and `export { default as TRANSFORM }`, not `Path` and `Transform`",
    file: "src/lower/bindings.rs",
    find: "        true => given_name(tcx, use_item).unwrap_or_else(own),\n",
    replace: "        true => own(),\n",
    tests: ["test/lowering-binding-paths.test.ts", "-t", "name attribute"],
  },
  {
    name: "field-symbol-name-unread",
    breaks: "a field a symbol names is named by its own name, `reason.response`, not `reason[RESPONSE_SYMBOL]`",
    file: "src/lower/bindings.rs",
    find: "    if let Some((item, span)) = symbol_name(tcx, field) {",
    replace: "    if let Some((item, span)) = symbol_name(tcx, field).filter(|_| false) {",
    tests: [
      "test/field-symbol-names.test.ts",
      "-t",
      "a field a symbol names is that symbol's key"
    ]
  },
  {
    name: "field-symbol-use-lowered",
    breaks: "the use beside a struct its symbol name makes rustc see is lowered, `const _ = (value) => {}`, JS no program wrote",
    file: "src/lower/bindings.rs",
    find: "    [\"import\", \"directive\", \"export_default\", \"on_load\", \"names_field\"]",
    replace: "    [\"import\", \"directive\", \"export_default\", \"on_load\"]",
    tests: [
      "test/field-symbol-names.test.ts",
      "-t",
      "a field a symbol names is that symbol's key"
    ]
  },
  {
    name: "proto-field-unrefused",
    breaks:
      "a struct's field named `__proto__` compiles, `this.__proto__ = proto` setting its prototype and a `None` left out reading it, where Rust reads its own field (rd754)",
    file: "src/lower/bindings.rs",
    find: '    if field_key(tcx, field).name() != Some("__proto__") {\n',
    replace: "    if true {\n",
    tests: ["test/corpus.test.ts", "-t", "^field_named_proto.rs$"],
  },
  {
    name: "proto-field-refused-by-its-rust-name",
    breaks:
      "a field named `__proto__` in Rust and given another JS name, `proto`, is refused, though its JS name is no accessor's (rd754)",
    file: "src/lower/bindings.rs",
    find: '    if field_key(tcx, field).name() != Some("__proto__") {\n',
    replace: '    if field.name.as_str() != "__proto__" {\n',
    tests: ["test/corpus.test.ts", "-t", "^field_named_proto_renamed.rs$"],
  },
  {
    name: "block-directive-allowed",
    breaks:
      "an `async` block's `js::directive!` is written nowhere, without an error",
    file: "src/lower/bindings.rs",
    find: "        if !owned\n",
    replace: "        if false && !owned\n",
    tests: ["test/modules.test.ts", "-t", "in a body is the body"],
  },
  {
    name: "directive-unwritten",
    breaks:
      'a client component\'s module has no \`"use client";\`, and Next.js renders it as a Server Component',
    file: "src/lower/bindings.rs",
    find: "            Some(directive) => directives.push(directive.to_string()),",
    replace: "            Some(_) => {}",
    tests: ["test/modules.test.ts", "-t", "make a module a Next.js route"],
  },
  {
    name: "then-export-allowed",
    breaks: "a module that exports `then` compiles, and `import()` of it calls it as `then(resolve, reject)` (#1068, ADR 1106)",
    file: "src/lower/bindings.rs",
    find: '    if name != "then" {\n        return false;\n    }\n',
    replace: "    if true {\n        return false;\n    }\n",
    tests: ["test/corpus.test.ts", "-t", "^then_export_refused.rs$"],
  },
  {
    name: "undefined-is-absent-of-the-webapi-crate",
    breaks: "each webapi binding is taken for a WebIDL operation, `scheduler.yield(options)`, Chrome's form no WebIDL declares, among them, given its trailing `undefined` where `#[rust_js::undefined_is_absent]` doesn't say it takes one as none (#1842)",
    file: "src/lower/bindings.rs",
    find: "    let path = [Symbol::intern(\"rust_js\"), Symbol::intern(\"undefined_is_absent\")];\n    tcx.get_attrs_by_path(def_id, &path).next().is_some()\n",
    replace: "    tcx.crate_name(def_id.krate).as_str() == \"webapi\"\n",
    tests: ["test/none-given.test.ts", "-t", "a WebIDL operation's trailing None"],
  },
];
