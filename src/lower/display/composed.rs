//! A std value's `{:?}` of a part that may stop part-way, by an `Err` or a
//! panic where it may be caught (`fmt_may_fail`, `Prints::stops_composed`),
//! written as std's own `Debug` writes it, through its builders, core's
//! `fmt/builders.rs`, the runtime's model (#1533): `Some(x)`'s
//! `debug_tuple("Some")`, a `Vec`'s `debug_list().entries(..)`, a map's
//! `debug_map()`. Each piece is written into a sink of its own as it's taken,
//! `$composed((f) => ..)`, so what came before the part that stops is kept:
//! a write's `Err` carries it out (`$fmtWritten`), and in a line, where a
//! panic stops it, the line has it already (#1561, #1530). Another std
//! value of such a part is refused, naming #1530.

use super::*;
use crate::lower::recognition::StdItem;

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// Whether `{:?}` of `ty`, a std value, shows a part that may stop
    /// part-way: one whose writer may give `Err` (`fmt_may_fail`), or that
    /// may panic where a line keeps what came before (`stops_composed`).
    pub(super) fn composed_may_stop(&self, ty: Ty<'tcx>) -> bool {
        let local = ty.ty_adt_def().is_some_and(|adt| adt.did().is_local());
        !local && (self.krate.prints.stops_composed(ty).is_some() || self.fmt_may_fail(self.debug_trait(), ty))
    }

    /// `{:?}` of `value`, a std value of `ty` a part of which may stop
    /// part-way (`composed_may_stop`), through std's builders' model; `None`
    /// of a kind it doesn't write so.
    pub(super) fn composed_debug(&mut self, value: Expr, ty: Ty<'tcx>, span: Span, pretty: &Pretty) -> R<Option<Expr>> {
        let one = |this: &mut Self, name: &str, part: Expr, part_ty: Ty<'tcx>| -> R<Expr> {
            let shown = this.debug_string_with(part, part_ty, span, pretty)?;
            Ok(this.composed(
                DebugBuilder::Tuple,
                Some(Expr::str(name)),
                pretty,
                vec![("field", vec![made_when_taken(shown)])],
            ))
        };
        // Read where it's read, of one that reads the same each time; else
        // as an arrow's parameter, a name of its own, which nothing the
        // builder names inside it shadows, the arrow called with it.
        let name = match value.reads_same() {
            true => None,
            false => Some(self.fresh(crate::lower::temporary_names::Role::Value)),
        };
        let subject = |_: &str| match &name {
            None => value.clone(),
            Some(name) => Expr::var(name),
        };
        let apply = |shown: Expr, _: &str| match &name {
            None => shown,
            Some(name) => Expr::call(arrow_of(name, shown), vec![value.clone()]),
        };
        let std = |item| self.is_std_type(ty, item);
        Ok(Some(match ty.kind() {
            _ if let Some(inner) = self.option_of(ty) => {
                let inside = match self.boxed_payload(inner) {
                    true => self.some_value(subject("value")),
                    false => subject("value"),
                };
                let some = one(self, "Some", inside, inner)?;
                let none = Expr::bin(Op::LooseEq, subject("value"), Expr::null());
                apply(Expr::cond(none, Expr::str("None"), some), "value")
            }
            ty::Adt(_, args) if std(StdItem::Result) => {
                let inside = || crate::js::result::held(subject("result"));
                // Each variant's name, as its derived `Debug` writes it.
                let names: Vec<String> = match ty.kind() {
                    ty::Adt(adt, _) => adt.variants().iter().map(|v| v.name.to_string()).collect(),
                    _ => unreachable!("a `Result`"),
                };
                let ok = one(self, &names[0], inside(), args.type_at(0))?;
                let err = one(self, &names[1], inside(), args.type_at(1))?;
                apply(Expr::cond(result::is(subject("result"), true), ok, err), "result")
            }
            ty::Tuple(tys) => {
                let mut steps = Vec::new();
                for (i, t) in tys.iter().enumerate() {
                    let item = Expr::index(subject("tuple"), Expr::int(i as i128));
                    steps.push((
                        "field",
                        vec![made_when_taken(self.debug_string_with(item, t, span, pretty)?)],
                    ));
                }
                let made = self.composed(DebugBuilder::Tuple, Some(Expr::str("")), pretty, steps);
                apply(made, "tuple")
            }
            ty::Array(item, _) | ty::Slice(item) => {
                self.composed_entries(DebugBuilder::List, value, &[*item], span, pretty)?
            }
            ty::Adt(_, args) if self.is_vec_like(ty) => {
                self.composed_entries(DebugBuilder::List, value, &[args.type_at(0)], span, pretty)?
            }
            ty::Adt(_, args) if self.is_set(ty) => {
                let items = self.in_order_of(value, ty, span)?;
                self.composed_entries(DebugBuilder::Set, items, &[args.type_at(0)], span, pretty)?
            }
            ty::Adt(_, args) if self.is_map(ty) => {
                let items = self.in_order_of(value, ty, span)?;
                let items = Expr::call(Expr::member(Expr::var("Array"), "from"), vec![items]);
                self.composed_entries(
                    DebugBuilder::Map,
                    items,
                    &[args.type_at(0), args.type_at(1)],
                    span,
                    pretty,
                )?
            }
            _ => return Ok(None),
        }))
    }

    /// `$composed((f) => { .. })` of entries, `entries(items, show..)`, a
    /// map's key's and value's apart.
    fn composed_entries(
        &mut self,
        kind: DebugBuilder,
        items: Expr,
        tys: &[Ty<'tcx>],
        span: Span,
        pretty: &Pretty,
    ) -> R<Expr> {
        let mut args = vec![items];
        for (i, &ty) in tys.iter().enumerate() {
            let entry = match tys.len() {
                1 => Expr::var("entry"),
                _ => Expr::index(Expr::var("entry"), Expr::int(i as i128)),
            };
            args.push(arrow_of("entry", self.debug_string_with(entry, ty, span, pretty)?));
        }
        Ok(self.composed(kind, None, pretty, vec![("entries", args)]))
    }

    /// `$composed((f) => { const list = $debugList(f, ..); list.step(..); ..
    /// list.finish(); })`: the builder of `kind`, its `steps`, finished.
    fn composed(
        &mut self,
        kind: DebugBuilder,
        type_name: Option<Expr>,
        pretty: &Pretty,
        steps: Vec<(&str, Vec<Expr>)>,
    ) -> Expr {
        let (made, role) = self.builder_of(kind, Expr::var("f"), type_name, pretty);
        let builder = self.fresh(role);
        let mut body = vec![StmtKind::Const(builder.clone(), made).at(js::Span::NONE)];
        for (method, args) in steps.into_iter().chain([("finish", Vec::new())]) {
            let step = Expr::call(Expr::member(Expr::var(&builder), method), args);
            body.push(StmtKind::Expr(step).at(js::Span::NONE));
        }
        Expr::call_helper("$composed", vec![Expr::arrow(vec!["f".into()], body)])
    }
}

/// `(name) => shown`.
fn arrow_of(name: &str, shown: Expr) -> Expr {
    Expr::arrow(
        vec![name.into()],
        vec![StmtKind::Return(Some(shown)).at(js::Span::NONE)],
    )
}

/// `shown`, made where a step's taken, `() => shown`.
fn made_when_taken(shown: Expr) -> Expr {
    Expr::arrow(Vec::new(), vec![StmtKind::Return(Some(shown)).at(js::Span::NONE)])
}
