//! What a test proved defined where lowering reads it, never `undefined`
//! though maybe `null` (`Known::ProvedDefined`, #1933 step 4 (D)): of
//! `js::classify(local)`, a switch whose edge isn't `Kind::Undefined`'s,
//! `!matches!(js::classify(x), Kind::Undefined)`'s true one; and of
//! `js::strict_equals(local, None)`, JS's `x === undefined`, its false
//! edge. Split by concept from `nulled`, whose `proved_by` it asks, as
//! `proved_present` does: every way to the read takes the edge, and
//! nothing writes the local since the test. Not `Proved`: a `null` there
//! is a `None`, which an `unwrap()` mustn't take for a value.

use super::*;

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// Whether `local` is proved defined there where the block lowered now
    /// runs (`proved_by`): a read of it there is `Known::ProvedDefined`, so
    /// a nullable place stores it as it is, no `?? null`.
    pub(in crate::lower::mir) fn proved_defined(&self, state: &State<'_, 'tcx>, local: Local) -> bool {
        self.proved_by(state, local, Self::defined_edge)
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
                if !crate::lower::recognition::is_js_strict_equals(self.tcx, callee) {
                    return None;
                }
                let tested = |side: &Operand<'tcx>| self.tested_local(state, side) == Some(local);
                let none = |side: &Operand<'tcx>| self.made_none(state, side);
                ((tested(&a.node) && none(&b.node)) || (none(&a.node) && tested(&b.node))).then_some(*no)
            }
            // `discriminant(k)` of `k = js::classify(local)`: the edge of
            // each value but those the switch names, `otherwise`, where
            // `Kind::Undefined`'s is one of them, of `Kind::Undefined |
            // Kind::Null` too, and none of them goes there.
            [Write::Assign(Rvalue::Discriminant(of))] if of.projection.is_empty() => {
                let made = self.whole_writes(state, of.local)?;
                let [Write::Call(_, func, [given])] = made.as_slice() else {
                    return None;
                };
                let (callee, _) = func.const_fn_def()?;
                if !crate::lower::recognition::is_js_classify(self.tcx, callee)
                    || self.tested_local(state, &given.node)? != local
                {
                    return None;
                }
                let ty::Adt(kind, _) = state.body.local_decls[of.local].ty.kind() else {
                    return None;
                };
                let undefined = (kind.variants().iter_enumerated())
                    .find(|(_, v)| crate::lower::untagged::is_undefined(self.tcx, v))?
                    .0;
                let value = kind.discriminant_for_variant(self.tcx, undefined).val;
                let otherwise = targets.otherwise();
                (values.iter().any(|&(v, _)| v == value) && values.iter().all(|&(_, to)| to != otherwise))
                    .then_some(otherwise)
            }
            _ => None,
        }
    }

    /// Whether `side`, a test's operand, is a `None` made for it: a local
    /// written once, `Option::None`, through the copies and references
    /// `tested_local` follows.
    fn made_none(&self, state: &State<'_, 'tcx>, side: &Operand<'tcx>) -> bool {
        let Some(held) = self.tested_local(state, side) else {
            return false;
        };
        matches!(self.whole_writes(state, held).as_deref(),
            Some([Write::Assign(Rvalue::Aggregate(kind, _))])
                if matches!(**kind, AggregateKind::Adt(did, variant, ..)
                    if variant.as_u32() == 0 && self.tcx.is_lang_item(did, LangItem::Option)))
    }
}
