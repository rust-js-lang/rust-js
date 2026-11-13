//! An assignment: a place given a value, a `let`'s rest, and the `&mut`s
//! a call of the crate's is given.

use super::*;
use crate::lower::aggregates::Given;
use crate::lower::copies::InPlaceWriter;
use crate::lower::generic_facts::Form;
use crate::lower::temporary_names::{Base, Role};

mod boxed_args;
mod rewritten;

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    pub(super) fn mir_assign(
        &mut self,
        state: &mut State<'_, 'tcx>,
        place: &Place<'tcx>,
        rvalue: &Rvalue<'tcx>,
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> R<()> {
        let span = self.assignment_span(state, rvalue, span);
        // A writer's `Formatter`, which it writes to as `f`, and its
        // `fmt::Result`, `Ok(())` or what a write gave: nothing.
        if let Some((aliases, _)) = &state.writer {
            if place.projection.is_empty()
                && (aliases.contains(&place.local) || state.builders.contains_key(&place.local))
            {
                return Ok(());
            }
            if place.local == mir::RETURN_PLACE {
                return match rvalue {
                    // One held, which may be its error, a `const`'s
                    // `FAILED` or a local's `let r = Err(fmt::Error); r`:
                    // thrown on, as a function's is (`returned_write_result`,
                    // #1011, #1411); an `Ok`'s, nothing.
                    Rvalue::Use(operand, _) => {
                        let value = self.mir_operand(state, operand, out)?;
                        let value = self.value_expr(value, span)?;
                        if crate::lower::display::may_be_write_error(&value) && self.krate.failing.any {
                            self.flush(state, out)?;
                            out.push(
                                StmtKind::Expr(Expr::call_helper("$writeCheck", vec![value])).at(self.js_span(span)),
                            );
                        }
                        Ok(())
                    }
                    Rvalue::Aggregate(kind, _) if matches!(**kind, AggregateKind::Adt(_, VariantIdx::ZERO, ..)) => {
                        Ok(())
                    }
                    // `Err(fmt::Error)`, thrown (ADR 0187).
                    Rvalue::Aggregate(kind, _) if matches!(**kind, AggregateKind::Adt(..)) => {
                        self.flush(state, out)?;
                        let thrown = Expr::call(Expr::helper("$fmtError"), Vec::new());
                        out.push(StmtKind::Expr(thrown).at(self.js_span(span)));
                        Ok(())
                    }
                    _ => Err(self.unsupported(span, "this `fmt::Result`, from its MIR")),
                };
            }
        }
        // The `()` of a block of only comments, `if c { // why }`'s, which
        // MIR has no other statement of: where its comments end, which
        // they're printed before (ADR 0753); and of an arm that does
        // nothing with comments above it, `// Fine.\n Ok(()) => {}`, its
        // (#508).
        if let Rvalue::Use(Operand::Constant(c), _) = rvalue
            && c.ty().is_unit()
            && let Some(end) = crate::lower::sources::comments_end(self.tcx, self.krate.sources, span).or_else(|| {
                let at = self.js_span(span);
                (state.commented.arms.iter()).find_map(|&(block, end)| (block == at).then_some(end))
            })
        {
            out.push(StmtKind::Comments.at(end));
        }
        // A function's `None` it gives at its end, a comment before it,
        // `// Don't return anything\n None`: where it is, which the comment
        // is printed before, as the `return;` of nothing it is goes (ADR
        // 0862).
        if *place == Place::from(mir::RETURN_PLACE)
            && let Rvalue::Aggregate(kind, _) = rvalue
            && let AggregateKind::Adt(did, VariantIdx::ZERO, ..) = **kind
            && self.tcx.is_lang_item(did, LangItem::Option)
            && let Some(at) = crate::lower::sources::commented(self.krate.sources, span)
        {
            out.push(StmtKind::Comments.at(at));
        }
        // What the function gives, a variable of its own nothing reads after:
        // that, handed over uncopied. Not of a `Cell` in it, which
        // a drop after may set. Nor of custom MIR, which may write the
        // variable after it's given, `RET = _5` then `(_5 as A).0 = 9`:
        // rustc writes what a function gives last (#1396).
        let handed;
        let rvalue = match rvalue {
            Rvalue::Use(Operand::Copy(from), retag)
                if *place == Place::from(mir::RETURN_PLACE)
                    && from.projection.is_empty()
                    && state.body.injection_phase.is_none()
                    && (from.ty(state.body, self.tcx).ty).is_freeze(self.tcx, self.typing_env) =>
            {
                handed = Rvalue::Use(Operand::Move(*from), *retag);
                &handed
            }
            _ => rvalue,
        };
        // A named constant only read or lent shared, `XS.contains(&x)`'s: the
        // one there is, uncopied, as nothing changes it.
        // Not what the function gives, which its caller may change.
        if place.projection.is_empty()
            && place.local != mir::RETURN_PLACE
            && let Rvalue::Use(Operand::Constant(c), _) = rvalue
            && let Const::Unevaluated(uv, _) = c.const_
            && uv.promoted.is_none()
            && matches!(self.tcx.def_kind(uv.def), DefKind::Const)
            && self.krate.fns.contains_key(&uv.def)
            && c.ty().is_freeze(self.tcx, self.typing_env)
            && only_read(state.body, place.local)
        {
            let constant = self.named_fn_ref(uv.def, None, c.span);
            return self.mir_store(state, *place, Value::Expr(constant), span, out);
        }
        // A `&mut` a variable keeps to a variable that's a `&mut` itself,
        // `let outer = &mut list`: `list`, where it's used, so `*outer = ..`
        // gives `list` it.
        if place.projection.is_empty()
            && state.locals.user[place.local]
            && state.locals.writes[place.local] == 1
            && !state.facts.pointed().borrowed.contains(&place.local)
            && let Rvalue::Ref(_, BorrowKind::Mut { .. }, target) = rvalue
            && target.projection.is_empty()
            && let ty::Ref(_, pointee, ty::Mutability::Mut) = state.body.local_decls[target.local].ty.kind()
            && !self.mut_ref_form(*pointee).is_cell()
        {
            let list = self.mir_place(state, *target, span, out)?;
            if matches!(list.kind, js::ExprKind::Var(_)) {
                state.refs.insert(place.local, list);
                return Ok(());
            }
        }
        // And to a part of one that's a `&mut` itself, `ref mut x` of an
        // `Option<&mut Foo>`'s: that part, where it's used. Of a place no
        // index variable reads, which borrowck keeps as it is meanwhile.
        if place.projection.is_empty()
            && state.locals.user[place.local]
            && state.locals.writes[place.local] == 1
            && !state.facts.pointed().borrowed.contains(&place.local)
            && let Rvalue::Ref(_, BorrowKind::Mut { .. }, target) = rvalue
            && !target.projection.is_empty()
            && let ty::Ref(_, _, ty::Mutability::Mut) = *target.ty(state.body, self.tcx).ty.kind()
            && !pending_of(state, target.local)
        {
            let part = self.mir_place(state, *target, span, out)?;
            if steady_place(&part) {
                state.refs.insert(place.local, part);
                return Ok(());
            }
        }
        // A `None` only nullable fields read: `null` (ADR 0275).
        if place.projection.is_empty()
            && state.facts.options().null_nones.contains(&place.local)
            && let Rvalue::Aggregate(kind, _) = rvalue
            && let AggregateKind::Adt(_, variant, ..) = **kind
            && variant.as_u32() == 0
        {
            return self.mir_store(state, *place, Value::Expr(Expr::null()), span, out);
        }
        // An `Option` of an `Option` held as its payload (ADR 1017): `None`
        // `undefined`, and a `Some` what it holds, its `None` `null`, as a
        // nullable field's is already, `previousInstance = instance`; of a
        // value never `undefined`, the value (`defined_payload`); of one
        // never `null`, `null` and the value (`undefined_payload`, rd645).
        if place.projection.is_empty()
            && state.facts.options().nested.contains(&place.local)
            && let Rvalue::Aggregate(_, operands) = rvalue
        {
            let form = self.held_form(state.body.local_decls[place.local].ty);
            let made = self.held_made(state, operands, form, out)?;
            return self.mir_store(state, *place, Value::Expr(made), span, out);
        }
        // Of a nested field, held as its payload (#470): read as it's held
        // by one that holds it so, and a reference to it so too.
        if place.projection.is_empty()
            && state.facts.options().nested.contains(&place.local)
            && let Rvalue::Use(Operand::Copy(from) | Operand::Move(from), _) | Rvalue::Ref(_, BorrowKind::Shared, from) =
                rvalue
            && self.nested_field(state.body, from.as_ref())
            // Not a parameter's part its pattern binds, bound there already.
            && state.parts.get(&place.local).is_none_or(|&arg| from.local != arg)
        {
            let held = self.mir_place(state, *from, span, out)?;
            return self.mir_store(state, *place, Value::Expr(held), span, out);
        }
        // A field held where it's made set (#470, rd645): a nested one's
        // `None` no key's `undefined`, a `Some` what it holds, its `None`
        // `null`, and one of a value never `null` its `None` `null`, a
        // `Some` its value; what one holding it so holds, as it is; and the
        // box of what doesn't.
        if self.nested_field(state.body, place.as_ref()) {
            let ty = place.ty(&state.body.local_decls, self.tcx).ty;
            let made = match rvalue {
                Rvalue::Aggregate(kind, operands) if matches!(**kind, AggregateKind::Adt(..)) => {
                    let form = self.held_form(ty);
                    self.held_made(state, operands, form, out)?
                }
                Rvalue::Use(operand, _) if self.holds_nested_operand(state, operand) => {
                    let place = operand.place().expect("a place");
                    self.mir_place(state, place, span, out)?
                }
                _ => {
                    let boxed = self.mir_rvalue(state, rvalue, span, out)?;
                    let boxed = self.value_expr(boxed, span)?;
                    self.held_of_boxed(boxed, ty, out)
                }
            };
            return self.mir_store(state, *place, Value::Expr(made), span, out);
        }
        // One with a flattened field, made outside JSX, an object of its
        // fields and the flattened one's (ADR 0204).
        if let Rvalue::Aggregate(kind, _) = rvalue
            && let AggregateKind::Adt(..) = **kind
            && !state.facts.jsx_props().contains(&place.local)
        {
            let ty = place.ty(&state.body.local_decls, self.tcx).ty;
            if bindings::has_flatten(self.tcx, ty) {
                let made = self.mir_rvalue(state, rvalue, span, out)?;
                let made = self.value_expr(made, span)?;
                let made = Value::Expr(self.flattened_object(made, ty));
                return self.mir_store(state, *place, made, span, out);
            }
        }
        // A part of a parameter its pattern binds, bound there already.
        if let Some(&arg) = state.parts.get(&place.local)
            && place.projection.is_empty()
            && matches!(rvalue, Rvalue::Use(Operand::Move(p) | Operand::Copy(p), _) if p.local == arg)
        {
            return Ok(());
        }
        // A `let`'s pattern's rest, `let P { href, rest } = p` (ADR 0291).
        if place.projection.is_empty() && state.facts.rests().contains(&place.local) {
            return self.mir_let_rest(state, place.local, rvalue, span, out);
        }
        // A flattened keyed enum written whole, its parent's keys (#504).
        if let Some(written) = self.keyed_write(state, place, rvalue, span, out) {
            return written;
        }
        state.binding_var = place.projection.is_empty() && state.locals.user[place.local];
        // A flattened struct a `match`'s pattern binds, which JS's props hold
        // flat, among their other fields.
        if state.binding_var
            && let Rvalue::Use(Operand::Move(read) | Operand::Copy(read), _) = rvalue
            && let Some((base, PlaceElem::Field(field, _))) = read.as_ref().last_projection()
            && bindings::is_flatten_field(
                self.tcx,
                Place::ty_from(base.local, base.projection, &state.body.local_decls, self.tcx).ty,
                field.as_usize(),
            )
        {
            return Err(self.unsupported(
                span,
                "flattened props taken apart here: take them apart where they're given, `fn f(Props { a, anchor }: Props)`",
            ));
        }
        // A pointer nothing reads, which making does nothing (ADR 0407).
        if let Rvalue::RawPtr(..) = rvalue
            && place.projection.is_empty()
            && state.locals.reads[place.local] == 0
            && !state.locals.borrowed[place.local]
        {
            state.binding_var = false;
            return Ok(());
        }
        // A reference given to a call that's given the pointer to its place
        // for it: made as that pointer, `&raw mut` of the place (ADR 1114).
        let as_pointer;
        let rvalue = match rvalue {
            Rvalue::Ref(_, _, of)
                if place.projection.is_empty() && state.facts.pointed().lent_as_pointers.contains(&place.local) =>
            {
                as_pointer = Rvalue::RawPtr(mir::RawPtrKind::Mut, *of);
                &as_pointer
            }
            _ => rvalue,
        };
        let value = self.mir_rvalue(state, rvalue, span, out);
        state.binding_var = false;
        let value = match rvalue {
            Rvalue::Aggregate(..) => value?,
            // What's `null` or a value already, as it is (ADR 0949).
            Rvalue::Use(operand, _) => {
                let nulled = self.nulled(state, operand);
                self.nullable_written(state, *place, value?, nulled, span)?
            }
            _ => self.nullable_written(state, *place, value?, false, span)?,
        };
        // A `#[rust_js::nullable]` field assigned, `p.title = None`: its
        // `None` `null`, as its cell's `set` writes it (ADRs 0772, 0810); a
        // `Some(..)`, or what's `null` or a value already, as it is. MIR
        // assigns what it makes through a temporary, which `nulled` follows.
        // So is a parameter its function names `#[rust_js::nullable(..)]`,
        // set, `children = None`, as React's `mapIntoArray` writes it (ADR
        // 0861), and a field a `get_mut` lent, `*t.types.get_mut() = None`
        // (ADR 0863).
        // What an optional one holds, given another, is as it is (ADR 0810,
        // amended).
        let form = self.place_none_form(state, *place);
        let nullable = form.stores_null();
        let optional = (self.place_field(state, *place)).is_some_and(|field| bindings::is_optional(self.tcx, field));
        let value = match nullable {
            true => {
                let nulled = match rvalue {
                    Rvalue::Use(operand, _) => match self.stored_given(state, operand) {
                        Given::Nulled => true,
                        Given::Optional => optional,
                        _ => false,
                    },
                    _ => false,
                };
                Value::Expr(form.storage().written(self.value_expr(value, span)?, nulled))
            }
            // Written through a reference, a `RefCell`'s borrow of a
            // nullable field, `*this.target_inst.borrow_mut() = target_inst`,
            // whose handle stores as the field does: what's `null` or a
            // value already, a nullable parameter, says so, as it's stored
            // as it is (ADRs 0791, 0861, amended).
            false
                if matches!(place.projection.last(), Some(PlaceElem::Deref))
                    && matches!(rvalue, Rvalue::Use(operand, _) if self.stored_given(state, operand) == Given::Nulled) =>
            {
                let value = self.value_expr(value, span)?;
                let none =
                    crate::lower::representation::NoneForm::Null.trust(crate::lower::representation::Writers::Exact);
                Value::Expr(Expr {
                    facts: none.map_or(value.facts, |none| value.facts.with(none)),
                    ..value
                })
            }
            false => value,
        };
        let value = self.returned_write_result(state, (*place, rvalue), value, span)?;
        self.mir_store(state, *place, value, span, out)
    }

    /// Whether `place` is a parameter of the function whose body this is
    /// that it names `#[rust_js::nullable(..)]`, TypeScript's `T | null`
    /// (ADRs 0275, 0861).
    /// Where an assignment is written, `span`, MIR's, or of one that drops
    /// what its place held, `*types.get_mut() = match ..`, whose MIR is
    /// where its place is: from its place to the end of the value it's
    /// given, the statement a comment inside it is in (ADR 0444, amended).
    fn assignment_span(&self, state: &State<'_, 'tcx>, rvalue: &Rvalue<'tcx>, span: Span) -> Span {
        let Rvalue::Use(Operand::Move(value), _) = rvalue else {
            return span;
        };
        let made = state.body.local_decls[value.local].source_info.span;
        let given = value.projection.is_empty() && made.eq_ctxt(span) && made.lo() >= span.hi();
        let written = || self.tcx.sess.source_map().span_to_snippet(span.between(made)).ok();
        match given && written().is_some_and(|between| between.trim() == "=") {
            true => span.to(made),
            false => span,
        }
    }

    pub(super) fn own_nullable_param(&self, state: &State<'_, 'tcx>, place: Place<'tcx>) -> bool {
        self.own_param_none_form(state, place) == crate::lower::representation::NoneForm::Null
    }

    /// How the function's own parameter `place` is, or a closure's binding of
    /// one, holds a `None` (`representation::param_none_form`).
    pub(super) fn own_param_none_form(
        &self,
        state: &State<'_, 'tcx>,
        place: Place<'tcx>,
    ) -> crate::lower::representation::NoneForm {
        use crate::lower::representation::NoneForm;
        let def_id = state.body.source.def_id();
        let arg = place.local.as_usize();
        if self.nullable_binding(state, place) {
            return NoneForm::Null;
        }
        let own = place.projection.is_empty()
            && (1..=state.body.arg_count).contains(&arg)
            && matches!(
                self.tcx.def_kind(def_id),
                rustc_hir::def::DefKind::Fn | rustc_hir::def::DefKind::AssocFn
            );
        match own {
            true => crate::lower::representation::none_form(
                self.tcx,
                crate::lower::representation::NonePlace::Param(def_id, arg - 1),
            ),
            false => NoneForm::Undefined,
        }
    }

    /// Whether `place` is a binding of a closure's parameter that its
    /// `#[rust_js::nullable(..)]` names, `function::with_this`'s
    /// `react_name` of `(react_name, ..)`, `T | null` as a function's own
    /// nullable parameter is (ADR 0861, amended): by its debug info, the
    /// binding's place, a variable or the tuple's item.
    fn nullable_binding(&self, state: &State<'_, 'tcx>, place: Place<'tcx>) -> bool {
        let Some(owner) = state.body.source.def_id().as_local() else {
            return false;
        };
        if !self.tcx.is_closure_like(owner.to_def_id()) {
            return false;
        }
        // The tuple's item it's bound of, `(_3.0)`, which its binding is
        // made of, as it is: a closure's parameters start after its own
        // captures, MIR's first.
        if let [PlaceElem::Field(item, _)] = place.projection[..]
            && (2..=state.body.arg_count).contains(&place.local.as_usize())
        {
            return bindings::nullable_tuple_item(self.tcx, owner, place.local.as_usize() - 2, item.as_usize());
        }
        let bindings = bindings::nullable_bindings(self.tcx, owner);
        !bindings.is_empty()
            && state.body.var_debug_info.iter().any(|info| {
                matches!(info.value, mir::VarDebugInfoContents::Place(p) if p == place)
                    && bindings
                        .iter()
                        .any(|&(name, span)| name == info.name && span == info.source_info.span)
            })
    }

    /// What's written to a `#[rust_js::nullable]` function's result, where
    /// it's written (ADR 0578): a `None` it makes is `null` already, and a
    /// `Some(x)` is `x` (`null_nones`); a call of a nullable function,
    /// this one or another, a binding too, is `null` for `None` as it is,
    /// which `calls::given_by` marks it (ADRs 0638, 0620, amended); another
    /// `Option`, a call's or a variable's, is
    /// `null` for `undefined`, `heap[0] ?? null`. Each `return` gives what
    /// was written, so it's written where it's made. So is a call's
    /// `Option` that only nullable fields and parameters read
    /// (`null_nones`), `c ? f(c) : null` of `map`'s `c ? f(c) : undefined`,
    /// as they then take it (ADR 0841). What's `nulled`, `null` or a value
    /// already, a nullable field's read, is as it is (ADR 0949).
    pub(super) fn nullable_written(
        &mut self,
        state: &mut State<'_, 'tcx>,
        place: Place<'tcx>,
        value: Value<'tcx>,
        nulled: bool,
        span: Span,
    ) -> R<Value<'tcx>> {
        let read_nullable = place.projection.is_empty()
            && place.local != mir::RETURN_PLACE
            && state.facts.options().null_nones.contains(&place.local);
        use crate::lower::representation::NoneForm;
        // A nullable thread-local's read, or the function's result.
        let form = match read_nullable {
            true => NoneForm::Null,
            false => crate::lower::representation::none_form(
                self.tcx,
                crate::lower::representation::NonePlace::Result(state.body.source.def_id()),
            ),
        };
        if !read_nullable
            && (place != Place::from(mir::RETURN_PLACE) || state.writer.is_some() || form != NoneForm::Null)
        {
            return Ok(value);
        }
        let value = self.value_expr(value, span)?;
        Ok(Value::Expr(form.storage().written(value, nulled)))
    }

    /// `rest = p.rest` of a `let`'s pattern, `let P { href, rest, .. } = p`:
    /// what's left of the props, each of their other fields named,
    /// `const { href: _href, title: _title, ...rest } = p;`, whose names tidy
    /// makes the pattern's where it binds them.
    fn mir_let_rest(
        &mut self,
        state: &mut State<'_, 'tcx>,
        local: Local,
        rvalue: &Rvalue<'tcx>,
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> R<()> {
        let decls = &state.body.local_decls;
        let props = match rvalue {
            Rvalue::Use(Operand::Move(read) | Operand::Copy(read), _) => match read.as_ref().last_projection() {
                Some((base, PlaceElem::Field(field, _))) => {
                    let ty = Place::ty_from(base.local, base.projection, decls, self.tcx).ty;
                    let place = Place {
                        local: base.local,
                        projection: self.tcx.mk_place_elems(base.projection),
                    };
                    bindings::is_rest_field(self.tcx, ty, field.as_usize()).then_some((place, ty))
                }
                _ => None,
            },
            _ => None,
        };
        let Some((place, ty)) = props.filter(|_| !state.locals.declared[local]) else {
            return Err(self.unsupported(span, "this rest of props, from its MIR"));
        };
        let Shape::Object(fields) = self.shape(ty) else {
            return Err(self.unsupported(span, "this rest of props, from its MIR"));
        };
        let value = self.mir_place(state, place, span, out)?;
        let tcx = self.tcx;
        let keys: Vec<&str> = (fields.iter().enumerate())
            .filter(|&(i, _)| !bindings::is_rest_field(tcx, ty, i))
            .map(|(_, (key, _))| key.text())
            .collect();
        let named = (keys.iter().zip(self.left_out(&keys)))
            .map(|(key, unused)| (key.to_string(), js::Pattern::Name(unused), None))
            .collect();
        self.flush(state, out)?;
        state.locals.declared[local] = true;
        out.push(
            StmtKind::Destructure {
                pattern: js::Pattern::Object(named, Some(state.locals.names[local].clone())),
                value,
                mutable: false,
            }
            .at(self.js_span(span)),
        );
        Ok(())
    }

    /// `place = value`: a temporary's value kept for where it's used, a
    /// variable's `const` where it's made once, or an assignment.
    /// Each `&mut` named where it's used, `state.refs`, and each place a
    /// temporary's read in, `state.in_place`, `s.v` of `&mut s.v`, whose
    /// place is through `local`, `x.value`, made of what `local` holds now,
    /// held: `local` is about to be written, `s = o` while `s.v[0]` is
    /// borrowed. Only those read past the write: one that isn't, the `&mut
    /// found` an `if let Some(found) = &mut found` tests, read in the branch
    /// it matched and not in the `else` that writes `found`, holds nothing,
    /// as its name, which that branch would read, is declared in this one.
    fn hold_refs_through(&mut self, state: &mut State<'_, 'tcx>, local: Local, out: &mut Vec<Stmt>) -> R<()> {
        let name = state.locals.names[local].clone();
        let at = state.location();
        let read: IndexSet<Local> = (state.refs.iter())
            .chain(state.in_place.iter())
            .filter(|(_, target)| root_var(target) == Some(name.as_str()))
            .map(|(&l, _)| l)
            .filter(|&l| read_past(state.body, at, l))
            .collect();
        if name.is_empty() || read.is_empty() {
            return Ok(());
        }
        self.flush(state, out)?;
        let held = self.spill(Base::Named(&name), Expr::var(&name), out);
        // A place holds no closure, so `substitute` renames it; else it's
        // refused, never left reading the variable the snapshot stands for
        // (#1525).
        let span = state.body.source_info(at).span;
        for (l, target) in (state.refs.iter_mut()).chain(state.in_place.iter_mut()) {
            if read.contains(l) {
                *target = (target.substitute(&|v| (v == name).then(|| held.clone())))
                    .ok_or_else(|| self.unsupported(span, "a place a closure reads, which can't be renamed"))?;
            }
        }
        Ok(())
    }

    /// Whether `expr`, a field of a variable written again, `shape._0` of a
    /// `mut` parameter, reads the same wherever `local`, a variant's
    /// binding of it, is read: of plain data, and written on no way from
    /// here to a read (`written_before_read`, #121).
    fn settled_until_read(&self, state: &State<'_, 'tcx>, expr: &Expr, local: Local) -> bool {
        let mut root = expr;
        while let js::ExprKind::Member(object, _) = &root.kind {
            if root.facts.has(js::Known::Getter) {
                return false;
            }
            root = object;
        }
        let js::ExprKind::Var(name) = &root.kind else {
            return false;
        };
        let Some(root) = (state.locals.names.iter_enumerated()).find_map(|(l, n)| (n == name).then_some(l)) else {
            return false;
        };
        root != local
            && self.locals.mir.settled.contains(name)
            && !written_before_read(state.body, state.location(), root, local)
    }

    pub(super) fn mir_store(
        &mut self,
        state: &mut State<'_, 'tcx>,
        place: Place<'tcx>,
        value: Value<'tcx>,
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> R<()> {
        // What's made is where its Rust is, for the source map: an array
        // too, `&[key], // path` of a call's arguments, the line's JS
        // (ADR 0444, amended).
        let value = match value {
            Value::Expr(mut e) if e.span.is_none() => {
                e.span = self.js_span(span);
                Value::Expr(e)
            }
            Value::List(items, at) if at.is_none() => Value::List(items, self.js_span(span)),
            Value::View(list, at) if at.is_none() => Value::View(list, self.js_span(span)),
            value => value,
        };
        // A write's result a function gives, of what it called last, that
        // may hold an `Err`, `$writeTry(..)`'s or `$writeOf(..)`'s: thrown on,
        // as `?` of it is, and `undefined` given (ADRs 0187, 0462). A call's
        // own throws it.
        if place == Place::from(mir::RETURN_PLACE)
            && self
                .recognition()
                .is_write_result(state.body.local_decls[mir::RETURN_PLACE].ty)
            && let Value::Expr(value) = &value
            && let js::ExprKind::Call(callee, _) = &value.kind
            && matches!(callee.helper_name(), Some("$writeTry" | "$writeOf"))
            && (state.writer.is_some() || self.write_thrown_here(state))
        {
            self.flush(state, out)?;
            self.fmt_check(value.clone(), self.js_span(span), out);
            if state.writer.is_some() {
                return Ok(());
            }
            return self.mir_store(state, place, Value::Expr(Expr::undefined()), span, out);
        }

        // A union's field written: the one it holds now (ADR 0406).
        if let Some((base, PlaceElem::Field(field, _))) = place.as_ref().last_projection()
            && self.is_union(base.ty(state.body, self.tcx).ty)
        {
            let union_ty = base.ty(state.body, self.tcx).ty;
            let base = Place {
                local: base.local,
                projection: self.tcx.mk_place_elems(base.projection),
            };
            let value = self.value_expr(value, span)?;
            self.flush(state, out)?;
            let union = self.mir_place(state, base, span, out)?;
            let written = self.union_write(union, union_ty, field, value);
            out.push(StmtKind::Expr(written).at(self.js_span(span)));
            return Ok(());
        }
        // What a strip made, read as its test and its cut where it's read
        // (ADR 0642).
        if place.projection.is_empty()
            && let Value::Cut(test, cut, variant) = value
        {
            state.cuts.insert(place.local, (test, cut, variant));
            return Ok(());
        }
        // A local or a field a raw pointer is taken to or into, an object
        // written whole again: written over what it holds, `$assign(p, v)`
        // (#1484, `rewritten.rs`).
        let Some(value) = self.rewritten_write(state, place, value, span, out)? else {
            return Ok(());
        };
        // A strip's cut given to a local read once: read there, `stack =
        // stack.slice(29)`.
        if place.projection.is_empty()
            && state.facts.offsets().taken_in_place(place.local)
            && let Value::Expr(cut) = &value
        {
            state.in_place.insert(place.local, cut.clone());
            return Ok(());
        }
        let local = place.local;
        let place = match self.whole(state.body, &place) {
            true => Place::from(local),
            false => place,
        };
        // A reference reassigned, `x = z`, that a `&mut` named where it's
        // used is through, `x.value` of `&mut *x` made before: what `x` is
        // now, held, so it's still the place it was.
        if place.projection.is_empty() {
            self.hold_refs_through(state, local, out)?;
        }
        // A field's path stored in a local, a getter's `element.isContentEditable`,
        // or a call's, `isItem()` of `isItem(item)`: whether it's a `bool`, as
        // of a place read (`mir_place`).
        if place.projection.is_empty()
            && let Value::Expr(e) = &value
            && let Some(path) = tidy::field_path(e).or_else(|| tidy::call_path(e))
        {
            let bool = state.body.local_decls[local].ty.is_bool();
            *self.locals.mir.paths.entry(path).or_insert(bool) &= bool;
        }
        // Of a binding only a guard reads, the arm's body reads none (ADR 0751).
        if place.projection.is_empty() && (state.locals.names[local].is_empty() || self.unread(state, local)) {
            // Of `()`, or a temporary nothing reads: what made it runs. Of
            // a reference, `let _ = &mut p.y` through `deref_mut`: what makes
            // its place, `deref_mut(p)`, no handle on it.
            let expr = match value {
                Value::Ref(mut place) => {
                    while let js::ExprKind::Member(object, _) = place.kind {
                        place = *object;
                    }
                    place
                }
                value => self.value_expr(value, span)?,
            };
            if expr.has_effects() {
                self.flush(state, out)?;
                out.push(StmtKind::Expr(expr).at(self.js_span(span)));
            }
            return Ok(());
        }
        // What `?` made: the value it tried, read where the branch is.
        if place.projection.is_empty()
            && !state.body.local_decls[local].ty.is_integral()
            && let Value::Branch(tried, ty) = &value
        {
            let mut tried = tried.clone();
            // An `Option`'s held, which is what a `let` binds of it, `let f =
            // xs.find(..)?;`: held under that name, `f`, bound there already.
            // One declared first, written again or a drop flag's, `let d;`:
            // assigned there, `d = it.next()`, not declared again (#425).
            if let js::ExprKind::Var(held) = &tried.kind
                && self.option_of(*ty).is_some_and(|inner| !self.boxed_payload(inner))
                && let Some((bound, from)) = continued_binding(state, local)
                && let Some(stmt) =
                    (out.iter_mut().rev()).find(|s| matches!(&s.kind, StmtKind::Const(n, _) if n == held))
            {
                let name = state.locals.names[bound].clone();
                stmt.kind = match std::mem::replace(&mut stmt.kind, StmtKind::Break(None)) {
                    StmtKind::Const(_, made) if state.locals.declared[bound] => {
                        StmtKind::Assign(Expr::var(&name), made)
                    }
                    StmtKind::Const(_, made) => StmtKind::Const(name.clone(), made),
                    kind => kind,
                };
                tried = Expr::var(&name);
                state.parts.insert(bound, from);
                state.locals.declared[bound] = true;
            }
            state.branches.insert(local, (tried, *ty));
            return Ok(());
        }
        // A `&mut` a temporary holds to a place, a guard's `value` its
        // `deref_mut` gives, `&mut copy.pages`: that place, where it's used
        // and written, which nothing else changes while it's borrowed.
        if place.projection.is_empty()
            && !state.locals.user[local]
            && state.locals.writes[local] == 1
            && let Value::Place(target) | Value::Expr(target) = &value
            && matches!(
                state.body.local_decls[local].ty.kind(),
                ty::Ref(_, _, ty::Mutability::Mut)
            )
            && matches!(target.kind, js::ExprKind::Member(..) | js::ExprKind::Index(..))
            && fixed_place(target)
        {
            state.in_place.insert(local, target.clone());
            return Ok(());
        }
        // A `&mut` a variable keeps, `let first = &mut h.list[0]`: its place
        // fixed where it's borrowed, what it's in and where (`fixed_kept`), so
        // what changes `h.list` later changes no borrow.
        let value = match value {
            Value::Ref(target) if place.projection.is_empty() && state.locals.user[local] => {
                Value::Ref(self.fixed_kept(state, target, out)?)
            }
            value => value,
        };
        // A `&mut` to a place that's always the same, made once: the
        // place, named where it's used, as borrowck keeps it the same.
        // Not one borrowed whole, `&r`, which is a value of its own then.
        if place.projection.is_empty()
            && state.locals.writes[local] == 1
            && !state.facts.pointed().borrowed.contains(&local)
            && let Value::Ref(target) = &value
            && same_place(state, target)
        {
            state.refs.insert(local, target.clone());
            return Ok(());
        }
        // A reference read through one a place holds, `_2 = &(*(*inner).s)`
        // of a `&str` field: borrowck keeps what it refers to the same, not
        // the place that held it, which a write before its read changes,
        // `inner.s = "new"`, where the JS of the place, `inner.s`, would read
        // the new one.
        let rewritten = match state.body.stmt_at(state.location()) {
            rustc_data_structures::either::Either::Left(mir::Statement {
                kind: StatementKind::Assign(assign),
                ..
            }) => match &assign.1 {
                Rvalue::Ref(_, _, from) | Rvalue::CopyForDeref(from) => {
                    through_held_reference(state.body, self.tcx, *from)
                        && written_before_its_read(state.body, state.location(), from.local, local)
                }
                _ => false,
            },
            _ => false,
        };
        // A shared reference a temporary or a variant's binding holds, made
        // once, to a place of a variable nothing changes, `&route.title`: the
        // place, read where it's used (ADR 0023);
        // borrowck keeps it the same meanwhile.
        // And a tuple of them, `format_args!`'s.
        // And a kept `&mut`'s, `&r` of `let r = &mut n`: a handle on its
        // place, which nothing else changes while it's borrowed.
        let held = |value: &Value<'tcx>, ty: Ty<'tcx>| match value {
            Value::Place(target)
                if !rewritten
                    && same_place(state, target)
                    && root_var(target).is_some_and(|root| self.locals.mir.alike.contains(root)) =>
            {
                Some(target.clone())
            }
            Value::Ref(target)
                if matches!(ty.kind(), ty::Ref(_, _, ty::Mutability::Not))
                    && same_place(state, target)
                    && fixed_place(target) =>
            {
                Some(Expr::handle(target.clone()))
            }
            _ => None,
        };
        // A payload's of a temporary keeps its name, which the temporary
        // takes (tidy's `named_temporaries`).
        let of_temporary = matches!(&value, Value::Place(target) if matches!(&target.kind, js::ExprKind::Var(name)
            if (state.locals.names.iter_enumerated()).any(|(l, n)| n == name && !state.locals.user[l])));
        let one = place.projection.is_empty()
            && (!state.locals.user[local] || (state.facts.payloads().contains(&local) && !of_temporary))
            && state.locals.writes[local] == 1
            && !state.locals.borrowed[local];
        // Each item's type, a tuple's field's.
        let ty = state.body.local_decls[local].ty;
        let item_ty = |i: usize| match ty.kind() {
            ty::Tuple(tys) => tys.get(i).copied().unwrap_or(ty),
            _ => ty,
        };
        let places = match &value {
            Value::List(items, at) => (items.iter().enumerate())
                .map(|(i, item)| held(item, item_ty(i)))
                .collect::<Option<Vec<_>>>()
                .map(|items| Value::List(items.into_iter().map(Value::Place).collect(), *at)),
            value => held(value, ty).map(Value::Place),
        };
        if one && let Some(places) = places {
            state.places.insert(local, places);
            return Ok(());
        }
        // One read by its fields, each once, a `format_args!`'s tuple: a list
        // whose items are each read where its field is.
        let parts = match &value {
            Value::List(items, _) => {
                let mut read = state.locals.fields[local].clone();
                read.sort();
                read == (0..items.len()).collect::<Vec<_>>()
            }
            _ => false,
        };
        // A reference to a variant's `RefCell` field that's the property it
        // holds, made once, a pattern's binding of it, `cleanup` of
        // `Kind::Effect { cleanup, .. }`, a derive's too: a handle on that
        // place, read where it's used (`handle_on_kept_field`, #503), each of
        // its borrows the place's. A `Cell`'s is a `const` of its handle,
        // which `read_handles` reads (ADR 0287, amended, #532).
        if place.projection.is_empty()
            && state.locals.writes[local] == 1
            && !state.locals.borrowed[local]
            && !state.named.contains(&local)
            && !state.facts.pointed().locals.contains(&local)
            && let Value::Expr(handle) | Value::Place(handle) = &value
            && self.handle_on_kept_field(handle)
            && refers_to_a_variant_field(state)
            && self.is_std_type(state.body.local_decls[local].ty.peel_refs(), StdItem::RefCell)
        {
            state.in_place.insert(local, handle.clone());
            return Ok(());
        }
        if place.projection.is_empty()
            && !state.facts.pointed().locals.contains(&local)
            && self.foldable(state, local)
            && (state.locals.reads[local] == 1 || parts)
        {
            state.pending.push((local, value));
            return Ok(());
        }
        let expr = self.value_expr(value, span)?;
        let expr = match place.projection.is_empty() {
            true => self.written_knowing_its_back(local, (expr, state.body.local_decls[local].ty), span)?,
            false => expr,
        };
        let js_span = self.js_span(span);
        // A temporary made once of what reads the same anywhere, however
        // often it's read, or what a variant's pattern binds to it: written
        // where it is.
        // A payload's of a temporary keeps its name, which the temporary
        // takes (tidy's `aliases`).
        let of_temporary = matches!(&expr.kind, js::ExprKind::Var(name)
            if (state.locals.names.iter_enumerated()).any(|(l, n)| n == name && !state.locals.user[l]));
        if place.projection.is_empty()
            && !state.facts.pointed().locals.contains(&local)
            && (!state.locals.user[local] || (state.facts.payloads().contains(&local) && !of_temporary))
            && state.locals.writes[local] == 1
            && !state.locals.borrowed[local]
            && !state.named.contains(&local)
            && (self.reads_same_anywhere(&expr, state.body.local_decls[local].ty)
                || self.settled_until_read(state, &expr, local))
        {
            state.in_place.insert(local, expr);
            return Ok(());
        }
        if place.projection.is_empty() && !state.locals.declared[local] {
            // A variable of a type JS can't hold: refused.
            self.check_variable(state, local, span)?;
            self.flush(state, out)?;
            state.locals.declared[local] = true;
            let name = state.locals.names[local].clone();
            // A variable of the program's own `Rc` or `Box` of a closure: a
            // function held as a value, a `const` of its arrow (ADR 0882,
            // amended).
            let mut expr = expr;
            if matches!(expr.kind, js::ExprKind::Arrow(..) | js::ExprKind::AsyncArrow(..))
                && crate::lower::recognition::holds_a_function(self.tcx, state.body.local_decls[local].ty)
                && state.facts.made_pointer(local)
            {
                expr.facts = expr.facts.with(js::Known::Valued);
            }
            // A part borrowed mutably, `ref mut n` of an `Option<i32>`, may be
            // the variable itself in JS, written through the borrow: a `let`,
            // which tidy makes a `const` where nothing assigns it.
            let js_span = state.facts.let_spans().get(&local).copied().unwrap_or(js_span);
            // One a pointer is taken to, a cell (ADR 0407).
            if state.facts.pointed().locals.contains(&local) {
                let cell = Expr::object(vec![js::Prop::Field(js::HELD.into(), expr)]);
                out.push(StmtKind::Const(name, cell).at(js_span));
                return Ok(());
            }
            super::temporaries::note_marks(state, local, &expr);
            out.push(
                match state.locals.borrowed[local] {
                    true => StmtKind::Let(name, Some(expr)),
                    false => StmtKind::Const(name, expr),
                }
                .at(js_span),
            );
            return Ok(());
        }
        // And one of a variable that holds an array's `$iter`, `it.map(f)`,
        // as MIR holds a `Vec`'s `IntoIter` before a closure that copies a
        // field of a value with a destructor is made (#780).
        let of_held_items = || {
            crate::lower::iterators::mapped_var(&expr).is_some_and(|name| {
                (state.locals.names.iter_enumerated())
                    .find(|(_, n)| *n == name)
                    .is_some_and(|(source, _)| self.steps_an_array(state.body.local_decls[source].ty))
            })
        };
        if place.projection.is_empty()
            && !state.locals.user[local]
            && (crate::lower::iterators::is_items_map(&expr)
                || of_held_items()
                || crate::lower::iterators::source_mapped_as_it_is(&expr).is_some())
        {
            state.iter_locals.maps.insert(local, expr.clone());
        }
        // The value is made before the place is reached, as Rust makes them:
        // but of a variable's fields, the object JS reaches first is the
        // one, which what makes the value can't make another, a closure's
        // `more = ..` assigning it in place: `more.has = work();` (ADR 0607).
        // And of a place JS reaches as a variable, a closure's capture's
        // `*(_1.0)`, `result = callback();`, which JS sets once the value is
        // made, nothing reached before it (ADR 0879). And of a place of
        // variables each given one value, `lanes[index] = ..`, which what
        // makes the value can't make another (ADR 0897).
        let mut value = expr;
        let fixed = (place.projection.iter()).all(|elem| matches!(elem, PlaceElem::Field(..)));
        let target = if !place.projection.is_empty() && value.has_effects() && !fixed {
            let mut reached = Vec::new();
            let target = self.mir_place(state, place, span, &mut reached)?;
            if !reached.is_empty() || !(matches!(target.kind, js::ExprKind::Var(_)) || settled_place(state, &target)) {
                value = self.spill(Role::Value, value, out);
            }
            out.extend(reached);
            target
        } else {
            self.mir_place(state, place, span, out)?
        };
        // A whole cell written to a field that's the property it holds, a
        // `Cell`'s, a struct's `RefCell`'s (`property_cell`): the property
        // what the cell holds, `wrap.n = 9` of `wrap.n = RefCell::new(9)`.
        let (target, value) = match self.whole_property_cell(state, place) {
            true => (Expr::held(target), Expr::held(value)),
            false => (target, value),
        };
        // A field held as generic code's box, written whole: the object's
        // in-place cell, `slot.at = $cellOf(other)` (#1405).
        let value = match self.boxed_field(state, place.as_ref()) {
            Some(crossing) => self.form_given(Form::Box, crossing, value, span)?,
            None => value,
        };
        self.flush(state, out)?;
        // `*r = v` of a `&mut` to an object: the object becomes `v`, so each
        // name for it sees it. Not of one MIR has as its place,
        // `place_reference`'s, `v[i]` of a `Vec`'s `index_mut` written at
        // once, which JS reaches as that place: the place is `v` (#349). A
        // `&mut` read from where one's kept, `result._0` of `get_mut(k)
        // .ok_or(..)?`, `holder.account`, is the object's, and its slot
        // isn't the object's place (#466).
        let decls = &state.body.local_decls;
        let names_place = |base: mir::PlaceRef<'tcx>| {
            base.projection.is_empty()
                && state.facts.place_reference(base.local)
                && matches!(target.kind, js::ExprKind::Member(..) | js::ExprKind::Index(..))
        };
        // Or through a box a `&mut` has the form of what it holds, `**b = v`
        // of a `&mut Box<I>` of an iterator (`mut_ref_form`, #1314): the
        // `&mut`'s, its box no place of its own.
        let through_box = |base: mir::PlaceRef<'tcx>| {
            let boxed = Place::ty_from(base.local, base.projection, decls, self.tcx).ty;
            boxed.boxed_ty().is_some()
                && !self.mut_ref_form(boxed).is_cell()
                && base.last_projection().is_some_and(|(outer, elem)| {
                    elem == PlaceElem::Deref
                        && matches!(
                            Place::ty_from(outer.local, outer.projection, decls, self.tcx).ty.kind(),
                            ty::Ref(_, _, ty::Mutability::Mut)
                        )
                })
        };
        if let Some((base, PlaceElem::Deref)) = place.as_ref().last_projection()
            && (matches!(
                Place::ty_from(base.local, base.projection, decls, self.tcx).ty.kind(),
                ty::Ref(_, _, ty::Mutability::Mut)
            ) || through_box(base))
            && self.mut_ref_form(place.ty(decls, self.tcx).ty).written_in_place()
            && !names_place(base)
        {
            let ty = place.ty(decls, self.tcx).ty;
            if let Some(writes) = self.kept_items_written(&target, &value, ty, js_span, out) {
                out.extend(writes);
                return Ok(());
            }
            let assigned = self.in_place(InPlaceWriter::Assign, ty, vec![target, value], span)?;
            out.push(StmtKind::Expr(assigned).at(js_span));
            return Ok(());
        }
        // `*r = v` of a `&mut` the variable `r` is, to what's neither an object
        // nor a cell, a closure: JS would only give `r` another, so it's
        // refused (not an alias `r` names a place by, `*outer = ..`).
        if let Some((base, PlaceElem::Deref)) = place.as_ref().last_projection()
            && base.projection.is_empty()
            && matches!(target.kind, js::ExprKind::Var(ref name) if *name == state.locals.names[base.local])
        {
            return Err(self.unsupported(span, "assigning a whole value through a `&mut`"));
        }
        // What an `Option`'s box holds, `$someValue(o)` read through
        // `$someMut`: the `Option` given a `Some` of it (ADR 0486).
        if let js::ExprKind::Call(callee, args) = &target.kind
            && callee.helper_name() == Some("$someValue")
            && let [option] = args.as_slice()
        {
            let some = self.some(value);
            out.push(StmtKind::Assign(option.clone(), some).at(js_span));
            return Ok(());
        }
        if place.projection.is_empty() {
            super::temporaries::note_marks(state, local, &value);
        }
        // `*p = v` of a pointer made where it's written, `*(&raw mut v) =
        // ..`: of an object JS holds as itself, the object's in place (#1352).
        let written = place.ty(&state.body.local_decls, self.tcx).ty;
        out.push(self.written_through(target, value, written, span)?.at(js_span));
        Ok(())
    }

    /// `*r = v` of an array `r` that `v`, an array written out, keeps some
    /// items of, each its own read where it is, `r[0]` first: the writes of
    /// the others, as upstream React shrinks a replay node, `node.length =
    /// 4; node[2] = []; node[3] = null;`, and grows one, `node[4] =
    /// fallback; node[5] = id;` (rd490). Its `length` is written first,
    /// but where no value of `ty` has more items and the last is written,
    /// which makes it that. What reads more than variables is held first,
    /// as `$assign` reads each item before it writes any. None where none
    /// is kept: `$assign(r, [..])`.
    fn kept_items_written(
        &mut self,
        target: &Expr,
        value: &Expr,
        ty: Ty<'tcx>,
        js_span: js::Span,
        out: &mut Vec<Stmt>,
    ) -> Option<Vec<Stmt>> {
        let js::ExprKind::Array(items) = &value.kind else {
            return None;
        };
        if !matches!(target.kind, js::ExprKind::Var(_)) {
            return None;
        }
        let kept = |i: usize, item: &Expr| {
            matches!(&item.kind, js::ExprKind::Index(object, index)
                if object.kind == target.kind && index.as_int() == Some(i as i128))
        };
        if !items.iter().enumerate().any(|(i, item)| kept(i, item)) {
            return None;
        }
        let mut writes = Vec::new();
        let len = items.len();
        let last_written = items.last().is_some_and(|item| !kept(len - 1, item));
        if !(last_written && self.most_items(ty).is_some_and(|most| most <= len)) {
            let length = Expr::member(target.clone(), "length");
            writes.push(StmtKind::Assign(length, Expr::int(len as i128)).at(js_span));
        }
        for (i, item) in items.iter().enumerate() {
            if kept(i, item) {
                continue;
            }
            let item = match item.reads_only_vars() || item.is_constant() {
                true => item.clone(),
                false => self.spill(Role::Value, item.clone(), out),
            };
            let at = Expr::index(target.clone(), Expr::int(i as i128));
            writes.push(StmtKind::Assign(at, item).at(js_span));
        }
        Some(writes)
    }

    /// The arguments of a call of the crate's: a `&mut` to a value JS can't
    /// change in place is a box, named for its parameter, copied back once
    /// the call returns (ADR 0074). What's given before it that does
    /// something runs first, before the box reads the place.
    /// The box `place` is the `value` of, given on as it is where a `&mut`
    /// to it goes in a box: one this function was given, `dest` of
    /// `dest.value`, and a cell a place holds, a reborrow's, `&mut **self`
    /// of an impl's for `&mut T`; not a box of a copy. `None` of another.
    pub(super) fn box_of_place(&self, state: &State<'_, 'tcx>, place: &Expr) -> Option<Expr> {
        if let js::ExprKind::Member(boxed, field) = &place.kind
            && field == js::HELD
            && let js::ExprKind::Var(name) = &boxed.kind
            && state.naming.boxes().contains(name)
        {
            return Some((**boxed).clone());
        }
        self.held_cell(place)
    }

    /// What a call of `def_id` given `generic_args` calls, whose parameters
    /// say what it's given: a trait's method resolved to its impl's, or the
    /// crate's trait's own, a dictionary's; `None` of a `dyn`'s, whose pair
    /// is its `&mut`, and of std's (`boxed_args`). Each question of what a
    /// call is given asks it, `lent_as_pointers` too (ADR 1114).
    pub(in crate::lower) fn call_callee(
        &self,
        def_id: rustc_span::def_id::DefId,
        generic_args: ty::GenericArgsRef<'tcx>,
    ) -> R<Option<(rustc_span::def_id::DefId, ty::GenericArgsRef<'tcx>)>> {
        // What's called: a trait's method resolved to its impl's, whose
        // parameters say which are boxes, or the crate's trait's own, a
        // dictionary's, and `Hash`'s with no impl here, a type only a
        // caller knows (a known type's std impl is a std call, lowered or
        // refused before), whose hasher is a `&mut H` of its own (ADR
        // 0463), and `Extend`'s, whose
        // `&mut Self` its dictionary takes in a box, an `I::Item`'s that's an
        // `IntoIterator` too (ADR 0478); not a `dyn`'s, whose pair is its
        // `&mut`, nor std's, but a provided method std's body is compiled
        // of, `scan`'s, whose function it gives its state's box (ADR 0933);
        // but where a std impl whose body is compiled writes the method
        // itself, `Chain`'s `read_until`, that one, which takes its `&mut
        // self` as its own body does, the object, not the provided method's
        // box of a `Self` (#1156).
        let compiled_std = |id| crate::lower::recognition::is_compiled_std(self.tcx, id);
        Ok(match self.tcx.trait_of_assoc(def_id) {
            None => Some((def_id, generic_args)),
            Some(_)
                if let Some(instance) = self.resolve_instance(def_id, generic_args)?
                    && instance.def_id() != def_id
                    && compiled_std(instance.def_id())
                    && self.tcx.trait_of_assoc(instance.def_id()).is_none() =>
            {
                Some((instance.def_id(), instance.args))
            }
            Some(_) if compiled_std(def_id) => Some((def_id, generic_args)),
            Some(trait_id) => match self.impl_method(def_id, generic_args)? {
                Some(method) => Some(method),
                None if (self.krate.foreign.compiled_by_rust_js(trait_id)
                    || is_std_def(self.tcx, trait_id, StdItem::Hash)
                    || crate::lower::recognition::is_extend(self.tcx, trait_id))
                    && !matches!(generic_args.type_at(0).kind(), ty::Dynamic(..)) =>
                {
                    Some((def_id, generic_args))
                }
                None => None,
            },
        })
    }

    /// A variable the program declares, refused: of what drops as JS can't drop
    /// it, or of a type JS can't hold. Not std's MIR put in the body,
    /// nor a JS iterator.
    /// Whether nothing reads `local`: a temporary, or a binding a guarded
    /// arm's pattern makes that only the guard reads, `first` of
    /// `Kind::Array(&[first, _]) if f(first) => ..`, whose arm's body reads
    /// none (ADR 0751). What's made into it is made for what making it does.
    pub(super) fn unread(&self, state: &State<'_, 'tcx>, local: Local) -> bool {
        let user = state.locals.user[local] && !locals::bound_for_a_guarded_arm(state.body, local);
        !user && state.locals.reads[local] == 0 && !state.locals.borrowed[local]
    }

    pub(super) fn check_variable(&self, state: &State<'_, 'tcx>, local: Local, span: Span) -> R<()> {
        let ty = state.body.local_decls[local].ty;
        if !state.locals.user[local] || inlined(state.body, local) || self.implements_iterator(ty) {
            return Ok(());
        }
        self.check_held_drops(ty, span)?;
        self.check_value_ty(ty, span)
    }

    /// `place`, a `&mut` a variable keeps, fixed: the object it's in held
    /// where it may change while it's borrowed, `const list = h.list`. Only
    /// a reference can, a variable of one written again too; MIR holds an
    /// index already.
    fn fixed_kept(&mut self, state: &mut State<'_, 'tcx>, place: Expr, out: &mut Vec<Stmt>) -> R<Expr> {
        // A cell made here, `leak()`'s `{ value: s }`: only it has it.
        if let js::ExprKind::Member(object, _) = &place.kind
            && matches!(object.kind, js::ExprKind::Object(..))
        {
            return Ok(place);
        }
        let (object, part) = match place.kind {
            js::ExprKind::Member(object, field) => (*object, Ok(field)),
            js::ExprKind::Index(object, index) => (*object, Err(*index)),
            kind => return Ok(Expr { kind, ..place }),
        };
        let stays = match &object.kind {
            js::ExprKind::Var(name) => (state.locals.names.iter_enumerated())
                .find(|(_, n)| *n == name)
                .is_none_or(|(l, _)| state.locals.writes[l] <= 1 || !state.body.local_decls[l].ty.is_ref()),
            _ => object.is_constant(),
        };
        let object = match stays {
            true => object,
            false => {
                self.flush(state, out)?;
                let field = match &object.kind {
                    js::ExprKind::Member(_, field) => Some(crate::lower::camel_case(field)),
                    _ => None,
                };
                let name = field.as_deref().map_or(Base::Role(Role::Item), Base::Named);
                // A name the program didn't write, which tidy reads as what
                // it holds where that's a variable nothing sets meanwhile,
                // a flattened state's `chunk` (`alias_variables`, rd482).
                self.held_in_place(name, object, out)
            }
        };
        Ok(match part {
            Ok(field) => Expr::member(object, field),
            Err(index) => Expr::index(object, index),
        })
    }
}

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// `place`, a field or an item, fixed: what holds it named, where it
    /// isn't fixed, an object that's the same however it's reached, and an
    /// index that isn't a constant or a name named too.
    pub(in crate::lower) fn fixed_place_of(&mut self, place: Expr, out: &mut Vec<Stmt>) -> Expr {
        let (span, facts) = (place.span, place.facts);
        // What holds it, named, a temporary of MIR's lowering's, which tidy
        // reads in place where it's read once.
        let mut fixed = |e: Expr, name: Base<'_>, this: &mut Self| match fixed_place(&e) {
            true => e,
            false => this.held_in_place(name, e, out),
        };
        // The same place, what's known of it kept: a nullable field's read
        // never `undefined` (ADR 0620, amended).
        let fixed = match place.kind {
            js::ExprKind::Member(object, field) => {
                Expr::member(fixed(*object, Role::Item.into(), self), &field).or_at(span)
            }
            js::ExprKind::Index(object, index) => {
                let object = fixed(*object, Role::Items.into(), self);
                let index = match index.is_constant() || matches!(index.kind, js::ExprKind::Var(_)) {
                    true => *index,
                    false => self.spill(Role::At, *index, out),
                };
                Expr::index(object, index).or_at(span)
            }
            kind => return Expr { kind, ..place },
        };
        Expr { facts, ..fixed }
    }
}

/// Whether the statement being lowered makes a reference to a variant's
/// field, `_5 = &((*_1) as Effect).0`, a pattern's binding of it. Not a
/// struct's, which a `let`'s pattern destructures as it's written.
fn refers_to_a_variant_field(state: &State<'_, '_>) -> bool {
    matches!(
        state.body.stmt_at(state.location()),
        rustc_data_structures::either::Either::Left(mir::Statement {
            kind: StatementKind::Assign(assign),
            ..
        }) if matches!(&assign.1, Rvalue::Ref(_, mir::BorrowKind::Shared, from)
            if matches!(from.projection.as_slice(), [.., PlaceElem::Downcast(..), PlaceElem::Field(..)]))
    )
}

/// Whether `local` holds a value made and not yet written, a call's
/// `_3 = same(r)`: reading it takes that value, so a read that may be
/// thrown away, a place tried as an alias, would lose the call (#1580).
fn pending_of(state: &State<'_, '_>, local: Local) -> bool {
    state.pending.iter().any(|(l, _)| *l == local)
}
