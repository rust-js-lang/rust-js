//! What JS couldn't tell apart of an untagged enum, refused before any of
//! it is lowered (ADR 0214): variants of one kind, a nullish variant that
//! holds something, a variant of more than one value, and a `From` into
//! one that isn't its variant of its argument.

use super::*;

/// Report each untagged enum JS couldn't tell the variants of apart, and
/// each `From` into one that isn't its variant of its argument. False if
/// there was one.
pub(in crate::lower) fn validate<'tcx>(tcx: TyCtxt<'tcx>, foreign: &crate::lower::library::Foreign<'_, 'tcx>) -> bool {
    // Each refusal says it's rust-js's, `rust-js: ..`, as every other
    // does: what reads a compile's errors tells a refusal so.
    let valid = std::cell::Cell::new(true);
    let error = |span, message: String| {
        tcx.dcx().span_err(span, format!("rust-js: {message}"));
        valid.set(false);
    };
    for def_id in tcx.hir_crate_items(()).definitions() {
        if tcx.def_kind(def_id) == DefKind::Enum && bindings::is_untagged(tcx, def_id.to_def_id()) {
            let recognition = Recognition {
                tcx,
                typing_env: ty::TypingEnv::non_body_analysis(tcx, def_id),
                trait_impls: &[],
                foreign,
            };
            let adt = tcx.adt_def(def_id);
            let args = ty::GenericArgs::identity_for_item(tcx, def_id);
            let mut seen: Vec<(Kind, String)> = Vec::new();
            let last = adt.variants().len() - 1;
            for (i, variant) in adt.variants().iter().enumerate() {
                let span = tcx.def_span(variant.def_id);
                // One without fields is its name's string, or `undefined`
                // or `null` (ADR 0676).
                if let Some(kind) = unit_kind(tcx, variant) {
                    if let Some((_, other)) = seen.iter().find(|(k, _)| k.overlaps(&kind))
                        && tested_by_impls(tcx, def_id.to_def_id(), &recognition)
                    {
                        error(
                            span,
                            format!(
                                "`{}` and `{other}` both hold {}, which JS can't tell apart in an untagged enum (ADR 0214)",
                                variant.name,
                                kind.describe()
                            ),
                        );
                    }
                    seen.push((kind, variant.name.to_string()));
                    continue;
                }
                // `undefined` and `null` hold nothing.
                if let Some(nullish) = bindings::given_nullish(tcx, variant.def_id) {
                    let marked = format!("{nullish:?}").to_lowercase();
                    error(
                        span,
                        format!(
                            "an untagged enum's `#[rust_js::{marked}]` variant, `{}`, is `{marked}`, and holds nothing, as `js::Kind::{nullish:?}` does (ADR 0676)",
                            variant.name
                        ),
                    );
                    continue;
                }
                let ([field], Some(CtorKind::Fn)) = (&variant.fields.raw[..], variant.ctor_kind()) else {
                    error(
                        span,
                        format!(
                            "an untagged enum's variant holds one value, as `{}(&'a str)` does, or none, its name's string (ADR 0214)",
                            variant.name
                        ),
                    );
                    continue;
                };
                // What the others aren't holds anything, and comes last; but
                // a payload of a kind JS tells, a tuple struct's array, is
                // taken by another's test of that kind first (#1083).
                if bindings::is_otherwise(tcx, variant.def_id) {
                    if i != last {
                        error(
                            span,
                            format!(
                                "an untagged enum's `#[rust_js::otherwise]` variant, `{}`, is its last (ADR 0214)",
                                variant.name
                            ),
                        );
                    }
                    if let Some(kind) = recognition.untagged_variant_kind(variant, args)
                        && let Some((_, other)) = seen.iter().find(|(k, _)| k.overlaps(&kind))
                        && tested_by_impls(tcx, def_id.to_def_id(), &recognition)
                    {
                        error(
                            span,
                            format!(
                                "`{}` and `{other}` both hold {}, which JS can't tell apart in an untagged enum (ADR 0214)",
                                variant.name,
                                kind.describe()
                            ),
                        );
                    }
                    continue;
                }
                let ty = field.ty(tcx, args).skip_normalization();
                let Some(kind) = recognition.untagged_kind(ty) else {
                    error(
                        span,
                        format!(
                            "JS can't tell a `{ty}` from another variant's value: an untagged enum's variant holds a string, a number, a `bool`, an array, a JS object, a function or a struct (ADR 0214)"
                        ),
                    );
                    continue;
                };
                // Two variants of one kind are told apart nowhere but where
                // one is tested: an enum only made, `getStaticProps`'s `{ props }`
                // or `{ notFound }`, is its payloads. An impl or a drop that
                // tests them is refused here.
                if let Some((_, other)) = seen.iter().find(|(k, _)| k.overlaps(&kind))
                    && tested_by_impls(tcx, def_id.to_def_id(), &recognition)
                {
                    error(
                        span,
                        format!(
                            "`{}` and `{other}` both hold {}, which JS can't tell apart in an untagged enum (ADR 0214)",
                            variant.name,
                            kind.describe()
                        ),
                    );
                }
                seen.push((kind, variant.name.to_string()));
            }
            // A variant told apart by a member is told by its kind (rd291).
            for variant in adt.variants() {
                let Some(Kind::Member(key, told)) = recognition.untagged_variant_kind(variant, args) else {
                    continue;
                };
                if matches!(*told, Kind::Object | Kind::Member(..) | Kind::Null | Kind::Undefined) {
                    error(
                        tcx.def_span(variant.def_id),
                        format!(
                            "an untagged enum's `{}` is told apart by its `{key}`, `#[rust_js::test]`, which holds a string, a number, a `bool`, an array or a class (rd291)",
                            variant.name
                        ),
                    );
                }
            }
        }
        // A member that tells a JS object type, `test(nodeType = 9)`: one
        // key, each of its values a number (#555).
        if tcx.def_kind(def_id) == DefKind::Struct && bindings::test_member(tcx, def_id.to_def_id()) == Some(None) {
            error(
                tcx.def_span(def_id),
                "a JS object type's `#[rust_js::test(..)]` names the member that tells it apart and each number \
                 it may be, `#[rust_js::test(nodeType = 3, nodeType = 4)]` (ADR 1062, amended)"
                    .to_string(),
            );
        }
        // serde's codecs read and write a tagged enum (ADR 0079): one without
        // a tag is serde's own `#[serde(untagged)]`, which is to come.
        if serde_impl(tcx, def_id.to_def_id()).is_some()
            && let Some((adt, _)) = untagged_adt(tcx, tcx.type_of(def_id).instantiate_identity().skip_normalization())
        {
            let what = format!("serde of the untagged enum `{}`", tcx.item_name(adt.did()));
            let refused = crate::lower::refusal::Refused::Because(&what, "see ADR 0214");
            tcx.dcx()
                .span_err(tcx.def_span(def_id), crate::lower::refusal::words(tcx, refused));
            valid.set(false);
        }
    }
    valid.get()
}
