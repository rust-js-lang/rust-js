//! serde's data model, of a hand-written `Serialize` (ADR 0459): its
//! serializer is the JSON writer, `s.serialize_str(x)` is `s.string(x)`, and
//! a struct, a sequence or a map it begins is the writer too, its fields and
//! elements written in turn. Each call gives `Ok`, as serde_json's writing
//! to a string does; an error thrown in it is the codec's, as a derived one's.

use crate::js::{Expr, StmtKind};
use crate::lower::recognition::{
    SerdeCompound, SerdeError, SerdeHint, SerdeNext, SerdeOp, SerdeScalar, VisitorMethod, visitor_methods,
};
use crate::lower::std_types::result;
use crate::lower::{FnCx, R};
use rustc_middle::ty;
use rustc_span::Span;

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// `op` of a hand-written impl, of its arguments' `values` and its call's
    /// `args`: the writer's steps as statements, and what the call gives.
    pub(in crate::lower) fn serde_call(
        &mut self,
        op: SerdeOp,
        args: ty::GenericArgsRef<'tcx>,
        mut values: Vec<Expr>,
        span: Span,
        out: &mut Vec<crate::js::Stmt>,
    ) -> R<Expr> {
        let step = |json: &Expr, method: &str, given: Vec<Expr>| {
            StmtKind::Expr(Expr::call(Expr::member(json.clone(), method), given)).at(crate::js::Span::NONE)
        };
        // The method's own type parameter, a value's `T`, after its trait's `Self`.
        let own = |at: usize| args.type_at(at);
        let done = result::ok(Expr::undefined());
        if op == SerdeOp::Custom {
            let message = self.display_string(values.remove(0), own(1), span)?;
            return Ok(Expr::call(Expr::helper("$jsonError"), vec![message]));
        }
        // A compound's `serialize_field` and the rest take `&mut self`, a box
        // in generic code (ADR 0454): the writer in it.
        // `T::deserialize(d)`: the reader's read of `T`, `Ok`, or what it
        // throws, `Err`.
        if op == SerdeOp::Deserialize {
            let reader = self.json_reader(args.type_at(0), span)?;
            return Ok(Expr::call(Expr::helper("$jsonRead"), vec![values.remove(0), reader]));
        }
        if let SerdeOp::DeError(error) = op {
            return Ok(self.serde_error(error, values));
        }
        // `seq.next_element::<T>()` and a map's: of the access its `&mut
        // self`'s box holds (ADR 0454), each type after its own and the
        // deserializer's lifetime.
        if let SerdeOp::Next(next) = op {
            let access = Expr::held(values.remove(0));
            let mut own = args.types().skip(1);
            return self.serde_next(next, access, (own.next(), own.next()), span);
        }
        if let SerdeOp::Visit(hint) = op {
            // The visitor is a hint's last argument, after `deserialize_struct`'s
            // name and fields.
            let visitor = values.pop().expect("a visitor");
            // Its visitor's type, after the deserializer's and its lifetime.
            let visitor_ty = args.types().nth(1).expect("`deserialize_*::<V>`");
            return self.serde_visit(hint, values.remove(0), (visitor, visitor_ty), span, out);
        }
        let json = match op {
            SerdeOp::Element | SerdeOp::Field | SerdeOp::SkipField | SerdeOp::Entry | SerdeOp::Key | SerdeOp::Value => {
                Expr::held(values.remove(0))
            }
            _ => values.remove(0),
        };
        match op {
            SerdeOp::Write(scalar) => {
                let method = match scalar {
                    SerdeScalar::Bool => "bool",
                    SerdeScalar::Int => "int",
                    SerdeScalar::Float { .. } => "number",
                    SerdeScalar::Char => "char",
                    SerdeScalar::Str => "string",
                    SerdeScalar::Null => "null",
                };
                // A unit struct's name isn't written; an `f32` its own digits.
                let mut given = match scalar {
                    SerdeScalar::Null => Vec::new(),
                    _ => vec![values.remove(0)],
                };
                if scalar == (SerdeScalar::Float { single: true }) {
                    given.push(Expr::bool(true));
                }
                out.push(step(&json, method, given));
            }
            SerdeOp::WriteValue { at } => {
                let value = values.remove(at - 1);
                self.serde_write(value, own(1), &json, span, out)?;
            }
            SerdeOp::UnitVariant => out.push(step(&json, "variant", vec![values.remove(2)])),
            SerdeOp::NewtypeVariant => {
                let value = values.remove(3);
                out.push(step(&json, "beginObject", Vec::new()));
                out.push(step(&json, "key", vec![values.remove(2)]));
                self.serde_write(value, own(1), &json, span, out)?;
                out.push(step(&json, "endObject", Vec::new()));
            }
            SerdeOp::CollectStr => {
                let shown = self.display_string(values.remove(0), own(1), span)?;
                out.push(step(&json, "string", vec![shown]));
            }
            SerdeOp::Begin(compound) => {
                // A variant's, `{ "Variant": .. }`, its name after the type's
                // and its index.
                if matches!(compound, SerdeCompound::TupleVariant | SerdeCompound::StructVariant) {
                    out.push(step(&json, "beginObject", Vec::new()));
                    out.push(step(&json, "key", vec![values.remove(2)]));
                }
                let method = match compound {
                    SerdeCompound::Seq | SerdeCompound::TupleVariant => "beginArray",
                    SerdeCompound::Tuple => "beginTuple",
                    SerdeCompound::TupleStruct => "beginTupleStruct",
                    SerdeCompound::Map | SerdeCompound::Struct | SerdeCompound::StructVariant => "beginObject",
                };
                out.push(step(&json, method, Vec::new()));
                return Ok(result::ok(json));
            }
            // `collect_seq::<I>(items)`, and `collect_map::<K, V, I>(entries)`.
            SerdeOp::CollectSeq => {
                let items_ty = own(1);
                let items = self.as_js_iterator(values.remove(0), items_ty, span)?;
                let write = self.json_writer(self.iterable_item(items_ty), span)?;
                let call = Expr::call(Expr::helper("$jsonCollectSeq"), vec![json, items, write]);
                out.push(StmtKind::Expr(call).at(crate::js::Span::NONE));
            }
            SerdeOp::CollectMap => {
                let entries = self.as_js_iterator(values.remove(0), own(3), span)?;
                let key = self.fresh("key");
                let text = self.json_key(Expr::var(&key), own(1), span)?;
                let key = Expr::arrow(
                    vec![key.into()],
                    vec![StmtKind::Return(Some(text)).at(crate::js::Span::NONE)],
                );
                let write = self.json_writer(own(2), span)?;
                let call = Expr::call(Expr::helper("$jsonCollectMap"), vec![json, entries, key, write]);
                out.push(StmtKind::Expr(call).at(crate::js::Span::NONE));
            }
            SerdeOp::Element => {
                out.push(step(&json, "element", Vec::new()));
                self.serde_write(values.remove(0), own(1), &json, span, out)?;
            }
            SerdeOp::Field => {
                out.push(step(&json, "key", vec![values.remove(0)]));
                self.serde_write(values.remove(0), own(1), &json, span, out)?;
            }
            SerdeOp::SkipField => {}
            SerdeOp::Entry => {
                let key = self.json_key(values.remove(0), own(1), span)?;
                out.push(step(&json, "key", vec![key]));
                self.serde_write(values.remove(0), own(2), &json, span, out)?;
            }
            SerdeOp::Key => {
                let key = self.json_key(values.remove(0), own(1), span)?;
                out.push(step(&json, "key", vec![key]));
            }
            SerdeOp::Value => self.serde_write(values.remove(0), own(1), &json, span, out)?,
            SerdeOp::End(compound) => {
                let method = match compound {
                    SerdeCompound::Seq
                    | SerdeCompound::Tuple
                    | SerdeCompound::TupleStruct
                    | SerdeCompound::TupleVariant => "endArray",
                    SerdeCompound::Map | SerdeCompound::Struct | SerdeCompound::StructVariant => "endObject",
                };
                out.push(step(&json, method, Vec::new()));
                // A variant's object of its name, closed too.
                if matches!(compound, SerdeCompound::TupleVariant | SerdeCompound::StructVariant) {
                    out.push(step(&json, "endObject", Vec::new()));
                }
            }
            // `v.serialize(s)`: the receiver is `v`, and the writer after it.
            SerdeOp::Serialize => {
                let writer = values.remove(0);
                self.serde_write(json, args.type_at(0), &writer, span, out)?;
            }
            SerdeOp::Custom | SerdeOp::Deserialize | SerdeOp::Visit(_) | SerdeOp::DeError(_) | SerdeOp::Next(_) => {
                unreachable!("made above")
            }
        }
        Ok(done)
    }

    /// `value`, of type `ty`, written by `json`: the writer's steps, as a
    /// derived `serialize` writes a field's, of a name, or its function
    /// called.
    fn serde_write(
        &mut self,
        value: Expr,
        ty: ty::Ty<'tcx>,
        json: &Expr,
        span: Span,
        out: &mut Vec<crate::js::Stmt>,
    ) -> R<()> {
        if let crate::js::ExprKind::Var(name) = &json.kind {
            let name = name.clone();
            return self.write_json(value, &name, ty.peel_refs(), span, out);
        }
        let writer = self.json_writer(ty, span)?;
        out.push(StmtKind::Expr(Expr::call(writer, vec![value, json.clone()])).at(crate::js::Span::NONE));
        Ok(())
    }

    /// `d.deserialize_any(visitor)`, or another hint: `$jsonVisit(d,
    /// expected, { u64, str, .. })`, the visitor's `visit_*` of the kinds
    /// the hint takes, each its function of the visitor, and what its
    /// `expecting` says, which an invalid type's error ends with.
    fn serde_visit(
        &mut self,
        hint: SerdeHint,
        json: Expr,
        (visitor, visitor_ty): (Expr, ty::Ty<'tcx>),
        span: Span,
        out: &mut Vec<crate::js::Stmt>,
    ) -> R<Expr> {
        let Some(imp) = self.recognition().serde_visitor_impl(visitor_ty) else {
            return Err(self.unsupported(span, &format!("a `{visitor_ty}` visitor, not the crate's own")));
        };
        let visitor = match visitor.reads_same() {
            true => visitor,
            false => self.spill("visitor", visitor, out),
        };
        let mut expected = None;
        let mut props = Vec::new();
        for (method, def_id) in visitor_methods(self.tcx, imp) {
            let key = match (method, hint) {
                (VisitorMethod::Expecting, _) => {
                    expected = Some(Expr::call(self.fn_ref(def_id), vec![visitor.clone()]));
                    continue;
                }
                (VisitorMethod::Unit, SerdeHint::Any | SerdeHint::Unit) => "unit",
                (VisitorMethod::Bool, SerdeHint::Any | SerdeHint::Bool) => "bool",
                (VisitorMethod::U64, SerdeHint::Any | SerdeHint::Number) => "u64",
                (VisitorMethod::I64, SerdeHint::Any | SerdeHint::Number) => "i64",
                (VisitorMethod::F64, SerdeHint::Any | SerdeHint::Number) => "f64",
                (VisitorMethod::Str, SerdeHint::Any | SerdeHint::Str) => "str",
                (VisitorMethod::BorrowedStr, SerdeHint::Any | SerdeHint::Str) => "borrowedStr",
                (VisitorMethod::Seq, SerdeHint::Any | SerdeHint::Seq | SerdeHint::Struct) => "seq",
                (VisitorMethod::Map, SerdeHint::Any | SerdeHint::Map | SerdeHint::Struct) => "map",
                (VisitorMethod::None, SerdeHint::Option) => "none",
                (VisitorMethod::Some, SerdeHint::Option) => "some",
                _ => continue,
            };
            // Each of the visitor and what it's given: nothing of a `null`.
            let given: Vec<String> = match method {
                VisitorMethod::Unit | VisitorMethod::None => Vec::new(),
                _ => vec![self.fresh("v")],
            };
            let args = std::iter::once(visitor.clone())
                .chain(given.iter().map(|v| Expr::var(v)))
                .collect();
            let call = Expr::call(self.fn_ref(def_id), args);
            let params = given.into_iter().map(Into::into).collect();
            let f = Expr::arrow(params, vec![StmtKind::Return(Some(call)).at(crate::js::Span::NONE)]);
            props.push(crate::js::Prop::Field(key.into(), f));
        }
        let expected = expected.expect("a `Visitor` writes its `expecting`");
        // A hint but `deserialize_any`'s errors of a kind it has no method for
        // where it peeks.
        let helper = match hint {
            SerdeHint::Any => "$jsonVisit",
            // `null`, or what's there read by its `visit_some`.
            SerdeHint::Option => "$jsonVisitOption",
            _ => "$jsonVisitHint",
        };
        Ok(Expr::call(
            Expr::helper(helper),
            vec![json, expected, Expr::object(props)],
        ))
    }

    /// serde's `error`, of its arguments' `values`: its message, as serde
    /// writes it, a `$jsonError`. What's expected, a `&dyn Expected`, is its
    /// text (`expected_text`).
    fn serde_error(&mut self, error: SerdeError, mut values: Vec<Expr>) -> Expr {
        let texts = |parts: &[&str]| parts.iter().map(|part| part.to_string()).collect::<Vec<_>>();
        let message = match error {
            SerdeError::InvalidType | SerdeError::InvalidValue => {
                let what = if error == SerdeError::InvalidType {
                    "invalid type: "
                } else {
                    "invalid value: "
                };
                let unexpected = Expr::call(Expr::helper("$jsonUnexpected"), vec![values.remove(0)]);
                Expr::template(texts(&[what, ", expected ", ""]), vec![unexpected, values.remove(0)])
            }
            SerdeError::InvalidLength => {
                let len = values.remove(0);
                Expr::template(
                    texts(&["invalid length ", ", expected ", ""]),
                    vec![len, values.remove(0)],
                )
            }
            SerdeError::UnknownVariant | SerdeError::UnknownField => {
                let what = if error == SerdeError::UnknownVariant {
                    "variant"
                } else {
                    "field"
                };
                let (name, names) = (values.remove(0), values.remove(0));
                return Expr::call(Expr::helper("$jsonUnknown"), vec![Expr::str(what), name, names]);
            }
            SerdeError::MissingField => Expr::template(texts(&["missing field `", "`"]), vec![values.remove(0)]),
            SerdeError::DuplicateField => Expr::template(texts(&["duplicate field `", "`"]), vec![values.remove(0)]),
        };
        Expr::call(Expr::helper("$jsonError"), vec![message])
    }

    /// A `&dyn Expected` made of a `value` of type `ty`: its text, a string's
    /// itself, a hand-written visitor's what its `expecting` says.
    pub(in crate::lower) fn expected_text(&mut self, value: Expr, ty: ty::Ty<'tcx>, span: Span) -> R<Expr> {
        let ty = ty.peel_refs();
        if self.is_string_like(ty) {
            return Ok(value);
        }
        let expecting = (self.recognition().serde_visitor_impl(ty)).and_then(|imp| {
            visitor_methods(self.tcx, imp)
                .into_iter()
                .find(|&(m, _)| m == VisitorMethod::Expecting)
        });
        match expecting {
            Some((_, f)) => Ok(Expr::call(self.fn_ref(f), vec![value])),
            None => Err(self.unsupported(span, &format!("a `{ty}` as what a deserializer expected"))),
        }
    }

    /// What `access` reads next, of a `next_element::<T>()`'s `T` or a map's
    /// key and value types: `$jsonNext(access, readT, boxed)` and the like, a
    /// `Result` of an `Option`, `Some` boxed where `T` looks like `None`.
    fn serde_next(
        &mut self,
        next: SerdeNext,
        access: Expr,
        (first, second): (Option<ty::Ty<'tcx>>, Option<ty::Ty<'tcx>>),
        span: Span,
    ) -> R<Expr> {
        let (first, second) = (first.expect("a type to read"), second);
        let boxed = |cx: &Self, t: ty::Ty<'tcx>| Expr::bool(cx.boxed_payload(t));
        Ok(match next {
            SerdeNext::Element => {
                let read = self.json_reader(first, span)?;
                Expr::call(Expr::helper("$jsonNextElement"), vec![access, read, boxed(self, first)])
            }
            SerdeNext::Key => {
                let read = self.json_key_reader(first, span)?;
                Expr::call(Expr::helper("$jsonNextKey"), vec![access, read, boxed(self, first)])
            }
            SerdeNext::Value => {
                let read = self.json_reader(first, span)?;
                Expr::call(Expr::helper("$jsonNextValue"), vec![access, read])
            }
            SerdeNext::Entry => {
                let value = second.expect("`next_entry::<K, V>`");
                let (key, read) = (self.json_key_reader(first, span)?, self.json_reader(value, span)?);
                Expr::call(Expr::helper("$jsonNextEntry"), vec![access, key, read])
            }
        })
    }
}
