//! `#[track_caller]` and `panic::Location` (ADR 0427): a location is
//! `{ file, line, column }`, where rustc's `span_as_caller_location` says a
//! call is. A function that tracks its caller is given its caller's,
//! `location`, before its evidence; `Location::caller()` is that, in its own
//! body, or where it's written anywhere else, a closure in it too.

use crate::calls::Call;
use crate::display::join;
use crate::recognition::{MethodOf, Std, StdPath, std_inherent_method};
use crate::{FnCx, R};
use rust_js_program::collections::IndexSet;
use rust_js_program::js::{Expr, ExprKind, Prop};
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

/// A location's parts, as its object has them, each with whether it's
/// text, `file`, or a number, `line` and `column`: `{ file, line, column }`.
/// The one list of what's made and what's declared (#560).
pub(crate) const PARTS: [(&str, bool); 3] = [("file", true), ("line", false), ("column", false)];

/// Whether `id` takes its caller's location, the crate's, which
/// `caller_locations` says (ADR 0427), or a library's, as its manifest
/// does: the one question its parameters, its calls and its declaration
/// ask.
pub(crate) fn takes_location(
    caller_locations: &IndexSet<rustc_span::def_id::DefId>,
    foreign: &crate::library::Foreign<'_, '_>,
    id: rustc_span::def_id::DefId,
) -> bool {
    caller_locations.contains(&id) || foreign.takes_location(id)
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
        let mut part = |name: &str| Expr::member(values.next().expect("a location"), name);
        Some(match op {
            LocationOp::Caller => self.caller_location(call.span),
            LocationOp::File => part("file"),
            LocationOp::Line => part("line"),
            LocationOp::Column => part("column"),
        })
    }

    /// Whether `id` takes its caller's location, the crate's or a
    /// library's (`takes_location`).
    // helper: delegates to its owner's answer (#2172)
    pub(crate) fn takes_caller_location(&self, id: rustc_span::def_id::DefId) -> bool {
        takes_location(self.krate.caller_locations, self.krate.foreign, id)
    }

    /// The location a call at `span` gives, or `Location::caller()` there
    /// is: in the body of a function that tracks its caller, the one it was
    /// given; anywhere else, `span`'s own (`location_at`).
    pub(crate) fn caller_location(&self, span: Span) -> Expr {
        self.given_location().unwrap_or_else(|| self.location_at(span))
    }

    /// Where `span` is, as rustc places a call, at the outermost macro's
    /// call, its file as rustc names it: a call's, or a definition's, which
    /// a shim of a function that takes its caller's location gives it, a
    /// function pointer's or a dictionary's entry's, as rustc's does.
    pub(crate) fn location_at(&self, span: Span) -> Expr {
        let topmost = span.ctxt().outer_expn().expansion_cause().unwrap_or(span);
        let at = self.tcx.sess.source_map().lookup_char_pos(topmost.lo());
        let file = at
            .file
            .name
            .display(RemapPathScopeComponents::MACRO)
            .to_string_lossy()
            .into_owned();
        let [(file_key, _), (line_key, _), (column_key, _)] = PARTS;
        Expr::object(vec![
            Prop::Field(file_key.into(), Expr::str(file)),
            Prop::Field(line_key.into(), Expr::int(at.line as i128)),
            Prop::Field(column_key.into(), Expr::int(at.col_display as i128 + 1)),
        ])
    }
}

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// `{}` of a `Location`, `file:line:column`, or `{:?}`, `Location {
    /// file: "..", line: .., column: .. }`, as std shows it: of one written
    /// where it's shown, `Location::caller()`'s, the text itself; of one
    /// read again, its parts; of one made where it's shown, a call's, its
    /// parts of it given a name, `((location) => ..)(at())`.
    pub(crate) fn location_shown(&mut self, value: Expr, debug: bool, span: Span) -> R<Expr> {
        if !value.reads_same() && !matches!(value.kind, ExprKind::Object(..)) {
            let location = rust_js_program::js::Name::declared("location".to_string());
            let shown = self.location_shown(Expr::var(&location), debug, span)?;
            let shown = Expr::arrow(
                vec![location.into()],
                vec![rust_js_program::js::StmtKind::Return(Some(shown)).at(rust_js_program::js::Span::NONE)],
            );
            return Ok(self.applied(shown, value));
        }
        let part = |name: &str| match &value.kind {
            ExprKind::Object(props, ..) => props.iter().find_map(|prop| match prop {
                Prop::Field(field, part) if field == name => Some(part.clone()),
                _ => None,
            }),
            _ if value.reads_same() => Some(Expr::member(value.clone(), name)),
            _ => None,
        };
        let (Some(file), Some(line), Some(column)) = (part("file"), part("line"), part("column")) else {
            return Err(self.unsupported(span, "showing this `Location`"));
        };
        Ok(match debug {
            false => join(vec![file, Expr::str(":"), line, Expr::str(":"), column]),
            true => {
                let file = match &file.kind {
                    ExprKind::Str(text) => Expr::str(format!("{text:?}")),
                    _ => Expr::call(Expr::helper("$debugStr"), vec![file]),
                };
                join(vec![
                    Expr::str("Location { file: "),
                    file,
                    Expr::str(", line: "),
                    line,
                    Expr::str(", column: "),
                    column,
                    Expr::str(" }"),
                ])
            }
        })
    }
}
