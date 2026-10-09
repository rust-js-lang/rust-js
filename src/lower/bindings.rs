//! Decode the binding language independently of call lowering.

use crate::js;
use rustc_ast::LitKind;
use rustc_hir::attrs::lang_items::LangItem;
use rustc_hir::def::{DefKind, Res};
use rustc_hir::{ExprKind, ItemKind, Stmt, StmtKind};
use rustc_middle::ty::{self, FieldDef, Ty, TyCtxt, VariantDef};
use rustc_span::def_id::{CRATE_MOD_ID, DefId, LocalModId};
use rustc_span::{Span, Symbol, sym};

/// Validate tool bindings even if no function calls them. A malformed binding
/// must not silently become an ordinary Rust function with an unreachable body.
pub(super) fn validate(tcx: TyCtxt<'_>) -> bool {
    let mut valid = true;
    for def in tcx.hir_crate_items(()).definitions() {
        // A derive of a discriminated union with an `otherwise` would take
        // the object it holds for a variant of its own (ADR 0284).
        if matches!(tcx.def_kind(def), DefKind::Impl { of_trait: true })
            && super::recognition::known_derive(tcx, def.to_def_id())
            && let ty::Adt(adt, _) = tcx.type_of(def).instantiate_identity().skip_normalization().kind()
            && adt.is_enum()
            && adt.variants().iter().any(|v| is_tagged_otherwise(tcx, adt.did(), v))
        {
            let tr = tcx
                .impl_trait_ref(def)
                .instantiate_identity()
                .skip_normalization()
                .def_id;
            let message = format!(
                "rust-js does not support deriving `{}` of a discriminated union with an `otherwise` variant yet: it would take the object it holds for a variant of its own",
                tcx.item_name(tr)
            );
            tcx.dcx().span_err(tcx.def_span(def), message);
            valid = false;
        }
        let attrs: Vec<_> = tcx
            .get_attrs_by_path(def.to_def_id(), &[Symbol::intern("rust_js"), sym::link_name])
            .collect();
        for attr in &attrs {
            if attrs.len() != 1
                || attr.value_str().is_none()
                || !matches!(tcx.def_kind(def), DefKind::Fn | DefKind::AssocFn)
            {
                tcx.dcx().span_err(
                    attr.span(),
                    "rust-js: a binding needs one `#[rust_js::link_name = \"...\"]` on a function or method",
                );
                valid = false;
            }
        }
        // What `#[rust_js::nullable(..)]` names is a parameter of its own,
        // an `Option`, whose `None` can be `null`.
        if matches!(tcx.def_kind(def), DefKind::Fn | DefKind::AssocFn) {
            let idents = tcx.fn_arg_idents(def.to_def_id());
            let inputs = tcx.fn_sig(def).skip_binder().skip_binder().inputs();
            for name in nullable_params(tcx, def.to_def_id()) {
                let at = idents
                    .iter()
                    .position(|ident| ident.is_some_and(|ident| ident.name == name));
                if !at.is_some_and(
                    |i| matches!(inputs[i].kind(), ty::Adt(adt, _) if tcx.is_lang_item(adt.did(), LangItem::Option)),
                ) {
                    let message = format!(
                        "rust-js: `#[rust_js::nullable({name})]` names no parameter of this function that's an `Option`"
                    );
                    tcx.dcx().span_err(tcx.def_span(def), message);
                    valid = false;
                }
            }
        }
        // A discriminated union's variants are objects of named fields, none
        // of them its tag's (ADR 0284), but its last, which may be any other
        // object, `#[rust_js::otherwise]`, a variant of one field: the object.
        if matches!(tcx.def_kind(def), DefKind::Enum)
            && let Some(key) = declared_tag(tcx, def.to_def_id())
        {
            let variants = tcx.adt_def(def).variants();
            for (index, variant) in variants.iter().enumerate() {
                let tuple = matches!(variant.ctor_kind(), Some(rustc_hir::def::CtorKind::Fn));
                if is_otherwise(tcx, variant.def_id) {
                    if index + 1 != variants.len() || !tuple || variant.fields.len() != 1 {
                        let message = format!(
                            "rust-js: `#[rust_js::tag = \"{key}\"]`'s `otherwise` variant is its last, of one field, `Other(&'static JsObject)`: the object whose tag is none of the others'"
                        );
                        tcx.dcx().span_err(tcx.def_span(variant.def_id), message);
                        valid = false;
                    }
                    continue;
                }
                let clash = variant.fields.iter().any(|f| field_key(tcx, f) == key);
                if tuple || clash {
                    let what = if tuple {
                        "a tuple variant, whose fields have no names"
                    } else {
                        "a field of the tag's name"
                    };
                    let message = format!(
                        "rust-js: `#[rust_js::tag = \"{key}\"]`'s enum has {what}: each variant's fields are named properties beside `{key}`"
                    );
                    tcx.dcx().span_err(tcx.def_span(variant.def_id), message);
                    valid = false;
                }
            }
        }
        // Two fields that are one JS property would overwrite each other.
        if matches!(tcx.def_kind(def), DefKind::Struct | DefKind::Enum) {
            for variant in tcx.adt_def(def).variants() {
                let mut keys: Vec<(String, Symbol)> = Vec::new();
                for field in variant.fields.iter() {
                    let key = field_key(tcx, field);
                    if let Some((_, other)) = keys.iter().find(|(k, _)| *k == key) {
                        let message = format!("rust-js: fields `{other}` and `{}` are both `{key}` in JS", field.name);
                        tcx.dcx().span_err(tcx.def_span(field.did), message);
                        valid = false;
                    }
                    keys.push((key, field.name));
                }
            }
        }
    }
    valid
}

/// What a JS function or global declared in an `extern` block is called:
/// its `#[link_name]`, or its Rust name. A dotted name (`console.log`) is
/// a path from a global.
pub(super) fn js_name(tcx: TyCtxt<'_>, def_id: DefId) -> String {
    if let Some(name) = tool_link_name(tcx, def_id) {
        return name.to_string();
    }
    match tcx.codegen_fn_attrs(def_id).symbol_name {
        Some(name) => name.to_string(),
        None => tcx.item_name(def_id).to_string(),
    }
}

/// A binding written as an ordinary function, which can be generic, as an
/// `extern` one can't (ADR 0039): `#[rust_js::link_name = "react#useState"]`.
/// Its body is never compiled.
pub(super) fn tool_link_name(tcx: TyCtxt<'_>, def_id: DefId) -> Option<Symbol> {
    if !matches!(tcx.def_kind(def_id), DefKind::Fn | DefKind::AssocFn) {
        return None;
    }
    tcx.get_attrs_by_path(def_id, &[Symbol::intern("rust_js"), sym::link_name])
        .next()?
        .value_str()
}

/// Is this function or static JS's: in an `extern` block, or a
/// `#[rust_js::link_name]` function?
pub(super) fn is_binding(tcx: TyCtxt<'_>, def_id: DefId) -> bool {
    tcx.is_foreign_item(def_id) || tool_link_name(tcx, def_id).is_some()
}

/// A JS function whose first parameter is named `this` is a method:
/// `f(x, a)` calls `x.f(a)`, a type's own as well, `Mouse::widen(this)`.
/// So is a Rust method's `self`.
pub(super) fn is_method(tcx: TyCtxt<'_>, def_id: DefId) -> bool {
    let named_this = || matches!(tcx.fn_arg_idents(def_id).first(), Some(Some(ident)) if ident.name.as_str() == "this");
    match tcx.def_kind(def_id) {
        DefKind::Fn => named_this(),
        DefKind::AssocFn => tcx.associated_item(def_id).is_method() || named_this(),
        _ => false,
    }
}

/// How a call to a JS function is written, from its `#[link_name]` (ADR 0024).
pub(super) enum JsForm {
    /// `name(..)`, or `this.name(..)` for a method.
    Call(String),
    /// `this.name`.
    Get(String),
    /// `this.name = value`.
    Set(String),
    /// `new Name(..)`.
    New(String),
    /// `this` itself: an unchecked cast.
    This,
    /// `this(..)`: `this` is a JS function, like React's `setCount`.
    CallThis,
    /// A JSX element (ADR 0040): `<div>`, `<>`, an imported component like
    /// `<react#StrictMode>`, or `<*>` for the component given first.
    Jsx(String),
    /// `prop className`: a JSX attribute of `this`, the element being built,
    /// or a field of `this`, an object being built. Just `prop`: the name
    /// comes first, as a string literal.
    Prop(Option<String>),
    /// `{}`, or `{__html}`: an object, with the arguments as its fields, in
    /// order. With `prop`, it builds `style={{ color: "red" }}` and options.
    Object(Vec<String>),
    /// `this instanceof Class`: a checked one, as a `bool`.
    InstanceOf(String),
    /// `get []`: `this[key]`, a property by a name given (ADR 0225).
    GetIndex,
    /// `in []`: `key in this`, whether it has a property by a name given.
    In,
    /// `!!`: `!!this`, whether it's truthy.
    Truthy,
    /// `set []`: `this[key] = value`.
    SetIndex,
    /// `import()`: the item given, of its module loaded when it's asked
    /// for; `import(*)`, the module whose default export it is (ADR 0304).
    Import { module: bool },
}

pub(super) fn js_form(tcx: TyCtxt<'_>, def_id: DefId) -> JsForm {
    let name = js_name(tcx, def_id);
    match name.as_str() {
        "this" => return JsForm::This,
        "this()" => return JsForm::CallThis,
        "prop" => return JsForm::Prop(None),
        "get []" => return JsForm::GetIndex,
        "in []" => return JsForm::In,
        "!!" => return JsForm::Truthy,
        "set []" => return JsForm::SetIndex,
        "import()" => return JsForm::Import { module: false },
        "import(*)" => return JsForm::Import { module: true },
        _ => {}
    }
    if let Some(tag) = name.strip_prefix('<').and_then(|t| t.strip_suffix('>')) {
        return JsForm::Jsx(tag.to_string());
    }
    if let Some(keys) = name.strip_prefix('{').and_then(|t| t.strip_suffix('}')) {
        let keys = keys.split(',').map(str::trim).filter(|k| !k.is_empty());
        return JsForm::Object(keys.map(str::to_string).collect());
    }
    if let Some(prop) = name.strip_prefix("prop ") {
        return JsForm::Prop(Some(prop.to_string()));
    }
    // Each prefix, and the form of what follows it.
    type Form = fn(String) -> JsForm;
    let forms: [(&str, Form); 4] = [
        ("get ", JsForm::Get),
        ("set ", JsForm::Set),
        ("new ", JsForm::New),
        ("instanceof ", JsForm::InstanceOf),
    ];
    for (prefix, form) in forms {
        if let Some(rest) = name.strip_prefix(prefix) {
            return form(rest.to_string());
        }
    }
    JsForm::Call(name)
}

/// The path a JS item is reached by, if it isn't a method or a property:
/// `document`, `console.log`, `Event` for `new Event`, or an import like
/// `node:path#join`.
pub(super) fn js_path(tcx: TyCtxt<'_>, def_id: DefId) -> Option<String> {
    match tcx.def_kind(def_id) {
        DefKind::Static { .. } => Some(js_name(tcx, def_id)),
        DefKind::Fn | DefKind::AssocFn => match js_form(tcx, def_id) {
            JsForm::Call(name) | JsForm::New(name) if !is_method(tcx, def_id) => Some(name),
            // A class to test against is a global or an import like any other.
            JsForm::InstanceOf(class) => Some(class),
            // So is a component: `<react#StrictMode>`.
            JsForm::Jsx(tag) if tag.contains('#') => Some(tag),
            _ => None,
        },
        _ => None,
    }
}

/// `#[rust_js::name = ".."]`: what an item is in JS, as written.
fn given_name(tcx: TyCtxt<'_>, def_id: DefId) -> Option<String> {
    let attr = tcx
        .get_attrs_by_path(def_id, &[Symbol::intern("rust_js"), sym::name])
        .next()?;
    attr.value_str().map(|s| s.to_string())
}

/// What a variant without fields is in JS: its name, or its
/// `#[rust_js::name = ".."]`, for a string that isn't a Rust name, as in
/// `enum Mode { #[rust_js::name = "hidden"] Hidden, .. }` (ADR 0039).
/// A unit struct's `#[rust_js::name]`, the string it is, as a fieldless
/// variant is (ADR 0013): `webapi`'s event names (ADR 0223). Without one,
/// it's `undefined`, holding nothing, as `()` does.
pub(super) fn unit_name(tcx: TyCtxt<'_>, def_id: DefId) -> Option<String> {
    given_name(tcx, def_id)
}

pub(super) fn variant_name(tcx: TyCtxt<'_>, variant: &VariantDef) -> String {
    given_name(tcx, variant.def_id).unwrap_or_else(|| variant.name.to_string())
}

/// The property an enum's variant is told by: `TAG` (ADR 0033), or the
/// one it names, `#[rust_js::tag = "status"]`, a discriminated union's,
/// each variant an object of it, its fieldless ones too (ADR 0284).
pub(super) fn tag_key(tcx: TyCtxt<'_>, adt: DefId) -> String {
    declared_tag(tcx, adt).unwrap_or_else(|| "TAG".to_string())
}

/// An enum's `#[rust_js::tag = ".."]`, if it has one.
pub(super) fn declared_tag(tcx: TyCtxt<'_>, adt: DefId) -> Option<String> {
    let path = [Symbol::intern("rust_js"), Symbol::intern("tag")];
    Some(tcx.get_attrs_by_path(adt, &path).next()?.value_str()?.to_string())
}

/// Whether `variant` is a discriminated union's `otherwise`: any object
/// whose tag is none of its others', the object itself (ADR 0284).
pub(super) fn is_tagged_otherwise(tcx: TyCtxt<'_>, adt: DefId, variant: &VariantDef) -> bool {
    declared_tag(tcx, adt).is_some() && is_otherwise(tcx, variant.def_id)
}

/// A variant without fields: its name (ADR 0013), or of a discriminated
/// union an object of it, `{ status: "pending" }` (ADR 0284).
pub(super) fn unit_variant(tcx: TyCtxt<'_>, adt: DefId, variant: &VariantDef) -> js::Expr {
    let name = js::Expr::str(variant_name(tcx, variant));
    match declared_tag(tcx, adt) {
        Some(key) => js::Expr::object(vec![js::Prop::Field(key, name)]),
        None => name,
    }
}

/// Whether an enum is untagged, `#[rust_js::untagged]`: a value is its
/// payload, as TS's `string | Blob` is (ADR 0214).
pub(super) fn is_untagged(tcx: TyCtxt<'_>, def_id: DefId) -> bool {
    let path = [Symbol::intern("rust_js"), Symbol::intern("untagged")];
    tcx.get_attrs_by_path(def_id, &path).next().is_some()
}

/// `#[rust_js::skips_falsy]`: a function that skips what's falsy, as
/// classnames does, given an argument shown only if a test holds as `test &&
/// value` (ADR 0221).
pub(super) fn skips_falsy(tcx: TyCtxt<'_>, def_id: DefId) -> bool {
    let path = [Symbol::intern("rust_js"), Symbol::intern("skips_falsy")];
    tcx.get_attrs_by_path(def_id, &path).next().is_some()
}

/// `#[rust_js::named_callback]`: a hook whose function a `let` names is
/// written where it's given, named, as React's effects are (ADR 0297).
pub(super) fn named_callback(tcx: TyCtxt<'_>, def_id: DefId) -> bool {
    let path = [Symbol::intern("rust_js"), Symbol::intern("named_callback")];
    tcx.get_attrs_by_path(def_id, &path).next().is_some()
}

/// `#[rust_js::position]`: a binding that gives a position in what it's
/// called on, or -1, as `indexOf` does (ADR 0302).
pub(super) fn position(tcx: TyCtxt<'_>, def_id: DefId) -> bool {
    let path = [Symbol::intern("rust_js"), Symbol::intern("position")];
    tcx.get_attrs_by_path(def_id, &path).next().is_some()
}

/// `#[rust_js::cannot_throw]`: a binding that can't throw, as reading a
/// data property, React's `ref.current`, can't (ADR 0301).
pub(super) fn cannot_throw(tcx: TyCtxt<'_>, def_id: DefId) -> bool {
    let path = [Symbol::intern("rust_js"), Symbol::intern("cannot_throw")];
    tcx.get_attrs_by_path(def_id, &path).next().is_some()
}

/// Whether a binding gives `undefined`, whatever JS's types say of it,
/// `#[rust_js::returns_undefined]`, as React's setter does (ADR 0040).
pub(super) fn returns_undefined(tcx: TyCtxt<'_>, def_id: DefId) -> bool {
    let path = [Symbol::intern("rust_js"), Symbol::intern("returns_undefined")];
    tcx.get_attrs_by_path(def_id, &path).next().is_some()
}

/// Whether a binding's last parameter, a slice, is JS's rest arguments,
/// `#[rust_js::variadic]` (ADR 0221).
pub(super) fn is_variadic(tcx: TyCtxt<'_>, def_id: DefId) -> bool {
    let path = [Symbol::intern("rust_js"), Symbol::intern("variadic")];
    tcx.get_attrs_by_path(def_id, &path).next().is_some()
}

/// Whether an untagged enum's variant is what its others aren't,
/// `#[rust_js::otherwise]` (ADR 0214).
pub(super) fn is_otherwise(tcx: TyCtxt<'_>, def_id: DefId) -> bool {
    let path = [Symbol::intern("rust_js"), Symbol::intern("otherwise")];
    tcx.get_attrs_by_path(def_id, &path).next().is_some()
}

/// A JS object type's test, `#[rust_js::test = "react#isValidElement"]`:
/// the function that says a value is one, where no class does (ADR 0214).
pub(super) fn test_of(tcx: TyCtxt<'_>, def_id: DefId) -> Option<String> {
    let path = [Symbol::intern("rust_js"), Symbol::intern("test")];
    let test = tcx.get_attrs_by_path(def_id, &path).next()?.value_str()?;
    Some(test.to_string())
}

/// A JS object type's class, as `instanceof` names it: its
/// `#[rust_js::name = ".."]`, `WebAssembly.Module` of `WebAssemblyModule`, or its name.
pub(super) fn class_name(tcx: TyCtxt<'_>, def_id: DefId) -> String {
    given_name(tcx, def_id).unwrap_or_else(|| tcx.item_name(def_id).to_string())
}

/// `js::camel_case!();` at the crate root: the crate's own functions and
/// fields are camelCase in JS, as its variables are (ADRs 0046 and 0110).
pub(super) fn camel_case_crate(tcx: TyCtxt<'_>) -> bool {
    marks(tcx, CRATE_MOD_ID, "camel_case").next().is_some()
}

/// A module's `js::import!` and `js::camel_case!`: each a `const _` with
/// the attribute of rust-js's tool its macro writes, `#[rust_js::import]`,
/// as stable Rust has no inner one of a tool (ADR 0110).
pub(super) fn marks<'tcx>(
    tcx: TyCtxt<'tcx>,
    module: LocalModId,
    name: &str,
) -> impl Iterator<Item = &'tcx rustc_hir::Attribute> {
    let path = [Symbol::intern("rust_js"), Symbol::intern(name)];
    tcx.hir_module_free_items(module).flat_map(move |item| {
        tcx.get_attrs_by_path(item.owner_id.to_def_id(), &path)
            .collect::<Vec<_>>()
    })
}

/// Is `def_id` a `js::on_load!`'s function, whose body is what its module
/// runs when it's loaded (ADR 0267)?
pub(super) fn is_on_load(tcx: TyCtxt<'_>, def_id: DefId) -> bool {
    (tcx.get_attrs_by_path(def_id, &[Symbol::intern("rust_js"), Symbol::intern("on_load")]))
        .next()
        .is_some()
}

/// Is `def_id` a `js::import!`'s, `js::camel_case!`'s, `js::directive!`'s,
/// `js::export_default!`'s or `js::on_load!`'s `const _`, which is rust-js's
/// to read, and has nothing to write?
pub(super) fn is_mark(tcx: TyCtxt<'_>, def_id: DefId) -> bool {
    ["import", "camel_case", "directive", "export_default", "on_load"]
        .iter()
        .any(|name| {
            tcx.get_attrs_by_path(def_id, &[Symbol::intern("rust_js"), Symbol::intern(name)])
                .next()
                .is_some()
        })
}

/// Whether `def_id` is a React component, as `jsx!` takes one: a
/// capitalized function of its props, or none, that returns react's
/// `Element`, or an `Option` of one (ADR 0286). React calls it with its
/// props and a value of its own, never a drop (ADR 0199) or a dictionary
/// (ADR 0201).
pub(crate) fn is_component(tcx: TyCtxt<'_>, def_id: DefId) -> bool {
    if tcx.def_kind(def_id) != DefKind::Fn {
        return false;
    }
    // Capitalized, as JSX tells a component from an element: no std name's.
    let name = tcx.item_name(def_id);
    if !name.as_str().starts_with(char::is_uppercase) {
        return false;
    }
    let sig = tcx
        .fn_sig(def_id)
        .instantiate_identity()
        .skip_normalization()
        .skip_binder();
    let element = [Symbol::intern("rust_js"), Symbol::intern("jsx_element")];
    let is_element = |ty: Ty<'_>| matches!(ty.kind(), ty::TyKind::Adt(adt, _) if tcx.get_attrs_by_path(adt.did(), &element).next().is_some());
    let rendered = match sig.output().kind() {
        ty::TyKind::Adt(adt, args) if tcx.is_lang_item(adt.did(), LangItem::Option) => args.type_at(0),
        _ => sig.output(),
    };
    sig.inputs().len() <= 1 && is_element(rendered)
}

/// Whether `ty` is react's `Rest`, the props a component's struct doesn't
/// name: `...rest` of its destructured props (ADR 0195).
pub(super) fn is_rest(tcx: TyCtxt<'_>, ty: Ty<'_>) -> bool {
    let path = [Symbol::intern("rust_js"), Symbol::intern("rest_props")];
    matches!(ty.kind(), ty::TyKind::Adt(adt, _) if tcx.get_attrs_by_path(adt.did(), &path).next().is_some())
}

/// Whether `field` is flattened, `#[rust_js::flatten]`: its struct's fields
/// are its parent's in JS, as `A & B`'s are in TypeScript (ADR 0204).
pub(super) fn is_flatten(tcx: TyCtxt<'_>, field: &FieldDef) -> bool {
    let path = [Symbol::intern("rust_js"), Symbol::intern("flatten")];
    tcx.get_attrs_by_path(field.did, &path).next().is_some()
}

/// Whether `def_id` is react's `__omitted`, what `jsx!` gives a prop it isn't
/// given: `undefined` (ADR 0213).
pub(super) fn is_omitted(tcx: TyCtxt<'_>, def_id: DefId) -> bool {
    let path = [Symbol::intern("rust_js"), Symbol::intern("omitted")];
    tcx.get_attrs_by_path(def_id, &path).next().is_some()
}

/// Whether a field of an `Option` is declared to take `null` too,
/// `#[rust_js::nullable]` (ADR 0196).
pub(super) fn is_nullable(tcx: TyCtxt<'_>, field: &FieldDef) -> bool {
    let path = [Symbol::intern("rust_js"), Symbol::intern("nullable")];
    tcx.get_attrs_by_path(field.did, &path).next().is_some()
}

/// The parameters a binding names `#[rust_js::nullable(..)]`, each
/// `T | null` as a nullable field is (ADR 0275). On the function, since a
/// parameter's own attributes are kept neither in an `extern` block nor
/// in another crate's metadata.
pub(super) fn nullable_params(tcx: TyCtxt<'_>, def_id: DefId) -> Vec<Symbol> {
    let path = [Symbol::intern("rust_js"), Symbol::intern("nullable")];
    tcx.get_attrs_by_path(def_id, &path)
        .filter_map(|attr| attr.meta_item_list())
        .flatten()
        .filter_map(|item| item.ident().map(|ident| ident.name))
        .collect()
}

/// A props field's default, `#[rust_js::default]` (ADR 0212): `Some(None)`
/// of its type's `Default`, `Some(Some(..))` of the literal it says,
/// `"_self"` or `true`.
pub(super) fn field_default(tcx: TyCtxt<'_>, field: &FieldDef) -> Option<Option<LitKind>> {
    let path = [Symbol::intern("rust_js"), Symbol::intern("default")];
    tcx.get_attrs_by_path(field.did, &path)
        .next()
        .map(|attr| attr.value_lit().map(|lit| lit.kind))
}

/// The `const` a props field's default names, `#[rust_js::default(NAME)]`
/// (ADR 0212): an object of literals, which no attribute can say.
pub(super) fn field_default_const(tcx: TyCtxt<'_>, field: &FieldDef) -> Option<Symbol> {
    let path = [Symbol::intern("rust_js"), Symbol::intern("default")];
    let attr = tcx.get_attrs_by_path(field.did, &path).next()?;
    match attr.meta_item_list()?.as_slice() {
        [name] => name.ident().map(|ident| ident.name),
        _ => None,
    }
}

/// Whether field `i` of `ty`, a struct, is flattened (ADR 0204).
pub(super) fn is_flatten_field(tcx: TyCtxt<'_>, ty: Ty<'_>, i: usize) -> bool {
    matches!(ty.kind(), ty::Adt(adt, _) if adt.is_struct()
        && adt.non_enum_variant().fields.iter().nth(i).is_some_and(|field| is_flatten(tcx, field)))
}

/// Whether field `i` of `ty`, a struct, holds what JS's `...rest` does: a
/// `Rest` (ADR 0195), or a flattened struct, typed (ADR 0204).
pub(super) fn is_rest_field<'tcx>(tcx: TyCtxt<'tcx>, ty: Ty<'tcx>, i: usize) -> bool {
    let ty::Adt(adt, args) = ty.kind() else { return false };
    if !adt.is_struct() {
        return false;
    }
    adt.non_enum_variant()
        .fields
        .iter()
        .nth(i)
        .is_some_and(|field| is_flatten(tcx, field) || is_rest(tcx, field.ty(tcx, args).skip_normalization()))
}

/// Whether `ty` is a struct with a flattened field, which JS holds flat:
/// made only as JSX's props, which flatten it (ADR 0204).
pub(super) fn has_flatten(tcx: TyCtxt<'_>, ty: Ty<'_>) -> bool {
    matches!(ty.kind(), ty::Adt(adt, _) if adt.is_struct() && adt.non_enum_variant().fields.iter().any(|f| is_flatten(tcx, f)))
}

/// Whether `def` builds an element as `jsx!` writes it: a tag's, `div()`, or
/// a prop's setter on one. Only the compiler calls these, and building an
/// element runs nothing (ADR 0040).
pub(super) fn is_element_builder(tcx: TyCtxt<'_>, def: DefId) -> bool {
    match js_form(tcx, def) {
        JsForm::Jsx(_) => true,
        JsForm::Prop(_) => {
            let ty = tcx
                .fn_sig(def)
                .instantiate_identity()
                .skip_normalization()
                .skip_binder()
                .output();
            matches!(ty.kind(), ty::Adt(adt, _) if tcx.get_attrs_by_path(adt.did(), &[Symbol::intern("rust_js"), Symbol::intern("jsx_element")]).next().is_some())
        }
        _ => false,
    }
}

/// Whether `id` is react's `ReactNode`, what React renders as a child: a
/// sealed trait, its types std's and React's (ADR 0201).
pub(super) fn is_jsx_node(tcx: TyCtxt<'_>, id: DefId) -> bool {
    let path = [Symbol::intern("rust_js"), Symbol::intern("jsx_node")];
    tcx.get_attrs_by_path(id, &path).next().is_some()
}

/// A module's `js::export_default!(page)`: the function its `const _`
/// names, `let _ = page;`, and where it's written (ADR 0192).
pub(super) fn default_exports(tcx: TyCtxt<'_>, module: LocalModId) -> Vec<(Option<DefId>, Span)> {
    let path = [Symbol::intern("rust_js"), Symbol::intern("export_default")];
    tcx.hir_module_free_items(module)
        .filter(|item| tcx.get_attrs_by_path(item.owner_id.to_def_id(), &path).next().is_some())
        .map(|item| {
            let def = item.owner_id.def_id;
            // `use page as _;`: a generic function's too, which Rust can't
            // name as a value unless it's given its types.
            let named = match tcx.hir_body_owned_by(def).value.kind {
                ExprKind::Block(block, _) => match block.stmts {
                    [
                        Stmt {
                            kind: StmtKind::Item(item),
                            ..
                        },
                    ] => match tcx.hir_item(*item).kind {
                        // A function, or a `thread_local!`'s const, a memo'd
                        // component's (ADR 0248).
                        ItemKind::Use(path, _) => match path.res.value_ns {
                            Some(Res::Def(DefKind::Fn | DefKind::Const { .. }, function)) => Some(function),
                            _ => None,
                        },
                        _ => None,
                    },
                    _ => None,
                },
                _ => None,
            };
            (named, tcx.def_span(def))
        })
        .collect()
}

/// What an item of this crate is called in JS: its `#[rust_js::name]`, or
/// its Rust name, camelCase in a `camel_case` crate.
fn name_in_js(tcx: TyCtxt<'_>, def_id: DefId, name: &str) -> String {
    match given_name(tcx, def_id) {
        Some(given) => given,
        None if def_id.is_local() && camel_case_crate(tcx) => super::camel_case(name),
        None => name.to_string(),
    }
}

/// A function's or `const`'s JS name.
pub(super) fn fn_name(tcx: TyCtxt<'_>, def_id: DefId) -> String {
    name_in_js(tcx, def_id, tcx.item_name(def_id).as_str())
}

/// A struct's or a variant's field, as a JS property.
pub(super) fn field_key(tcx: TyCtxt<'_>, field: &FieldDef) -> String {
    name_in_js(tcx, field.did, field.name.as_str())
}

/// What a JS module exports under a name, `("node:path", "join")`, or
/// `"default"` or `"*"` for its default export or the module itself.
pub(super) type Export = (String, String);

/// An import from a JS module (ADR 0028): `"@codemirror/state#EditorState.create"`
/// is the export `("@codemirror/state", "EditorState")`, then the rest of
/// the path, `".create"`. The last `#` splits them, since a module's name
/// can start with one (Node's `#internal`).
pub(super) fn js_import(path: &str) -> Option<(Export, &str)> {
    let (from, path) = path.rsplit_once('#')?;
    let export = path.split('.').next().unwrap_or_default();
    (!from.is_empty() && !export.is_empty()).then(|| ((from.to_string(), export.to_string()), &path[export.len()..]))
}

/// What a default or namespace import is called: after its module, as in
/// ReScript. `./greet.js` is `greet`, and `@codemirror/lang-rust` is `langRust`.
pub(super) fn module_binding(from: &str) -> String {
    let file = from.rsplit(['/', ':']).find(|s| !s.is_empty()).unwrap_or_default();
    let stem = file.split('.').next().unwrap_or_default();
    let words = stem
        .split(|c: char| !c.is_ascii_alphanumeric() && c != '_' && c != '$')
        .filter(|w| !w.is_empty());
    let mut name = String::new();
    for (i, word) in words.enumerate() {
        let mut chars = word.chars();
        if i > 0
            && let Some(first) = chars.next()
        {
            name.push(first.to_ascii_uppercase());
        }
        name.extend(chars);
    }
    if name.is_empty() || name.starts_with(|c: char| c.is_ascii_digit()) {
        format!("_{name}")
    } else {
        name
    }
}
