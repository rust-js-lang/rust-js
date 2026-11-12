//! The names a temporary is given, one rule (ADR 0569, #890): what made
//! it, as rust-analyzer suggests a Rust developer name it, the call the
//! source writes by its name or the program's type it holds; else the word
//! of the role it has, `value`, `arg`, `tmp`. Each place that names a
//! temporary asks here; none spells a temporary's word itself.

use crate::lower::FnCx;
use crate::lower::recognition::StdItem;
use rustc_middle::ty::{self, Ty, TyCtxt};
use rustc_span::def_id::DefId;

/// What a temporary is kept as where nothing that made it names it: the
/// word of its role, given apart where another has it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::lower) enum Role {
    /// A value held to be read more than once.
    Value,
    /// An argument held, `_arg`.
    Arg,
    /// A `let` its branches set.
    BranchSet,
    /// A trait object whose method is called, `_5.impl.add(_5.value)`.
    Receiver,
    /// What an element is given as its child.
    Children,
}

impl Role {
    /// The word a temporary of this role is kept as.
    pub(in crate::lower) fn word(self) -> &'static str {
        match self {
            Role::Value => "value",
            Role::Arg => "arg",
            Role::BranchSet => "tmp",
            Role::Receiver => "receiver",
            Role::Children => "children",
        }
    }
}

/// A temporary's name, of the names what made it suggests, first to last,
/// and its role: the first no other name in its function has, else its
/// role's word, given apart where it must be, `value$1`. The one pick.
pub(in crate::lower) fn pick(
    suggestions: Vec<String>,
    role: Role,
    taken: &mut crate::collections::IndexSet<String>,
) -> String {
    let base = (suggestions.into_iter())
        .find(|name| !taken.contains(name))
        .unwrap_or_else(|| role.word().to_string());
    crate::names::fresh_in(taken, &base)
}

/// What rust-analyzer names a call's result: a method's name without a
/// conversion's prefix, `uppercase` of `to_uppercase`, and none of a
/// method that names nothing; a function's without a constructor's
/// prefix, `config` of `from_config`; and none of a name that's no noun of
/// what it gives, `new`, `default`, `str`. Rust's spelling, before
/// `suggested` makes it a JS variable's.
pub(in crate::lower) fn call_name(tcx: TyCtxt<'_>, def_id: DefId) -> Option<String> {
    const USELESS_METHODS: &[&str] = &[
        "to_string",
        "as_str",
        "to_owned",
        "as_ref",
        "clone",
        "cloned",
        "expect",
        "expect_none",
        "unwrap",
        "unwrap_none",
        "unwrap_or",
        "unwrap_or_default",
        "unwrap_or_else",
        "unwrap_unchecked",
        "iter",
        "into_iter",
        "iter_mut",
        "into_future",
        // And the `Err` side of what rust-analyzer's list has, as it names
        // nothing either.
        "unwrap_err",
        "expect_err",
    ];
    const USELESS_NAMES: &[&str] = &[
        "new", "default", "option", "some", "none", "ok", "err", "str", "string", "from", "into",
    ];
    let name = tcx.item_name(def_id);
    let name = name.as_str();
    let method = tcx.opt_associated_item(def_id).is_some_and(|item| item.is_method());
    let name = match method {
        true if USELESS_METHODS.contains(&name) => return None,
        true => ["into_", "as_", "to_"]
            .iter()
            .find_map(|p| name.strip_prefix(p))
            .unwrap_or(name),
        false => ["from_", "with_", "into_"]
            .iter()
            .find_map(|p| name.strip_prefix(p))
            .unwrap_or(name),
    };
    let plain = name.starts_with(|c: char| c.is_ascii_lowercase()) && !name.starts_with('_');
    let keyword = rustc_span::symbol::Ident::from_str(name).is_reserved();
    (plain && !keyword && !USELESS_NAMES.contains(&name)).then(|| name.to_string())
}

/// What a call names its result as a JS variable, `trailingZeros` of
/// `a.trailing_zeros()`: `call_name`'s, `suggested`.
pub(in crate::lower) fn call_suggestion(tcx: TyCtxt<'_>, def_id: DefId) -> Option<String> {
    suggested(&call_name(tcx, def_id)?)
}

/// A name rust-analyzer suggests, Rust's spelling or a type's, as a JS
/// variable's, lower camel case: `trailingZeros` of `trailing_zeros`,
/// `httpServer` of `HTTPServer`; none where that's no JS identifier.
pub(in crate::lower) fn suggested(name: &str) -> Option<String> {
    let name = variable_name(&crate::lower::camel_case(name));
    (crate::names::js_ident(&name) == name).then_some(name)
}

impl<'tcx> FnCx<'_, 'tcx> {
    /// The name a value of `ty` is kept by, its type's, `order` of an
    /// `Order`, through a reference and what JS holds as the value it
    /// holds, an `Option`, a `Box`, an `Rc`, a `Cell` and a `RefCell` of the
    /// value itself: of the program's own or a library's type, `None` of
    /// std's. Not a `Result`'s `Ok`, as rust-analyzer's is: JS holds a
    /// `Result` as `{ TAG, _0 }`, which `order.TAG` would misname.
    pub(in crate::lower) fn held_type_name(&self, ty: Ty<'tcx>) -> Option<String> {
        let mut ty = ty.peel_refs();
        loop {
            ty = match ty.kind() {
                _ if let Some(inner) = self.option_of(ty) => inner.peel_refs(),
                ty::Adt(_, args) if ty.is_box() || self.is_std_type(ty, StdItem::Rc) => args.type_at(0).peel_refs(),
                // A `Cell` a temporary holds, and a `RefCell` of the value
                // itself, are what they hold: `Cell::new(Some(error))`'s
                // `error` (rd413).
                ty::Adt(_, args) if self.is_std_type(ty, StdItem::Cell) || self.is_value_cell(ty) => {
                    args.type_at(0).peel_refs()
                }
                _ => break,
            };
        }
        let ty::Adt(adt, _) = ty.kind() else { return None };
        if crate::lower::recognition::is_std_crate(self.tcx, adt.did()) {
            return None;
        }
        Some(self.tcx.item_name(adt.did()).to_string())
    }
}

/// A type's name as a variable's, lower camel case, as JS names one:
/// `order` of `Order`, `httpServer` of `HTTPServer`, `url` of `URL`.
fn variable_name(type_name: &str) -> String {
    let chars: Vec<char> = type_name.chars().collect();
    // The capitals it starts with, but for one that starts the next word.
    let run = chars.iter().take_while(|c| c.is_ascii_uppercase()).count();
    let lowered = match run > 1 && run < chars.len() {
        true => run - 1,
        false => run,
    };
    (chars.iter().enumerate())
        .map(|(i, c)| match i < lowered.max(1) {
            true => c.to_ascii_lowercase(),
            false => *c,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::variable_name;

    // A type's name as a variable's: its first capital lowered, a run of them
    // too, but for the one that starts the next word.
    #[test]
    fn a_type_name_is_a_variable_name() {
        assert_eq!(variable_name("Order"), "order");
        assert_eq!(variable_name("HTTPServer"), "httpServer");
        assert_eq!(variable_name("URL"), "url");
        assert_eq!(variable_name("IOError"), "ioError");
        assert_eq!(variable_name("trailingZeros"), "trailingZeros");
    }
}
