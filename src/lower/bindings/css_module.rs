//! `#[rust_js::css_module]`, of a binding's struct: a CSS Module's
//! classes, as a program reads them, each a getter, `#[rust_js::link_name =
//! "get codeBlock"]`, and the module's default import a `static` of it,
//! `#[link_name = "./site.module.css#default"]`. Each getter's class is one
//! the file exports, checked against it as each compile reads it, so a
//! class misspelled, or gone from the file, is a compile error at the
//! getter naming the file's classes (#1512, ADR 1133).

use super::{JsForm, is_binding, is_method, js_form, js_import, js_name};
use crate::program::JsModules;
use rustc_hir::def::DefKind;
use rustc_middle::ty::{self, TyCtxt};
use rustc_span::def_id::DefId;
use rustc_span::{Span, Symbol, sym};

/// Whether `def_id` is marked `#[rust_js::css_module]`.
fn marked(tcx: TyCtxt<'_>, def_id: DefId) -> bool {
    let path = [Symbol::intern("rust_js"), Symbol::intern("css_module")];
    tcx.get_attrs_by_path(def_id, &path).next().is_some()
}

/// Report each of the crate's CSS Module bindings that its file doesn't
/// bear out, as rustc errors: whether any was.
pub(in crate::lower) fn refuse_unverified(tcx: TyCtxt<'_>, js_modules: &dyn JsModules) -> bool {
    let mut refused = false;
    let mut refuse = |span, message: String| {
        tcx.dcx().span_err(span, message);
        refused = true;
    };
    for def in tcx.hir_crate_items(()).definitions() {
        let def = def.to_def_id();
        if marked(tcx, def) && tcx.def_kind(def) != DefKind::Struct {
            let message = "rust-js: `#[rust_js::css_module]` is of a binding's struct of a CSS Module's classes, `pub struct Styles(PhantomData<JsObject>);`";
            refuse(tcx.def_span(def), message.to_string());
            continue;
        }
        if !matches!(tcx.def_kind(def), DefKind::Static { .. }) || !tcx.is_foreign_item(def) {
            continue;
        }
        let ty::Adt(classes, _) = tcx
            .type_of(def)
            .instantiate_identity()
            .skip_normalization()
            .peel_refs()
            .kind()
        else {
            continue;
        };
        if !marked(tcx, classes.did()) {
            continue;
        }
        let (static_name, struct_name) = (tcx.item_name(def), tcx.item_name(classes.did()));
        let path = js_name(tcx, def);
        let file = match js_import(&path) {
            Some(((from, export), "")) if export == "default" && is_module_css(&from) => from,
            _ => {
                let message = format!(
                    "rust-js: `{static_name}` is a `{struct_name}`, a CSS Module's classes, so it's the module's default import, a file of the app's, `#[link_name = \"./{}.module.css#default\"]`, not `{path}`",
                    super::file_stem(&path),
                );
                refuse(tcx.def_span(def), message);
                continue;
            }
        };
        let keys = match js_modules.css_module_keys(&file) {
            Ok(keys) => keys,
            Err(why) => {
                let message =
                    format!("rust-js: the classes of `{file}`, which `{static_name}` reads, can't be said: {why}");
                refuse(tcx.def_span(def), message);
                continue;
            }
        };
        for getter in getters(tcx, classes.did()) {
            match js_form(tcx, getter) {
                JsForm::Get(class) if is_method(tcx, getter) && keys.contains(&class) => {}
                JsForm::Get(class) if is_method(tcx, getter) => {
                    let message = format!(
                        "rust-js: `{file}` exports no class `{class}`; its classes are {}",
                        listed(&keys)
                    );
                    refuse(link_name_span(tcx, getter), message);
                }
                // A class by a name the program has, `js::get`'s form.
                JsForm::GetIndex => {}
                _ => {
                    let message = format!(
                        "rust-js: a CSS Module's binding reads its classes: `#[rust_js::link_name = \"get {}\"]`, a getter of `&self`, or `\"get []\"` of a class named as the program runs, not `{}`",
                        keys.first().map_or("title", String::as_str),
                        js_name(tcx, getter),
                    );
                    refuse(tcx.def_span(getter), message);
                }
            }
        }
    }
    refused
}

/// Whether `from` is a file of the app's a bundler reads as a CSS Module,
/// `./site.module.css`.
fn is_module_css(from: &str) -> bool {
    crate::program::is_relative_specifier(from) && from.ends_with(".module.css")
}

/// The bindings of `classes`'s impls, in the order they're written.
fn getters(tcx: TyCtxt<'_>, classes: DefId) -> impl Iterator<Item = DefId> {
    (tcx.inherent_impls(classes).iter())
        .flat_map(move |&imp| tcx.associated_item_def_ids(imp).iter().copied())
        .filter(move |&item| tcx.def_kind(item) == DefKind::AssocFn && is_binding(tcx, item))
}

/// Where a getter's `#[rust_js::link_name]` is, which names its class.
fn link_name_span(tcx: TyCtxt<'_>, getter: DefId) -> Span {
    let path = [Symbol::intern("rust_js"), sym::link_name];
    tcx.get_attrs_by_path(getter, &path)
        .next()
        .map_or_else(|| tcx.def_span(getter), |attr| attr.span())
}

/// The classes, `` `card`, `title` and `wide` ``, or that it has none.
fn listed(keys: &[String]) -> String {
    let quoted: Vec<String> = keys.iter().map(|key| format!("`{key}`")).collect();
    match quoted.split_last() {
        None => "none".to_string(),
        Some((last, [])) => last.clone(),
        Some((last, rest)) => format!("{} and {last}", rest.join(", ")),
    }
}
