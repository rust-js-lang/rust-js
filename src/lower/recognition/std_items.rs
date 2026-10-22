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
        if tcx.crate_name(def_id.krate) == sym::core
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
        is_std_item(self.tcx, id)
    }

    /// Is `id` the standard library's: std, core, alloc, or a crate rustc's
    /// sysroot holds beside them, as `hashbrown`, which std's maps are made of?
    pub(in crate::lower) fn in_sysroot(&self, id: DefId) -> bool {
        self.is_std(id) || {
            let sysroots: Vec<&std::path::Path> = self.tcx.sess.opts.sysroot.all_paths().collect();
            self.tcx
                .used_crate_source(id.krate)
                .paths()
                .any(|path| sysroots.iter().any(|sysroot| path.starts_with(sysroot)))
        }
    }
}

/// A std item's path as std names it, `std::str::Chars`: a `#![no_std]`
/// crate's rustc names it `core::str::Chars`, or `alloc::..`, which every
/// path compared here would miss.
/// std's functions of the process a program runs in, which Node has and a
/// browser hasn't (ADR 0391), each by its path: the one list of what works
/// in Node only. Each is a helper of the runtime's `process.js`.
const NODE_ONLY: &[(&str, ProcessOp)] = &[
    ("std::env::args", ProcessOp::Args),
    ("std::env::var", ProcessOp::Var),
    ("std::env::var_os", ProcessOp::VarOs),
    ("std::env::vars", ProcessOp::Vars),
    ("std::env::vars_os", ProcessOp::VarsOs),
    ("std::process::exit", ProcessOp::Exit),
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

/// Which of std's functions that work in Node only `id` is, if it's one.
pub(crate) fn node_only(tcx: TyCtxt<'_>, id: DefId) -> Option<ProcessOp> {
    let path = std_path(tcx, id);
    NODE_ONLY.iter().find(|(of, _)| *of == path).map(|&(_, op)| op)
}

/// std's functions of addressable memory, which JavaScript has none of
/// (ADR 0390): `std::ptr`'s and a raw pointer's own, and what makes a value
/// of bytes, `transmute` and `zeroed`.
pub(crate) fn addresses_memory(tcx: TyCtxt<'_>, id: DefId) -> bool {
    let path = std_path(tcx, id);
    path.starts_with("std::ptr::") || matches!(path.as_str(), "std::mem::transmute_copy" | "std::mem::uninitialized")
}

/// std's bodies rust-js compiles from the MIR the toolchain ships, into
/// `@rust-js/std` (ADR 0561): each by its type, as
/// `core::iter` exports it, the trait of the impl, or none for the type's
/// own, and the method. Each is one the measure says is safe
/// (`docs/std-coverage.txt`); what reaches memory stays recognized.
pub(super) const COMPILED_STD: &[(&str, Option<&str>, &str)] = &[
    ("Peekable", Some("Iterator"), "next"),
    ("Peekable", Some("DoubleEndedIterator"), "next_back"),
    ("Peekable", Some("DoubleEndedIterator"), "rfold"),
    ("Peekable", Some("Clone"), "clone"),
    ("Peekable", Some("Debug"), "fmt"),
    ("Fuse", Some("Iterator"), "next"),
    ("Fuse", Some("Iterator"), "size_hint"),
    ("Fuse", Some("DoubleEndedIterator"), "next_back"),
    ("Fuse", Some("DoubleEndedIterator"), "rfold"),
    ("Fuse", Some("ExactSizeIterator"), "len"),
    ("Fuse", Some("FuseImpl"), "next"),
    ("Fuse", Some("FuseImpl"), "next_back"),
    ("Fuse", Some("Clone"), "clone"),
    ("Fuse", Some("Debug"), "fmt"),
];

/// And the iterator traits' provided methods compiled, each by its trait and
/// its name: a function over `Self`, given its dictionary (ADR 0185).
const COMPILED_STD_PROVIDED: &[(&str, &str)] = &[("Iterator", "peekable"), ("Iterator", "fuse")];

thread_local! {
    /// `compiled_std`'s answer, found once for the session's rustc.
    static COMPILED: std::cell::OnceCell<std::rc::Rc<Vec<DefId>>> = const { std::cell::OnceCell::new() };
}

/// The provided method `method` of std's trait `trait_name` compiled into
/// `@rust-js/std`, `Iterator::fuse`'s: the one place that finds it.
pub(crate) fn compiled_provided_method(tcx: TyCtxt<'_>, trait_name: &str, method: &str) -> Option<DefId> {
    let trait_def = tcx.get_diagnostic_item(Symbol::intern(trait_name))?;
    (tcx.associated_items(trait_def).in_definition_order())
        .find(|item| matches!(item.kind, ty::AssocKind::Fn { .. }) && item.name().as_str() == method)
        .map(|item| item.def_id)
}

/// std's bodies compiled into `@rust-js/std`: the items `COMPILED_STD` and
/// `COMPILED_STD_PROVIDED` name, each found in std, and the closures each
/// makes, found as they're collected.
pub(crate) fn compiled_std(tcx: TyCtxt<'_>) -> std::rc::Rc<Vec<DefId>> {
    COMPILED.with(|cache| cache.get_or_init(|| std::rc::Rc::new(find_compiled_std(tcx))).clone())
}

/// The module of a crate given std's bodies that a std body is written
/// into: the one named as std's own is, `iter` of `core::iter::Peekable`'s.
pub(crate) fn std_home_module(tcx: TyCtxt<'_>, id: DefId) -> Option<LocalModId> {
    let home = std_home(tcx, id);
    let mut found = None;
    tcx.hir_for_each_module(|module| {
        if found.is_none() && module != CRATE_MOD_ID && tcx.item_name(module.to_def_id()).as_str() == home {
            found = Some(module);
        }
    });
    found
}

/// Whether `id` is a std body compiled into `@rust-js/std`, one of
/// `compiled_std`'s, or a closure one makes.
pub(crate) fn is_compiled_std(tcx: TyCtxt<'_>, id: DefId) -> bool {
    !id.is_local()
        && (compiled_std(tcx).contains(&id) || {
            let mut owner = id;
            while matches!(tcx.def_kind(owner), DefKind::Closure) {
                owner = tcx.parent(owner);
            }
            owner != id && compiled_std(tcx).contains(&owner)
        })
}

fn find_compiled_std(tcx: TyCtxt<'_>) -> Vec<DefId> {
    let trait_named = |name: &str| tcx.get_diagnostic_item(Symbol::intern(name));
    let mut found = Vec::new();
    for &(type_name, trait_name, method) in COMPILED_STD {
        let Some(adt) = registry::by_path(tcx, "core", &["iter", type_name]) else {
            continue;
        };
        // A trait std keeps private is beside the type, `FuseImpl` in
        // `Fuse`'s module.
        let private_trait = |name: &str| {
            (tcx.module_children(tcx.parent(adt)).iter())
                .find(|child| child.ident.as_str() == name)
                .and_then(|child| child.res.opt_def_id())
        };
        // Or one `core::iter` exports without a diagnostic item,
        // `ExactSizeIterator`.
        let exported = |name: &str| registry::by_path(tcx, "core", &["iter", name]);
        let impls: Vec<DefId> = match trait_name {
            None => tcx.inherent_impls(adt).to_vec(),
            Some(trait_name) => match trait_named(trait_name)
                .or_else(|| exported(trait_name))
                .or_else(|| private_trait(trait_name))
            {
                Some(trait_def) => tcx
                    .all_impls(trait_def)
                    .filter(|&imp| matches!(tcx.type_of(imp).skip_binder().kind(), ty::Adt(of, _) if of.did() == adt))
                    .collect(),
                None => Vec::new(),
            },
        };
        for imp in impls {
            found.extend(
                (tcx.associated_items(imp).in_definition_order())
                    .filter(|item| matches!(item.kind, ty::AssocKind::Fn { .. }) && item.name().as_str() == method)
                    .map(|item| item.def_id),
            );
        }
    }
    for &(trait_name, method) in COMPILED_STD_PROVIDED {
        found.extend(compiled_provided_method(tcx, trait_name, method));
    }
    // And each function std keeps private that a compiled body calls,
    // `Fuse`'s `and_then_or_clear`, compiled with it, as its closures are:
    // no program can name it, so no recognizer can rewrite it.
    let mut at = 0;
    while at < found.len() {
        let def_id = found[at];
        at += 1;
        if !tcx.is_mir_available(def_id) {
            continue;
        }
        let typing_env = ty::TypingEnv::post_analysis(tcx, def_id);
        for data in tcx.optimized_mir(def_id).basic_blocks.iter() {
            let Some(terminator) = &data.terminator else { continue };
            let (TerminatorKind::Call { func, .. } | TerminatorKind::TailCall { func, .. }) = &terminator.kind else {
                continue;
            };
            let Some((callee, args)) = func.const_fn_def() else {
                continue;
            };
            if let Ok(Some(instance)) = crate::lower::items::resolve_instance_in(tcx, typing_env, callee, args)
                && let ty::InstanceKind::Item(called) = instance.def
                && matches!(tcx.def_kind(called), DefKind::Fn | DefKind::AssocFn)
                && !tcx.visibility(called).is_public()
                && tcx.trait_of_assoc(called).is_none()
                && tcx.trait_impl_of_assoc(called).is_none()
                && tcx.is_mir_available(called)
                && !found.contains(&called)
            {
                found.push(called);
            }
        }
    }
    found
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
            std_path(tcx, id).as_str(),
            "std::intrinsics::cold_path"
                | "std::intrinsics::assume"
                | "std::intrinsics::likely"
                | "std::intrinsics::unlikely"
                | "std::intrinsics::black_box"
                | "std::intrinsics::ub_checks"
        )
}

/// Whether `id` is std's: of `std`, `core` or `alloc`.
pub(crate) fn is_std_crate(tcx: TyCtxt<'_>, id: DefId) -> bool {
    !id.is_local() && matches!(tcx.crate_name(id.krate).as_str(), "core" | "alloc" | "std")
}

pub(super) fn std_path(tcx: TyCtxt<'_>, id: DefId) -> String {
    let mut path = tcx.def_path_str(id);
    // Named through a crate's own name for std's, hashbrown's `extern crate
    // alloc as stdalloc`: std's own path, `alloc::alloc::alloc`.
    if is_std_crate(tcx, id) && !matches!(path.split("::").next(), Some("std" | "core" | "alloc")) {
        path = ty::print::with_no_visible_paths!(tcx.def_path_str(id));
    }
    match path.split_once("::") {
        Some(("core" | "alloc", rest)) if !id.is_local() => format!("std::{rest}"),
        _ => path,
    }
}

/// Whether `id` is `mem::transmute`, which reads one type's bytes as
/// another's (ADR 0408).
pub(crate) fn is_transmute(tcx: TyCtxt<'_>, id: DefId) -> bool {
    tcx.is_diagnostic_item(sym::transmute, id)
}

/// A `char`'s `to_uppercase()` or `to_lowercase()`: the `char`s of its
/// mapping, an array (ADR 0070).
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
        None => is_std_item(tcx, id) && tcx.item_name(id).as_str() == name,
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

pub(in crate::lower) fn is_std_item(tcx: TyCtxt<'_>, id: DefId) -> bool {
    [sym::core, sym::alloc, sym::std].contains(&tcx.crate_name(id.krate))
}

/// Is `id` of a crate linked at two versions, whose paths alone don't tell
/// the two apart (ADR 0331)?
pub(crate) fn linked_twice(tcx: TyCtxt<'_>, id: DefId) -> bool {
    let name = tcx.crate_name(id.krate);
    tcx.crates(()).iter().filter(|&&c| tcx.crate_name(c) == name).count() > 1
}

/// A std type whose bodies are compiled into `@rust-js/std` (the std-bodies
/// decision), `Peekable`: std's struct, `{ iter, peeked }`, as a type of
/// the crate's is, wherever a value of it is made or read.
pub(crate) fn is_compiled_std_type(tcx: TyCtxt<'_>, id: DefId) -> bool {
    !id.is_local()
        && COMPILED_STD
            .iter()
            .any(|&(type_name, _, _)| std_path(tcx, id) == format!("std::iter::{type_name}"))
}

/// A std item another module asks about by name: only recognition spells
/// std's names, as its diagnostic items, which the rest ask by this.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum StdItem {
    Any,
    Arc,
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
    Debug,
    Default,
    Display,
    Eq,
    Error,
    FmtWrite,
    Formatter,
    IoWrite,
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
    Ord,
    PhantomData,
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
            StdItem::Debug => Symbol::intern("Debug"),
            StdItem::Default => Symbol::intern("Default"),
            StdItem::Display => Symbol::intern("Display"),
            StdItem::Eq => sym::Eq,
            StdItem::Error => Symbol::intern("Error"),
            StdItem::FmtWrite => Symbol::intern("FmtWrite"),
            StdItem::Formatter => Symbol::intern("Formatter"),
            StdItem::IoWrite => Symbol::intern("IoWrite"),
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
            StdItem::Ord => sym::Ord,
            StdItem::PhantomData => Symbol::intern("PhantomData"),
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
            StdItem::Vec => sym::Vec,
            StdItem::VecDeque => Symbol::intern("VecDeque"),
            StdItem::Wrapping => Symbol::intern("Wrapping"),
        }
    }
}

/// Is `id` `mem::swap`, `mem::replace` or `mem::take`, which replace what a
/// `&mut` is to whole (ADR 0147)?
pub(crate) fn replaces_whole(tcx: TyCtxt<'_>, id: DefId) -> bool {
    tcx.is_diagnostic_item(Symbol::intern("mem_swap"), id)
        || tcx.is_diagnostic_item(Symbol::intern("mem_replace"), id)
        || tcx.crate_name(id.krate) == sym::core && std_path(tcx, id) == "std::mem::take"
}

/// Is `id` std's `FromStr`, which has no diagnostic item to know it by?
pub(crate) fn is_from_str(tcx: TyCtxt<'_>, id: DefId) -> bool {
    tcx.crate_name(id.krate) == sym::core
        && tcx.def_kind(id) == DefKind::Trait
        && std_path(tcx, id).ends_with("str::FromStr")
}

pub(crate) fn is_std_def(tcx: TyCtxt<'_>, id: DefId, item: StdItem) -> bool {
    match item {
        // Known by its crate and name: `core`'s `OnceCell` and `LazyCell`,
        // `std`'s `OnceLock` and `LazyLock`.
        StdItem::OnceCell | StdItem::LazyCell => {
            let (cell, lock) = match item {
                StdItem::OnceCell => ("OnceCell", "OnceLock"),
                _ => ("LazyCell", "LazyLock"),
            };
            tcx.def_kind(id) == DefKind::Struct && {
                let name = tcx.item_name(id);
                name.as_str() == cell && tcx.crate_name(id.krate) == sym::core
                    || name.as_str() == lock && tcx.crate_name(id.krate) == sym::std
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
    if !is_std_item(tcx, id) {
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
