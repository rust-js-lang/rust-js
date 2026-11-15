//! Each binding validated, even where no function calls it, so a malformed
//! one is refused, not an ordinary Rust function of an unreachable body:
//! split by concept from bindings.rs (#1334).

use super::*;

/// Validate tool bindings even if no function calls them. A malformed binding
/// must not silently become an ordinary Rust function with an unreachable body.
pub(in crate::lower) fn validate(tcx: TyCtxt<'_>) -> bool {
    let mut valid = given_names_differ(tcx);
    // A closure's parameter's `#[rust_js::nullable(..)]` names its own
    // `Option`s; a function's is on the function (ADR 0861, amended).
    for owner in tcx.hir_body_owners() {
        for (span, message) in nullable::nullable_parameters_refused(tcx, owner) {
            tcx.dcx().span_err(span, message);
            valid = false;
        }
    }
    for def in tcx.hir_crate_items(()).definitions() {
        // A derive of a discriminated union with an `otherwise` would take
        // the object it holds for a variant of its own (ADR 0284).
        if matches!(tcx.def_kind(def), DefKind::Impl { of_trait: true })
            && crate::lower::recognition::known_derive(tcx, def.to_def_id())
            && let ty::Adt(adt, _) = tcx.type_of(def).instantiate_identity().skip_normalization().kind()
            && adt.is_enum()
            && adt.variants().iter().any(|v| is_tagged_otherwise(tcx, adt.did(), v))
        {
            let tr = tcx
                .impl_trait_ref(def)
                .instantiate_identity()
                .skip_normalization()
                .def_id;
            let what = format!(
                "deriving `{}` of a discriminated union with an `otherwise` variant",
                tcx.item_name(tr)
            );
            let why = "it would take the object it holds for a variant of its own";
            crate::lower::refusal::refuse(
                tcx,
                tcx.def_span(def),
                crate::lower::refusal::Refused::Because(&what, why),
            );
            valid = false;
        }
        // A numbered enum's variant's name is its `const`'s, a JS name
        // (ADR 0722, amended).
        if tcx.def_kind(def) == DefKind::Enum && numbered(tcx, tcx.adt_def(def)) {
            for variant in tcx.adt_def(def).variants() {
                if let Some(given) = given_name(tcx, variant.def_id)
                    && !crate::names::is_identifier(&given)
                {
                    let message = format!(
                        "rust-js: `{}`'s `#[rust_js::name]` names its module's `const` of its number, as a `#[repr(..)]` enum's variant is one, so it's a JS name: `{given}` isn't one",
                        tcx.item_name(variant.def_id)
                    );
                    tcx.dcx().span_err(tcx.def_span(variant.def_id), message);
                    valid = false;
                }
            }
        }
        let attrs: Vec<_> = tcx
            .get_attrs_by_path(def.to_def_id(), &[Symbol::intern("rust_js"), sym::link_name])
            .collect();
        for attr in &attrs {
            if attrs.len() != 1
                || attr.value_str().is_none()
                || !matches!(tcx.def_kind(def), DefKind::Fn | DefKind::AssocFn | DefKind::Const)
            {
                tcx.dcx().span_err(
                    attr.span(),
                    "rust-js: a binding needs one `#[rust_js::link_name = \"...\"]` on a function, a method or a constant",
                );
                valid = false;
            }
        }
        // What `#[rust_js::nullable(..)]` names is a parameter of its own,
        // an `Option`, whose `None` can be `null`; and alone, its result.
        if matches!(tcx.def_kind(def), DefKind::Fn | DefKind::AssocFn) {
            if let Some(message) = nullable::refused_result(tcx, def.to_def_id()) {
                tcx.dcx().span_err(tcx.def_span(def), message);
                valid = false;
            }
            if let Some(message) = nullable::refused_optional_function(tcx, def.to_def_id()) {
                tcx.dcx().span_err(tcx.def_span(def), message);
                valid = false;
            }
            // What `#[rust_js::present(..)]` names is a parameter of its own,
            // an `Option`, not a nullable one, whose key is there already.
            let present_inputs = tcx.fn_sig(def).skip_binder().skip_binder().inputs();
            let present_idents = tcx.fn_arg_idents(def.to_def_id());
            let nulled = nullable::nullable_params(tcx, def.to_def_id());
            for name in nullable::present_params(tcx, def.to_def_id()) {
                let at = present_idents
                    .iter()
                    .position(|ident| ident.is_some_and(|ident| ident.name == name));
                let option = at.and_then(|at| present_inputs.get(at)).is_some_and(
                    |ty| matches!(ty.kind(), ty::Adt(adt, _) if tcx.is_lang_item(adt.did(), LangItem::Option)),
                );
                if !option || nulled.contains(&name) {
                    let message = format!(
                        "rust-js: `#[rust_js::present({name})]` is of a parameter of its own, an `Option` that isn't `#[rust_js::nullable]`, given `{name}: T | undefined` where a trailing one may be left out; `{name}` isn't"
                    );
                    tcx.dcx().span_err(tcx.def_span(def), message);
                    valid = false;
                }
            }
            let idents = tcx.fn_arg_idents(def.to_def_id());
            let inputs = tcx.fn_sig(def).skip_binder().skip_binder().inputs();
            for name in lazy_params(tcx, def.to_def_id()) {
                if !idents.iter().any(|ident| ident.is_some_and(|ident| ident.name == name)) {
                    let message = format!("rust-js: `#[rust_js::lazy({name})]` names no parameter of this function");
                    tcx.dcx().span_err(tcx.def_span(def), message);
                    valid = false;
                }
            }
            // Or of a type parameter, `create_cursor<T>(default_value: T)`,
            // whose `None` is `null` where it's given an `Option`.
            for name in nullable_params(tcx, def.to_def_id()) {
                let at = idents
                    .iter()
                    .position(|ident| ident.is_some_and(|ident| ident.name == name));
                if !at.is_some_and(|i| match inputs[i].kind() {
                    ty::Adt(adt, _) => tcx.is_lang_item(adt.did(), LangItem::Option),
                    ty::Param(_) => true,
                    _ => false,
                }) {
                    let message = format!(
                        "rust-js: `#[rust_js::nullable({name})]` names no parameter of this function that's an `Option`, or a type parameter's value"
                    );
                    tcx.dcx().span_err(tcx.def_span(def), message);
                    valid = false;
                }
            }
        }
        // `#[rust_js::whole]` is a range of whole numbers a binding's `f64`
        // is in (ADR 0302, amended); `position` was one, `indexOf`'s.
        if let Some(message) = whole_refused(tcx, def.to_def_id()) {
            tcx.dcx().span_err(tcx.def_span(def), message);
            valid = false;
        }
        // An indexed list is a binding's type (ADR 1042).
        if let Some(message) = indexed_refused(tcx, def.to_def_id()) {
            tcx.dcx().span_err(tcx.def_span(def), message);
            valid = false;
        }
        // A binding's path is JS's, each property by its name or a quoted
        // key, `window['$RT']` (ADR 1033).
        if let Some(message) = path_refused(tcx, def.to_def_id()) {
            tcx.dcx().span_err(tcx.def_span(def), message);
            valid = false;
        }
        // A `#[rust_js::nullable]` thread-local or `const` is of an `Option`
        // its `None` is `null` of (ADR 0586, amended).
        if matches!(tcx.def_kind(def), DefKind::Const)
            && let Some(message) = nullable_const_refused(tcx, def.to_def_id())
        {
            tcx.dcx().span_err(tcx.def_span(def), message);
            valid = false;
        }
        // A `#[rust_js::nullable]` field is an `Option` whose `None` is `null`
        // (ADR 0196), or a `Cell` of one (ADR 0772), or a `Vec` of them,
        // or a type parameter's value, one where it's an `Option` (ADR
        // 0586, amended): of one that isn't, it's nothing to do.
        if matches!(tcx.def_kind(def), DefKind::Struct | DefKind::Enum | DefKind::Union) {
            for field in tcx.adt_def(def).all_fields() {
                let ty = tcx.type_of(field.did).instantiate_identity().skip_normalization();
                if let Some(message) = prototype_key_refused(tcx, field) {
                    tcx.dcx().span_err(tcx.def_span(field.did), message);
                    valid = false;
                }
                // A field's test is bare, by its value's kind, or `test(key)`,
                // by its key alone (ADR 1162).
                if is_test_list(tcx, field.did) && !is_key_test_field(tcx, field.did) {
                    let message = "rust-js: a field's `#[rust_js::test]` is bare, telling its struct apart by its value's kind, `Array.isArray(child.children)`, or `#[rust_js::test(key)]`, by its key alone, `\"bound\" in move`";
                    tcx.dcx().span_err(tcx.def_span(field.did), message);
                    valid = false;
                }
                if let Some(message) = nested_refused(tcx, field) {
                    tcx.dcx().span_err(tcx.def_span(field.did), message);
                    valid = false;
                }
                if is_nullable(tcx, field)
                    && nullable_option(tcx, ty).is_none()
                    && nullable_items(tcx, ty).is_none()
                    && !of_type_parameter(tcx, ty)
                {
                    let message = "rust-js: `#[rust_js::nullable]` is of a field that's an `Option`, or a `Vec` of them, or a `Cell` or a `RefCell` of one, or of a type parameter's value, whose `None` is `null`; this one isn't";
                    tcx.dcx().span_err(tcx.def_span(field.did), message);
                    valid = false;
                }
                // An `Option` field's `None` is left out already (ADRs 0280,
                // 0810); a cell's keeps its key, `undefined`, absent until
                // it's set only where it's nullable and optional (ADR 0280,
                // amended), or a `OnceCell`, which has no `None` to write but
                // its empty one, React's lazy payload's `_ioInfo`; or a cell of
                // an `Option`, left out where it's made `None` and set as it's
                // given, react-art's Text's `_text`, which its ref sets (rd331g).
                let once = matches!(ty.kind(), ty::Adt(cell, _)
                    if crate::lower::recognition::is_std_def(tcx, cell.did(), crate::lower::recognition::StdItem::OnceCell));
                let cell_of_option = nullable_option(tcx, ty).is_some_and(|option| option != ty);
                // One that holds a `Some` of what may be `undefined` as it
                // is (rd645) can't leave a `None` out: it'd be that `Some`.
                if is_optional(tcx, field) && holds_value(tcx, field) {
                    let message = "rust-js: a `#[rust_js::nullable]` field of an `Option` of what may be `undefined` holds its `Some` as it is, `undefined` too, so it can't be `#[rust_js::optional]`, absent for `None`: its `None` is `null` where it's made";
                    tcx.dcx().span_err(tcx.def_span(field.did), message);
                    valid = false;
                } else if is_optional(tcx, field) && holds_nested(tcx, field) {
                    // A nested one's `None` is left out already, its
                    // `Some(None)` `null` (ADR 1017), which the general
                    // message below, asking for a nullable field, misnamed:
                    // this one is (#959).
                    let message = "rust-js: a `#[rust_js::nullable]` field of an `Option` of an `Option` is left out already for `None` and `null` for `Some(None)`, so `#[rust_js::optional]` says nothing more of it; remove it";
                    tcx.dcx().span_err(tcx.def_span(field.did), message);
                    valid = false;
                } else if is_present(tcx, field)
                    && (is_nullable(tcx, field) || once || cell_of_option || nullable_option(tcx, ty) != Some(ty))
                {
                    // A present slot's key is written where it's made `None`: a
                    // plain `Option` field's, which is left out otherwise; a
                    // nullable or a cell's key is written already (ADR 1161).
                    let message = "rust-js: `#[rust_js::present]` is of a plain `Option` field, whose `None` is left out otherwise, `point?: T`: its key written, `point: T | undefined`; a cell's or a `#[rust_js::nullable]` field's key is written already, and this field isn't an `Option`";
                    tcx.dcx().span_err(tcx.def_span(field.did), message);
                    valid = false;
                } else if is_optional(tcx, field) && !is_nullable(tcx, field) && !once && !cell_of_option {
                    let message = "rust-js: `#[rust_js::optional]` is of a cell, a `Cell<Option<T>>`, a `RefCell` of one or a `OnceCell`, left out where it's made `None` or empty, or of a `#[rust_js::nullable]` field, whose `None` is then left out where its struct is made and `null` where it's set; an `Option` field's `None` is left out already";
                    tcx.dcx().span_err(tcx.def_span(field.did), message);
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
                let clash = variant
                    .fields
                    .iter()
                    .any(|f| field_key(tcx, f).name() == Some(key.as_str()));
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
        // A name is a string, but a discriminated union's variant's, its tag,
        // may be `true` or `false` (ADR 0284).
        let variants = match tcx.def_kind(def) {
            DefKind::Enum => tcx.adt_def(def).variants().iter().map(|v| v.def_id).collect(),
            _ => Vec::new(),
        };
        for named in std::iter::once(def.to_def_id()).chain(variants) {
            if let Some(attr) = tcx
                .get_attrs_by_path(named, &[Symbol::intern("rust_js"), sym::name])
                .next()
                && attr.value_str().is_none()
                && !(matches!(tcx.def_kind(named), DefKind::Variant)
                    && declared_tag(tcx, tcx.parent(named)).is_some()
                    && given_bool(tcx, named).is_some())
            {
                let message = "rust-js: `#[rust_js::name]` is a string, or `true` or `false` of a variant of a `#[rust_js::tag]` enum, its tag";
                tcx.dcx().span_err(tcx.def_span(named), message);
                valid = false;
            }
        }
        // Two fields that are one JS property would overwrite each other.
        if matches!(tcx.def_kind(def), DefKind::Struct | DefKind::Enum) {
            for variant in tcx.adt_def(def).variants() {
                let mut keys: Vec<(FieldKey, Symbol)> = Vec::new();
                for field in variant.fields.iter() {
                    let key = field_key(tcx, field);
                    if let Some((_, other)) = keys.iter().find(|(k, _)| *k == key) {
                        let shown = match key.symbol_item() {
                            Some((item, _)) => format!("[{}]", tcx.def_path_str(item)),
                            None => key.text().to_string(),
                        };
                        let message = format!(
                            "rust-js: fields `{other}` and `{}` are both `{shown}` in JS",
                            field.name
                        );
                        tcx.dcx().span_err(tcx.def_span(field.did), message);
                        valid = false;
                    }
                    keys.push((key, field.name));
                }
            }
        }
    }
    // What can't name its field by a symbol (ADR 1051).
    valid &= symbol_names::validate(tcx);
    valid
}

/// Why a binding's path isn't one (ADR 1033): a static's, or a function's
/// it calls, reads, writes, constructs or asks `instanceof` of, starts from
/// a JS name, each property after it `.name` or a quoted key, `['$RT']`; a
/// method's names its property, never quoted, or its key, a global's path,
/// `[Symbol.asyncIterator]` (rd585).
fn path_refused(tcx: TyCtxt<'_>, def_id: DefId) -> Option<String> {
    let path = match tcx.def_kind(def_id) {
        DefKind::Static { .. } if tcx.is_foreign_item(def_id) => js_name(tcx, def_id),
        DefKind::Fn | DefKind::AssocFn if tool_link_name(tcx, def_id).is_some() => match js_form(tcx, def_id) {
            JsForm::Call(path)
            | JsForm::Get(path)
            | JsForm::Set(path)
            | JsForm::New(path)
            | JsForm::InstanceOf(path) => path,
            JsForm::CallKeyed(key) => {
                let why = match is_method(tcx, def_id) {
                    true => paths::global_refused(&key),
                    false => Some("a key, `[Symbol.asyncIterator]`, is a method's, of `&self` or `this`".to_string()),
                };
                return why.map(|why| format!("rust-js: `[{key}]` isn't a binding's path: {why} (ADR 1033)"));
            }
            _ => return None,
        },
        _ => return None,
    };
    let why = match is_method(tcx, def_id) {
        true => path.contains(['[', '\'']).then(|| {
            "a method's property is named, `matchMedia`; a quoted key, `['$RT']`, is a global's path's".to_string()
        }),
        // An import's, past its module and its export, `react#`, `* as
        // Scheduler` or `*`, which `js_import` reads.
        false => match js_import(&path) {
            Some((_, rest)) => paths::steps(rest).err(),
            None => paths::global_refused(&path),
        },
    };
    why.map(|why| format!("rust-js: `{path}` isn't a binding's path: {why} (ADR 1033)"))
}
