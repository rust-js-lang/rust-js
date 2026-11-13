//! serde's derives, and serde_json, as JS (ADR 0077).
//!
//! `#[derive(Serialize)]` writes an impl that drives any `Serializer`, with
//! generic machinery JS has no use for. rust-js leaves that impl's code out
//! and writes the steps it would take against one serializer, `$json`, which
//! lays JSON out as serde_json does:
//!
//! ```text
//!   #[derive(Serialize)] struct Order { id: u32, note: Option<String> }
//!
//!   function orderSerialize_serialize(order, json) {
//!     json.beginObject();
//!     json.key("id");
//!     json.int(order.id);
//!     json.key("note");
//!     if (order.note == null) json.null(); else json.string(order.note);
//!     json.endObject();
//!   }
//! ```
//!
//! What a type's JSON looks like is serde's to say, from its `#[serde(..)]`
//! attributes, read here as serde_derive reads them. Reading JSON back is
//! in `de`.

mod codecs;
mod de;
mod protocol;
mod std_forms;
mod value;

pub(super) use super::recognition::Json;
use crate::collections::IndexMap;
use crate::lower::temporary_names::{Base, Role};

use super::recognition::{SkipPredicate, StdItem, std_item};
use super::representation::{Num, variant_field};
use super::{FnCx, R, lower_first};
use crate::js::result;
use crate::js::{self, Expr, Op, Stmt, StmtKind};
use rustc_ast::visit::{self, Visitor};
use rustc_hir::def::CtorKind;
use rustc_hir::{self as hir, intravisit};
use rustc_middle::ty::{self, Ty, TyCtxt};
use rustc_span::def_id::DefId;
use rustc_span::{Span, Symbol};

/// One `#[serde(..)]` item: `rename = "a"`, or `rename(serialize = "a")`.
pub struct SerdeItem {
    name: String,
    value: Option<String>,
    nested: Vec<(String, Option<String>)>,
    span: Span,
}

/// Each item's, variant's and field's `#[serde(..)]`, by its `DefId`. HIR
/// has no `#[serde]`: it's a derive's helper, so it's read from the crate as
/// it was expanded, each node's `DefId` the resolver's. Not by the span of
/// its name, which `-C incremental`, Cargo's default, keeps relative to its
/// item, so no HIR span is the expanded crate's (#450).
pub type SerdeAttributes = IndexMap<DefId, Vec<SerdeItem>>;

/// `#[serde(..)]` everywhere in the expanded crate, before rustc lowers it.
pub fn attributes(tcx: TyCtxt<'_>) -> SerdeAttributes {
    struct Collector<'r, 'tcx> {
        found: SerdeAttributes,
        resolver: &'r rustc_middle::middle::resolve::ResolverAstLowering<'tcx>,
        /// The item whose variants and fields are visited, its resolver's.
        owner: Option<&'r rustc_middle::middle::resolve::PerOwnerResolverData<'tcx>>,
    }
    impl Collector<'_, '_> {
        /// The `DefId` of the node `id`: an item's own, or of a variant's or a
        /// field's, its item's.
        fn def_id(&self, id: rustc_ast::NodeId) -> Option<DefId> {
            let owner = self.owner?;
            let local = match owner.id == id {
                true => owner.def_id,
                false => *owner.node_id_to_def_id.get(&id)?,
            };
            Some(local.to_def_id())
        }

        fn record(&mut self, id: rustc_ast::NodeId, attrs: &[rustc_ast::Attribute]) {
            let Some(def_id) = self.def_id(id) else {
                return;
            };
            for attr in attrs.iter().filter(|a| a.has_name(Symbol::intern("serde"))) {
                for item in attr.meta_item_list().unwrap_or_default() {
                    let nested = item
                        .meta_item_list()
                        .unwrap_or_default()
                        .iter()
                        .map(|n| {
                            (
                                n.name().map(|s| s.to_string()).unwrap_or_default(),
                                n.value_str().map(|v| v.to_string()),
                            )
                        })
                        .collect();
                    self.found.entry(def_id).or_default().push(SerdeItem {
                        name: item.name().map(|n| n.to_string()).unwrap_or_default(),
                        value: item.value_str().map(|v| v.to_string()),
                        nested,
                        span: item.span(),
                    });
                }
            }
        }
    }
    impl<'a> Visitor<'a> for Collector<'_, '_> {
        fn visit_item(&mut self, item: &'a rustc_ast::Item) {
            let outer = std::mem::replace(&mut self.owner, self.resolver.owners.get(&item.id));
            if item.kind.ident().is_some() {
                self.record(item.id, &item.attrs);
            }
            visit::walk_item(self, item);
            self.owner = outer;
        }
        fn visit_variant(&mut self, variant: &'a rustc_ast::Variant) {
            self.record(variant.id, &variant.attrs);
            visit::walk_variant(self, variant);
        }
        fn visit_field_def(&mut self, field: &'a rustc_ast::FieldDef) {
            self.record(field.id, &field.attrs);
            visit::walk_field_def(self, field);
        }
    }
    let (resolver, krate) = tcx.resolver_for_lowering();
    let (resolver, krate) = (resolver.borrow(), krate.borrow());
    let mut collector = Collector {
        found: IndexMap::default(),
        resolver: &resolver,
        owner: None,
    };
    visit::walk_crate(&mut collector, &krate);
    collector.found
}

/// Each `#[serde(remote = ..)]`, refused before anything is lowered: serde's
/// derive of it writes inherent functions of a remote type's stand-in,
/// which no codec of rust-js's reads, so a field's `with` of it found none
/// and the compiler panicked (#1541). Whether any was.
pub(super) fn refuse_unread(tcx: TyCtxt<'_>, attrs: &SerdeAttributes) -> bool {
    let mut refused = false;
    for item in attrs.values().flatten().filter(|item| item.name == "remote") {
        let what = super::refusal::Refused::Feature("`#[serde(remote = ..)]` (#1615)");
        super::refusal::refuse(tcx, item.span, what);
        refused = true;
    }
    refused
}

/// `Some(true)` for serde's `Serialize`, `Some(false)` for `Deserialize`
/// and `DeserializeOwned`.
pub(super) use super::recognition::serde_trait;

/// How `rename_all` turns a Rust name into a JSON one: serde_derive's rules.
#[derive(Clone, Copy, PartialEq)]
enum Rule {
    Lower,
    Upper,
    Pascal,
    Camel,
    Snake,
    ScreamingSnake,
    Kebab,
    ScreamingKebab,
}

impl Rule {
    fn parse(name: &str) -> Option<Rule> {
        Some(match name {
            "lowercase" => Rule::Lower,
            "UPPERCASE" => Rule::Upper,
            "PascalCase" => Rule::Pascal,
            "camelCase" => Rule::Camel,
            "snake_case" => Rule::Snake,
            "SCREAMING_SNAKE_CASE" => Rule::ScreamingSnake,
            "kebab-case" => Rule::Kebab,
            "SCREAMING-KEBAB-CASE" => Rule::ScreamingKebab,
            _ => return None,
        })
    }

    /// A variant's name, written in `PascalCase`.
    fn variant(self, variant: &str) -> String {
        match self {
            Rule::Pascal => variant.to_owned(),
            Rule::Lower => variant.to_ascii_lowercase(),
            Rule::Upper => variant.to_ascii_uppercase(),
            Rule::Camel => variant[..1].to_ascii_lowercase() + &variant[1..],
            Rule::Snake => {
                let mut snake = String::new();
                for (i, ch) in variant.char_indices() {
                    if i > 0 && ch.is_uppercase() {
                        snake.push('_');
                    }
                    snake.push(ch.to_ascii_lowercase());
                }
                snake
            }
            Rule::ScreamingSnake => Rule::Snake.variant(variant).to_ascii_uppercase(),
            Rule::Kebab => Rule::Snake.variant(variant).replace('_', "-"),
            Rule::ScreamingKebab => Rule::ScreamingSnake.variant(variant).replace('_', "-"),
        }
    }

    /// A field's name, written in `snake_case`.
    fn field(self, field: &str) -> String {
        match self {
            Rule::Lower | Rule::Snake => field.to_owned(),
            Rule::Upper | Rule::ScreamingSnake => field.to_ascii_uppercase(),
            Rule::Pascal => {
                let mut pascal = String::new();
                let mut capitalize = true;
                for ch in field.chars() {
                    if ch == '_' {
                        capitalize = true;
                    } else if capitalize {
                        pascal.push(ch.to_ascii_uppercase());
                        capitalize = false;
                    } else {
                        pascal.push(ch);
                    }
                }
                pascal
            }
            Rule::Camel => {
                let pascal = Rule::Pascal.field(field);
                pascal[..1].to_ascii_lowercase() + &pascal[1..]
            }
            Rule::Kebab => field.replace('_', "-"),
            Rule::ScreamingKebab => field.to_ascii_uppercase().replace('_', "-"),
        }
    }
}

/// How an enum is written (serde's "enum representations").
#[derive(Clone, PartialEq)]
enum Tagging {
    /// `{"Circle": 2.5}`, the default.
    External,
    /// `#[serde(tag = "type")]`: `{"type": "Circle", ..}`.
    Internal(String),
    /// `#[serde(tag = "t", content = "c")]`: `{"t": "Circle", "c": 2.5}`.
    Adjacent(String, String),
    /// `#[serde(untagged)]`: `2.5`.
    Untagged,
}

/// `#[serde(default)]`, or `#[serde(default = "path")]`.
#[derive(Clone, Copy)]
enum SerdeDefault {
    Default,
    Path,
}

/// The `#[serde(..)]` attributes of a container, a variant or a field.
/// `rename` is the name JSON is written with, and `de_rename` the one it's
/// read with: `rename(serialize = "a", deserialize = "b")` tells them apart.
#[derive(Default)]
struct Attrs {
    rename: Option<String>,
    rename_all: Option<Rule>,
    rename_all_fields: Option<Rule>,
    de_rename: Option<String>,
    de_rename_all: Option<Rule>,
    de_rename_all_fields: Option<Rule>,
    aliases: Vec<String>,
    tag: Option<String>,
    content: Option<String>,
    untagged: bool,
    transparent: bool,
    skip_serializing: bool,
    skip_serializing_if: Option<String>,
    skip_deserializing: bool,
    default: Option<SerdeDefault>,
    deny_unknown_fields: bool,
    other: bool,
    expecting: Option<String>,
    /// `#[serde(from = "T")]`, `try_from` and `into`: the type is the one
    /// rustc resolved in the derive (`conversion`).
    from: bool,
    try_from: bool,
    into: bool,
    flatten: bool,
    /// `#[serde(serialize_with = "f")]`, `deserialize_with` and `with =
    /// "module"`: the attribute that names a field's own writer or reader.
    serialize_with: Option<&'static str>,
    deserialize_with: Option<&'static str>,
}

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// `#[serde(..)]` on `def_id`, as serde_derive reads it.
    fn serde_attrs(&self, def_id: DefId) -> R<Attrs> {
        let mut attrs = Attrs::default();
        for item in self.krate.serde_attrs.get(&def_id).into_iter().flatten() {
            let name = item.name.as_str();
            let value = item.value.clone();
            // `rename(serialize = "a", deserialize = "b")`: the one JSON is
            // written with, and the one it's read with.
            let named = |which: &str| {
                value.clone().or_else(|| {
                    item.nested
                        .iter()
                        .find(|(n, _)| n == which)
                        .and_then(|(_, v)| v.clone())
                })
            };
            let (written, read) = (named("serialize"), named("deserialize"));
            let rule = |name: &Option<String>| -> R<Option<Rule>> {
                match name {
                    Some(name) => Rule::parse(name)
                        .map(Some)
                        .ok_or_else(|| self.unsupported(item.span, &format!("`rename_all = {name:?}`"))),
                    None => Ok(None),
                }
            };
            match name {
                "rename" => (attrs.rename, attrs.de_rename) = (written, read),
                "rename_all" => (attrs.rename_all, attrs.de_rename_all) = (rule(&written)?, rule(&read)?),
                "rename_all_fields" => {
                    (attrs.rename_all_fields, attrs.de_rename_all_fields) = (rule(&written)?, rule(&read)?)
                }
                "alias" => attrs.aliases.extend(value),
                "tag" => attrs.tag = value,
                "content" => attrs.content = value,
                "untagged" => attrs.untagged = true,
                "transparent" => attrs.transparent = true,
                "skip" => (attrs.skip_serializing, attrs.skip_deserializing) = (true, true),
                "skip_serializing" => attrs.skip_serializing = true,
                "skip_serializing_if" => attrs.skip_serializing_if = value,
                "serialize_with" => attrs.serialize_with = Some("serialize_with"),
                "deserialize_with" => attrs.deserialize_with = Some("deserialize_with"),
                "with" => (attrs.serialize_with, attrs.deserialize_with) = (Some("with"), Some("with")),
                "skip_deserializing" => attrs.skip_deserializing = true,
                "default" => {
                    attrs.default = Some(if value.is_some() {
                        SerdeDefault::Path
                    } else {
                        SerdeDefault::Default
                    })
                }
                "deny_unknown_fields" => attrs.deny_unknown_fields = true,
                "other" => attrs.other = true,
                "expecting" => attrs.expecting = value,
                "from" => attrs.from = true,
                "try_from" => attrs.try_from = true,
                "into" => attrs.into = true,
                "flatten" => attrs.flatten = true,
                // Lifetimes and bounds, which rustc has checked; JS has neither.
                "borrow" | "bound" => {}
                _ => return Err(self.unsupported(item.span, &format!("`#[serde({name})]`"))),
            }
        }
        Ok(attrs)
    }

    /// A derived `serialize` or `deserialize`.
    pub(super) fn lower_codec(&mut self, method: DefId) -> R<js::Function> {
        match super::analysis::serde_impl(self.tcx, self.tcx.parent(method)) {
            Some(true) => self.lower_serialize(method),
            _ => self.lower_deserialize(method),
        }
    }

    /// `function orderSerialize_serialize(order, json) { .. }`: the derived
    /// `serialize`, as the steps it takes with `$json`.
    fn lower_serialize(&mut self, method: DefId) -> R<js::Function> {
        let imp = self.tcx.parent(method);
        let span = self.tcx.def_span(imp);
        let self_ty = self.tcx.type_of(imp).instantiate_identity().skip_normalization();
        let ty::Adt(adt, args) = *self_ty.kind() else {
            return Err(self.unsupported(span, "serializing this"));
        };
        let value = self.fresh(Base::Named(&lower_first(self.tcx.item_name(adt.did()).as_str())));
        let json = self.fresh(Role::Json);
        let mut params = vec![value.clone().into(), json.clone().into()];
        params.extend(self.codec_params(args, "write"));
        let mut body = Vec::new();
        self.write_adt(Expr::var(&value), &json, adt, args, span, &mut body)?;
        let js_span = self.js_span(span);
        Ok(js::Function {
            name: self.krate.fns[&method].name.clone(),
            params,
            body,
            export: false,
            is_async: false,
            noinline: false,
            span: js_span,
            name_span: js_span,
            param_spans: Vec::new(),
            set_apart: false,
        })
    }

    /// `serde_json::to_string(&v)` and `to_string_pretty`: `$toJson((json) =>
    /// { .. }, pretty)`, the steps for `v` in a function the writer calls.
    pub(super) fn json_text(&mut self, value: Expr, ty: Ty<'tcx>, pretty: bool, span: Span) -> R<Expr> {
        let write = self.json_writer(ty, span)?;
        Ok(Expr::call(
            Expr::helper("$toJson"),
            vec![value, write, Expr::bool(pretty)],
        ))
    }

    /// The function that writes a `ty` value, `(value, json) => ..`: a
    /// generic codec's parameter, the crate's own type's `serialize`, or
    /// the steps for it.
    pub(super) fn json_writer(&mut self, ty: Ty<'tcx>, span: Span) -> R<Expr> {
        let ty = ty.peel_refs();
        if let Some(writer) = self.serde_evidence(ty, true) {
            return Ok(writer);
        }
        if let Some(writer) = self.json_value_writer(ty) {
            return Ok(writer);
        }
        if let ty::Adt(_, args) = ty.kind()
            && args.types().next().is_none()
            && let Some(f) = self.serialize_fn(ty)
        {
            return Ok(self.codec_callee(f));
        }
        let (item, json) = (self.fresh(Role::Value), self.fresh(Role::Json));
        let mut body = Vec::new();
        self.write_json(Expr::var(&item), &json, ty, span, &mut body)?;
        Ok(Expr::arrow(vec![item.into(), json.into()], body))
    }

    /// `json.method(args)` as a statement.
    fn emit(&self, json: &str, method: &str, args: Vec<Expr>, out: &mut Vec<Stmt>) {
        let call = Expr::call(Expr::member(Expr::var(json), method), args);
        out.push(StmtKind::Expr(call).at(js::Span::NONE));
    }

    /// The steps that write `value`, a `ty`, as serde would.
    pub(super) fn write_json(
        &mut self,
        value: Expr,
        json: &str,
        ty: Ty<'tcx>,
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> R<()> {
        let ty = ty.peel_refs();
        // A type parameter: its writer's call.
        if let Some(writer) = self.serde_evidence(ty, true).or_else(|| self.json_value_writer(ty)) {
            let call = Expr::call(writer, vec![value, Expr::var(json)]);
            out.push(StmtKind::Expr(call).at(js::Span::NONE));
            return Ok(());
        }
        // A std type whose impl is serde's own, as its catalog row says,
        // a `Duration` too, which rust-js holds as a number (#1623).
        if let Some(form) = self.std_form(ty) {
            return self.write_std_form(form, value, json, span, out);
        }
        // Each kind by its own method, which a flattened field tells apart.
        if ty.is_unit() {
            self.emit(json, "null", Vec::new(), out);
            return Ok(());
        }
        if ty.is_bool() {
            self.emit(json, "bool", vec![value], out);
            return Ok(());
        }
        // serde_json writes an `f32` with its own shortest digits, and a
        // `Value` of it is its `f64` (ADR 0122).
        if Num::of(ty) == Some(Num::F32) {
            self.emit(json, "number", vec![value, Expr::bool(true)], out);
            return Ok(());
        }
        // A 128-bit one by its type's name, which a `Value`'s writer holds
        // only where it fits in 64 bits, serde_json's `number out of range`
        // (#1547, #1623).
        if Num::of(ty).is_some_and(|n| n.bits() == 128) {
            self.emit(json, "int", vec![value, Expr::str(ty.to_string())], out);
            return Ok(());
        }
        if Num::of(ty).is_some_and(|n| !n.float()) {
            self.emit(json, "int", vec![value], out);
            return Ok(());
        }
        if Num::of(ty) == Some(Num::F64) {
            self.emit(json, "number", vec![value], out);
            return Ok(());
        }
        if self.is_string_like(ty) {
            self.emit(json, if ty.is_char() { "char" } else { "string" }, vec![value], out);
            return Ok(());
        }
        // Read more than once below.
        let value = if value.reads_same() {
            value
        } else {
            self.spill(Role::Value, value, out)
        };
        if let Some(inner) = self.option_of(ty) {
            // Of a type parameter, a `Some` may be boxed (ADR 0051).
            let payload = if self.boxed_payload(inner) {
                self.some_value(value.clone())
            } else {
                value.clone()
            };
            let mut some = Vec::new();
            self.write_json(payload, json, inner, span, &mut some)?;
            let mut none = Vec::new();
            self.emit(json, "null", Vec::new(), &mut none);
            let test = Expr::presence(value, false);
            out.push(StmtKind::If(test, none, Some(some)).at(js::Span::NONE));
            return Ok(());
        }
        match ty.kind() {
            // A counted `Rc` writes what it points at (ADR 0320).
            ty::Adt(_, args) if self.counted_rc(ty).is_some() => {
                let pointee = self.counted_pointee(value, args.type_at(0));
                self.write_json(pointee, json, args.type_at(0), span, out)
            }
            ty::Adt(_, args) if ty.is_box() || self.is_rc(ty) => {
                self.write_json(value, json, args.type_at(0), span, out)
            }
            // `{"Ok": ..}` or `{"Err": ..}`, as serde's impl writes one.
            ty::Adt(adt, args) if self.is_std_type(ty, StdItem::Result) => {
                let mut branches = Vec::new();
                for (i, variant) in adt.variants().iter().enumerate() {
                    let mut body = Vec::new();
                    self.emit(json, "beginObject", Vec::new(), &mut body);
                    self.emit(json, "key", vec![Expr::str(variant.name.as_str())], &mut body);
                    self.write_json(result::held(value.clone()), json, args.type_at(i), span, &mut body)?;
                    self.emit(json, "endObject", Vec::new(), &mut body);
                    branches.push(body);
                }
                let err = branches.pop().expect("an `Err` branch");
                let ok = branches.pop().expect("an `Ok` branch");
                let test = result::is(value, true);
                out.push(StmtKind::If(test, ok, Some(err)).at(js::Span::NONE));
                Ok(())
            }
            ty::Adt(_, args) if self.is_vec_like(ty) || (self.is_set(ty) && !self.is_sorted(ty)) => {
                self.write_items(value, json, "beginArray", args.type_at(0), span, out)
            }
            ty::Adt(_, args) if self.is_set(ty) => {
                let items = self.in_order_of(value, ty, span)?;
                self.write_items(items, json, "beginArray", args.type_at(0), span, out)
            }
            // serde writes an array as a tuple.
            ty::Array(item, _) => self.write_items(value, json, "beginTuple", *item, span, out),
            ty::Slice(item) => self.write_items(value, json, "beginArray", *item, span, out),
            ty::Tuple(tys) => {
                let tys: Vec<Ty<'tcx>> = tys.to_vec();
                self.emit(json, "beginTuple", Vec::new(), out);
                for (i, t) in tys.into_iter().enumerate() {
                    self.emit(json, "element", Vec::new(), out);
                    self.write_json(Expr::index(value.clone(), Expr::int(i as i128)), json, t, span, out)?;
                }
                self.emit(json, "endArray", Vec::new(), out);
                Ok(())
            }
            ty::Adt(_, args) if self.is_map(ty) => {
                let (key_ty, item_ty) = (args.type_at(0), args.type_at(1));
                let entries = self.in_order_of(value, ty, span)?;
                let (key, item) = (self.fresh(Role::Key), self.fresh(Role::Item));
                let mut body = Vec::new();
                let key_text = self.json_key(Expr::var(&key), key_ty, span)?;
                self.emit(json, "key", vec![key_text], &mut body);
                self.write_json(Expr::var(&item), json, item_ty, span, &mut body)?;
                self.emit(json, "beginObject", Vec::new(), out);
                out.push(
                    StmtKind::ForOf {
                        label: None,
                        pattern: js::Pattern::names_array([Some(key), Some(item)]),
                        mutable: false,
                        iterable: entries,
                        body,
                        keys: false,
                    }
                    .at(js::Span::NONE),
                );
                self.emit(json, "endObject", Vec::new(), out);
                Ok(())
            }
            ty::Adt(..) if let Some(serialize) = self.serialize_fn(ty) => {
                let callee = self.codec_callee(serialize);
                let mut call_args = vec![value, Expr::var(json)];
                for t in self.codec_type_args(ty, true) {
                    call_args.push(self.json_writer(t, span)?);
                }
                out.push(StmtKind::Expr(Expr::call(callee, call_args)).at(js::Span::NONE));
                Ok(())
            }
            _ => Err(self.unsupported(span, &format!("serializing `{ty}`"))),
        }
    }

    /// `[a, b]`: each item, in its turn, after `begin`, `beginArray` or
    /// `beginTuple`.
    fn write_items(
        &mut self,
        items: Expr,
        json: &str,
        begin: &str,
        item_ty: Ty<'tcx>,
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> R<()> {
        let item = self.fresh(Role::Item);
        let mut body = Vec::new();
        self.emit(json, "element", Vec::new(), &mut body);
        self.write_json(Expr::var(&item), json, item_ty, span, &mut body)?;
        self.emit(json, begin, Vec::new(), out);
        out.push(
            StmtKind::ForOf {
                label: None,
                pattern: js::Pattern::Name(item),
                mutable: false,
                iterable: items,
                body,
                keys: false,
            }
            .at(js::Span::NONE),
        );
        self.emit(json, "endArray", Vec::new(), out);
        Ok(())
    }

    /// A map's key, as serde_json writes one: a string, or a number or a
    /// `bool` in quotes.
    pub(super) fn json_key(&mut self, key: Expr, ty: Ty<'tcx>, span: Span) -> R<Expr> {
        let ty = ty.peel_refs();
        if self.is_string_like(ty) {
            return Ok(key);
        }
        if ty.is_bool() || Num::of(ty).is_some_and(|n| !n.float()) {
            return Ok(Expr::call(Expr::js_global("String"), vec![key]).built_in());
        }
        Err(self.unsupported(span, &format!("a map key of `{ty}` in JSON")))
    }

    /// A struct or an enum of the crate's own, by its `#[serde]` attributes.
    fn write_adt(
        &mut self,
        value: Expr,
        json: &str,
        adt: ty::AdtDef<'tcx>,
        args: ty::GenericArgsRef<'tcx>,
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> R<()> {
        let container = self.serde_attrs(adt.did())?;
        let ty = Ty::new_adt(self.tcx, adt, args);
        // `#[serde(into = "T")]`: a clone of it, as a `T`, written as one.
        if container.into {
            let Some(target) = self.conversion(adt.did(), StdItem::Into, true) else {
                return Err(self.unsupported(span, "this `#[serde(into)]`"));
            };
            let cloned = self.clone_value(value, ty, span, out)?;
            let converted = self.convert(StdItem::From, target, ty, cloned, span)?;
            return self.write_json(converted, json, target, span, out);
        }
        if adt.is_struct() {
            let variant = adt.non_enum_variant();
            let fields: Vec<(Expr, Ty<'tcx>)> = (0..variant.fields.len())
                .map(|i| {
                    (
                        self.project(value.clone(), ty, i),
                        variant
                            .fields
                            .iter()
                            .nth(i)
                            .map(|field| self.field_ty(field, args))
                            .expect("a field"),
                    )
                })
                .collect();
            // `#[serde(transparent)]` and a newtype `struct Id(u32)`: the field itself.
            if container.transparent || (variant.ctor_kind() == Some(CtorKind::Fn) && fields.len() == 1) {
                let written = self.written_field(variant, &fields)?;
                let (field, value, field_ty) = written
                    .into_iter()
                    .next()
                    .ok_or_else(|| self.unsupported(span, "this struct"))?;
                // By its own writer, where it names one (#1536).
                let codec = self.field_codec(field, &self.serde_attrs(field)?, true, span)?;
                return self.write_field(codec, value, json, field_ty, span, out);
            }
            return match variant.ctor_kind() {
                Some(CtorKind::Const) => {
                    self.emit(json, "null", Vec::new(), out);
                    Ok(())
                }
                Some(CtorKind::Fn) => {
                    let counts = self.skip_counts(variant, &fields, span)?;
                    out.extend(counts);
                    self.emit(json, "beginTupleStruct", Vec::new(), out);
                    self.write_tuple_fields(json, variant, &fields, span, out)?;
                    self.emit(json, "endArray", Vec::new(), out);
                    Ok(())
                }
                None => {
                    let counts = self.skip_counts(variant, &fields, span)?;
                    out.extend(counts);
                    self.emit(json, "beginObject", Vec::new(), out);
                    self.write_struct_tag(json, adt, &container, out);
                    self.write_fields(json, variant, &fields, container.rename_all, span, out)?;
                    self.emit(json, "endObject", Vec::new(), out);
                    Ok(())
                }
            };
        }
        let tagging = match (&container.tag, &container.content, container.untagged) {
            (_, _, true) => Tagging::Untagged,
            (Some(tag), Some(content), _) => Tagging::Adjacent(tag.clone(), content.clone()),
            (Some(tag), None, _) => Tagging::Internal(tag.clone()),
            _ => Tagging::External,
        };
        // Each variant in turn: `if (v === "Dot") .. else if (v.TAG === "Ring") ..`, each
        // tested as a pattern tests it.
        let mut chain: Option<Vec<Stmt>> = None;
        for variant in adt.variants().iter().rev() {
            let attrs = self.serde_attrs(variant.def_id)?;
            let rust_name = variant.name.to_string();
            let name = attrs.rename.clone().unwrap_or_else(|| {
                container
                    .rename_all
                    .map_or(rust_name.clone(), |r| r.variant(&rust_name))
            });
            let mut body = Vec::new();
            if attrs.skip_serializing {
                let message = format!(
                    "the enum variant {}::{} cannot be serialized",
                    self.tcx.item_name(adt.did()),
                    rust_name
                );
                body.push(
                    StmtKind::Throw(Expr::call(Expr::helper("$jsonError"), vec![Expr::str(message)]))
                        .at(js::Span::NONE),
                );
            } else {
                let fields: Vec<(Expr, Ty<'tcx>)> = variant
                    .fields
                    .iter()
                    .enumerate()
                    .map(|(i, f)| {
                        (
                            Expr::member(value.clone(), variant_field(self.tcx, variant, i)),
                            self.field_ty(f, args),
                        )
                    })
                    .collect();
                let fields_rule = attrs.rename_all.or(container.rename_all_fields);
                let tagging = if attrs.untagged { &Tagging::Untagged } else { &tagging };
                self.write_variant(json, tagging, &name, variant, &fields, fields_rule, span, &mut body)?;
            }
            chain = Some(match chain {
                None => body,
                Some(rest) => {
                    // As a pattern tests it, of any representation (ADR 0284).
                    let index = adt.variant_index_with_id(variant.def_id);
                    let test = self.variant_test(value.clone(), Ty::new_adt(self.tcx, adt, args), adt, index, false);
                    vec![StmtKind::If(test, body, Some(rest)).at(js::Span::NONE)]
                }
            });
        }
        out.extend(chain.unwrap_or_default());
        Ok(())
    }

    /// The fields that aren't skipped, as `(field, value, type)`: for a
    /// `#[serde(transparent)]` struct, the one that's written.
    fn written_field(&self, variant: &ty::VariantDef, fields: &[(Expr, Ty<'tcx>)]) -> R<Vec<(DefId, Expr, Ty<'tcx>)>> {
        let mut written = Vec::new();
        for (field, (value, ty)) in variant.fields.iter().zip(fields) {
            if !self.serde_attrs(field.did)?.skip_serializing {
                written.push((field.did, value.clone(), *ty));
            }
        }
        Ok(written)
    }

    fn write_struct_tag(&self, json: &str, adt: ty::AdtDef<'tcx>, attrs: &Attrs, out: &mut Vec<Stmt>) {
        if let Some(tag) = &attrs.tag {
            let name = attrs
                .rename
                .clone()
                .unwrap_or_else(|| self.tcx.item_name(adt.did()).to_string());
            self.emit(json, "key", vec![Expr::str(tag.as_str())], out);
            self.emit(json, "string", vec![Expr::str(name)], out);
        }
    }

    fn write_tuple_fields(
        &mut self,
        json: &str,
        variant: &ty::VariantDef,
        fields: &[(Expr, Ty<'tcx>)],
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> R<()> {
        for (field, (value, ty)) in variant.fields.iter().zip(fields) {
            let attrs = self.serde_attrs(field.did)?;
            if attrs.skip_serializing {
                continue;
            }
            let codec = self.field_codec(field.did, &attrs, true, span)?;
            let mut body = Vec::new();
            self.emit(json, "element", Vec::new(), &mut body);
            self.write_field(codec, value.clone(), json, *ty, span, &mut body)?;
            if let Some(path) = attrs.skip_serializing_if {
                let skip = self.skip_test(field.did, &path, value.clone(), *ty, span)?;
                out.push(StmtKind::If(Expr::unary(js::UnaryOp::Not, skip), body, None).at(js::Span::NONE));
            } else {
                out.extend(body);
            }
        }
        Ok(())
    }

    /// An internally tagged newtype variant's payload, `value`, as serde's
    /// `TaggedSerializer` writes it (#1540): by its own writer, given the
    /// entries' adapter `within` the variant, `json.flat(value, writer,
    /// "E::A")`, which writes a struct's or a map's entries after the tag,
    /// opened first (`opening`), `()` as the tag alone, a variant as an entry
    /// of its name, and serde's own error for anything else. An `Option`,
    /// which no writer tells from `()`, is serde's error from its type,
    /// returned before anything's opened (`tagged_optional`).
    #[allow(clippy::too_many_arguments)]
    fn write_tagged_payload(
        &mut self,
        value: Expr,
        json: &str,
        ty: Ty<'tcx>,
        within: String,
        opening: Vec<Stmt>,
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> R<()> {
        if self.tagged_optional(ty, span)? {
            let message = format!("cannot serialize tagged newtype variant {within} containing an optional");
            out.push(
                StmtKind::Throw(Expr::call(Expr::helper("$jsonError"), vec![Expr::str(message)])).at(js::Span::NONE),
            );
            return Ok(());
        }
        let writer = self.json_writer(ty, span)?;
        out.extend(opening);
        self.emit(json, "flat", vec![value, writer, Expr::str(within)], out);
        Ok(())
    }

    /// Whether `ty`, an internally tagged newtype variant's payload, is an
    /// `Option` as serde writes one: itself, or what a `Box`, an `Rc`, a
    /// newtype struct or a `transparent` one holds, each written as what it
    /// holds. Refused where the type doesn't say: a type parameter's, or a
    /// payload serde's derive doesn't write, whose `Option` a writer can't
    /// tell, which #1568 is for.
    fn tagged_optional(&mut self, ty: Ty<'tcx>, span: Span) -> R<bool> {
        let ty = ty.peel_refs();
        let refused = |this: &Self, what: &str| {
            Err(this.unsupported(span, &format!("an internally tagged newtype variant of {what} (#1568)")))
        };
        if self.option_of(ty).is_some() {
            return Ok(true);
        }
        match *ty.kind() {
            // Of a type parameter, whose writer a caller gives.
            _ if self.serde_evidence(ty, true).is_some() => refused(self, "a type parameter"),
            ty::Adt(_, args) if ty.is_box() || self.is_rc(ty) || self.counted_rc(ty).is_some() => {
                self.tagged_optional(args.type_at(0), span)
            }
            ty::Adt(adt, args)
                if adt.is_struct()
                    && let Some(f) = self.serialize_fn(ty) =>
            {
                let attrs = self.serde_attrs(adt.did())?;
                if !super::recognition::serde_derived(self.tcx, self.tcx.parent(f)) || attrs.into {
                    return refused(self, "a payload that isn't written by serde's derive of its fields");
                }
                let variant = adt.non_enum_variant();
                if !attrs.transparent && !(variant.ctor_kind() == Some(CtorKind::Fn) && variant.fields.len() == 1) {
                    return Ok(false);
                }
                for field in &variant.fields {
                    let field_attrs = self.serde_attrs(field.did)?;
                    if field_attrs.skip_serializing {
                        continue;
                    }
                    if field_attrs.serialize_with.is_some() {
                        return refused(self, "a payload whose field has its own writer");
                    }
                    let field_ty = self.field_ty(field, args);
                    return self.tagged_optional(field_ty, span);
                }
                Ok(false)
            }
            _ => Ok(false),
        }
    }

    /// serde_derive's count of what a struct, a tuple struct or a variant of
    /// them writes, `serialize_struct(name, len)`: each `skip_serializing_if`
    /// predicate called before any field is written, then again as each is,
    /// as statements, in its fields' order. A crate's own predicate may
    /// answer each call differently, or show it; std's (`skip_predicate`)
    /// only asks what it's given, so its count is nothing. None where a field
    /// is flattened: serde then writes a map of no length,
    /// `serialize_map(None)` (#1543).
    fn skip_counts(&mut self, variant: &ty::VariantDef, fields: &[(Expr, Ty<'tcx>)], span: Span) -> R<Vec<Stmt>> {
        let mut counts = Vec::new();
        let mut flattened = false;
        for (field, (value, ty)) in variant.fields.iter().zip(fields) {
            let attrs = self.serde_attrs(field.did)?;
            flattened |= attrs.flatten;
            let Some(path) = attrs.skip_serializing_if.filter(|_| !attrs.skip_serializing) else {
                continue;
            };
            if (self.resolved_skip(field.did)).is_some_and(|f| self.recognition().skip_predicate(f, *ty).is_some()) {
                continue;
            }
            let test = self.skip_test(field.did, &path, value.clone(), *ty, span)?;
            counts.push(StmtKind::Expr(test).at(js::Span::NONE));
        }
        Ok(if flattened { Vec::new() } else { counts })
    }

    /// A struct's (or a struct variant's) fields, inside its `{ .. }`.
    fn write_fields(
        &mut self,
        json: &str,
        variant: &ty::VariantDef,
        fields: &[(Expr, Ty<'tcx>)],
        rule: Option<Rule>,
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> R<()> {
        for (field, (value, ty)) in variant.fields.iter().zip(fields) {
            let attrs = self.serde_attrs(field.did)?;
            if attrs.skip_serializing {
                continue;
            }
            let codec = self.field_codec(field.did, &attrs, true, span)?;
            let rust_name = field.name.to_string();
            let rust_name = rust_name.strip_prefix("r#").unwrap_or(&rust_name).to_string();
            let name = attrs
                .rename
                .unwrap_or_else(|| rule.map_or(rust_name.clone(), |r| r.field(&rust_name)));
            let mut body = Vec::new();
            // `#[serde(flatten)]`: its entries, among these.
            if attrs.flatten {
                // Its own writer, where it names one, given the entries' (#1536).
                let writer = match codec {
                    Some(codec) => codec,
                    None => self.json_writer(*ty, span)?,
                };
                self.emit(json, "flat", vec![value.clone(), writer], &mut body);
            } else {
                self.emit(json, "key", vec![Expr::str(name)], &mut body);
                // Written only when it's `Some`, so it's written as what it holds.
                let written = match (attrs.skip_serializing_if.as_ref(), self.option_of(*ty)) {
                    (Some(_), Some(inner))
                        if !self.boxed_payload(inner)
                            && self
                                .resolved_skip(field.did)
                                .is_some_and(|f| self.recognition().skips_none(f)) =>
                    {
                        inner
                    }
                    _ => *ty,
                };
                self.write_field(codec, value.clone(), json, written, span, &mut body)?;
            }
            match attrs.skip_serializing_if {
                Some(path) => {
                    let skip = self.skip_test(field.did, &path, value.clone(), *ty, span)?;
                    out.push(StmtKind::If(Expr::unary(js::UnaryOp::Not, skip), body, None).at(js::Span::NONE));
                }
                None => out.extend(body),
            }
        }
        Ok(())
    }

    /// `#[serde(skip_serializing_if = "Option::is_none")]`: the test, for
    /// std's own. The crate's own function is called as it's named.
    fn skip_test(&mut self, field: DefId, path: &str, value: Expr, ty: Ty<'tcx>, span: Span) -> R<Expr> {
        let Some(f) = self.resolved_skip(field) else {
            return Err(self.unsupported(span, &format!("`skip_serializing_if = {path:?}` of `{ty}`")));
        };
        match self.recognition().skip_predicate(f, ty) {
            Some(SkipPredicate::None) => Ok(Expr::bin(Op::LooseEq, value, Expr::null())),
            Some(SkipPredicate::Some) => Ok(Expr::bin(Op::LooseNe, value, Expr::null())),
            Some(SkipPredicate::Empty) => Ok(Expr::bin(Op::Eq, Expr::member(value, "length"), Expr::int(0))),
            _ if self.krate.fns.contains_key(&f) => Ok(Expr::call(self.fn_ref(f), vec![value])),
            _ => Err(self.unsupported(span, &format!("`skip_serializing_if = {path:?}` of `{ty}`"))),
        }
    }

    /// `<to as From<from>>::from(value)` (or `TryFrom`), of the crate's own impl.
    fn convert(&mut self, convert: StdItem, to: Ty<'tcx>, from: Ty<'tcx>, value: Expr, span: Span) -> R<Expr> {
        let trait_id = std_item(self.tcx, convert);
        let method = self
            .tcx
            .associated_items(trait_id)
            .in_definition_order()
            .find(|item| item.is_fn())
            .expect("a conversion method")
            .def_id;
        let args = self.tcx.mk_args(&[to.into(), from.into()]);
        self.impl_call(method, args, vec![value], span)
    }

    /// A field's own writer, or reader, where `#[serde(serialize_with =
    /// "f")]`, `deserialize_with` or `with = "module"` names one: the function
    /// rustc resolved, a hand-written codec, `$jsonCodec(f)` (ADR 0459).
    fn field_codec(&mut self, field: DefId, attrs: &Attrs, serialize: bool, span: Span) -> R<Option<Expr>> {
        let named = if serialize {
            attrs.serialize_with
        } else {
            attrs.deserialize_with
        };
        let Some(name) = named else {
            return Ok(None);
        };
        let Some(f) = self.resolved_path(field, name, serialize) else {
            return Err(self.unsupported(span, &format!("`#[serde({name} = ..)]` of this field")));
        };
        Ok(Some(Expr::call(Expr::helper("$jsonCodec"), vec![self.fn_ref(f)])))
    }

    /// A newtype variant's own writer, or reader: `#[serde(with = ..)]`,
    /// `serialize_with` or `deserialize_with` on the variant, which serde
    /// gives its one field, or else on its field (`field_codec`). On a
    /// variant of no field or of more, refused: serde gives it all of them,
    /// which this doesn't (#1536).
    fn newtype_variant_codec(&mut self, variant: &ty::VariantDef, serialize: bool, span: Span) -> R<Option<Expr>> {
        let attrs = self.serde_attrs(variant.def_id)?;
        let own = if serialize {
            attrs.serialize_with
        } else {
            attrs.deserialize_with
        };
        match (variant.ctor_kind(), variant.fields.len()) {
            (Some(CtorKind::Fn), 1) => match own {
                Some(_) => self.field_codec(variant.def_id, &attrs, serialize, span),
                None => {
                    let field = variant.fields.iter().next().expect("a field").did;
                    let field_attrs = self.serde_attrs(field)?;
                    self.field_codec(field, &field_attrs, serialize, span)
                }
            },
            _ => match own {
                Some(name) => Err(self.unsupported(
                    span,
                    &format!("`#[serde({name} = ..)]` on a variant that isn't a newtype (#1556)"),
                )),
                None => Ok(None),
            },
        }
    }

    /// A field's `value` written: by its own writer (`field_codec`), or as
    /// its type is.
    #[allow(clippy::too_many_arguments)]
    fn write_field(
        &mut self,
        codec: Option<Expr>,
        value: Expr,
        json: &str,
        ty: Ty<'tcx>,
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> R<()> {
        match codec {
            Some(codec) => {
                out.push(StmtKind::Expr(Expr::call(codec, vec![value, Expr::var(json)])).at(js::Span::NONE));
                Ok(())
            }
            None => self.write_json(value, json, ty, span, out),
        }
    }

    /// Serde's derive has already asked rustc to resolve the predicate. Its
    /// path retains the attribute's source span, including imports and aliases.
    fn resolved_skip(&self, field: DefId) -> Option<DefId> {
        self.resolved_path(field, "skip_serializing_if", true)
    }

    /// The function a `#[serde(name = "path")]` on `def_id` names, as rustc
    /// resolved it in the derived `Serialize` impl, or `Deserialize`'s.
    fn resolved_path(&self, def_id: DefId, name: &str, serialize: bool) -> Option<DefId> {
        struct Predicate<'tcx> {
            types: &'tcx ty::TypeckResults<'tcx>,
            span: Span,
            found: Option<DefId>,
        }
        impl<'tcx> intravisit::Visitor<'tcx> for Predicate<'tcx> {
            fn visit_expr(&mut self, expr: &'tcx hir::Expr<'tcx>) {
                // Not what `?` of its call desugars to, `lower(d)?`'s `branch`
                // and `from_residual`, under the same span.
                if let hir::ExprKind::Path(ref path) = expr.kind
                    && expr.span.desugaring_kind().is_none()
                    && self.span.lo() <= expr.span.lo()
                    && expr.span.hi() <= self.span.hi()
                    && let hir::def::Res::Def(_, id) = self.types.qpath_res(path, expr.hir_id)
                {
                    self.found = Some(id);
                }
                intravisit::walk_expr(self, expr);
            }
        }
        let span = self
            .krate
            .serde_attrs
            .get(&def_id)?
            .iter()
            .find(|item| item.name == name)?
            .span;
        for owner in self.tcx.hir_body_owners() {
            // Deserializing, it's called in the derive's visitor, inside the impl.
            let mut parent = self.tcx.opt_parent(owner.to_def_id());
            while let Some(id) = parent
                && super::analysis::serde_impl(self.tcx, id).is_none()
            {
                parent = self.tcx.opt_parent(id);
            }
            if parent.is_none_or(|parent| super::analysis::serde_impl(self.tcx, parent) != Some(serialize)) {
                continue;
            }
            let body = self.tcx.hir_body_owned_by(owner);
            let mut predicate = Predicate {
                types: self.tcx.typeck_body(body.id()),
                span,
                found: None,
            };
            intravisit::Visitor::visit_body(&mut predicate, body);
            if predicate.found.is_some() {
                return predicate.found;
            }
        }
        None
    }

    /// One enum variant, as the enum's tagging writes it.
    #[allow(clippy::too_many_arguments)]
    fn write_variant(
        &mut self,
        json: &str,
        tagging: &Tagging,
        name: &str,
        variant: &ty::VariantDef,
        fields: &[(Expr, Ty<'tcx>)],
        fields_rule: Option<Rule>,
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> R<()> {
        let kind = variant.ctor_kind();
        // A newtype variant's writer: the variant's own, `#[serde(with)]` on
        // it, or its field's (#1536).
        let codec = self.newtype_variant_codec(variant, true, span)?;
        if codec.is_some() && matches!(tagging, Tagging::Internal(_)) {
            return Err(self.unsupported(span, "an internally tagged newtype variant with its own writer (#1568)"));
        }
        // A tuple or struct variant's count of what it writes: before it's
        // opened, or adjacently tagged, before its content, which serde
        // writes as a struct of its own (`skip_counts`).
        let counts = match kind {
            Some(CtorKind::Fn) if fields.len() == 1 => Vec::new(),
            Some(CtorKind::Const) => Vec::new(),
            _ => self.skip_counts(variant, fields, span)?,
        };
        let adjacent = matches!(tagging, Tagging::Adjacent(..));
        // The variant's content, without its tag.
        let content = |this: &mut Self, out: &mut Vec<Stmt>| -> R<()> {
            match kind {
                Some(CtorKind::Const) => {
                    this.emit(json, "null", Vec::new(), out);
                    Ok(())
                }
                Some(CtorKind::Fn) if fields.len() == 1 => {
                    this.write_field(codec.clone(), fields[0].0.clone(), json, fields[0].1, span, out)
                }
                Some(CtorKind::Fn) => {
                    if adjacent {
                        out.extend(counts.iter().cloned());
                    }
                    this.emit(json, "beginTuple", Vec::new(), out);
                    this.write_tuple_fields(json, variant, fields, span, out)?;
                    this.emit(json, "endArray", Vec::new(), out);
                    Ok(())
                }
                None => {
                    if adjacent {
                        out.extend(counts.iter().cloned());
                    }
                    this.emit(json, "beginObject", Vec::new(), out);
                    this.write_fields(json, variant, fields, fields_rule, span, out)?;
                    this.emit(json, "endObject", Vec::new(), out);
                    Ok(())
                }
            }
        };
        if !adjacent {
            out.extend(counts.iter().cloned());
        }
        match tagging {
            Tagging::External if kind == Some(CtorKind::Const) => {
                self.emit(json, "variant", vec![Expr::str(name)], out);
            }
            Tagging::External => {
                self.emit(json, "beginObject", Vec::new(), out);
                self.emit(json, "key", vec![Expr::str(name)], out);
                content(self, out)?;
                self.emit(json, "endObject", Vec::new(), out);
            }
            Tagging::Untagged => content(self, out)?,
            Tagging::Adjacent(tag, body) => {
                self.emit(json, "beginObject", Vec::new(), out);
                self.emit(json, "key", vec![Expr::str(tag.as_str())], out);
                self.emit(json, "string", vec![Expr::str(name)], out);
                if kind != Some(CtorKind::Const) {
                    self.emit(json, "key", vec![Expr::str(body.as_str())], out);
                    content(self, out)?;
                }
                self.emit(json, "endObject", Vec::new(), out);
            }
            Tagging::Internal(tag) => {
                let mut opening = Vec::new();
                self.emit(json, "beginObject", Vec::new(), &mut opening);
                self.emit(json, "key", vec![Expr::str(tag.as_str())], &mut opening);
                self.emit(json, "string", vec![Expr::str(name)], &mut opening);
                match kind {
                    Some(CtorKind::Const) | None => out.extend(opening),
                    // A newtype variant: its payload, after the tag, as serde's
                    // `TaggedSerializer` writes it (`write_tagged_payload`).
                    Some(CtorKind::Fn) if fields.len() == 1 => {
                        let (inner, inner_ty) = fields[0].clone();
                        let enum_name = self.tcx.item_name(self.tcx.parent(variant.def_id));
                        let within = format!("{enum_name}::{}", variant.name);
                        self.write_tagged_payload(inner, json, inner_ty, within, opening, span, out)?;
                    }
                    Some(CtorKind::Fn) => return Err(self.unsupported(span, "an internally tagged tuple variant")),
                }
                if kind.is_none() {
                    self.write_fields(json, variant, fields, fields_rule, span, out)?;
                }
                self.emit(json, "endObject", Vec::new(), out);
            }
        }
        Ok(())
    }
}
