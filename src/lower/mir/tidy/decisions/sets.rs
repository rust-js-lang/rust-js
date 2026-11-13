//! The values a test of a match's tree says of what it tests (`decisions`,
//! ADR 0364): of `c >= "a" && c <= "z"`, `c` and `"a"..="z"`, a set of
//! intervals of numbers, text or `char`s, read again as the fewest tests
//! that say it (#221, #673). Split from `decisions.rs` by concept.

use super::*;

/// The value `e` tests against constants, and the values that pass: of
/// `c >= "a" && c <= "z"`, `c` and `"a"..="z"`.
pub(super) fn atom(e: &Expr) -> Option<(Expr, Set)> {
    match &e.kind {
        ExprKind::Unary(UnaryOp::Not, a) => atom(a).map(|(s, set)| (s, set.complement())),
        // Of integers, `n < 1` is `!(n >= 1)`, its converse's complement,
        // as `Expr::negated_test` makes one of the other: one set of each.
        ExprKind::Binary(Op::Lt | Op::Le, ..) if e.facts.has(js::Known::NeverNaN) => {
            atom(&e.clone().negated_test()).map(|(s, set)| (s, set.complement()))
        }
        ExprKind::Binary(op @ (Op::And | Op::Or), a, b) => {
            let ((s, x), (t, y)) = (atom(a)?, atom(b)?);
            (same_subject(&s, &t) && x.kind == y.kind).then(|| {
                let set = if *op == Op::And { x.intersect(&y) } else { x.union(&y) };
                (s, set)
            })
        }
        ExprKind::Binary(op @ (Op::Eq | Op::Ne | Op::Lt | Op::Le | Op::Gt | Op::Ge), a, b) => {
            let (subject, value, op) = match (Value::of(a), Value::of(b)) {
                (None, Some(value)) => (&**a, value, *op),
                (Some(value), None) => (&**b, value, flipped(*op)),
                _ => return None,
            };
            if !is_subject(subject) {
                return None;
            }
            let kind = value.kind();
            let point = Interval {
                lo: Some((value.clone(), true)),
                hi: Some((value.clone(), true)),
            };
            let pieces = match op {
                Op::Eq | Op::Ne => vec![point],
                Op::Lt | Op::Le => vec![Interval {
                    lo: None,
                    hi: Some((value, op == Op::Le)),
                }],
                _ => vec![Interval {
                    lo: Some((value, op == Op::Ge)),
                    hi: None,
                }],
            };
            let set = Set {
                kind,
                pieces,
                nan: false,
            };
            Some((subject.clone(), if op == Op::Ne { set.complement() } else { set }))
        }
        _ => None,
    }
}

/// `a < b` as `b > a`.
pub(super) fn flipped(op: Op) -> Op {
    match op {
        Op::Lt => Op::Gt,
        Op::Le => Op::Ge,
        Op::Gt => Op::Lt,
        Op::Ge => Op::Le,
        op => op,
    }
}

/// A value a test reads the same each time: a variable, a property or an
/// item of one, or `typeof` one.
pub(super) fn is_subject(e: &Expr) -> bool {
    match &e.kind {
        ExprKind::Var(_) => true,
        ExprKind::Member(object, _) => is_subject(object),
        ExprKind::Index(object, index) => is_subject(object) && Value::of(index).is_some(),
        ExprKind::Unary(UnaryOp::Typeof, of) => is_subject(of),
        _ => false,
    }
}

pub(super) fn same_subject(a: &Expr, b: &Expr) -> bool {
    match (&a.kind, &b.kind) {
        (ExprKind::Unary(UnaryOp::Typeof, x), ExprKind::Unary(UnaryOp::Typeof, y)) => same_subject(x, y),
        _ => a.written_alike(b),
    }
}

/// `subject` in `set`, as a person writes it: the values that pass, or
/// those that don't, whichever is shorter and says the same of `NaN`.
pub(super) fn render(subject: &Expr, set: &Set) -> Expr {
    let other = set.complement();
    // `x !== a && x !== b`: all but some, and `NaN` if a number.
    let points = other.pieces.iter().all(Interval::is_point) && (set.nan || set.kind != Kind::Num);
    if points {
        return all(other
            .pieces
            .iter()
            .map(|p| test(subject, Op::Ne, &p.lo.as_ref().expect("a point").0))
            .collect());
    }
    let passing = any(set.pieces.iter().map(|p| p.render(subject)).collect());
    // A number's comparisons are false of `NaN`; one where it passes is
    // `!` of those of the others.
    if set.nan {
        return any(other.pieces.iter().map(|p| p.render(subject)).collect()).negated_test();
    }
    let failing = || match set.kind {
        Kind::Num => None,
        _ => Some(all(other.pieces.iter().map(|p| p.render_outside(subject)).collect())),
    };
    match failing() {
        Some(failing) if atoms(&failing) < atoms(&passing) => failing,
        _ => passing,
    }
}

pub(super) fn test(subject: &Expr, op: Op, value: &Value) -> Expr {
    Expr::bin(op, subject.clone(), value.expr.clone())
}

/// What a test compares to: a string, a number, or a `BigInt`.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Kind {
    Str,
    Num,
    Big,
}

/// A constant tested against, and how it's written.
#[derive(Clone)]
pub(super) struct Value {
    pub(super) expr: Expr,
    pub(super) of: Lit,
}

#[derive(Clone, PartialEq)]
pub(super) enum Lit {
    /// As JS orders strings, by UTF-16 code unit.
    Str(Vec<u16>),
    Num(f64),
    Big(i128),
}

impl Value {
    pub(super) fn of(e: &Expr) -> Option<Value> {
        let of = match &e.kind {
            ExprKind::Str(s) => Lit::Str(s.encode_utf16().collect()),
            ExprKind::Num(n) if !n.is_nan() => Lit::Num(*n),
            ExprKind::BigInt(n) => Lit::Big(*n),
            ExprKind::BigUint(n) => Lit::Big(i128::try_from(*n).ok()?),
            ExprKind::Unary(UnaryOp::Neg, inner) => match Value::of(inner)?.of {
                Lit::Num(n) => Lit::Num(-n),
                Lit::Big(n) => Lit::Big(n.checked_neg()?),
                Lit::Str(_) => return None,
            },
            _ => return None,
        };
        Some(Value { expr: e.clone(), of })
    }

    pub(super) fn kind(&self) -> Kind {
        match self.of {
            Lit::Str(_) => Kind::Str,
            Lit::Num(_) => Kind::Num,
            Lit::Big(_) => Kind::Big,
        }
    }

    pub(super) fn cmp(&self, other: &Value) -> Ordering {
        match (&self.of, &other.of) {
            (Lit::Str(a), Lit::Str(b)) => a.cmp(b),
            (Lit::Num(a), Lit::Num(b)) => a.partial_cmp(b).expect("no NaN is a constant here"),
            (Lit::Big(a), Lit::Big(b)) => a.cmp(b),
            _ => unreachable!("a set holds values of one kind"),
        }
    }
}

/// Values of one kind from `lo` to `hi`, each `None` where there's no
/// bound, and whether it's in.
#[derive(Clone)]
pub(super) struct Interval {
    pub(super) lo: Option<(Value, bool)>,
    pub(super) hi: Option<(Value, bool)>,
}

impl Interval {
    pub(super) fn is_empty(&self) -> bool {
        match (&self.lo, &self.hi) {
            (Some((lo, a)), Some((hi, b))) => match lo.cmp(hi) {
                Ordering::Greater => true,
                Ordering::Equal => !(*a && *b),
                Ordering::Less => false,
            },
            _ => false,
        }
    }

    pub(super) fn is_point(&self) -> bool {
        matches!((&self.lo, &self.hi), (Some((lo, true)), Some((hi, true))) if lo.cmp(hi) == Ordering::Equal)
    }

    pub(super) fn intersect(&self, other: &Interval) -> Interval {
        // The higher low bound, the one that leaves its value out of two at one value.
        let lo = match (&self.lo, &other.lo) {
            (None, b) => b.clone(),
            (a, None) => a.clone(),
            (Some(a), Some(b)) => Some(match a.0.cmp(&b.0) {
                Ordering::Greater => a.clone(),
                Ordering::Less => b.clone(),
                Ordering::Equal => (a.0.clone(), a.1 && b.1),
            }),
        };
        let hi = match (&self.hi, &other.hi) {
            (None, b) => b.clone(),
            (a, None) => a.clone(),
            (Some(a), Some(b)) => Some(match a.0.cmp(&b.0) {
                Ordering::Less => a.clone(),
                Ordering::Greater => b.clone(),
                Ordering::Equal => (a.0.clone(), a.1 && b.1),
            }),
        };
        Interval { lo, hi }
    }

    /// `subject` within it.
    pub(super) fn render(&self, subject: &Expr) -> Expr {
        if self.is_point() {
            return test(subject, Op::Eq, &self.lo.as_ref().expect("a point").0);
        }
        let lo = (self.lo.as_ref()).map(|(v, i)| test(subject, if *i { Op::Ge } else { Op::Gt }, v));
        let hi = (self.hi.as_ref()).map(|(v, i)| test(subject, if *i { Op::Le } else { Op::Lt }, v));
        all(lo.into_iter().chain(hi).collect())
    }

    /// `subject` outside it, of a kind that has no `NaN`.
    pub(super) fn render_outside(&self, subject: &Expr) -> Expr {
        if self.is_point() {
            return test(subject, Op::Ne, &self.lo.as_ref().expect("a point").0);
        }
        let lo = (self.lo.as_ref()).map(|(v, i)| test(subject, if *i { Op::Lt } else { Op::Le }, v));
        let hi = (self.hi.as_ref()).map(|(v, i)| test(subject, if *i { Op::Gt } else { Op::Ge }, v));
        any(lo.into_iter().chain(hi).collect())
    }
}

/// The values of one kind a test passes: intervals apart and in order, and
/// for numbers whether `NaN` passes.
#[derive(Clone)]
pub(super) struct Set {
    pub(super) kind: Kind,
    pub(super) pieces: Vec<Interval>,
    pub(super) nan: bool,
}

impl PartialEq for Set {
    fn eq(&self, other: &Set) -> bool {
        let bound = |a: &Option<(Value, bool)>, b: &Option<(Value, bool)>| match (a, b) {
            (None, None) => true,
            (Some(a), Some(b)) => a.1 == b.1 && a.0.cmp(&b.0) == Ordering::Equal,
            _ => false,
        };
        self.kind == other.kind
            && self.nan == other.nan
            && self.pieces.len() == other.pieces.len()
            && (self.pieces.iter().zip(&other.pieces)).all(|(a, b)| bound(&a.lo, &b.lo) && bound(&a.hi, &b.hi))
    }
}

impl Set {
    pub(super) fn all(kind: Kind) -> Set {
        Set {
            kind,
            pieces: vec![Interval { lo: None, hi: None }],
            nan: kind == Kind::Num,
        }
    }

    pub(super) fn is_empty(&self) -> bool {
        self.pieces.is_empty() && !self.nan
    }

    /// Whether each value it passes `other` passes too.
    pub(super) fn within(&self, other: &Set) -> bool {
        self.kind == other.kind && self.intersect(other) == *self
    }

    pub(super) fn intersect(&self, other: &Set) -> Set {
        let mut pieces: Vec<Interval> = (self.pieces.iter())
            .flat_map(|a| other.pieces.iter().map(move |b| a.intersect(b)))
            .filter(|p| !p.is_empty())
            .collect();
        pieces.sort_by(|a, b| match (&a.lo, &b.lo) {
            (None, None) => Ordering::Equal,
            (None, _) => Ordering::Less,
            (_, None) => Ordering::Greater,
            (Some(a), Some(b)) => a.0.cmp(&b.0).then(b.1.cmp(&a.1)),
        });
        Set {
            kind: self.kind,
            pieces,
            nan: self.nan && other.nan,
        }
        .merged()
    }

    /// The values it leaves out.
    pub(super) fn complement(&self) -> Set {
        let mut pieces = Vec::new();
        // Where the gap after the last piece starts, `None` of none before.
        let mut from: Option<(Value, bool)> = None;
        let mut to_the_end = true;
        for piece in &self.pieces {
            if let Some((v, i)) = &piece.lo {
                let gap = Interval {
                    lo: from.clone(),
                    hi: Some((v.clone(), !i)),
                };
                if !gap.is_empty() {
                    pieces.push(gap);
                }
            }
            match &piece.hi {
                Some((v, i)) => from = Some((v.clone(), !i)),
                None => {
                    to_the_end = false;
                    break;
                }
            }
        }
        if to_the_end {
            pieces.push(Interval { lo: from, hi: None });
        }
        Set {
            kind: self.kind,
            pieces,
            nan: self.kind == Kind::Num && !self.nan,
        }
    }

    pub(super) fn union(&self, other: &Set) -> Set {
        self.complement().intersect(&other.complement()).complement()
    }

    /// Pieces that overlap or meet, one.
    pub(super) fn merged(mut self) -> Set {
        let mut pieces: Vec<Interval> = Vec::new();
        for piece in self.pieces {
            if let Some(last) = pieces.last_mut()
                && meets(&last.hi, &piece.lo)
            {
                let further = match (&last.hi, &piece.hi) {
                    (None, _) => false,
                    (_, None) => true,
                    (Some(a), Some(b)) => match b.0.cmp(&a.0) {
                        Ordering::Greater => true,
                        Ordering::Equal => b.1 && !a.1,
                        Ordering::Less => false,
                    },
                };
                if further {
                    last.hi = piece.hi;
                }
                continue;
            }
            pieces.push(piece);
        }
        self.pieces = pieces;
        self
    }
}

/// Whether a piece that ends at `hi` and one that starts at `lo` leave no
/// value between them.
pub(super) fn meets(hi: &Option<(Value, bool)>, lo: &Option<(Value, bool)>) -> bool {
    match (hi, lo) {
        (None, _) | (_, None) => true,
        (Some((h, hi_in)), Some((l, lo_in))) => match l.cmp(h) {
            Ordering::Less => true,
            Ordering::Equal => *hi_in || *lo_in,
            Ordering::Greater => false,
        },
    }
}
