//! An iterator's value as the lowering makes it: what it's made of, and the
//! facts a step past its maker asks of it, written once, by `emit`, as its
//! JS and the `Known` facts that JS carries (ADR 1092's owner (b), #730).
//! A reader asks this module, never the JS's shape: whether a value is
//! closed, lent or a range's numbers is what its maker said, not what a
//! helper's name looks like.

use crate::js::{Expr, Known};

/// What a JS iterator is made as, each its maker's.
pub(in crate::lower) enum Form {
    /// `$iterator(value, next, ..)`: `value`, the crate's own iterator or
    /// what a dictionary's `next_back` steps, stepped by `next`, and what
    /// else it's given, each in its place (`stepped_ends`).
    Own { value: Expr, next: Expr, given: Vec<Expr> },
    /// A range's numbers, `$iterator(range, step)`: the range, its bounds,
    /// stepped by its kind's step (`std_types/range.rs`'s `range_stepper`).
    Range { range: Expr, step: Expr },
    /// `$given(value, dictionary, ..)`: a type parameter's value given as it
    /// is, stepped by its dictionaries (`given_js_iterator`, ADR 0411).
    Given {
        value: Expr,
        dictionary: Expr,
        given: Vec<Expr>,
    },
    /// `$knowing(it, back, len, ..)`: std's compiled struct, given its back
    /// and its length by a use site (`knowing_held`, ADR 0561).
    Knowing { it: Expr, given: Vec<Expr> },
    /// Any other JS iterator, as its maker wrote it.
    Js(Expr),
}

/// What closes a value to drop what it hasn't given, `$closing(it, drop)`
/// (`closed_by`, ADRs 0422, 0465): `drop`, and whether it drops each item
/// of an array's as it's passed, not as one slice (#234).
struct Closing {
    drop: Expr,
    passes_each: bool,
}

/// An iterator's value: its form, and whether it's closed to drop what it
/// holds, or lent, `$lent(it)`, which nothing past it closes (ADR 0071).
pub(in crate::lower) struct IterValue {
    form: Form,
    closes: Option<Closing>,
    lent: bool,
}

impl IterValue {
    /// A value of `form`, neither closed nor lent.
    pub(in crate::lower) fn of(form: Form) -> IterValue {
        IterValue {
            form,
            closes: None,
            lent: false,
        }
    }

    /// `it`, a JS iterator its maker wrote.
    pub(in crate::lower) fn js(it: Expr) -> IterValue {
        IterValue::of(Form::Js(it))
    }

    /// It, closed by `drop`, `passes_each` or not (`closed_by`). Of one
    /// closed or lent already, that closed again around it.
    pub(in crate::lower) fn closed(self, drop: Expr, passes_each: bool) -> IterValue {
        let mut value = match self.closes.is_some() || self.lent {
            true => IterValue::js(self.emit()),
            false => self,
        };
        value.closes = Some(Closing { drop, passes_each });
        value
    }

    /// It, lent (`lent_iterator`).
    pub(in crate::lower) fn lent(mut self) -> IterValue {
        self.lent = true;
        self
    }

    /// Its JS, each part marked with what it is: the one writer of
    /// `$iterator`, `$given`, `$knowing`, `$closing` and `$lent`, and of
    /// the facts each carries.
    pub(in crate::lower) fn emit(self) -> Expr {
        let made = |name: &'static str, args: Vec<Expr>, known: &[Known]| {
            let mut e = Expr::call_helper(name, args);
            for &fact in known {
                e.facts = e.facts.with(fact);
            }
            e
        };
        let mut e = match self.form {
            Form::Own { value, next, given } => {
                let mut args = vec![value, next];
                args.extend(given);
                made("$iterator", args, &[Known::IterStepped])
            }
            Form::Range { range, step } => {
                made("$iterator", vec![range, step], &[Known::IterStepped, Known::IterRange])
            }
            Form::Given {
                value,
                dictionary,
                given,
            } => {
                let mut args = vec![value, dictionary];
                args.extend(given);
                made("$given", args, &[Known::IterStepped])
            }
            Form::Knowing { it, given } => {
                let mut args = vec![it];
                args.extend(given);
                made("$knowing", args, &[Known::IterKnowing])
            }
            Form::Js(e) => e,
        };
        if let Some(Closing { drop, passes_each }) = self.closes {
            let mut args = vec![e, drop];
            if passes_each {
                args.push(Expr::bool(false));
            }
            e = made("$closing", args, &[Known::IterClosing]);
        }
        if self.lent {
            e = made("$lent", vec![e], &[Known::IterLent]);
        }
        e
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::js;

    fn var(name: &str) -> Expr {
        Expr::var(name)
    }

    #[test]
    fn each_form_is_written_and_marked() {
        let range = IterValue::of(Form::Range {
            range: var("r"),
            step: Expr::helper("$rangeNext"),
        })
        .emit();
        assert!(matches!(&range.kind, js::ExprKind::Call(callee, args)
            if callee.helper_name() == Some("$iterator") && args.len() == 2));
        assert!(range.facts.has(Known::IterStepped) && range.facts.has(Known::IterRange));
        let closed = IterValue::js(var("it")).closed(var("drop"), true).lent().emit();
        assert!(closed.facts.has(Known::IterLent) && !closed.facts.has(Known::IterClosing));
        let js::ExprKind::Call(_, inner) = &closed.kind else {
            panic!("a call")
        };
        assert!(inner[0].facts.has(Known::IterClosing));
        let js::ExprKind::Call(_, parts) = &inner[0].kind else {
            panic!("a call")
        };
        assert_eq!(parts.len(), 3);
    }
}
