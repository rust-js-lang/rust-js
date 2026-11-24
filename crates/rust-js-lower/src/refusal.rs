//! What rust-js refuses, and the one way it says so (ADRs 0262, 0390):
//! "rust-js does not support .. yet", of a feature, a call or a type; and
//! of addressable memory, which JavaScript has none of, that, with why.

use super::recognition::{OpenCall, addresses_memory, is_memory_type, open_call, open_item, open_type};
use rustc_hir::def_id::DefId;
use rustc_middle::ty::{Ty, TyCtxt};
use rustc_span::{ErrorGuaranteed, Span};

/// What's refused.
#[derive(Clone, Copy)]
pub enum Refused<'a, 'tcx> {
    /// A feature, as its words say: "a `&mut` to a union's field".
    Feature(&'a str),
    /// A call of a function: "calling `std::env::args`".
    Call(DefId),
    /// Values, constants or statics of a type: "values of type `*const u8`".
    Type(&'a str, Ty<'tcx>),
    /// An impl of a trait: "user implementations of `std::alloc::Allocator`".
    Implementing(DefId),
    /// A feature, and why it's refused: "a component's `Clone`", "React
    /// gives a component no dictionary".
    Because(&'a str, &'a str),
    /// A feature, and what to write instead, its help: "a `#[track_caller]`
    /// function as a value ..", "call it in a closure, ..".
    Instead(&'a str, &'a str),
    /// A feature of addressable memory, as its words say: "a raw pointer to a value".
    Memory(&'a str),
    /// What's refused, said as it is, there only because a crossing
    /// doesn't convert a form or a fact yet: a stopgap of Campaign 6's
    /// (ADR 1201), which its cell's issue lifts.
    Unconverted(Crossing, &'a Refused<'a, 'tcx>),
}

/// A cell of Campaign 6's matrix (#2539, ADR 1201): a form or a fact, and
/// the crossing into or out of generic code it isn't converted at yet. A
/// refusal of one is a stopgap of idiomatic Rust whose behavior JS can
/// keep; the campaign ends when no refusal names one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Crossing {
    /// A value JS holds as no object, inside a generic struct or type the
    /// body holds in a place, given as it is (#2255).
    PlacedArgument,
    /// One also given as another iterator's items (#2534), or stepped as
    /// them through their own bound, `S::Item: Iterator` (#2541).
    PlacedItem,
    /// A `RefCell` guard's `&mut` given where generic code may write it whole.
    GuardArgument,
    /// A `&mut` object inside the type a generic impl is for, given by its
    /// dictionary (#2532).
    BoxDictionary,
    /// A `&mut` of a `&mut` of an object given to generic code.
    HeldBoxArgument,
    /// A `&mut` nested in a generic function's parameters, result or field.
    NestedBox,
    /// A `&mut` of a number given, given back or kept on the MIR path, or a
    /// parameter both called and written through.
    BoxOnMir,
    /// A borrow in generic code of where an associated type's object is stored.
    LentDictionary,
    /// A borrow given to generic code that may let it out as a raw pointer,
    /// of JS's copy.
    HandleArgument,
    /// A write's result given where a bound reads a `Result` of type parameters.
    TaggedArgument,
    /// A write's result held inside what generic code has as a `Result` of
    /// its type parameters, a `Vec<Result<T, E>>`'s, given or given back.
    TaggedWithin,
    /// Write results as the items of an iterator given to generic code that
    /// reads them as a `Result` of its type parameters (#2560).
    TaggedItem,
    /// A `TrustedRandomAccess` or `TrustedLen` fact std's zip or `eq` is
    /// picked by, at any crossing.
    TrustedLen,
    /// The hint-knowing fact and the items' size of an `extend` generic code makes.
    HintArgument,
    /// The hint of a `dyn Iterator` taken from a library or a binding.
    HintBoundary,
    /// A type's fact given to generic code.
    FactArgument,
    /// A fact a caller's dictionary doesn't carry.
    FactDictionary,
    /// A fact a library's contract can't say: a type nested past its
    /// templates, or a layout.
    FactBoundary,
    /// A type's or a const's fact only a caller knows, of a type or a
    /// const parameter: its name, size, discriminant, a map's class, a
    /// sort's layout, a length (Campaign 6's step 7).
    TypeFact,
}

impl Crossing {
    /// The open issue whose fix lifts every refusal of the cell: the one
    /// place each is mapped, which test/architecture-stopgaps.test.ts and
    /// scripts/stopgap-refusals.ts read (ADR 1201).
    pub fn lifted_by(self) -> u32 {
        match self {
            Crossing::PlacedArgument => 2549,
            Crossing::PlacedItem => 2535,
            Crossing::GuardArgument => 2475,
            Crossing::BoxDictionary => 2550,
            Crossing::HeldBoxArgument => 1632,
            Crossing::NestedBox => 1421,
            Crossing::BoxOnMir => 2551,
            Crossing::LentDictionary => 1295,
            Crossing::HandleArgument => 2552,
            Crossing::TaggedArgument => 1416,
            Crossing::TaggedWithin => 2568,
            Crossing::TaggedItem => 2566,
            Crossing::TrustedLen => 2546,
            Crossing::HintArgument => 2553,
            Crossing::HintBoundary => 1562,
            Crossing::FactArgument => 2554,
            Crossing::FactDictionary => 2555,
            Crossing::FactBoundary => 2556,
            Crossing::TypeFact => 2570,
        }
    }
}

/// Why lowering stopped, a value the pipeline reads (ADR 1148, amended):
/// an error emitted, which rustc counts. Made here alone.
#[derive(Clone, Copy, Debug)]
pub enum Refusal {
    Emitted(ErrorGuaranteed),
}

/// The error `message` at `span`, emitted, of what isn't a refusal of a
/// feature, `refuse`'s: a binding used as it can't be, a component's JSX.
pub fn error(tcx: TyCtxt<'_>, span: Span, message: impl Into<String>) -> Refusal {
    Refusal::Emitted(tcx.dcx().span_err(span, message.into()))
}

/// An error rustc emitted already, resolving an instance: lowering stops
/// there too.
pub(crate) fn emitted(error: ErrorGuaranteed) -> Refusal {
    Refusal::Emitted(error)
}

/// The error that `refused` isn't supported, in its `words`: of
/// addressable memory, with why.
pub fn refuse<'tcx>(tcx: TyCtxt<'tcx>, span: Span, refused: Refused<'_, 'tcx>) -> Refusal {
    // A stopgap is said as what it refuses is.
    if let Refused::Unconverted(_, said) = refused {
        return refuse(tcx, span, *said);
    }
    let mut error = tcx.dcx().struct_span_err(span, words(tcx, refused));
    if of_memory(tcx, refused) {
        error = error.with_note(MEMORY);
    }
    if let Refused::Instead(_, write) = refused {
        error = error.with_help(write.to_string());
    }
    // Of a call into, or an impl of a trait of, a crate rust-js couldn't
    // compile, why (#1420).
    if let Refused::Call(id) | Refused::Implementing(id) = refused
        && let Some(why) = uncompiled(tcx, id)
    {
        error = error.with_note(format!(
            "its crate is its types alone: rust-js couldn't compile it (ADR 0415): {why}"
        ));
    }
    Refusal::Emitted(error.emit_err())
}

/// Why rust-js didn't compile the crate of `id`, a crate of crates.io's it
/// refused, which rustc checked, its types alone (ADR 0415), as written
/// beside its metadata.
pub(crate) fn uncompiled(tcx: TyCtxt<'_>, id: DefId) -> Option<String> {
    if id.is_local() {
        return None;
    }
    let metadata = tcx.used_crate_source(id.krate).rmeta.clone()?;
    std::fs::read_to_string(rust_js_program::library::refusal(&metadata)).ok()
}

/// Why addressable memory is refused.
const MEMORY: &str = "JavaScript has no memory to address: a raw pointer, `transmute` and `std::ptr` \
                      read or write bytes by where they are (ADR 0390)";

/// What a refusal says: "rust-js does not support .. yet", and of
/// addressable memory, "rust-js does not support addressable memory yet: ..".
pub(crate) fn words<'tcx>(tcx: TyCtxt<'tcx>, refused: Refused<'_, 'tcx>) -> String {
    let what = match refused {
        Refused::Feature(what) | Refused::Because(what, _) | Refused::Instead(what, _) | Refused::Memory(what) => {
            what.to_string()
        }
        Refused::Call(id) => calling(tcx, id),
        Refused::Implementing(id) => implementing(tcx, id),
        // A stopgap is said as what it refuses is.
        Refused::Unconverted(_, said) => return words(tcx, *said),
        Refused::Type(what, ty) => format!(
            "{what} of type `{ty}`{}",
            open_type(tcx, ty).map(issue_of).unwrap_or_default()
        ),
    };
    match (of_memory(tcx, refused), refused) {
        (true, _) => format!("rust-js does not support addressable memory yet: {what}"),
        (false, Refused::Because(_, why)) => format!("rust-js does not support {what} yet: {why}"),
        (false, _) => format!("rust-js does not support {what} yet"),
    }
}

/// Whether what's refused is of addressable memory: a call of std's that
/// reads or writes it, a type with a raw pointer or a union in it, or a
/// feature said to be.
fn of_memory<'tcx>(tcx: TyCtxt<'tcx>, refused: Refused<'_, 'tcx>) -> bool {
    match refused {
        Refused::Call(id) => addresses_memory(tcx, id),
        Refused::Type(_, ty) => is_memory_type(tcx, ty),
        Refused::Memory(_) => true,
        Refused::Unconverted(_, said) => of_memory(tcx, *said),
        Refused::Feature(_) | Refused::Because(..) | Refused::Instead(..) | Refused::Implementing(_) => false,
    }
}

/// What a refused call is called: "calling `std::env::args`", and where
/// an open issue covers it, that issue, "calling `std::hint::spin_loop`
/// (#1285)", as a refusal names what's left to do (guide limitations.md).
pub fn calling(tcx: TyCtxt<'_>, id: DefId) -> String {
    let issue = open_call(tcx, id).map(issue_of);
    format!("calling `{}`{}", tcx.def_path_str(id), issue.unwrap_or_default())
}

/// What a refused impl is called: "user implementations of
/// `std::alloc::Allocator`", and where an open issue covers it, that issue.
fn implementing(tcx: TyCtxt<'_>, trait_id: DefId) -> String {
    let issue = open_item(tcx, trait_id).map(issue_of);
    format!(
        "user implementations of `{}`{}",
        tcx.def_path_str(trait_id),
        issue.unwrap_or_default()
    )
}

/// What a missing dictionary is called: "implementation evidence for
/// `<T as Trait>`".
pub(crate) fn evidence(tr: rustc_middle::ty::TraitRef<'_>) -> String {
    format!("implementation evidence for `{tr}`")
}

/// The issue or decision an open family of refusals names, ` (#1452)`.
pub(crate) fn issue_of(open: OpenCall) -> &'static str {
    match open {
        OpenCall::Conversion => " (#1452)",
        OpenCall::Thread => " (ADR 1126)",
        OpenCall::BuildHasher => " (#1482)",
        OpenCall::NewIn1_100 => " (#1656)",
        OpenCall::SerdeJsonBytes => " (#1626)",
        OpenCall::SerdeJsonErrorApi => " (#1627)",
        OpenCall::NetText => " (#1644)",
    }
}
