//! `#[rust_js::whole = "lo..=hi"]`, the whole numbers a binding's `f64` is
//! (ADRs 0302, 0994, amended): its bounds, and why one is refused.

use super::is_binding;
use rustc_hir::attrs::lang_items::LangItem;
use rustc_hir::def::DefKind;
use rustc_middle::ty::{self, Ty, TyCtxt};
use rustc_span::Symbol;
use rustc_span::def_id::DefId;

/// Why `def_id`'s `#[rust_js::whole]` is refused: it's on no binding
/// giving an `f64`, or its bounds aren't `"lo..=hi"`. Or it's marked
/// `#[rust_js::position]`, which `whole` replaced.
pub(super) fn whole_refused(tcx: TyCtxt<'_>, def_id: DefId) -> Option<String> {
    let position = [Symbol::intern("rust_js"), Symbol::intern("position")];
    if tcx.get_attrs_by_path(def_id, &position).next().is_some() {
        return Some(
            "rust-js: `#[rust_js::position]` is `#[rust_js::whole = \"-1..=4294967294\"]`, the whole numbers a position or -1 is"
                .to_string(),
        );
    }
    let path = [Symbol::intern("rust_js"), Symbol::intern("whole")];
    let attr = tcx.get_attrs_by_path(def_id, &path).next()?;
    let gives_f64 = matches!(tcx.def_kind(def_id), DefKind::Fn | DefKind::AssocFn)
        && is_binding(tcx, def_id)
        && whole_output(tcx, tcx.fn_sig(def_id).skip_binder().skip_binder().output());
    if !gives_f64 {
        return Some(
            "rust-js: `#[rust_js::whole]` is of a binding that gives an `f64`, or an `Option<f64>`, a nullable one's"
                .to_string(),
        );
    }
    match attr.value_str().and_then(|text| whole_bounds(text.as_str())) {
        Some(_) => None,
        None => Some(
            "rust-js: `#[rust_js::whole = \"lo..=hi\"]` writes the least and the most whole number the binding gives, `\"0..=65535\"`"
                .to_string(),
        ),
    }
}

/// Whether `output` is what `#[rust_js::whole]` is of: an `f64`, or an
/// `Option<f64>`, whose `Some`'s number it says, a nullable WebIDL
/// integer's, `selectionStart`'s `unsigned long?`.
pub(in crate::lower) fn whole_output<'tcx>(tcx: TyCtxt<'tcx>, output: Ty<'tcx>) -> bool {
    output.is_floating_point()
        || matches!(output.kind(), ty::Adt(adt, args)
            if tcx.is_lang_item(adt.did(), LangItem::Option) && args.type_at(0).is_floating_point())
}

/// `#[rust_js::whole = "lo..=hi"]`: a binding whose number is always a
/// whole one from `lo` to `hi`, never `-0`, `NaN` or an infinity: a WebIDL
/// integer's, `unsigned short` `status`'s `"0..=65535"`, or `indexOf`'s
/// position or -1 (ADRs 0302, 0994, amended). Its bounds, where it's one
/// `whole_bounds` reads; `validate` refuses any other.
pub(in crate::lower) fn whole_range(tcx: TyCtxt<'_>, def_id: DefId) -> Option<(f64, f64)> {
    let path = [Symbol::intern("rust_js"), Symbol::intern("whole")];
    whole_bounds(tcx.get_attrs_by_path(def_id, &path).next()?.value_str()?.as_str())
}

/// The bounds `#[rust_js::whole = "lo..=hi"]` writes: two integers, the
/// first no more than the second, each below 10^21 in size, where JS
/// writes a whole number's digits, `${n}`, as Rust's `{}` does, not
/// `1e+21`.
fn whole_bounds(text: &str) -> Option<(f64, f64)> {
    let (lo, hi) = text.split_once("..=")?;
    let bound = |b: &str| b.parse::<i128>().ok().filter(|n| n.unsigned_abs() < 10u128.pow(21));
    let (lo, hi) = (bound(lo)?, bound(hi)?);
    (lo <= hi).then_some((lo as f64, hi as f64))
}
