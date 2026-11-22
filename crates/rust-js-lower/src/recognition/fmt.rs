//! Formatting and writing: `fmt`'s traits, a `Formatter` and its builders,
//! and the writers `write!` writes to.

use super::*;
use rustc_middle::mir;

impl<'a, 'tcx> Recognition<'a, 'tcx> {
    /// Whether `ty` is standard output, or its lock, `Some(false)`, or
    /// standard error, `Some(true)` (ADR 0132).
    pub(crate) fn stream(&self, ty: Ty<'tcx>) -> Option<bool> {
        stream_of(self.tcx, ty)
    }

    /// Whether a call of `def_id`, given `args`, writes to standard output,
    /// or of `error`, standard error: `print!`'s or `eprint!`'s, or
    /// `io::Write`'s of `io::stdout()`, `io::stderr()` or a lock (#1490).
    pub(crate) fn writes_stream(&self, def_id: DefId, args: ty::GenericArgsRef<'tcx>, error: bool) -> bool {
        super::print_of(self.tcx, def_id) == Some(error)
            || (self.tcx.trait_of_assoc(def_id)).is_some_and(|trait_| is_std_path(self.tcx, trait_, StdPath::IoWrite))
                && args
                    .types()
                    .next()
                    .is_some_and(|ty| self.stream(ty.peel_refs()) == Some(error))
    }

    /// The runtime's `io::Write` of `ty`, or of what a `&mut` to it is to,
    /// if std's is one rust-js knows (ADR 0392): a `Vec<u8>`'s, which adds
    /// to it, a standard stream's, which writes to it, or a `Box<dyn
    /// Write>`'s, which is the pair it holds and writes through it.
    pub(crate) fn io_writer(&self, ty: Ty<'tcx>) -> Option<&'static str> {
        let ty = match ty.kind() {
            ty::Ref(_, inner, Mutability::Mut) => *inner,
            _ => ty,
        };
        match self.stream(ty) {
            Some(false) => return Some("$stdoutWriter"),
            Some(true) => return Some("$stderrWriter"),
            None => {}
        }
        if ty.boxed_ty().is_some_and(|inner| {
            matches!(inner.kind(), ty::Dynamic(traits, ..)
                if traits.principal_def_id().is_some_and(|id| is_std_def(self.tcx, id, StdItem::IoWrite)))
        }) {
            return Some("$dynWriter");
        }
        // A `File`'s, or a `&File`'s, a call of the OS's at its position
        // (#1156, #1397).
        if super::is_file_handle(self.tcx, ty) {
            return Some("$fileWriter");
        }
        // A `&mut [u8]`'s, its slice written and re-pointed past it, std's
        // cursor over a buffer (#1282).
        if is_byte_slice(ty) {
            return Some("$sliceWriter");
        }
        let bytes = matches!(ty.kind(), ty::Adt(_, args) if self.is_std_type(ty, StdItem::Vec)
            && matches!(args.type_at(0).kind(), ty::Uint(ty::UintTy::U8)));
        bytes.then_some("$vecWriter")
    }

    /// A write's result, `fmt::Result` or `io::Result<()>` (`is_write_result_type`).
    pub fn is_write_result(&self, ty: Ty<'tcx>) -> bool {
        is_write_result_type(self.tcx, ty)
    }

    /// `io::Result<()>`, a write's result (`is_write_result_type`).
    pub fn is_io_unit_result(&self, ty: Ty<'tcx>) -> bool {
        is_io_unit_result_type(self.tcx, ty)
    }

    /// The `fmt` trait a placeholder of `kind` calls, `{:x}`'s `LowerHex`,
    /// where it's one of `OTHER_FMT_TRAITS`.
    pub fn other_fmt_trait(&self, kind: Std) -> Option<DefId> {
        let name = match kind {
            Std::FmtRadix(Radix::LowerHex) => "LowerHex",
            Std::FmtRadix(Radix::UpperHex) => "UpperHex",
            Std::FmtRadix(Radix::Octal) => "Octal",
            Std::FmtRadix(Radix::Binary) => "Binary",
            Std::FmtExp(false) => "LowerExp",
            Std::FmtExp(true) => "UpperExp",
            Std::FmtPointer => "Pointer",
            _ => return None,
        };
        self.tcx
            .all_traits_including_private()
            .find(|&id| is_other_fmt_trait(self.tcx, id) && self.tcx.item_name(id).as_str() == name)
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum WriteCall {
    Text,
    /// `f.pad(s)`: `s`, given the `Formatter`'s options (ADR 0143).
    Pad,
    /// `f.pad_integral(non_negative, prefix, digits)`: a number's digits, as
    /// a number's are given the `Formatter`'s options.
    PadIntegral,
    Display,
    Debug,
    /// `LowerHex::fmt(x, f)` and the like: of a type of the crate's, its impl's
    /// (ADR 0165).
    OtherFmt,
    StructFields,
    TupleFields,
    Struct,
    Tuple,
    Function,
    Trait,
    Unsupported,
}

#[derive(Clone, Copy, PartialEq)]
pub(crate) enum SkipPredicate {
    None,
    Some,
    Empty,
}

impl<'a, 'tcx> Recognition<'a, 'tcx> {
    /// Whether `def_id`, of the trait `trait_` on `ty`, is a `fmt::Write`'s
    /// call std writes with the writer's `write_str`: `write!` and
    /// `write_char` into a writer of the crate's own (ADR 0166), and
    /// `write!` into a `W: fmt::Write`, a `dyn`'s too, each piece through
    /// its dictionary (#1652). One the crate's writer writes itself is
    /// called as it is.
    pub(crate) fn writes_through_write_str(&self, def_id: DefId, trait_: DefId, ty: Ty<'tcx>) -> bool {
        let ty = ty.peel_refs();
        let generic = super::known_by_a_dictionary(ty);
        self.tcx.is_diagnostic_item(Symbol::intern("FmtWrite"), trait_)
            && match self.tcx.item_name(def_id).as_str() {
                "write_fmt" => generic || self.has_user_impl(trait_, ty),
                name => name == "write_char" && self.has_user_impl(trait_, ty),
            }
    }

    /// A `fmt::Write`'s method, `def_id` of `trait_` on `ty`: `Some` once
    /// decided, of what it is or that it's none of std's, and `None` to
    /// look further. `write!(s, ..)` into a `String` is `s += ..`, as
    /// `push_str` is; writing to a string can't fail, so its `fmt::Result`
    /// is nothing (ADR 0148). A writer of the crate's own, or a `W:
    /// fmt::Write`: what `write!` and `write_char` give its `write_str` (ADR
    /// 0166, #1652).
    pub(super) fn classify_fmt_write(&self, def_id: DefId, trait_: DefId, ty: Ty<'tcx>) -> Option<Option<Std>> {
        let tcx = self.tcx;
        if self.is_lang_adt(ty.peel_refs(), LangItem::String)
            && tcx.is_diagnostic_item(Symbol::intern("FmtWrite"), trait_)
        {
            return Some(match tcx.item_name(def_id).as_str() {
                "write_fmt" | "write_str" | "write_char" => Some(Std::PushStr),
                _ => None,
            });
        }
        self.writes_through_write_str(def_id, trait_, ty)
            .then_some(Some(Std::UserWrite))
    }

    /// Whether a call of `def_id` of `generic_args` is a `write_fmt` into a
    /// writer std writes a piece at a time (`writes_through_write_str`):
    /// `w.write_fmt(args)`, or `fmt::write(w, args)`, which is it.
    pub fn writes_arguments_piecewise(&self, def_id: DefId, generic_args: ty::GenericArgsRef<'tcx>) -> bool {
        if self.is_fmt_write(def_id) || self.writes_through_a_dictionary(def_id, generic_args) {
            return true;
        }
        self.tcx.item_name(def_id).as_str() == "write_fmt"
            && self.classify(def_id, generic_args) == Some(Std::UserWrite)
    }

    /// Whether such a write (`writes_arguments_piecewise`) is into a writer
    /// only a caller's dictionary says, a `W: fmt::Write`'s or a `dyn`'s,
    /// which may define its own `write_fmt`: given the function of its
    /// `Arguments`, `W.writeFmt(w, args)`, as Rust calls `W::write_fmt`
    /// (#1652, step 6 of ADR 1135). The trait's method by its identity, not
    /// by what a recogniser made of it: the function is what every
    /// dictionary's `writeFmt` takes, recognised or not (#1891). Not
    /// `fmt::write`, std's own `write_fmt`, which writes through the
    /// writer's `write_str` whatever its own `write_fmt` (#1697).
    pub fn writes_through_a_dictionary(&self, def_id: DefId, generic_args: ty::GenericArgsRef<'tcx>) -> bool {
        super::is_write_fmt(self.tcx, def_id)
            && (generic_args.types().next()).is_some_and(|writer| super::known_by_a_dictionary(writer.peel_refs()))
    }

    /// `&mut Formatter<'_>`.
    /// `fmt::Write`'s `write_str(f, s)` of a `Formatter`, called through the
    /// trait: its `&mut Self` is the `Formatter`, as `f.write_str(s)`'s is.
    pub fn fmt_write_on_formatter(&self, def_id: DefId, args: ty::GenericArgsRef<'tcx>) -> bool {
        self.tcx
            .trait_of_assoc(def_id)
            .is_some_and(|t| self.tcx.is_diagnostic_item(Symbol::intern("FmtWrite"), t))
            && args
                .types()
                .next()
                .is_some_and(|this| self.is_std_type(this, StdItem::Formatter))
    }

    /// `f.write_str(s)`, `write!(f, ..)` or `f.write_char(c)` of a
    /// `Formatter`, inherent or `fmt::Write`'s.
    pub(crate) fn formatter_write(&self, def_id: DefId, args: ty::GenericArgsRef<'tcx>) -> bool {
        let inherent = (self.tcx.inherent_impl_of_assoc(def_id)).is_some_and(|imp| {
            self.is_std_type(
                self.tcx.type_of(imp).instantiate_identity().skip_normalization(),
                StdItem::Formatter,
            )
        });
        (inherent && matches!(self.tcx.item_name(def_id).as_str(), "write_str" | "write_fmt"))
            || self.fmt_write_on_formatter(def_id, args)
    }

    /// `write!(f, ..)` of a `Formatter`, inherent or `fmt::Write`'s.
    pub fn formatter_write_fmt(&self, def_id: DefId, args: ty::GenericArgsRef<'tcx>) -> bool {
        self.tcx.item_name(def_id).as_str() == "write_fmt" && self.formatter_write(def_id, args)
    }

    /// Is `def_id` a `write_char`, a `Formatter`'s or `fmt::Write`'s?
    pub fn is_write_char(&self, def_id: DefId) -> bool {
        self.tcx.item_name(def_id).as_str() == "write_char"
    }

    /// `Display::fmt(&x, f)`, `Debug::fmt` or another `fmt` trait's, called
    /// as a function, on a `Formatter` held where it isn't its writer's
    /// (ADR 0461): what it writes.
    pub(super) fn formatter_fmt_to(&self, def_id: DefId) -> Option<WriteCall> {
        let operation = self.write_call(def_id, false);
        if !matches!(operation, WriteCall::Display | WriteCall::Debug | WriteCall::OtherFmt) {
            return None;
        }
        let sig = self
            .tcx
            .instantiate_bound_regions_with_erased(self.tcx.fn_sig(def_id).instantiate_identity().skip_normalization());
        sig.inputs()
            .get(1)
            .is_some_and(|t| self.is_formatter(*t))
            .then_some(operation)
    }

    /// A `Formatter`, whose text is a string in JS (ADR 0054).
    pub fn is_formatter_type(&self, ty: Ty<'tcx>) -> bool {
        self.is_std_type(ty, StdItem::Formatter)
    }

    fn is_formatter(&self, ty: Ty<'tcx>) -> bool {
        matches!(ty.kind(), ty::Ref(_, inner, Mutability::Mut)
            if self.is_std_type(*inner, StdItem::Formatter))
    }
    pub(crate) fn display_trait(&self) -> DefId {
        self.tcx
            .get_diagnostic_item(Symbol::intern("Display"))
            .expect("std has `Display`")
    }
    pub(crate) fn is_fmt_result(&self, ty: Ty<'tcx>) -> bool {
        is_fmt_result_type(self.tcx, ty)
    }
    /// Is `ty` a `fmt::Error`, a `fmt::Result`'s `Err` (ADR 0187)?
    pub(crate) fn is_fmt_error(&self, ty: Ty<'tcx>) -> bool {
        matches!(fmt_result(self.tcx).map(|result| *result.kind()), Some(ty::Adt(_, args))
            if self.tcx.erase_and_anonymize_regions(ty) == self.tcx.erase_and_anonymize_regions(args.type_at(1)))
    }
    /// Whether `def_id`, a function or a closure, writes (ADR 0054): which
    /// of its parameters is its `Formatter`, given it returns a `fmt::Result`.
    pub fn formatter_param(&self, def_id: DefId) -> Option<usize> {
        if self.tcx.def_kind(def_id) == DefKind::Closure {
            let ty = self.tcx.type_of(def_id).instantiate_identity().skip_normalization();
            let ty::Closure(_, args) = ty.kind() else { return None };
            let sig = args.as_closure().sig().skip_binder();
            return self.writes_to(sig.inputs()[0].tuple_fields().as_slice(), sig.output());
        }
        let sig = self.tcx.fn_sig(def_id).skip_binder().skip_binder();
        self.writes_to(sig.inputs(), sig.output())
    }

    /// Of a call of a callable, `Fn::call(&g, (s, f))` given its result's
    /// type: which of its arguments is the `Formatter` it writes to, where
    /// it's a writer, as a writer function's is (ADR 0054).
    pub fn callable_formatter(&self, def_id: DefId, args: ty::GenericArgsRef<'tcx>, output: Ty<'tcx>) -> Option<usize> {
        let fn_trait = self.tcx.trait_of_assoc(def_id)?;
        self.tcx.fn_trait_kind_from_def_id(fn_trait)?;
        let ty::Tuple(given) = args.type_at(1).kind() else {
            return None;
        };
        self.writes_to(given.as_slice(), output)
    }

    /// A call through a `fn` pointer, `callee`, that writes to a
    /// `Formatter` it's given, `cb(x, f)` of a `cb: fn(&u32, &mut
    /// Formatter) -> fmt::Result`: which argument it is (ADR 0538).
    pub fn pointer_formatter(&self, callee: Ty<'tcx>) -> Option<usize> {
        let ty::FnPtr(..) = callee.kind() else { return None };
        let sig = self.tcx.instantiate_bound_regions_with_erased(callee.fn_sig(self.tcx));
        self.writes_to(sig.inputs(), sig.output())
    }

    /// Which of `inputs` is the `Formatter` a writer writes to, of one
    /// that returns `output`, a `fmt::Result`.
    fn writes_to(&self, inputs: &[Ty<'tcx>], output: Ty<'tcx>) -> Option<usize> {
        if !self.is_fmt_result(output) {
            return None;
        }
        inputs.iter().position(|&t| self.is_formatter(t))
    }

    pub fn write_call(&self, def_id: DefId, known_function: bool) -> WriteCall {
        let tcx = self.tcx;
        let trait_id = tcx.trait_of_assoc(def_id);
        let is_trait = |name: &str| trait_id.is_some_and(|t| tcx.is_diagnostic_item(Symbol::intern(name), t));
        let owner = tcx
            .inherent_impl_of_assoc(def_id)
            .map(|imp| tcx.type_of(imp).instantiate_identity().skip_normalization());
        let on_formatter = owner.is_some_and(|t| self.is_std_type(t, StdItem::Formatter));
        match tcx.item_name(def_id).as_str() {
            "write_fmt" | "write_str" | "write_char" if on_formatter || is_trait("FmtWrite") => WriteCall::Text,
            "pad" if on_formatter => WriteCall::Pad,
            "pad_integral" if on_formatter => WriteCall::PadIntegral,
            "fmt" if is_trait("Display") => WriteCall::Display,
            "fmt" if is_trait("Debug") => WriteCall::Debug,
            "fmt" if trait_id.is_some_and(|t| is_other_fmt_trait(tcx, t)) => WriteCall::OtherFmt,
            "debug_struct_fields_finish" if on_formatter => WriteCall::StructFields,
            "debug_tuple_fields_finish" if on_formatter => WriteCall::TupleFields,
            name if on_formatter && name.starts_with("debug_struct_field") && name.ends_with("_finish") => {
                WriteCall::Struct
            }
            name if on_formatter && name.starts_with("debug_tuple_field") && name.ends_with("_finish") => {
                WriteCall::Tuple
            }
            _ if known_function && trait_id.is_none() => WriteCall::Function,
            // Of the crate's trait, or of one std keeps private whose impls
            // `@rust-js/std` compiles with what calls it, `Zip`'s `ZipFmt`,
            // its impl picked by its sides' facts (#2252).
            _ if trait_id
                .is_some_and(|t| t.is_local() || (std_crate(tcx, t.krate).is_some() && std_private(tcx, t))) =>
            {
                WriteCall::Trait
            }
            _ => WriteCall::Unsupported,
        }
    }

    pub(crate) fn skip_predicate(&self, function: DefId, ty: Ty<'tcx>) -> Option<SkipPredicate> {
        let standard = matches!(
            std_crate(self.tcx, function.krate),
            Some(StdCrate::Core | StdCrate::Alloc)
        );
        match self.tcx.item_name(function).as_str() {
            "is_none" if standard && self.option_of(ty).is_some() => Some(SkipPredicate::None),
            "is_some" if standard && self.option_of(ty).is_some() => Some(SkipPredicate::Some),
            "is_empty" if standard && (self.is_vec_like(ty.peel_refs()) || self.is_string_like(ty)) => {
                Some(SkipPredicate::Empty)
            }
            _ => None,
        }
    }

    pub(crate) fn skips_none(&self, function: DefId) -> bool {
        of_std_crate(self.tcx, function, StdCrate::Core) && self.tcx.item_name(function).as_str() == "is_none"
    }
}

/// The `fmt` traits of a placeholder's other than `{}` and `{:?}`: `{:x}`'s
/// `LowerHex` and the like, and `{:p}`'s `Pointer`, most with no diagnostic
/// item (ADR 0165).
const OTHER_FMT_TRAITS: [&str; 7] = [
    "LowerHex", "UpperHex", "Octal", "Binary", "LowerExp", "UpperExp", "Pointer",
];

/// Is `id` a formatting trait's `fmt`, `Display`'s, `Debug`'s or one of
/// `OTHER_FMT_TRAITS`'? Its dictionary's entry isn't the method: it takes
/// the value and gives its text, no `Formatter` (#1477).
pub fn is_fmt_method(tcx: TyCtxt<'_>, id: DefId) -> bool {
    tcx.item_name(id).as_str() == "fmt" && tcx.trait_of_assoc(id).is_some_and(|tr| is_fmt_trait(tcx, tr))
}

/// Is `id` a formatting trait: `Display`, `Debug` or one of core's
/// `OTHER_FMT_TRAITS`?
pub(crate) fn is_fmt_trait(tcx: TyCtxt<'_>, id: DefId) -> bool {
    [sym::Display, sym::Debug]
        .iter()
        .any(|&item| tcx.is_diagnostic_item(item, id))
        || is_other_fmt_trait(tcx, id)
}

/// The type the formatting impl `imp` is of, whose writer its writer is:
/// its self type, through the references, `Box`es and `Pin`s around it,
/// which the orphan rules let a crate write a foreign trait's impl for
/// where what they hold is its own, `impl Display for &ByRef` and `impl
/// Display for Pin<Box<Pinned>>` of `ByRef` and `Pinned` (#1798). Of a
/// wrapper of a type parameter, std's `impl<T: Display> Display for
/// Box<T>`, the wrapper; of `&T`, none.
pub(crate) fn formatted_adt(tcx: TyCtxt<'_>, imp: DefId) -> Option<DefId> {
    formatted_adt_of(tcx.type_of(imp).instantiate_identity().skip_normalization())
}

/// `formatted_adt`, of `ty`, a formatting impl's self type: as it's
/// written, or a type whose impl is its own (`Recognition::formats_itself`),
/// `&dep::ByRef`'s, whose wrappers are none of std's that pass it on.
pub(super) fn formatted_adt_of(mut ty: Ty<'_>) -> Option<DefId> {
    loop {
        let inner = match *ty.kind() {
            ty::Ref(_, inner, _) => inner,
            ty::Adt(adt, args) if adt.is_fundamental() => args.types().next()?,
            _ => break,
        };
        if !matches!(inner.kind(), ty::Ref(..) | ty::Adt(..)) {
            break;
        }
        ty = inner;
    }
    ty.ty_adt_def().map(|adt| adt.did())
}

/// Is `id` std's formatting, an item of `std::fmt`: a `Formatter`'s
/// method, a builder's, `Arguments`', `rt::Argument`'s, `format`, a
/// formatting trait's method (#1496).
pub(crate) fn is_std_formatting(tcx: TyCtxt<'_>, id: DefId) -> bool {
    std_path_under(tcx, id, StdPath::FmtMod)
}

/// Is `id` a `fmt::rt::Argument`'s constructor, which a `format_args!`
/// makes of each value it shows, the value's type its own (#1496).
pub(crate) fn is_fmt_argument(tcx: TyCtxt<'_>, id: DefId) -> bool {
    is_std_path(tcx, id, StdPath::RtArgument)
        || matches!(
            std_inherent_method(tcx, id),
            Some((MethodOf::Type(StdPath::RtArgument), _))
        )
}

/// Is `id` one of core's `OTHER_FMT_TRAITS`?
pub(super) fn is_other_fmt_trait(tcx: TyCtxt<'_>, id: DefId) -> bool {
    of_std_crate(tcx, id, StdCrate::Core)
        && tcx.def_kind(id) == DefKind::Trait
        && OTHER_FMT_TRAITS.contains(&tcx.item_name(id).as_str())
        && std_path_under(tcx, id, StdPath::FmtMod)
}

impl<'a, 'tcx> Recognition<'a, 'tcx> {
    /// Of the call's own types, as `json::stringify_with` of an `Unknown`
    /// may throw where one of a `JsonText` can't (ADR 0310).
    /// `Display::fmt`, whose impl says how `format!` shows a value.
    pub(crate) fn display_fmt(&self) -> Option<DefId> {
        let display = self.tcx.get_diagnostic_item(sym::Display)?;
        self.tcx.associated_item_def_ids(display).first().copied()
    }
}

/// The type parameters whose names `{:?}` of `ty` shows, each a
/// `PhantomData<K>`'s, `PhantomData<{type_name::<K>()}>` (ADR 0452), in it
/// or a struct of the crate's it holds, through what points at it.
pub(crate) fn phantom_named<'tcx>(tcx: TyCtxt<'tcx>, ty: Ty<'tcx>) -> Vec<u32> {
    let mut found = Vec::new();
    phantom_named_in(tcx, pointer_target(tcx, ty), &mut Vec::new(), &mut found);
    found
}

/// `phantom_named` of a bound's type, where it's `Debug`'s: what `{:?}`
/// given that bound shows.
pub(crate) fn debug_named<'tcx>(tcx: TyCtxt<'tcx>, bound: ty::TraitRef<'tcx>) -> Vec<u32> {
    match tcx.is_diagnostic_item(Symbol::intern("Debug"), bound.def_id) {
        true => phantom_named(tcx, bound.self_ty()),
        false => Vec::new(),
    }
}

fn phantom_named_in<'tcx>(tcx: TyCtxt<'tcx>, ty: Ty<'tcx>, seen: &mut Vec<Ty<'tcx>>, found: &mut Vec<u32>) {
    let ty = ty.peel_refs();
    if seen.contains(&ty) {
        return;
    }
    seen.push(ty);
    match ty.kind() {
        ty::Adt(adt, args) if adt.is_phantom_data() => {
            if let ty::Param(param) = args.type_at(0).kind() {
                found.push(param.index);
            }
        }
        ty::Adt(adt, args) if adt.did().is_local() => {
            for field in adt.all_fields() {
                phantom_named_in(tcx, field.ty(tcx, args).skip_normalization(), seen, found);
            }
        }
        ty::Adt(_, args) => args.types().for_each(|t| phantom_named_in(tcx, t, seen, found)),
        ty::Tuple(items) => items.iter().for_each(|t| phantom_named_in(tcx, t, seen, found)),
        ty::Array(item, _) | ty::Slice(item) => phantom_named_in(tcx, *item, seen, found),
        _ => {}
    }
}

/// Does `ty` point at a `dyn Debug`, as a derived `Debug` gives each field?
pub(crate) fn points_at_dyn_debug<'tcx>(tcx: TyCtxt<'tcx>, ty: Ty<'tcx>) -> bool {
    matches!(pointer_target(tcx, ty).kind(), ty::Dynamic(traits, ..)
        if traits.principal_def_id().is_some_and(|id| tcx.is_diagnostic_item(Symbol::intern("Debug"), id)))
}

/// Is `ty` a `fmt::Result` (ADR 0054)?
/// A write's result, `fmt::Result` or `io::Result<()>`: `undefined` where
/// it's `Ok` (ADRs 0054, 0132), its error thrown where it's an `Err` (ADRs
/// 0187, 0462), so `?` of one never breaks.
pub(crate) fn is_write_result_type<'tcx>(tcx: TyCtxt<'tcx>, ty: Ty<'tcx>) -> bool {
    is_fmt_result_type(tcx, ty) || is_io_unit_result_type(tcx, ty)
}

/// `Recognition::stream`, of `tcx` alone: a standard stream's `Some(true)`
/// of standard error (ADR 0132). Its JS value is `undefined`.
fn stream_of<'tcx>(tcx: TyCtxt<'tcx>, ty: Ty<'tcx>) -> Option<bool> {
    let ty::Adt(adt, _) = ty.kind() else { return None };
    match std_path_item(tcx, adt.did()) {
        Some(StdPath::IoStdout | StdPath::IoStdoutLock) => Some(false),
        Some(StdPath::IoStderr | StdPath::IoStderrLock) => Some(true),
        _ => None,
    }
}

/// Whether a value of std's `ty` may be `undefined` in JS, as a `()` is:
/// a standard stream or its lock, which is nothing (ADR 0132), a disabled
/// `Backtrace` (ADR 0568), or a write's result, whose `Ok` is (ADRs 0187,
/// 0462); so an `Option`'s `Some` of one is boxed (#1010).
pub(crate) fn may_be_undefined_std<'tcx>(tcx: TyCtxt<'tcx>, ty: Ty<'tcx>) -> bool {
    stream_of(tcx, ty).is_some() || is_backtrace_type(tcx, ty) || is_write_result_type(tcx, ty)
}

/// `io::Result<()>`, a write's result.
fn is_io_unit_result_type<'tcx>(tcx: TyCtxt<'tcx>, ty: Ty<'tcx>) -> bool {
    matches!(ty.kind(), ty::Adt(result, args) if is_std_def(tcx, result.did(), StdItem::Result)
        && args.type_at(0).is_unit()
        && matches!(args.type_at(1).kind(), ty::Adt(error, _) if is_std_path(tcx, error.did(), StdPath::IoError)))
}

pub(crate) fn is_fmt_result_type<'tcx>(tcx: TyCtxt<'tcx>, ty: Ty<'tcx>) -> bool {
    fmt_result(tcx).is_some_and(|result| tcx.erase_and_anonymize_regions(ty) == tcx.erase_and_anonymize_regions(result))
}

/// Is a result of `declared` type, a generic one, `Result<(), S::Error>` or
/// `Result<Self::Output, Self::Err>`, a `fmt::Result` once `args` are given
/// and its projections normalized, in `typing_env`? Generic code has it as
/// a `Result`, `{ TAG }`, where a `fmt::Result` is `undefined` or its error
/// thrown (ADRs 0187, 0461; #2057).
pub(crate) fn is_generic_fmt_result<'tcx>(
    tcx: TyCtxt<'tcx>,
    typing_env: ty::TypingEnv<'tcx>,
    declared: Ty<'tcx>,
    args: ty::GenericArgsRef<'tcx>,
) -> bool {
    let Some(result) = fmt_result(tcx) else {
        return false;
    };
    let (ty::Adt(result_adt, _), ty::Adt(adt, _)) = (*result.kind(), *declared.kind()) else {
        return false;
    };
    if adt != result_adt || !ty::TypeVisitableExt::has_param(&declared) {
        return false;
    }
    let given = ty::EarlyBinder::bind(tcx, declared)
        .instantiate(tcx, args)
        .skip_normalization();
    let given = (tcx.try_normalize_erasing_regions(typing_env, ty::Unnormalized::new_wip(given))).unwrap_or(given);
    tcx.erase_and_anonymize_regions(given) == tcx.erase_and_anonymize_regions(result)
}

/// `fmt::Result`, what `Display::fmt` gives, if std's `Display` is loaded.
fn fmt_result(tcx: TyCtxt<'_>) -> Option<Ty<'_>> {
    let display = tcx.get_diagnostic_item(Symbol::intern("Display"))?;
    let fmt = tcx.associated_item_def_ids(display)[0];
    Some(tcx.fn_sig(fmt).skip_binder().skip_binder().output())
}

/// The types whose `fmt` formatting a value of `ty` may call, as std's
/// `Vec<T>`'s calls `T`'s: `ty` and each it holds, never one only a binder
/// inside holds, `&'a u32` of a `for<'a> fn(&'a u32)`, which is no value's
/// type, and of which rustc's trait selection can't be asked (#1111).
pub(crate) fn formatted_parts<'tcx>(ty: Ty<'tcx>) -> impl Iterator<Item = Ty<'tcx>> {
    ty.walk()
        .filter_map(|part| part.as_type())
        .filter(|part| !rustc_middle::ty::TypeVisitableExt::has_escaping_bound_vars(part))
}

/// The fmt trait whose `fmt` of its type argument `def_id` calls: a
/// placeholder's `Argument::new_display` its `Display`, `new_lower_hex` its
/// `LowerHex`, and `to_string` its `Display` (ADR 0187).
pub(crate) fn fmt_trait_called(tcx: TyCtxt<'_>, def_id: DefId) -> Option<DefId> {
    let named = |name: &str| match name {
        "Display" | "Debug" => tcx.get_diagnostic_item(Symbol::intern(name)),
        _ => tcx
            .all_traits_including_private()
            .find(|&id| is_other_fmt_trait(tcx, id) && tcx.item_name(id).as_str() == name),
    };
    if tcx.is_diagnostic_item(Symbol::intern("to_string_method"), def_id) {
        return named("Display");
    }
    let owner = tcx.inherent_impl_of_assoc(def_id)?;
    let ty::Adt(adt, _) = tcx.type_of(owner).instantiate_identity().skip_normalization().kind() else {
        return None;
    };
    if !tcx.is_lang_item(adt.did(), LangItem::FormatArgument) {
        return None;
    }
    named(match tcx.item_name(def_id).as_str() {
        "new_display" => "Display",
        "new_debug" => "Debug",
        "new_lower_hex" => "LowerHex",
        "new_upper_hex" => "UpperHex",
        "new_binary" => "Binary",
        "new_octal" => "Octal",
        "new_lower_exp" => "LowerExp",
        "new_upper_exp" => "UpperExp",
        "new_pointer" => "Pointer",
        _ => return None,
    })
}

/// What a writer asks its `Formatter` (ADRs 0137, 0143).
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum FormatterQuery {
    Alternate,
    Width,
    Precision,
    Fill,
    Align,
    SignPlus,
    SignAwareZeroPad,
}

/// What a `fmt::Result`'s method gives (ADR 0148), of an `Ok` and, where
/// it may be one, of an `Err` (ADR 0187).
#[derive(Clone, Copy)]
pub enum FmtResultAnswer {
    /// `unwrap()`: `()`, or a panic.
    Unwrap,
    /// `expect(..)`: `()`, or a panic with its message.
    Expect,
    /// `is_ok()`, `is_err()`.
    Is(bool),
    /// `ok()`: `Some(())`, or `None` (ADR 0392).
    Ok,
    /// `map_err(f)`: `Ok(())`, or `Err(f(error))`, a `Result` of its own,
    /// arrayvec's `write_fmt(f).map_err(CapacityError::new)`.
    MapErr,
    /// `unwrap_or(())`, `unwrap_or_default()`: `()` either way.
    UnwrapOr,
    /// `map(f)`: `Ok(f(()))`, or the `Err`, writeable's
    /// `write_to(sink).map(Ok)`.
    Map,
    /// `unwrap_err()`, `expect_err(..)`: the error, or a panic, `: ()` after
    /// its message, chrono's `write!(..).expect_err(..)`.
    UnwrapErr,
}

/// The answer of the `fmt::Result` method `def_id`, if it's one of these.
pub fn fmt_result_answer(tcx: TyCtxt<'_>, def_id: DefId) -> Option<FmtResultAnswer> {
    Some(match tcx.item_name(def_id).as_str() {
        "unwrap" => FmtResultAnswer::Unwrap,
        "expect" => FmtResultAnswer::Expect,
        "is_ok" => FmtResultAnswer::Is(true),
        "is_err" => FmtResultAnswer::Is(false),
        "ok" => FmtResultAnswer::Ok,
        "map_err" => FmtResultAnswer::MapErr,
        "map" => FmtResultAnswer::Map,
        "unwrap_or" | "unwrap_or_default" => FmtResultAnswer::UnwrapOr,
        "unwrap_err" | "expect_err" => FmtResultAnswer::UnwrapErr,
        _ => return None,
    })
}

/// A `Debug` builder of std's, by its type, which a `Formatter`'s
/// `debug_struct(..)` and the like make (ADR 0136).
#[derive(Clone, Copy, PartialEq)]
pub enum DebugBuilder {
    Struct,
    Tuple,
    List,
    Set,
    Map,
}

/// A step of a `Debug` builder, by its method: what it's given, then what
/// it writes.
#[derive(Clone, Copy, PartialEq)]
pub enum BuilderStep {
    Field,
    Entry,
    Entries,
    Finish { non_exhaustive: bool },
}

/// The `Debug` builder `did`, a type, is: std's `DebugStruct` and the like.
pub fn debug_builder(tcx: TyCtxt<'_>, did: DefId) -> Option<DebugBuilder> {
    if !std_path_under(tcx, did, StdPath::FmtMod) {
        return None;
    }
    Some(match tcx.item_name(did).as_str() {
        "DebugStruct" => DebugBuilder::Struct,
        "DebugTuple" => DebugBuilder::Tuple,
        "DebugList" => DebugBuilder::List,
        "DebugSet" => DebugBuilder::Set,
        "DebugMap" => DebugBuilder::Map,
        _ => return None,
    })
}

/// The step of a `Debug` builder `def_id`, a method, is.
pub fn builder_step(tcx: TyCtxt<'_>, def_id: DefId) -> Option<BuilderStep> {
    let owner = tcx
        .type_of(tcx.inherent_impl_of_assoc(def_id)?)
        .instantiate_identity()
        .skip_normalization();
    let ty::Adt(builder, _) = owner.kind() else {
        return None;
    };
    debug_builder(tcx, builder.did())?;
    Some(match tcx.item_name(def_id).as_str() {
        "field" => BuilderStep::Field,
        "entry" => BuilderStep::Entry,
        "entries" => BuilderStep::Entries,
        "finish" => BuilderStep::Finish { non_exhaustive: false },
        "finish_non_exhaustive" => BuilderStep::Finish { non_exhaustive: true },
        _ => return None,
    })
}

/// Whether `def_id`, a `Formatter`'s method, makes a `Debug` builder:
/// `debug_struct(..)` and the like, not what writes one whole,
/// `debug_struct_field1_finish`.
pub fn makes_debug_builder(tcx: TyCtxt<'_>, def_id: DefId) -> bool {
    let name = tcx.item_name(def_id);
    name.as_str().starts_with("debug_") && !name.as_str().ends_with("_finish")
}

/// Whether `def_id`, a `Formatter`'s method, writes through a `Debug`
/// builder: one that makes one, `debug_list()`, or a derived `Debug`'s,
/// which writes one whole, `debug_struct_field2_finish`.
pub(crate) fn writes_debug_builder(tcx: TyCtxt<'_>, def_id: DefId) -> bool {
    is_formatter_method(tcx, def_id) && tcx.item_name(def_id).as_str().starts_with("debug_")
}

/// Whether `def_id` is a method of std's `Formatter`, its own.
fn is_formatter_method(tcx: TyCtxt<'_>, def_id: DefId) -> bool {
    tcx.inherent_impl_of_assoc(def_id).is_some_and(|imp| {
        matches!(tcx.type_of(imp).skip_binder().kind(),
            ty::Adt(adt, _) if is_std_def(tcx, adt.did(), StdItem::Formatter))
    })
}

/// The question `def_id` asks of a `Formatter`: `f.width()` and the like.
pub fn formatter_query(tcx: TyCtxt<'_>, def_id: DefId) -> Option<FormatterQuery> {
    if !is_formatter_method(tcx, def_id) {
        return None;
    }
    Some(match tcx.item_name(def_id).as_str() {
        "alternate" => FormatterQuery::Alternate,
        "width" => FormatterQuery::Width,
        "precision" => FormatterQuery::Precision,
        "fill" => FormatterQuery::Fill,
        "align" => FormatterQuery::Align,
        "sign_plus" => FormatterQuery::SignPlus,
        "sign_aware_zero_pad" => FormatterQuery::SignAwareZeroPad,
        _ => return None,
    })
}

/// `Error`'s deprecated `cause`, `Some(true)`, std's its `source`, or
/// `description`, `Some(false)`, std's a constant text, which generic code
/// calls through its dictionary (ADR 0141).
pub(crate) fn error_default(tcx: TyCtxt<'_>, id: DefId) -> Option<bool> {
    match () {
        _ if is_std_method(tcx, id, StdItem::Error, "cause") => Some(true),
        _ if is_std_method(tcx, id, StdItem::Error, "description") => Some(false),
        _ => None,
    }
}

/// `io::Write`'s `write_all`, `Some(true)`, or `write_fmt`, `Some(false)`,
/// which std's writes with `write` (ADR 0392).
pub(crate) fn io_write_default(tcx: TyCtxt<'_>, id: DefId) -> Option<bool> {
    match () {
        _ if is_std_method(tcx, id, StdItem::IoWrite, "write_all") => Some(true),
        _ if is_std_method(tcx, id, StdItem::IoWrite, "write_fmt") => Some(false),
        _ => None,
    }
}

/// `fmt::Write`'s `write_char` or `write_fmt`, which std's writes with
/// `write_str`: a `char`'s text, or a `write!`'s, which is a string in JS
/// (ADR 0166).
pub(crate) fn is_writer_default(tcx: TyCtxt<'_>, id: DefId) -> bool {
    is_std_method(tcx, id, StdItem::FmtWrite, "write_char") || is_std_method(tcx, id, StdItem::FmtWrite, "write_fmt")
}

/// `fmt::Write::write_fmt`, the trait's method.
pub(crate) fn is_write_fmt(tcx: TyCtxt<'_>, id: DefId) -> bool {
    is_std_method(tcx, id, StdItem::FmtWrite, "write_fmt")
}

/// `fmt::Arguments::new` and its kin, what `format_args!` makes.
pub(crate) fn is_arguments_new(tcx: TyCtxt<'_>, id: DefId) -> bool {
    matches!(std_inherent_method(tcx, id), Some((MethodOf::Type(StdPath::FmtArguments), name)) if name.as_str() == "new")
}

/// `fmt::Arguments::from_str`, or `from_str_nonconst`, what `format_args!`
/// makes of a text alone, `println!("done")`'s.
pub(crate) fn is_arguments_from_str(tcx: TyCtxt<'_>, id: DefId) -> bool {
    matches!(std_inherent_method(tcx, id), Some((MethodOf::Type(StdPath::FmtArguments), name))
        if matches!(name.as_str(), "from_str" | "from_str_nonconst"))
}

/// An `fmt::Arguments::from_str(<text>)` rustc's MIR inliner put in a
/// block of an optimized body, another crate's (#1569): `panic!("..")`'s
/// and an `assert!`'s message, which 1.100 makes the text's pointer's bits,
/// `_p = copy _r as *const u8 (PtrToPtr); _t = copy _p as NonNull<u8>
/// (Transmute); .. _a = Arguments { template: move _t, args: move _n }`.
pub(crate) struct InlinedText<'tcx> {
    /// The block's statements of its inlined scope, in order.
    pub(crate) statements: Vec<usize>,
    /// Of those, the one that makes the `Arguments`, the last but the ends
    /// of its locals' storage.
    pub(crate) made_at: usize,
    /// The `Arguments` made.
    pub(crate) made: mir::Place<'tcx>,
    /// The text: what its inlined `len()` reads the bytes of, a `&str`
    /// made a `&[u8]`.
    pub(crate) text: mir::Operand<'tcx>,
    /// `from_str` and its generic arguments, as it was inlined.
    pub(crate) from_str: (DefId, ty::GenericArgsRef<'tcx>),
    /// Where it was called: the scope it was inlined into and the call's
    /// span.
    pub(crate) called_at: mir::SourceInfo,
}

/// The first `fmt::Arguments::from_str` inlined in `block` of `body`, as
/// `InlinedText` says, exactly: each statement of its scope in the block
/// up to the one that makes its `Arguments`, none in another block or
/// past it, one text, and each local its statements write but the
/// `Arguments` read by them alone. `None` otherwise, its form left as
/// rustc's inliner made it.
pub(crate) fn inlined_text<'tcx>(
    tcx: TyCtxt<'tcx>,
    body: &mir::Body<'tcx>,
    block: mir::BasicBlock,
) -> Option<InlinedText<'tcx>> {
    use rustc_middle::mir::visit::{PlaceContext, Visitor};
    // The inlined `from_str` a scope is in, and its scope.
    let from_str_of = |mut scope: mir::SourceScope| loop {
        let data = &body.source_scopes[scope];
        if let Some((instance, _)) = data.inlined
            && is_arguments_from_str(tcx, instance.def_id())
        {
            return Some((scope, instance));
        }
        scope = data.parent_scope?;
    };
    let statements = &body.basic_blocks[block].statements;
    let (scope, instance) = statements.iter().find_map(|s| from_str_of(s.source_info.scope))?;
    let of_it = |info: mir::SourceInfo| from_str_of(info.scope).is_some_and(|(s, _)| s == scope);
    let its: Vec<usize> = (0..statements.len())
        .filter(|&at| of_it(statements[at].source_info))
        .collect();
    // Its `Arguments`, made by its last statement in the block but the ends
    // of its locals' storage.
    let is_storage = |at: usize| {
        matches!(
            statements[at].kind,
            mir::StatementKind::StorageLive(_) | mir::StatementKind::StorageDead(_)
        )
    };
    let &made_at = its.iter().rev().find(|&&at| !is_storage(at))?;
    let (made, mir::Rvalue::Aggregate(kind, _)) = statements[made_at].kind.as_assign()? else {
        return None;
    };
    let mir::AggregateKind::Adt(adt, ..) = **kind else {
        return None;
    };
    if !is_std_path(tcx, adt, StdPath::FmtArguments) || made.as_local().is_none() {
        return None;
    }
    // Its one text.
    let texts: Vec<&mir::Operand<'tcx>> = (its.iter())
        .filter_map(|&at| match statements[at].kind.as_assign()? {
            (_, mir::Rvalue::Cast(mir::CastKind::Transmute, operand, _))
                if matches!(operand.ty(&body.local_decls, tcx).kind(),
                    ty::Ref(_, inner, _) if inner.is_str()) =>
            {
                Some(operand)
            }
            _ => None,
        })
        .collect();
    let [text] = texts[..] else {
        return None;
    };
    // Of no other block, nor any terminator, and what it writes but its
    // `Arguments`, the text's local aside, read by it alone.
    let mut written: Vec<mir::Local> = (its.iter())
        .filter_map(|&at| statements[at].kind.as_assign())
        .map(|(place, _)| place.local)
        .collect();
    if text.place().is_some_and(|place| written.contains(&place.local)) {
        return None;
    }
    written.retain(|&local| Some(local) != made.as_local());
    struct ReadElsewhere<'b> {
        written: &'b [mir::Local],
        found: bool,
    }
    impl<'tcx> Visitor<'tcx> for ReadElsewhere<'_> {
        fn visit_local(&mut self, local: mir::Local, context: PlaceContext, _: mir::Location) {
            self.found |= self.written.contains(&local) && !context.is_storage_marker();
        }
    }
    let mut elsewhere = ReadElsewhere {
        written: &written,
        found: false,
    };
    for (b, data) in body.basic_blocks.iter_enumerated() {
        for (at, statement) in data.statements.iter().enumerate() {
            let its_own = b == block && its.contains(&at);
            if !its_own && of_it(statement.source_info) {
                return None;
            }
            if !its_own {
                let location = mir::Location {
                    block: b,
                    statement_index: at,
                };
                elsewhere.visit_statement(statement, location);
            }
        }
        if of_it(data.terminator().source_info) {
            return None;
        }
        elsewhere.visit_terminator(data.terminator(), body.terminator_loc(b));
    }
    if elsewhere.found {
        return None;
    }
    // And what's between its statements, the caller's, which goes before
    // the call, neither reads nor ends its text.
    let mut text_read = ReadElsewhere {
        written: &text.place().map(|place| vec![place.local]).unwrap_or_default(),
        found: false,
    };
    for at in (its[0]..made_at).filter(|at| !its.contains(at)) {
        let location = mir::Location {
            block,
            statement_index: at,
        };
        text_read.visit_statement(&statements[at], location);
        if text.place().is_some_and(
            |place| matches!(statements[at].kind, mir::StatementKind::StorageDead(local) if local == place.local),
        ) {
            return None;
        }
    }
    if text_read.found {
        return None;
    }
    let (_, call) = body.source_scopes[scope].inlined?;
    Some(InlinedText {
        statements: its,
        made_at,
        made: *made,
        text: text.clone(),
        from_str: (instance.def_id(), instance.args),
        called_at: mir::SourceInfo {
            span: call,
            scope: body.source_scopes[scope].parent_scope?,
        },
    })
}

/// `Argument::new_debug(&x)`, which `format_args!` makes of a `{:?}`
/// placeholder's argument (ADR 0137), as `classify` has it.
pub(crate) fn is_debug_argument(tcx: TyCtxt<'_>, id: DefId) -> bool {
    tcx.item_name(id).as_str() == "new_debug"
        && tcx.opt_parent(id).is_some_and(|parent| {
            matches!(tcx.def_kind(parent), DefKind::Impl { .. })
                && matches!(tcx.type_of(parent).skip_binder().kind(),
                    ty::Adt(adt, _) if tcx.is_lang_item(adt.did(), LangItem::FormatArgument))
        })
}

/// `f.pad(s)` of a `Formatter` (ADR 0143).
pub(crate) fn is_formatter_pad(tcx: TyCtxt<'_>, id: DefId) -> bool {
    tcx.item_name(id).as_str() == "pad" && is_formatter_method(tcx, id)
}

/// Whether `ty` is `[u8]`, what a `&mut [u8]` writer writes (#1282).
pub(crate) fn is_byte_slice(ty: Ty<'_>) -> bool {
    matches!(ty.kind(), ty::Slice(item) if matches!(item.kind(), ty::Uint(ty::UintTy::U8)))
}
