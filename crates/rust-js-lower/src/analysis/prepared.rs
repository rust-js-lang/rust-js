//! The MIR a body is lowered from, as it's taken (ADR 0364): what each
//! `impl Trait` hides revealed (`revealed.rs`), what can't throw never
//! unwinding, `unreachable_unchecked()` unreachable code, and of another
//! crate's optimized body, a box read as the value it holds and the
//! switches and reads its optimization left settled; and of it, a
//! closure's body in another's terms (`instantiated`) or as `call_once`
//! runs it (`called_once`).

use super::{awaits, revealed};
use crate::bindings;
use crate::recognition::{PureStd, is_unreachable_hint, pure_std};
use rust_js_program::collections::{IndexMap, IndexSet};
use rustc_abi::FieldIdx;
use rustc_hir::def::DefKind;
use rustc_index::IndexVec;
use rustc_middle::mir::visit::{MutatingUseContext, PlaceContext, Visitor};
use rustc_middle::mir::{
    self, BasicBlock, Local, Operand, Place, PlaceElem, Promoted, Rvalue, StatementKind, TerminatorKind, UnwindAction,
};
use rustc_middle::ty::{self, TyCtxt};
use rustc_span::Span;
use rustc_span::def_id::LocalDefId;

/// A body's MIR, as borrowck reads it (ADR 0364), and its promoted constants.
#[derive(Clone)]
pub struct Mir<'tcx> {
    pub body: mir::Body<'tcx>,
    pub promoted: IndexVec<Promoted, mir::Body<'tcx>>,
    /// Of a coroutine's, an `async` body's: the blocks whose call is an
    /// `.await` (`awaits.rs`).
    pub awaits: IndexSet<BasicBlock>,
    /// Of each `match` of an enum its arms' `match` made (`threaded.rs`):
    /// each block an arm goes on to its case through, and where the
    /// `match` is written, whose comments are before the case.
    pub cases: IndexMap<BasicBlock, Span>,
    /// rustc's own body of it, its drops elaborated, where the session
    /// compares them with rust-js's (the `drops` shadow, `elaborated.rs`).
    /// Of a closure's body as `call_once` runs it (`called_once`), the
    /// closure's own: its drops but those of itself, whose are rustc's
    /// `ClosureOnce` shim's. Its captures, which rustc's reads through its
    /// `&self`, `(*_1).0`, aren't dropped there: none of the corpus's is.
    pub elaborated: Option<mir::Body<'tcx>>,
    /// Whether it's a closure's body as `call_once` runs it (`called_once`):
    /// itself taken by value, and dropped where the body returns or
    /// unwinds, as rustc's shim drops it (#2177).
    pub called_once: bool,
}

impl<'tcx> Mir<'tcx> {
    /// It, of a function's generics, in another's terms, `args` of them: a
    /// closure's whose function rustc's MIR inlined (ADR 0899).
    pub fn instantiated(&self, tcx: ty::TyCtxt<'tcx>, args: ty::GenericArgsRef<'tcx>) -> Self {
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
            elaborated: self.elaborated.as_ref().map(of),
            called_once: self.called_once,
        }
    }

    /// `body`, its `.await`s each one call, and what can't throw never
    /// unwinding.
    pub fn new(
        tcx: ty::TyCtxt<'tcx>,
        mut body: mir::Body<'tcx>,
        mut promoted: IndexVec<Promoted, mir::Body<'tcx>>,
    ) -> Mir<'tcx> {
        (body, promoted) = revealed::revealed(tcx, (body, promoted));
        // A call that can't throw, a binding marked so or `unwrap_unchecked`,
        // has nothing to clean up (ADR 0301): rustc's dataflow of what's there,
        // which says what's dropped, follows no unwinding of it. Of std's
        // `len()`, `is_empty()` and the like, where its MIR says so, as it
        // resolves: not `Range::is_empty` of a `K` whose `PartialOrd` is the
        // crate's, which may panic (#1666).
        let typing_env = ty::TypingEnv::post_analysis(tcx, body.source.def_id());
        for data in body.basic_blocks.as_mut() {
            if let TerminatorKind::Call { func, unwind, .. } = &mut data.terminator_mut().kind
                && let Some((id, args)) = func.const_fn_def()
                && match pure_std(tcx, id) {
                    Some(PureStd::Question) => crate::analysis::std_call_never_unwinds(tcx, typing_env, (id, args)),
                    Some(PureStd::BoxNew | PureStd::Unchecked) => true,
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
            texts_called_again(tcx, &mut body);
            // A block that assumes `UbChecks`, which rust-js reads as off,
            // never runs (ADR 0917).
            for data in body.basic_blocks.as_mut() {
                if crate::std_coverage::assumes_ub_checks(data) {
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
            // else of the block reads: dropped with it. 1.100's derived `Hash`
            // reads one in its block's call, `write_isize(state, discr)`.
            for data in body.basic_blocks.as_mut() {
                data.statements.retain(|statement| {
                    !matches!(&statement.kind, StatementKind::Intrinsic(intrinsic)
                        if matches!(**intrinsic, mir::NonDivergingIntrinsic::Assume(_)))
                });
                let mut read = ReadLocals(Vec::new());
                for statement in &data.statements {
                    Visitor::visit_statement(&mut read, statement, mir::Location::START);
                }
                Visitor::visit_terminator(&mut read, data.terminator(), mir::Location::START);
                data.statements.retain(|statement| {
                    !matches!(&statement.kind, StatementKind::Assign(assign)
                        if matches!(assign.1, Rvalue::Discriminant(_))
                            && assign.0.as_local().is_some_and(|local| !read.0.contains(&local)))
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
            elaborated: None,
            called_once: false,
        }
    }

    /// Of a closure's body rustc infers `Fn` or `FnMut`, which takes
    /// itself by `&` or `&mut`, the body its `call_once` runs, rustc's
    /// `ClosureOnceShim` inlined: itself taken by value, and dropped as it
    /// returns, or as it unwinds, after what its own body drops. A closure
    /// a binding calls once is called so (ADR 1020). `None` where its body
    /// reads itself other than through the reference, as its fields.
    pub fn called_once(&self, tcx: ty::TyCtxt<'tcx>) -> Option<Self> {
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
            elaborated: self.elaborated.clone(),
            called_once: true,
        })
    }
}

/// Of an optimized body, another crate's, each `Box` read through the
/// pointer rustc's `ElaborateBoxDerefs` makes of it, `_t = copy (_b.0:
/// Unique<T>) as *const T (BoxDerefTransmute); (*_t).x = ..`, read as the
/// crate's own body reads it, `(*_b).x`: a box is the value it holds (ADR
/// 0914, amended). The pointer is made just before the one place that reads
/// through it, past the others made for that statement, so nothing changes
/// the box between; one read otherwise is left as it is. And a box copied
/// out of a place just before, `_b = copy (*_r); _t = copy (_b.0 ..)`,
/// which std's `Box` is its pointer's bits of, the same allocation, is read
/// through that place, `&mut (*(*_r))`: a copy of a box of a number is the
/// number in JS, a write through it lost (#1455).
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
    struct Through<'tcx>(Local, Place<'tcx>, ty::TyCtxt<'tcx>);
    impl<'tcx> MutVisitor<'tcx> for Through<'tcx> {
        fn tcx(&self) -> ty::TyCtxt<'tcx> {
            self.2
        }
        fn visit_place(&mut self, place: &mut Place<'tcx>, _: PlaceContext, _: mir::Location) {
            if place.local == self.0 {
                *place = self.1.project_deeper(place.projection, self.2);
            }
        }
    }
    // A statement `_b = copy <place>` of a box, its local and that place.
    let box_copy = |statement: &mir::Statement<'tcx>| match &statement.kind {
        StatementKind::Assign(assign) => match (&assign.0, &assign.1) {
            (copied, Rvalue::Use(Operand::Copy(from), _)) => copied.as_local().map(|copied| (copied, *from)),
            _ => None,
        },
        _ => None,
    };
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
            // Of a box copied out of a place just before, read nowhere else
            // and that place not of the box itself: that place's.
            // A box copied out of a place, `_b = copy (*_r)`, is read through
            // that place, never as the copy, which of a number is the number
            // in JS (#1455): its copy found earlier in the block, where
            // rustc's box elaboration makes each; safe Rust can't replace the
            // box between, its copy borrowed. One whose copy is elsewhere is
            // left as its pointer, which rust-js refuses.
            let defined: Vec<mir::Location> = {
                let mut box_uses = Uses(boxed, Vec::new());
                box_uses.visit_body(body);
                let data = &body.basic_blocks[block];
                (box_uses.1.into_iter())
                    .map(|(location, _)| location)
                    .filter(|location| {
                        location.block == block
                            && (data.statements.get(location.statement_index))
                                .and_then(|statement| statement.kind.as_assign())
                                .is_some_and(|assign| assign.0.as_local() == Some(boxed))
                    })
                    .collect()
            };
            let copied = match defined[..] {
                [copy_at] if copy_at.statement_index < at => {
                    box_copy(&body.basic_blocks[block].statements[copy_at.statement_index])
                        .map(|(_, from)| (copy_at.statement_index, from))
                        .filter(|&(_, from)| from.local != boxed && from.ty(&body.local_decls, tcx).ty.is_box())
                }
                _ => None,
            };
            let copy_of_a_place = copied.is_some()
                || (body.basic_blocks.iter()).any(|data| {
                    (data.statements.iter())
                        .filter_map(&box_copy)
                        .any(|(copied, from)| copied == boxed && from.local != boxed)
                });
            let base = match copied {
                Some((_, from)) => from,
                _ if copy_of_a_place => {
                    at += 1;
                    continue;
                }
                _ => Place::from(boxed),
            };
            let mut through = Through(pointer, base, tcx);
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

/// Of an optimized body, another crate's, each `fmt::Arguments::from_str`
/// its inliner put in it called again, as the crate's own MIR calls it,
/// `_a = Arguments::from_str(<text>)`: what `panic!("..")` and an
/// `assert!`'s message make of a text alone, which the inliner left as the
/// text's pointer's bits, JS having no memory for them (#1569). The
/// statements recognition finds of it (`inlined_text`) go, and the block
/// is split at the call; one it doesn't find is left as it is, refused as
/// its transmute is.
fn texts_called_again<'tcx>(tcx: ty::TyCtxt<'tcx>, body: &mut mir::Body<'tcx>) {
    let mut block = BasicBlock::ZERO;
    while block < body.basic_blocks.next_index() {
        let Some(text) = crate::recognition::inlined_text(tcx, body, block) else {
            block += 1;
            continue;
        };
        let blocks = body.basic_blocks.as_mut();
        let next = blocks.next_index();
        let data = &mut blocks[block];
        let mut at = 0;
        let (before, after): (Vec<_>, Vec<_>) = (data.statements.drain(..))
            .map(|statement| {
                at += 1;
                (at - 1, statement)
            })
            .filter(|(at, _)| !text.statements.contains(at))
            .partition(|&(at, _)| at < text.made_at);
        data.statements = before.into_iter().map(|(_, statement)| statement).collect();
        let after: Vec<mir::Statement<'tcx>> = after.into_iter().map(|(_, statement)| statement).collect();
        let called = mir::Terminator {
            source_info: text.called_at,
            kind: TerminatorKind::Call {
                func: Operand::function_handle(tcx, text.from_str.0, text.from_str.1, text.called_at.span),
                args: Box::new([rustc_span::Spanned {
                    node: text.text,
                    span: text.called_at.span,
                }]),
                destination: text.made,
                target: Some(next),
                unwind: UnwindAction::Unreachable,
                call_source: mir::CallSource::Normal,
                fn_span: text.called_at.span,
            },
            attributes: Default::default(),
        };
        let went_on = data.terminator.replace(called);
        let is_cleanup = data.is_cleanup;
        let mut rest = mir::BasicBlockData::new(went_on, is_cleanup);
        rest.statements = after;
        blocks.push(rest);
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

/// Keeps rustc's body of `local`, its drops elaborated, beside rust-js's
/// own of it, which every compile compares them by: of a function or a
/// closure. rustc's query steals `local`'s built MIR alone, which `mir` is
/// a copy of already; borrowck, which it runs first, only borrows the built
/// MIR of the closures in it. Of a coroutine, an `async` body, whose
/// elaborated body is its state machine's, its locals fields of it: its
/// built MIR, `built`, run through the passes rustc's query runs
/// (`run_analysis_to_runtime_passes`, which a driver may run itself) but
/// the state machine's, where no yield is said, so its drops are rustc's
/// elaboration of the body rust-js lowers (#2177).
pub(crate) fn with_elaborated<'tcx>(
    tcx: TyCtxt<'tcx>,
    local: LocalDefId,
    built: &rustc_data_structures::steal::Steal<mir::Body<'tcx>>,
    mir: &mut Mir<'tcx>,
) {
    if !matches!(tcx.def_kind(local), DefKind::Fn | DefKind::AssocFn | DefKind::Closure) {
        return;
    }
    if tcx.is_coroutine(local.to_def_id()) {
        let mut body = built.borrow().clone();
        if let Some(coroutine) = body.coroutine.as_mut() {
            coroutine.yield_ty = None;
        }
        rustc_mir_transform::run_analysis_to_runtime_passes(tcx, &mut body);
        mir.elaborated = Some(body);
        return;
    }
    let elaborated = tcx.mir_drops_elaborated_and_const_checked(local);
    if !elaborated.is_stolen() {
        mir.elaborated = Some(elaborated.borrow().clone());
    }
}
