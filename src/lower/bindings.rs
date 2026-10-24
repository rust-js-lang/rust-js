//! Decode the binding language independently of call lowering.

use crate::js;
use rustc_ast::LitKind;
use rustc_hir::attrs::lang_items::LangItem;
use rustc_hir::def::{DefKind, Res};
use rustc_hir::{CoroutineKind, CoroutineSource};
use rustc_hir::{ExprKind, ItemKind, Stmt, StmtKind};
use rustc_middle::ty::{self, FieldDef, Ty, TyCtxt, VariantDef};
use rustc_span::def_id::{DefId, LocalDefId, LocalModId};
use rustc_span::{Span, Symbol, kw, sym};
use std::collections::HashMap;

/// Validate tool bindings even if no function calls them. A malformed binding
/// must not silently become an ordinary Rust function with an unreachable body.
pub(super) fn validate(tcx: TyCtxt<'_>) -> bool {
    let mut valid = given_names_differ(tcx);
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
            let what = format!(
                "deriving `{}` of a discriminated union with an `otherwise` variant",
                tcx.item_name(tr)
            );
            let why = "it would take the object it holds for a variant of its own";
            super::refusal::refuse(tcx, tcx.def_span(def), super::refusal::Refused::Because(&what, why));
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
        // an `Option`, whose `None` can be `null`; and alone, its result.
        if matches!(tcx.def_kind(def), DefKind::Fn | DefKind::AssocFn) {
            let output = tcx.fn_sig(def).skip_binder().skip_binder().output();
            if nullable_result(tcx, def.to_def_id())
                && !matches!(output.kind(), ty::Adt(adt, _) if tcx.is_lang_item(adt.did(), LangItem::Option))
            {
                let message = "rust-js: `#[rust_js::nullable]` of a function is of its result, which isn't an `Option`";
                tcx.dcx().span_err(tcx.def_span(def), message);
                valid = false;
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
        // A `#[rust_js::nullable]` field is an `Option` whose `None` is `null`
        // (ADR 0196), or a `Cell` of one (ADR 0772): of one that isn't, it's
        // nothing to do.
        if matches!(tcx.def_kind(def), DefKind::Struct | DefKind::Enum | DefKind::Union) {
            for field in tcx.adt_def(def).all_fields() {
                let ty = tcx.type_of(field.did).instantiate_identity().skip_normalization();
                if is_nullable(tcx, field) && nullable_option(tcx, ty).is_none() {
                    let message = "rust-js: `#[rust_js::nullable]` is of a field that's an `Option`, or a `Cell` of one, whose `None` is `null`; this one isn't";
                    tcx.dcx().span_err(tcx.def_span(field.did), message);
                    valid = false;
                }
                // An `Option` field's `None` is left out already (ADRs 0280, 0810).
                if is_optional(tcx, field) && !is_nullable(tcx, field) {
                    let message = "rust-js: `#[rust_js::optional]` is of a `#[rust_js::nullable]` field, whose `None` is then left out where its struct is made and `null` where it's set; an `Option` field's `None` is left out already";
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

/// A function in an `extern` block, refused wherever it's declared or used:
/// a JS function's binding is one form, an ordinary function with
/// `#[rust_js::link_name]` (ADR 0039), which can be generic as an `extern`
/// one can't. An `extern` block is for a `static`, which has no body.
pub(super) fn extern_fn_refused(tcx: TyCtxt<'_>, def_id: DefId) -> String {
    format!(
        "rust-js: a JS function is bound as an ordinary function, `#[cfg_attr(rust_js, rust_js::link_name = {:?})] fn {}(..) {{ unreachable!() }}`, not in an `extern` block (ADR 0373)",
        js_name(tcx, def_id),
        tcx.item_name(def_id),
    )
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

/// Does this binding read a JS string's `length`, `get length` of a
/// string `this`, as the js crate's `string::length` does? Its value is
/// any engine's string's, which the integer ranges know is at most
/// `2^31 - 1` (ADRs 0632, 0682).
pub(super) fn reads_string_length(tcx: TyCtxt<'_>, def_id: DefId) -> bool {
    is_binding(tcx, def_id)
        && is_method(tcx, def_id)
        && matches!(js_form(tcx, def_id), JsForm::Get(name) if name == "length")
        && (tcx.fn_sig(def_id).skip_binder().skip_binder().inputs().first())
            .is_some_and(|this| this.peel_refs().is_str())
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
    /// `new this(..)`: `this` is a JS function, constructed, `js::new`'s
    /// (ADR 0695).
    NewThis,
    /// `delete []`: `delete this[key]`, an own property gone, `js::delete`'s
    /// (ADR 0697).
    Delete,
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
    /// `===`: `this === other`, JS's strict equality (ADR 0721).
    StrictEquals,
    /// `typeof`: `typeof this`, what it is, as text (ADR 0310).
    TypeOf,
    /// `typeof !== "undefined"`: whether the host has the global of the
    /// binding given, `typeof MessageChannel !== "undefined"` (ADR 0597).
    Defined,
    /// `"" +`: `"" + this`, any value as JS's `+` makes it text (ADRs 0310,
    /// 0639).
    Text,
    /// `set []`: `this[key] = value`.
    SetIndex,
    /// `import()`: the item given, of its module loaded when it's asked
    /// for; `import(*)`, the module whose default export it is (ADR 0304).
    Import { module: bool },
    /// `for in`: the keys `for (const key in this)` visits, `js::keys_in`'s,
    /// a JS iterator, `$keysIn(this)`, which a `for` loop of it is that
    /// loop (ADR 0719).
    KeysIn,
    /// `throw`: `throw this`, a JS value thrown as it is, `js::throw`'s
    /// (ADR 0760).
    Throw,
    /// `class`: the constructor function of the class its type argument
    /// is, `Component`, `js::class::<Component>()`'s (ADR 0761).
    Class,
}

pub(super) fn js_form(tcx: TyCtxt<'_>, def_id: DefId) -> JsForm {
    let name = js_name(tcx, def_id);
    match name.as_str() {
        "this" => return JsForm::This,
        "this()" => return JsForm::CallThis,
        "new this()" => return JsForm::NewThis,
        "delete []" => return JsForm::Delete,
        "prop" => return JsForm::Prop(None),
        "get []" => return JsForm::GetIndex,
        "in []" => return JsForm::In,
        "!!" => return JsForm::Truthy,
        "===" => return JsForm::StrictEquals,
        "typeof" => return JsForm::TypeOf,
        "typeof !== \"undefined\"" => return JsForm::Defined,
        "\"\" +" => return JsForm::Text,
        "set []" => return JsForm::SetIndex,
        "import()" => return JsForm::Import { module: false },
        "import(*)" => return JsForm::Import { module: true },
        "for in" => return JsForm::KeysIn,
        "throw" => return JsForm::Throw,
        "class" => return JsForm::Class,
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

/// A binding a module holds as a value of its own, its default or a `pub
/// use`: its `const`, the JS its path reads, `const assign =
/// Object.assign;` (ADRs 0622, 0633).
#[derive(Clone)]
pub(super) struct HeldBinding {
    pub(super) binding: DefId,
    /// Its name in the module in Rust, which a caller's path names.
    pub(super) ident: Symbol,
    /// Its name in the module's JS, the `use`'s.
    pub(super) name: String,
    /// What it reads, `Object.assign`.
    pub(super) path: String,
    /// Whether a `pub use` names it, so the module exports it by its name.
    pub(super) export: bool,
    /// Whether it's the module's default.
    pub(super) default: bool,
    /// The `use` that names it.
    pub(super) span: Option<Span>,
    /// Where it names the binding through another of the crate's modules
    /// that holds it: that module's, re-exported (ADR 0633), not a
    /// `const` of its own.
    pub(super) through: Option<HeldRead>,
}

impl HeldBinding {
    /// Whether the module exports it by its name: a `pub use` names it,
    /// and it isn't the module's default, which is how every caller reads
    /// it, in the crate and in another, as upstream's `shared/assign` has
    /// it alone (ADR 0773).
    pub(super) fn exported_by_name(&self) -> bool {
        self.export && !self.default
    }

    /// What a caller that names it through its module imports: the
    /// module's default, as a function's (ADR 0240), else its name.
    pub(super) fn import(&self) -> Option<&str> {
        match (self.default, self.export) {
            (true, _) => Some("default"),
            (false, true) => Some(&self.name),
            (false, false) => None,
        }
    }
}

/// The JS a binding is that a module can hold as its `const`: a global
/// function, no method and no JS module's import, `Object.assign` (ADR
/// 0622); or a static, a global or a path in an import,
/// `react#*.__CLIENT_INTERNALS..` (ADR 0633). A JS module's export itself,
/// `react#useState`, is that module's, re-exported (ADR 0240).
pub(super) fn held_path(tcx: TyCtxt<'_>, def_id: DefId) -> Option<String> {
    match tcx.def_kind(def_id) {
        DefKind::Static { .. } if is_binding(tcx, def_id) => {
            let path = js_name(tcx, def_id);
            match js_import(&path) {
                Some((_, "")) => None,
                _ => Some(path),
            }
        }
        DefKind::Fn => match js_form(tcx, def_id) {
            JsForm::Call(name) if is_binding(tcx, def_id) && !is_method(tcx, def_id) && js_import(&name).is_none() => {
                Some(name)
            }
            _ => None,
        },
        _ => None,
    }
}

/// Where a caller's path reads a binding through the module that holds it
/// (ADR 0633): the module, what the caller imports of it, the default or
/// a name, and its `const`'s name there.
#[derive(Clone)]
pub(super) struct HeldRead {
    pub(super) holder: Holder,
    pub(super) import: String,
    pub(super) name: String,
    pub(super) binding: DefId,
}

/// A module that holds a binding: the crate's, or a library's, by its JS
/// module (ADR 0100).
#[derive(Clone)]
pub(super) enum Holder {
    Module(LocalModId),
    Library(String),
}

/// What each of the crate's modules holds of a binding as a value of its
/// own, and where each path of its bodies names one through a module that
/// holds it, by the path's span, MIR's and THIR's of what it names (ADR
/// 0633). Asked once, of every module and body; each read, a call, a `fn`
/// taken, a static's, asks `reads`.
#[derive(Default)]
pub(super) struct Held {
    pub(super) modules: HashMap<LocalModId, Vec<HeldBinding>>,
    pub(super) reads: HashMap<Span, HeldRead>,
}

pub(super) fn held<'tcx>(tcx: TyCtxt<'tcx>, foreign: &super::library::Foreign<'_, 'tcx>) -> Held {
    let mut held = Held::default();
    tcx.hir_for_each_module(|module| {
        module_held(tcx, foreign, module, &mut held.modules);
    });
    let mut paths = Paths {
        tcx,
        foreign,
        modules: &mut held.modules,
        reads: HashMap::new(),
    };
    for owner in tcx.hir_body_owners() {
        rustc_hir::intravisit::walk_body(&mut paths, tcx.hir_body_owned_by(owner));
    }
    held.reads = paths.reads;
    held
}

/// Each path of a body that names a binding through a module that holds it.
struct Paths<'a, 'b, 'tcx> {
    tcx: TyCtxt<'tcx>,
    foreign: &'a super::library::Foreign<'b, 'tcx>,
    modules: &'a mut HashMap<LocalModId, Vec<HeldBinding>>,
    reads: HashMap<Span, HeldRead>,
}

impl<'tcx> rustc_hir::intravisit::Visitor<'tcx> for Paths<'_, '_, 'tcx> {
    fn visit_expr(&mut self, expr: &'tcx rustc_hir::Expr<'tcx>) {
        if let ExprKind::Path(rustc_hir::QPath::Resolved(None, path)) = expr.kind
            && let Res::Def(DefKind::Fn | DefKind::Static { .. }, binding) = path.res
            && held_path(self.tcx, binding).is_some()
        {
            let scope = self.tcx.parent_module(expr.hir_id);
            if let Some(read) = named_through(self.tcx, self.foreign, scope, path.segments, binding, self.modules) {
                self.reads.insert(expr.span, read);
            }
        }
        rustc_hir::intravisit::walk_expr(self, expr);
    }
}

/// The module a path names `binding` through, where that module holds it
/// (ADR 0633): the module its last but one segment is, `crate::assign` of
/// `crate::assign::assign`; or, of a name alone, the `use` in `scope` that
/// names it, `use crate::assign::assign;`, or a glob's module. One that
/// names the binding itself, `js::object::assign`, or a module that only
/// declares it, reads it in place.
fn named_through<'tcx>(
    tcx: TyCtxt<'tcx>,
    foreign: &super::library::Foreign<'_, 'tcx>,
    scope: LocalModId,
    segments: &[rustc_hir::PathSegment<'_>],
    binding: DefId,
    modules: &mut HashMap<LocalModId, Vec<HeldBinding>>,
) -> Option<HeldRead> {
    let [.., module, last] = segments else {
        let [only] = segments else { return None };
        return tcx.hir_module_items(scope).free_items().find_map(|id| {
            let item = tcx.hir_item(id);
            match item.kind {
                ItemKind::Use(path, rustc_hir::UseKind::Single(ident))
                    if ident.name == only.ident.name
                        && matches!(path.res.value_ns, Some(Res::Def(_, named)) if named == binding) =>
                {
                    named_through(tcx, foreign, scope, path.segments, binding, modules)
                }
                ItemKind::Use(path, rustc_hir::UseKind::Glob) => match path.res.type_ns {
                    Some(Res::Def(DefKind::Mod, module)) => {
                        held_by(tcx, foreign, module, only.ident.name, binding, modules)
                    }
                    _ => None,
                },
                _ => None,
            }
        });
    };
    match module.res {
        Res::Def(DefKind::Mod, module) => held_by(tcx, foreign, module, last.ident.name, binding, modules),
        _ => None,
    }
}

/// How `module` holds `binding` by its Rust name `ident`, if it does, and
/// exports it: the crate's module, as `module_held` says, or a library's,
/// as its manifest does.
fn held_by<'tcx>(
    tcx: TyCtxt<'tcx>,
    foreign: &super::library::Foreign<'_, 'tcx>,
    module: DefId,
    ident: Symbol,
    binding: DefId,
    modules: &mut HashMap<LocalModId, Vec<HeldBinding>>,
) -> Option<HeldRead> {
    let Some(local) = module.as_local() else {
        let held = foreign.held(module, ident, binding)?;
        return Some(HeldRead {
            holder: Holder::Library(held.from.clone()),
            import: held.export.clone(),
            name: held.name.clone(),
            binding,
        });
    };
    let module = LocalModId::new_unchecked(local);
    let held = module_held(tcx, foreign, module, modules);
    let found = held.iter().find(|h| h.binding == binding && h.ident == ident)?;
    Some(HeldRead {
        holder: Holder::Module(module),
        import: found.import()?.to_string(),
        name: found.name.clone(),
        binding,
    })
}

/// Each binding `module` names as a value of its own, its default or a
/// `pub use`: its `const`, the JS it reads as it is, `const assign =
/// Object.assign;`, which a JS caller calls with JS's arguments, exported
/// where it's a `pub use`'s (ADR 0622). Not one its `use` names through
/// another module that holds it, which is that module's, re-exported (ADR
/// 0633). Its JS, its `.d.ts` and its library's manifest ask this.
fn module_held<'m, 'tcx>(
    tcx: TyCtxt<'tcx>,
    foreign: &super::library::Foreign<'_, 'tcx>,
    module: LocalModId,
    modules: &'m mut HashMap<LocalModId, Vec<HeldBinding>>,
) -> &'m Vec<HeldBinding> {
    if !modules.contains_key(&module) {
        // Rust refuses a cycle of `use`s, so none is met here.
        modules.insert(module, Vec::new());
        let held = held_of(tcx, foreign, module, modules);
        modules.insert(module, held);
    }
    &modules[&module]
}

fn held_of<'tcx>(
    tcx: TyCtxt<'tcx>,
    foreign: &super::library::Foreign<'_, 'tcx>,
    module: LocalModId,
    modules: &mut HashMap<LocalModId, Vec<HeldBinding>>,
) -> Vec<HeldBinding> {
    let default = (default_exports(tcx, module).into_iter())
        .find_map(|(named, _)| named.filter(|&named| held_path(tcx, named).is_some()));
    let mut held: Vec<HeldBinding> = Vec::new();
    let mut add = |made: HeldBinding| match held.iter_mut().find(|h| h.name == made.name) {
        Some(was) => {
            was.export |= made.export;
            was.default |= made.default;
        }
        None => held.push(made),
    };
    // Where its `use` is, that names it; the default by its own name.
    for id in tcx.hir_module_items(module).free_items() {
        let item = tcx.hir_item(id);
        let ItemKind::Use(path, rustc_hir::UseKind::Single(ident)) = item.kind else {
            continue;
        };
        let Some(Res::Def(_, binding)) = path.res.value_ns else {
            continue;
        };
        let Some(read) = held_path(tcx, binding) else {
            continue;
        };
        let export = super::analysis::is_reexport(tcx, item);
        let name = match ident.name == tcx.item_name(binding) {
            true => fn_name(tcx, binding),
            false => alias_name(tcx, item.owner_id.to_def_id(), ident.name),
        };
        let named_default = default == Some(binding) && name == fn_name(tcx, binding);
        if !export && !named_default {
            continue;
        }
        let through = named_through(tcx, foreign, module, path.segments, binding, modules);
        // A library's, as a `pub use` of its function, writes nothing.
        if through
            .as_ref()
            .is_some_and(|read| matches!(read.holder, Holder::Library(_)))
        {
            continue;
        }
        add(HeldBinding {
            binding,
            ident: ident.name,
            name,
            path: read,
            export,
            default: named_default,
            span: Some(item.span),
            through,
        });
    }
    if let Some(binding) = default
        && let Some(path) = held_path(tcx, binding)
    {
        add(HeldBinding {
            binding,
            ident: tcx.item_name(binding),
            name: fn_name(tcx, binding),
            path,
            export: false,
            default: true,
            span: None,
            through: None,
        });
    }
    held
}

/// The path a JS item is reached by, if it isn't a method or a property:
/// `document`, `console.log`, `Event` for `new Event`, or an import like
/// `node:path#join`.
pub(super) fn js_path(tcx: TyCtxt<'_>, def_id: DefId) -> Option<String> {
    match tcx.def_kind(def_id) {
        DefKind::Static { .. } => Some(js_name(tcx, def_id)),
        DefKind::Fn | DefKind::AssocFn => match js_form(tcx, def_id) {
            // A static property, `Notification.permission` or an export
            // read at each call, `get node:path#sep`.
            JsForm::Call(name) | JsForm::New(name) | JsForm::Get(name) if !is_method(tcx, def_id) => Some(name),
            // A class to test against is a global or an import like any other.
            JsForm::InstanceOf(class) => Some(class),
            // So is a component: `<react#StrictMode>`.
            JsForm::Jsx(tag) if tag.contains('#') => Some(tag),
            _ => None,
        },
        _ => None,
    }
}

/// What a `pub use` names its item by in JS, `use_item` the `use`'s own:
/// its `#[rust_js::name]`, `unstable_shouldYield`, or its name camelCase, as
/// any other (ADRs 0573, 0596).
pub(super) fn alias_name(tcx: TyCtxt<'_>, use_item: DefId, alias: Symbol) -> String {
    given_name(tcx, use_item).unwrap_or_else(|| super::camel_case(alias.as_str()))
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
    (given_name(tcx, variant.def_id))
        .or_else(|| given_bool(tcx, variant.def_id).map(|b| b.to_string()))
        .unwrap_or_else(|| default_name(tcx, variant.def_id))
}

/// A variant's tag, the one answer to what a fieldless variant is in JS:
/// its name; a numbered enum's, its number, as rustc numbers it,
/// `ArrayKind::Empty` `0` (`numbered`, ADR 0722); or a discriminated
/// union's `true` or `false`, `#[rust_js::name = true]`, react-dom's
/// `FormStatus`'s `pending` (ADR 0284).
pub(super) fn variant_tag(tcx: TyCtxt<'_>, variant: &VariantDef) -> js::Expr {
    if let DefKind::Variant = tcx.def_kind(variant.def_id) {
        let adt = tcx.adt_def(tcx.parent(variant.def_id));
        if numbered(tcx, adt) {
            let index = adt.variant_index_with_id(variant.def_id);
            return js::Expr::int(super::discriminants(tcx, adt)[index.as_usize()].1);
        }
    }
    match given_bool(tcx, variant.def_id) {
        Some(b) => js::Expr::bool(b),
        None => js::Expr::str(variant_name(tcx, variant)),
    }
}

/// Whether an enum's variants are its numbers in JS, as rustc numbers
/// them: one of no fields under a `#[repr(..)]` of an integer JS holds as
/// a number, of 32 bits or fewer, none of its variants named
/// (`#[rust_js::name]`), neither tagged nor untagged, nor std's (ADR
/// 0722). What `variant_tag` asks; each other question of its value asks
/// this.
pub(super) fn numbered(tcx: TyCtxt<'_>, adt: ty::AdtDef<'_>) -> bool {
    use rustc_abi::IntegerType;
    let narrow = match adt.repr().int {
        Some(IntegerType::Pointer(_)) => true,
        Some(IntegerType::Fixed(int, _)) => int.size().bits() <= 32,
        None => false,
    };
    narrow
        && adt.is_enum()
        && !adt.variants().is_empty()
        && (adt.variants().iter())
            .all(|v| v.fields.is_empty() && given_name(tcx, v.def_id).is_none() && given_bool(tcx, v.def_id).is_none())
        && declared_tag(tcx, adt.did()).is_none()
        && !is_untagged(tcx, adt.did())
        && !super::recognition::is_std_item(tcx, adt.did())
}

/// A discriminated union's variant's `#[rust_js::name = true]` or `false`.
pub(super) fn given_bool(tcx: TyCtxt<'_>, def_id: DefId) -> Option<bool> {
    let attr = tcx
        .get_attrs_by_path(def_id, &[Symbol::intern("rust_js"), sym::name])
        .next()?;
    match attr.value_lit()?.kind {
        LitKind::Bool(b) => Some(b),
        _ => None,
    }
}

/// The property an enum's variant is told by: `TAG` (ADR 0033), or the
/// one it names, `#[rust_js::tag = "status"]`, a discriminated union's,
/// each variant an object of it, its fieldless ones too (ADR 0284).
pub(super) fn tag_key(tcx: TyCtxt<'_>, adt: DefId) -> String {
    declared_tag(tcx, adt).unwrap_or_else(|| js::TAG.to_string())
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
    let name = variant_tag(tcx, variant);
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

/// `#[rust_js::require]`: a binding of a module read through
/// `require(module)`, not imported (ADR 0305).
pub(super) fn requires(tcx: TyCtxt<'_>, def_id: DefId) -> bool {
    let path = [Symbol::intern("rust_js"), Symbol::intern("require")];
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

/// Whether `ty` is rest arguments as a value, `&dyn js::Rest<T>`, of the
/// trait marked `#[rust_js::rest]` (ADR 0667): spread where it's given, to
/// a variadic binding or to a `fn` pointer whose last parameter it is.
pub(super) fn is_rest_value(tcx: TyCtxt<'_>, ty: Ty<'_>) -> bool {
    let path = [Symbol::intern("rust_js"), Symbol::intern("rest")];
    match ty.kind() {
        ty::Ref(_, inner, _) => match inner.kind() {
            ty::Dynamic(traits, ..) => traits
                .principal_def_id()
                .is_some_and(|t| tcx.get_attrs_by_path(t, &path).next().is_some()),
            _ => false,
        },
        _ => false,
    }
}

/// Each parameter of the crate's function or closure `owner` marked
/// `#[rust_js::rest]`, JS's rest parameter (ADR 0664), by its index: read
/// from its body, as a parameter's attributes are kept nowhere else.
pub(super) fn marked_rest<'tcx>(
    tcx: TyCtxt<'tcx>,
    owner: LocalDefId,
) -> impl Iterator<Item = (usize, &'tcx rustc_hir::Param<'tcx>)> {
    let path = [Symbol::intern("rust_js"), Symbol::intern("rest")];
    let params = tcx.hir_maybe_body_owned_by(owner).map_or(&[][..], |body| body.params);
    (params.iter().enumerate())
        .filter(move |(_, param)| tcx.hir_attrs(param.hir_id).iter().any(|attr| attr.path_matches(&path)))
}

/// Whether the crate's function `def_id` takes JS's rest arguments, its last
/// parameter, a slice, marked `#[rust_js::rest]` (ADR 0664), which
/// analysis allows nowhere else. A library's says so in its manifest.
pub(super) fn takes_rest(tcx: TyCtxt<'_>, def_id: DefId) -> bool {
    def_id
        .as_local()
        .is_some_and(|local| marked_rest(tcx, local).next().is_some())
}

/// Whether an untagged enum's variant is what its others aren't,
/// `#[rust_js::otherwise]` (ADR 0214).
pub(super) fn is_otherwise(tcx: TyCtxt<'_>, def_id: DefId) -> bool {
    let path = [Symbol::intern("rust_js"), Symbol::intern("otherwise")];
    tcx.get_attrs_by_path(def_id, &path).next().is_some()
}

/// What JS value an untagged enum's variant without fields is, where it's
/// none of a string: `null`, `#[rust_js::null]`, or `undefined`,
/// `#[rust_js::undefined]`, `js::Kind`'s `Null` and `Undefined` (ADR 0676).
#[derive(Clone, Copy, PartialEq, Debug)]
pub(super) enum Nullish {
    Null,
    Undefined,
}

/// `def_id`'s `#[rust_js::null]` or `#[rust_js::undefined]`, if it has one.
pub(super) fn given_nullish(tcx: TyCtxt<'_>, def_id: DefId) -> Option<Nullish> {
    let marked = |name: &str| {
        let path = [Symbol::intern("rust_js"), Symbol::intern(name)];
        tcx.get_attrs_by_path(def_id, &path).next().is_some()
    };
    [("null", Nullish::Null), ("undefined", Nullish::Undefined)]
        .into_iter()
        .find_map(|(name, nullish)| marked(name).then_some(nullish))
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
    given_name(tcx, def_id).unwrap_or_else(|| default_name(tcx, def_id))
}

/// A module's `js::import!`: each a `const _` with
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

/// What a `js::directive!` is the directive of (ADR 0349): the function or
/// closure whose body it's written in, past the coroutine an `async` one's
/// body is, or else its module.
pub(super) fn directive_owner(tcx: TyCtxt<'_>, mark: LocalDefId) -> LocalDefId {
    let mut owner = tcx.local_parent(mark);
    while matches!(
        tcx.coroutine_kind(owner),
        Some(CoroutineKind::Desugared(
            _,
            CoroutineSource::Fn | CoroutineSource::Closure
        ))
    ) {
        owner = tcx.local_parent(owner);
    }
    owner
}

/// The directives of `owner`, a module, a function or a closure: each
/// `js::directive!` in it, as [`directive_owner`] tells, and where it is.
pub(super) fn directives_of(tcx: TyCtxt<'_>, owner: LocalDefId) -> Vec<(Option<Symbol>, Span)> {
    let path = [Symbol::intern("rust_js"), Symbol::intern("directive")];
    let module = match tcx.def_kind(owner) {
        DefKind::Mod => LocalModId::new_unchecked(owner),
        _ => tcx.parent_module_from_def_id(owner),
    };
    tcx.hir_module_free_items(module)
        .filter(|item| directive_owner(tcx, item.owner_id.def_id) == owner)
        .flat_map(|item| tcx.get_attrs_by_path(item.owner_id.to_def_id(), &path))
        .map(|attr| (attr.value_str(), attr.span()))
        .collect()
}

/// Is `def_id` a `js::on_load!`'s function, whose body is what its module
/// runs when it's loaded (ADR 0267)?
pub(super) fn is_on_load(tcx: TyCtxt<'_>, def_id: DefId) -> bool {
    (tcx.get_attrs_by_path(def_id, &[Symbol::intern("rust_js"), Symbol::intern("on_load")]))
        .next()
        .is_some()
}

/// Is `def_id` a `js::import!`'s, `js::directive!`'s, `js::export_default!`'s
/// or `js::on_load!`'s `const _`, which is rust-js's to read, and has nothing
/// to write? Not a constant the default is itself (ADR 0625).
pub(super) fn is_mark(tcx: TyCtxt<'_>, def_id: DefId) -> bool {
    if held_by_default(tcx, def_id) {
        return false;
    }
    ["import", "directive", "export_default", "on_load"].iter().any(|name| {
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

/// Whether a nullable field is absent until set, `#[rust_js::optional]`, `field?: T | null` (ADR 0810).
pub(super) fn is_optional(tcx: TyCtxt<'_>, field: &FieldDef) -> bool {
    let path = [Symbol::intern("rust_js"), Symbol::intern("optional")];
    tcx.get_attrs_by_path(field.did, &path).next().is_some()
}

/// Whether a field's `None` is `null` from where it's made on: a nullable one's, not an optional one's (ADR 0810).
pub(super) fn null_where_made(tcx: TyCtxt<'_>, field: &FieldDef) -> bool {
    is_nullable(tcx, field) && !is_optional(tcx, field)
}

/// The `Option` a `#[rust_js::nullable]` field of type `ty` holds: itself,
/// or a `Cell`'s, whose `None` is `null` in JS (ADRs 0196, 0772). `None` of
/// any other type.
pub(super) fn nullable_option<'tcx>(tcx: TyCtxt<'tcx>, ty: Ty<'tcx>) -> Option<Ty<'tcx>> {
    let option = |ty: Ty<'tcx>| matches!(ty.kind(), ty::Adt(adt, _) if tcx.is_lang_item(adt.did(), LangItem::Option));
    match ty.kind() {
        _ if option(ty) => Some(ty),
        ty::Adt(adt, args)
            if adt.did() == super::recognition::std_item(tcx, super::recognition::StdItem::Cell)
                && option(args.type_at(0)) =>
        {
            Some(args.type_at(0))
        }
        _ => None,
    }
}

/// What a `#[rust_js::nullable]` field is written, of `made`, its
/// `Option`'s JS: a `None` is `null`, a constant itself, and what's
/// `nulled`, `null` or a value already, as it is; else `made ?? null`
/// (ADRs 0275, 0772).
pub(super) fn nullable_written(made: js::Expr, nulled: bool) -> js::Expr {
    match made.kind {
        js::ExprKind::Undefined => js::Expr::null().or_at(made.span),
        _ if made.is_constant() || nulled => made,
        _ => js::Expr::bin(js::Op::Coalesce, made, js::Expr::null()),
    }
}

/// The parameters a binding names `#[rust_js::lazy(..)]`, each a function
/// JS calls once for the value, React's `useState(init)`: given a closure
/// whose body does nothing and gives no function, its value (ADR 0593).
pub(super) fn lazy_params(tcx: TyCtxt<'_>, def_id: DefId) -> Vec<Symbol> {
    let path = [Symbol::intern("rust_js"), Symbol::intern("lazy")];
    tcx.get_attrs_by_path(def_id, &path)
        .filter_map(|attr| attr.meta_item_list())
        .flatten()
        .filter_map(|item| item.ident().map(|ident| ident.name))
        .collect()
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

/// Whether a function says its result's `None` is `null`,
/// `#[rust_js::nullable]` alone on it, TypeScript's `T | null`, as a JS
/// caller of it tests `!== null`: React's scheduler's `peek(queue)`.
pub(super) fn nullable_result(tcx: TyCtxt<'_>, def_id: DefId) -> bool {
    let path = [Symbol::intern("rust_js"), Symbol::intern("nullable")];
    tcx.get_attrs_by_path(def_id, &path).any(|attr| attr.is_word())
}

/// Whether a binding's `index`th parameter, `this` too, is optional,
/// TypeScript's `x?`: an `Option`, whose `None` is left out where it's
/// last (ADR 0443). Not one whose `None` is `null`, `#[rust_js::nullable]`,
/// nor a setter's value, which JS is given as it is, `undefined` too:
/// `Error.prepareStackTrace = undefined` (ADR 0648).
pub(super) fn is_optional_param(tcx: TyCtxt<'_>, def_id: DefId, index: usize) -> bool {
    if matches!(js_form(tcx, def_id), JsForm::Set(_)) {
        return false;
    }
    let sig = tcx.fn_sig(def_id).instantiate_identity().skip_normalization();
    let Some(param) = sig.skip_binder().inputs().get(index) else {
        return false;
    };
    let named = tcx
        .fn_arg_idents(def_id)
        .get(index)
        .copied()
        .flatten()
        .map(|ident| ident.name);
    matches!(param.kind(), ty::Adt(adt, _) if tcx.is_lang_item(adt.did(), LangItem::Option))
        && !named.is_some_and(|name| nullable_params(tcx, def_id).contains(&name))
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

/// `#[rust_js::unexpanded_jsx]`: what an element is to a plain rustc,
/// `Element::__jsx`, a `jsx!` rust-js didn't expand: one written by a path
/// to it, `react::jsx!`, where it's found by the name it's imported as.
pub(super) fn is_unexpanded_jsx(tcx: TyCtxt<'_>, def: DefId) -> bool {
    let path = [Symbol::intern("rust_js"), Symbol::intern("unexpanded_jsx")];
    tcx.get_attrs_by_path(def, &path).next().is_some()
}

/// Whether `id` is react's `ReactNode`, what React renders as a child: a
/// sealed trait, its types std's and React's (ADR 0201).
pub(super) fn is_jsx_node(tcx: TyCtxt<'_>, id: DefId) -> bool {
    let path = [Symbol::intern("rust_js"), Symbol::intern("jsx_node")];
    tcx.get_attrs_by_path(id, &path).next().is_some()
}

/// A module's `js::export_default!(page)`: the function its `const _`
/// names, `let _ = page;`, and where it's written (ADR 0192).
/// Whether `function`, its module's default export, is declared as it,
/// `export default function noop() {}`: of `js::export_default! { fn noop()
/// {} }`, which marks the function itself, not a `const _` naming it (ADR
/// 0611).
pub(super) fn default_declared(tcx: TyCtxt<'_>, function: DefId) -> bool {
    let path = [Symbol::intern("rust_js"), Symbol::intern("export_default")];
    held_by_default(tcx, function) && tcx.get_attrs_by_path(function, &path).next().is_some()
}

/// Whether `def`, marked `#[rust_js::export_default]`, is the default
/// itself, as `js::export_default! { .. }` writes it: a function, or a
/// `const` whose value is the default (ADR 0625), not the `const _` the
/// naming form makes.
fn held_by_default(tcx: TyCtxt<'_>, def: DefId) -> bool {
    match tcx.def_kind(def) {
        DefKind::Fn => true,
        DefKind::Const { .. } => tcx.item_name(def) != kw::Underscore,
        _ => false,
    }
}

pub(super) fn default_exports(tcx: TyCtxt<'_>, module: LocalModId) -> Vec<(Option<DefId>, Span)> {
    let path = [Symbol::intern("rust_js"), Symbol::intern("export_default")];
    tcx.hir_module_free_items(module)
        .filter(|item| tcx.get_attrs_by_path(item.owner_id.to_def_id(), &path).next().is_some())
        .map(|item| {
            let def = item.owner_id.def_id;
            // Of the function itself (ADR 0611), or the constant (ADR 0625).
            if held_by_default(tcx, def.to_def_id()) {
                return (Some(def.to_def_id()), tcx.def_span(def));
            }
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
                        // component's (ADR 0248), or a binding's static (ADR 0633).
                        ItemKind::Use(path, _) => match path.res.value_ns {
                            Some(Res::Def(DefKind::Fn | DefKind::Const { .. } | DefKind::Static { .. }, function)) => {
                                Some(function)
                            }
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

/// What an item is called in JS: its `#[rust_js::name]`, else its
/// `default_name`.
fn name_in_js(tcx: TyCtxt<'_>, def_id: DefId) -> String {
    given_name(tcx, def_id).unwrap_or_else(|| default_name(tcx, def_id))
}

/// What an item is called in JS where it says nothing (ADR 0573): a
/// function's or a field's Rust name as JS writes one, camelCase,
/// `sort_index` as `sortIndex`, `use_clicks` as `useClicks`, as a
/// variable's is (ADR 0038); a `thread_local!`'s, its module's variable
/// (ADR 0270), too, `TASK_QUEUE` as `taskQueue` (`variable_case`), and
/// capitalized where it holds a component, but one of a primitive, never
/// written, a constant, as a `const` is (ADR 0619); a `const`'s, as JS
/// writes a constant, `MAX_SIZE`; a variant's and a JS class's, its name.
fn default_name(tcx: TyCtxt<'_>, def_id: DefId) -> String {
    let name = tcx.item_name(def_id);
    match tcx.def_kind(def_id) {
        DefKind::Variant | DefKind::Struct | DefKind::Enum | DefKind::Union => name.to_string(),
        DefKind::Const { .. } if thread_local_value(tcx, def_id).is_some_and(|held| primitive(tcx, held)) => {
            name.to_string()
        }
        DefKind::Const { .. } if thread_local_value(tcx, def_id).is_some() => {
            let variable = variable_name(tcx, def_id);
            match another_named(tcx, def_id, &variable) {
                true => name.to_string(),
                false => variable,
            }
        }
        _ => super::camel_case(name.as_str()),
    }
}

/// A thread-local's name as its module's variable: camelCase,
/// capitalized where it holds a component.
fn variable_name(tcx: TyCtxt<'_>, def_id: DefId) -> String {
    let variable = super::variable_case(tcx.item_name(def_id).as_str());
    match holds_component(tcx, def_id) {
        true => super::upper_first(&variable),
        false => variable,
    }
}

/// Whether another item of `def_id`'s module, a function's, a constant's
/// or a thread-local's, is `name` in JS: a thread-local named after the
/// function that reads it, `CACHE` of `cache()`, keeps its Rust name, as
/// JS keeps two names apart.
fn another_named(tcx: TyCtxt<'_>, def_id: DefId, name: &str) -> bool {
    let Some(module) = tcx
        .parent(def_id)
        .as_local()
        .filter(|&m| tcx.def_kind(m) == DefKind::Mod)
    else {
        return false;
    };
    (tcx.hir_module_items(LocalModId::new_unchecked(module)).definitions()).any(|other| {
        let other = other.to_def_id();
        other != def_id
            && tcx.parent(other) == tcx.parent(def_id)
            && matches!(
                tcx.def_kind(other),
                DefKind::Fn | DefKind::Const { .. } | DefKind::Static { .. }
            )
            && given_name(tcx, other).unwrap_or_else(|| match thread_local_value(tcx, other) {
                Some(_) => variable_name(tcx, other),
                None => super::camel_case(tcx.item_name(other).as_str()),
            }) == name
    })
}

/// What a `thread_local!`, the `const` of a `LocalKey<T>`, holds: `T`.
fn thread_local_value<'tcx>(tcx: TyCtxt<'tcx>, def_id: DefId) -> Option<Ty<'tcx>> {
    let ty = tcx.type_of(def_id).instantiate_identity().skip_normalization();
    match ty.kind() {
        ty::Adt(key, args) if super::recognition::is_std_def(tcx, key.did(), super::recognition::StdItem::LocalKey) => {
            args.types().next()
        }
        _ => None,
    }
}

/// Whether a thread-local's value is a JS primitive, a number, text, a
/// `bool` or a symbol, which nothing writes, as a `Cell` would be: React's
/// `REACT_ELEMENT_TYPE = Symbol.for(..)` (ADR 0619). A function, an object,
/// is a variable's, camelCase, `localSetTimeout`.
fn primitive<'tcx>(tcx: TyCtxt<'tcx>, held: ty::Ty<'tcx>) -> bool {
    let held = held.peel_refs();
    super::representation::Num::of(held).is_some()
        || held.is_bool()
        || held.is_str()
        || matches!(held.kind(), ty::Adt(adt, _)
            if tcx.is_lang_item(adt.did(), LangItem::String)
                || super::declarations::written_types(tcx, adt.did()).as_deref() == Some("symbol"))
}

/// Whether a `thread_local!` holds a component, of a type React marks
/// `#[rust_js::component]`, a context, `memo`'s, `lazy`'s, `forward_ref`'s:
/// JSX takes a name as a component's only capitalized, `ThemeContext` of
/// `THEME_CONTEXT`.
fn holds_component(tcx: TyCtxt<'_>, def_id: DefId) -> bool {
    matches!(thread_local_value(tcx, def_id).map(|held| *held.kind()), Some(ty::Adt(held, _))
        if tcx.get_attrs_by_path(held.did(), &[Symbol::intern("rust_js"), Symbol::intern("component")]).next().is_some())
}

/// Whether each `#[rust_js::name]` of the crate gives a name its item
/// hasn't anyway: one that repeats its `default_name` is a second way to
/// write it, and rust-js's error (ADR 0573). A unit struct's says the string
/// it is, which it has no other way to; what isn't a function, a `const`, a
/// `static`, a field, a variant or a class is named otherwise.
pub fn given_names_differ(tcx: TyCtxt<'_>) -> bool {
    let mut differ = true;
    // Each item, and each variant and field of a struct's or an enum's, once.
    let mut defs = Vec::new();
    for def in tcx.hir_crate_items(()).definitions() {
        let def = def.to_def_id();
        defs.push(def);
        if matches!(tcx.def_kind(def), DefKind::Struct | DefKind::Enum | DefKind::Union) {
            for variant in tcx.adt_def(def).variants() {
                defs.push(variant.def_id);
                defs.extend(variant.fields.iter().map(|field| field.did));
            }
        }
    }
    let mut seen = std::collections::HashSet::new();
    for def in defs.into_iter().filter(|&def| seen.insert(def)) {
        let named = match tcx.def_kind(def) {
            DefKind::Struct => tcx.adt_def(def).non_enum_variant().ctor_kind() != Some(rustc_hir::def::CtorKind::Const),
            kind => matches!(
                kind,
                DefKind::Fn
                    | DefKind::AssocFn
                    | DefKind::Const { .. }
                    | DefKind::AssocConst { .. }
                    | DefKind::Static { .. }
                    | DefKind::Field
                    | DefKind::Variant
            ),
        };
        if named
            && let Some(given) = given_name(tcx, def)
            && given == default_name(tcx, def)
        {
            let message = format!(
                "rust-js: `{}` is `{given}` in JS without `#[rust_js::name = \"{given}\"]`: remove it",
                tcx.item_name(def)
            );
            tcx.dcx().span_err(tcx.def_span(def), message);
            differ = false;
        }
    }
    differ
}

/// A function's or `const`'s JS name.
pub(crate) fn fn_name(tcx: TyCtxt<'_>, def_id: DefId) -> String {
    name_in_js(tcx, def_id)
}

/// What trait `trait_id`'s method `name` is called in JS, its key in a
/// dictionary: its `fn_name`, `asRef` of `AsRef::as_ref`. A dictionary
/// std's impl makes, or the runtime's, is keyed so, as the crate's are.
pub(super) fn trait_method_name(tcx: TyCtxt<'_>, trait_id: DefId, name: &str) -> String {
    fn_name(tcx, super::recognition::trait_method(tcx, trait_id, name))
}

/// A struct's or a variant's field, as a JS property.
pub(crate) fn field_key(tcx: TyCtxt<'_>, field: &FieldDef) -> String {
    name_in_js(tcx, field.did)
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
    // Its words, `lang` and `rust`, joined as a Rust name's are in JS.
    let words: Vec<&str> = stem
        .split(|c: char| !c.is_ascii_alphanumeric() && c != '_' && c != '$')
        .filter(|w| !w.is_empty())
        .collect();
    let name = super::camel_case(&words.join("_"));
    if name.is_empty() || name.starts_with(|c: char| c.is_ascii_digit()) {
        format!("_{name}")
    } else {
        name
    }
}
