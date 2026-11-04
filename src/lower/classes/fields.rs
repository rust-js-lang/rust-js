//! An ES class's field declarations (#568, ADR 0761 amended): `class
//! ReactFlightWebpackPlugin { clientReferences; chunkName; ..
//! constructor(options) { .. } }`, as upstream declares them, where JS's
//! define of a declared field can't do other than the constructor's
//! assignment that follows it.

use std::collections::HashSet;

use rustc_middle::ty::TyCtxt;
use rustc_span::def_id::{DefId, LocalModId};

use crate::js::{self, ExprKind, Stmt, StmtKind};

/// Of `module`'s items, the fields of each class that declares them,
/// `#[rust_js::class(es, fields)]` (#568, the user's decision of
/// 2026-11-04), `js::Class::fields`, in the order its constructor sets
/// them, or what's refused where a declaration would change what its
/// constructor does.
///
/// A declaration defines each field on the instance, `undefined`, before
/// the constructor's body runs, where an assignment sets it after, through
/// what the prototype chain has of its name. They differ, so it's refused,
/// saying why, where:
/// - the crate reads its prototype as a value, `js::class::<T>().prototype()`,
///   where a setter may be defined of a field's name, which the assignment
///   calls and the define hides;
/// - its constructor doesn't set each field, by its JS name, in one run of
///   `this.name = value;` before which nothing reads `this`: one left
///   unset, its `None` left out, would be present, `undefined`, where it's
///   absent;
/// - a field isn't a JS name (one named `__proto__` is refused of any
///   struct, rd754, and `constructor` of any class, rd750).
///
/// A class that extends a binding's, whose prototype may have an accessor
/// of a field's name, is refused `es` already (`forms::form_misuse`).
pub(super) fn declare_fields(
    tcx: TyCtxt<'_>,
    (items, module): (&mut [js::Item], LocalModId),
    prototypes_read: &HashSet<DefId>,
    owner_of: impl Fn(DefId) -> Option<String>,
    sources: &crate::lower::sources::CapturedSources,
) {
    let refuse = |adt: DefId, why: &str| {
        tcx.dcx().span_err(
            tcx.def_span(adt),
            format!("rust-js: a `#[rust_js::class(es, fields)]` declares its fields where that can't change what its constructor does: {why}; write `#[rust_js::class(es)]`, whose constructor sets them (#568, ADR 0761)"),
        );
    };
    for adt in super::es_classes(tcx) {
        if super::home(tcx, adt) != Some(module) || !super::declares_fields(tcx, adt) {
            continue;
        }
        if prototypes_read.contains(&adt) {
            refuse(
                adt,
                "its prototype is read, `js::class::<T>().prototype()`, where a setter of a field's name may be defined, which a declaration would hide",
            );
            continue;
        }
        let Some(owner) = super::constructor_of(tcx, adt).and_then(&owner_of) else {
            continue;
        };
        // Each field's JS name, and where it's written, which its declaration
        // is, the struct's lines apart from its constructor's.
        let keys: Option<Vec<(String, js::Span)>> = (tcx.adt_def(adt).non_enum_variant().fields.iter())
            .map(|field| {
                let name = crate::lower::bindings::field_key(tcx, field).name()?.to_owned();
                Some((name, sources.span(tcx.def_span(field.did))))
            })
            .collect();
        let Some(keys) = keys else { continue };
        for item in items.iter_mut() {
            let js::Item::Namespace(n) = item else { continue };
            if n.name != owner {
                continue;
            }
            let Some(class) = n.class.as_mut() else { continue };
            let Some(constructor) = n.methods.iter().find(|f| f.name == class.constructor) else {
                continue;
            };
            let names: Vec<String> = keys.iter().map(|(name, _)| name.clone()).collect();
            match declarable(&constructor.body, &names) {
                Ok(fields) => {
                    let at = |name: &String| {
                        keys.iter()
                            .find(|(key, _)| key == name)
                            .map_or(js::Span::NONE, |&(_, at)| at)
                    };
                    class.fields = fields.into_iter().map(|name| (at(&name), name)).collect();
                }
                Err(why) => refuse(adt, &why),
            }
        }
    }
}

/// The fields `body`, a constructor's, sets of `keys`, in the order it sets
/// them, where a declaration of each, in that order, can't be told from
/// its assignment: each set in one run of `this.key = value;`, the first
/// of the body's statements that reads `this`, which Rust's struct
/// literal is, `Self { .. }`, set before anything can read the instance.
/// Else why not: one it leaves unset, its `None` left out, or a key that
/// can't be declared.
fn declarable(body: &[Stmt], keys: &[String]) -> Result<Vec<String>, String> {
    let start = body.iter().position(|stmt| js::reads_this(std::slice::from_ref(stmt)));
    let mut set: Vec<String> = Vec::new();
    for stmt in body[start.unwrap_or(body.len())..].iter().take(keys.len()) {
        let StmtKind::Assign(target, _) = &stmt.kind else { break };
        let ExprKind::Member(of, name) = &target.kind else {
            break;
        };
        if !matches!(of.kind, ExprKind::This) {
            break;
        }
        set.push(name.clone());
    }
    if let Some(unset) = keys.iter().find(|key| !set.contains(key)) {
        return Err(format!(
            "its constructor leaves `{unset}` unset where it sets the others, its `None` left out, which a declaration would make present, `undefined`"
        ));
    }
    match set.iter().find(|name| !declarable_name(name)) {
        Some(name) => Err(format!("`{name}` is no JS name, which no class declares")),
        None => Ok(set),
    }
}

/// Whether a field named `name` may be declared as it's set: a JS name,
/// `chunkName;` (one named `__proto__` is refused of any struct, rd754,
/// and `constructor` of any class, rd750).
fn declarable_name(name: &str) -> bool {
    let mut chars = name.chars();
    chars
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic() || c == '_' || c == '$')
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '$')
}

/// Why a class's field named `constructor` is refused (rd750): no
/// TypeScript class declares one (TS18006), so its `.d.ts` can't be
/// written, and `this.constructor = ..` hides its prototype's.
pub(super) const NAMED_CONSTRUCTOR: &str = "has no field named `constructor`, which no TypeScript class declares and whose `this.constructor` hides its prototype's: name it otherwise, or give it another JS name, `#[rust_js::name = \"..\"]` (rd750)";

/// Where `class`'s field whose JS name is `constructor` is written, if it
/// has one (rd750).
pub(super) fn named_constructor(tcx: TyCtxt<'_>, class: DefId) -> Option<rustc_span::Span> {
    (tcx.adt_def(class).non_enum_variant().fields.iter())
        .find(|field| crate::lower::bindings::field_key(tcx, field).name() == Some("constructor"))
        .map(|field| tcx.def_span(field.did))
}
