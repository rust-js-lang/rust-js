//! How a place that stores so holds what's written to it (`Storage`, ADR
//! 0791): as it's given, a flattened state's keys, or a `None` `null`.

use super::*;

impl Storage {
    /// A flattened state's storage of its `keys` (rd380, ADR 1044), each
    /// type's kept for the compile, as a handle's storage is copied.
    pub fn in_place(keys: &[String]) -> Storage {
        thread_local! {
            static KEYS: std::cell::RefCell<std::collections::HashMap<String, &'static str>> = Default::default();
        }
        let joined = keys.join(",");
        Storage::InPlace(KEYS.with_borrow_mut(|known| {
            *known
                .entry(joined.clone())
                .or_insert_with(|| Box::leak(joined.into_boxed_str()))
        }))
    }

    /// Of an `InPlace` storage, `value` written to `place`, the object a
    /// flattened state is in (ADR 1044): of a variant made there, its tag
    /// and each of its fields set there, as written, a `None` `undefined`,
    /// so a key an earlier variant set and it hasn't stays, as upstream's
    /// do; of any other value, each key of it, once it's computed, held
    /// where that's needed, `spill`'s. `None` of any other storage.
    pub fn written_in_place(
        self,
        place: &Expr,
        value: Expr,
        spill: &mut dyn FnMut(Expr, &mut Vec<Stmt>) -> Expr,
        at: Span,
    ) -> Option<Vec<Stmt>> {
        let Storage::InPlace(keys) = self else { return None };
        let keys: Vec<&str> = keys.split(',').collect();
        let set = |key: &str, value: Expr| StmtKind::Assign(Expr::member(place.clone(), key), value).at(at);
        if let ExprKind::Object(props, _) = &value.kind
            && props
                .iter()
                .all(|prop| matches!(prop, Prop::Field(name, _) if keys.contains(&name.as_str())))
        {
            let sets = props.iter().map(|prop| match prop {
                Prop::Field(name, value) => set(name, value.clone()),
                _ => unreachable!("an object of fields"),
            });
            return Some(sets.collect());
        }
        let mut out = Vec::new();
        let of = match value.reads_same() && !value.has_effects() {
            true => value,
            false => spill(value, &mut out),
        };
        out.extend(keys.iter().map(|key| set(key, Expr::member(of.clone(), *key))));
        Some(out)
    }

    /// What a place stored so holds, of `made`, an `Option`'s JS where it's
    /// `NullForNone`: a `None` is `null`; what's `nulled`, `null` or a
    /// value already, as MIR tells, and what's never `undefined`
    /// (`may_be_undefined`), a constant, an object made here or a `Some`'s
    /// value, as it is; a test's branches each so, `c ? f(c) : null` of `c ?
    /// f(c) : undefined`; else `made ?? null` (ADRs 0275, 0772, 0791). The
    /// one answer, where a struct is made, a `Cell` set, a nullable
    /// parameter given, and a handle's setter printed.
    pub fn written(self, made: Expr, nulled: bool) -> Expr {
        match self {
            Storage::AsGiven | Storage::InPlace(_) => made,
            Storage::NullForNone if nulled && !matches!(made.kind, ExprKind::Undefined | ExprKind::GivenUndefined) => {
                made
            }
            Storage::NullForNone => match null_for_none(&made) {
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
            (Storage::AsGiven | Storage::InPlace(_), kind) => Expr { kind, ..made },
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
/// `undefined` is itself; `None` where it may be `undefined` otherwise.
pub(super) fn null_for_none(made: &Expr) -> Option<Expr> {
    match &made.kind {
        ExprKind::Undefined | ExprKind::GivenUndefined => Some(Expr::null().or_at(made.span)),
        _ if !made.may_be_undefined() => Some(made.clone()),
        ExprKind::Cond(test, yes, no) => Some(Expr {
            facts: made.facts,
            kind: ExprKind::Cond(
                test.clone(),
                Box::new(null_for_none(yes)?),
                Box::new(null_for_none(no)?),
            ),
            span: made.span,
        }),
        _ => None,
    }
}
