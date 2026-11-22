//! A std value's `Debug` written into a writer's sink a piece at a time,
//! as core's builders write it (#1652, step 4 of ADR 1135): a leaf by
//! `sink_writes_given`, a sequence by `debug_list`'s `entries`, a set by
//! `debug_set`'s, a map by `debug_map`'s, each entry's key and value, an `Option`
//! and a `Result` by their derived tuples, a tuple by `debug_tuple("")`,
//! each part written by a writing function into the builder's sink, std's
//! `PadAdapter` for `{:#?}` (`$padAdapter`). The builders are the runtime's
//! one model of core's (`debug_builders.js`, #1533).

use super::*;
use crate::lower::temporary_names::Role;

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// What `value`'s `Debug`, of `ty`, writes into a writer's sink, given
    /// the `Formatter`'s `options` and whether it's `{:#?}`, `pretty`, each
    /// known as it runs: a leaf's writes, or a builder's of each part.
    /// `None` of a value whose split isn't followed yet (#1652): a generic
    /// or `dyn` value's, and another std type's.
    pub fn debug_sink_writes(
        &mut self,
        value: Expr,
        ty: Ty<'tcx>,
        (options, pretty): (&Expr, &Expr),
        span: Span,
    ) -> R<Option<Vec<SinkWrite>>> {
        // What it points at, read as `{:?}` reads it: a `&mut i32` a struct
        // holds, its cell's `value` (#1755), a counted `Rc`'s, a `Box`'s.
        let (value, ty) = self.shown_through(value, ty, Std::FmtDebug);
        if let Some(written) = self.sink_writes_given(value.clone(), (Std::FmtDebug, ty), options.clone(), span)? {
            return Ok(Some(written));
        }
        let helper = |name: &'static str, args: Vec<Expr>| Ok(Some(vec![SinkWrite::Helper(name, args)]));
        // A text that leaves the value unread, `()`'s: the value evaluated
        // first, where reading it runs code, a call's (#1756).
        let evaluated = value.has_effects().then(|| value.clone());
        let unread = |text: Expr| evaluated_first(evaluated.clone(), text);
        // A parse error's derived `Debug`, of its message (ADR 0063).
        if self.recognition().is_parse_error(ty)
            && let ty::Adt(adt, _) = ty.kind()
        {
            let name = Expr::str(self.tcx.item_name(adt.did()).as_str());
            return helper("$writeDebugParseError", vec![value, name, pretty.clone()]);
        }
        // A `Layout`'s derived `Debug`, `Layout { size: 8, align: 4 (1 <<
        // 2) }`, its `size` by the options, and a `LayoutError`'s, its name
        // (ADR 0417, #1728).
        if let Some(layout) = self.recognition().layout(ty) {
            return match layout {
                true => helper("$writeDebugLayout", vec![value, pretty.clone(), options.clone()]),
                false => Ok(Some(vec![SinkWrite::Str(unread(Expr::str("LayoutError")))])),
            };
        }
        // `()`'s, `f.pad("()")`.
        if ty.is_unit() {
            return helper("$writePadded", vec![unread(Expr::str("()")), options.clone()]);
        }
        // A `PhantomData<T>`'s, its `write!(f, "PhantomData<{}>",
        // type_name::<T>())`: three writes, the name the one `type_name`
        // gives (ADR 0132), of a type parameter its caller's (ADR 0452).
        if let ty::Adt(_, args) = ty.kind()
            && self.is_lang_adt(ty, LangItem::PhantomData)
        {
            let name = self.type_fact_value(args.type_at(0), crate::lower::recognition::TypeFact::Name, span)?;
            return Ok(Some(vec![
                SinkWrite::Str(unread(Expr::str("PhantomData<"))),
                SinkWrite::Str(name),
                SinkWrite::Str(Expr::str(">")),
            ]));
        }
        // `Some(x)`'s derived tuple, `None` one write.
        if let Some(inner) = self.option_of(ty) {
            let boxed = self.boxed_payload(inner);
            let Some(write) = self.writing_fn(|cx, option| {
                let payload = match boxed {
                    true => cx.some_value(option),
                    false => option,
                };
                cx.debug_sink_writes(payload, inner, (options, pretty), span)
            })?
            else {
                return Ok(None);
            };
            return helper("$writeDebugOption", vec![value, write, pretty.clone()]);
        }
        // `Ok(x)`'s or `Err(e)`'s derived tuple.
        if self.recognition().is_result(ty)
            && !self.is_write_result(ty)
            && let ty::Adt(_, args) = ty.kind()
        {
            let (ok, err) = (args.type_at(0), args.type_at(1));
            let mut writes = Vec::new();
            for part in [ok, err] {
                let Some(write) = self.writing_fn(|cx, result| {
                    cx.debug_sink_writes(rust_js_program::js::result::held(result), part, (options, pretty), span)
                })?
                else {
                    return Ok(None);
                };
                writes.push(write);
            }
            let mut args = vec![value];
            args.extend(writes);
            args.push(pretty.clone());
            return helper("$writeDebugResult", args);
        }
        // A tuple's, `debug_tuple("")`: each field one step.
        if let ty::Tuple(tys) = ty.kind() {
            let mut fields = Vec::new();
            for (i, part) in tys.iter().enumerate() {
                let Some(write) = self.writing_fn(|cx, tuple| {
                    cx.debug_sink_writes(Expr::index(tuple, Expr::int(i as i128)), part, (options, pretty), span)
                })?
                else {
                    return Ok(None);
                };
                fields.push(write);
            }
            return helper(
                "$writeDebugFields",
                vec![value, Expr::str(""), Expr::array(fields), pretty.clone()],
            );
        }
        // A `Cell`'s and a `RefCell`'s, `debug_struct("RefCell")`'s one
        // field, `value`: what it holds, or of a `RefCell` mutably borrowed,
        // `<borrowed>` (ADR 0328).
        let cell = |item| self.recognition().is_std_type(ty, item);
        if let ty::Adt(_, args) = ty.kind()
            && (cell(crate::lower::recognition::StdItem::Cell) || cell(crate::lower::recognition::StdItem::RefCell))
        {
            let refcell = cell(crate::lower::recognition::StdItem::RefCell);
            let inner = args.type_at(0);
            let Some(write) = self.writing_fn(|cx, held| cx.debug_sink_writes(held, inner, (options, pretty), span))?
            else {
                return Ok(None);
            };
            let (shown, write) = match refcell && !matches!(value.kind, js::ExprKind::Handle(..)) {
                true => {
                    let itself = Expr::bool(self.is_value_cell(ty));
                    let borrowed = self.writing_fn(|_, cell| {
                        Ok(Some(vec![SinkWrite::Helper(
                            "$writeBorrowed",
                            vec![cell, write.clone(), itself.clone()],
                        )]))
                    })?;
                    (value, borrowed.expect("a write"))
                }
                false => (Expr::held(value), write),
            };
            let name = if refcell { "RefCell" } else { "Cell" };
            let field = Expr::array(vec![Expr::str("value"), shown, write]);
            return helper(
                "$writeDebugStruct",
                vec![Expr::str(name), Expr::array(vec![field]), pretty.clone()],
            );
        }
        // A set's, `debug_set().entries(..)`, and a map's,
        // `debug_map().entries(..)`, each in the order its `Debug` shows them.
        if let ty::Adt(_, args) = ty.kind()
            && (self.is_set(ty) || self.is_map(ty))
        {
            let items = self.in_order_of(value, ty, span)?;
            let item = args.type_at(0);
            let Some(write) =
                self.writing_fn(|cx, entry| cx.debug_sink_writes(entry, item, (options, pretty), span))?
            else {
                return Ok(None);
            };
            if self.is_set(ty) {
                return helper("$writeDebugSet", vec![items, write, pretty.clone()]);
            }
            let held = args.type_at(1);
            let Some(write_value) =
                self.writing_fn(|cx, entry| cx.debug_sink_writes(entry, held, (options, pretty), span))?
            else {
                return Ok(None);
            };
            return helper("$writeDebugMap", vec![items, write, write_value, pretty.clone()]);
        }
        // An iterator over an array or a slice, `IntoIter([2, 3])`, its
        // `debug_tuple` of what it has left (ADR 0473; #1712); a slice's
        // chunks or windows, their `debug_struct` of it and their size,
        // `Chunks { v: [3], chunk_size: 2 }`, of a value read twice; an
        // `Option`'s iterators and `once(x)`'s, their `debug_struct` of the
        // item left, `IntoIter { inner: Item { opt: .. } }` (#2252).
        if let Some(slice) = self.recognition().shows_its_slice(ty)
            && let crate::lower::recognition::ShownLeft::Item(around) = slice.left
        {
            let (_, opt_ty) = self.slice_left(value.clone(), ty, slice, span)?;
            let Some(write_opt) =
                self.writing_fn(|cx, opt| cx.debug_sink_writes(opt, opt_ty, (options, pretty), span))?
            else {
                return Ok(None);
            };
            // `(f, it) => $writeDebugStruct(f, name, [[field, value of it, write]], pretty)`.
            let struct_of = |name: &str, field: &str, value: Expr, write: Expr| {
                let fields = Expr::array(vec![Expr::array(vec![Expr::str(field), value, write])]);
                SinkWrite::Helper("$writeDebugStruct", vec![Expr::str(name), fields, pretty.clone()])
            };
            let Some(write_item) = self.writing_fn(|cx, it| {
                let (opt, _) = cx.slice_left(it, ty, slice, span)?;
                Ok(Some(vec![struct_of("Item", "opt", opt, write_opt.clone())]))
            })?
            else {
                return Ok(None);
            };
            let write = match around {
                None => struct_of(slice.name, "inner", value, write_item),
                Some(around) => {
                    let Some(write_inner) = self
                        .writing_fn(|_, it| Ok(Some(vec![struct_of(slice.name, "inner", it, write_item.clone())])))?
                    else {
                        return Ok(None);
                    };
                    struct_of(around, "inner", value, write_inner)
                }
            };
            return Ok(Some(vec![write]));
        }
        if let Some(slice) = self.recognition().shows_its_slice(ty)
            && (slice.left == crate::lower::recognition::ShownLeft::Slice || value.reads_same())
        {
            let (_, item) = self.slice_left(value.clone(), ty, slice, span)?;
            let Some(write) =
                self.writing_fn(|cx, entry| cx.debug_sink_writes(entry, item, (options, pretty), span))?
            else {
                return Ok(None);
            };
            let pretty_list = pretty.clone();
            let Some(list) = self.writing_fn(|cx, it| {
                let (left, _) = cx.slice_left(it, ty, slice, span)?;
                Ok(Some(vec![SinkWrite::Helper(
                    "$writeDebugList",
                    vec![left, write, pretty_list],
                )]))
            })?
            else {
                return Ok(None);
            };
            let crate::lower::recognition::ShownLeft::Sized(field) = slice.left else {
                let field = Expr::array(vec![value, list]);
                return helper(
                    "$writeDebugTuple",
                    vec![Expr::str(slice.name), Expr::array(vec![field]), pretty.clone()],
                );
            };
            let usize = self.tcx.types.usize;
            let Some(size_write) =
                self.writing_fn(|cx, size| cx.debug_sink_writes(size, usize, (options, pretty), span))?
            else {
                return Ok(None);
            };
            let size = Expr::call_helper("$partsSize", vec![value.clone()]);
            let fields = vec![
                Expr::array(vec![Expr::str("v"), value, list]),
                Expr::array(vec![Expr::str(field), size, size_write]),
            ];
            return helper(
                "$writeDebugStruct",
                vec![Expr::str(slice.name), Expr::array(fields), pretty.clone()],
            );
        }
        // A range's, which std writes straight to the formatter, no builder:
        // `self.start.fmt(f)?; write!(f, "..")?; self.end.fmt(f)`, each bound
        // by the same options (#1897); and of a `RangeInclusive` that may be
        // exhausted, then `if self.exhausted { write!(f, " (exhausted)")? }`,
        // a write only where it is, tested after the end's (#1963).
        if let Some(kind) = self.range_kind(ty) {
            let mut out = Vec::new();
            let (parts, exhausted) = self.range_state(value.clone(), ty, &mut out);
            if !value.reads_same() || !out.is_empty() {
                return Ok(None);
            }
            let mut writes = Vec::new();
            let mut parts = parts.into_iter();
            let bound = |cx: &mut Self, part: Option<Expr>, writes: &mut Vec<SinkWrite>| -> R<bool> {
                let (Some(part), Some(index)) = (part, cx.range_index(ty)) else {
                    return Ok(false);
                };
                let Some(written) = cx.debug_sink_writes(part, index, (options, pretty), span)? else {
                    return Ok(false);
                };
                writes.extend(written);
                Ok(true)
            };
            if kind.bounds().first() == Some(&"start") && !bound(self, parts.next(), &mut writes)? {
                return Ok(None);
            }
            writes.push(SinkWrite::Str(Expr::str(kind.dots())));
            if kind.bounds().last() == Some(&"end") && !bound(self, parts.next(), &mut writes)? {
                return Ok(None);
            }
            if let Some(exhausted) = exhausted {
                writes.push(SinkWrite::If(
                    exhausted,
                    vec![SinkWrite::Str(Expr::str(" (exhausted)"))],
                    Vec::new(),
                ));
            }
            return Ok(Some(writes));
        }
        // A sequence's, `debug_list().entries(..)`.
        let item = match ty.kind() {
            ty::Array(item, _) | ty::Slice(item) => Some(*item),
            ty::Adt(_, args) if self.is_vec_like(ty) => Some(args.type_at(0)),
            _ => None,
        };
        if let Some(item) = item {
            let Some(write) =
                self.writing_fn(|cx, entry| cx.debug_sink_writes(entry, item, (options, pretty), span))?
            else {
                return Ok(None);
            };
            return helper("$writeDebugList", vec![value, write, pretty.clone()]);
        }
        Ok(None)
    }

    /// A writing function of a value, `(g, value) => { .. }`, its writes
    /// `write` makes of the value, each into `g`, the sink a builder's step
    /// gives it. `None` where `write` has none.
    pub fn writing_fn(&mut self, write: impl FnOnce(&mut Self, Expr) -> R<Option<Vec<SinkWrite>>>) -> R<Option<Expr>> {
        let (sink, value) = (self.fresh(Role::F), self.fresh(Role::Value));
        let Some(writes) = write(self, Expr::var(&value))? else {
            return Ok(None);
        };
        let body = (writes.into_iter())
            .map(|w| w.written_into(&Expr::var(&sink), js::Span::NONE))
            .collect();
        Ok(Some(Expr::arrow(
            vec![js::Pattern::Name(sink), js::Pattern::Name(value)],
            body,
        )))
    }
}
