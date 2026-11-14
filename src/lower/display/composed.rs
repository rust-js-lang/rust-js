//! A std value's `{:?}` of a part that may stop part-way, by an `Err` or a
//! panic where it may be caught (`fmt_may_fail`, `Prints::stops_composed`),
//! written as std's own `Debug` writes it, through its builders, core's
//! `fmt/builders.rs`, the runtime's model (#1533): `Some(x)`'s
//! `debug_tuple("Some")`, a `Vec`'s `debug_list().entries(..)`, a map's
//! `debug_map()`, `Reverse(x)`'s and a `OnceCell`'s `debug_tuple`. Each
//! piece is written into a sink of its own as it's taken, `$composed((f) =>
//! ..)`, so what came before the part that stops is kept: a write's `Err`
//! carries it out (`$fmtWritten`), and in a line, where a panic stops it,
//! the line has it already (#1561, #1530). A range's, which std writes
//! straight to the formatter, `self.start.fmt(f)?; write!(f, "..")?`, with
//! no builder, is written so in the same sink, `f.write(..)` a piece
//! (`composed_written`). One whose `Debug` is the std package's compiled
//! body, an iterator adapter's, `Rev { iter: .. }`, which makes its text
//! whole, is refused, naming #1633.

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
            Ok(this.composed_tuple(name, vec![shown], pretty))
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
                let items = Expr::builtin(Expr::js_global("Array"), "from", vec![items]);
                self.composed_entries(
                    DebugBuilder::Map,
                    items,
                    &[args.type_at(0), args.type_at(1)],
                    span,
                    pretty,
                )?
            }
            // `RefCell { value: .. }`, its value shown while it's borrowed,
            // or `<borrowed>`, as `$showBorrowed` shows it; a `Cell`'s
            // `get()`; each std's `debug_struct` (#1530).
            ty::Adt(_, args) if std(StdItem::RefCell) || std(StdItem::Cell) => {
                let inner = args.type_at(0);
                let held = std(StdItem::Cell) || matches!(value.kind, js::ExprKind::Handle(..));
                let shown = match held {
                    true => self.debug_string_with(Expr::held(subject("cell")), inner, span, pretty)?,
                    false => {
                        let shown = self.debug_string_with(Expr::var("value"), inner, span, pretty)?;
                        let mut given = vec![subject("cell"), arrow_of("value", shown)];
                        if self.is_value_cell(ty) {
                            given.push(Expr::bool(true));
                        }
                        Expr::call_helper("$showBorrowed", given)
                    }
                };
                let name = if self.is_std_type(ty, StdItem::Cell) {
                    "Cell"
                } else {
                    "RefCell"
                };
                let steps = vec![("field", vec![Expr::str("value"), made_when_taken(shown)])];
                let made = self.composed(DebugBuilder::Struct, Some(Expr::str(name)), pretty, steps);
                apply(made, "cell")
            }
            // `Mutex { data: .., poisoned: false, .. }`, as `$showLocked`
            // shows it, through std's `debug_struct` (#1530).
            ty::Adt(_, args) if std(StdItem::Mutex) || std(StdItem::RwLock) => {
                let mutex = std(StdItem::Mutex);
                let shown = self.debug_string_with(Expr::var("value"), args.type_at(0), span, pretty)?;
                let data = Expr::call_helper(
                    "$lockedData",
                    vec![subject("lock"), arrow_of("value", shown), Expr::bool(mutex)],
                );
                let poisoned = join(vec![
                    Expr::str(""),
                    Expr::call_helper("$lockPoisoned", vec![subject("lock")]),
                ]);
                let steps = vec![
                    ("field", vec![Expr::str("data"), made_when_taken(data)]),
                    ("field", vec![Expr::str("poisoned"), made_when_taken(poisoned)]),
                ];
                let name = Expr::str(if mutex { "Mutex" } else { "RwLock" });
                let made = self.composed_to(DebugBuilder::Struct, Some(name), pretty, steps, "finishNonExhaustive");
                apply(made, "lock")
            }
            // `Reverse(..)`, its derived `Debug`'s `debug_tuple` (#1530).
            ty::Adt(_, args) if self.is_reverse(ty) => {
                let held = Expr::index(subject("reverse"), Expr::int(0));
                let made = one(self, "Reverse", held, args.type_at(0))?;
                apply(made, "reverse")
            }
            // `OnceCell(1)` or `LazyCell(1)`, `OnceLock`'s and `LazyLock`'s
            // alike, or `OnceCell(<uninit>)` where it holds none yet: each
            // std's `debug_tuple`, of `format_args!("<uninit>")` where it
            // holds none (#1530).
            ty::Adt(adt, args) if std(StdItem::OnceCell) || std(StdItem::LazyCell) => {
                let type_name = self.tcx.item_name(adt.did()).to_string();
                let item = args.type_at(0);
                let held = Expr::held(subject("cell"));
                let field = match self.is_std_type(ty, StdItem::LazyCell) {
                    true => {
                        let shown = self.debug_string_with(held, item, span, pretty)?;
                        let made = crate::lower::std_types::lazy::is_made(subject("cell"));
                        Expr::cond(made, shown, Expr::str("<uninit>"))
                    }
                    false => {
                        let inside = match self.boxed_payload(item) {
                            true => self.some_value(held.clone()),
                            false => held.clone(),
                        };
                        let shown = self.debug_string_with(inside, item, span, pretty)?;
                        let none = Expr::bin(Op::LooseEq, held, Expr::null());
                        Expr::cond(none, Expr::str("<uninit>"), shown)
                    }
                };
                let made = self.composed_tuple(&type_name, vec![field], pretty);
                apply(made, "cell")
            }
            // `Included(..)`, `Ready(..)`: a derived `Debug`'s `debug_tuple`
            // of each variant, a fieldless one's name (#1530).
            ty::Adt(_, args) if let Some(adt) = self.std_tuple_variants(ty) => {
                self.tuple_variants_debug(adt, args, value.clone(), span, pretty, true)?
            }
            // A slice's iterator, `Iter([..])`, and a `Vec`'s, an array's or a
            // `Drain`'s, `IntoIter([..])`: std's `debug_tuple` of what it has
            // left, its `as_slice()` (ADR 0473, #1530).
            ty::Adt(..) if let Some(type_name) = self.recognition().shows_its_slice(ty) => {
                let item = self.iterator_item(ty).expect("an iterator's item").peel_refs();
                let left = self.stepped_left(subject("it"), ty, false, span)?;
                let shown = self.debug_string_with(left, Ty::new_slice(self.tcx, item), span, pretty)?;
                apply(self.composed_tuple(type_name, vec![shown], pretty), "it")
            }
            // `start..end`, each piece written as it's made (#1530).
            _ if self.range_kind(ty).is_some() => self.range_debug(value.clone(), ty, span, pretty, true)?,
            _ => return Ok(None),
        }))
    }

    /// `$composed((f) => { const tuple = $debugTuple(f, "Reverse"); ..
    /// })` of `name`'s fields, each made as it's taken.
    pub(super) fn composed_tuple(&mut self, name: &str, fields: Vec<Expr>, pretty: &Pretty) -> Expr {
        let steps = fields.into_iter().map(|shown| ("field", vec![made_when_taken(shown)]));
        self.composed(DebugBuilder::Tuple, Some(Expr::str(name)), pretty, steps.collect())
    }

    /// `$composed((f) => { f.write(..); .. })`: `pieces` written straight
    /// to the sink, each made as it's written, as std's `Debug` of a range
    /// writes its bounds and dots to the formatter with no builder, so a
    /// piece that stops part-way leaves those before it written, and those
    /// after it unmade, as `?` returns (#1530).
    pub(in crate::lower) fn composed_written(&mut self, pieces: Vec<Expr>) -> Expr {
        let written = (pieces.into_iter()).map(|piece| {
            StmtKind::Expr(Expr::call(Expr::member(Expr::var("f"), "write"), vec![piece])).at(js::Span::NONE)
        });
        Expr::call_helper("$composed", vec![Expr::arrow(vec!["f".into()], written.collect())])
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
        self.composed_to(kind, type_name, pretty, steps, "finish")
    }

    /// `composed`, finished by `finish`, `finishNonExhaustive` of a lock's
    /// `..`.
    fn composed_to(
        &mut self,
        kind: DebugBuilder,
        type_name: Option<Expr>,
        pretty: &Pretty,
        steps: Vec<(&str, Vec<Expr>)>,
        finish: &str,
    ) -> Expr {
        let (made, role) = self.builder_of(kind, Expr::var("f"), type_name, pretty);
        let builder = self.fresh(role);
        let mut body = vec![StmtKind::Const(builder.clone(), made).at(js::Span::NONE)];
        for (method, args) in steps.into_iter().chain([(finish, Vec::new())]) {
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
