//! A path's own methods (ADR 0405): what reads it, its runtime helper of
//! `path.js`, which goes over its components as Rust's `Components` does
//! on Unix. A path is its text (ADR 0173); a `PathBuf` changed in place,
//! `push` and the rest, is its text's edit, `StringEdit`'s.

use crate::js::Expr;
use crate::lower::FnCx;
use crate::lower::recognition::Std;

/// What reads a path.
#[derive(Clone, Copy, PartialEq, Debug)]
pub(in crate::lower) enum PathOp {
    /// `base.join(path)`: `path` itself if it's absolute.
    Join,
    /// `p.parent()`, `p.file_name()`, `p.file_stem()`, `p.extension()`:
    /// `None` where there's none.
    Parent,
    FileName,
    FileStem,
    Extension,
    /// `p.with_extension(e)`, `p.with_file_name(n)`: a new path.
    WithExtension,
    WithFileName,
    /// `p.starts_with(base)`, `p.ends_with(child)`: by whole components.
    StartsWith,
    EndsWith,
    /// `p.strip_prefix(base)`: `Ok` of what's after it, or `Err` of a
    /// `StripPrefixError`, `undefined`, which holds nothing.
    StripPrefix,
    /// `p.is_absolute()` and `p.has_root()`, one question on Unix, and
    /// `p.is_relative()`: whether it starts with a separator, or doesn't.
    IsAbsolute,
    IsRelative,
    /// `p.iter()`: its components' text, an array; `p.ancestors()`: it,
    /// its parent and theirs, to the last.
    Iter,
    Ancestors,
}

impl PathOp {
    /// The runtime's helper that does it, in `path.js`, `None` of what's
    /// written in place.
    fn helper(self) -> Option<&'static str> {
        Some(match self {
            PathOp::Join => "$pathJoin",
            PathOp::Parent => "$pathParent",
            PathOp::FileName => "$pathFileName",
            PathOp::FileStem => "$pathFileStem",
            PathOp::Extension => "$pathExtension",
            PathOp::WithExtension => "$pathWithExtension",
            PathOp::WithFileName => "$pathSetFileName",
            PathOp::StartsWith => "$pathStartsWith",
            PathOp::EndsWith => "$pathEndsWith",
            PathOp::StripPrefix => "$pathStripPrefix",
            PathOp::Iter => "$pathParts",
            PathOp::Ancestors => "$pathAncestors",
            PathOp::IsAbsolute | PathOp::IsRelative => return None,
        })
    }
}

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// A path's call: its helper, given what it's given, or `p.startsWith("/")`.
    /// `None` if `known` is another.
    pub(in crate::lower) fn path_call(&mut self, known: Std, values: &mut std::vec::IntoIter<Expr>) -> Option<Expr> {
        let Std::Path(op) = known else {
            return None;
        };
        Some(match op.helper() {
            Some(helper) => Expr::call(Expr::helper(helper), values.collect()),
            None => {
                let rooted = Expr::call(
                    Expr::member(values.next().expect("the path"), "startsWith"),
                    vec![Expr::str("/")],
                );
                match op {
                    PathOp::IsRelative => Expr::unary(crate::js::UnaryOp::Not, rooted),
                    _ => rooted,
                }
            }
        })
    }
}
