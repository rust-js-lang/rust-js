//! An or-pattern's alternatives that JS holds as one: each binds what's at
//! one JS place, `Shape::Circle { r } | Shape::Square { r }`'s `shape.r`, a
//! flattened state's `Pending { value } | Blocked { value }`'s
//! `chunk.value`, so its arm is one test of either and its binding that
//! place, as upstream writes `case PENDING: case BLOCKED:` (rd430).

use super::*;

/// A body's or-pattern alternatives JS holds as an earlier one
/// (`mir_alike_alternatives`).
#[derive(Default)]
pub(super) struct Alternatives {
    /// Each switch's target that's an earlier target's alternative, and
    /// that target: the edges to it are that target's, as a guard's that
    /// holds are its arm's (`held_arms`).
    pub(super) held_as: HashMap<BasicBlock, BasicBlock>,
    /// The blocks of those targets, to where they bind, which JS never
    /// runs: what they write written once, by the earlier's.
    pub(super) unreached: HashSet<BasicBlock>,
}

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// Each switch's target whose alternative is an earlier target's in
    /// JS, and that target: their blocks, past empty ones, each statement
    /// alike but of a field of another variant at one JS place
    /// (`one_variant_place`), going on to one block.
    pub(super) fn mir_alike_alternatives(&self, body: &mir::Body<'tcx>) -> Alternatives {
        // Where a block goes on to past empty ones, a false edge's.
        let landing = |mut at: BasicBlock| {
            let mut seen = HashSet::new();
            while seen.insert(at) {
                let data = &body.basic_blocks[at];
                match &data.terminator().kind {
                    TerminatorKind::Goto { target }
                    | TerminatorKind::FalseEdge {
                        real_target: target, ..
                    } if data.statements.is_empty() => at = *target,
                    _ => break,
                }
            }
            at
        };
        let mut alike = Alternatives::default();
        for data in body.basic_blocks.iter() {
            let TerminatorKind::SwitchInt { targets, .. } = &data.terminator().kind else {
                continue;
            };
            let targets: Vec<BasicBlock> = targets.iter().map(|(_, target)| target).collect();
            for (k, &later) in targets.iter().enumerate() {
                if alike.held_as.contains_key(&later) {
                    continue;
                }
                let earlier = (targets[..k].iter())
                    .find(|&&earlier| earlier != later && self.alike_blocks(body, landing(earlier), landing(later)));
                if let Some(&earlier) = earlier {
                    alike.held_as.insert(later, earlier);
                    // Its blocks to where it binds: its own, a false edge
                    // to it the earlier alternative's.
                    let (mut at, end) = (later, landing(later));
                    while at != end && alike.unreached.insert(at) {
                        at = match body.basic_blocks[at].terminator().kind {
                            TerminatorKind::Goto { target }
                            | TerminatorKind::FalseEdge {
                                real_target: target, ..
                            } => target,
                            _ => break,
                        };
                    }
                    alike.unreached.insert(end);
                }
            }
        }
        alike
    }

    /// Whether blocks `a` and `b` are one in JS: each statement alike
    /// (`alike_places`), one at least of a field of another variant, an
    /// alternative's binding by reference or by copy, never a move, whose
    /// drops follow which part moved; and each going on to one block. Blocks alike
    /// in all, two arms written alike, stay the program's.
    fn alike_blocks(&self, body: &mir::Body<'tcx>, a: BasicBlock, b: BasicBlock) -> bool {
        let differ = std::cell::Cell::new(false);
        let (a, b) = (&body.basic_blocks[a], &body.basic_blocks[b]);
        let goes_on = |data: &mir::BasicBlockData<'tcx>| match data.terminator().kind {
            TerminatorKind::Goto { target } => Some(target),
            _ => None,
        };
        let place = |x: &Place<'tcx>, y: &Place<'tcx>| {
            differ.set(differ.get() || x != y);
            self.alike_places(body, *x, *y)
        };
        let operand = |x: &Operand<'tcx>, y: &Operand<'tcx>| match (x, y) {
            // What's moved out of a variant, which drops what's left of
            // it by which moved, only of one place in both.
            (Operand::Copy(x), Operand::Copy(y)) => place(x, y),
            _ => x == y,
        };
        let alike = |x: &mir::Statement<'tcx>, y: &mir::Statement<'tcx>| match (&x.kind, &y.kind) {
            (StatementKind::Assign(x), StatementKind::Assign(y)) => {
                x.0 == y.0
                    && match (&x.1, &y.1) {
                        (Rvalue::Ref(_, kx, px), Rvalue::Ref(_, ky, py)) => kx == ky && place(px, py),
                        (Rvalue::Use(ox, wx), Rvalue::Use(oy, wy)) => wx == wy && operand(ox, oy),
                        (Rvalue::CopyForDeref(px), Rvalue::CopyForDeref(py)) => place(px, py),
                        (rx, ry) => rx == ry,
                    }
            }
            (x, y) => x == y,
        };
        goes_on(a).is_some()
            && goes_on(a) == goes_on(b)
            && a.statements.len() == b.statements.len()
            && a.statements.iter().zip(&b.statements).all(|(x, y)| alike(x, y))
            && differ.get()
    }

    /// Whether places `x` and `y` are one JS place: alike but where each
    /// is of a field of another variant at one JS place (`one_variant_place`).
    fn alike_places(&self, body: &mir::Body<'tcx>, x: Place<'tcx>, y: Place<'tcx>) -> bool {
        if x.local != y.local || x.projection.len() != y.projection.len() {
            return false;
        }
        let (xs, ys) = (x.projection.as_slice(), y.projection.as_slice());
        let mut k = 0;
        while k < xs.len() {
            match (xs[k], xs.get(k + 1), ys[k], ys.get(k + 1)) {
                (
                    PlaceElem::Downcast(_, vx),
                    Some(&PlaceElem::Field(fx, _)),
                    PlaceElem::Downcast(_, vy),
                    Some(&PlaceElem::Field(fy, _)),
                ) if vx != vy => {
                    let of = Place::ty_from(x.local, &xs[..k], &body.local_decls, self.tcx).ty;
                    if !self.one_variant_place(of, (vx, fx), (vy, fy)) {
                        return false;
                    }
                    k += 2;
                }
                (ex, _, ey, _) if ex == ey => k += 1,
                _ => return false,
            }
        }
        true
    }
}
