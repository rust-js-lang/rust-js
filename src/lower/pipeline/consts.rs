//! What each of the crate's constants and statics is: its JS constant, of
//! rustc's value, in its module, its type's object or the function it's
//! written in; or its initializer, lowered as code where one JS value can
//! be every use's (ADRs 0031, 0096).

use crate::collections::{IndexMap, IndexSet};
use crate::js::{self, Expr, Prop};
use crate::lower::analysis::{LocalConsts, is_thread_local};
use crate::lower::{Body, FnInfo, const_js, static_value};
use rustc_hir::def::DefKind;
use rustc_middle::ty::{self, TyCtxt};
use rustc_span::def_id::{DefId, LocalDefId, LocalModId};

/// What `lower_consts` makes of the crate's constants and statics.
pub(super) struct Consts<'b, 'tcx> {
    /// Each module's constants.
    pub(super) items: IndexMap<LocalModId, Vec<js::Const>>,
    /// Each inherent impl's constant, its type's object's, by its item: a
    /// module's constant may have its name, `pub(crate) const MIN` beside
    /// chrono's `impl TimeDelta { pub const MIN: Self = MIN; }` (#513).
    pub(super) owned: IndexMap<DefId, js::Const>,
    /// Each `const` written in a function's body, its function's (ADR 0031,
    /// amended).
    pub(super) local_holes: Vec<(DefId, js::Const)>,
    /// The statics and constants whose initializers are lowered as code.
    pub(super) initialized: Vec<&'b Body<'tcx>>,
    /// Whether one was refused.
    pub(super) failed: bool,
}

/// What the crate's constants and statics, `consts`, are, of their
/// `initializers`, as `Consts` holds them.
pub(super) fn lower_consts<'b, 'tcx>(
    tcx: TyCtxt<'tcx>,
    (consts, initializers): (&[LocalDefId], &'b [Body<'tcx>]),
    (fns, foreign): (&IndexMap<DefId, FnInfo>, &crate::lower::library::Foreign<'_, 'tcx>),
    (flattened, sources): (
        &crate::lower::bindings::Flattened,
        &crate::lower::sources::CapturedSources,
    ),
    (thread_local_inits, local_consts): (&IndexMap<LocalDefId, LocalDefId>, &LocalConsts),
    called_from_elsewhere: &IndexSet<DefId>,
) -> Consts<'b, 'tcx> {
    let mut const_items: IndexMap<LocalModId, Vec<js::Const>> = IndexMap::default();
    let mut owned_consts: IndexMap<DefId, js::Const> = IndexMap::default();
    let mut local_const_holes: Vec<(DefId, js::Const)> = Vec::new();
    let mut initialized: Vec<&'b Body<'tcx>> = Vec::new();
    let mut failed = false;
    for &def_id in consts.iter().filter(|&&d| !is_thread_local(tcx, d)) {
        let span = tcx.def_span(def_id);
        let typing_env = ty::TypingEnv::fully_monomorphized();
        let args = ty::GenericArgs::identity_for_item(tcx, def_id);
        let initializer = initializers.iter().find(|body| body.def_id == def_id.to_def_id());
        // A `const { .. }` block's type is its value's.
        let ty = match (tcx.def_kind(def_id), initializer) {
            (DefKind::AnonConst, Some(body)) => body.thir[body.expr].ty,
            _ => tcx.type_of(def_id).instantiate_identity().skip_normalization(),
        };
        // Its initializer, lowered as code, where one JS value can be every
        // use's, and it reads no static, whose value may not be made yet.
        // A `OnceLock`, only ever made empty, or a `LazyLock`, which
        // makes its value when it's used, is one JS cell too (ADRs 0317, 0318),
        // and so is a `Mutex`, an `RwLock` or an atomic, which every use
        // changes in place, the one place a static is, and a `static mut`,
        // its value's `{ value }` (ADR 0096).
        let mutable = matches!(tcx.def_kind(def_id), DefKind::Static { mutability, .. } if mutability.is_mut());
        let once = matches!(tcx.def_kind(def_id), DefKind::Static { .. })
            && matches!(ty.kind(), ty::Adt(adt, _)
            if [
                crate::lower::recognition::StdItem::OnceCell,
                crate::lower::recognition::StdItem::LazyCell,
                crate::lower::recognition::StdItem::Mutex,
                crate::lower::recognition::StdItem::RwLock,
                crate::lower::recognition::StdItem::Atomic,
            ]
                .into_iter()
                .any(|item| crate::lower::recognition::is_std_def(tcx, adt.did(), item)));
        // A constant's each use is a value of its own, copied where it may
        // change (`named_const`), so its one JS value needn't be shareable:
        // `const EMPTY: Vec<i32> = Vec::new()`'s `[]` (ADR 0031).
        let constant = !matches!(tcx.def_kind(def_id), DefKind::Static { .. });
        let as_code = initializer.filter(|body| {
            (mutable || constant || crate::lower::copies::shareable(tcx, ty) || once)
                && !crate::lower::body_queries::reads_statics(tcx, &body.thir, tcx.parent_module_from_def_id(def_id))
        });
        // One written of other constants read by their names, `A | B`, is
        // its initializer's, as it's written, not rustc's value of it, which
        // would lose the names, and what a library's flag says where it's
        // swapped for another (ADR 0031, amended), or of `js::null()`, a call
        // written as the `null` it gives, where rustc's `None` would be left
        // out (ADR 0935, amended).
        let by_name = |id: DefId| fns.contains_key(&id) || foreign.item(id).is_some();
        let in_place = |id: DefId| crate::lower::bindings::is_js_null(tcx, id);
        if let Some(body) = as_code
            && crate::lower::body_queries::written_of_constants(&body.thir, &by_name, &in_place)
        {
            initialized.push(body);
            continue;
        }
        // A `static mut` is its value's `{ value }` (ADR 0096), which any
        // module can write.
        let (value, what) = match tcx.def_kind(def_id) {
            DefKind::Static { mutability, .. } => (
                static_value(tcx, def_id)
                    .and_then(|v| const_js(tcx, &flattened.keyed, v))
                    .map(|v| crate::lower::literals::const_string(tcx, def_id, v, &|span| sources.span(span)))
                    .map(|v| crate::lower::aggregates::const_object(tcx, def_id, v))
                    .map(|v| match mutability.is_mut() {
                        true => Expr::object(vec![Prop::Field(js::HELD.into(), v)]),
                        false => v,
                    }),
                "statics",
            ),
            // One written as `Math`'s constant is its, `Math.LN2` (ADR 0887).
            _ => (
                initializer
                    .and_then(|body| {
                        crate::lower::std_types::number::math::written_constant(tcx, &body.thir, body.expr)
                    })
                    .or_else(|| {
                        crate::lower::representation::const_value_js(
                            tcx,
                            &flattened.keyed,
                            typing_env,
                            def_id.to_def_id(),
                            args,
                            span,
                        )
                        .map(|v| crate::lower::literals::const_string(tcx, def_id, v, &|span| sources.span(span)))
                        .map(|v| crate::lower::aggregates::const_object(tcx, def_id, v))
                    }),
                "constants",
            ),
        };
        let Some(value) = value else {
            // A value rustc's can't say, a function's, a `dyn`'s or one too
            // large for a value tree, is its initializer's.
            if let Some(body) = as_code {
                initialized.push(body);
                continue;
            }
            crate::lower::refusal::refuse(tcx, span, crate::lower::refusal::Refused::Type(what, ty));
            failed = true;
            continue;
        };
        // Where its literal is, which it's written as (ADR 0627), an array's
        // items each where it's written (rd344a).
        let value = crate::lower::literals::const_placed(tcx, def_id, value, &|span| sources.span(span));
        // A `const { .. }` thread-local's value, the `const` in its block, is its variable, by its name.
        let key = thread_local_inits.get(&def_id).copied();
        let named = key.unwrap_or(def_id);
        // A nullable one's `None` `null`: a `const`'s, or one's of the `Option`, `const { None }` (ADR 0586).
        use crate::lower::representation::{NoneForm, NonePlace, none_form};
        let nullable =
            |id: rustc_span::def_id::LocalDefId| none_form(tcx, NonePlace::Local(id.to_def_id())) == NoneForm::Null;
        let form = match key {
            None => none_form(tcx, NonePlace::Local(def_id.to_def_id())),
            Some(key) => none_form(tcx, NonePlace::LocalOption(key.to_def_id())),
        };
        let value = match key {
            _ if form == NoneForm::Null => form.storage().written(value, false),
            Some(key) if nullable(key) => crate::lower::std_types::cell::nullable_made(value),
            _ => value,
        };
        let info = &fns[&named.to_def_id()];
        let constant = js::Const {
            name: info.name.clone(),
            value,
            mutable: false,
            fields: js::Fields::default(),
            export: tcx.visibility(named).is_public() || called_from_elsewhere.contains(&named.to_def_id()),
            span: sources.item_span(tcx, named),
        };
        // One written in a function's body, where it's written there.
        match local_consts.contains_key(&named.to_def_id()) {
            true => local_const_holes.push((named.to_def_id(), constant)),
            false if info.owner.is_some() => _ = owned_consts.insert(named.to_def_id(), constant),
            false => const_items.entry(info.module).or_default().push(constant),
        }
    }
    Consts {
        items: const_items,
        owned: owned_consts,
        local_holes: local_const_holes,
        initialized,
        failed,
    }
}

/// Each constant placed once the crate's bodies are lowered: those a body
/// made, its module's; a numbered enum's variant, its module's `const` of
/// its number, where something reads it or another module or crate may,
/// exported then, as a `const` item is (ADR 0722, amended); and an
/// inherent impl's, its type's object's, `S.ID`, as its methods are (ADR
/// 0410).
pub(super) fn place_consts(
    tcx: TyCtxt<'_>,
    pass: &mut super::Pass,
    (const_items, owned_consts): (
        &mut IndexMap<LocalModId, Vec<js::Const>>,
        &mut IndexMap<DefId, js::Const>,
    ),
    (consts, variants): (&[LocalDefId], &[LocalDefId]),
    fns: &IndexMap<DefId, FnInfo>,
    (called_from_elsewhere, reached): (&IndexSet<DefId>, &IndexSet<DefId>),
    sources: &crate::lower::sources::CapturedSources,
) {
    for (module, consts) in pass.local_consts.drain(..) {
        const_items.entry(module).or_default().extend(consts);
    }
    // A numbered enum's variant, its module's `const` of its number, where
    // something reads it or another module or crate may, exported then, as
    // a `const` item is (ADR 0722, amended).
    let read_elsewhere: IndexSet<DefId> = pass.references.iter().map(|&(_, id)| id).collect();
    for &variant in variants {
        let id = variant.to_def_id();
        let export =
            tcx.visibility(id).is_public() || called_from_elsewhere.contains(&id) || read_elsewhere.contains(&id);
        if !export && !reached.contains(&id) {
            continue;
        }
        let info = &fns[&id];
        let adt = tcx.adt_def(tcx.parent(id));
        const_items.entry(info.module).or_default().push(js::Const {
            name: info.name.clone(),
            value: crate::lower::bindings::variant_tag(tcx, adt.variant_with_id(id)),
            mutable: false,
            fields: js::Fields::default(),
            export,
            span: sources.item_span(tcx, variant),
        });
    }
    // An inherent impl's constant is its type's object's, `S.ID`, as its
    // methods are (ADR 0410).
    for &def_id in consts {
        let info = &fns[&def_id.to_def_id()];
        let Some(owner) = &info.owner else { continue };
        let Some(constant) = owned_consts.swap_remove(&def_id.to_def_id()) else {
            continue;
        };
        let namespace = js::Namespace::of(pass.namespaces.entry(info.module).or_default(), owner);
        namespace.export |= constant.export;
        namespace.constants.push(constant);
    }
}
