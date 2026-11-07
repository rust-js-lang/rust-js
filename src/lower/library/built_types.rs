//! A type built of a function's slots, as a library's manifest says it
//! (#737): what its function asks the layout of, which no bound of it names,
//! so a consumer reads it here and makes it of the types its call gives.

use crate::library::TypeTemplate;
use crate::lower::traits::{BuiltSlots, drop_items, drop_slot};
use rustc_middle::ty::{self, Ty, TyCtxt};
use rustc_span::def_id::DefId;

/// `ty`, of `id`'s slots, as a template of them: each of `id`'s type
/// parameters and their associated types (`drop_slot`) by its index, each
/// struct or enum by rustc's `DefPathHash` of it. `None` of what a template
/// can't say, a const argument, a `dyn`, a function: refused.
pub(super) fn template<'tcx>(
    tcx: TyCtxt<'tcx>,
    foreign: &super::Foreign<'_, 'tcx>,
    id: DefId,
    ty: Ty<'tcx>,
) -> Option<TypeTemplate> {
    let items = drop_items(tcx, foreign, id);
    let slot = |ty: Ty<'tcx>| match *ty.kind() {
        ty::Param(param) => Some(param.index),
        ty::Alias(..) => (items.iter())
            .position(|&item| crate::lower::traits::alias_of(item) == crate::lower::traits::alias_of(ty))
            .map(|at| crate::lower::traits::item_slot(tcx, id, at)),
        _ => None,
    };
    of(tcx, &slot, ty)
}

fn of<'tcx>(tcx: TyCtxt<'tcx>, slot: &dyn Fn(Ty<'tcx>) -> Option<u32>, ty: Ty<'tcx>) -> Option<TypeTemplate> {
    if let Some(index) = slot(ty) {
        return Some(TypeTemplate::Slot(index));
    }
    Some(match *ty.kind() {
        ty::Adt(adt, args) => {
            let mut types = Vec::new();
            for arg in args.iter() {
                match arg.kind() {
                    ty::GenericArgKind::Lifetime(_) => {}
                    ty::GenericArgKind::Type(ty) => types.push(of(tcx, slot, ty)?),
                    ty::GenericArgKind::Const(_) => return None,
                }
            }
            TypeTemplate::Adt {
                key: def_key(tcx, adt.did()),
                args: types,
            }
        }
        ty::Tuple(parts) => TypeTemplate::Tuple(parts.iter().map(|part| of(tcx, slot, part)).collect::<Option<_>>()?),
        ty::Ref(_, to, mutability) => TypeTemplate::Ref {
            mutable: mutability.is_mut(),
            to: Box::new(of(tcx, slot, to)?),
        },
        ty::Array(item, len) => TypeTemplate::Array {
            len: len.try_to_target_usize(tcx)?,
            of: Box::new(of(tcx, slot, item)?),
        },
        ty::Bool | ty::Char | ty::Int(_) | ty::Uint(_) | ty::Float(_) => TypeTemplate::Primitive(ty.to_string()),
        _ => return None,
    })
}

/// A template's type, of `id`'s slots in its own terms; `None` of a key no
/// crate this one loads has.
pub(super) fn made<'tcx>(
    tcx: TyCtxt<'tcx>,
    foreign: &super::Foreign<'_, 'tcx>,
    id: DefId,
    template: &TypeTemplate,
) -> Option<Ty<'tcx>> {
    Some(match template {
        // A bound's, never another built one.
        &TypeTemplate::Slot(index) => {
            let bounded = crate::lower::traits::item_slot(tcx, id, drop_items(tcx, foreign, id).len());
            if index >= bounded {
                return None;
            }
            drop_slot(tcx, (foreign, &BuiltSlots::default()), id, index)
        }
        TypeTemplate::Adt { key, args } => {
            let adt = def_of_key(tcx, key)?;
            let mut types = args.iter();
            let mut made_args = Vec::new();
            for param in &tcx.generics_of(adt).own_params {
                made_args.push(match param.kind {
                    ty::GenericParamDefKind::Lifetime => tcx.lifetimes.re_erased.into(),
                    ty::GenericParamDefKind::Type { .. } => made(tcx, foreign, id, types.next()?)?.into(),
                    ty::GenericParamDefKind::Const { .. } => return None,
                });
            }
            if types.next().is_some() {
                return None;
            }
            Ty::new_adt(tcx, tcx.adt_def(adt), tcx.mk_args(&made_args))
        }
        TypeTemplate::Tuple(parts) => Ty::new_tup_from_iter(
            tcx,
            parts
                .iter()
                .map(|part| made(tcx, foreign, id, part))
                .collect::<Option<Vec<_>>>()?
                .into_iter(),
        ),
        TypeTemplate::Ref { mutable, to } => {
            let mutability = match mutable {
                true => ty::Mutability::Mut,
                false => ty::Mutability::Not,
            };
            Ty::new_ref(tcx, tcx.lifetimes.re_erased, made(tcx, foreign, id, to)?, mutability)
        }
        TypeTemplate::Array { len, of } => Ty::new_array(tcx, made(tcx, foreign, id, of)?, *len),
        TypeTemplate::Primitive(name) => primitive(tcx, name)?,
    })
}

/// A primitive type by its name, as `template` writes it.
fn primitive<'tcx>(tcx: TyCtxt<'tcx>, name: &str) -> Option<Ty<'tcx>> {
    let types = &tcx.types;
    Some(match name {
        "bool" => types.bool,
        "char" => types.char,
        "i8" => types.i8,
        "i16" => types.i16,
        "i32" => types.i32,
        "i64" => types.i64,
        "i128" => types.i128,
        "isize" => types.isize,
        "u8" => types.u8,
        "u16" => types.u16,
        "u32" => types.u32,
        "u64" => types.u64,
        "u128" => types.u128,
        "usize" => types.usize,
        "f32" => types.f32,
        "f64" => types.f64,
        _ => return None,
    })
}

/// rustc's `DefPathHash` of `id`, its bytes in hex, which `def_of_key`
/// reads back.
fn def_key(tcx: TyCtxt<'_>, id: DefId) -> String {
    (tcx.def_path_hash(id).0.to_le_bytes().iter())
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn def_of_key(tcx: TyCtxt<'_>, key: &str) -> Option<DefId> {
    let mut bytes = [0u8; 16];
    if key.len() != 32 {
        return None;
    }
    for (at, byte) in bytes.iter_mut().enumerate() {
        *byte = u8::from_str_radix(key.get(at * 2..at * 2 + 2)?, 16).ok()?;
    }
    let hash = rustc_span::def_id::DefPathHash(rustc_data_structures::fingerprint::Fingerprint::from_le_bytes(bytes));
    tcx.def_path_hash_to_def_id(hash)
}
