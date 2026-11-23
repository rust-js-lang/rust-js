//! How a place that stores so holds what's written to it (`Storage`, ADR
//! 0791): as it's given, a flattened state's keys, or a `None` `null`.

use super::*;
use crate::collections::IndexMap;

/// What a nullable place's write is told of whether what it's given may
/// be `undefined` (`Storage::written`, `Expr::may_be_undefined`): that MIR
/// says it's `null` or a value already, `Never`; where lowering has no
/// such answer, what its JS form says alone, a constant or an object made
/// here never is, `ByForm`. A write whose place's storage is learned past
/// lowering carries its answer on its statement (`StmtFacts::written`);
/// no mark lowering wrote on a value is asked (#2112 C S4c).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Undefined {
    Never,
    #[default]
    ByForm,
}

impl Undefined {
    /// `Never` where MIR says what's given is `nulled`, `null` or a value
    /// already; else what its form says, `ByForm`.
    pub fn never_if(nulled: bool) -> Undefined {
        match nulled {
            true => Undefined::Never,
            false => Undefined::ByForm,
        }
    }
}

/// Of a flattened state written whole (`Storage::written_in_place`), a
/// key an earlier variant set that the variant written lacks: `Cleared`,
/// set `undefined`, as Rust's value has no such field, of another crate's
/// object, a library's, whose program reads it as Rust does (ADR 1022),
/// `Object.keys` among it (#1067); `Kept` of the crate's own, as
/// upstream's do, a rejected chunk's `value` of React's port.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum StaleKeys {
    Kept,
    Cleared,
}

impl Storage {
    /// A flattened state's storage of its `keys` (rd380, ADR 1044), each
    /// type's kept for the compile, as a handle's storage is copied; with
    /// what becomes of a key an earlier variant set (`StaleKeys`).
    pub fn in_place(keys: &[String], stale: StaleKeys) -> Storage {
        thread_local! {
            static KEYS: std::cell::RefCell<IndexMap<String, &'static str>> = Default::default();
        }
        let joined = keys.join(",");
        let keys = KEYS.with_borrow_mut(|known| {
            *known
                .entry(joined.clone())
                .or_insert_with(|| Box::leak(joined.into_boxed_str()))
        });
        Storage::InPlace(keys, stale)
    }

    /// Whether a write through a handle on a variable stored so may set the
    /// variable, `x = value`: not an object's whole `$assign`ed (#968). A
    /// flattened state's handle is taken to, as it was: its keys are set
    /// on its object, and its variable stays a `let` until a form change
    /// is chosen.
    pub fn rebinds(self) -> bool {
        match self {
            Storage::AsGiven | Storage::NullForNone | Storage::InPlace(..) => true,
            Storage::Assigned => false,
        }
    }

    /// Whether a write through a handle on a place stored so assigns the
    /// place what `written` makes of the value, as it's given or a `None`
    /// `null`: not a flattened state's keys each set, nor an object
    /// `$assign`ed (`written_in_place`), whose place a write would rebind.
    pub fn assigns_its_place(self) -> bool {
        match self {
            Storage::AsGiven | Storage::NullForNone => true,
            Storage::InPlace(..) | Storage::Assigned => false,
        }
    }

    /// The helper a write through a handle on a place stored so is written
    /// with, `$assign` of `Assigned`, which preparation writes out: asked
    /// for where the handle is written (#968).
    pub fn helper(self) -> Option<&'static str> {
        match self {
            Storage::Assigned => Some("$assign"),
            Storage::AsGiven | Storage::NullForNone | Storage::InPlace(..) => None,
        }
    }

    /// Of an `InPlace` storage, `value` written to `place`, the object a
    /// flattened state is in (ADR 1044): of a variant made there, its tag
    /// and each of its fields set there, as written, a `None` `undefined`,
    /// so a key an earlier variant set and it hasn't stays, as upstream's
    /// do, but of a library's object, `StaleKeys::Cleared`, set
    /// `undefined` after them, as Rust's value has no such field (#1067);
    /// of any other value, each key of it, once it's computed, held
    /// where that's needed, `spill`'s, given the name it's held by.
    /// Of `Assigned`, `$assign(place, value)`. `None` of any other storage.
    ///
    /// Rust computes the whole value before it writes any of it: a field
    /// that computes, `reason: thrown(error)`, or reads the object a key
    /// set before it changes, is held first, `const reason = ..`, each in
    /// the order written (#779); any other is set as it is, an object of
    /// variables or a read of another place (#784).
    pub fn written_in_place(
        self,
        place: &Expr,
        value: Expr,
        spill: &mut dyn FnMut(&str, Expr, &mut Vec<Stmt>) -> Expr,
        at: Span,
    ) -> Option<Vec<Stmt>> {
        // An object's whole, written in place (ADR 0147, #968).
        if self == Storage::Assigned {
            let assigned = Expr::call(Expr::helper("$assign"), vec![place.clone(), value]);
            return Some(vec![StmtKind::Expr(assigned).at(at)]);
        }
        let Storage::InPlace(keys, stale) = self else {
            return None;
        };
        let keys: Vec<&str> = keys.split(',').collect();
        let set = |key: &str, value: Expr| StmtKind::Assign(Expr::member(place.clone(), key), value).at(at);
        if let ExprKind::Object(props, _) = &value.kind
            && props
                .iter()
                .all(|prop| matches!(prop, Prop::Field(name, _) if keys.contains(&name.as_str())))
        {
            // What an earlier key's set can't change, nor see: no call, no
            // getter, and no read of the object written; another place it
            // reads, a field of `handler`, Rust's borrow of the state keeps
            // apart from it (#784).
            let mut root = place;
            while let ExprKind::Member(of, _) | ExprKind::Index(of, _) = &root.kind {
                root = of;
            }
            let apart = |value: &Expr| match &root.kind {
                ExprKind::Var(object) => !value.has_effects() && !value.mentions_name(object),
                _ => value.is_constant() || value.reads_only_vars(),
            };
            let mut out = Vec::new();
            let fields: Vec<(&str, Expr)> = props
                .iter()
                .map(|prop| match prop {
                    Prop::Field(name, value) if apart(value) => (name.as_str(), value.clone()),
                    Prop::Field(name, value) => (name.as_str(), spill(name, value.clone(), &mut out)),
                    _ => unreachable!("an object of fields"),
                })
                .collect();
            let cleared: Vec<&str> = match stale {
                StaleKeys::Cleared => (keys.iter().copied())
                    .filter(|key| !fields.iter().any(|(name, _)| name == key))
                    .collect(),
                StaleKeys::Kept => Vec::new(),
            };
            out.extend(fields.into_iter().map(|(name, value)| set(name, value)));
            out.extend(cleared.into_iter().map(|key| set(key, Expr::js_undefined())));
            return Some(out);
        }
        let mut out = Vec::new();
        let of = match value.reads_same() && !value.has_effects() {
            true => value,
            false => spill("state", value, &mut out),
        };
        out.extend(keys.iter().map(|key| set(key, Expr::member(of.clone(), *key))));
        Some(out)
    }

    /// What a place stored so holds, of `made`, an `Option`'s JS where it's
    /// `NullForNone`: a `None` is `null`; what's `null` or a value already,
    /// as MIR tells (`Undefined::Never`), and what's never `undefined` as
    /// `given` says (`may_be_undefined`), a constant, an object made here,
    /// as it is; a test's branches each so, `c ? f(c) : null` of `c ? f(c) :
    /// undefined`; else `made ?? null` (ADRs 0275, 0772, 0791). The one
    /// answer, where a struct is made, a `Cell` set, a nullable parameter
    /// given, and a handle's setter printed.
    pub fn written(self, made: Expr, given: Undefined) -> Expr {
        match self {
            Storage::AsGiven | Storage::InPlace(..) | Storage::Assigned => made,
            Storage::NullForNone => match null_for_none(&made, given) {
                Some(stored) => stored,
                None => Expr::bin(Op::Coalesce, made, Expr::null()),
            },
        }
    }

    /// What a place stored so holds of `made` where nothing reads it apart,
    /// `null` from `undefined` (#665): a `None` `null`, and so each branch
    /// of a test, `__DEV__ && debug ? null : request`, and anything else as
    /// it is, what may be `undefined` too, never `made ?? null`.
    pub fn written_alike(self, made: Expr) -> Expr {
        match (self, made.kind) {
            (Storage::AsGiven | Storage::InPlace(..) | Storage::Assigned, kind) => Expr { kind, ..made },
            (Storage::NullForNone, ExprKind::Undefined | ExprKind::GivenUndefined) => Expr::null().or_at(made.span),
            (Storage::NullForNone, ExprKind::Cond(test, yes, no)) => Expr {
                kind: ExprKind::Cond(
                    test,
                    Box::new(self.written_alike(*yes)),
                    Box::new(self.written_alike(*no)),
                ),
                ..made
            },
            (Storage::NullForNone, kind) => Expr { kind, ..made },
        }
    }
}

/// `made` stored with its `None` `null`, where that's what it is, or each
/// branch of its test is: `undefined` is `null`, and what's never
/// `undefined` is itself, what MIR says is `null` or a value (`Never`) or
/// its form or facts say isn't; `None` where it may be `undefined`
/// otherwise. `Storage::written`'s answer, and `Expr::store_again`'s.
pub(super) fn null_for_none(made: &Expr, given: Undefined) -> Option<Expr> {
    match &made.kind {
        ExprKind::Undefined | ExprKind::GivenUndefined => Some(Expr::null().or_at(made.span)),
        _ if given == Undefined::Never || !made.may_be_undefined() => Some(made.clone()),
        ExprKind::SomeWhere(test, value) => Some(Expr {
            facts: made.facts,
            kind: ExprKind::Cond(test.clone(), value.clone(), Box::new(Expr::null())),
            span: made.span,
            held_as: Default::default(),
        }),
        ExprKind::Cond(test, yes, no) => Some(Expr {
            facts: made.facts,
            kind: ExprKind::Cond(
                test.clone(),
                Box::new(null_for_none(yes, given)?),
                Box::new(null_for_none(no, given)?),
            ),
            span: made.span,
            held_as: Default::default(),
        }),
        _ => None,
    }
}
