//! What a library's contract says of its writes into a writer a piece at a
//! time (#1652, ADR 1135): its types whose formatting dictionaries carry
//! `fmtInto`, the writer functions it frames, the parameters its functions
//! and its traits' methods write so, and the `dyn`s it writes or makes.

use super::{Foreign, key};
use rustc_span::def_id::DefId;

impl Foreign<'_, '_> {
    /// Is `adt` a library's type whose formatting dictionaries its contract
    /// doesn't say carry `fmtInto` (#1652, #1688)? Not `@rust-js/std`'s,
    /// whose dictionaries are made where they're given.
    pub(in crate::lower) fn lacks_fmt_into(&self, adt: DefId) -> bool {
        !adt.is_local()
            && self
                .library(adt.krate)
                .is_some_and(|library| !library.builtin && !library.written_into.contains(&key(self.tcx, adt)))
    }

    /// Is `adt` a library's type whose formatting dictionaries its contract
    /// says carry `fmtInto`, which writes it into a writer a piece at a time
    /// (#1652): one it publishes, or writes so itself; and a std type whose
    /// writer `@rust-js/std` compiles, `Fuse<I>`'s derived `Debug` (#1712).
    pub(in crate::lower) fn lists_fmt_into(&self, adt: DefId) -> bool {
        !adt.is_local()
            && self
                .library(adt.krate)
                .is_some_and(|library| true && library.written_into.contains(&key(self.tcx, adt)))
    }

    /// Does any of the libraries write a value into a writer a piece at a
    /// time: frame a writer, publish a type, or write one it's given so
    /// (#1652)?
    pub(in crate::lower) fn writes_piecewise(&self) -> bool {
        (self.dependencies.libraries.values()).any(|library| {
            !library.builtin
                && (!library.framed_writers.is_empty() || !library.written_into.is_empty() || library.dyn_written_into)
        })
    }

    /// Is `id` a library's writer function, given a `Formatter`, which its
    /// library frames so it writes through the sink a consumer's writer
    /// arms for it, as its contract says (`framed_writers`, #1652): not a
    /// formatting impl's, which `lists_fmt_into` says.
    pub(in crate::lower) fn frames_writer_fn(&self, id: DefId) -> bool {
        !id.is_local()
            && self
                .library(id.krate)
                .is_some_and(|library| !library.builtin && library.framed_writers.contains(&key(self.tcx, id)))
    }

    /// Does any of the libraries write a `dyn` it's given into a writer a
    /// piece at a time (#1652)?
    pub(in crate::lower) fn writes_dyns_given(&self) -> bool {
        (self.dependencies.libraries.values()).any(|library| !library.builtin && library.dyn_written_into)
    }

    /// May any of the libraries make a `dyn` whose dictionary carries no
    /// `fmtInto` (#1652, #1688)?
    pub(in crate::lower) fn makes_fmt_dyns(&self) -> bool {
        (self.dependencies.libraries.values()).any(|library| !library.builtin && library.makes_fmt_dyns)
    }

    /// The type parameters `id`, a library's function, writes into a writer
    /// through their formatting dictionaries, by index, as its manifest says
    /// (#1652): none of another.
    pub(in crate::lower) fn written_marks(&self, id: DefId) -> Vec<u32> {
        let item = (self.library(id.krate)).and_then(|library| library.items.get(&key(self.tcx, id)));
        (item.into_iter().flat_map(|item| item.calling.params.iter()))
            .filter(|param| param.has(crate::library::ParamFact::WrittenInto))
            .map(|param| param.index)
            .collect()
    }

    /// The `fmt::Arguments` parameters of `id`, a library's function, by
    /// index, that it writes into a writer a piece at a time, as its
    /// contract says (`Calling.written_arguments`, #1652).
    pub(in crate::lower) fn written_arguments(&self, id: DefId) -> Vec<u32> {
        (self.library(id.krate))
            .and_then(|library| library.items.get(&key(self.tcx, id)))
            .map(|item| item.calling.written_arguments.clone())
            .or_else(|| {
                (self.library(id.krate))
                    .and_then(|library| library.written_trait_arguments.get(&key(self.tcx, id)).cloned())
            })
            .unwrap_or_default()
    }
}
