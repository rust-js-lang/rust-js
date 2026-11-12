//! The names a temporary is given, one rule (ADR 0569, #890): what made
//! it, as rust-analyzer suggests a Rust developer name it, the call the
//! source writes by its name or the program's type it holds; else the word
//! of the role it has, `value`, `arg`, `tmp`. Each place that names a
//! temporary asks here; none spells a temporary's word itself.

use crate::lower::FnCx;
use crate::lower::recognition::StdItem;
use rustc_middle::ty::{self, Ty, TyCtxt};
use rustc_span::def_id::DefId;

/// What a name the compiler picks is given for, where nothing the program
/// wrote names it: a temporary held, a callback's parameter, a `catch`'s
/// binding. Each is one word, `word()`'s, given apart where another has it;
/// lowering names nothing by a word of its own (#890).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::lower) enum Role {
    A,
    AKey,
    Any,
    /// An argument held, `_arg`.
    Arg,
    Args,
    At,
    Atomic,
    B,
    BKey,
    Base,
    /// A `let` its branches set.
    BranchSet,
    By,
    Bytes,
    C,
    Capacity,
    Captured,
    Cell,
    /// What an element is given as its child.
    Children,
    Collection,
    Compare,
    Component,
    Condition,
    Contained,
    Content,
    Cow,
    D,
    Defaults,
    Drained,
    Drop,
    E,
    Entry,
    Error,
    Exact,
    Exchanged,
    Exp,
    Expected,
    F,
    Fallback,
    Given,
    Grown,
    Guard,
    H,
    Handle,
    Hasher,
    Holds,
    I,
    Index,
    Inner,
    Inspect,
    Inspected,
    It,
    Item,
    Items,
    Iter,
    J,
    Json,
    Keep,
    Kept,
    Key,
    Kind,
    Last,
    Lazy,
    Left,
    Length,
    Load,
    Location,
    M,
    Made,
    Make,
    Map,
    Mapped,
    N,
    Nanos,
    Next,
    Object,
    Old,
    Operand,
    Option,
    Options,
    Other,
    Parent,
    Part,
    Parts,
    Payload,
    Peekable,
    Piece,
    Place,
    Pointer,
    Popped,
    Previous,
    Props,
    R,
    Range,
    Rc,
    Reader,
    /// A trait object whose method is called, `_5.impl.add(_5.value)`.
    Receiver,
    Removed,
    Result,
    Right,
    S,
    Set,
    Shown,
    Slice,
    Slot,
    Slots,
    Source,
    Split,
    Spread,
    Start,
    State,
    Step,
    T,
    Taken,
    Test,
    Text,
    Then,
    To,
    /// What nothing reads, `_`.
    Unread,
    Unset,
    Unzipped,
    V,
    /// A value held to be read more than once.
    Value,
    Values,
    Variant,
    Visitor,
    Wrapping,
    X,
    Y,
}

impl Role {
    /// Whether this role's word says nothing of the value, `value` or
    /// `option`, so what made it names it better, its call's name (#890);
    /// not a word the lowering chose for what it holds, `at` of a found
    /// index, as upstream's `idx`, `fallback` of `a || fallback`, nor a
    /// `Result`'s `result`, which its call names worse, `parse` (#769).
    pub(in crate::lower) fn says_nothing(self) -> bool {
        matches!(self, Role::Value | Role::Option)
    }

    /// The word a name of this role is.
    pub(in crate::lower) fn word(self) -> &'static str {
        match self {
            Role::A => "a",
            Role::AKey => "aKey",
            Role::Any => "any",
            Role::Arg => "arg",
            Role::Args => "args",
            Role::At => "at",
            Role::Atomic => "atomic",
            Role::B => "b",
            Role::BKey => "bKey",
            Role::Base => "base",
            Role::BranchSet => "tmp",
            Role::By => "by",
            Role::Bytes => "bytes",
            Role::C => "c",
            Role::Capacity => "capacity",
            Role::Captured => "captured",
            Role::Cell => "cell",
            Role::Children => "children",
            Role::Collection => "collection",
            Role::Compare => "compare",
            Role::Component => "Component",
            Role::Condition => "condition",
            Role::Contained => "contained",
            Role::Content => "content",
            Role::Cow => "cow",
            Role::D => "d",
            Role::Defaults => "defaults",
            Role::Drained => "drained",
            Role::Drop => "drop",
            Role::E => "e",
            Role::Entry => "entry",
            Role::Error => "error",
            Role::Exact => "exact",
            Role::Exchanged => "exchanged",
            Role::Exp => "exp",
            Role::Expected => "expected",
            Role::F => "f",
            Role::Fallback => "fallback",
            Role::Given => "given",
            Role::Grown => "grown",
            Role::Guard => "guard",
            Role::H => "H",
            Role::Handle => "handle",
            Role::Hasher => "hasher",
            Role::Holds => "holds",
            Role::I => "i",
            Role::Index => "index",
            Role::Inner => "inner",
            Role::Inspect => "inspect",
            Role::Inspected => "inspected",
            Role::It => "it",
            Role::Item => "item",
            Role::Items => "items",
            Role::Iter => "iter",
            Role::J => "j",
            Role::Json => "json",
            Role::Keep => "keep",
            Role::Kept => "kept",
            Role::Key => "key",
            Role::Kind => "kind",
            Role::Last => "last",
            Role::Lazy => "lazy",
            Role::Left => "left",
            Role::Length => "length",
            Role::Load => "load",
            Role::Location => "location",
            Role::M => "m",
            Role::Made => "made",
            Role::Make => "make",
            Role::Map => "map",
            Role::Mapped => "mapped",
            Role::N => "n",
            Role::Nanos => "nanos",
            Role::Next => "next",
            Role::Object => "object",
            Role::Old => "old",
            Role::Operand => "operand",
            Role::Option => "option",
            Role::Options => "options",
            Role::Other => "other",
            Role::Parent => "parent",
            Role::Part => "part",
            Role::Parts => "parts",
            Role::Payload => "payload",
            Role::Peekable => "peekable",
            Role::Piece => "piece",
            Role::Place => "place",
            Role::Pointer => "pointer",
            Role::Popped => "popped",
            Role::Previous => "previous",
            Role::Props => "props",
            Role::R => "r",
            Role::Range => "range",
            Role::Rc => "rc",
            Role::Reader => "reader",
            Role::Receiver => "receiver",
            Role::Removed => "removed",
            Role::Result => "result",
            Role::Right => "right",
            Role::S => "s",
            Role::Set => "set",
            Role::Shown => "shown",
            Role::Slice => "slice",
            Role::Slot => "slot",
            Role::Slots => "slots",
            Role::Source => "source",
            Role::Split => "split",
            Role::Spread => "spread",
            Role::Start => "start",
            Role::State => "state",
            Role::Step => "step",
            Role::T => "t",
            Role::Taken => "taken",
            Role::Test => "test",
            Role::Text => "text",
            Role::Then => "then",
            Role::To => "to",
            Role::Unread => "_",
            Role::Unset => "unset",
            Role::Unzipped => "unzipped",
            Role::V => "v",
            Role::Value => "value",
            Role::Values => "values",
            Role::Variant => "variant",
            Role::Visitor => "visitor",
            Role::Wrapping => "wrapping",
            Role::X => "x",
            Role::Y => "y",
        }
    }
}

/// The name MIR's lowering holds an argument by, `_arg`, which tidy names
/// `arg` (`Role::Arg`).
pub(in crate::lower) const HELD_ARG: &str = "_arg";

/// A MIR local's name until tidy names it, `_12`.
pub(in crate::lower) fn temporary(local: usize) -> String {
    format!("_{local}")
}

/// What a name lowering picks is made of: a role's word (`Role`), the `i`th
/// of a role's, `arg0` (`Numbered`), a role's word before a name as it's
/// written, `dropT`, or after one, capitalised, `countValue` (`Prefixed`,
/// `Suffixed`); a name the program
/// wrote, a field's, a key's, a binding's, or what's made of one, `Named`;
/// or what tidy names later, a MIR local, `Local`, a temporary of MIR's
/// kind, `Temporary`, or an argument held, `HeldArg`. The one thing `spill` and `fresh` take, so
/// lowering writes no word of its own (#890).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::lower) enum Base<'a> {
    Role(Role),
    Numbered(Role, usize),
    Prefixed(Role, &'a str),
    Suffixed(&'a str, Role),
    Named(&'a str),
    Local(usize),
    Temporary,
    HeldArg,
}

impl From<Role> for Base<'_> {
    fn from(role: Role) -> Self {
        Base::Role(role)
    }
}

impl Base<'_> {
    /// The name it's made of, before it's given apart.
    pub(in crate::lower) fn word(&self) -> std::borrow::Cow<'_, str> {
        match *self {
            Base::Role(role) => role.word().into(),
            Base::Numbered(role, i) => format!("{}{i}", role.word()).into(),
            Base::Prefixed(role, name) => format!("{}{name}", role.word()).into(),
            Base::Suffixed(name, role) => format!("{name}{}", crate::lower::upper_first(role.word())).into(),
            Base::Named(name) => name.into(),
            Base::Local(local) => temporary(local).into(),
            Base::Temporary => "_$".into(),
            Base::HeldArg => HELD_ARG.into(),
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
