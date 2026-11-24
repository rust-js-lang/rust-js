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
use rustc_middle::ty::{self, TyCtxt};
use rustc_span::def_id::DefId;
use rustc_span::{RemapPathScopeComponents, Span};

/// The refusal of a `#[track_caller]` function made a value whose calls the
/// body doesn't say, which says the location its panic names (#2335, ADR
/// 0427 amended): of the crate's own body, at `span`, where a binary
/// writes the location; of a library's, `linked`, its crate's name, where
/// a binary links it. One wording, with what to write.
pub fn refuse_untold(
    tcx: TyCtxt<'_>,
    span: Span,
    untold: &rust_js_program::library::UntoldLocation,
    linked: Option<&str>,
) -> crate::refusal::Refusal {
    let UntoldLocation { value, at, closure } = untold;
    let what = match linked {
        None => format!(
            "a value of `{value}`, a `#[track_caller]` function, called where this function doesn't say how (#2335)"
        ),
        Some(library) => format!(
            "a library whose function makes a value of `{value}`, a `#[track_caller]` function, called where it \
             doesn't say how, `{library}`'s at {at}, in a binary, which writes where a panic was raised (#2335)"
        ),
    };
    let help = format!(
        "where it's called says the location its panic names: give it straight to the function that calls it, or \
         write a closure, `{closure}`, whose line is the location, as in Rust"
    );
    crate::refusal::refuse(tcx, span, crate::refusal::Refused::Instead(&what, &help))
}

/// A binary that links a library whose bodies make a `#[track_caller]`
/// function a value whose calls they don't say, refused, naming each: the
/// library keeps its caller's line, which the binary would write where its
/// panic is raised (#2335, ADR 0427 amended). Of a library at all, however
/// much of it the binary reaches: a value it hands out, a `dyn Fn` or a
/// `fn` pointer, is called where no call of the binary's names it.
pub fn refuse_untold_locations(tcx: TyCtxt<'_>, dependencies: &rust_js_program::library::Dependencies) {
    if !crate::recognition::prints_panic_locations(tcx) {
        return;
    }
    for library in dependencies.libraries.values() {
        for untold in &library.untold_locations {
            let span = tcx.def_span(rustc_span::def_id::CRATE_DEF_ID);
            refuse_untold(tcx, span, untold, Some(&library.name));
        }
    }
}

use rust_js_program::library::UntoldLocation;

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// A `#[track_caller]` function `def_id` made a value at `span` whose
    /// calls this body doesn't say (#2335): as written, `Option::unwrap`,
    /// where, as a location is written, and a closure of its arity to
    /// write instead, `|x| Option::unwrap(x)`.
    pub fn untold_location(&self, def_id: DefId, span: Span) -> UntoldLocation {
        let tcx = self.tcx;
        let value = (tcx.sess.source_map().span_to_snippet(span).ok()).unwrap_or_else(|| tcx.def_path_str(def_id));
        let sig = tcx.fn_sig(def_id).instantiate_identity().skip_normalization();
        let params = match tcx.instantiate_bound_regions_with_erased(sig).inputs().len() {
            1 => "x".to_string(),
            n => (0..n)
                .map(|i| char::from(b'a' + i as u8).to_string())
                .collect::<Vec<_>>()
                .join(", "),
        };
        let at = match &self.location_at(span).kind {
            ExprKind::Str(text) => text.to_string(),
            _ => tcx.sess.source_map().span_to_diagnostic_string(span),
        };
        UntoldLocation {
            closure: format!("|{params}| {value}({params})"),
            value,
            at,
        }
    }
}

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
/// does; or one without a default whose declaration tracks its caller,
/// `Index::index`, as rustc's every impl of it does (#2350): the one
/// question its parameters, its calls and its declaration ask. Not a std
/// method whose model raises at its caller, `Div::div`, `Clone::clone`,
/// whose entry takes it where its impl tracks it and generic code's call
/// gives it where such an impl may reach (`gives_location`, #2457).
pub(crate) fn takes_location(
    tcx: TyCtxt<'_>,
    caller_locations: &IndexSet<rustc_span::def_id::DefId>,
    foreign: &crate::library::Foreign<'_, '_>,
    id: rustc_span::def_id::DefId,
) -> bool {
    caller_locations.contains(&id) || foreign.takes_location(id) || tracked_without_default(tcx, id)
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

    /// The location generic code's call of `id`, a method whose model raises
    /// at its caller, gives its entry, where an instance whose impl tracks
    /// it may reach (`Located::gives_location`, #2457): its caller's, or
    /// `None`; of one given more after it (`location_followed`), `undefined`
    /// in its place.
    pub(crate) fn model_location(&self, (id, args): (DefId, ty::GenericArgsRef<'tcx>), span: Span) -> Option<Expr> {
        if self
            .krate
            .located
            .gives_location(self.tcx, self.frame.owner, (id, args))
        {
            // Where the lowering raises at another's line, a `cloned()`'s
            // row of std's step, that line, as a concrete `RefCell`'s clone
            // takes it (`Expr::raised_at`, #2509, #2522).
            return Some(Expr::raising_at().unwrap_or_else(|| self.caller_location(span)));
        }
        crate::recognition::location_followed(self.tcx, id).then(Expr::js_undefined)
    }

    /// Whether `id` given `args`, made a value, tracks its caller, so is
    /// given the location of the shim it's called through (#2335): a
    /// function that does, or a trait's method whose impl the instances its
    /// callers give may be one that does, generic code's `T::clone` of a
    /// `RefCell`'s (`Located::gives_location`, #2457, #2522).
    pub fn value_tracks_caller(&self, (id, args): (DefId, ty::GenericArgsRef<'tcx>)) -> bool {
        crate::recognition::tracks_caller(self.tcx, id)
            || (self.tcx.trait_of_assoc(id).is_some()
                && self
                    .krate
                    .located
                    .gives_location(self.tcx, self.frame.owner, (id, args)))
    }

    /// Whether an entry of `id` given `args`, a method whose model raises at
    /// its caller, takes its caller's location: where its impl tracks it, or
    /// where the call gives more after it (`location_followed`, #2457).
    pub(crate) fn entry_takes_location(&self, id: DefId, args: ty::GenericArgsRef<'tcx>) -> bool {
        crate::recognition::model_raises_at_caller(self.tcx, id)
            && (crate::recognition::location_followed(self.tcx, id)
                || crate::recognition::impl_raises_at_caller(self.tcx, self.typing_env, id, args))
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
    /// function pointer's or a dictionary's entry's, as rustc's does; a
    /// file of a dependency Cargo fetched under `/cargo/` (`cargo_named`).
    pub fn location_at(&self, span: Span) -> Expr {
        let topmost = span.ctxt().outer_expn().expansion_cause().unwrap_or(span);
        let at = self.tcx.sess.source_map().lookup_char_pos(topmost.lo());
        let file = at
            .file
            .name
            .display(RemapPathScopeComponents::MACRO)
            .to_string_lossy()
            .into_owned();
        let file = rust_js_program::library::registry_sites::cargo_named(std::path::Path::new(&file)).unwrap_or(file);
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
