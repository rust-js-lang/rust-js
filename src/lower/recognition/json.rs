//! rust-js's JSON types and what a call of one is.

use super::*;

/// serde_json's own types, by name.
#[derive(Clone, Copy, PartialEq)]
pub(in crate::lower) enum Json {
    Value,
    Number,
    Map,
}

impl<'a, 'tcx> Recognition<'a, 'tcx> {
    pub(in crate::lower) fn json_type(&self, ty: Ty<'tcx>) -> Option<Json> {
        let ty::Adt(adt, _) = ty.peel_refs().kind() else {
            return None;
        };
        if self.tcx.crate_name(adt.did().krate).as_str() != "serde_json" {
            return None;
        }
        Some(match self.tcx.item_name(adt.did()).as_str() {
            "Value" => Json::Value,
            "Number" => Json::Number,
            "Map" => Json::Map,
            _ => return None,
        })
    }

    /// serde_json's `Value`, whose `Null` is the string `"Null"` and each
    /// other variant an object (ADR 0083).
    pub(in crate::lower) fn is_json_value(&self, ty: Ty<'tcx>) -> bool {
        self.json_type(ty) == Some(Json::Value)
    }

    pub(in crate::lower) fn is_json_map(&self, ty: Ty<'tcx>) -> bool {
        self.json_type(ty) == Some(Json::Map)
    }
}

/// Recognized serde_json calls carry type facts, never lowered operands.
pub(in crate::lower) enum JsonCall<'tcx> {
    ToValue(Ty<'tcx>),
    FromValue(Ty<'tcx>),
    Index,
    Equal {
        other: Ty<'tcx>,
        value_first: bool,
        negate: bool,
    },
    Convert {
        to: Ty<'tcx>,
        from: Ty<'tcx>,
        json: Json,
    },
    Default,
    Method(JsonMethod),
    /// serde's data model, of a hand-written impl (ADR 0459), its call's
    /// arguments' types.
    Serde(SerdeOp, ty::GenericArgsRef<'tcx>),
}

impl<'a, 'tcx> Recognition<'a, 'tcx> {
    pub(in crate::lower) fn json_call(
        &self,
        def_id: DefId,
        generic_args: ty::GenericArgsRef<'tcx>,
    ) -> Option<JsonCall<'tcx>> {
        let tcx = self.tcx;
        if let Some(op) = serde_op(tcx, def_id) {
            return Some(JsonCall::Serde(op, generic_args));
        }
        let name = tcx.item_name(def_id);
        if tcx.crate_name(def_id.krate).as_str() == "serde_json" && tcx.trait_of_assoc(def_id).is_none() {
            match name.as_str() {
                "to_value" => {
                    return Some(JsonCall::ToValue(
                        generic_args.types().next().expect("`to_value::<T>`").peel_refs(),
                    ));
                }
                // `serde_json::from_value::<T>(v)`: `T` read from the `Value`.
                "from_value" => {
                    return Some(JsonCall::FromValue(
                        generic_args.types().next().expect("`from_value::<T>`"),
                    ));
                }
                _ => {}
            }
        }
        if let Some(trait_id) = tcx.trait_of_assoc(def_id) {
            let tr = ty::TraitRef::from_assoc(tcx, trait_id, generic_args);
            let (this, other) = (tr.self_ty(), tr.args.types().nth(1));
            let this_value = self.json_type(this) == Some(Json::Value);
            let other_value = other.is_some_and(|o| self.json_type(o) == Some(Json::Value));
            if tcx.is_lang_item(trait_id, LangItem::Index) && this_value {
                return Some(JsonCall::Index);
            }
            if tcx.is_lang_item(trait_id, LangItem::PartialEq)
                && let Some(other) = other
                && this_value != other_value
            {
                return Some(JsonCall::Equal {
                    other: if this_value { other } else { this },
                    value_first: this_value,
                    negate: name.as_str() == "ne",
                });
            }
            let converting = match other {
                Some(other) if tcx.is_diagnostic_item(sym::From, trait_id) => Some((this, other)),
                Some(other) if tcx.is_diagnostic_item(sym::Into, trait_id) => Some((other, this)),
                _ => None,
            };
            if let Some((to, from)) = converting
                && let Some(json) = self.json_type(to)
            {
                return Some(JsonCall::Convert { to, from, json });
            }
            if tcx.is_diagnostic_item(Symbol::intern("Default"), trait_id) && this_value {
                return Some(JsonCall::Default);
            }
            return None;
        }
        let imp = tcx.inherent_impl_of_assoc(def_id)?;
        let owner = tcx.type_of(imp).instantiate_identity().skip_normalization();
        match self.json_type(owner)? {
            json @ (Json::Value | Json::Number) => Some(JsonCall::Method(JsonMethod::recognize(json, name))),
            Json::Map => None,
        }
    }
}

/// Inherent Value/Number operations, selected before operand emission.
pub(in crate::lower) enum JsonMethod {
    NumberFromF64,
    NumberAsF64,
    /// `as_u64()` and `as_i64()`: `"u64"` or `"i64"`, a BigInt (ADR 0086).
    NumberAsInt(&'static str),
    NumberKind(&'static str),
    NumberIsI64,
    IsNull,
    IsTag(&'static str),
    IsNumber(&'static str),
    AsTag(&'static str),
    /// `as_array_mut()` and `as_object_mut()` of a `&mut Value`, a cell of it.
    AsTagMut(&'static str),
    AsF64,
    AsInt(&'static str),
    Get,
    /// `get_mut(key)`: a handle on the entry, `$jsonGetMut`.
    GetMut,
    Unsupported {
        owner: &'static str,
        name: Symbol,
    },
}

impl JsonMethod {
    fn recognize(json: Json, name: Symbol) -> Self {
        if json == Json::Number {
            return match name.as_str() {
                "from_f64" => Self::NumberFromF64,
                "as_f64" => Self::NumberAsF64,
                "as_u64" => Self::NumberAsInt("u64"),
                "as_i64" => Self::NumberAsInt("i64"),
                "is_f64" => Self::NumberKind("f"),
                "is_u64" => Self::NumberKind("u"),
                "is_i64" => Self::NumberIsI64,
                _ => Self::Unsupported { owner: "Number", name },
            };
        }
        match name.as_str() {
            "is_null" => Self::IsNull,
            "is_boolean" => Self::IsTag("Bool"),
            "is_number" => Self::IsTag("Number"),
            "is_string" => Self::IsTag("String"),
            "is_array" => Self::IsTag("Array"),
            "is_object" => Self::IsTag("Object"),
            "is_f64" => Self::IsNumber("f64"),
            "is_u64" => Self::IsNumber("u64"),
            "is_i64" => Self::IsNumber("i64"),
            "as_bool" => Self::AsTag("Bool"),
            "as_str" => Self::AsTag("String"),
            "as_array" => Self::AsTag("Array"),
            "as_object" => Self::AsTag("Object"),
            "as_array_mut" => Self::AsTagMut("Array"),
            "as_object_mut" => Self::AsTagMut("Object"),
            "as_number" => Self::AsTag("Number"),
            "as_f64" => Self::AsF64,
            "as_u64" => Self::AsInt("u64"),
            "as_i64" => Self::AsInt("i64"),
            "get" => Self::Get,
            "get_mut" => Self::GetMut,
            _ => Self::Unsupported { owner: "Value", name },
        }
    }
}

/// The established Value representation for one source type. Container elements
/// are recognized recursively when lowering their conversion functions.
pub(in crate::lower) enum JsonConversion<'tcx> {
    Tag(&'static str),
    Null,
    Float,
    Integer,
    Same,
    Vector(Ty<'tcx>),
    Option(Ty<'tcx>),
}

impl<'a, 'tcx> Recognition<'a, 'tcx> {
    /// `T`, for an `Option<T>`.
    pub(in crate::lower) fn option_of(&self, ty: Ty<'tcx>) -> Option<Ty<'tcx>> {
        match ty.kind() {
            ty::Adt(adt, args) if self.tcx.is_lang_item(adt.did(), LangItem::Option) => args.types().next(),
            _ => None,
        }
    }

    pub(in crate::lower) fn json_conversion(&self, from: Ty<'tcx>) -> Option<JsonConversion<'tcx>> {
        let from = from.peel_refs();
        if self.is_string_like(from) && !from.is_char() {
            return Some(JsonConversion::Tag("String"));
        }
        if from.is_bool() {
            return Some(JsonConversion::Tag("Bool"));
        }
        if from.is_unit() {
            return Some(JsonConversion::Null);
        }
        match Num::of(from) {
            // An `f32`'s `Value` is its `f64`, as serde_json's `Number::from_f32` is
            // (ADR 0122).
            Some(Num::F64 | Num::F32) => return Some(JsonConversion::Float),
            Some(_) => return Some(JsonConversion::Integer),
            None => {}
        }
        match self.json_type(from) {
            Some(Json::Map) => return Some(JsonConversion::Tag("Object")),
            Some(Json::Number) => return Some(JsonConversion::Tag("Number")),
            Some(Json::Value) => return Some(JsonConversion::Same),
            None => {}
        }
        if let ty::Adt(_, args) = from.kind()
            && self.is_std_type(from, StdItem::Vec)
        {
            return Some(JsonConversion::Vector(args.type_at(0)));
        }
        self.option_of(from).map(JsonConversion::Option)
    }

    /// The scalar comparison category used by serde_json's PartialEq impls.
    pub(in crate::lower) fn json_comparison(&self, other: Ty<'tcx>) -> Option<&'static str> {
        let other = other.peel_refs();
        if self.is_string_like(other) {
            Some("String")
        } else if other.is_bool() {
            Some("Bool")
        } else {
            match Num::of(other)? {
                Num::F64 => Some("f64"),
                // serde_json's `as_f32()`, the number `as f32` (ADR 0122).
                Num::F32 => Some("f32"),
                n if n.signed() => Some("i64"),
                _ => Some("u64"),
            }
        }
    }
    pub(in crate::lower) fn is_json_error(&self, ty: Ty<'tcx>) -> bool {
        matches!(ty.kind(), ty::Adt(adt, _) if self.tcx.crate_name(adt.did().krate).as_str() == "serde_json"
            && self.tcx.item_name(adt.did()).as_str() == "Error")
    }
}
