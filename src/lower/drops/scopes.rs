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
        self.drop_state.types.slots = (slots.into_iter())
            .filter_map(|ty| match *ty.kind() {
                ty::Alias(_, alias) => Some(alias),
                _ => None,
            })
            .collect();
    }

    /// The function being lowered is given a drop `index`, named `name`, of
    /// `ty`: its type parameter's (ADR 0098), or a std trait's associated
    /// type of one (ADR 0421).
    pub(in crate::lower) fn give_drop_param(&mut self, index: u32, name: String, ty: Ty<'tcx>) {
        match ty.kind() {
            ty::Alias(_, alias) => {
                self.drop_state.types.items.insert(*alias, index);
            }
            _ => {
                self.drop_state.types.given.insert(index);
            }
        }
        self.drop_state.param_drops.insert(index, name);
    }

    /// Each drop this function is given of a std associated type of a type
    /// parameter, `dropTItem` of `T::Item`, by its type, which a copied
    /// default of its impl is given too (ADR 0421).
    pub(in crate::lower) fn given_item_drops(&self) -> Vec<(ty::AliasTy<'tcx>, String)> {
        let mut given: Vec<(ty::AliasTy<'tcx>, u32)> = (self.drop_state.types.items.iter())
            .filter(|(_, index)| self.drop_state.param_drops.contains_key(index))
            .map(|(&alias, &index)| (alias, index))
            .collect();
        given.sort_by_key(|&(_, index)| index);
        (given.into_iter())
            .map(|(alias, index)| (alias, self.drop_state.param_drops[&index].clone()))
            .collect()
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
        owned_params(self.tcx, held)
            .into_iter()
            .all(|index| self.drop_state.param_drops.contains_key(&index))
    }

    /// The index of the drop this function is given for a `ty`, a type
    /// parameter or an associated type of one, if it's given one.
    pub(in crate::lower) fn given_drop(&self, ty: Ty<'tcx>) -> Option<u32> {
        let index = match ty.kind() {
            ty::Param(param) => param.index,
            ty::Alias(_, alias) => self.drop_query().slot_of(*alias)?,
            _ => return None,
        };
        self.drop_state.param_drops.contains_key(&index).then_some(index)
    }

    /// A drop given for a while, a dictionary entry's for its method's own
    /// type parameter (ADR 0163), `index`, of `ty`: what was given before,
    /// to put back. An associated type's, found by its type (ADR 0421), is
    /// lent under an index no type parameter has, so it hides none's.
    pub(in crate::lower) fn lend_drop_param(&mut self, index: u32, name: String, ty: Ty<'tcx>) -> LentDrop<'tcx> {
        let alias = match *ty.kind() {
            ty::Alias(_, alias) => Some(alias),
            _ => None,
        };
        let index = match alias {
            Some(alias) => {
                let index = u32::MAX - self.drop_state.types.items.len() as u32;
                self.drop_state.types.items.insert(alias, index);
                index
            }
            None => {
                self.drop_state.types.given.insert(index);
                index
            }
        };
        let before = self.drop_state.param_drops.insert(index, name);
        LentDrop { index, alias, before }
    }

    /// Put back what `lend_drop_param` replaced.
    pub(in crate::lower) fn return_drop_param(&mut self, lent: LentDrop<'tcx>) {
        if let Some(alias) = lent.alias {
            self.drop_state.types.items.remove(&alias);
        } else if lent.before.is_none() {
            self.drop_state.types.given.remove(&lent.index);
        }
        match lent.before {
            Some(name) => self.drop_state.param_drops.insert(lent.index, name),
            None => self.drop_state.param_drops.remove(&lent.index),
        };
    }

    /// A copied default body's drops, in place of this function's, for its
    /// trait's type parameters, `Self` among them (ADR 0049): each is what the
    /// impl's argument for it drops, with the impl's own drops. What was
    /// there, and what was found with it, is given back by `restore_drops`.
    fn swap_drops(&mut self, (drops, unsupported, items, slots, self_args): DefaultDrops<'tcx>) -> SwappedDrops<'tcx> {
        let cache = std::mem::take(&mut *self.drop_state.types.cache.borrow_mut());
        let sizes = std::mem::take(&mut *self.drop_state.sizes.borrow_mut());
        let given = (drops.keys().copied())
            .filter(|index| !items.values().any(|at| at == index))
            .collect();
        SwappedDrops {
            params: std::mem::replace(&mut self.drop_state.param_drops, drops),
            given: std::mem::replace(&mut self.drop_state.types.given, given),
            items: std::mem::replace(&mut self.drop_state.types.items, items),
            slots: std::mem::replace(&mut self.drop_state.types.slots, slots),
            // And the default's own parameters, the body's owner's, which a
            // type in its trait's terms names.
            self_args: (self.drop_state.types.self_args).replace((
                self_args,
                self.impl_typing_env().unwrap_or(self.typing_env),
                ty::GenericArgs::identity_for_item(self.tcx, self.frame.owner),
            )),
            used: std::mem::take(&mut self.drop_state.used_drops),
            unsupported: std::mem::replace(&mut self.drop_state.types.unsupported, unsupported),
            cache,
            sizes,
        }
    }

    /// The type parameters whose drops the body being lowered has used.
    pub(in crate::lower) fn used_drops(&self) -> HashSet<u32> {
        self.drop_state.used_drops.clone()
    }

    /// What `f` makes of what's in the impl's terms, in a copied default:
    /// the arguments of the impl's own method its body calls, or a
    /// dictionary of its impl's, whose type parameters are the impl's, not
    /// its trait's, which share their indexes: asked of the impl's drops
    /// (ADR 0495). Anywhere else, `f`'s.
    pub(in crate::lower) fn with_impl_drops<T>(&mut self, f: impl FnOnce(&mut Self) -> R<T>) -> R<T> {
        let Some(mut outer) = self.drop_state.outer.pop() else {
            return f(self);
        };
        self.exchange_drops(&mut outer);
        let made = f(self);
        self.exchange_drops(&mut outer);
        self.drop_state.outer.push(outer);
        made
    }

    /// The function's drops and `other`'s, each in the other's place.
    fn exchange_drops(&mut self, other: &mut SwappedDrops<'tcx>) {
        let state = &mut self.drop_state;
        std::mem::swap(&mut state.param_drops, &mut other.params);
        std::mem::swap(&mut state.types.given, &mut other.given);
        std::mem::swap(&mut state.types.items, &mut other.items);
        std::mem::swap(&mut state.types.slots, &mut other.slots);
        std::mem::swap(&mut state.types.self_args, &mut other.self_args);
        std::mem::swap(&mut state.used_drops, &mut other.used);
        std::mem::swap(&mut state.types.unsupported, &mut other.unsupported);
        std::mem::swap(&mut *state.types.cache.borrow_mut(), &mut other.cache);
        std::mem::swap(&mut *state.sizes.borrow_mut(), &mut other.sizes);
    }

    fn restore_drops(&mut self, swapped: SwappedDrops<'tcx>) {
        self.drop_state.param_drops = swapped.params;
        self.drop_state.types.given = swapped.given;
        self.drop_state.types.items = swapped.items;
        self.drop_state.types.slots = swapped.slots;
        self.drop_state.types.self_args = swapped.self_args;
        self.drop_state.used_drops = swapped.used;
        self.drop_state.types.unsupported = swapped.unsupported;
        *self.drop_state.types.cache.borrow_mut() = swapped.cache;
        *self.drop_state.sizes.borrow_mut() = swapped.sizes;
    }

    /// The drops this function is given, as it takes them: by their type
    /// parameters' order.
    pub(in crate::lower) fn given_drops(&self) -> Vec<String> {
        let mut given: Vec<(&u32, &String)> = self.drop_state.param_drops.iter().collect();
        given.sort();
        given.into_iter().map(|(_, name)| name.clone()).collect()
    }

    /// `ZZone.$dropOffset`, an associated type's drop, of the dictionary of
    /// the impl it's of, which has one where its type does (ADR 0178).
    pub(in crate::lower) fn item_drop(&self, ty: Ty<'tcx>) -> Option<Expr> {
        let (owner, item) = self.drop_query().item_drop_of(ty)?;
        let dictionary = self.evidence_for(owner)?;
        Some(Expr::member(dictionary, item_drop_key(self.tcx, item)))
    }
}
