//! serde's data model, of a hand-written `Serialize` (ADR 0459): its
//! serializer is the JSON writer, `s.serialize_str(x)` is `s.string(x)`, and
//! a struct, a sequence or a map it begins is the writer too, its fields and
//! elements written in turn. Each call gives `Ok`, as serde_json's writing
//! to a string does; an error thrown in it is the codec's, as a derived one's.

use crate::js::{Expr, StmtKind};
use crate::lower::recognition::{SerdeCompound, SerdeOp, SerdeScalar};
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
                let method = match compound {
                    SerdeCompound::Seq => "beginArray",
                    SerdeCompound::Tuple => "beginTuple",
                    SerdeCompound::TupleStruct => "beginTupleStruct",
                    SerdeCompound::Map | SerdeCompound::Struct => "beginObject",
                };
                out.push(step(&json, method, Vec::new()));
                return Ok(result::ok(json));
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
                    SerdeCompound::Seq | SerdeCompound::Tuple | SerdeCompound::TupleStruct => "endArray",
                    SerdeCompound::Map | SerdeCompound::Struct => "endObject",
                };
                out.push(step(&json, method, Vec::new()));
            }
            // `v.serialize(s)`: the receiver is `v`, and the writer after it.
            SerdeOp::Serialize => {
                let writer = values.remove(0);
                self.serde_write(json, args.type_at(0), &writer, span, out)?;
            }
            SerdeOp::Custom | SerdeOp::Deserialize => unreachable!("made above"),
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
}
