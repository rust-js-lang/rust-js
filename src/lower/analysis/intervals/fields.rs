//! A struct's fields a reference parameter is read through as locals of
//! their own (#1672 M3, step 5): `self.items[self.at]` of `&mut self`, the
//! walk's rules of a local then the field's. A test narrows it, a copy is
//! one of it, a `&mut` of it lends it, and a `Vec` field lent only to calls
//! keeps its length (`called_on`).

use super::*;

/// `body` with each field of a struct a reference parameter points to, a
/// local of its own: `(*self).at`, `at`. Of a parameter only ever read
/// through its fields, `(*self).f`, never itself, `&mut *self` given a
/// method, which may reach any field; or of a shared one to a struct that
/// holds no cell itself, whose fields can't change while it's borrowed. Its blocks and statements are the
/// body's, each where it was. None where no parameter is so.
pub(super) fn fields_as_locals<'tcx>(tcx: TyCtxt<'tcx>, body: &Body<'tcx>) -> Option<Body<'tcx>> {
    use rustc_middle::mir::visit::{MutVisitor, PlaceContext, Visitor};
    let pointers: Vec<Local> = (1..=body.arg_count)
        .map(Local::from_usize)
        .filter(|&param| {
            matches!(body.local_decls[param].ty.builtin_deref(true).map(|to| *to.kind()),
                Some(ty::Adt(adt, _)) if adt.is_struct())
                && body.local_decls[param].ty.is_ref()
        })
        .collect();
    if pointers.is_empty() {
        return None;
    }
    // A place of a field through one, `(*p).f`, by its parameter and field.
    let field = |place: &mir::Place<'tcx>| match place.projection.as_slice() {
        [mir::PlaceElem::Deref, mir::PlaceElem::Field(k, ty), ..] => Some((k.index(), *ty)),
        _ => None,
    };
    struct Uses<'a, 'tcx> {
        pointers: &'a [Local],
        field: &'a dyn Fn(&mir::Place<'tcx>) -> Option<(usize, Ty<'tcx>)>,
        whole: IndexSet<Local>,
        fields: IndexMap<(Local, usize), Ty<'tcx>>,
    }
    impl<'tcx> Visitor<'tcx> for Uses<'_, 'tcx> {
        fn visit_place(&mut self, place: &mir::Place<'tcx>, context: PlaceContext, _: Location) {
            if !self.pointers.contains(&place.local) || matches!(context, PlaceContext::NonUse(_)) {
                return;
            }
            match (self.field)(place) {
                Some((k, ty)) => {
                    self.fields.insert((place.local, k), ty);
                }
                None => {
                    self.whole.insert(place.local);
                }
            }
        }
    }
    let mut uses = Uses {
        pointers: &pointers,
        field: &field,
        whole: IndexSet::default(),
        fields: IndexMap::default(),
    };
    uses.visit_body(body);
    // Of a shared reference to what holds no cell itself, whose fields
    // nothing changes while it's borrowed, also where it's used whole,
    // returned or compared, `find(&node.children[0])` beside `node`.
    let typing_env = ty::TypingEnv::post_analysis(tcx, body.source.def_id());
    let frozen = |pointer: Local| matches!(*body.local_decls[pointer].ty.kind(), ty::Ref(_, to, ty::Mutability::Not) if to.is_freeze(tcx, typing_env));
    uses.fields
        .retain(|&(pointer, _), _| !uses.whole.contains(&pointer) || frozen(pointer));
    if uses.fields.is_empty() {
        return None;
    }
    let mut rewritten = body.clone();
    let locals: IndexMap<(Local, usize), Local> = (uses.fields.iter())
        .map(|(&at, &ty)| (at, rewritten.local_decls.push(mir::LocalDecl::new(ty, body.span))))
        .collect();
    struct Rewrite<'a, 'tcx> {
        tcx: TyCtxt<'tcx>,
        field: &'a dyn Fn(&mir::Place<'tcx>) -> Option<(usize, Ty<'tcx>)>,
        locals: &'a IndexMap<(Local, usize), Local>,
    }
    impl<'tcx> MutVisitor<'tcx> for Rewrite<'_, 'tcx> {
        fn tcx(&self) -> TyCtxt<'tcx> {
            self.tcx
        }
        fn visit_place(&mut self, place: &mut mir::Place<'tcx>, _: PlaceContext, _: Location) {
            let Some((k, _)) = (self.field)(place) else {
                return;
            };
            let Some(&local) = self.locals.get(&(place.local, k)) else {
                return;
            };
            *place = mir::Place {
                local,
                projection: self.tcx.mk_place_elems(&place.projection[2..]),
            };
        }
    }
    Rewrite {
        tcx,
        field: &field,
        locals: &locals,
    }
    .visit_body(&mut rewritten);
    Some(rewritten)
}
