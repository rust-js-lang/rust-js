//! A std collection's `Extend` dictionary's entries of the trait's provided
//! methods its std impl writes itself (#2414): what `$provided` asks of the
//! dictionary before std's default over `Self`. `extend_reserve`, the
//! collection's own `reserve`, which a crate reading capacities sees and
//! whose room check may panic; std's `extend_one`, its `push` or `insert`,
//! is what the default, `extend(Some(item))`, does to it, so it has none.
//!
//! A hash table's rule is hashbrown's, read from its source, whose empty
//! table's branch no native run tells from its other (ADR 1191):
//! std-source-pin: vendor/hashbrown-0.17.1/src/map.rs `fn extend_reserve` 932c3a3facf18bb3634ec3037f85a63c6b4c9936c38173e5a3c4082cd3a373c0

use super::*;

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// `extendReserve` of `ty`'s `Extend<item>` dictionary, where its std impl
    /// writes `extend_reserve` (`overridden_in_reach`'s question, of this
    /// impl): `(collection, additional) => ..` of the collection's own
    /// `reserve`, as std's impl calls it. A `Vec`'s, a `VecDeque`'s and a
    /// `BinaryHeap`'s, its room checked and, where capacities are kept,
    /// `$reserve`d; a `String`'s, `$textRoomFor`; a hash table's,
    /// hashbrown's, the whole lower bound of an empty one, else its half
    /// rounded up, `div_ceil(2)`, its room checked. `None` where the impl
    /// doesn't write it; refused for another that does.
    pub(super) fn extend_reserve_entry(&mut self, tr: ty::TraitRef<'tcx>, span: Span) -> R<Option<Prop>> {
        let tcx = self.tcx;
        let ty = tr.self_ty();
        let reserve = crate::recognition::trait_method(tcx, tr.def_id, "extend_reserve");
        let writes = evidence_impl(tcx, self.typing_env, tr)
            .is_some_and(|(imp, _)| tcx.impl_item_implementor_ids(imp).contains_key(&reserve));
        if !writes {
            return Ok(None);
        }
        let [collection, additional] = ["collection", "additional"].map(|word| js::Name::declared(word.to_string()));
        let held = Expr::member(Expr::var(collection.clone()), js::HELD);
        let asked = Expr::var(additional.clone());
        let mut body = Vec::new();
        let args = match ty.kind() {
            ty::Adt(_, args) => *args,
            _ => ty::List::empty(),
        };
        if self.is_vec_like(ty) {
            let elem = args.type_at(0);
            let size = self.room_item_size(elem);
            self.room_for(
                (Some(Expr::member(held.clone(), "length")), asked.clone()),
                size,
                span,
                &mut body,
            );
            if self.krate.keeps_capacity {
                let size = self.capacity_item_size(elem).unwrap_or_else(Expr::js_undefined);
                let reserved = Expr::call_helper("$reserve", vec![held, asked, size]);
                body.push(StmtKind::Expr(reserved).at(js::Span::NONE));
            }
        } else if self.is_string_like(ty) {
            let checked = Expr::call_helper("$textRoomFor", vec![held, asked]);
            body.push(StmtKind::Expr(checked).at(js::Span::NONE));
        } else if self.is_map(ty) && !self.is_sorted(ty) {
            let entry = match self.recognition().is_set(ty) {
                true => args.type_at(0),
                false => Ty::new_tup(tcx, &[args.type_at(0), args.type_at(1)]),
            };
            let size = Expr::member(held, "size");
            let half = Expr::call(
                Expr::member(Expr::global("Math"), "ceil"),
                vec![Expr::bin(Op::Div, asked.clone(), Expr::int(2))],
            );
            let reserved = Expr::cond(Expr::bin(Op::Eq, size.clone(), Expr::int(0)), asked, half);
            self.table_room_for(Expr::bin(Op::Add, size, reserved), entry, span, &mut body);
        } else {
            let what = format!("an `Extend` dictionary of `{ty}`, whose `extend_reserve` std's impl writes (#2414)");
            return Err(self.unsupported(span, &what));
        }
        // Made at no site of the source: a dictionary of one type is one,
        // declared once where a body gives it twice (ADR 0979).
        for stmt in &mut body {
            stmt.span = js::Span::NONE;
        }
        let entry = Expr::arrow(vec![collection.into(), additional.into()], body);
        Ok(Some(Prop::Field(bindings::fn_name(tcx, reserve), entry)))
    }
}
