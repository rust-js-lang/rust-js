//! std's own items: where std defines each, which rust-js compiles, and
//! the calls rust-js knows by where they're defined.

use super::*;

impl<'a, 'tcx> Recognition<'a, 'tcx> {
    /// `a += b` of a number whose `+` is its type's own, a `Duration`'s or
    /// a clock's, not its number's: `a = a + b`, of that `+`. The operator's
    /// method, its arguments, and what it is (ADRs 0188, 0447).
    pub(in crate::lower) fn assigned_by(
        &self,
        assign_trait: DefId,
        ty: Ty<'tcx>,
        rhs: Ty<'tcx>,
    ) -> Option<(DefId, ty::GenericArgsRef<'tcx>, Std)> {
        let tcx = self.tcx;
        let op = assign_operator(tcx, assign_trait)?;
        let (item, name) = [
            (BinOp::Add, LangItem::Add, "add"),
            (BinOp::Sub, LangItem::Sub, "sub"),
            (BinOp::Mul, LangItem::Mul, "mul"),
            (BinOp::Div, LangItem::Div, "div"),
            (BinOp::Rem, LangItem::Rem, "rem"),
        ]
        .into_iter()
        .find_map(|(o, item, name)| (o == op).then_some((item, name)))?;
        let method = trait_method(tcx, tcx.lang_items().get(item)?, name);
        let args = tcx.mk_args(&[ty.into(), rhs.into()]);
        Some((method, args, self.classify(method, args)?))
    }

    /// The call a std function is, which rust-js lowers as that call:
    /// `fmt::write(w, args)` is `w.write_fmt(args)`, of the `dyn fmt::Write`
    /// it takes, anyhow's (ADR 0446). Another is itself.
    pub(in crate::lower) fn call_it_is(
        &self,
        def_id: DefId,
        generic_args: ty::GenericArgsRef<'tcx>,
    ) -> (DefId, ty::GenericArgsRef<'tcx>) {
        let tcx = self.tcx;
        if of_std_crate(tcx, def_id, StdCrate::Core)
            && std_path(tcx, def_id).ends_with("fmt::write")
            && let sig = tcx
                .instantiate_bound_regions_with_erased(tcx.fn_sig(def_id).instantiate_identity().skip_normalization())
            && let Some(&writer) = sig.inputs().first()
        {
            let write_fmt = trait_method(tcx, std_item(tcx, StdItem::FmtWrite), "write_fmt");
            return (write_fmt, tcx.mk_args(&[writer.peel_refs().into()]));
        }
        (def_id, generic_args)
    }

    /// Of a call of a std trait's method `def_id` of `&mut self` whose
    /// `Self`, in `generic_args`, is a `&mut W`, which resolves to a method
    /// std's impl writes: `W`'s own method's arguments. std's impls of its
    /// traits for a `&mut W` pass each method of `&mut self` they write on
    /// to `(**self)`'s, its other arguments as they are, `impl<W: Write +
    /// ?Sized> Write for &mut W`'s, `Read`'s, `BufRead`'s, `Seek`'s,
    /// `fmt::Write`'s, `Hasher`'s and `Iterator`'s `next` and `nth`
    /// (#1309), so the call is `W`'s, given the `&mut W` its receiver, a
    /// `&mut &mut W`, is of: a `&mut &mut String`'s `write_str` writes the
    /// `String`. Not a method the impl leaves its trait's default, which
    /// runs of the `&mut W` itself, `by_ref()`'s; nor one of `self`, an
    /// iterator's `fold`, which std's impl makes of `I`'s `try_fold`; nor
    /// one of `&self`, `len()`'s or `cmp`'s, whose receiver, a `&&mut I`,
    /// the lowering reads as it is. (`impl Write for &mut [u8]`, of a type
    /// of std's own, writes the slice, which has no `Write` to call, so its
    /// call stays refused.)
    pub(in crate::lower) fn passed_through_mut_ref(
        &self,
        def_id: DefId,
        generic_args: ty::GenericArgsRef<'tcx>,
    ) -> Option<ty::GenericArgsRef<'tcx>> {
        let tcx = self.tcx;
        if !self.is_std(tcx.trait_of_assoc(def_id)?) {
            return None;
        }
        let &ty::Ref(_, inner, Mutability::Mut) = generic_args.types().next()?.kind() else {
            return None;
        };
        let sig = tcx
            .fn_sig(def_id)
            .instantiate_identity()
            .skip_normalization()
            .skip_binder();
        if !matches!(*sig.inputs().first()?.kind(), ty::Ref(_, of, Mutability::Mut) if of == tcx.types.self_param) {
            return None;
        }
        self.std_impl_writes(def_id, generic_args)
            .then(|| tcx.mk_args_from_iter(std::iter::once(inner.into()).chain(generic_args.iter().skip(1))))
    }

    /// Of a call of a std iterator trait's method `def_id` whose `Self`, in
    /// `generic_args`, is a `Box<I>` of a sized `I`, which resolves to a
    /// method std's impl writes: `I`'s own method's arguments. std's `impl
    /// Iterator for Box<I>`, and its `DoubleEndedIterator` and
    /// `ExactSizeIterator`, pass each method they write on to `I`'s,
    /// `(**self).next()`, so the call is `I`'s, of the box's value, which a
    /// `Box<I>` is in JS (#1278), as `passed_through_mut_ref` says of a
    /// `&mut W`'s (#1309). Not a method the impl leaves its trait's default,
    /// `by_ref()`'s or `take(n)`'s, which hold the box itself.
    pub(in crate::lower) fn passed_through_box(
        &self,
        def_id: DefId,
        generic_args: ty::GenericArgsRef<'tcx>,
    ) -> Option<ty::GenericArgsRef<'tcx>> {
        let tcx = self.tcx;
        let trait_id = tcx.trait_of_assoc(def_id)?;
        let iterator_trait = is_std_def(tcx, trait_id, StdItem::Iterator)
            || [self.double_ended_iterator(), self.exact_size_iterator()].contains(&Some(trait_id));
        // Of `self` or `&self`, the box's value, `I`'s, a default's too,
        // `take(n)`'s or `sum()`'s; of `&mut self`, only what std's impl
        // writes, not `by_ref()`, which gives the `&mut` to the box. And of
        // `self` of a `&mut Box<I>`, `(&mut boxed).take(1)`'s, a `&mut I`'s.
        let sig = tcx
            .fn_sig(def_id)
            .instantiate_identity()
            .skip_normalization()
            .skip_binder();
        let lends = matches!(*sig.inputs().first()?.kind(), ty::Ref(_, _, Mutability::Mut));
        let by_value = !sig.inputs().first()?.is_ref();
        let (inner, unboxed) = match *generic_args.types().next()?.kind() {
            ty::Ref(region, boxed, Mutability::Mut) if by_value => {
                let inner = boxed.boxed_ty()?;
                (inner, Ty::new_mut_ref(tcx, region, inner))
            }
            _ => {
                let inner = generic_args.types().next()?.boxed_ty()?;
                (inner, inner)
            }
        };
        (iterator_trait
            && inner.is_sized(tcx, self.typing_env)
            && (!lends || self.std_impl_writes(def_id, generic_args)))
        .then(|| tcx.mk_args_from_iter(std::iter::once(unboxed.into()).chain(generic_args.iter().skip(1))))
    }

    /// Whether a call of `def_id` with `generic_args` resolves to a method a
    /// std impl writes, not its trait's default (`passed_through_mut_ref`,
    /// `passed_through_box`).
    fn std_impl_writes(&self, def_id: DefId, generic_args: ty::GenericArgsRef<'tcx>) -> bool {
        let method = crate::lower::items::resolve_instance_in(self.tcx, self.typing_env, def_id, generic_args);
        method.ok().flatten().is_some_and(|instance| {
            self.tcx.impl_of_assoc(instance.def_id()).is_some() && self.is_std(instance.def_id())
        })
    }

    /// A type made of std's types alone, and no cell: no impl of the crate's
    /// can be for it, nor one that borrows (ADR 0328).
    pub(in crate::lower) fn std_alone(&self, ty: Ty<'tcx>) -> bool {
        ty.walk().all(|part| {
            part.as_type().is_none_or(|t| match t.kind() {
                ty::Adt(adt, _) => self.is_std(adt.did()) && !self.is_borrowed_cell(t),
                ty::Bool
                | ty::Char
                | ty::Int(_)
                | ty::Uint(_)
                | ty::Float(_)
                | ty::Str
                | ty::Never
                | ty::Array(..)
                | ty::Slice(_)
                | ty::Ref(..)
                | ty::Tuple(_) => true,
                _ => crate::lower::pointers::raw_pointee(self.tcx, t).is_some(),
            })
        })
    }

    pub(in crate::lower) fn is_std(&self, id: DefId) -> bool {
        is_std_crate(self.tcx, id)
    }

    /// Is `id` the standard library's: std, core, alloc, or a crate rustc's
    /// sysroot holds beside them, as `hashbrown`, which std's maps are made of?
    pub(in crate::lower) fn in_sysroot(&self, id: DefId) -> bool {
        self.is_std(id) || from_sysroot(self.tcx, id)
    }
}

/// Is `id` of a crate rustc loaded from its sysroot, not one a program
/// built?
fn from_sysroot(tcx: TyCtxt<'_>, id: DefId) -> bool {
    !id.is_local() && {
        let sysroots: Vec<&std::path::Path> = tcx.sess.opts.sysroot.all_paths().collect();
        tcx.used_crate_source(id.krate)
            .paths()
            .any(|path| sysroots.iter().any(|sysroot| path.starts_with(sysroot)))
    }
}

/// Is `id` libtest's, the `test` crate of rustc's sysroot, not a program's
/// crate of that name (ADR 1054)?
pub(super) fn is_libtest(tcx: TyCtxt<'_>, id: DefId) -> bool {
    tcx.crate_name(id.krate).as_str() == "test" && from_sysroot(tcx, id)
}

/// A std item's path as std names it, `std::str::Chars`: a `#![no_std]`
/// crate's rustc names it `core::str::Chars`, or `alloc::..`, which every
/// path compared here would miss.
/// One of std's functions that works in Node only (ADR 0391): of the
/// process a program runs in, a helper of the runtime's `process.js`, or of
/// its files, of `fs.js` (#1156).
#[derive(Clone, Copy, PartialEq, Debug)]
pub(crate) enum NodeOnly {
    Process(ProcessOp),
    Fs(FsOp),
}

/// std's functions of the process a program runs in and of its files,
/// which Node has and a browser hasn't (ADR 0391), each by its path: the one
/// list of what works in Node only.
const NODE_ONLY: &[(&str, NodeOnly)] = &[
    ("std::env::args", NodeOnly::Process(ProcessOp::Args)),
    ("std::env::var", NodeOnly::Process(ProcessOp::Var)),
    ("std::env::var_os", NodeOnly::Process(ProcessOp::VarOs)),
    ("std::env::vars", NodeOnly::Process(ProcessOp::Vars)),
    ("std::env::vars_os", NodeOnly::Process(ProcessOp::VarsOs)),
    ("std::process::exit", NodeOnly::Process(ProcessOp::Exit)),
    ("std::fs::File::open", NodeOnly::Fs(FsOp::Open)),
    ("std::fs::File::create", NodeOnly::Fs(FsOp::Create)),
    ("std::fs::OpenOptions::open", NodeOnly::Fs(FsOp::OptionsOpen)),
    ("std::fs::read", NodeOnly::Fs(FsOp::Read)),
    ("std::fs::read_to_string", NodeOnly::Fs(FsOp::ReadToString)),
    ("std::fs::write", NodeOnly::Fs(FsOp::Write)),
    ("std::fs::remove_file", NodeOnly::Fs(FsOp::RemoveFile)),
    ("std::fs::metadata", NodeOnly::Fs(FsOp::Metadata)),
    ("std::fs::symlink_metadata", NodeOnly::Fs(FsOp::SymlinkMetadata)),
    ("std::fs::create_dir", NodeOnly::Fs(FsOp::CreateDir)),
    ("std::fs::create_dir_all", NodeOnly::Fs(FsOp::CreateDirAll)),
    ("std::fs::remove_dir", NodeOnly::Fs(FsOp::RemoveDir)),
    ("std::fs::read_dir", NodeOnly::Fs(FsOp::ReadDir)),
];

/// Whether `def_id`, a function, is given its caller's location: its own
/// `#[track_caller]`, or its trait's declaration's, which rustc's
/// `codegen_fn_attrs` says of both (ADR 0427).
pub(crate) fn tracks_caller(tcx: TyCtxt<'_>, def_id: DefId) -> bool {
    matches!(tcx.def_kind(def_id), DefKind::Fn | DefKind::AssocFn)
        && tcx
            .codegen_fn_attrs(def_id)
            .flags
            .contains(rustc_middle::middle::codegen_fn_attrs::CodegenFnAttrFlags::TRACK_CALLER)
}

/// Whether `def_id` is a C-variadic function, `extern "C" fn f(mut args:
/// ...)`, whose last parameter gathers what it's given past the rest
/// (ADR 0428).
pub(crate) fn is_c_variadic(tcx: TyCtxt<'_>, def_id: DefId) -> bool {
    matches!(tcx.def_kind(def_id), DefKind::Fn | DefKind::AssocFn)
        && tcx.fn_sig(def_id).skip_binder().skip_binder().c_variadic()
}

/// Whether `def_id` is marked `#[inline(never)]`: kept out of its callers'
/// place, by rust-js too, and written `/** @noinline */`, Closure Compiler's
/// same meaning (ADR 0649). `#[inline]` and `#[inline(always)]` say
/// nothing: a JS engine decides.
pub(crate) fn is_noinline(tcx: TyCtxt<'_>, def_id: DefId) -> bool {
    rustc_hir::find_attr!(tcx, def_id, Inline(rustc_hir::attrs::InlineAttr::Never, _))
}

/// Which of std's functions that work in Node only `id` is, if it's one.
pub(crate) fn node_only(tcx: TyCtxt<'_>, id: DefId) -> Option<NodeOnly> {
    let path = std_path(tcx, id);
    NODE_ONLY.iter().find(|(of, _)| *of == path).map(|&(_, op)| op)
}

/// std's functions of addressable memory, which JavaScript has none of
/// (ADR 0390): `std::ptr`'s and a raw pointer's own, and what makes a value
/// of bytes, `transmute` and `zeroed`.
pub(crate) fn addresses_memory(tcx: TyCtxt<'_>, id: DefId) -> bool {
    let path = std_path(tcx, id);
    path.starts_with("std::ptr::") || matches!(path, "std::mem::transmute_copy" | "std::mem::uninitialized")
}

/// std's functions that may give a value of a type they weren't given one
/// of (ADR 0907): what addresses memory, an intrinsic of memory,
/// `transmute`'s, and a downcast of a `dyn Any`, which gives what was
/// made one anywhere. Every other std function gives only what it was
/// given, or makes of it: an empty `Vec<T>` holds no `T`.
pub(crate) fn gives_what_it_isnt_given(tcx: TyCtxt<'_>, id: DefId) -> bool {
    addresses_memory(tcx, id)
        || memory_intrinsic(tcx, id)
        || (is_std_crate(tcx, id) && tcx.item_name(id).as_str().starts_with("downcast"))
}

/// Whether `ty` is a `TrustedLen`, whose `Vec::from_iter` sizes it by its
/// upper bound, as rustc selects it (ADR 0940).
pub(crate) fn is_trusted_len<'tcx>(tcx: TyCtxt<'tcx>, typing_env: ty::TypingEnv<'tcx>, ty: Ty<'tcx>) -> bool {
    super::fact_trait(tcx, TypeFact::TrustedLen).is_some_and(|trusted| {
        let tr = ty::TraitRef::new(tcx, trusted, [ty]);
        crate::lower::traits::select(tcx, typing_env, tr).is_ok()
    })
}

/// Whether std's zip of `a` and `b`, `IntoIterator`s, reads each item by
/// its index, as its
/// `TrustedRandomAccess` specialization does, where that's seen: both
/// sides `TrustedRandomAccess`, a slice's or a range's say, and a step of
/// `a` may run code, its `MAY_HAVE_SIDE_EFFECT`, a `map`'s closure or a
/// `cloned()`'s clone. It pairs while both have an item left, so `a` is
/// never stepped once `b` has none, where the general impl steps `a` first
/// (ADR 0973).
pub(crate) fn zips_by_index<'tcx>(
    tcx: TyCtxt<'tcx>,
    typing_env: ty::TypingEnv<'tcx>,
    a: Ty<'tcx>,
    b: Ty<'tcx>,
) -> bool {
    let [a, b] = [a, b].map(|ty| iterator_type_of(tcx, typing_env, ty));
    let Some(random_access) = super::fact_trait(tcx, TypeFact::RandomAccess) else {
        return false;
    };
    let is_random_access = |ty: Ty<'tcx>| {
        let tr = tcx.erase_and_anonymize_regions(ty::TraitRef::new(tcx, random_access, [ty]));
        crate::lower::traits::select(tcx, typing_env, tr).is_ok()
    };
    is_random_access(a) && is_random_access(b) && steps_code(tcx, typing_env, a)
}

/// `<ty as IntoIterator>::IntoIter`, what `ty` is stepped as, `zip`'s
/// argument's: an iterator's, itself.
pub(crate) fn iterator_type_of<'tcx>(tcx: TyCtxt<'tcx>, typing_env: ty::TypingEnv<'tcx>, ty: Ty<'tcx>) -> Ty<'tcx> {
    let into_iterator = std_item(tcx, StdItem::IntoIterator);
    let into_iter = trait_method(tcx, into_iterator, "IntoIter");
    let projection = Ty::new_projection(tcx, ty::IsRigid::No, into_iter, [ty]);
    tcx.try_normalize_erasing_regions(typing_env, ty::Unnormalized::new_wip(projection))
        .unwrap_or(projection)
}

/// `MAY_HAVE_SIDE_EFFECT` of `ty`, a `TrustedRandomAccess`: whether reading
/// an item of it may run code. Where rustc can't say, it may.
pub(crate) fn steps_code<'tcx>(tcx: TyCtxt<'tcx>, typing_env: ty::TypingEnv<'tcx>, ty: Ty<'tcx>) -> bool {
    let effect = super::fact_trait(tcx, TypeFact::RandomAccessNoCoerce).and_then(|no_coerce| {
        tcx.associated_items(no_coerce)
            .filter_by_name_unhygienic(Symbol::intern("MAY_HAVE_SIDE_EFFECT"))
            .next()
            .map(|item| item.def_id)
    });
    let value = effect.and_then(|effect| {
        let args = tcx.mk_args(&[tcx.erase_and_anonymize_regions(ty).into()]);
        let instance = ty::Instance::try_resolve(tcx, typing_env, effect, args).ok()??;
        let value = tcx
            .const_eval_instance(typing_env, instance, rustc_span::DUMMY_SP)
            .ok()?;
        value.try_to_scalar()?.to_bool().discard_err()
    });
    value.unwrap_or(true)
}

/// Whether `id` is `mem::drop`, which closes an iterator it's given, of
/// either representation (ADR 0871).
pub(crate) fn is_mem_drop(tcx: TyCtxt<'_>, id: DefId) -> bool {
    tcx.is_diagnostic_item(rustc_span::sym::mem_drop, id)
}

/// Whether `id` is `Iterator::by_ref`, whose `&mut` lends what it's of.
pub(crate) fn is_by_ref(tcx: TyCtxt<'_>, id: DefId) -> bool {
    tcx.trait_of_assoc(id)
        .is_some_and(|tr| is_std_def(tcx, tr, StdItem::Iterator))
        && tcx.item_name(id).as_str() == "by_ref"
}

/// Whether `id` is `Iterator::flat_map`, which steps what its function
/// gives, there.
pub(crate) fn is_flat_map(tcx: TyCtxt<'_>, id: DefId) -> bool {
    tcx.trait_of_assoc(id)
        .is_some_and(|tr| is_std_def(tcx, tr, StdItem::Iterator))
        && tcx.item_name(id).as_str() == "flat_map"
}

/// Whether `param` is a trait's `Self`.
pub(crate) fn is_trait_self(param: ty::ParamTy) -> bool {
    param.name == rustc_span::symbol::kw::SelfUpper
}

/// std's `ExactSizeIterator`.
pub(crate) fn exact_size_iterator_trait(tcx: TyCtxt<'_>) -> Option<DefId> {
    tcx.get_diagnostic_item(rustc_span::Symbol::intern("ExactSizeIterator"))
}

/// A trait only std can name: private to its crate, or unstable, which a
/// program on stable Rust can't implement or name (ADR 0110).
pub(crate) fn std_private(tcx: TyCtxt<'_>, trait_def: DefId) -> bool {
    !tcx.visibility(trait_def).is_public() || tcx.lookup_stability(trait_def).is_some_and(|s| s.is_unstable())
}

/// The module of `@rust-js/std` a std body is written into: std's own
/// module's name under its crate, `iter` of `core::iter::Peekable`'s.
pub(crate) fn std_home(tcx: TyCtxt<'_>, id: DefId) -> String {
    let path = std_path(tcx, id);
    let mut parts = path.split("::");
    let _krate = parts.next();
    parts.next().unwrap_or_default().to_string()
}

/// An intrinsic of addressable memory, or of anything JS hasn't (ADR
/// 0561): any but a hint to the optimizer, `cold_path`,
/// `assume`, `likely`, `unlikely`, `black_box`, and `ub_checks`, which
/// rust-js reads as off (ADR 0364).
pub(crate) fn memory_intrinsic(tcx: TyCtxt<'_>, id: DefId) -> bool {
    tcx.intrinsic(id).is_some()
        && !matches!(
            std_path(tcx, id),
            "std::intrinsics::cold_path"
                | "std::intrinsics::assume"
                | "std::intrinsics::likely"
                | "std::intrinsics::unlikely"
                | "std::intrinsics::black_box"
                | "std::intrinsics::ub_checks"
        )
}

/// Whether `id` is std's `ub_checks::check_language_ub`, which reads
/// `UbChecks`, off as rust-js reads it (ADRs 0364, 0917).
pub(crate) fn is_check_language_ub(tcx: TyCtxt<'_>, id: DefId) -> bool {
    is_std_crate(tcx, id) && std_path(tcx, id) == "std::ub_checks::check_language_ub"
}

/// The name `id` has in `std::f64::consts`, `LN_2`, if it's one of its
/// constants, `core`'s by any path (ADR 0887).
pub(crate) fn f64_const(tcx: TyCtxt<'_>, id: DefId) -> Option<String> {
    if !is_std_crate(tcx, id) || !matches!(tcx.def_kind(id), DefKind::Const { .. }) {
        return None;
    }
    std_path(tcx, id).strip_prefix("std::f64::consts::").map(str::to_owned)
}

/// Whether `id` is `mem::transmute`, which reads one type's bytes as
/// another's (ADR 0408).
pub(crate) fn is_transmute(tcx: TyCtxt<'_>, id: DefId) -> bool {
    tcx.is_diagnostic_item(sym::transmute, id)
}

/// A `char`'s `to_uppercase()` or `to_lowercase()`: the `char`s of its
/// mapping, an array (ADR 0070).
/// Whether what's written at `span` is what std's `concat!` made, the one
/// string literal of its arguments (ADR 0641): by the macro, not its name,
/// which a crate's own may have.
pub(crate) fn from_concat(tcx: TyCtxt<'_>, span: rustc_span::Span) -> bool {
    let expansion = span.ctxt().outer_expn_data();
    matches!(expansion.kind, ExpnKind::Macro(MacroKind::Bang, _))
        && (expansion.macro_def_id).is_some_and(|id| tcx.is_diagnostic_item(Symbol::intern("macro_concat"), id))
}

/// `pin!`'s `Pin::new_unchecked` of the variable it pins.
pub(crate) fn is_pin_macro_fn(tcx: TyCtxt<'_>, def_id: DefId) -> bool {
    std_path(tcx, def_id) == "std::pin::pin_new_unchecked_in_helper"
}

impl<'a, 'tcx> Recognition<'a, 'tcx> {
    /// A map's `VacantEntry`, `[m, key]` (ADR 0345), hashbrown's too (ADR 0423).
    pub(in crate::lower) fn is_vacant_entry(&self, ty: ty::Ty<'tcx>) -> bool {
        matches!(ty.kind(), ty::Adt(adt, _) if map_entry_item(self.tcx, adt.did(), "VacantEntry"))
    }

    /// A map's `OccupiedEntry`, `[m, key]` (ADR 0345), hashbrown's too (ADR 0423).
    pub(in crate::lower) fn is_occupied_entry(&self, ty: ty::Ty<'tcx>) -> bool {
        matches!(ty.kind(), ty::Adt(adt, _) if map_entry_item(self.tcx, adt.did(), "OccupiedEntry"))
    }
}

/// Is `id` std's, core's or alloc's?
/// Is `id` indexmap's `IndexMap` or `IndexSet`, std's maps that keep the
/// order they're added in (ADR 0424)?
pub(super) fn is_index_map(tcx: TyCtxt<'_>, id: DefId) -> bool {
    tcx.crate_name(id.krate).as_str() == "indexmap" && matches!(bound_map_item(tcx, id), Some("HashMap" | "HashSet"))
}

/// std's map entry `name`, or that of a crate's map that's std's (ADRs
/// 0423, 0424).
fn map_entry_item(tcx: TyCtxt<'_>, id: DefId, name: &str) -> bool {
    match bound_map_item(tcx, id) {
        Some(item) => item == name,
        None => is_std_crate(tcx, id) && tcx.item_name(id).as_str() == name,
    }
}

impl<'tcx> Recognition<'_, 'tcx> {
    /// Whether `id` is the memchr rustc builds, not one rust-js compiled
    /// (ADR 0529), nor one Cargo built as its types alone, not qualified,
    /// which says so beside its metadata.
    fn is_shipped_memchr(&self, id: DefId) -> bool {
        self.tcx.crate_name(id.krate).as_str() == "memchr"
            && !self.foreign.in_library(id)
            && crate::lower::refusal::uncompiled(self.tcx, id).is_none()
    }

    /// Which of memchr's searches `id` is (ADR 0529): `memchr`, `memchr2`
    /// and `memchr3`, their `memrchr`s, and each one's iterator, by its name.
    pub(super) fn memchr_op(&self, id: DefId) -> Option<crate::lower::shipped::MemchrOp> {
        use crate::lower::shipped::MemchrOp;
        // core's own, `slice::memchr::memchr`, which std's bodies call,
        // `read_until`'s (#1156): of one needle, as memchr's.
        if of_std_crate(self.tcx, id, StdCrate::Core) {
            return match std_path(self.tcx, id) {
                "std::slice::memchr::memchr" => Some(MemchrOp::Find { back: false }),
                "std::slice::memchr::memrchr" => Some(MemchrOp::Find { back: true }),
                _ => None,
            };
        }
        if !self.is_shipped_memchr(id) {
            return None;
        }
        Some(match self.tcx.item_name(id).as_str() {
            "memchr" | "memchr2" | "memchr3" => MemchrOp::Find { back: false },
            "memrchr" | "memrchr2" | "memrchr3" => MemchrOp::Find { back: true },
            "memchr_iter" | "memchr2_iter" | "memchr3_iter" => MemchrOp::Iter { back: false },
            "memrchr_iter" | "memrchr2_iter" | "memrchr3_iter" => MemchrOp::Iter { back: true },
            _ => return None,
        })
    }

    /// Whether `ty` is memchr's iterator of its searches, `Memchr`,
    /// `Memchr2` or `Memchr3`: a JS iterator that knows its back and
    /// carries its clone (ADR 0529).
    pub(crate) fn is_memchr_iter(&self, ty: Ty<'tcx>) -> bool {
        matches!(ty.kind(), ty::Adt(adt, _) if self.is_shipped_memchr(adt.did())
            && matches!(self.tcx.item_name(adt.did()).as_str(), "Memchr" | "Memchr2" | "Memchr3"))
    }
}

/// Which of std's map items an item of a crate whose maps are std's is:
/// hashbrown's, what std's are made of (ADR 0423), and indexmap's, kept in
/// the order they're added, as a JS `Map` is (ADR 0424). `HashMap`,
/// `HashSet`, or its map's `Entry`, `OccupiedEntry` or `VacantEntry`.
fn bound_map_item(tcx: TyCtxt<'_>, id: DefId) -> Option<&'static str> {
    match tcx.crate_name(id.krate).as_str() {
        // By its own path: its set's and its table's have entries too.
        "hashbrown" => Some(match ty::print::with_no_visible_paths!(tcx.def_path_str(id)).as_str() {
            "hashbrown::map::HashMap" => "HashMap",
            "hashbrown::set::HashSet" => "HashSet",
            "hashbrown::map::Entry" => "Entry",
            "hashbrown::map::OccupiedEntry" => "OccupiedEntry",
            "hashbrown::map::VacantEntry" => "VacantEntry",
            _ => return None,
        }),
        // By its name: its entries are its map's alone.
        "indexmap" => Some(match tcx.item_name(id).as_str() {
            "IndexMap" => "HashMap",
            "IndexSet" => "HashSet",
            "Entry" => "Entry",
            "OccupiedEntry" => "OccupiedEntry",
            "VacantEntry" => "VacantEntry",
            _ => return None,
        }),
        _ => None,
    }
}

/// Is `id` of a crate linked at two versions, whose paths alone don't tell
/// the two apart (ADR 0331)? Or of a crate named as one of std's is, a
/// workspace's `core` beside std's, the crate compiled too, which every
/// crate that sees it sees beside std's: never std's own, whose paths are
/// then theirs alone, the same in every crate, as `@rust-js/std` reads
/// a `String`'s (ADR 1054).
pub(crate) fn linked_twice(tcx: TyCtxt<'_>, id: DefId) -> bool {
    let name = tcx.crate_name(id.krate);
    !is_std_crate(tcx, id)
        && (tcx.crates(()).iter().chain([&rustc_span::def_id::LOCAL_CRATE]))
            .filter(|&&c| tcx.crate_name(c) == name)
            .count()
            > 1
}

/// A std item another module asks about by name: only recognition spells
/// std's names, as its diagnostic items, which the rest ask by this.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum StdItem {
    Any,
    Arc,
    /// `hash::BuildHasherDefault<H>`, which holds nothing, building an
    /// `H::default()` (#1155).
    BuildHasherDefault,
    ArrayIntoIter,
    AsMut,
    AsRef,
    Atomic,
    BinaryHeap,
    Borrow,
    BorrowMut,
    BoxNew,
    BTreeEntry,
    BTreeMap,
    BTreeSet,
    Cell,
    ControlFlow,
    Debug,
    Default,
    Display,
    Eq,
    Error,
    FmtWrite,
    Formatter,
    IoWrite,
    /// `io::Read`, `io::BufRead` and `io::Seek` (#1156).
    IoRead,
    IoBufRead,
    IoSeek,
    From,
    Hash,
    HashMap,
    HashMapEntry,
    HashSet,
    FromIterator,
    Into,
    IntoIterator,
    Iterator,
    /// `LazyCell` or `LazyLock`, which have no diagnostic items, as `OnceCell`.
    LazyCell,
    LinkedList,
    LocalKey,
    MaybeUninit,
    Mutex,
    MutexGuard,
    /// `std::ptr::NonNull`, which is its pointer (ADR 0407).
    NonNull,
    /// `std::ops::RangeBounds`, a range's bounds, which generic code asks.
    RangeBounds,
    SliceIndex,
    /// `OnceCell` or `OnceLock`, which have no diagnostic items.
    OnceCell,
    /// `std::future::Ready`, a future already done, which has none either:
    /// an `.await` of it runs nothing (#965).
    FutureReady,
    Ord,
    Rc,
    RefCell,
    RefCellRef,
    RefCellRefMut,
    Result,
    RwLock,
    RwLockReadGuard,
    RwLockWriteGuard,
    SliceIter,
    ToString,
    TryFrom,
    TryInto,
    Vec,
    VecDeque,
    Wrapping,
}

impl StdItem {
    /// Its diagnostic item's name in std.
    pub(super) fn name(self) -> Symbol {
        match self {
            StdItem::Any => Symbol::intern("Any"),
            StdItem::Arc => sym::Arc,
            StdItem::ArrayIntoIter => Symbol::intern("ArrayIntoIter"),
            StdItem::AsMut => Symbol::intern("AsMut"),
            StdItem::AsRef => sym::AsRef,
            StdItem::Atomic => Symbol::intern("Atomic"),
            StdItem::BinaryHeap => Symbol::intern("BinaryHeap"),
            StdItem::Borrow => Symbol::intern("Borrow"),
            StdItem::BorrowMut => Symbol::intern("BorrowMut"),
            StdItem::BoxNew => Symbol::intern("box_new"),
            StdItem::BTreeEntry => Symbol::intern("BTreeEntry"),
            StdItem::BTreeMap => Symbol::intern("BTreeMap"),
            StdItem::BTreeSet => Symbol::intern("BTreeSet"),
            StdItem::Cell => Symbol::intern("Cell"),
            StdItem::ControlFlow => Symbol::intern("ControlFlow"),
            StdItem::Debug => Symbol::intern("Debug"),
            StdItem::Default => Symbol::intern("Default"),
            StdItem::Display => Symbol::intern("Display"),
            StdItem::Eq => sym::Eq,
            StdItem::Error => Symbol::intern("Error"),
            StdItem::FmtWrite => Symbol::intern("FmtWrite"),
            StdItem::Formatter => Symbol::intern("Formatter"),
            StdItem::IoWrite => Symbol::intern("IoWrite"),
            StdItem::IoRead => Symbol::intern("IoRead"),
            StdItem::IoBufRead => Symbol::intern("IoBufRead"),
            StdItem::IoSeek => Symbol::intern("IoSeek"),
            StdItem::From => sym::From,
            StdItem::Hash => sym::Hash,
            StdItem::HashMap => sym::HashMap,
            StdItem::HashMapEntry => Symbol::intern("HashMapEntry"),
            StdItem::HashSet => sym::HashSet,
            StdItem::FromIterator => sym::FromIterator,
            StdItem::Into => sym::Into,
            StdItem::IntoIterator => sym::IntoIterator,
            StdItem::Iterator => sym::Iterator,
            StdItem::LazyCell => Symbol::intern("LazyCell"),
            StdItem::LinkedList => Symbol::intern("LinkedList"),
            StdItem::LocalKey => Symbol::intern("LocalKey"),
            StdItem::MaybeUninit => Symbol::intern("MaybeUninit"),
            StdItem::Mutex => Symbol::intern("Mutex"),
            StdItem::MutexGuard => Symbol::intern("MutexGuard"),
            StdItem::NonNull => Symbol::intern("NonNull"),
            StdItem::RangeBounds => Symbol::intern("RangeBounds"),
            StdItem::SliceIndex => Symbol::intern("SliceIndex"),
            StdItem::OnceCell => Symbol::intern("OnceCell"),
            StdItem::FutureReady => Symbol::intern("FutureReady"),
            StdItem::BuildHasherDefault => Symbol::intern("BuildHasherDefault"),
            StdItem::Ord => sym::Ord,
            StdItem::Rc => sym::Rc,
            StdItem::RefCell => Symbol::intern("RefCell"),
            StdItem::RefCellRef => Symbol::intern("RefCellRef"),
            StdItem::RefCellRefMut => Symbol::intern("RefCellRefMut"),
            StdItem::Result => sym::Result,
            StdItem::RwLock => Symbol::intern("RwLock"),
            StdItem::RwLockReadGuard => Symbol::intern("RwLockReadGuard"),
            StdItem::RwLockWriteGuard => Symbol::intern("RwLockWriteGuard"),
            StdItem::SliceIter => Symbol::intern("SliceIter"),
            StdItem::ToString => Symbol::intern("ToString"),
            StdItem::TryFrom => sym::TryFrom,
            StdItem::TryInto => Symbol::intern("TryInto"),
            StdItem::Vec => sym::Vec,
            StdItem::VecDeque => Symbol::intern("VecDeque"),
            StdItem::Wrapping => Symbol::intern("Wrapping"),
        }
    }
}

/// Is `id` `mem::swap`, `mem::replace` or `mem::take`, which replace what a
/// `&mut` is to whole (ADR 0147), or `Cell::from_mut`, whose cell's writes
/// do, in place (ADR 0153, amended, #527)?
pub(crate) fn replaces_whole(tcx: TyCtxt<'_>, id: DefId) -> bool {
    tcx.is_diagnostic_item(Symbol::intern("mem_swap"), id)
        || tcx.is_diagnostic_item(Symbol::intern("mem_replace"), id)
        || of_std_crate(tcx, id, StdCrate::Core)
            && matches!(std_path(tcx, id), "std::mem::take" | "std::cell::Cell::<T>::from_mut")
}

/// Is `id` a slice of arrays' `as_flattened_mut`, a `&mut` to the items of
/// each array, which it changes through?
pub(crate) fn flattens_mut(tcx: TyCtxt<'_>, id: DefId) -> bool {
    of_std_crate(tcx, id, StdCrate::Core)
        && tcx.item_name(id).as_str() == "as_flattened_mut"
        && tcx.inherent_impl_of_assoc(id).is_some()
}

/// Is `id` std's `FromStr`, which has no diagnostic item to know it by?
pub(crate) fn is_from_str(tcx: TyCtxt<'_>, id: DefId) -> bool {
    of_std_crate(tcx, id, StdCrate::Core)
        && tcx.def_kind(id) == DefKind::Trait
        && std_path(tcx, id).ends_with("str::FromStr")
}

pub(crate) fn is_std_def(tcx: TyCtxt<'_>, id: DefId, item: StdItem) -> bool {
    match item {
        // Known by its crate and name: `core`'s `OnceCell` and `LazyCell`,
        // `std`'s `OnceLock` and `LazyLock`.
        // `core`'s `future::Ready`, by its crate and name.
        StdItem::FutureReady => {
            tcx.def_kind(id) == DefKind::Struct
                && tcx.item_name(id).as_str() == "Ready"
                && of_std_crate(tcx, id, StdCrate::Core)
        }
        // `core`'s `hash::BuildHasherDefault`, by its crate and name.
        StdItem::BuildHasherDefault => {
            tcx.def_kind(id) == DefKind::Struct
                && tcx.item_name(id).as_str() == "BuildHasherDefault"
                && of_std_crate(tcx, id, StdCrate::Core)
        }
        StdItem::OnceCell | StdItem::LazyCell => {
            let (cell, lock) = match item {
                StdItem::OnceCell => ("OnceCell", "OnceLock"),
                _ => ("LazyCell", "LazyLock"),
            };
            tcx.def_kind(id) == DefKind::Struct && {
                let name = tcx.item_name(id);
                name.as_str() == cell && of_std_crate(tcx, id, StdCrate::Core)
                    || name.as_str() == lock && of_std_crate(tcx, id, StdCrate::Std)
            }
        }
        // A lang item, which has no diagnostic item.
        StdItem::MaybeUninit => tcx.is_lang_item(id, LangItem::MaybeUninit),
        // A crate's whose maps are std's (ADRs 0423, 0424).
        StdItem::HashMap | StdItem::HashSet | StdItem::HashMapEntry if let Some(bound) = bound_map_item(tcx, id) => {
            let name = match item {
                StdItem::HashMap => "HashMap",
                StdItem::HashSet => "HashSet",
                _ => "Entry",
            };
            bound == name
        }
        _ => tcx.is_diagnostic_item(item.name(), id),
    }
}

/// Std's `item`, which every std has: one of `core`'s, which a `#![no_std]`
/// crate has too. One of `alloc`'s or `std`'s may not be loaded.
pub(crate) fn std_item(tcx: TyCtxt<'_>, item: StdItem) -> DefId {
    tcx.get_diagnostic_item(item.name()).expect("std has it")
}

/// Std's `item` if the crate loads what has it: `alloc`'s `ToString` isn't
/// a `#![no_std]` crate's unless it loads `alloc`.
pub(crate) fn opt_std_item(tcx: TyCtxt<'_>, item: StdItem) -> Option<DefId> {
    tcx.get_diagnostic_item(item.name())
}

/// The method of `trait_id` named `name`, which it has.
pub(crate) fn trait_method(tcx: TyCtxt<'_>, trait_id: DefId, name: &str) -> DefId {
    tcx.associated_item_def_ids(trait_id)
        .iter()
        .copied()
        .find(|&id| tcx.item_name(id).as_str() == name)
        .expect("the trait has the method")
}

impl<'a, 'tcx> Recognition<'a, 'tcx> {
    /// The crate's own `eq` of `adt`, its impl's of `PartialEq`, where it
    /// has no `ne` of its own, which may say otherwise (ADR 0864).
    pub(in crate::lower) fn own_eq(&self, adt: DefId) -> Option<DefId> {
        self.own_impl_method(adt, self.tcx.lang_items().eq_trait()?, "eq")
    }

    /// Whether `adt`'s `PartialEq<Self>` is a binding's of JS's `===`, its
    /// `eq` `#[rust_js::link_name = "==="]`, its `ne` std's: what a binding
    /// crate, whose bodies rust-js never reads, says of a JS object equal
    /// only to itself, webapi's classes' (ADR 1016).
    pub(in crate::lower) fn eq_bound_to_strict_equality(&self, adt: DefId) -> bool {
        let tcx = self.tcx;
        let Some(partial_eq) = tcx.lang_items().eq_trait() else {
            return false;
        };
        let of_adt = |ty: Ty<'tcx>| matches!(ty.kind(), ty::Adt(of, _) if of.did() == adt);
        let (eq, ne) = (trait_method(tcx, partial_eq, "eq"), trait_method(tcx, partial_eq, "ne"));
        let impls = tcx.trait_impls_of(partial_eq).non_blanket_impls();
        let simplified = rustc_middle::ty::fast_reject::SimplifiedType::Adt(adt);
        impls.get(&simplified).into_iter().flatten().any(|&imp| {
            let tr = tcx.impl_trait_ref(imp).instantiate_identity().skip_normalization();
            let implementors = tcx.impl_item_implementor_ids(imp);
            of_adt(tr.self_ty())
                && of_adt(tr.args.type_at(1))
                && !implementors.contains_key(&ne)
                && (implementors.get(&eq)).is_some_and(|&eq| {
                    super::super::bindings::is_binding(tcx, eq)
                        && matches!(
                            super::super::bindings::js_form(tcx, eq),
                            super::super::bindings::JsForm::StrictEquals
                        )
                })
        })
    }

    /// The crate's own `eq` and `hash` of `adt`, its impls' of `PartialEq`
    /// and `Hash`, `own_eq`'s and its `Hash`'s (ADR 0864).
    pub(in crate::lower) fn eq_and_hash(&self, adt: DefId) -> Option<(DefId, DefId)> {
        Some((
            self.own_eq(adt)?,
            self.own_impl_method(adt, std_item(self.tcx, StdItem::Hash), "hash")?,
        ))
    }

    /// The method `method` of the crate's impl of `trait_id` for `adt`; of
    /// `PartialEq`, none where it has a `ne` of its own.
    fn own_impl_method(&self, adt: DefId, trait_id: DefId, method: &str) -> Option<DefId> {
        let tcx = self.tcx;
        let own = |trait_id: DefId, method: &str| -> Option<DefId> {
            let imp = crate::lower::traits::declared_impls_of(tcx, trait_id).find(|&imp| {
                matches!(tcx.type_of(imp).instantiate_identity().skip_normalization().kind(),
                    ty::Adt(of, _) if of.did() == adt)
            })?;
            let implementors = tcx.impl_item_implementor_ids(imp);
            let partial_eq = tcx.lang_items().eq_trait()?;
            if trait_id == partial_eq && implementors.contains_key(&trait_method(tcx, partial_eq, "ne")) {
                return None;
            }
            implementors.get(&trait_method(tcx, trait_id, method)).copied()
        };
        own(trait_id, method)
    }
}

/// Is `id` the method `name` of std's trait `item`, `Error::source`?
pub(crate) fn is_std_method(tcx: TyCtxt<'_>, id: DefId, item: StdItem, name: &str) -> bool {
    tcx.item_name(id).as_str() == name && tcx.trait_of_assoc(id).is_some_and(|tr| is_std_def(tcx, tr, item))
}

/// A std function that runs no code of the crate's (ADR 0069).
#[derive(Clone, Copy, PartialEq, Debug)]
pub(crate) enum PureStd {
    /// A question of what it's called on: `len()`, `is_some()`, `is_ok()`.
    Question,
    /// A copy or a string of it, which is pure where what it copies is:
    /// `clone()`, `to_owned()`, `to_string()`, `as_str()`.
    Copy,
    /// `Box::new(x)`, which is `x`.
    BoxNew,
    /// `unwrap_unchecked()`, which is its value, and can't panic.
    Unchecked,
}

/// Which pure std function `id` is, if it's one.
pub(crate) fn pure_std(tcx: TyCtxt<'_>, id: DefId) -> Option<PureStd> {
    if !is_std_crate(tcx, id) {
        return None;
    }
    match tcx.item_name(id).as_str() {
        "len" | "is_empty" | "is_some" | "is_none" | "is_ok" | "is_err" if tcx.trait_of_assoc(id).is_none() => {
            Some(PureStd::Question)
        }
        "clone" | "to_owned" | "to_string" | "as_str" => Some(PureStd::Copy),
        "unwrap_unchecked" => Some(PureStd::Unchecked),
        _ if is_std_def(tcx, id, StdItem::BoxNew) => Some(PureStd::BoxNew),
        _ => None,
    }
}
