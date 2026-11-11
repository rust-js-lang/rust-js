//! The MIR a body is lowered from, as it's taken (ADR 0364): what each
//! `impl Trait` hides revealed (`revealed.rs`), what can't throw never
//! unwinding, `unreachable_unchecked()` unreachable code, and of another
//! crate's optimized body, a box read as the value it holds and the
//! switches and reads its optimization left settled; and of it, a
//! closure's body in another's terms (`instantiated`) or as `call_once`
//! runs it (`called_once`).

use super::*;

/// A body's MIR, as borrowck reads it (ADR 0364), and its promoted constants.
pub struct Mir<'tcx> {
    pub(in crate::lower) body: mir::Body<'tcx>,
    pub(in crate::lower) promoted: IndexVec<Promoted, mir::Body<'tcx>>,
    /// Of a coroutine's, an `async` body's: the blocks whose call is an
    /// `.await` (`awaits.rs`).
    pub(in crate::lower) awaits: IndexSet<BasicBlock>,
    /// Of each `match` of an enum its arms' `match` made (`threaded.rs`):
    /// each block an arm goes on to its case through, and where the
    /// `match` is written, whose comments are before the case.
    pub(in crate::lower) cases: IndexMap<BasicBlock, Span>,
}

impl<'tcx> Mir<'tcx> {
    /// It, of a function's generics, in another's terms, `args` of them: a
    /// closure's whose function rustc's MIR inlined (ADR 0899).
    pub(in crate::lower) fn instantiated(&self, tcx: ty::TyCtxt<'tcx>, args: ty::GenericArgsRef<'tcx>) -> Self {
        let of = |body: &mir::Body<'tcx>| {
            ty::EarlyBinder::bind(tcx, body.clone())
                .instantiate(tcx, args)
                .skip_normalization()
        };
        Mir {
            body: of(&self.body),
            promoted: self.promoted.iter().map(of).collect(),
            awaits: self.awaits.clone(),
            cases: self.cases.clone(),
        }
    }

    /// `body`, its `.await`s each one call, and what can't throw never
    /// unwinding.
    pub(in crate::lower) fn new(
        tcx: ty::TyCtxt<'tcx>,
        mut body: mir::Body<'tcx>,
        mut promoted: IndexVec<Promoted, mir::Body<'tcx>>,
    ) -> Mir<'tcx> {
        (body, promoted) = revealed::revealed(tcx, (body, promoted));
        // A call that can't throw, a binding marked so or `unwrap_unchecked`,
        // has nothing to clean up (ADR 0301): rustc's dataflow of what's there,
        // which says what's dropped, follows no unwinding of it.
        for data in body.basic_blocks.as_mut() {
            if let TerminatorKind::Call { func, unwind, .. } = &mut data.terminator_mut().kind
                && let Some((id, _)) = func.const_fn_def()
                && match pure_std(tcx, id) {
                    Some(PureStd::Question | PureStd::BoxNew | PureStd::Unchecked) => true,
                    _ => bindings::is_binding(tcx, id) && bindings::cannot_throw(tcx, id),
                }
            {
                *unwind = UnwindAction::Unreachable;
            }
            // `hint::unreachable_unchecked()` is MIR's unreachable code (ADR
            // 0399): a switch's way to it alone is left out, as one to
            // `unreachable` is, so `match` tests the arms that run.
            if let TerminatorKind::Call { func, .. } = &data.terminator().kind
                && func.const_fn_def().is_some_and(|(id, _)| is_unreachable_hint(tcx, id))
            {
                data.terminator_mut().kind = TerminatorKind::Unreachable;
            }
        }
        // An optimized body's, std's, switch that goes one way whatever it
        // tests, which drop elaboration leaves of a flag it settled, is the
        // way it goes; the discriminant it read and nothing else does, nothing.
        if !body.source.def_id().is_local() {
            boxes_read_as_values(tcx, &mut body);
            // A block that assumes `UbChecks`, which rust-js reads as off,
            // never runs (ADR 0917).
            for data in body.basic_blocks.as_mut() {
                if crate::lower::std_coverage::assumes_ub_checks(data) {
                    data.statements.clear();
                    data.terminator_mut().kind = TerminatorKind::Unreachable;
                }
            }
            let unreachable: Vec<BasicBlock> = (body.basic_blocks.iter_enumerated())
                .filter(|(_, data)| matches!(data.terminator().kind, TerminatorKind::Unreachable))
                .map(|(block, _)| block)
                .collect();
            for data in body.basic_blocks.as_mut() {
                let terminator = data.terminator_mut();
                if let TerminatorKind::SwitchInt { targets, .. } = &terminator.kind {
                    let mut ways: Vec<BasicBlock> = (targets.all_targets().iter().copied())
                        .filter(|b| !unreachable.contains(b))
                        .collect();
                    ways.dedup();
                    if let [way] = ways[..] {
                        terminator.kind = TerminatorKind::Goto { target: way };
                    }
                }
            }
            // An `assume` of what an inlined `?` knows of its residual, which
            // runs nothing, goes; and a discriminant read for a switch that
            // went, which the block's terminator no longer tests and nothing
            // else reads: dropped with it.
            for data in body.basic_blocks.as_mut() {
                data.statements.retain(|statement| {
                    !matches!(&statement.kind, StatementKind::Intrinsic(intrinsic)
                        if matches!(**intrinsic, mir::NonDivergingIntrinsic::Assume(_)))
                });
                let tested = match &data.terminator().kind {
                    TerminatorKind::SwitchInt { discr, .. } => discr.place().and_then(|place| place.as_local()),
                    _ => None,
                };
                let mut read = ReadLocals(Vec::new());
                for statement in &data.statements {
                    Visitor::visit_statement(&mut read, statement, mir::Location::START);
                }
                data.statements.retain(|statement| {
                    !matches!(&statement.kind, StatementKind::Assign(assign)
                        if matches!(assign.1, Rvalue::Discriminant(_))
                            && assign.0.as_local().is_some_and(|local| tested != Some(local) && !read.0.contains(&local)))
                });
            }
        }
        let awaits = match body.coroutine.is_some() {
            true => awaits::awaits(tcx, &mut body),
            false => Default::default(),
        };
        Mir {
            body,
            promoted,
            awaits,
            cases: IndexMap::default(),
        }
    }

    /// Of a closure's body rustc infers `Fn` or `FnMut`, which takes
    /// itself by `&` or `&mut`, the body its `call_once` runs, rustc's
    /// `ClosureOnceShim` inlined: itself taken by value, and dropped as it
    /// returns, or as it unwinds, after what its own body drops. A closure
    /// a binding calls once is called so (ADR 1020). `None` where its body
    /// reads itself other than through the reference, as its fields.
    pub(in crate::lower) fn called_once(&self, tcx: ty::TyCtxt<'tcx>) -> Option<Self> {
        use rustc_middle::mir::visit::MutVisitor;
        struct ByValue<'tcx> {
            tcx: ty::TyCtxt<'tcx>,
            whole: bool,
        }
        impl<'tcx> MutVisitor<'tcx> for ByValue<'tcx> {
            fn tcx(&self) -> ty::TyCtxt<'tcx> {
                self.tcx
            }
            fn visit_place(&mut self, place: &mut Place<'tcx>, _: PlaceContext, _: mir::Location) {
                if place.local != SELF {
                    return;
                }
                match place.projection.first() {
                    Some(PlaceElem::Deref) => {
                        place.projection = self.tcx.mk_place_elems(&place.projection[1..]);
                    }
                    _ => self.whole = true,
                }
            }
        }
        const SELF: Local = Local::from_u32(1);
        let mut body = self.body.clone();
        let ty::Ref(_, closure, _) = *body.local_decls[SELF].ty.kind() else {
            return None;
        };
        let mut by_value = ByValue { tcx, whole: false };
        by_value.visit_body(&mut body);
        if by_value.whole {
            return None;
        }
        body.local_decls[SELF].ty = closure;
        let terminator = |kind| mir::Terminator {
            source_info: mir::SourceInfo::outermost(body.span),
            kind,
            attributes: Default::default(),
        };
        let block = |terminator, is_cleanup| mir::BasicBlockData::new(Some(terminator), is_cleanup);
        let dropped = |target, unwind| TerminatorKind::Drop {
            place: SELF.into(),
            target,
            unwind,
            replace: false,
            drop: None,
        };
        let blocks = body.basic_blocks.as_mut();
        let resumed = blocks.push(block(terminator(TerminatorKind::UnwindResume), true));
        let unwound = blocks.push(block(
            terminator(dropped(
                resumed,
                UnwindAction::Terminate(mir::UnwindTerminateReason::InCleanup),
            )),
            true,
        ));
        let returned = blocks.push(block(terminator(TerminatorKind::Return), false));
        for (at, data) in blocks.iter_enumerated_mut() {
            if [resumed, unwound, returned].contains(&at) {
                continue;
            }
            // What unwinds through its body's own cleanup goes on to drop
            // it. A call that unwinds with no cleanup of its own needs no
            // edge to it: the drop as it returns is a `finally` around
            // what can throw, as the lowering writes one.
            let kind = &mut data.terminator_mut().kind;
            match kind {
                TerminatorKind::Return => *kind = dropped(returned, UnwindAction::Continue),
                TerminatorKind::UnwindResume => *kind = TerminatorKind::Goto { target: unwound },
                _ => {}
            }
        }
        Some(Mir {
            body,
            promoted: self.promoted.clone(),
            awaits: self.awaits.clone(),
            cases: self.cases.clone(),
        })
    }
}

/// Of an optimized body, another crate's, each `Box` read through the
/// pointer rustc's `ElaborateBoxDerefs` makes of it, `_t = copy (_b.0:
/// Unique<T>) as *const T (BoxDerefTransmute); (*_t).x = ..`, read as the
/// crate's own body reads it, `(*_b).x`: a box is the value it holds (ADR
/// 0914, amended). The pointer is made just before the one place that reads
/// through it, past the others made for that statement, so nothing changes
/// the box between; one read otherwise is left as it is.
fn boxes_read_as_values<'tcx>(tcx: ty::TyCtxt<'tcx>, body: &mut mir::Body<'tcx>) {
    use rustc_middle::mir::visit::MutVisitor;
    struct Uses(Local, Vec<(mir::Location, bool)>);
    impl<'tcx> Visitor<'tcx> for Uses {
        fn visit_place(&mut self, place: &Place<'tcx>, _: PlaceContext, location: mir::Location) {
            if place.local == self.0 {
                self.1
                    .push((location, place.projection.first() == Some(&PlaceElem::Deref)));
            }
            for elem in place.projection {
                if elem == PlaceElem::Index(self.0) {
                    self.1.push((location, false));
                }
            }
        }
        fn visit_local(&mut self, local: Local, _: PlaceContext, location: mir::Location) {
            if local == self.0 {
                self.1.push((location, false));
            }
        }
    }
    struct Through<'tcx>(Local, Local, ty::TyCtxt<'tcx>);
    impl<'tcx> MutVisitor<'tcx> for Through<'tcx> {
        fn tcx(&self) -> ty::TyCtxt<'tcx> {
            self.2
        }
        fn visit_place(&mut self, place: &mut Place<'tcx>, _: PlaceContext, _: mir::Location) {
            if place.local == self.0 {
                place.local = self.1;
            }
        }
    }
    let pointer_of = |statement: &mir::Statement<'_>| match &statement.kind {
        StatementKind::Assign(assign) => match (&assign.0, &assign.1) {
            (pointer, Rvalue::Cast(mir::CastKind::BoxDerefTransmute, Operand::Copy(unique), _))
                if let Some(pointer) = pointer.as_local()
                    && let [PlaceElem::Field(field, _)] = unique.projection[..]
                    && field == FieldIdx::ZERO =>
            {
                Some((pointer, unique.local))
            }
            _ => None,
        },
        _ => None,
    };
    let blocks: Vec<BasicBlock> = body.basic_blocks.indices().collect();
    for block in blocks {
        let mut at = 0;
        while at < body.basic_blocks[block].statements.len() {
            let data = &body.basic_blocks[block];
            let Some((pointer, boxed)) = pointer_of(&data.statements[at]) else {
                at += 1;
                continue;
            };
            if !body.local_decls[boxed].ty.is_box() {
                at += 1;
                continue;
            }
            // The statement that reads through it: the next but the other
            // pointers made for it, or the block's terminator.
            let read = (at + 1..data.statements.len())
                .find(|&i| pointer_of(&data.statements[i]).is_none())
                .unwrap_or(data.statements.len());
            let made = mir::Location {
                block,
                statement_index: at,
            };
            let read_at = mir::Location {
                block,
                statement_index: read,
            };
            let mut uses = Uses(pointer, Vec::new());
            uses.visit_body(body);
            let only_there =
                (uses.1.iter()).all(|&(location, deref)| location == made || (location == read_at && deref));
            if !only_there || !uses.1.iter().any(|&(location, _)| location == read_at) {
                at += 1;
                continue;
            }
            let mut through = Through(pointer, boxed, tcx);
            let data = &mut body.basic_blocks.as_mut_preserves_cfg()[block];
            match data.statements.get_mut(read) {
                Some(statement) => through.visit_statement(statement, read_at),
                None => through.visit_terminator(data.terminator_mut(), read_at),
            }
            data.statements[at].make_nop(true);
            at += 1;
        }
    }
}

/// The locals a body reads, as an operand or in a place: what its
/// optimized form's dead switches leave unread is dropped with them.
struct ReadLocals(Vec<Local>);

impl<'tcx> Visitor<'tcx> for ReadLocals {
    fn visit_local(&mut self, local: Local, context: PlaceContext, _: mir::Location) {
        if !matches!(context, PlaceContext::MutatingUse(MutatingUseContext::Store)) && !self.0.contains(&local) {
            self.0.push(local);
        }
    }
}
