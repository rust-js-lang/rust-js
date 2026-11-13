//! Whether formatting may print to stdout, the one owner of it (#1490, ADR
//! 1130): Rust's stdout is a `LineWriter`, so what a writer prints while
//! `println!` formats lands inside the line formatted so far, `a inner 1`,
//! and a writer's text written before its print comes before it. Each of
//! the crate's writers, a function given a `Formatter`, may print where
//! what it may run (`may_run`) reaches a body that prints to stdout, or
//! where that can't be followed; those, and every body they may run, are
//! what may run while a line is formatted.
//!
//! And a writer that may panic, in a program that may catch it (#1496,
//! the user's decision): what it wrote before its panic stays in stdout's
//! line, `x half`, which what the program prints after the `catch_unwind`
//! ends, so it writes through the line as one that prints does. Where
//! nothing catches it, the program ends on the panic, and the line it had
//! begun is lost: ADR 0262's decided exception, the guide's difference 57.

use crate::collections::IndexSet;
use crate::lower::Body;
use crate::lower::recognition::Recognition;
use rustc_hir::def::DefKind;
use rustc_middle::thir::visit::{Visitor, walk_expr};
use rustc_middle::thir::{self, ExprKind, Thir};
use rustc_middle::ty;
use rustc_span::def_id::DefId;

use super::may_run::Runs;

/// What `prints` found of a crate.
#[derive(Default)]
pub(in crate::lower) struct Prints {
    /// The writers whose text is written through a sink (`$sink()`), piece
    /// by piece: those that may print to stdout as they format, through the
    /// line they're formatting into; and those whose `Debug` builder's step
    /// or derived `Debug`'s field may give a write's `Err`, `fmt_failures`'
    /// answer, so what they wrote before it is kept, as std's builders write
    /// it (#1557).
    pub(in crate::lower) through_sink: IndexSet<DefId>,
    /// The bodies that may run while a line is formatted: those writers and
    /// what each may run. A print of one is written through stdout's line
    /// (`$out`), and a writer's text it makes for anything but that line,
    /// `format!`'s, is made apart from it (`$formatting`).
    pub(in crate::lower) while_formatting: IndexSet<DefId>,
    /// The crate's types one of whose writers may print.
    pub(in crate::lower) types: IndexSet<DefId>,
    /// Of those, the ones one of whose writers that print may stop
    /// part-way, by a panic or an `Err`, in a program that may catch the
    /// panic and print on (`stops_shown`, #1521, #1530); and where a write
    /// to stderr may run while a line is formatted, the ones whose `Debug`
    /// may print, which a std value's `Debug` writes piece by piece too
    /// (#1490).
    pub(in crate::lower) stopping: IndexSet<DefId>,
    /// Of stdout and of stderr, whether a print of the crate's may leave
    /// the stream's line unended: a `print!` or `eprint!` whose text may not
    /// end with a newline, or a write to the stream's `io::Write`, of any
    /// bytes. A browser's console takes whole lines, so where one may be
    /// left unended a whole line is printed through what the runtime holds
    /// (#1599).
    pub(in crate::lower) unended: [bool; 2],
    /// Of a library's types, those one of whose writers may stop part-way,
    /// by a panic, were a consumer to catch it: what it wrote before is
    /// in Rust's stdout's line, where the library's JS, which catches
    /// nothing, makes its text whole. Its contract's, so a consumer that
    /// catches a panic refuses to show one in a line (#1608, #1530).
    pub(in crate::lower) stopping_where_caught: IndexSet<DefId>,
}

impl Prints {
    /// Does `writer` write its text through a sink, piece by piece: the
    /// line it formats into, where it prints, or a string of its own?
    pub(in crate::lower) fn writes_through_sink(&self, writer: DefId) -> bool {
        self.through_sink.contains(&writer)
    }

    /// May any writer of the crate's print as it formats: what may run in
    /// an `fmt::Arguments` given on, which may hold any value, may (#1565).
    pub(in crate::lower) fn any_writer_prints(&self) -> bool {
        !self.types.is_empty()
    }

    /// May a print of the crate's leave stdout's line, or of `error`,
    /// stderr's, unended?
    pub(in crate::lower) fn leaves_unended(&self, error: bool) -> bool {
        self.unended[usize::from(error)]
    }

    /// May `body` run while a line is formatted?
    pub(in crate::lower) fn runs_while_formatting(&self, body: DefId) -> bool {
        self.while_formatting.contains(&body)
    }

    /// May showing a value of `ty` print as it's formatted: a type of the
    /// crate's one of whose writers may, or one made of one, `Vec<Loud>`;
    /// or a type only a caller, or a `dyn`, says, where any writer may.
    pub(in crate::lower) fn shown_may_print(&self, ty: ty::Ty<'_>, arguments: impl Fn(DefId) -> bool) -> bool {
        !self.types.is_empty()
            && ty
                .walk()
                .filter_map(|part| part.as_type())
                .any(|part| match part.kind() {
                    ty::Adt(adt, _) => self.types.contains(&adt.did()) || arguments(adt.did()),
                    ty::Param(_) | ty::Dynamic(..) | ty::Alias(..) => true,
                    _ => false,
                })
    }

    /// The type of the crate's whose writer may stop part-way, printing or not,
    /// where a panic may be caught, shown in `ty`, `Loud` or one made of
    /// it, `Vec<Loud>`. A `Debug` that shows it among other parts, a std
    /// value's (`stops_composed`, #1521) or a derived `Debug`'s field or a
    /// builder's step (#1495), makes each part's text whole before its own
    /// reaches the line, so a part that stops loses the parts before it,
    /// `Holder { first: !Grid(1), second: !Grid(` where Rust's stdout keeps
    /// them: refused (#1530). One that may print and never stops is shown
    /// as the line has it, each print marked where it ran (#1521).
    pub(in crate::lower) fn stops_shown<'tcx>(&self, ty: ty::Ty<'tcx>) -> Option<ty::Ty<'tcx>> {
        (ty.walk().filter_map(|part| part.as_type()))
            .find(|part| part.ty_adt_def().is_some_and(|adt| self.stopping.contains(&adt.did())))
    }

    /// `stops_shown` of a std value's `Debug`, `Vec<Loud>`'s, `Option<Loud>`'s
    /// or `(Loud, u8)`'s (#1521). Of a type of the crate's, whose writer is
    /// the crate's own, none: its derived `Debug`'s fields are asked where
    /// it shows them (`stops_shown`, #1495).
    pub(in crate::lower) fn stops_composed<'tcx>(&self, ty: ty::Ty<'tcx>) -> Option<ty::Ty<'tcx>> {
        match ty.ty_adt_def().is_some_and(|adt| adt.did().is_local()) {
            true => None,
            false => self.stops_shown(ty),
        }
    }
}

/// The crate's writers that may print to stdout as they format, or that
/// may stop part-way, by a panic, where the crate catches one (#1496), and
/// what may run while one does; and the types of those that may stop
/// part-way, by a panic or an `Err`, where the crate catches a panic
/// (`stopping`, #1521).
///
/// And the writers that write through a sink as they may fail by a
/// builder's step or a derived `Debug`'s field (`through_sink`, #1557).
pub(super) fn prints<'tcx>(
    recognition: &Recognition<'_, 'tcx>,
    all_bodies: &[&Body<'tcx>],
    runs: &Runs<'_, 'tcx>,
    unwinding: (&IndexSet<DefId>, &IndexSet<DefId>),
    library: bool,
) -> Prints {
    let mut found = printing_writers(recognition, all_bodies, runs, unwinding, library);
    for error in [false, true] {
        found.unended[usize::from(error)] = recognition.foreign.leaves_unended(error)
            || all_bodies.iter().any(|body| leaves_unended(recognition, body, error));
    }
    let failing = unwinding.1;
    found.through_sink.extend(
        (all_bodies.iter())
            .filter(|body| failing.contains(&body.def_id) && recognition.formatter_param(body.def_id).is_some())
            .filter(|body| {
                (crate::lower::instances::calls_of(body).into_iter())
                    .any(|(callee, _, _)| crate::lower::recognition::writes_debug_builder(recognition.tcx, callee))
            })
            .map(|body| body.def_id),
    );
    found
}

/// The writers that may print as they format, or stop part-way where a
/// panic may be caught, what may run while one does, and the types of
/// those; `prints`'.
fn printing_writers<'tcx>(
    recognition: &Recognition<'_, 'tcx>,
    all_bodies: &[&Body<'tcx>],
    runs: &Runs<'_, 'tcx>,
    unwinding: (&IndexSet<DefId>, &IndexSet<DefId>),
    library: bool,
) -> Prints {
    let printing: IndexSet<DefId> = (all_bodies.iter())
        .filter(|body| prints_to(recognition, body, false))
        .map(|body| body.def_id)
        .collect();
    let mut found = Prints::default();
    // A library's types the crate's bodies have, of whose writers its
    // contract says they may print, or that a std value's `Debug` shows
    // piece by piece (#1608): as the crate's own types are.
    let foreign = &recognition.foreign;
    let mut foreign_stopping: IndexSet<DefId> = IndexSet::default();
    for body in all_bodies {
        for expr in body.thir.exprs.iter() {
            for adt in (expr.ty.walk().filter_map(|part| part.as_type()))
                .filter_map(|part| part.ty_adt_def().map(|adt| adt.did()))
                .filter(|did| !did.is_local())
            {
                if foreign.prints_shown(adt) {
                    found.types.insert(adt);
                }
                if foreign.stops_shown(adt) {
                    found.stopping.insert(adt);
                }
                if foreign.stops_where_caught(adt) {
                    foreign_stopping.insert(adt);
                }
            }
        }
    }
    let tcx = recognition.tcx;
    // The crate's writers: each function or closure given a `Formatter`,
    // not a `const`'s body.
    let writers: Vec<&Body<'tcx>> = (all_bodies.iter().copied())
        .filter(|body| {
            matches!(
                recognition.tcx.def_kind(body.def_id),
                DefKind::Fn | DefKind::AssocFn | DefKind::Closure
            ) && recognition.formatter_param(body.def_id).is_some()
        })
        .collect();
    let type_of = |writer: DefId| {
        let imp = tcx.impl_of_assoc(writer)?;
        Some(
            tcx.type_of(imp)
                .instantiate_identity()
                .skip_normalization()
                .ty_adt_def()?
                .did(),
        )
    };
    // A library's, for its contract: the types of its writers that may stop
    // part-way, by a panic, were a consumer to catch it (#1608).
    if library {
        let stopping = stopping(
            (all_bodies, &writers),
            runs,
            (unwinding.0, &IndexSet::default()),
            &type_of,
            &foreign_stopping,
        );
        found.stopping_where_caught = (writers.iter())
            .filter(|writer| stopping.contains(&writer.def_id))
            .filter_map(|writer| type_of(writer.def_id))
            .collect();
    }
    let catches = all_bodies.iter().any(|body| catches_unwinding(recognition, body));
    if catches && !foreign_stopping.is_empty() {
        refuse_shown_stopping(recognition, (all_bodies, &writers), runs, &type_of, &foreign_stopping);
    }
    if printing.is_empty() && !catches && found.types.is_empty() {
        return found;
    }
    let mut unfollowed = false;
    // Of those that may stop, the ones that may write into stdout's line,
    // where what they wrote before is kept: in a `format!`, it's dropped
    // with the `String` the panic never gives (#1496).
    let stops = match catches {
        true => {
            let in_a_line = in_a_line(recognition, (all_bodies, &writers), runs, &type_of);
            let mut stops = stopping((all_bodies, &writers), runs, unwinding, &type_of, &foreign_stopping);
            if let Some(in_a_line) = in_a_line {
                stops.retain(|id| in_a_line.contains(id));
            }
            stops
        }
        false => IndexSet::default(),
    };
    for body in &writers {
        let writer = body.def_id;
        // One that may stop part-way, where a `catch_unwind` may catch it,
        // writes through the line as one that prints: what it wrote before
        // its panic stays in the line (#1496).
        let stops = stops.contains(&writer);
        match runs.of(writer) {
            Some(run) => {
                if stops || printing.contains(&writer) || run.iter().any(|id| printing.contains(id)) {
                    found.through_sink.insert(writer);
                    found.while_formatting.insert(writer);
                    found.while_formatting.extend(run);
                }
            }
            // What it may run can't be followed: it may print, and what it
            // runs may be any body that prints.
            None => {
                unfollowed = true;
                found.through_sink.insert(writer);
                found.while_formatting.insert(writer);
            }
        }
    }
    if unfollowed {
        found.while_formatting.extend(printing.iter().copied());
    }
    let own_types: Vec<DefId> = (found.through_sink.iter())
        .filter_map(|&writer| type_of(writer))
        .collect();
    found.types.extend(own_types);
    // The types a `Debug` of which may stop part-way, which a `Debug` that
    // shows them among other parts can't write as the line has it (#1530).
    let own_stopping: Vec<DefId> = (stops.iter())
        .filter(|&&writer| {
            (tcx.trait_impl_of_assoc(writer).map(|imp| tcx.impl_trait_id(imp))).is_some_and(|of| {
                of == crate::lower::recognition::std_item(tcx, crate::lower::recognition::StdItem::Debug)
            })
        })
        .filter_map(|&writer| type_of(writer))
        .collect();
    found.stopping.extend(own_stopping);
    // A writer that shows a value of a type one of whose writers may print,
    // a derived `Debug` of a field's, which std's builders format through a
    // `&dyn Debug` what may run doesn't follow: it may print too, to a fixed
    // point.
    loop {
        let more: Vec<DefId> = (writers.iter())
            .filter(|body| !found.through_sink.contains(&body.def_id))
            .filter(|body| {
                body.thir.exprs.iter().any(|expr| {
                    (expr.ty.walk().filter_map(|part| part.as_type()))
                        .any(|part| part.ty_adt_def().is_some_and(|adt| found.types.contains(&adt.did())))
                })
            })
            .map(|body| body.def_id)
            .collect();
        if more.is_empty() {
            break;
        }
        for writer in more {
            found.through_sink.insert(writer);
            found.while_formatting.insert(writer);
            found.while_formatting.extend(runs.of(writer).unwrap_or_default());
            found.types.extend(type_of(writer));
        }
    }
    // Where a write to stderr may run while a line is formatted, a std
    // value's `Debug` of a type whose `Debug` may print is written piece by
    // piece too, as one that may stop is: what of the line Rust's stdout
    // had written by that write is known only of a writer that writes as it
    // goes (#1490).
    let stderr_while_formatting = (all_bodies.iter())
        .any(|body| found.while_formatting.contains(&body.def_id) && prints_to(recognition, body, true));
    if stderr_while_formatting {
        let debug = crate::lower::recognition::std_item(tcx, crate::lower::recognition::StdItem::Debug);
        let printing_debug = (found.through_sink.iter())
            .filter(|&&writer| (tcx.trait_impl_of_assoc(writer).map(|imp| tcx.impl_trait_id(imp))) == Some(debug))
            .filter_map(|&writer| type_of(writer))
            .collect::<Vec<_>>();
        found.stopping.extend(printing_debug);
    }
    found
}

/// Of the crate's writers, where it catches a panic, those that may stop
/// part-way as they format, by a panic, all but `never_unwinding`'s (ADR
/// 0902) of what their formatting shows, or by an `Err`, `failing`'s (ADR
/// 0187); or that show a value that may (#1496, #1521, #1530). What a body
/// shows is each type its expressions have, of a writer of the crate's
/// that may stop, or of a library's whose contract says its writer may
/// (`foreign_stopping`, #1608), or what can't be known (`unknown_shown`).
/// A writer stops where a body it may run does (`runs`), or where that
/// can't be followed: what may run is asked of the writers alone, as a
/// library asks it of each of them, not of every body (#1608). To a fixed
/// point, as a type stops where a writer of it does.
fn stopping<'tcx>(
    (all_bodies, writers): (&[&Body<'tcx>], &[&Body<'tcx>]),
    runs: &Runs<'_, 'tcx>,
    (never_unwinding, failing): (&IndexSet<DefId>, &IndexSet<DefId>),
    type_of: &dyn Fn(DefId) -> Option<DefId>,
    foreign_stopping: &IndexSet<DefId>,
) -> IndexSet<DefId> {
    // The crate's own: another crate's writer is shown whole (`unknown_part`).
    let all_bodies: Vec<&Body<'tcx>> = (all_bodies.iter().copied())
        .filter(|body| body.def_id.is_local())
        .collect();
    let ran: Vec<(&Body<'tcx>, Option<Vec<DefId>>)> = (writers.iter().copied())
        .filter(|writer| writer.def_id.is_local())
        .map(|writer| (writer, runs.of(writer.def_id)))
        .collect();
    let mut types: IndexSet<DefId> = foreign_stopping.clone();
    loop {
        let direct: IndexSet<DefId> = (all_bodies.iter())
            .filter(|body| {
                let id = body.def_id;
                !never_unwinding.contains(&id) || failing.contains(&id) || shows_what_may_stop(body, &types)
            })
            .map(|body| body.def_id)
            .collect();
        let stops: IndexSet<DefId> = (ran.iter())
            .filter(|(body, run)| {
                direct.contains(&body.def_id) || run.as_ref().is_none_or(|run| run.iter().any(|id| direct.contains(id)))
            })
            .map(|(body, _)| body.def_id)
            .collect();
        let more: IndexSet<DefId> = (writers.iter())
            .filter(|writer| stops.contains(&writer.def_id))
            .filter_map(|writer| type_of(writer.def_id))
            .chain(foreign_stopping.iter().copied())
            .collect();
        if more.len() == types.len() {
            return stops;
        }
        types = more;
    }
}

/// The crate's writers that may write into stdout's line: a type's that a
/// print shows, `println!("{:?}", v)`'s `Vec<Loud>`'s `Loud`'s, of any
/// type there one of its parts; and to a fixed point, those such a writer
/// may run or shows a value of. `None`, all of them, where a print shows,
/// or such a writer shows, what can't be known (`shown_unknown`), or what
/// one may run can't be followed (#1496).
fn in_a_line<'tcx>(
    recognition: &Recognition<'_, 'tcx>,
    (all_bodies, writers): (&[&Body<'tcx>], &[&Body<'tcx>]),
    runs: &Runs<'_, 'tcx>,
    type_of: &dyn Fn(DefId) -> Option<DefId>,
) -> Option<IndexSet<DefId>> {
    let tcx = recognition.tcx;
    let mut shown: Vec<ty::Ty<'tcx>> = Vec::new();
    for body in all_bodies {
        for expr in body.thir.exprs.iter() {
            let ExprKind::Call { fun, ref args, .. } = expr.kind else {
                continue;
            };
            let ty::FnDef(id, generics) = *body.thir[fun].ty.kind() else {
                continue;
            };
            if recognition.writes_stream(id, generics.skip_binder(), false) {
                let mut found = ShownByArguments {
                    thir: &body.thir,
                    tcx,
                    shown: Vec::new(),
                };
                for &arg in args.iter() {
                    found.visit_expr(&body.thir[arg]);
                }
                shown.extend(found.shown);
            }
        }
    }
    let mut types: IndexSet<DefId> = IndexSet::default();
    for ty in &shown {
        if (ty.walk().filter_map(|part| part.as_type())).any(|part| unknown_part(part, true)) {
            return None;
        }
        types.extend(local_parts(*ty));
    }
    let mut line: IndexSet<DefId> = IndexSet::default();
    loop {
        let more: Vec<&Body<'tcx>> = (writers.iter().copied())
            .filter(|writer| !line.contains(&writer.def_id))
            .filter(|writer| type_of(writer.def_id).is_some_and(|of| types.contains(&of)))
            .collect();
        if more.is_empty() {
            return Some(line);
        }
        for writer in more {
            line.insert(writer.def_id);
            // What it runs, a writer it calls among it, and what it shows.
            let ran = runs.of(writer.def_id)?;
            let bodies = ran.iter().filter_map(|&id| runs.body(id)).chain([writer]);
            line.extend(ran.iter().copied());
            for expr in bodies.flat_map(|body| body.thir.exprs.iter()) {
                if unknown_shown(expr) {
                    return None;
                }
                types.extend(local_parts(expr.ty));
            }
        }
    }
}

/// The types each `fmt::rt::Argument` under an expression is made of: what
/// a print shows, `println!("{:?} {}", v, n)`'s `Vec<Loud>` and `u32`.
struct ShownByArguments<'a, 'tcx> {
    thir: &'a Thir<'tcx>,
    tcx: ty::TyCtxt<'tcx>,
    shown: Vec<ty::Ty<'tcx>>,
}

impl<'a, 'tcx> Visitor<'a, 'tcx> for ShownByArguments<'a, 'tcx> {
    fn thir(&self) -> &'a Thir<'tcx> {
        self.thir
    }

    fn visit_expr(&mut self, expr: &'a thir::Expr<'tcx>) {
        if let ExprKind::Call { fun, .. } = expr.kind
            && let ty::FnDef(id, generics) = *self.thir[fun].ty.kind()
            && crate::lower::recognition::is_fmt_argument(self.tcx, id)
        {
            self.shown.extend(generics.skip_binder().types());
        }
        walk_expr(self, expr);
    }
}

/// Whether `expr` shows a value whose writer can't be known: of a type
/// parameter, another crate's type but std's, or a `dyn` read from a place
/// or given by a call, not one made here, as a derived `Debug`'s `&dyn
/// Debug` of a field, which is the field's own type, an expression's too.
fn unknown_shown(expr: &thir::Expr<'_>) -> bool {
    let read = matches!(
        expr.kind,
        ExprKind::Field { .. }
            | ExprKind::Deref { .. }
            | ExprKind::Index { .. }
            | ExprKind::VarRef { .. }
            | ExprKind::UpvarRef { .. }
            | ExprKind::Call { .. }
    );
    (expr.ty.walk().filter_map(|part| part.as_type())).any(|part| unknown_part(part, read))
}

/// Whether `part`'s writer can't be known: a type parameter's, or, where
/// `dyn_unknown`, a `dyn`'s. Another crate's writer is shown whole, its
/// text made before the line gets it, as a library's that prints is
/// (#1490), so it's taken not to stop, but where its contract says it may
/// (`foreign_stopping`, #1608).
fn unknown_part(part: ty::Ty<'_>, dyn_unknown: bool) -> bool {
    match part.kind() {
        ty::Dynamic(..) => dyn_unknown,
        ty::Param(_) | ty::Alias(..) => true,
        _ => false,
    }
}

/// The crate's own types `ty` is made of, itself among them.
fn local_parts(ty: ty::Ty<'_>) -> impl Iterator<Item = DefId> + '_ {
    (ty.walk().filter_map(|part| part.as_type()))
        .filter_map(|part| part.ty_adt_def().map(|adt| adt.did()))
        .filter(|did| did.is_local())
}

/// Where the crate catches a panic, each print to stdout that shows a
/// value of a library's type one of whose writers may stop part-way by a
/// panic (`foreign_stopping`), or of what can't be known where the crate
/// has one; and each writer of the crate's that may write into stdout's
/// line (`in_a_line`) and has one: refused. Rust's stdout keeps what that
/// writer wrote before it stopped, where its library's JS, which catches
/// nothing, makes its text whole, `x ` where native's line is `x [!L1, !`
/// (#1608, #1530).
fn refuse_shown_stopping<'tcx>(
    recognition: &Recognition<'_, 'tcx>,
    (all_bodies, writers): (&[&Body<'tcx>], &[&Body<'tcx>]),
    runs: &Runs<'_, 'tcx>,
    type_of: &dyn Fn(DefId) -> Option<DefId>,
    foreign_stopping: &IndexSet<DefId>,
) {
    let tcx = recognition.tcx;
    let stopping_part = |ty: ty::Ty<'tcx>| {
        (ty.walk().filter_map(|part| part.as_type())).find(|part| {
            part.ty_adt_def()
                .is_some_and(|adt| foreign_stopping.contains(&adt.did()))
        })
    };
    let refuse = |span, shown: String| {
        let what = format!("showing {shown} in stdout's line where a `catch_unwind` may catch a panic");
        let why = "its writer, of another crate, may stop part-way, and Rust's stdout keeps what it wrote before it stopped, where its crate's JS makes its text whole: show it in a `format!` first, whose text a panic drops, or where no `catch_unwind` catches (#1530)";
        crate::lower::refusal::refuse(tcx, span, crate::lower::refusal::Refused::Because(&what, why));
    };
    let own: Vec<&Body<'tcx>> = (all_bodies.iter().copied())
        .filter(|body| body.def_id.is_local())
        .collect();
    let any = foreign_stopping
        .first()
        .map(|&adt| tcx.def_path_str(adt))
        .unwrap_or_default();
    for body in &own {
        for expr in body.thir.exprs.iter() {
            let ExprKind::Call { fun, ref args, .. } = expr.kind else {
                continue;
            };
            let ty::FnDef(id, generics) = *body.thir[fun].ty.kind() else {
                continue;
            };
            if !recognition.writes_stream(id, generics.skip_binder(), false) {
                continue;
            }
            let mut found = ShownByArguments {
                thir: &body.thir,
                tcx,
                shown: Vec::new(),
            };
            for &arg in args.iter() {
                found.visit_expr(&body.thir[arg]);
            }
            for ty in found.shown {
                if let Some(part) = stopping_part(ty) {
                    refuse(expr.span, format!("a `{part}`"));
                } else if (ty.walk().filter_map(|part| part.as_type())).any(|part| unknown_part(part, true)) {
                    refuse(expr.span, format!("a `{ty}`, which may be a `{any}`,"));
                }
            }
        }
    }
    let line = in_a_line(recognition, (all_bodies, writers), runs, type_of);
    for writer in writers.iter().filter(|writer| writer.def_id.is_local()) {
        if line.as_ref().is_some_and(|line| !line.contains(&writer.def_id)) {
            continue;
        }
        if let Some((expr, part)) = (writer.thir.exprs.iter()).find_map(|expr| Some((expr, stopping_part(expr.ty)?))) {
            refuse(expr.span, format!("a `{part}`"));
        }
    }
}

/// Whether `body` shows a value that may stop part-way as it's shown: of
/// a type among `types`, or of what can't be known (`unknown_shown`).
fn shows_what_may_stop(body: &Body<'_>, types: &IndexSet<DefId>) -> bool {
    body.thir.exprs.iter().any(|expr| {
        (expr.ty.walk().filter_map(|part| part.as_type()))
            .any(|part| part.ty_adt_def().is_some_and(|adt| types.contains(&adt.did())))
            || unknown_shown(expr)
    })
}

/// Whether `body` calls `catch_unwind`, past which a program a panic
/// stopped may print on (#1496, #1521).
fn catches_unwinding<'tcx>(recognition: &Recognition<'_, 'tcx>, body: &Body<'tcx>) -> bool {
    body.thir.exprs.iter().any(|expr| {
        let ExprKind::Call { fun, .. } = expr.kind else {
            return false;
        };
        let ty::FnDef(id, _) = *body.thir[fun].ty.kind() else {
            return false;
        };
        crate::lower::recognition::unwind_op(recognition.tcx, id)
            == Some(crate::lower::std_types::unwind::UnwindOp::Catch)
    })
}

/// Whether `body` prints to stdout itself, or of `error`, to stderr:
/// `print!` or `eprint!` and their lines, or a write to `io::stdout()` or
/// `io::stderr()`.
fn prints_to<'tcx>(recognition: &Recognition<'_, 'tcx>, body: &Body<'tcx>, error: bool) -> bool {
    (crate::lower::instances::calls_of(body).into_iter())
        .any(|(id, args, _)| recognition.writes_stream(id, args, error))
}

/// Whether `body` may leave stdout's line, or of `error`, stderr's,
/// unended: a `print!`'s or `eprint!`'s whose text's last piece isn't text
/// ending with a newline, or can't be told; or a write to the stream's
/// `io::Write`, which writes any bytes (#1599).
fn leaves_unended<'tcx>(recognition: &Recognition<'_, 'tcx>, body: &Body<'tcx>, error: bool) -> bool {
    let tcx = recognition.tcx;
    let thir = &body.thir;
    let made: crate::collections::IndexMap<thir::ExprId, Vec<crate::lower::format_args::Piece>> =
        (super::debug::format_calls(tcx, thir).into_iter())
            .map(|(call, pieces, _)| (call, pieces))
            .collect();
    let ends_its_line = |arg: thir::ExprId| {
        let mut at = arg;
        loop {
            at = crate::lower::strip(thir, at);
            match thir[at].kind {
                ExprKind::Borrow { arg, .. } | ExprKind::Deref { arg } => at = arg,
                // `format_args!`'s block, `{ let args = ..; Arguments::new(..) }`.
                ExprKind::Block { block } => match thir[block].expr {
                    Some(value) => at = value,
                    None => break,
                },
                _ => break,
            }
        }
        // A text alone, `println!("done")`'s, `Arguments::from_str("done\n")`.
        if let ExprKind::Call { fun, ref args, .. } = thir[at].kind
            && let ty::FnDef(id, _) = *thir[fun].ty.kind()
            && crate::lower::recognition::is_arguments_from_str(tcx, id)
            && let [text] = args[..]
        {
            let mut text = text;
            loop {
                text = crate::lower::strip(thir, text);
                match thir[text].kind {
                    ExprKind::Borrow { arg, .. } | ExprKind::Deref { arg } => text = arg,
                    _ => break,
                }
            }
            return matches!(thir[text].kind, ExprKind::Literal { lit, .. }
                if matches!(lit.node, rustc_ast::LitKind::Str(s, _) if s.as_str().ends_with('\n')));
        }
        made.get(&at).is_some_and(|pieces| {
            matches!(pieces.last(), Some(crate::lower::format_args::Piece::Text(text)) if text.ends_with('\n'))
        })
    };
    thir.exprs.iter().any(|expr| {
        let ExprKind::Call { fun, ref args, .. } = expr.kind else {
            return false;
        };
        let ty::FnDef(id, generics) = *thir[fun].ty.kind() else {
            return false;
        };
        match crate::lower::recognition::print_of(tcx, id) {
            Some(stream) => stream == error && !args.first().is_some_and(|&arg| ends_its_line(arg)),
            None => recognition.writes_stream(id, generics.skip_binder(), error),
        }
    })
}
