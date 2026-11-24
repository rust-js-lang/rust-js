//! The brands of the types no other crate can make (#2403 step 2c, ADR
//! 1196): what the declarations brand, `sealed` (ADR 1187), whose JS is an
//! object, is branded where Rust makes one, `$brand(key, [text])`, and
//! checked by it where JS gives one, so JS's own `["not an email"]` is
//! refused. One of a `#[repr(transparent)]` newtype of a number or a
//! string, which no `WeakSet` holds, keeps the declarations' brand alone.
//!
//! A type's brand is one set per type, not per instantiation, found by its
//! key, its crate's name, its disambiguator and its path, `app#1f2e::Email`,
//! in a registry every copy of the runtime shares (`$brandOf`): a library's
//! maker and its app's check read one set; two versions of one crate, two
//! types to Rust, two.

use rust_js_program::js::Expr;
use rustc_middle::ty::{self, Ty};
use rustc_span::def_id::DefId;

use crate::declarations::contract::sealed;
use crate::{FnCx, Shape};

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// The brand's key of `ty`, a struct no other crate can make whose JS is
    /// an object of its own; none of any other.
    pub fn brand_key(&self, ty: Ty<'tcx>) -> Option<String> {
        let ty::Adt(adt, _) = ty.kind() else { return None };
        // Not std's, whose types its own declarations name (ADR 1149), not
        // `sealed`'s, which declares a crate's.
        if !adt.is_struct()
            || crate::recognition::is_std_crate(self.tcx, adt.did())
            || !sealed(self.tcx, adt.did())
            || self.always_holds(ty).is_some()
        {
            return None;
        }
        match self.shape(ty) {
            Shape::Object(_) | Shape::Array(_) => Some(brand_key_of(self.tcx, adt.did())),
            Shape::Other => None,
        }
    }

    /// Whether a value of `did`, a struct, that this function makes may
    /// reach JS, as `copies::reached` follows it (#2408): never of a guard
    /// holding a borrow, which can't leave Rust. A `const`'s or a `static`'s,
    /// whose initializer it follows not, may.
    fn may_reach_js(&self, did: DefId) -> bool {
        let root = self.tcx.typeck_root_def_id(self.item);
        !matches!(
            self.tcx.def_kind(root),
            rustc_hir::def::DefKind::Fn | rustc_hir::def::DefKind::AssocFn
        ) || (self.krate.js_copies.branded.get(&root)).is_some_and(|branded| branded.contains(&did))
    }

    /// `made`, a value of `ty` Rust made, branded where `ty` is: a mark on
    /// its literal, which every pass reads past (`js::brands`), else, of
    /// what's no literal, the call that brands it.
    pub fn branded(&self, ty: Ty<'tcx>, made: Expr) -> Expr {
        // Not of a value this function makes that can't leave Rust.
        match ty.kind() {
            ty::Adt(adt, _) if !self.may_reach_js(adt.did()) => made,
            _ => self.branded_wherever(ty, made),
        }
    }

    /// `branded`'s, of a value that may go anywhere, made apart from the
    /// MIR `copies::reached` follows: what serde reads, which its reader
    /// makes for the visitor's body.
    pub fn branded_wherever(&self, ty: Ty<'tcx>, mut made: Expr) -> Expr {
        let Some(key) = self.brand_key(ty) else { return made };
        match &mut made.kind {
            rust_js_program::js::ExprKind::Object(_, layout) => layout.brand = Some(key),
            rust_js_program::js::ExprKind::Array(_, layout) => layout.brand = Some(key),
            _ => return Expr::call(Expr::helper("$brand"), vec![Expr::str(key), made]),
        }
        made
    }
}

/// `did`'s brand's key: its crate's identity, its name and disambiguator,
/// and its path in its crate.
pub(crate) fn brand_key_of(tcx: ty::TyCtxt<'_>, did: DefId) -> String {
    let path = tcx.def_path(did).to_string_no_crate_verbose();
    format!("{}{path}", crate::recognition::crate_identity(tcx, did.krate))
}

/// "an Email", "a Temperature": the name of a type a check refuses a value
/// of, as an error writes it.
pub(super) fn with_article(name: &str) -> String {
    match name.starts_with(['A', 'E', 'I', 'O', 'U']) {
        true => format!("an {name}"),
        false => format!("a {name}"),
    }
}
