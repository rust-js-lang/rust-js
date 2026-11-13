//! serde's own impls of std's types (#1623): the one catalog of their JSON
//! forms, a row each, as serde's `ser/impls.rs` and `de/impls.rs` write and
//! read them. Writing and reading each ask `std_form`, extended row by row.
//!
//! - A struct form is serde's `serialize_struct(name, ..)` of its fields,
//!   `{"start":1,"end":5}`, read by serde's own visitor of a map or a
//!   sequence, `struct Range`, which knows its fields' names and refuses any
//!   other, `unknown field `x`, expected `start` or `end``: the runtime's
//!   `struct` reader, which denies unknown fields. Its value holds the
//!   fields by their names, a range's `{ start, end }`, or is a `Duration`'s
//!   nanoseconds, whose parts serde writes, `{"secs":1,"nanos":5}`, taken by
//!   `Duration`'s own lowering, and which a maker of serde's checks makes.
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
    /// types, held by its JS value as `held` says.
    Struct {
        name: &'static str,
        fields: Vec<(&'static str, Ty<'tcx>)>,
        held: Held,
    },
    /// What a derive of the enum's own definition writes and reads, its
    /// variants named in this order, serde's.
    Enum(ty::AdtDef<'tcx>, ty::GenericArgsRef<'tcx>, &'static [&'static str]),
}

/// How a struct form's JS value holds its fields.
pub(super) enum Held {
    /// Under their own names, as a range's `{ start, end }` holds them.
    Fields,
    /// A `Duration`'s nanoseconds, a BigInt (ADR 0188), whose whole seconds
    /// and nanoseconds under one are its two fields, as `as_secs()` and
    /// `subsec_nanos()` give them; of what `through`, a runtime helper,
    /// gives of the value first, a `SystemTime`'s time since the epoch, or
    /// its serde error. Made by `make` of the two, a runtime helper of
    /// serde's checks, `overflow deserializing Duration`, then
    /// `Duration::new`'s; and of what's read whole, by `then`, a check serde
    /// makes past its reader, whose error has no position, a `SystemTime`'s
    /// `UNIX_EPOCH.checked_add` (#1623).
    Duration {
        through: Option<&'static str>,
        make: &'static str,
        then: Option<&'static str>,
    },
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
                held: Held::Fields,
            })
        };
        // Time: a `Duration`'s `secs` and `nanos`, and a `SystemTime`'s since
        // the epoch, which serde writes only of one after it.
        let time = |name, fields: [&'static str; 2], held| {
            let fields = vec![(fields[0], self.tcx.types.u64), (fields[1], self.tcx.types.u32)];
            Some(StdForm::Struct { name, fields, held })
        };
        if crate::lower::recognition::is_duration_ty(ty) {
            let held = Held::Duration {
                through: None,
                make: "$durationOfJson",
                then: None,
            };
            return time("Duration", ["secs", "nanos"], held);
        }
        if crate::lower::recognition::clock_type_of(ty) == Some(crate::lower::recognition::ClockType::System) {
            let held = Held::Duration {
                through: Some("$systemTimeSinceEpochForJson"),
                make: "$systemTimeOffsetOfJson",
                then: Some("$systemTimeOfJson"),
            };
            return time("SystemTime", ["secs_since_epoch", "nanos_since_epoch"], held);
        }
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
            StdForm::Struct { fields, held, .. } => {
                let parts: Vec<Expr> = match held {
                    Held::Fields => (fields.iter())
                        .map(|(name, _)| Expr::member(value.clone(), *name))
                        .collect(),
                    Held::Duration { through, .. } => {
                        let duration = match through {
                            Some(helper) => Expr::call_helper(helper, vec![value]),
                            None => value,
                        };
                        // Read twice.
                        let duration = match duration.reads_same() {
                            true => duration,
                            false => self.spill(Role::Value, duration, out),
                        };
                        use crate::lower::std_types::number::{duration_as, duration_subsec};
                        vec![
                            duration_as(duration.clone(), 1_000_000_000),
                            duration_subsec(duration, 1),
                        ]
                    }
                };
                self.emit(json, "beginObject", Vec::new(), out);
                for ((name, field_ty), part) in fields.into_iter().zip(parts) {
                    self.emit(json, "key", vec![Expr::str(name)], out);
                    self.write_json(part, json, field_ty, span, out)?;
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
            StdForm::Struct { name, fields, held } => {
                let then = match held {
                    Held::Duration { then, .. } => then,
                    Held::Fields => None,
                };
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
                let made = match held {
                    Held::Fields => Expr::object(
                        (names.iter().zip(&locals))
                            .map(|(name, local)| js::Prop::Field((*name).into(), Expr::var(local)))
                            .collect(),
                    ),
                    Held::Duration { make, .. } => {
                        Expr::call_helper(make, locals.iter().map(|local| Expr::var(local)).collect())
                    }
                };
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
                let read = Expr::call(
                    Expr::member(Expr::var(&json), "struct"),
                    vec![expected, Expr::array(entries), build, options],
                );
                match then {
                    Some(then) => Expr::call_helper(then, vec![read]),
                    None => read,
                }
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
