//! A handle on a `&mut`'s place, and a `dyn`'s pair on one: the one way
//! each is made of a place, so what's written through it is stored as the
//! place stores it, a nullable field's `None` as `null` (ADR 0791, #2427).
//! A handle a variable holds of a `&mut`'s place (`Made::handles`) is
//! that handle wherever its `value` is lent on, as a call's argument, what
//! a call gives back, a vector keeps or a `dyn`'s pair holds. Split from
//! `mut_refs.rs` by concept.

use super::*;

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// A handle on `place`, which reads and writes it: of a held handle's
    /// `value` (`held_handle`), that handle, which stores as its place
    /// does; of any other, one that stores what's written as it's given.
    pub fn handle_on(&self, place: Expr) -> Expr {
        match (self.held_handle(&place), &place.kind) {
            (Some(_), js::ExprKind::Member(handle, _)) => (**handle).clone(),
            _ => Expr::handle(place),
        }
    }

    /// A `dyn`'s pair on `place` and its `dictionary`: of a held handle's
    /// `value` (`held_handle`), on that handle's place, stored as it
    /// stores, `set value(value) { fiber.owner = value ?? null; }` (#977);
    /// of any other, one that stores what's written as it's given.
    pub fn pair_on(&self, place: Expr, dictionary: Expr) -> Expr {
        match self.held_handle(&place) {
            Some(handled) => Expr::stored_pair(handled.on.clone(), dictionary, handled.storage),
            None => Expr::pair(place, dictionary),
        }
    }

    /// `pair_on`'s pair on `place`, as it is where it's made
    /// (`place_where_made`, #968): a held handle's is on that handle's
    /// place, asked before what reaches it is named again.
    pub fn pair_made(&mut self, place: Expr, dictionary: Expr, out: &mut Vec<Stmt>) -> Expr {
        match self.held_handle(&place).is_some() {
            true => self.pair_on(place, dictionary),
            false => {
                let place = self.place_where_made(place, out);
                Expr::pair(place, dictionary)
            }
        }
    }
}
