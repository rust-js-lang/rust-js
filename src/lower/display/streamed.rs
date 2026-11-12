//! A `Debug` builder of a writer that writes through stdout's line (#1490),
//! written as it's taken: std's builder's model, the runtime's
//! `$debugList(f)` and the rest, each piece written at its step, as core's
//! `fmt/builders.rs` writes it, so what the writer prints between them
//! lands where Rust's stdout puts it (#1533). Whether a writer's builders
//! are streamed is the writer's sink's answer (`writes_through_a_sink`),
//! read where a builder's made, never asked again here.

use super::*;
use crate::lower::recognition::BuilderStep;
use crate::lower::temporary_names::Role;

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// The model of a `Debug` builder of `kind` written through `sink`,
    /// `$debugStruct(f, "P", options?.alternate)`, which writes its opening,
    /// and the role of the variable that holds it.
    pub(in crate::lower) fn streamed_builder(
        &self,
        kind: DebugBuilder,
        sink: Expr,
        type_name: Option<Expr>,
    ) -> (Expr, Role) {
        let (helper, role) = match kind {
            DebugBuilder::Struct => ("$debugStruct", Role::Struct),
            DebugBuilder::Tuple => ("$debugTuple", Role::Tuple),
            DebugBuilder::List => ("$debugList", Role::List),
            DebugBuilder::Set => ("$debugSet", Role::Set),
            DebugBuilder::Map => ("$debugMap", Role::Map),
        };
        let mut args = vec![sink];
        args.extend(type_name);
        let pretty = self.writer_pretty();
        if !matches!(pretty, Pretty::Plain) {
            args.push(pretty.alternate());
        }
        (Expr::call_helper(helper, args), role)
    }

    /// A step of a streamed `Debug` builder of `kind`, held in `builder`:
    /// `field`, `entry` or `entries`, given what each shows as a function,
    /// made only where the step is taken, as std's `field_with` makes it, or
    /// `finish`.
    #[allow(clippy::type_complexity)]
    pub(in crate::lower) fn streamed_step(
        &mut self,
        kind: DebugBuilder,
        builder: &str,
        (step, values, generic_args): (BuilderStep, Vec<(Expr, Ty<'tcx>)>, ty::GenericArgsRef<'tcx>),
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> R<Expr> {
        let pretty = self.writer_pretty();
        let mut values = values.into_iter();
        let shown = |this: &mut Self, (value, ty): (Expr, Ty<'tcx>)| -> R<Expr> {
            Ok(made_when_taken(this.debug_string_with(value, ty, span, &pretty)?))
        };
        let (method, args) = match (kind, step) {
            (_, BuilderStep::Finish { non_exhaustive }) => {
                let method = if non_exhaustive {
                    "finishNonExhaustive"
                } else {
                    "finish"
                };
                (method, Vec::new())
            }
            (DebugBuilder::Struct, BuilderStep::Field) => {
                let (field, value) = (values.next().expect("a name"), values.next().expect("a value"));
                ("field", vec![field.0, shown(self, value)?])
            }
            (DebugBuilder::Tuple, BuilderStep::Field) => ("field", vec![shown(self, values.next().expect("a value"))?]),
            (DebugBuilder::List | DebugBuilder::Set, BuilderStep::Entry) => {
                ("entry", vec![shown(self, values.next().expect("a value"))?])
            }
            (DebugBuilder::Map, BuilderStep::Entry) => {
                let (key, value) = (values.next().expect("a key"), values.next().expect("a value"));
                ("entry", vec![shown(self, key)?, shown(self, value)?])
            }
            (DebugBuilder::List | DebugBuilder::Set | DebugBuilder::Map, BuilderStep::Entries) => {
                let items = values.next().expect("the entries").0;
                let entries = self.entries_shown(kind, items, generic_args, span, out)?;
                let mut args = vec![entries.items];
                args.extend(entries.shows.into_iter().map(|shown| {
                    Expr::arrow(
                        vec!["entry".into()],
                        vec![StmtKind::Return(Some(shown)).at(js::Span::NONE)],
                    )
                }));
                args.extend(entries.drop);
                ("entries", args)
            }
            _ => return Err(self.unsupported(span, "this step of a `Debug` builder")),
        };
        Ok(Expr::call(Expr::member(Expr::var(builder), method), args))
    }

    /// A derived `Debug`'s body (ADR 0060), `operation` of its arguments
    /// but the `Formatter`, `values`, of a writer that writes through the
    /// line, its sink `sink`: std's builder's model, each field written as
    /// it's shown, as std's `debug_struct_field2_finish` writes it, so what
    /// a field wrote before a panic stopped it stays in the line (#1533,
    /// #1530). `None` of another write.
    pub(in crate::lower) fn streamed_derived(
        &mut self,
        operation: WriteCall,
        sink: &str,
        values: &[Expr],
        js_span: js::Span,
    ) -> Option<Vec<Stmt>> {
        let array = |e: &Expr| match &e.kind {
            js::ExprKind::Array(items) => Some(items.clone()),
            _ => None,
        };
        let (kind, type_name, fields): (_, _, Vec<(Option<Expr>, Expr)>) = match (operation, values) {
            (WriteCall::Struct, [type_name, rest @ ..]) => (
                DebugBuilder::Struct,
                type_name,
                rest.chunks(2)
                    .map(|pair| (Some(pair[0].clone()), pair[1].clone()))
                    .collect(),
            ),
            (WriteCall::Tuple, [type_name, rest @ ..]) => (
                DebugBuilder::Tuple,
                type_name,
                rest.iter().map(|v| (None, v.clone())).collect(),
            ),
            (WriteCall::StructFields, [type_name, names, shown, ..]) => (
                DebugBuilder::Struct,
                type_name,
                (array(names)?.into_iter().map(Some)).zip(array(shown)?).collect(),
            ),
            (WriteCall::TupleFields, [type_name, shown]) => (
                DebugBuilder::Tuple,
                type_name,
                array(shown)?.into_iter().map(|v| (None, v)).collect(),
            ),
            _ => return None,
        };
        let (made, role) = self.streamed_builder(kind, Expr::var(sink), Some(type_name.clone()));
        let builder = self.fresh(role);
        let mut written = vec![StmtKind::Const(builder.clone(), made).at(js_span)];
        for (name, value) in fields {
            let mut args: Vec<Expr> = name.into_iter().collect();
            args.push(made_when_taken(value));
            let step = Expr::call(Expr::member(Expr::var(&builder), "field"), args);
            written.push(StmtKind::Expr(step).at(js_span));
        }
        let finish = Expr::call(Expr::member(Expr::var(&builder), "finish"), Vec::new());
        written.push(StmtKind::Expr(finish).at(js_span));
        Some(written)
    }
}

/// `shown`, made where a step's taken, `() => shown`, so a step after one
/// that failed isn't made, as std's builders skip it.
fn made_when_taken(shown: Expr) -> Expr {
    Expr::arrow(Vec::new(), vec![StmtKind::Return(Some(shown)).at(js::Span::NONE)])
}
