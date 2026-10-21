//! What rust-js can represent: a value of a type it can't is an error where
//! it's made or bound, which says what part of it is the one.

use super::recognition::{StdItem, is_std_def};
use super::refusal::{Refused, refuse};
use super::representation::Num;
use super::{FnCx, R, Shape};
use rustc_ast::Mutability;
use rustc_hir as hir;
use rustc_hir::attrs::lang_items::LangItem;
use rustc_middle::ty;
use rustc_middle::ty::Ty;
use rustc_span::Span;

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    pub(super) fn check_value_ty(&self, ty: Ty<'tcx>, span: Span) -> R<()> {
        match self.unsupported_part(ty) {
            None => Ok(()),
            Some(part) => Err(refuse(self.tcx, span, Refused::Type("values", part))),
        }
    }

    /// The first type inside `ty` (or `ty` itself) that rust-js can't represent.
    pub(super) fn unsupported_part(&self, ty: Ty<'tcx>) -> Option<Ty<'tcx>> {
        self.unsupported_in(ty, &mut Vec::new())
    }

    /// `unsupported_part`, for a type inside the ones in `seen`. A type
    /// inside itself (`Tree` in `Node(Box<Tree>, ..)`) is being checked
    /// already, further out.
    pub(super) fn unsupported_in(&self, ty: Ty<'tcx>, seen: &mut Vec<Ty<'tcx>>) -> Option<Ty<'tcx>> {
        if ty.is_bool()
            || ty.is_unit()
            || ty.is_never()
            || ty.is_str()
            || ty.is_char()
            || Num::of(ty).is_some()
            || self.is_str_split(ty)
        {
            return None;
        }
        if self.is_array_iter(ty) || self.recognition().is_occupied_entry(ty) || self.recognition().is_vacant_entry(ty)
        {
            return None;
        }
        match ty.kind() {
            _ if self.is_unknown(ty) => return None,
            ty::Dynamic(predicates, ..)
                if predicates.principal_def_id().is_none_or(|id| {
                    // The crate's own or a library's (ADR 0100), whose
                    // `dyn` is its pair either way.
                    self.recognition().is_rust_trait(id) && self.dyn_supported(id)
                        || self.recognition().is_std_pair_trait(id)
                }) =>
            {
                return None;
            }
            // A channel's end, the queue it shares (ADR 0142), of its items.
            ty::Adt(_, args) if self.recognition().channel_end(ty).is_some() => {
                return self.unsupported_in(args.type_at(0), seen);
            }
            // A JS iterator, of its `Item`s.
            ty::Dynamic(traits, ..) if self.recognition().is_dyn_iter(ty) => {
                return traits
                    .projection_bounds()
                    .find_map(|item| item.skip_binder().term.as_type())
                    .and_then(|item| self.unsupported_in(item, seen));
            }
            // A JS value from an `extern` block, and closures: JS functions.
            ty::Foreign(_) | ty::Closure(..) | ty::CoroutineClosure(..) | ty::FnDef(..) | ty::FnPtr(..) => return None,
            // Futures are JS promises (ADR 0029): an `async` block, what an
            // `async fn` returns, and `dyn Future`.
            ty::Coroutine(..) => return None,
            ty::Alias(
                _,
                ty::AliasTy {
                    kind: ty::Opaque { def_id },
                    ..
                },
            ) if matches!(self.tcx.opaque_ty_origin(*def_id), hir::OpaqueTyOrigin::AsyncFn { .. }) => {
                return None;
            }
            // `impl Iterator<Item = u32>` is the type it hides (ADR 0061).
            ty::Alias(
                _,
                ty::AliasTy {
                    kind: ty::Opaque { .. },
                    ..
                },
            ) if self.reveal(ty) != ty => return self.unsupported_in(self.reveal(ty), seen),
            ty::Dynamic(..) if self.recognition().is_dyn_future(ty) => return None,
            // `&dyn Any` is any JS value, as the webapi crate's `object`
            // parameters take: a struct, say, which is a JS object already.
            ty::Dynamic(traits, ..)
                if traits
                    .principal_def_id()
                    .is_some_and(|t| is_std_def(self.tcx, t, StdItem::Any)) =>
            {
                return None;
            }
            // `dyn ReactNode` is the node itself, any of them (ADR 0242).
            ty::Dynamic(traits, ..)
                if traits
                    .principal_def_id()
                    .is_some_and(|t| super::bindings::is_jsx_node(self.tcx, t)) =>
            {
                return None;
            }
            ty::Adt(..) if self.is_js_object(ty) => return None,
            // A standard stream, and what writing to one gives: nothing JS
            // needs, `undefined` (ADR 0132).
            ty::Adt(..)
                if self.recognition().stream(ty).is_some()
                    || self.recognition().is_io_unit_result(ty)
                    || self.recognition().is_io_error(ty) =>
            {
                return None;
            }
            // `{ message, line, column }` (ADR 0077), and `{ kind, value }` (ADR 0083).
            ty::Adt(..) if self.is_json_error(ty) || self.is_json_number(ty) => return None,
            // `{ file, line, column }` (ADR 0427).
            ty::Adt(..) if self.recognition().is_location(ty) => return None,
            // `undefined`, or JS's stack (ADR 0568).
            ty::Adt(..) if self.recognition().is_backtrace(ty) => return None,
            // `dyn Debug` is the string it shows (ADR 0060), and serde's `dyn
            // Expected` its text (ADR 0459).
            ty::Dynamic(..) if self.is_dyn_debug(ty) || super::recognition::is_dyn_expected(self.tcx, ty) => {
                return None;
            }
            // A `HashMap` or `HashSet` (ADR 0059): keys JS compares by value.
            ty::Adt(_, args) if self.is_map(ty) => {
                let key = args.type_at(0);
                // A type parameter's, a hash map's, its class its caller's
                // (ADR 0466).
                let given = super::recognition::known_by_bounds(key) && !self.is_sorted(ty);
                if !given && !self.is_key(key, self.is_sorted(ty)) {
                    return Some(key);
                }
                // A map's value; after it, and after a set's key, the hasher.
                let set = self.is_set(ty);
                return args
                    .types()
                    .skip(1)
                    .take(usize::from(!set))
                    .find_map(|t| self.unsupported_in(t, seen));
            }
            ty::Dynamic(traits, ..)
                if traits
                    .principal_def_id()
                    .is_some_and(|t| self.tcx.fn_trait_kind_from_def_id(t).is_some()) =>
            {
                return None;
            }
            ty::Ref(_, inner, Mutability::Not) => return self.unsupported_in(*inner, seen),
            // A raw pointer, a handle on its place, of what it points to
            // (ADR 0407), a `NonNull` too, and one to a slice, the slice;
            // one to a `dyn` that's a pair, the pair (ADR 0562); one to
            // text, which holds a length too, isn't yet.
            _ if let Some((inner, _)) = super::pointers::raw_pointee(self.tcx, ty)
                && !matches!(self.tcx.struct_tail_for_codegen(inner, self.typing_env).kind(), ty::Str) =>
            {
                return self.unsupported_in(inner, seen);
            }
            // `&mut` to a JS object is the object; to anything else, it would
            // need a place to point at.
            // And to a generic iterator, the JS iterator it steps (ADR 0071).
            ty::Ref(_, inner, Mutability::Mut)
                if self.is_object(*inner) || self.is_callable(*inner) || self.is_generic_iter(*inner) =>
            {
                return self.unsupported_in(*inner, seen);
            }
            // A `&mut` to a `Pin` of a reference is the pin, which only what
            // it points at is changed through (ADR 0329).
            ty::Ref(_, inner, Mutability::Mut)
                if self
                    .recognition()
                    .pinned(*inner)
                    .is_some_and(|pointer| pointer.is_ref()) =>
            {
                return self.unsupported_in(*inner, seen);
            }
            // A `&mut` to anything else is a cell (ADR 0099).
            ty::Ref(_, inner, Mutability::Mut) if self.is_cell_pointee(*inner) => return None,
            ty::Array(elem, _) | ty::Slice(elem) => return self.unsupported_in(*elem, seen),
            ty::Adt(_, _) if self.is_lang_adt(ty, LangItem::String) => return None,
            // A `TypeId` is its type's key (ADR 0331).
            ty::Adt(..) if self.recognition().is_type_id(ty) => return None,
            // A `Discriminant` is its number (ADR 0393).
            ty::Adt(..) if self.recognition().is_discriminant(ty) => return None,
            // An `Option` is its value or `undefined` (ADR 0030), or a box where
            // the value looks like `None` (ADR 0051).
            ty::Adt(..) if let Some(inner) = self.option_of(ty) => return self.unsupported_in(inner, seen),
            // `format_args!`'s pieces are strings by the time JS sees them.
            ty::Adt(_, _)
                if self.is_lang_adt(ty, LangItem::FormatArguments)
                    || self.is_lang_adt(ty, LangItem::FormatArgument) =>
            {
                return None;
            }
            // A `ManuallyDrop`, or what `pin!` pins, what it holds.
            ty::Adt(..) if let Some(held) = self.transparent(ty) => return self.unsupported_in(held, seen),
            ty::Adt(_, args) if self.is_std_wrapper(ty) => {
                return args.types().next().and_then(|t| self.unsupported_in(t, seen));
            }
            // A thread-local is its value (ADR 0037).
            ty::Adt(_, args) if self.is_std_type(ty, StdItem::LocalKey) => {
                return args.types().next().and_then(|t| self.unsupported_in(t, seen));
            }
            _ => {}
        }
        if let Some(&found) = self.walks().representable.borrow().get(&ty) {
            return found;
        }
        // Inside itself, `Tree` in `Box<Tree>`: fine, if it is where it's
        // being walked further out, so what's found under that is only as
        // sure as the walk out there is.
        if let Some(at) = seen.iter().position(|&t| t == ty) {
            self.walks().assumed.set(self.walks().assumed.get().min(at));
            return None;
        }
        let depth = seen.len();
        let outer = self.walks().assumed.replace(usize::MAX);
        seen.push(ty);
        let found = match (ty.kind(), self.shape(ty)) {
            // An enum with fields (ADR 0033), and a union, the field it holds
            // (ADR 0406): every field.
            (ty::Adt(adt, args), _) if adt.is_enum() || adt.is_union() => {
                let fields: Vec<Ty<'tcx>> = adt.all_fields().map(|f| self.field_ty(f, args)).collect();
                fields.into_iter().find_map(|t| self.unsupported_in(t, seen))
            }
            (_, Shape::Object(fields)) => fields.iter().find_map(|&(_, t)| self.unsupported_in(t, seen)),
            (_, Shape::Array(tys)) => tys.iter().find_map(|&t| self.unsupported_in(t, seen)),
            (ty::Adt(adt, _), Shape::Other) if adt.is_struct() => None, // a unit struct
            _ => Some(ty),
        };
        seen.pop();
        let assumed = self.walks().assumed.get();
        self.walks().assumed.set(outer.min(assumed));
        // What it can't be is sure; that it's fine is, unless the walk under
        // it took a type further out as fine.
        if found.is_some() || assumed >= depth {
            self.walks().representable.borrow_mut().insert(ty, found);
        }
        found
    }
}
