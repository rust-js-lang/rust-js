//! serde's own impls of std's types (#1623): the one catalog of their JSON
//! forms, a row each, as serde's `ser/impls.rs` and `de/impls.rs` write and
//! read them. Writing and reading each ask `std_form`, extended row by row.
//!
//! - A struct form is serde's `serialize_struct(name, ..)` of its fields,
//!   `{"start":1,"end":5}`, read by serde's own visitor of a map or a
//!   sequence, `struct Range`, which knows its fields' names and refuses any
//!   other, `unknown field `x`, expected `start` or `end``: the runtime's
//!   `struct` reader, which denies unknown fields.
//! - An enum form is externally tagged, `"Unbounded"` or `{"Included":3}`,
//!   each variant written and read as a derive of the enum's own
//!   definition does, the variants named in serde's order, which its
//!   messages list, `expected one of `Unbounded`, `Included`, `Excluded``,
//!   not the enum's: for every message serde_json can reach, serde's impl
//!   is that (a variant is read from a string, never by its index).

use super::*;
use crate::lower::std_types::range::RangeKind;

/// One row's form.
pub(super) enum StdForm<'tcx> {
    /// `serialize_struct(name, ..)` of these fields, by their names and
    /// types, each held by its JS value under its own name, as a range's
    /// `{ start, end }` holds them.
    Struct {
        name: &'static str,
        fields: Vec<(&'static str, Ty<'tcx>)>,
    },
    /// What a derive of the enum's own definition writes and reads, its
    /// variants named in this order, serde's.
    Enum(ty::AdtDef<'tcx>, ty::GenericArgsRef<'tcx>, &'static [&'static str]),
}

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// The catalog: `ty`'s serde form, of std's types whose impl is serde's
    /// own; `None` of any other.
    pub(super) fn std_form(&self, ty: Ty<'tcx>) -> Option<StdForm<'tcx>> {
        let ty::Adt(adt, args) = ty.kind() else {
            return None;
        };
        // A range's `start` and `end`, which `RangeInclusive`'s `start()`
        // and `end()` are too: its fields (ADR 0129).
        let range = |name| {
            let idx = args.type_at(0);
            Some(StdForm::Struct {
                name,
                fields: vec![("start", idx), ("end", idx)],
            })
        };
        match self.range_kind(ty) {
            Some(RangeKind::Exclusive) => return range("Range"),
            Some(RangeKind::Inclusive) => return range("RangeInclusive"),
            _ => {}
        }
        if self.is_std_type(ty, StdItem::Bound) {
            return Some(StdForm::Enum(*adt, args, &["Unbounded", "Included", "Excluded"]));
        }
        None
    }

    /// `value`, of `ty`, written as its row's form says.
    pub(super) fn write_std_form(
        &mut self,
        form: StdForm<'tcx>,
        value: Expr,
        json: &str,
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> R<()> {
        match form {
            StdForm::Struct { fields, .. } => {
                self.emit(json, "beginObject", Vec::new(), out);
                for (name, field_ty) in fields {
                    self.emit(json, "key", vec![Expr::str(name)], out);
                    self.write_json(Expr::member(value.clone(), name), json, field_ty, span, out)?;
                }
                self.emit(json, "endObject", Vec::new(), out);
                Ok(())
            }
            StdForm::Enum(adt, args, _) => self.write_adt(value, json, adt, args, span, out),
        }
    }

    /// The reader of `ty`, of its row's form: `(json) => json.struct("struct
    /// Range", [["start", read], ["end", read]], build, { deny: true })`.
    pub(super) fn read_std_form(&mut self, form: StdForm<'tcx>, span: Span) -> R<Expr> {
        let json = self.fresh(Role::Json);
        let read = match form {
            StdForm::Struct { name, fields } => {
                let mut entries = Vec::new();
                let mut names = Vec::new();
                for &(field, field_ty) in &fields {
                    let read = self.json_reader(field_ty, span)?;
                    entries.push(Expr::array(vec![Expr::str(field), read]));
                    names.push(field);
                }
                // Made of the values in the fields' order, as the struct
                // holds them: `([start, end]) => ({ start, end })`.
                let locals: Vec<String> = names.iter().map(|name| self.fresh(Base::Named(name))).collect();
                let made = Expr::object(
                    (names.iter().zip(&locals))
                        .map(|(name, local)| js::Prop::Field((*name).into(), Expr::var(local)))
                        .collect(),
                );
                let build = Expr::arrow(
                    vec![js::Pattern::names_array(locals.into_iter().map(Some))],
                    vec![StmtKind::Return(Some(made)).at(js::Span::NONE)],
                );
                // Its fields' names known, and any other refused; and
                // serde's visitor's own words, `struct Range`, of a sequence
                // too short.
                let expected = Expr::str(format!("struct {name}"));
                let options = Expr::object(vec![
                    js::Prop::Field("deny".into(), Expr::bool(true)),
                    js::Prop::Field("expecting".into(), expected.clone()),
                ]);
                Expr::call(
                    Expr::member(Expr::var(&json), "struct"),
                    vec![expected, Expr::array(entries), build, options],
                )
            }
            StdForm::Enum(adt, args, order) => {
                let variants = (order.iter())
                    .map(|name| {
                        let variant = (adt.variants().iter())
                            .find(|v| v.name.as_str() == *name)
                            .expect("a row names its enum's variants");
                        (variant, Attrs::default())
                    })
                    .collect::<Vec<_>>();
                let type_name = self.tcx.item_name(adt.did()).to_string();
                let variants: Vec<(&ty::VariantDef, Attrs)> = variants;
                self.read_tagged(
                    &json,
                    &Tagging::External,
                    &variants,
                    adt,
                    args,
                    &Attrs::default(),
                    &type_name,
                    span,
                )?
            }
        };
        Ok(Expr::arrow(
            vec![json.into()],
            vec![StmtKind::Return(Some(read)).at(js::Span::NONE)],
        ))
    }
}
