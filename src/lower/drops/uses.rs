//! What a body's drops use: the drop a call's argument is given, its owned
//! parameters, the drops it may not have, and the arguments it keeps.

use super::*;

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// The drop function for a `ty` that `callee`, which takes only the
    /// drops it uses, is given for its type parameter `index`: an argument
    /// the pipeline keeps where `callee` uses it (ADR 0300). The drops of
    /// this function's that it's made of are passed on, not used, where
    /// this function takes only those it uses too.
    pub(in crate::lower) fn drop_argument(
        &mut self,
        callee: DefId,
        index: u32,
        (ty, iterated): (Ty<'tcx>, bool),
        span: Span,
    ) -> R<Expr> {
        let before = std::mem::take(&mut self.drop_state.used_drops);
        let drop = self.held_drop_function(ty, iterated, span);
        let through = std::mem::replace(&mut self.drop_state.used_drops, before);
        if !self.in_copied_default() && self.krate.takes_used_drops.contains(self.item) {
            let mut observed = self.drops_observed.borrow_mut();
            observed
                .passed
                .extend(through.into_iter().map(|param| ((self.item, param), (callee, index))));
        } else {
            self.drop_state.used_drops.extend(through);
        }
        let drop = drop?.unwrap_or_else(Expr::undefined);
        Ok(Expr::drop_argument(callee.index.as_u32(), index, drop))
    }

    /// Note which drops `id`, which takes only those it uses, used, and the
    /// names it took them by (ADR 0300).
    pub(in crate::lower) fn note_drop_uses(&self, id: DefId) {
        if !self.krate.takes_used_drops.contains(id) {
            return;
        }
        let mut observed = self.drops_observed.borrow_mut();
        observed
            .used
            .extend(self.drop_state.used_drops.iter().map(|&param| (id, param)));
        let names = self
            .drop_state
            .param_drops
            .iter()
            .map(|(&index, name)| (index, name.clone()));
        observed.names.insert(id, names.collect());
    }
}

/// The type parameters a value of `ty` holds, not behind a reference: what
/// it drops of a caller's (ADR 0190).
pub(super) fn owned_params<'tcx>(tcx: TyCtxt<'tcx>, ty: Ty<'tcx>) -> Vec<u32> {
    let mut params = Vec::new();
    let mut walker = ty.walk();
    while let Some(part) = walker.next() {
        let Some(part) = part.as_type() else { continue };
        match part.kind() {
            // Behind a reference, or a pointer, a `NonNull`'s too (ADR 0407).
            ty::Ref(..) => walker.skip_current_subtree(),
            _ if crate::lower::pointers::raw_pointee(tcx, part).is_some() => walker.skip_current_subtree(),
            ty::Param(param) => params.push(param.index),
            _ => {}
        }
    }
    params
}

/// The functions given drops that take only those they use (ADR 0300):
/// what the crate knows before it lowers any, which lowering asks.
pub(in crate::lower) struct TakesUsed(HashSet<DefId>);

impl TakesUsed {
    /// Each function given drops that's called where it's named, whose
    /// callers are all seen or told by its manifest: not a trait's method,
    /// which a dictionary's callers call, nor an impl's.
    pub(in crate::lower) fn new(tcx: TyCtxt<'_>, drop_params: &HashMap<DefId, Vec<u32>>) -> TakesUsed {
        TakesUsed(
            drop_params
                .keys()
                .copied()
                .filter(|&id| {
                    id.is_local()
                        && tcx.trait_of_assoc(id).is_none()
                        && tcx.trait_impl_of_assoc(id).is_none()
                        && !matches!(tcx.def_kind(id), DefKind::Impl { .. })
                })
                .collect(),
        )
    }

    pub(in crate::lower) fn contains(&self, id: DefId) -> bool {
        self.0.contains(&id)
    }
}

/// What lowering a function found of drops (ADRs 0190, 0300), returned
/// with it, lowered or not, and gathered by the pipeline, which resolves
/// them once all are: the type parameters each takes no destructor of, the
/// calls of the crate's functions to check, and of those that take only
/// the drops they use, those they used, those they passed on as another's,
/// and the names they took them by.
#[derive(Default)]
pub(in crate::lower) struct DropsObserved {
    pub(in crate::lower) no_drops: HashMap<DefId, BTreeSet<u32>>,
    pub(in crate::lower) checks: Vec<DropCheck>,
    used: HashSet<(DefId, u32)>,
    passed: Vec<((DefId, u32), (DefId, u32))>,
    names: HashMap<DefId, Vec<(u32, String)>>,
}

impl DropsObserved {
    /// `other`'s too, a function's after those before it, in the order
    /// they're lowered.
    pub(in crate::lower) fn gather(&mut self, other: DropsObserved) {
        for (id, params) in other.no_drops {
            self.no_drops.entry(id).or_default().extend(params);
        }
        self.checks.extend(other.checks);
        self.used.extend(other.used);
        self.passed.extend(other.passed);
        self.names.extend(other.names);
    }

    /// The drops each function keeps, those it uses or passes on to one
    /// that keeps it, in `drop_params`; and the lowered functions with only
    /// those, as parameters and arguments.
    pub(in crate::lower) fn keep(
        self,
        takes_used: &TakesUsed,
        drop_params: &mut HashMap<DefId, Vec<u32>>,
        items: &mut [(DefId, crate::lower::LoweredFn)],
    ) {
        let mut kept = self.used;
        let mut changed = true;
        while changed {
            changed = false;
            for &(from, to) in &self.passed {
                if kept.contains(&to) && kept.insert(from) {
                    changed = true;
                }
            }
        }
        for id in &takes_used.0 {
            if let Some(indices) = drop_params.get_mut(id) {
                indices.retain(|&index| kept.contains(&(*id, index)));
            }
        }
        for (id, item) in items {
            if let Some(names) = self.names.get(id) {
                let unused: HashSet<&str> = names
                    .iter()
                    .filter(|&&(index, _)| !kept.contains(&(*id, index)))
                    .map(|(_, name)| name.as_str())
                    .collect();
                item.function
                    .params
                    .retain(|param| !matches!(param, js::Pattern::Name(name) if unused.contains(name.as_str())));
            }
            js::each_expr_mut(&mut item.function.body, &mut |e| keep_arguments(e, &kept));
        }
    }
}

/// `e` with only the drop arguments kept: a call's, and the call an arrow
/// that only passes its parameters on is, which is then its callee,
/// `(arg0) => keep(arg0)` being `keep`.
fn keep_arguments(e: &mut Expr, kept: &HashSet<(DefId, u32)>) {
    let local = |item: u32| DefId {
        krate: rustc_span::def_id::LOCAL_CRATE,
        index: rustc_span::def_id::DefIndex::from_u32(item),
    };
    // One left last, `undefined`, is no argument: prepare leaves it out.
    let keep = |args: &mut Vec<Expr>| {
        *args = std::mem::take(args)
            .into_iter()
            .filter_map(|arg| match arg.kind {
                js::ExprKind::DropArgument(item, index, drop) => kept.contains(&(local(item), index)).then_some(*drop),
                _ => Some(arg),
            })
            .collect();
    };
    let has_drops = |args: &[Expr]| args.iter().any(|a| matches!(a.kind, js::ExprKind::DropArgument(..)));
    let callee = match &mut e.kind {
        js::ExprKind::Arrow(params, body) => {
            let [
                Stmt {
                    kind: StmtKind::Return(Some(call)),
                    ..
                },
            ] = body.as_mut_slice()
            else {
                return;
            };
            let js::ExprKind::Call(callee, args) = &mut call.kind else {
                return;
            };
            if !has_drops(args) {
                return;
            }
            keep(args);
            let names: Vec<&str> = params
                .iter()
                .filter_map(|param| match param {
                    js::Pattern::Name(name) => Some(name.as_str()),
                    _ => None,
                })
                .collect();
            let passed = names.len() == params.len()
                && names.len() == args.len()
                && names
                    .iter()
                    .zip(args.iter())
                    .all(|(name, arg)| matches!(&arg.kind, js::ExprKind::Var(a) if a == name))
                && match &callee.kind {
                    js::ExprKind::Var(name) => !names.contains(&name.as_str()),
                    js::ExprKind::Symbol(_) => true,
                    _ => false,
                };
            passed.then(|| (**callee).clone())
        }
        js::ExprKind::Call(_, args) | js::ExprKind::OptionalCall(_, args) | js::ExprKind::New(_, args)
            if has_drops(args) =>
        {
            keep(args);
            None
        }
        _ => None,
    };
    if let Some(callee) = callee {
        *e = callee;
    }
}

/// The name of `callee`'s type parameter `index`, `I`.
pub(super) fn param_name(tcx: TyCtxt<'_>, callee: DefId, index: u32) -> String {
    tcx.generics_of(callee).param_at(index as usize, tcx).name.to_string()
}

/// `impl.$drop?.(value)`: a `dyn`'s value dropped through its dictionary,
/// if what it holds has a drop.
pub(super) fn dyn_dropped(dictionary: Expr, value: Expr, js_span: js::Span) -> Stmt {
    let drop = Expr::member(dictionary, js::DICTIONARY_DROP);
    StmtKind::Expr(Expr {
        known: js::Known::Nothing,
        kind: js::ExprKind::OptionalCall(Box::new(drop), vec![value]),
        span: js_span,
    })
    .at(js_span)
}
