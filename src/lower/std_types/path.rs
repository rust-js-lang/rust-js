//! A path's own methods (ADR 0405): what reads it, its runtime helper of
//! `path.js`, which goes over its components as Rust's `Components` does
//! on Unix. A path is its text (ADR 0173); a `PathBuf` changed in place,
//! `push` and the rest, is its text's edit, `StringEdit`'s.

use crate::js::Expr;
use crate::lower::recognition::Std;
use crate::lower::{FnCx, R};
use rustc_span::Span;
use rustc_span::def_id::DefId;

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
    /// `p.components()`: its components, each a `Component`, std's enum
    /// as any enum is (ADR 0033), `"RootDir"` or `{ TAG: "Normal", _0: "usr" }`;
    /// and a component's `as_os_str()`, its text.
    Components,
    ComponentText,
    /// `Path::new(s)` and `OsStr::new(s)`: what it's given, as a path
    /// (`path_given`).
    New,
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
            PathOp::Components => "$pathComponents",
            PathOp::ComponentText => "$componentText",
            PathOp::IsAbsolute | PathOp::IsRelative | PathOp::New => return None,
        })
    }
}

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// What `callee`'s `at`th argument, of type `ty`, an `AsRef<Path>` or an
    /// `AsRef<OsStr>` as its signature bounds it, is given as: its text,
    /// which a path, a string or an `OsStr` is (ADR 0173), or of another
    /// type its impl's `as_ref()`, through its dictionary: a `Component`'s,
    /// `$componentText`, or the crate's own type's (#1388).
    pub(in crate::lower) fn path_given(
        &mut self,
        value: Expr,
        ty: rustc_middle::ty::Ty<'tcx>,
        (callee, at): (DefId, usize),
        span: Span,
    ) -> R<Expr> {
        let recognition = self.recognition();
        let of = ty.peel_refs();
        if of.is_str() || recognition.is_string_like(of) || recognition.is_path_like(of) {
            return Ok(value);
        }
        let Some(target) = self.as_ref_target(callee, at) else {
            return Ok(value);
        };
        let as_ref = crate::lower::recognition::std_item(self.tcx, crate::lower::recognition::StdItem::AsRef);
        let dictionary = self.dictionary(rustc_middle::ty::TraitRef::new(self.tcx, as_ref, [of, target]), span)?;
        Ok(Expr::call(Expr::member(dictionary, "asRef"), vec![value]))
    }

    /// What `callee`'s `at`th parameter, a type parameter or a `&` of one, is
    /// bounded to be an `AsRef` of, `Path` of `P: AsRef<Path>`, `OsStr` of
    /// `Path::new`'s `&S`.
    fn as_ref_target(&self, callee: DefId, at: usize) -> Option<rustc_middle::ty::Ty<'tcx>> {
        let tcx = self.tcx;
        let declared = self.signature_of(callee).0.get(at)?.peel_refs();
        let as_ref = crate::lower::recognition::std_item(tcx, crate::lower::recognition::StdItem::AsRef);
        tcx.param_env(callee).caller_bounds().iter().find_map(|clause| {
            let tr = clause.as_trait_clause()?.skip_binder().trait_ref;
            (tr.def_id == as_ref && tr.self_ty() == declared).then(|| tr.args.type_at(1))
        })
    }

    /// A path's call: its helper, given what it's given, or `p.startsWith("/")`.
    /// `None` if `known` is another.
    pub(in crate::lower) fn path_call(&mut self, known: Std, values: &mut std::vec::IntoIter<Expr>) -> Option<Expr> {
        let Std::Path(op) = known else {
            return None;
        };
        Some(match op.helper() {
            Some(helper) => Expr::call(Expr::helper(helper), values.collect()),
            None if op == PathOp::New => values.next().expect("what it's given"),
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
