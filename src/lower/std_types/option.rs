//! An `Option`'s or a `Result`'s method, and `bool::then` (ADRs 0030, 0062).

use crate::js;
use crate::js::{Expr, Op, Prop, Stmt, StmtKind, UnaryOp};
use crate::lower::calls::{Call, apply};
use crate::lower::drops::Drops;

use crate::lower::recognition::Std;
use crate::lower::std_types::result;
use crate::lower::{FnCx, R};
use rustc_hir::attrs::lang_items::LangItem;
use rustc_hir::def_id::DefId;
use rustc_middle::ty::{self, Ty};
use rustc_span::Span;

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// An `Option`'s or a `Result`'s method, and `bool::then` (ADRs 0030, 0062): `None` if `known` is another.
    pub(in crate::lower) fn option_call(
        &mut self,
        known: Std,
        call: Call<'_, 'tcx>,
        values: &mut std::vec::IntoIter<Expr>,
        boxed: bool,
        out: &mut Vec<Stmt>,
    ) -> R<Option<Expr>> {
        let Call {
            generic_args,
            tys,
            span,
            ..
        } = call;
        let mut arg = || values.next().expect("rustc checked the arguments");
        Ok(Some(match known {
            Std::Then => Expr::bin(Op::Or, arg(), arg()),
            // `Some(x)`, boxed where it may look like `None` (ADR 0051).
            Std::OptionFrom if boxed => self.some(arg()),
            Std::OptionFrom => arg(),
            Std::ThenWith => {
                let (first, next) = (arg(), arg());
                // `then_with(|| a.cmp(b))` is `first || $cmp(a, b)`: the closure's body in place.
                let then = match next.kind {
                    js::ExprKind::Arrow(ref params, ref body) if params.is_empty() => match body.as_slice() {
                        [
                            js::Stmt {
                                kind: StmtKind::Return(Some(value)),
                                ..
                            },
                        ] => value.clone(),
                        _ => Expr::call(next.clone(), vec![]),
                    },
                    _ => Expr::call(next.clone(), vec![]),
                };
                Expr::bin(Op::Or, first, then)
            }
            Std::IsOk(ok) => result::is(arg(), ok),
            Std::UnwrapOk => {
                let mut list: Vec<Expr> = (0..tys.len()).map(|_| arg()).collect();
                if let Some(value) = result::unwrapped_made(&list[0], &list[1..]) {
                    return Ok(Some(value));
                }
                // The error by its own `Debug`, where `$debug`, which knows no
                // types, would show it otherwise: `Missing { name: "ink" }`, not
                // `{ TAG: "Missing", name: "ink" }`, and a parse error, which is
                // its message (ADR 0063), as `ParseIntError { kind: .. }`.
                if let Some(error) = generic_args.types().nth(1) {
                    self.typed_debug(&mut list, error, span)?;
                }
                Expr::call(Expr::helper("$unwrapOk"), list)
            }
            Std::UnwrapErr => {
                let mut list: Vec<Expr> = (0..tys.len()).map(|_| arg()).collect();
                if let Some(value) = generic_args.types().next() {
                    self.typed_debug(&mut list, value, span)?;
                }
                Expr::call(Expr::helper("$unwrapErr"), list)
            }
            // `r.TAG === "Ok" ? r._0 : d`, with `r` computed once, and `d` too,
            // before the test, as Rust does.
            // An `Err` with a destructor is dropped (ADR 0179).
            Std::ResultOk if self.drops(generic_args.type_at(1)) != Drops::Nothing => {
                let result = arg();
                let result = if result.reads_same() {
                    result
                } else {
                    self.spill("result", result, out)
                };
                let ok = result::is(result.clone(), true);
                let value = result::held(result.clone());
                let value = if boxed { self.some(value) } else { value };
                let name = self.fresh("value");
                let then = vec![StmtKind::Assign(Expr::var(&name), value).at(js::Span::NONE)];
                let error = (Expr::member(result, "_0"), generic_args.type_at(1));
                self.or_dropped(ok, name, then, error, Expr::undefined(), span, out)?
            }
            Std::ResultOk | Std::ResultOr => {
                let mut result = arg();
                let fallback = (known == Std::ResultOr).then(arg);
                if let Some(d) = &fallback
                    && let Some(value) = result::unwrapped_made(&result, std::slice::from_ref(d))
                {
                    return Ok(Some(value));
                }
                if result.has_effects() {
                    result = self.spill("result", result, out);
                }
                let otherwise = match fallback {
                    Some(d) if d.has_effects() => self.spill("fallback", d, out),
                    Some(d) => d,
                    None => Expr::undefined(),
                };
                let ok = result::is(result.clone(), true);
                let value = result::held(result);
                let value = if boxed { self.some(value) } else { value };
                Expr::cond(ok, value, otherwise)
            }
            Std::IsSome => self.present(arg(), generic_args.type_at(0)),
            Std::IsNone => self.absent(arg(), generic_args.type_at(0)),
            Std::Unwrap => {
                // `expect` has a message too.
                let list = (0..tys.len()).map(|_| arg()).collect();
                let unwrapped = Expr::call(Expr::helper("$unwrap"), list);
                if self.boxed_payload(generic_args.type_at(0)) {
                    self.some_value(unwrapped)
                } else {
                    unwrapped
                }
            }
            // Rust leaves a `None` undefined behavior: the value, as `o!` is.
            Std::UnwrapUnchecked => {
                let value = arg();
                if self.boxed_payload(generic_args.type_at(0)) {
                    self.some_value(value)
                } else {
                    value
                }
            }
            // `??` skips its right side when it isn't needed, and Rust
            // evaluates it either way: one with effects runs first, in order.
            Std::UnwrapOr => {
                let (mut option, mut default) = (arg(), arg());
                if default.has_effects() {
                    if option.has_effects() {
                        option = self.spill("option", option, out);
                    }
                    default = self.spill("fallback", default, out);
                }
                // Of a generic `T` (ADR 0051): `$someValue(o ?? $some(d))`.
                if self.boxed_payload(generic_args.type_at(0)) {
                    let default = self.some(default);
                    self.some_value(Expr::bin(Op::Coalesce, option, default))
                } else if let Some((kept, value)) = filtered(&option) {
                    Expr::cond(kept, value, default)
                } else {
                    Expr::bin(Op::Coalesce, option, default)
                }
            }
            // `o.map(|x| value)` is `o != null ? value : undefined`, with the
            // option for `x`, read once: `const h = half(n); h != null ? h + 1 : undefined`.
            // A function, or a closure of statements, is called with it.
            Std::OptionMap => {
                let (option, f) = (arg(), arg());
                // By a function giving each variant its own name, the option
                // itself: `section.map(Section::as_str)` is `section` (ADR 0264).
                if let Some((function, _)) = crate::lower::fn_def(tys[1])
                    && self.gives_own_name(function)
                    && !self.boxed_payload(generic_args.type_at(0))
                    && !self.boxed_payload(generic_args.type_at(1))
                {
                    return Ok(Some(option));
                }
                // By what gives it as it is, `map(String::as_str)`, or a binding
                // that's the value itself, `map(js::unknown)`: the option.
                if f.is_identity() {
                    return Ok(Some(option));
                }
                let mapped = generic_args.type_at(1);
                // `|_| 7` has no parameter left (ADR 0038): `Some(None)`.
                let param = match &f.kind {
                    js::ExprKind::Arrow(params, _) if params.len() <= 1 => Some(params.first().cloned()),
                    _ => None,
                };
                let body = match &f.kind {
                    js::ExprKind::Arrow(_, body) => match body.as_slice() {
                        [
                            js::Stmt {
                                kind: StmtKind::Return(Some(value)),
                                ..
                            },
                        ] => Some(value.clone()),
                        _ => None,
                    },
                    _ => None,
                };
                let base = match &param {
                    Some(Some(js::Pattern::Name(name))) => name.clone(),
                    _ => "option".to_string(),
                };
                // What a `filter` kept is its variable, where its test holds.
                let fused = match self.boxed_payload(generic_args.type_at(0)) {
                    false => filtered(&option),
                    true => None,
                };
                let (option, present) = match fused {
                    Some((kept, value)) => (value, kept),
                    None => {
                        // A property of what's in it, `o.map(|e| e.id)`, is `o?.id`,
                        // which reads `o` once, as a `const` of it would.
                        if let (Some(Some(js::Pattern::Name(p))), Some(b)) = (&param, &body)
                            && !self.boxed_payload(generic_args.type_at(0))
                            && !self.boxed_payload(mapped)
                            && let Some(chain) = optional_chain(&option, b, p)
                        {
                            return Ok(Some(chain));
                        }
                        // Read twice: a variable, or a field of a plain Rust value,
                        // `p.title`, as it is; a getter's, in a `const` first.
                        let option = match &option.kind {
                            js::ExprKind::Var(_) => option,
                            js::ExprKind::Member(object, _) if matches!(&object.kind, js::ExprKind::Var(name) if self.plain_value(name)) => {
                                option
                            }
                            _ => self.spill(&base, option, out),
                        };
                        let present = self.present(option.clone(), generic_args.type_at(0));
                        (option, present)
                    }
                };
                // Of a generic `T`, the closure gets what's inside (ADR 0051).
                let option = if self.boxed_payload(generic_args.type_at(0)) {
                    self.some_value(option)
                } else {
                    option
                };
                let value = param.zip(body).and_then(|(param, body)| {
                    let with = |name: &str| match &param {
                        None => None,
                        Some(js::Pattern::Name(p)) => (p == name).then(|| option.clone()),
                        // A closure's parameter is never a function's rest.
                        Some(js::Pattern::Rest(_) | js::Pattern::Defaulted(..)) => None,
                        Some(js::Pattern::Array(items)) => items
                            .iter()
                            .position(|item| item.as_ref().and_then(js::Pattern::name) == Some(name))
                            .map(|i| Expr::index(option.clone(), Expr::int(i as i128))),
                        // The rest, `...rest`, is no field's.
                        Some(js::Pattern::Object(_, Some(rest))) if rest == name => None,
                        Some(js::Pattern::Object(fields, _)) => fields
                            .iter()
                            .find(|(_, var, _)| var.name() == Some(name))
                            .map(|(field, _, _)| Expr::member(option.clone(), field.clone())),
                    };
                    body.substitute(&with)
                });
                let value = match value {
                    Some(value) => value,
                    // Not `((h) => { .. })(h)`: the closure gets a name first.
                    None if matches!(f.kind, js::ExprKind::Arrow(..)) => {
                        let f = self.spill("map", f, out);
                        Expr::call(f, vec![option.clone()])
                    }
                    None => Expr::call(f, vec![option.clone()]),
                };
                let value = if self.boxed_payload(mapped) {
                    self.some(value)
                } else {
                    value
                };
                // Of an `Option` never falsy, `!!o`, as of `o != null`: `o?.m(x)`
                // (ADR 0298); else its test as it is.
                let tested = js::presence_of(&present).map(|o| Expr::bin(Op::LooseNe, o.clone(), Expr::null()));
                match Expr::cond(tested.unwrap_or_else(|| present.clone()), value, Expr::undefined()) {
                    Expr {
                        kind: js::ExprKind::Cond(_, value, none),
                        span,
                        ..
                    } => Expr {
                        reads: false,
                        kind: js::ExprKind::Cond(Box::new(present), value, none),
                        span,
                    },
                    chain => chain,
                }
            }
            // Of a number or text, a handle on the `Option`'s own place: `Some(x)`
            // of one is `x` (ADR 0030), so writing it writes the `Option`.
            Std::OptionIterMut => {
                let item = self.option_of(tys[0].peel_refs()).expect("an `Option` has a `T`");
                let option = arg();
                self.option_items_mut(option, tys[0], item, span, out)?
            }
            Std::OptionIter => {
                let item = self.option_of(tys[0].peel_refs()).expect("an `Option` has a `T`");
                let option = arg();
                self.option_items(option, item, out)
            }
            // An `Option` of an `Option` is the inner one, or its box one
            // level deeper (ADR 0051): `$someValue` takes a level off,
            // `Some(None)` to `None`, and `None` is itself.
            Std::OptionFlatten => self.some_value(arg()),
            // `Some(&x)` is `x`, and its clone is `x`'s.
            Std::OptionCloned => {
                let item = generic_args.types().next().expect("`Option<T>` has a `T`");
                let value = arg();
                let ty = ty::Ty::new_adt(
                    self.tcx,
                    self.tcx.adt_def(self.tcx.require_lang_item(LangItem::Option, span)),
                    self.tcx.mk_args(&[item.into()]),
                );
                self.clone_value(value, ty, span, out)?
            }
            _ => return Ok(None),
        }))
    }

    /// The `debug` a `$unwrapOk` or a `$unwrapErr` is given, after its
    /// message: `ty`'s own `Debug`, unless `$debug`, which knows no types,
    /// shows it as Rust does already, an integer, a `bool`, `()` or a string.
    fn typed_debug(&mut self, list: &mut Vec<Expr>, ty: Ty<'tcx>, span: rustc_span::Span) -> R<()> {
        let peeled = ty.peel_refs();
        if peeled.is_integral()
            || peeled.is_bool()
            || peeled.is_unit()
            || peeled.is_str()
            || self.is_lang_adt(peeled, LangItem::String)
        {
            return Ok(());
        }
        let e = self.fresh("e");
        let shown = self.debug_string(Expr::var(&e), ty, span)?;
        if list.len() == 1 {
            list.push(Expr::undefined());
        }
        // `stockErrorDebug_fmt` itself, not `(e) => stockErrorDebug_fmt(e)`.
        let debug = match &shown.kind {
            js::ExprKind::Call(callee, args)
                if matches!(callee.kind, js::ExprKind::Var(_))
                    && matches!(args.as_slice(), [arg] if matches!(&arg.kind, js::ExprKind::Var(name) if *name == e)) =>
            {
                (**callee).clone()
            }
            _ => Expr::arrow(vec![e.into()], vec![StmtKind::Return(Some(shown)).at(js::Span::NONE)]),
        };
        list.push(debug);
        Ok(())
    }

    /// `Some` of `items[index]`, or `None` if there's none (ADR 0051).
    pub(in crate::lower) fn some_at(&mut self, items: Expr, index: Expr) -> Expr {
        Expr::call(Expr::helper("$someAt"), vec![items, index])
    }

    /// `Some(value)` of what could look like `None` (ADR 0051): `$some(value)`,
    /// or a literal itself where it can't, as `$some(4)` is `4`.
    pub(in crate::lower) fn some(&mut self, value: Expr) -> Expr {
        let nullish = matches!(value.kind, js::ExprKind::Undefined | js::ExprKind::Null);
        let literal = matches!(
            value.kind,
            js::ExprKind::Array(_) | js::ExprKind::Object(_) | js::ExprKind::Template(..)
        );
        if (value.is_constant() && !nullish) || literal {
            return value;
        }
        Expr::call(Expr::helper("$some"), vec![value])
    }

    /// An `Option<T>`'s items, as its `iter()` gives them (ADR 0128):
    /// `option == null ? [] : [option]`.
    pub(in crate::lower) fn option_items(&mut self, option: Expr, item: Ty<'tcx>, out: &mut Vec<Stmt>) -> Expr {
        let option = if option.reads_same() {
            option
        } else {
            self.spill("option", option, out)
        };
        let value = if self.boxed_payload(item) {
            self.some_value(option.clone())
        } else {
            option.clone()
        };
        Expr::cond(
            Expr::bin(Op::LooseEq, option, Expr::null()),
            Expr::array(vec![]),
            Expr::array(vec![value]),
        )
    }

    /// The `&mut`s to what's in an `Option`, of its `&mut`, `option`: of a
    /// number or text a handle on its own place, as `Some(x)` of one is `x`
    /// (ADR 0030), so writing it writes the `Option`; of an object, the
    /// object (ADR 0343).
    pub(in crate::lower) fn option_items_mut(
        &mut self,
        option: Expr,
        option_ref: Ty<'tcx>,
        item: Ty<'tcx>,
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> R<Expr> {
        if !self.is_boxable(item) {
            // The `Option` the `&mut` is to, through its cell.
            let (option, _) = self.through_refs(option, option_ref);
            return Ok(self.option_items(option, item, out));
        }
        if self.boxed_payload(item) {
            return Err(self.unsupported(span, "a `&mut` to what's in an `Option` of an `Option`"));
        }
        let (place, cell) = match option.kind {
            js::ExprKind::Handle(place) => (*place.clone(), Expr::handle(*place)),
            _ => {
                let cell = self.spill("option", option, out);
                (Expr::held(cell.clone()), cell)
            }
        };
        Ok(Expr::cond(
            Expr::bin(Op::LooseEq, place, Expr::null()),
            Expr::array(vec![]),
            Expr::array(vec![cell]),
        ))
    }

    /// What's in an `Option` of a generic `T` (ADR 0051): `$someValue(option)`.
    pub(in crate::lower) fn some_value(&mut self, option: Expr) -> Expr {
        Expr::call(Expr::helper("$someValue"), vec![option])
    }
}

/// What `$someValue(option)` reads, of an `Option` of a generic `T`.
pub(in crate::lower) fn some_value_of(e: &Expr) -> Option<&Expr> {
    match &e.kind {
        js::ExprKind::Call(callee, args) if callee.helper_name() == Some("$someValue") => match args.as_slice() {
            [option] => Some(option),
            _ => None,
        },
        _ => None,
    }
}

/// The field of the box of a `Some` that looks like `None` (ADR 0051),
/// `{ $someNone: 0 }`, how deep a `None` it holds, as the runtime's `$some`
/// makes it.
const SOME_NONE: &str = "$someNone";

/// `Some` of a constant, as `$some` makes it (ADR 0051): itself, or a box
/// where it looks like `None`, `{ $someNone: 0 }`, one deeper for a box.
pub(in crate::lower) fn some_literal(inner: Expr) -> Expr {
    let depth = match &inner.kind {
        js::ExprKind::Undefined | js::ExprKind::Null => 0,
        js::ExprKind::Object(props) => match props.as_slice() {
            [Prop::Field(name, depth)] if name == SOME_NONE => match depth.as_int() {
                Some(n) => n + 1,
                None => return inner,
            },
            _ => return inner,
        },
        _ => return inner,
    };
    Expr::object(vec![Prop::Field(SOME_NONE.into(), Expr::int(depth))])
}

/// Is `value` the box of a `Some` that looks like `None` (ADR 0051), not
/// what's in one: `value.$someNone !== undefined`.
pub(in crate::lower) fn is_some_box(value: Expr) -> Expr {
    Expr::bin(Op::Ne, Expr::member(value, SOME_NONE), Expr::undefined())
}

/// `m ? m : d`, of an option `e` in a `const m` read only there, is
/// `e ?? d`; `m ? m[1] : d`, of a part never `null` or `undefined`, is
/// `e?.[1] ?? d`, which end where `e` is `None` and give `d` (ADR 0309).
/// Its test is whether `m` is there, `m != null`.
pub(in crate::lower) fn nullish_or(
    m: &str,
    option: &Expr,
    test: &Expr,
    yes: &Expr,
    no: &Expr,
    part_never_nullish: bool,
) -> Option<Expr> {
    let tested = match &test.kind {
        js::ExprKind::Binary(Op::LooseNe, tested, null) if matches!(null.kind, js::ExprKind::Null) => tested,
        _ => return None,
    };
    if !matches!(&tested.kind, js::ExprKind::Var(v) if v == m) || no.mentions_var(m) {
        return None;
    }
    let value = match &yes.kind {
        js::ExprKind::Var(v) if v == m => option.clone(),
        _ if part_never_nullish => optional_part(yes, m, option)?,
        _ => return None,
    };
    Some(match no.kind {
        js::ExprKind::Undefined => value,
        _ => Expr::bin(Op::Coalesce, value, no.clone()),
    })
}

/// `part`, a member or index chain of `m`, `m.a[1]`, as one of `option` that
/// ends where it's `None`: `option?.a[1]`.
fn optional_part(part: &Expr, m: &str, option: &Expr) -> Option<Expr> {
    let is_m = |of: &Expr| matches!(&of.kind, js::ExprKind::Var(v) if v == m);
    match &part.kind {
        js::ExprKind::Member(of, field) if is_m(of) => Some(Expr::optional_member(option.clone(), field.clone())),
        js::ExprKind::Index(of, index) if is_m(of) && !index.mentions_var(m) => {
            Some(Expr::optional_index(option.clone(), (**index).clone()))
        }
        js::ExprKind::Member(of, field) => Some(Expr::member(optional_part(of, m, option)?, field.clone())),
        js::ExprKind::Index(of, index) if !index.mentions_var(m) => {
            Some(Expr::index(optional_part(of, m, option)?, (**index).clone()))
        }
        _ => None,
    }
}

/// `body`, a property chain of `param`, `param.a.b`, as one of `option`
/// that ends where it's `None`: `option?.a.b`.
fn optional_chain(option: &Expr, body: &Expr, param: &str) -> Option<Expr> {
    let js::ExprKind::Member(of, field) = &body.kind else {
        return None;
    };
    match &of.kind {
        js::ExprKind::Var(name) if name == param => Some(Expr::optional_member(option.clone(), field.clone())),
        _ => Some(Expr::member(optional_chain(option, of, param)?, field.clone())),
    }
}

/// `x` of `!!x`.
fn truthy_of(test: &Expr) -> Option<&Expr> {
    let js::ExprKind::Unary(js::UnaryOp::Not, inner) = &test.kind else {
        return None;
    };
    match &inner.kind {
        js::ExprKind::Unary(js::UnaryOp::Not, of) => Some(of),
        _ => None,
    }
}

/// A `filter`'s `Option`, `x != null && keep ? x : undefined` of a
/// variable `x`: its test, which holds only where `x` isn't `None`, and
/// `x`. What takes the `Option` next takes them in place of a `const` of
/// it: `filter(..).unwrap_or(d)` is `x != null && keep ? x : d`. A
/// conditional of another `Option`, `c ? maybe() : undefined`, isn't one:
/// its `maybe()` may be `None`.
pub(in crate::lower) fn filtered(option: &Expr) -> Option<(Expr, Expr)> {
    let js::ExprKind::Cond(test, value, none) = &option.kind else {
        return None;
    };
    let (js::ExprKind::Var(name), js::ExprKind::Undefined) = (&value.kind, &none.kind) else {
        return None;
    };
    let present = match &test.kind {
        js::ExprKind::Binary(Op::And, first, _) => first,
        _ => test,
    };
    // `x != null`, or text's `!!x`, which holds only where it isn't.
    let tested = match &present.kind {
        js::ExprKind::Binary(Op::LooseNe, tested, null) if matches!(null.kind, js::ExprKind::Null) => tested,
        _ => truthy_of(present)?,
    };
    let tests_it = matches!(&tested.kind, js::ExprKind::Var(v) if v == name);
    tests_it.then(|| ((**test).clone(), (**value).clone()))
}

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// Whether an `Option<ty>`'s value is never falsy in JS, an object, an
    /// array or a function, so its truth is whether it's there (ADR 0298).
    /// Not a binding's `unknown` or `any`, which may be `0` or `""`.
    pub(in crate::lower) fn never_falsy(&self, ty: Ty<'tcx>) -> bool {
        self.never_falsy_within(ty, &mut Vec::new())
    }

    /// `never_falsy`, of untagged enums `within` one another: one that holds
    /// itself, through a reference, may be.
    fn never_falsy_within(&self, ty: Ty<'tcx>, within: &mut Vec<DefId>) -> bool {
        let ty = ty.peel_refs();
        let any = matches!(ty.kind(), ty::Adt(adt, _)
            if matches!(crate::lower::declarations::written_types(self.tcx, adt.did()).as_deref(), Some("unknown" | "any")));
        // An untagged enum is its payload (ADR 0214): never falsy where each
        // variant's is, an array's or a JS object's; not where one may be
        // text, `""`.
        if let Some((adt, args)) = self.untagged(ty) {
            if within.contains(&adt.did()) {
                return false;
            }
            within.push(adt.did());
            let each = adt.variants().iter().all(|variant| {
                matches!(&variant.fields.raw[..], [field] if self.never_falsy_within(self.field_ty(field, args), within))
            });
            within.pop();
            return each;
        }
        // A `NonZero` is its number (ADR 0177), never `0` or `0n`.
        crate::lower::recognition::is_non_zero_ty(ty)
            || self.option_of(ty).is_none()
                && !any
                && (self.is_object(ty) || matches!(ty.kind(), ty::Closure(..) | ty::FnDef(..) | ty::FnPtr(..)))
    }

    /// Whether an `Option<ty>` is `Some`: `o != null`, or `!!o` of a value
    /// never falsy, which a test reads as `o` (ADR 0298).
    pub(in crate::lower) fn present(&self, option: Expr, ty: Ty<'tcx>) -> Expr {
        match self.never_falsy(ty) {
            true => Expr::unary(UnaryOp::Not, Expr::unary(UnaryOp::Not, option)),
            false => Expr::bin(Op::LooseNe, option, Expr::null()),
        }
    }

    /// Whether it's `None`: `o == null`, or `!o`.
    pub(in crate::lower) fn absent(&self, option: Expr, ty: Ty<'tcx>) -> Expr {
        match self.never_falsy(ty) {
            true => Expr::unary(UnaryOp::Not, option),
            false => Expr::bin(Op::LooseEq, option, Expr::null()),
        }
    }
}

/// An `Option`'s methods that write its place (ADR 0326).
#[derive(Clone, Copy, PartialEq)]
pub(in crate::lower) enum OptionPlaceOp {
    /// `get_or_insert(v)`, `get_or_insert_with(f)` and
    /// `get_or_insert_default()`: what it holds, made where it holds none.
    GetOrInsert,
    GetOrInsertWith,
    GetOrInsertDefault,
    Insert,
    TakeIf,
}

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// An `Option`'s method that writes its place, of the place, a variable
    /// or a field, its item's type and what's given.
    pub(in crate::lower) fn option_place_values(
        &mut self,
        op: OptionPlaceOp,
        place: Expr,
        item: Ty<'tcx>,
        given: Vec<Expr>,
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> R<Expr> {
        let mut given = given.into_iter();
        let js_span = self.js_span(span);
        let through = match self.is_boxable(item) {
            true => Expr::handle(place.clone()),
            false => place.clone(),
        };
        let value = match op {
            OptionPlaceOp::GetOrInsert | OptionPlaceOp::Insert => given.next().expect("its value"),
            OptionPlaceOp::GetOrInsertWith => apply(given.next().expect("its function"), Vec::new()),
            OptionPlaceOp::GetOrInsertDefault => self.default_value(item, span)?,
            // What it holds, taken where `p` of a `&mut` to it holds.
            OptionPlaceOp::TakeIf => {
                let taken = self.fresh("taken");
                let holds = apply(given.next().expect("its predicate"), vec![through]);
                let present = Expr::bin(Op::LooseNe, place.clone(), Expr::null());
                let take = vec![
                    StmtKind::Assign(Expr::var(&taken), place.clone()).at(js_span),
                    StmtKind::Assign(place, Expr::undefined()).at(js_span),
                ];
                out.push(StmtKind::Let(taken.clone(), None).at(js_span));
                out.push(StmtKind::If(Expr::bin(Op::And, present, holds), take, None).at(js_span));
                return Ok(Expr::var(&taken));
            }
        };
        // Of a value with a destructor, the one it doesn't keep dropped,
        // `insert`'s old one, or `get_or_insert`'s given one where it holds
        // one (ADR 0098).
        let drop = match matches!(op, OptionPlaceOp::Insert | OptionPlaceOp::GetOrInsert) {
            true => self.std_drop_function(item, span)?,
            false => None,
        };
        let present = Expr::bin(Op::LooseNe, place.clone(), Expr::null());
        let assign = StmtKind::Assign(place.clone(), value.clone()).at(js_span);
        match (op, drop) {
            (OptionPlaceOp::Insert, Some(drop)) => {
                let dropped = StmtKind::Expr(apply(drop, vec![place.clone()])).at(js_span);
                out.push(StmtKind::If(present, vec![dropped], None).at(js_span));
                out.push(assign);
            }
            (OptionPlaceOp::Insert, None) => out.push(assign),
            (_, drop) => {
                let absent = Expr::bin(Op::LooseEq, place, Expr::null());
                let otherwise = drop.map(|drop| vec![StmtKind::Expr(apply(drop, vec![value])).at(js_span)]);
                out.push(StmtKind::If(absent, vec![assign], otherwise).at(js_span));
            }
        }
        Ok(through)
    }
}
