//! `#[track_caller]` and `panic::Location` (ADR 0427, amended by #2292):
//! a location is the text `file:line:column`, where rustc's
//! `span_as_caller_location` says a call is, each panic's as each
//! `Location::caller()`'s, read from its end by the runtime's `location.js`. A function that tracks its caller is given its caller's,
//! `location`, before its evidence; `Location::caller()` is that, in its own
//! body, or where it's written anywhere else, a closure in it too.

use crate::calls::Call;
use crate::recognition::{MethodOf, Std, StdPath, std_inherent_method};
use crate::{FnCx, R};
use rust_js_program::collections::IndexSet;
use rust_js_program::js::{Expr, ExprKind, Op};
use rustc_middle::ty::TyCtxt;
use rustc_span::def_id::DefId;
use rustc_span::{RemapPathScopeComponents, Span};

/// What's asked of a `Location`.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum LocationOp {
    /// `Location::caller()`: where this function was called, if it tracks
    /// its caller, or where it's written.
    Caller,
    /// `l.file()`, `l.line()`, `l.column()`: its parts.
    File,
    Line,
    Column,
}

/// Which of `Location`'s functions `id` is.
pub(crate) fn location_op(tcx: TyCtxt<'_>, id: DefId) -> Option<LocationOp> {
    let (MethodOf::Type(StdPath::PanicLocation), name) = std_inherent_method(tcx, id)? else {
        return None;
    };
    Some(match name.as_str() {
        "caller" => LocationOp::Caller,
        "file" => LocationOp::File,
        "line" => LocationOp::Line,
        "column" => LocationOp::Column,
        _ => return None,
    })
}

/// Whether `id` takes its caller's location, the crate's, which
/// `caller_locations` says (ADR 0427), or a library's, as its manifest
/// does; or, in every crate alike, as one crate's generic call is given
/// another's dictionary, a trait method of std's whose impls raise their
/// panic at their caller, `Div::div`, `Ord::clamp`, `SliceIndex::index`
/// (`std_raises_at_caller`, #2324), or one without a default whose
/// declaration tracks its caller, `Index::index`, as rustc's every impl of
/// it does (#2350): the one question its parameters, its calls and its
/// declaration ask.
pub(crate) fn takes_location(
    tcx: TyCtxt<'_>,
    caller_locations: &IndexSet<rustc_span::def_id::DefId>,
    foreign: &crate::library::Foreign<'_, '_>,
    id: rustc_span::def_id::DefId,
) -> bool {
    caller_locations.contains(&id)
        || foreign.takes_location(id)
        || crate::recognition::std_raises_at_caller(tcx, id)
        || tracked_without_default(tcx, id)
}

/// Whether `id` is a trait's method without a default whose declaration
/// tracks its caller, which no crate's manifest need say (#2350).
pub(crate) fn tracked_without_default(tcx: TyCtxt<'_>, id: DefId) -> bool {
    tcx.trait_of_assoc(id).is_some()
        && matches!(tcx.def_kind(id), rustc_hir::def::DefKind::AssocFn)
        && !tcx.defaultness(id).has_value()
        && crate::recognition::tracks_caller(tcx, id)
}

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// A `Location`'s function: `None` if `known` is another.
    pub(crate) fn location_call(
        &mut self,
        known: Std,
        call: Call<'_, 'tcx>,
        values: &mut std::vec::IntoIter<Expr>,
    ) -> Option<Expr> {
        let Std::Location(op) = known else {
            return None;
        };
        let mut part =
            |helper: &'static str| Expr::call(Expr::helper(helper), vec![values.next().expect("a location")]);
        Some(match op {
            LocationOp::Caller => self.caller_location(call.span),
            LocationOp::File => part("$locationFile"),
            LocationOp::Line => part("$locationLine"),
            LocationOp::Column => part("$locationColumn"),
        })
    }

    /// Whether `id` takes its caller's location, the crate's or a
    /// library's (`takes_location`).
    // helper: delegates to its owner's answer (#2172)
    pub(crate) fn takes_caller_location(&self, id: rustc_span::def_id::DefId) -> bool {
        takes_location(self.tcx, self.krate.caller_locations, self.krate.foreign, id)
    }

    /// The location a call at `span` gives, or `Location::caller()` there
    /// is: in the body of a function that tracks its caller, the one it was
    /// given; anywhere else, `span`'s own (`location_at`).
    pub fn caller_location(&self, span: Span) -> Expr {
        (self.value_location())
            .or_else(|| self.given_location())
            .unwrap_or_else(|| self.location_at(span))
    }

    /// Where `span` is, as `location_at` says it, but of std's own files,
    /// `core`'s or `alloc`'s, which this compile reads from the toolchain's
    /// `rust-src` where this machine keeps it: by the name the shipped std
    /// gives them, `/rustc/<commit>/library/..`, as native code's location of
    /// one is, the toolchain's root the runtime's `$stdSites.source`, and so
    /// the same JS on every machine (#2335, #2343). A shim of std's,
    /// `FnMut::call_mut`'s, or a function of std's own, made a `fn` pointer.
    pub fn location_as_shipped(&self, span: Span) -> Expr {
        let topmost = span.ctxt().outer_expn().expansion_cause().unwrap_or(span);
        let at = self.tcx.sess.source_map().lookup_char_pos(topmost.lo());
        let std = crate::recognition::std_crate(self.tcx, at.file.cnum).is_some();
        let shown = self.location_at(span);
        let (true, ExprKind::Str(text)) = (std, &shown.kind) else {
            return shown;
        };
        let Some(from) = text.find("/library/") else {
            return shown;
        };
        Expr::bin(
            Op::Add,
            Expr::member(Expr::helper("$stdSites"), "source"),
            Expr::str(&text[from..]),
        )
    }

    /// Where `span` is, as rustc places a call, at the outermost macro's
    /// call, its file as rustc names it: a call's, or a definition's, which
    /// a shim of a function that takes its caller's location gives it, a
    /// function pointer's or a dictionary's entry's, as rustc's does.
    pub fn location_at(&self, span: Span) -> Expr {
        let topmost = span.ctxt().outer_expn().expansion_cause().unwrap_or(span);
        let at = self.tcx.sess.source_map().lookup_char_pos(topmost.lo());
        let file = at
            .file
            .name
            .display(RemapPathScopeComponents::MACRO)
            .to_string_lossy()
            .into_owned();
        Expr::str(format!("{file}:{}:{}", at.line, at.col_display + 1))
    }
}

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// `{}` of a `Location`, `file:line:column`, its text itself, or `{:?}`,
    /// `Location { file: "..", line: .., column: .. }`, as std shows it:
    /// of one written where it's shown, `Location::caller()`'s, written so
    /// as it compiles; of another, its text read by `$locationDebug`.
    pub(crate) fn location_shown(&mut self, value: Expr, debug: bool, _span: Span) -> R<Expr> {
        if !debug {
            return Ok(value);
        }
        Ok(match &value.kind {
            ExprKind::Str(text) => {
                let column = text.rfind(':').expect("a location's column");
                let line = text[..column].rfind(':').expect("a location's line");
                let file = &text[..line];
                Expr::str(format!(
                    "Location {{ file: {file:?}, line: {}, column: {} }}",
                    &text[line + 1..column],
                    &text[column + 1..]
                ))
            }
            _ => Expr::call(Expr::helper("$locationDebug"), vec![value]),
        })
    }
}
