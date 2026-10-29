//! serde's traits, derives and visitors, and the operation a call of one is.

use super::*;

impl<'a, 'tcx> Recognition<'a, 'tcx> {
    /// The crate's impl of serde's `Visitor` for `ty`, a hand-written
    /// `Deserialize`'s (ADR 0459).
    pub(in crate::lower) fn serde_visitor_impl(&self, ty: Ty<'tcx>) -> Option<DefId> {
        let visitor = (self.tcx.all_traits_including_private()).find(|&id| serde_visitor(self.tcx, id))?;
        // `Visitor<'de>` of `ty`: its `Self`, and its lifetime erased.
        let args = ty::GenericArgs::for_item(self.tcx, visitor, |param, _| match param.kind {
            ty::GenericParamDefKind::Lifetime => self.tcx.lifetimes.re_erased.into(),
            _ => ty.into(),
        });
        let tr = ty::TraitRef::new_from_args(self.tcx, visitor, args);
        match self.tcx.codegen_select_candidate(self.typing_env.as_query_input(tr)) {
            Ok(ImplSource::UserDefined(imp)) if self.trait_impls.contains(&imp.impl_def_id) => Some(imp.impl_def_id),
            _ => None,
        }
    }
}

/// A call of serde's data model a hand-written `Serialize` or `Deserialize`
/// makes, over the JSON writer and reader (ADR 0459).
#[derive(Clone, Copy, Debug, PartialEq)]
pub(in crate::lower) enum SerdeOp {
    /// `s.serialize_bool(b)` and the like: a value the writer writes itself.
    Write(SerdeScalar),
    /// `serialize_some(&v)` and `serialize_newtype_struct(name, &v)`: `v`, as
    /// it is, its argument at `at`.
    WriteValue {
        at: usize,
    },
    /// `serialize_unit_variant(name, index, variant)`: its name.
    UnitVariant,
    /// `serialize_newtype_variant(name, index, variant, &v)`: `{ variant: v }`.
    NewtypeVariant,
    /// `collect_str(&v)`: what `v` shows, as a string.
    CollectStr,
    /// `collect_seq(items)` and `collect_map(entries)`: each written in turn,
    /// in an array or an object.
    CollectSeq,
    CollectMap,
    /// `serialize_seq(len)` and the rest: the writer, begun.
    Begin(SerdeCompound),
    /// `serialize_element(&v)`, of a sequence or a tuple: its next.
    Element,
    /// `serialize_field(key, &v)` of a struct: its key, then it.
    Field,
    /// `skip_field(key)`: nothing.
    SkipField,
    /// `serialize_entry(&k, &v)`, `serialize_key(&k)` and `serialize_value(&v)`
    /// of a map.
    Entry,
    Key,
    Value,
    /// `end()`: the compound closed.
    End(SerdeCompound),
    /// `v.serialize(s)` of a `Serialize`: its writer's.
    Serialize,
    /// `T::deserialize(d)` of a `Deserialize`: its reader's, a `Result`.
    Deserialize,
    /// `d.deserialize_any(v)` and the other hints, of a hand-written
    /// visitor: what's there, given to the `visit_*` of its kind the hint
    /// takes.
    Visit(SerdeHint),
    /// `seq.next_element::<T>()`, and a map's `next_key`, `next_value` and
    /// `next_entry`: what's next, a `Result` of an `Option` but a value's.
    Next(SerdeNext),
    /// `S::Error::custom(msg)` and `D::Error::custom(msg)`: an error of what
    /// `msg` shows.
    Custom,
    /// `E::invalid_type(unexpected, &self)` and serde's other errors of a
    /// deserializer, its messages.
    DeError(SerdeError),
}

/// serde's `de::Error`s but `custom`, each its message.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(in crate::lower) enum SerdeError {
    InvalidType,
    InvalidValue,
    InvalidLength,
    UnknownVariant,
    UnknownField,
    MissingField,
    DuplicateField,
}

/// What a writer writes itself, of serde's scalars.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(in crate::lower) enum SerdeScalar {
    Bool,
    Int,
    Float {
        single: bool,
    },
    Char,
    Str,
    /// `()`, `None` and a unit struct: `null`.
    Null,
}

/// What a `deserialize_*` hint takes, of the kinds a JSON value is: any
/// of them, a `bool`, a number, a string, or `null`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(in crate::lower) enum SerdeHint {
    Any,
    Bool,
    Number,
    Str,
    Unit,
    /// A sequence, `deserialize_seq`'s and a tuple's.
    Seq,
    Map,
    /// `deserialize_struct`'s, a map or a sequence of its fields.
    Struct,
    /// `deserialize_option`'s: `null`, or anything else, given to read.
    Option,
}

/// What a `SeqAccess` or a `MapAccess` reads next.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(in crate::lower) enum SerdeNext {
    Element,
    Key,
    Value,
    Entry,
}

/// A hand-written visitor's method, of those serde_json calls: what it's
/// expecting, and a `visit_*` of each kind of JSON value.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(in crate::lower) enum VisitorMethod {
    Expecting,
    Unit,
    Bool,
    U64,
    I64,
    F64,
    Str,
    BorrowedStr,
    Seq,
    Map,
    /// `visit_none` and `visit_some`, of `deserialize_option`.
    None,
    Some,
}

/// The methods of serde's `Visitor` the impl `imp` writes, of those
/// serde_json calls, each with its function (ADR 0459).
pub(in crate::lower) fn visitor_methods(tcx: TyCtxt<'_>, imp: DefId) -> Vec<(VisitorMethod, DefId)> {
    (tcx.associated_items(imp).in_definition_order())
        .filter_map(|item| {
            let method = match item.name().as_str() {
                "expecting" => VisitorMethod::Expecting,
                "visit_unit" => VisitorMethod::Unit,
                "visit_bool" => VisitorMethod::Bool,
                "visit_u64" => VisitorMethod::U64,
                "visit_i64" => VisitorMethod::I64,
                "visit_f64" => VisitorMethod::F64,
                "visit_str" => VisitorMethod::Str,
                "visit_borrowed_str" => VisitorMethod::BorrowedStr,
                "visit_seq" => VisitorMethod::Seq,
                "visit_map" => VisitorMethod::Map,
                "visit_none" => VisitorMethod::None,
                "visit_some" => VisitorMethod::Some,
                _ => return None,
            };
            Some((method, item.def_id))
        })
        .collect()
}

/// What a `serialize_*` begins: a sequence, a tuple, a tuple struct, a map
/// or a struct.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(in crate::lower) enum SerdeCompound {
    Seq,
    Tuple,
    TupleStruct,
    Map,
    Struct,
    /// A variant's, in an object of its name.
    TupleVariant,
    StructVariant,
}

/// Is `ty` a `dyn` of serde's `de::Expected`, what a deserializer's error
/// says it expected, which rust-js holds as its text (ADR 0459)?
pub(in crate::lower) fn is_dyn_expected(tcx: TyCtxt<'_>, ty: Ty<'_>) -> bool {
    matches!(ty.peel_refs().kind(), ty::Dynamic(traits, ..)
        if traits.principal_def_id().is_some_and(|id| matches!(tcx.crate_name(id.krate).as_str(), "serde" | "serde_core")
            && tcx.item_name(id).as_str() == "Expected"))
}

/// Is `id` serde's `de::Visitor`?
pub(in crate::lower) fn serde_visitor(tcx: TyCtxt<'_>, id: DefId) -> bool {
    matches!(tcx.crate_name(id.krate).as_str(), "serde" | "serde_core")
        && tcx.def_kind(id) == DefKind::Trait
        && tcx.item_name(id).as_str() == "Visitor"
}

/// `def_id` as a call of serde's data model a hand-written impl makes (ADR
/// 0459), or `None`.
pub(in crate::lower) fn serde_op(tcx: TyCtxt<'_>, def_id: DefId) -> Option<SerdeOp> {
    let trait_id = tcx.trait_of_assoc(def_id)?;
    if !matches!(tcx.crate_name(trait_id.krate).as_str(), "serde" | "serde_core") {
        return None;
    }
    let method = tcx.item_name(def_id);
    let compound = |name: &str| match name {
        "SerializeSeq" => Some(SerdeCompound::Seq),
        "SerializeTuple" => Some(SerdeCompound::Tuple),
        "SerializeTupleStruct" => Some(SerdeCompound::TupleStruct),
        "SerializeMap" => Some(SerdeCompound::Map),
        "SerializeStruct" => Some(SerdeCompound::Struct),
        "SerializeTupleVariant" => Some(SerdeCompound::TupleVariant),
        "SerializeStructVariant" => Some(SerdeCompound::StructVariant),
        _ => None,
    };
    let trait_name = tcx.item_name(trait_id);
    Some(match (trait_name.as_str(), method.as_str()) {
        ("Serialize", "serialize") => SerdeOp::Serialize,
        ("Deserialize", "deserialize") => SerdeOp::Deserialize,
        ("Deserializer", name) => SerdeOp::Visit(match name {
            "deserialize_any" => SerdeHint::Any,
            "deserialize_bool" => SerdeHint::Bool,
            "deserialize_i8" | "deserialize_i16" | "deserialize_i32" | "deserialize_i64" | "deserialize_u8"
            | "deserialize_u16" | "deserialize_u32" | "deserialize_u64" | "deserialize_f32" | "deserialize_f64" => {
                SerdeHint::Number
            }
            "deserialize_str" | "deserialize_string" | "deserialize_identifier" => SerdeHint::Str,
            "deserialize_unit" => SerdeHint::Unit,
            "deserialize_seq" | "deserialize_tuple" | "deserialize_tuple_struct" => SerdeHint::Seq,
            "deserialize_map" => SerdeHint::Map,
            "deserialize_struct" => SerdeHint::Struct,
            "deserialize_option" => SerdeHint::Option,
            _ => return None,
        }),
        ("SeqAccess", "next_element") => SerdeOp::Next(SerdeNext::Element),
        ("MapAccess", "next_key") => SerdeOp::Next(SerdeNext::Key),
        ("MapAccess", "next_value") => SerdeOp::Next(SerdeNext::Value),
        ("MapAccess", "next_entry") => SerdeOp::Next(SerdeNext::Entry),
        ("Serializer", name) => match name {
            "serialize_bool" => SerdeOp::Write(SerdeScalar::Bool),
            "serialize_i8" | "serialize_i16" | "serialize_i32" | "serialize_i64" | "serialize_i128"
            | "serialize_u8" | "serialize_u16" | "serialize_u32" | "serialize_u64" | "serialize_u128" => {
                SerdeOp::Write(SerdeScalar::Int)
            }
            "serialize_f32" => SerdeOp::Write(SerdeScalar::Float { single: true }),
            "serialize_f64" => SerdeOp::Write(SerdeScalar::Float { single: false }),
            "serialize_char" => SerdeOp::Write(SerdeScalar::Char),
            "serialize_str" => SerdeOp::Write(SerdeScalar::Str),
            "serialize_unit" | "serialize_none" | "serialize_unit_struct" => SerdeOp::Write(SerdeScalar::Null),
            "serialize_some" => SerdeOp::WriteValue { at: 1 },
            "serialize_newtype_struct" => SerdeOp::WriteValue { at: 2 },
            "serialize_unit_variant" => SerdeOp::UnitVariant,
            "serialize_newtype_variant" => SerdeOp::NewtypeVariant,
            "collect_str" => SerdeOp::CollectStr,
            "collect_seq" => SerdeOp::CollectSeq,
            "collect_map" => SerdeOp::CollectMap,
            "serialize_tuple_variant" => SerdeOp::Begin(SerdeCompound::TupleVariant),
            "serialize_struct_variant" => SerdeOp::Begin(SerdeCompound::StructVariant),
            "serialize_seq" => SerdeOp::Begin(SerdeCompound::Seq),
            "serialize_tuple" => SerdeOp::Begin(SerdeCompound::Tuple),
            "serialize_tuple_struct" => SerdeOp::Begin(SerdeCompound::TupleStruct),
            "serialize_map" => SerdeOp::Begin(SerdeCompound::Map),
            "serialize_struct" => SerdeOp::Begin(SerdeCompound::Struct),
            _ => return None,
        },
        (name, "end") => SerdeOp::End(compound(name)?),
        (name, "serialize_element") if compound(name).is_some() => SerdeOp::Element,
        // A struct's field, its key and its value; a tuple's, its value.
        (name, "serialize_field") => match compound(name)? {
            SerdeCompound::Struct | SerdeCompound::StructVariant => SerdeOp::Field,
            _ => SerdeOp::Element,
        },
        (name, "skip_field") if compound(name).is_some() => SerdeOp::SkipField,
        ("SerializeMap", "serialize_entry") => SerdeOp::Entry,
        ("SerializeMap", "serialize_key") => SerdeOp::Key,
        ("SerializeMap", "serialize_value") => SerdeOp::Value,
        ("Error", "custom") if matches!(tcx.item_name(tcx.parent(trait_id)).as_str(), "ser" | "de") => SerdeOp::Custom,
        ("Error", name) if tcx.item_name(tcx.parent(trait_id)).as_str() == "de" => SerdeOp::DeError(match name {
            "invalid_type" => SerdeError::InvalidType,
            "invalid_value" => SerdeError::InvalidValue,
            "invalid_length" => SerdeError::InvalidLength,
            "unknown_variant" => SerdeError::UnknownVariant,
            "unknown_field" => SerdeError::UnknownField,
            "missing_field" => SerdeError::MissingField,
            "duplicate_field" => SerdeError::DuplicateField,
            _ => return None,
        }),
        _ => return None,
    })
}

pub(in crate::lower) fn serde_trait(tcx: TyCtxt<'_>, trait_id: DefId) -> Option<bool> {
    if !matches!(tcx.crate_name(trait_id.krate).as_str(), "serde" | "serde_core") {
        return None;
    }
    match tcx.item_name(trait_id).as_str() {
        "Serialize" => Some(true),
        "Deserialize" | "DeserializeOwned" => Some(false),
        _ => None,
    }
}

/// A derive whose meaning rust-js has itself, never its body's: std's own,
/// `#[derive(Clone)]` and the rest. Another crate's, strum's or thiserror's,
/// writes code like any of the crate's, `#[automatically_derived]` though it
/// is (ADR 0186); serde's is left out where it's read, as its codecs are
/// rust-js's (ADR 0077). rustc tells which macro wrote only this crate's,
/// the one compiled here: another's, core's `IntErrorKind`'s `Debug`, is as
/// it says.
pub(crate) fn known_derive(tcx: TyCtxt<'_>, id: DefId) -> bool {
    match id.is_local() {
        true => tcx.is_builtin_derived(id),
        false => tcx.is_automatically_derived(id),
    }
}

pub(in crate::lower) fn from_serde_derive(tcx: TyCtxt<'_>, def_id: LocalDefId) -> bool {
    let mut item = Some(def_id);
    while let Some(id) = item {
        let mut ctxt = tcx.def_span(id).ctxt();
        while !ctxt.is_root() {
            let expansion = ctxt.outer_expn_data();
            // By the macro, not its name: `serde::Deserialize` and an alias are
            // named as they're written.
            if let ExpnKind::Macro(MacroKind::Derive, _) = expansion.kind
                && expansion
                    .macro_def_id
                    .is_some_and(|id| tcx.crate_name(id.krate).as_str() == "serde_derive")
            {
                return true;
            }
            ctxt = expansion.call_site.ctxt();
        }
        item = tcx.opt_local_parent(id);
    }
    false
}

pub(in crate::lower) fn serde_impl(tcx: TyCtxt<'_>, id: DefId) -> Option<bool> {
    if !matches!(tcx.def_kind(id), DefKind::Impl { of_trait: true }) {
        return None;
    }
    let tr = tcx.impl_trait_ref(id).instantiate_identity().skip_normalization();
    let own = matches!(tr.self_ty().kind(), ty::Adt(adt, _)
        if adt.did().as_local().is_some_and(|local| !from_serde_derive(tcx, local)));
    serde_trait(tcx, tr.def_id).filter(|_| own)
}

/// Whether serde impl `id` is serde's derive's, a codec rust-js writes by
/// its `#[serde]` attributes (ADR 0077), not a hand-written one, whose body
/// is the crate's (ADR 0459). A library's, by its `#[automatically_derived]`.
pub(in crate::lower) fn serde_derived(tcx: TyCtxt<'_>, id: DefId) -> bool {
    match id.as_local() {
        Some(local) => from_serde_derive(tcx, local),
        None => tcx.is_automatically_derived(id),
    }
}
