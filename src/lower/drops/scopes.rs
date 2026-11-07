//! A body's drops entered and left, hoisted, and the drop slots a call
//! is given, lent, returned and swapped.

use super::*;

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// Start lowering a body inside the one being lowered: its own facts, and
    /// none of the enclosing one's statement and scopes. A copied default's,
    /// `default`, is given its trait's type parameters' drops and those it
    /// can't make (ADR 0098). It's a JS function of its own, a closure's or
    /// a copied default's, which declares its own drop functions.
    pub(in crate::lower) fn enter_body_drops(
        &mut self,
        default: Option<DefaultDrops<'tcx>>,
    ) -> R<EnclosingDrops<'tcx>> {
        let swapped = default.is_some();
        if let Some(default) = default {
            let outer = self.swap_drops(default);
            self.drop_state.outer.push(outer);
        }
        // One that fails to start leaves the enclosing body's drops as they were.
        if let Err(refused) = self.drop_facts() {
            if swapped {
                let outer = self.drop_state.outer.pop().expect("the function's drops");
                self.restore_drops(outer);
            }
            return Err(refused);
        }
        Ok(EnclosingDrops {
            swapped,
            hoisted: self.begin_hoisting(),
        })
    }

    /// Declare drop functions once, at the top of the body that's starting:
    /// what the enclosing one had, to give back to `end_hoisting`.
    pub(in crate::lower) fn begin_hoisting(&mut self) -> Option<Hoisted<'tcx>> {
        self.drop_state.hoisted.replace(Hoisted::default())
    }

    /// The drop functions the body that's ending declared, for its top, and
    /// the enclosing body's back.
    pub(in crate::lower) fn end_hoisting(&mut self, enclosing: Option<Hoisted<'tcx>>) -> Vec<Stmt> {
        std::mem::replace(&mut self.drop_state.hoisted, enclosing).map_or_else(Vec::new, |hoisted| hoisted.defs)
    }

    /// Finish the body `enter_body_drops` started: every owner it has had a
    /// scope, and every move its flag cleared, and the enclosing body's back.
    pub(in crate::lower) fn leave_body_drops(&mut self, enclosing: EnclosingDrops<'tcx>) -> Vec<Stmt> {
        if enclosing.swapped {
            let outer = self.drop_state.outer.pop().expect("the function's drops");
            self.restore_drops(outer);
        }
        self.end_hoisting(enclosing.hoisted)
    }

    /// The std associated types of the function being lowered's type
    /// parameters a caller would give a drop for, `drop_items` (ADR 0489).
    pub(in crate::lower) fn give_drop_slots(&mut self, slots: impl IntoIterator<Item = Ty<'tcx>>) {
        self.drop_state.types.set_slots(slots);
    }

    /// The function being lowered is given a drop `index`, named `name`, of
    /// `ty`: its type parameter's (ADR 0098), or a std trait's associated
    /// type of one (ADR 0421).
    pub(in crate::lower) fn give_drop_param(&mut self, index: u32, name: String, ty: Ty<'tcx>) {
        self.drop_state.types.give(ty);
        self.drop_state.param_drops.insert(ty, GivenDrop::own(index, name));
    }

    /// Whether a drop of `ty`, which holds nothing with a destructor but
    /// type parameters' values, is written with drops this function is
    /// given: each type parameter it holds is given one, a closure's of what
    /// it took (ADR 0485).
    pub(in crate::lower) fn holds_given_drops(&self, ty: Ty<'tcx>) -> bool {
        let held = match crate::lower::recognition::held_types(ty) {
            Some((_, held)) => Ty::new_tup(self.tcx, held),
            None => ty,
        };
        owned_params(self.tcx, held).into_iter().all(|index| {
            (self.drop_state.param_drops.keys()).any(|given| matches!(given.kind(), ty::Param(p) if p.index == index))
        })
    }

    /// The slot this function is given a drop for of a `ty`, a type
    /// parameter or an associated type of one, if it's given one: keyed by
    /// its type, its slot's, alone.
    pub(in crate::lower) fn given_drop(&self, ty: Ty<'tcx>) -> Option<Ty<'tcx>> {
        (crate::lower::recognition::known_by_bounds(ty) && self.drop_state.param_drops.contains_key(&ty)).then_some(ty)
    }

    /// The drop this function is given for a `ty`, `given_drop`'s, noted as
    /// used: `dropT`.
    pub(in crate::lower) fn use_given_drop(&mut self, ty: Ty<'tcx>) -> Option<Expr> {
        let ty = self.given_drop(ty)?;
        self.drop_state.used_drops.insert(ty);
        Some(Expr::var(&self.drop_state.param_drops[&ty].name))
    }

    /// A drop given for a while, `name`, of `ty`, a dictionary entry's for
    /// its method's own type parameter or an associated type of one (ADRs
    /// 0163, 0421): what was given before, to put back. Keyed by its type,
    /// it hides only a drop of that type.
    pub(in crate::lower) fn lend_drop_param(&mut self, name: String, ty: Ty<'tcx>) -> LentDrop<'tcx> {
        let newly_given = self.drop_state.types.given.insert(ty);
        let before = self.drop_state.param_drops.insert(ty, GivenDrop::lent(name));
        LentDrop {
            ty,
            before,
            newly_given,
        }
    }

    /// Put back what `lend_drop_param` replaced.
    pub(in crate::lower) fn return_drop_param(&mut self, lent: LentDrop<'tcx>) {
        if lent.newly_given {
            self.drop_state.types.given.swap_remove(&lent.ty);
        }
        match lent.before {
            Some(before) => self.drop_state.param_drops.insert(lent.ty, before),
            None => self.drop_state.param_drops.swap_remove(&lent.ty),
        };
    }

    /// A copied default body's drops, in place of this function's, for its
    /// trait's type parameters, `Self` among them (ADR 0049): each is what the
    /// impl's argument for it drops, with the impl's own drops. What was
    /// there, and what was found with it, is given back by `restore_drops`.
    fn swap_drops(&mut self, (drops, slots): DefaultDrops<'tcx>) -> SwappedDrops<'tcx> {
        let cache = std::mem::take(&mut *self.drop_state.types.cache.borrow_mut());
        let sizes = std::mem::take(&mut *self.drop_state.sizes.borrow_mut());
        let given = drops.keys().copied().collect();
        SwappedDrops {
            params: std::mem::replace(&mut self.drop_state.param_drops, drops),
            given: std::mem::replace(&mut self.drop_state.types.given, given),
            slots: std::mem::replace(&mut self.drop_state.types.slots, slots),
            used: std::mem::take(&mut self.drop_state.used_drops),
            cache,
            sizes,
        }
    }

    /// The drops of the function a copied default is copied into, its
    /// impl's dictionary's, which its body, in the impl's terms, drops with
    /// (`default_args`): each given, those it can't make, its std associated
    /// types' and each it could be given (ADRs 0421, 0489).
    pub(in crate::lower) fn impl_drops_for_default(&self) -> DefaultDrops<'tcx> {
        (self.drop_state.param_drops.clone(), self.drop_state.types.slots.clone())
    }

    /// The types whose given drops the body being lowered has used.
    pub(in crate::lower) fn used_drops(&self) -> IndexSet<Ty<'tcx>> {
        self.drop_state.used_drops.clone()
    }

    fn restore_drops(&mut self, swapped: SwappedDrops<'tcx>) {
        self.drop_state.param_drops = swapped.params;
        self.drop_state.types.given = swapped.given;
        self.drop_state.types.slots = swapped.slots;
        self.drop_state.used_drops = swapped.used;
        *self.drop_state.types.cache.borrow_mut() = swapped.cache;
        *self.drop_state.sizes.borrow_mut() = swapped.sizes;
    }

    /// The drops this function is given, as it takes them: by their type
    /// parameters' order.
    pub(in crate::lower) fn given_drops(&self) -> Vec<String> {
        let mut given: Vec<&GivenDrop> = self.drop_state.param_drops.values().collect();
        given.sort_by_key(|given| (given.index.is_none(), given.index));
        given.into_iter().map(|given| given.name.clone()).collect()
    }

    /// `ZZone.$dropOffset`, an associated type's drop, of the dictionary of
    /// the impl it's of, which has one where its type does (ADR 0178).
    pub(in crate::lower) fn item_drop(&self, ty: Ty<'tcx>) -> Option<Expr> {
        let (owner, item) = self.drop_query().item_drop_of(ty)?;
        let dictionary = self.evidence_for(owner)?;
        Some(Expr::member(dictionary, item_drop_key(self.tcx, item)))
    }
}
