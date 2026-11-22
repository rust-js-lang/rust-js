//! What a test proved defined where lowering reads it, never `undefined`
//! though maybe `null` (`Presence::Defined`, #1933 step 4 (D)): of
//! `js::classify(local)`, a switch whose edge isn't `Kind::Undefined`'s,
//! `!matches!(js::classify(x), Kind::Undefined)`'s true one; and of
//! `js::strict_equals(local, None)`, JS's `x === undefined`, its false
//! edge. Split by concept from `nulled`, whose `proved_by` it asks, as
//! `proved_present` does: every way to the read takes the edge, and
//! nothing writes the local since the test. Not `Present`: a `null` there
//! is a `None`, which an `unwrap()` mustn't take for a value.

use super::*;

impl<'mcx, 'a, 'tcx> MirCx<'mcx, 'a, 'tcx> {
    /// Whether `local` is proved defined there where the block lowered now
    /// runs (`proved_by`): a read of it there is `Presence::Defined`, so
    /// a nullable place stores it as it is, no `?? null`.
    pub(crate) fn proved_defined(&self, state: &State<'_, 'tcx>, local: Local) -> bool {
        self.proved_by(state, local, Self::defined_edge, state.at)
    }

    /// The edge of a switch on `flag` that `local` is defined on.
    fn defined_edge(
        &self,
        state: &State<'_, 'tcx>,
        flag: Local,
        targets: &mir::SwitchTargets,
        local: Local,
    ) -> Option<BasicBlock> {
        let values: Vec<(u128, BasicBlock)> = targets.iter().collect();
        match self.whole_writes(state, flag)?.as_slice() {
            // `js::strict_equals(local, None)`: its false edge.
            [Write::Call(_, func, [a, b])] => {
                let [(0, no)] = values.as_slice() else {
                    return None;
                };
                let (callee, _) = func.const_fn_def()?;
                if !rust_js_lower::recognition::is_js_strict_equals(self.tcx, callee) {
                    return None;
                }
                let tested = |side: &Operand<'tcx>| self.tested_local(state, side, Some(local)) == Some(local);
                let none = |side: &Operand<'tcx>| self.made_none(state, side);
                ((tested(&a.node) && none(&b.node)) || (none(&a.node) && tested(&b.node))).then_some(*no)
            }
            // `discriminant(k)` of `k = js::classify(local)`: the edge of
            // each value but those the switch names, `otherwise`, where
            // `Kind::Undefined`'s is one of them, of `Kind::Undefined |
            // Kind::Null` too, and none of them goes there.
            [Write::Assign(Rvalue::Discriminant(of))] if of.projection.is_empty() => {
                let kind = self.classified(state, of.local, local)?;
                let undefined = (kind.variants().iter_enumerated())
                    .find(|(_, v)| rust_js_lower::untagged::is_undefined(self.tcx, v))?
                    .0;
                let value = kind.discriminant_for_variant(self.tcx, undefined).val;
                let otherwise = targets.otherwise();
                (values.iter().any(|&(v, _)| v == value) && values.iter().all(|&(_, to)| to != otherwise))
                    .then_some(otherwise)
            }
            _ => None,
        }
    }

    /// The `js::Kind` enum `k` is, where `k = js::classify(local)`, its one
    /// write: a switch on `discriminant(k)` tests what `local` is, each
    /// edge of the variants that reach it (ADR 0668).
    pub(super) fn classified(&self, state: &State<'_, 'tcx>, k: Local, local: Local) -> Option<ty::AdtDef<'tcx>> {
        let made = self.whole_writes(state, k)?;
        let [Write::Call(_, func, [given])] = made.as_slice() else {
            return None;
        };
        let (callee, _) = func.const_fn_def()?;
        if !rust_js_lower::recognition::is_js_classify(self.tcx, callee)
            || self.tested_local(state, &given.node, Some(local))? != local
        {
            return None;
        }
        let ty::Adt(kind, _) = state.body.local_decls[k].ty.kind() else {
            return None;
        };
        Some(*kind)
    }

    /// The edge of a switch on `discriminant(k)`, `k = js::classify(local)`,
    /// that only values neither `undefined` nor `null` take: a variant's
    /// that isn't `Kind::Undefined` or `Kind::Null` (`untagged::is_nullish`),
    /// `if let Kind::Function(_) = js::classify(exports)`'s, no other value
    /// going there; or `otherwise`, where both of those are named and go
    /// elsewhere. Of several, the one that leads to the block lowered now.
    pub(super) fn classified_present_edge(
        &self,
        state: &State<'_, 'tcx>,
        k: Local,
        targets: &mir::SwitchTargets,
        local: Local,
    ) -> Option<BasicBlock> {
        let kind = self.classified(state, k, local)?;
        let values: Vec<(u128, BasicBlock)> = targets.iter().collect();
        let nullish: Vec<u128> = (kind.variants().iter_enumerated())
            .filter(|(_, v)| rust_js_lower::untagged::is_nullish(self.tcx, v))
            .map(|(at, _)| kind.discriminant_for_variant(self.tcx, at).val)
            .collect();
        let otherwise = targets.otherwise();
        // Each block a value goes to, but a nullish one's or `otherwise`'s.
        let mut edges: Vec<BasicBlock> = (values.iter())
            .map(|&(_, to)| to)
            .filter(|&to| to != otherwise && values.iter().all(|&(v, at)| at != to || !nullish.contains(&v)))
            .collect();
        let named = |v: u128| values.iter().any(|&(at, _)| at == v);
        if nullish.len() == 2 && nullish.iter().all(|&v| named(v)) && values.iter().all(|&(_, to)| to != otherwise) {
            edges.push(otherwise);
        }
        let dominators = state.body.basic_blocks.dominators();
        (edges.iter().copied())
            .find(|&edge| dominators.dominates(edge, state.at))
            .or_else(|| edges.first().copied())
    }

    /// Whether `side`, a test's operand, is a `None` made for it: a local
    /// written once, `Option::None`, through the copies and references
    /// `tested_local` follows.
    fn made_none(&self, state: &State<'_, 'tcx>, side: &Operand<'tcx>) -> bool {
        let Some(held) = self.tested_local(state, side, None) else {
            return false;
        };
        matches!(self.whole_writes(state, held).as_deref(),
            Some([Write::Assign(Rvalue::Aggregate(kind, _))])
                if matches!(**kind, AggregateKind::Adt(did, variant, ..)
                    if variant.as_u32() == 0 && self.tcx.is_lang_item(did, LangItem::Option)))
    }
}
