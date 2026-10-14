//! serde_json's `Value` (ADR 0083): an enum as rust-js makes one, `"Null"`
//! or `{ TAG: "String", _0: s }`, so a `match` on one is a `match` on any
//! enum. Its `Map<String, Value>` is a JS `Map`, sorted as the `BTreeMap`
//! it wraps is, and its `Number` is `{ kind, value }`, a `"u"`nsigned,
//! `"i"`nteger or `"f"`loat, as serde_json keeps one.

use crate::js::{self, Expr, Op, Prop, StmtKind};
use crate::lower::display::Pretty;
use crate::lower::recognition::{Json, JsonCall, JsonConversion, JsonMethod};
use crate::lower::representation::Num;
use crate::lower::{FnCx, R};

use crate::lower::std_types::result;
use rustc_middle::ty::Ty;
use rustc_span::Span;

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    pub(in crate::lower) fn is_json_number(&self, ty: Ty<'tcx>) -> bool {
        self.json_type(ty) == Some(Json::Number)
    }

    /// `operation`, a serde_json call, of its arguments' `values` (ADR 0083).
    pub(in crate::lower) fn json_call_values(
        &mut self,
        operation: JsonCall<'tcx>,
        mut values: Vec<Expr>,
        span: Span,
    ) -> R<Expr> {
        match operation {
            JsonCall::ToValue(ty) => {
                let value = values.remove(0);
                // Of a string, a number, a `bool` or `()`, it can't fail: `Ok` of it.
                if self.is_string_like(ty) || ty.is_bool() || ty.is_unit() || Num::of(ty).is_some() {
                    let value = if ty.is_char() {
                        Expr::object(vec![
                            Prop::Field("TAG".into(), Expr::str("String")),
                            Prop::Field("_0".into(), value),
                        ])
                    } else {
                        self.json_value_from(value, ty, span)?
                    };
                    return Ok(result::ok(value));
                }
                let writer = self.json_writer(ty, span)?;
                Ok(Expr::call(Expr::helper("$toJsonValue"), vec![value, writer]))
            }
            JsonCall::FromValue(ty) => {
                let value = values.remove(0);
                let reader = self.json_reader(ty, span)?;
                Ok(Expr::call(Expr::helper("$fromJsonValue"), vec![value, reader]))
            }
            JsonCall::Index => Ok(Expr::call(Expr::helper("$jsonIndex"), values)),
            JsonCall::Equal {
                other,
                value_first,
                negate,
            } => {
                let (b, a) = (values.pop().expect("two sides"), values.pop().expect("two sides"));
                let eq = if value_first {
                    self.json_value_eq(a, b, other, span)?
                } else {
                    self.json_value_eq(b, a, other, span)?
                };
                Ok(if negate { Expr::unary(js::UnaryOp::Not, eq) } else { eq })
            }
            JsonCall::Convert { to, from, json } => {
                let x = values.remove(0);
                Ok(match json {
                    Json::Value => self.json_value_from(x, from, span)?,
                    Json::Number if Num::of(from.peel_refs()).is_some_and(|n| !n.float()) => {
                        Expr::call(Expr::helper("$jsonInt"), vec![x])
                    }
                    _ => return Err(self.unsupported(span, &format!("`{to}::from` of `{from}`"))),
                })
            }
            JsonCall::Default => Ok(Expr::str("Null")),
            JsonCall::Method(method) => self.json_value_method(method, values, span),
        }
    }

    /// `$jsonValueWrite` and `$jsonNumberWrite`: what writes a `Value` or a
    /// `Number`, `(value, json) => ..`.
    pub(in crate::lower) fn json_value_writer(&self, ty: Ty<'tcx>) -> Option<Expr> {
        let name = match self.json_type(ty)? {
            Json::Value => "$jsonValueWrite",
            Json::Number => "$jsonNumberWrite",
            Json::Map => return None,
        };
        Some(Expr::helper(name))
    }

    /// `clone()` of a `Value` or a `Map`: a copy of its arrays and maps.
    pub(in crate::lower) fn json_value_clone(&self, value: Expr, ty: Ty<'tcx>) -> Option<Expr> {
        let name = match self.json_type(ty)? {
            Json::Value => "$jsonValueClone",
            Json::Map => "$jsonMapClone",
            Json::Number => return None,
        };
        Some(Expr::call(Expr::helper(name), vec![value]))
    }

    /// `{}` of a `Value` (its compact JSON, or with `{:#}`, its pretty) or a
    /// `Number`.
    pub(in crate::lower) fn json_value_display(&self, value: Expr, ty: Ty<'tcx>, pretty: bool) -> Option<Expr> {
        let json = self.json_type(ty)?;
        Some(match json {
            Json::Value => Expr::call(Expr::helper("$jsonValueText"), vec![value, Expr::bool(pretty)]),
            Json::Number => Expr::call(Expr::helper("$jsonNumberText"), vec![value]),
            Json::Map => return None,
        })
    }

    /// `{:?}` of a `Value` or a `Number`, as serde_json writes one:
    /// `Object {"a": Number(1)}`.
    pub(in crate::lower) fn json_value_debug(&self, value: Expr, ty: Ty<'tcx>, pretty: &Pretty) -> Option<Expr> {
        let json = self.json_type(ty)?;
        Some(match json {
            // Pretty as `{:#?}` asks (ADR 0137).
            Json::Value if matches!(pretty, Pretty::Plain) => Expr::call(Expr::helper("$debugJsonValue"), vec![value]),
            Json::Value => Expr::call(Expr::helper("$debugJsonValue"), vec![value, pretty.alternate()]),
            Json::Number => Expr::call(Expr::helper("$debugJsonNumber"), vec![value]),
            Json::Map => return None,
        })
    }

    /// A method of `Value` or `Number`, as serde_json's does it.
    fn json_value_method(&self, method: JsonMethod, mut values: Vec<Expr>, span: Span) -> R<Expr> {
        // `Number::from_f64(x)`: `None` unless it's finite.
        match method {
            JsonMethod::NumberFromF64 => return Ok(Expr::call(Expr::helper("$jsonNumberOfF64"), values)),
            JsonMethod::Unsupported { owner, name } => {
                return Err(self.unsupported(span, &format!("`{owner}::{name}`")));
            }
            _ => {}
        }
        let receiver = values.remove(0);
        let call = |helper: &'static str, mut args: Vec<Expr>| {
            args.insert(0, receiver.clone());
            Expr::call(Expr::helper(helper), args)
        };
        Ok(match method {
            JsonMethod::NumberAsF64 => call("$jsonNumberF64", Vec::new()),
            JsonMethod::NumberAsInt(kind) => call("$jsonNumberInt", vec![Expr::str(kind)]),
            JsonMethod::NumberKind(kind) => Expr::bin(Op::Eq, Expr::member(receiver.clone(), "kind"), Expr::str(kind)),
            JsonMethod::NumberIsI64 => call("$jsonNumberIsI64", Vec::new()),
            JsonMethod::IsNull => Expr::bin(Op::Eq, receiver.clone(), Expr::str("Null")),
            JsonMethod::IsTag(tag) => Expr::bin(Op::Eq, Expr::member(receiver.clone(), "TAG"), Expr::str(tag)),
            JsonMethod::IsNumber(kind) => call("$jsonValueIs", vec![Expr::str(kind)]),
            JsonMethod::AsTag(tag) => call("$jsonValueAs", vec![Expr::str(tag)]),
            JsonMethod::AsF64 => call("$jsonValueF64", Vec::new()),
            JsonMethod::AsInt(kind) => call("$jsonValueInt", vec![Expr::str(kind)]),
            // `get(k)` of an object, or `get(i)` of an array: `None` if it
            // isn't there.
            JsonMethod::Get => {
                let key = values.remove(0);
                call("$jsonGet", vec![key])
            }
            JsonMethod::NumberFromF64 | JsonMethod::Unsupported { .. } => {
                unreachable!("handled before receiver extraction")
            }
        })
    }

    /// `Value::from(x)` and `x.into()`: `x`, as serde_json's `From` impls
    /// make a `Value` of it.
    pub(in crate::lower) fn json_value_from(&mut self, value: Expr, from: Ty<'tcx>, span: Span) -> R<Expr> {
        let from = from.peel_refs();
        let variant = |tag: &str, value: Expr| {
            Expr::object(vec![
                Prop::Field("TAG".into(), Expr::str(tag)),
                Prop::Field("_0".into(), value),
            ])
        };
        // A `Vec` of what makes one, or an `Option` of one: `Null` for `None`.
        let convert = |this: &mut Self, item: Ty<'tcx>| -> R<Expr> {
            let x = this.fresh("x");
            let converted = this.json_value_from(Expr::var(&x), item, span)?;
            Ok(Expr::arrow(
                vec![x.into()],
                vec![StmtKind::Return(Some(converted)).at(js::Span::NONE)],
            ))
        };
        Ok(match self.recognition().json_conversion(from) {
            Some(JsonConversion::Tag(tag)) => variant(tag, value),
            Some(JsonConversion::Null) => Expr::str("Null"),
            Some(JsonConversion::Float) => Expr::call(Expr::helper("$jsonFloat"), vec![value]),
            Some(JsonConversion::Integer) => variant("Number", Expr::call(Expr::helper("$jsonInt"), vec![value])),
            Some(JsonConversion::Same) => value,
            Some(JsonConversion::Vector(item)) => {
                let arrow = convert(self, item)?;
                variant("Array", Expr::call(Expr::member(value, "map"), vec![arrow]))
            }
            Some(JsonConversion::Option(item)) => {
                let arrow = convert(self, item)?;
                Expr::call(Expr::helper("$jsonOption"), vec![value, arrow])
            }
            None => return Err(self.unsupported(span, &format!("`Value::from` of `{from}`"))),
        })
    }

    /// `value == x` of a `Value` and a string, a number or a `bool`, as
    /// serde_json's `PartialEq` impls compare them.
    pub(in crate::lower) fn json_value_eq(&self, value: Expr, other: Expr, other_ty: Ty<'tcx>, span: Span) -> R<Expr> {
        let other_ty = other_ty.peel_refs();
        let kind = self
            .recognition()
            .json_comparison(other_ty)
            .ok_or_else(|| self.unsupported(span, &format!("`==` of a `Value` and a `{other_ty}`")))?;
        Ok(Expr::call(
            Expr::helper("$jsonValueEq"),
            vec![value, other, Expr::str(kind)],
        ))
    }
}
