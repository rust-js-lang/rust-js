//! How much of std's data structures rust-js lowers (ADR 0314): each stable
//! inherent method of each, and whether recognition knows it, for the
//! ratchet in `test/std-coverage.test.ts`, which no method may leave.

use std::fmt::Write as _;

use rustc_middle::ty::{self, TyCtxt};

use super::recognition::{Recognition, registry};

/// Each type's stable methods, `+ Vec::push` where rust-js knows it and
/// `- Vec::shrink_to` where it doesn't, under `# Vec 35 of 160`.
pub fn report<'tcx>(tcx: TyCtxt<'tcx>, dependencies: &crate::library::Dependencies) -> String {
    let foreign = super::library::Foreign::new(tcx, dependencies);
    let mut out = String::new();
    for (name, impls) in registry::data_structures(tcx) {
        let mut methods: Vec<(String, bool)> = Vec::new();
        for imp in impls {
            for item in tcx.associated_items(imp).in_definition_order() {
                let def_id = item.def_id;
                if !matches!(item.kind, ty::AssocKind::Fn { .. })
                    || tcx.lookup_stability(def_id).is_none_or(|s| s.is_unstable())
                    || tcx.is_doc_hidden(def_id)
                {
                    continue;
                }
                let recognition = Recognition {
                    tcx,
                    typing_env: ty::TypingEnv::non_body_analysis(tcx, def_id),
                    trait_impls: &[],
                    foreign: &foreign,
                };
                let args = ty::GenericArgs::identity_for_item(tcx, def_id);
                // One known for some of its own type arguments counts: a `str`'s
                // `split` of a `&str` or a `char` pattern, its patterns rust-js takes,
                // and a slice's `get` of an index.
                let parent_count = tcx.generics_of(def_id).parent_count;
                let given = |pattern: ty::Ty<'tcx>| {
                    ty::GenericArgs::for_item(tcx, def_id, |param, _| match param.kind {
                        ty::GenericParamDefKind::Type { .. } if param.index as usize >= parent_count => pattern.into(),
                        _ => args[param.index as usize],
                    })
                };
                let known = recognition.classify(def_id, args).is_some()
                    || [ty::Ty::new_static_str(tcx), tcx.types.char, tcx.types.usize]
                        .into_iter()
                        .any(|pattern| recognition.classify(def_id, given(pattern)).is_some());
                let method = format!("{name}::{}", tcx.item_name(def_id));
                match methods.iter_mut().find(|(m, _)| *m == method) {
                    Some((_, k)) => *k |= known,
                    None => methods.push((method, known)),
                }
            }
        }
        methods.sort();
        let known = methods.iter().filter(|(_, k)| *k).count();
        let _ = writeln!(out, "# {name} {known} of {}", methods.len());
        for (method, known) in methods {
            let _ = writeln!(out, "{} {method}", if known { "+" } else { "-" });
        }
    }
    out
}
