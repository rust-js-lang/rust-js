//! Formatting and writing: `fmt`'s traits, a `Formatter` and its builders,
//! and the writers `write!` writes to.

use super::*;

impl<'a, 'tcx> Recognition<'a, 'tcx> {
    /// Whether `ty` is standard output, or its lock, `Some(false)`, or
    /// standard error, `Some(true)` (ADR 0132).
    pub(in crate::lower) fn stream(&self, ty: Ty<'tcx>) -> Option<bool> {
        let ty::Adt(adt, _) = ty.kind() else { return None };
        match std_path(self.tcx, adt.did()).as_str() {
            "std::io::Stdout" | "std::io::StdoutLock" => Some(false),
            "std::io::Stderr" | "std::io::StderrLock" => Some(true),
            _ => None,
        }
    }

    /// The runtime's `io::Write` of `ty`, or of what a `&mut` to it is to,
    /// if std's is one rust-js knows (ADR 0392): a `Vec<u8>`'s, which adds
    /// to it, a standard stream's, which writes to it, or a `Box<dyn
    /// Write>`'s, which is the pair it holds and writes through it.
    pub(in crate::lower) fn io_writer(&self, ty: Ty<'tcx>) -> Option<&'static str> {
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
        let bytes = matches!(ty.kind(), ty::Adt(_, args) if self.is_std_type(ty, StdItem::Vec)
            && matches!(args.type_at(0).kind(), ty::Uint(ty::UintTy::U8)));
        bytes.then_some("$vecWriter")
    }

    /// A write's result, `fmt::Result` or `io::Result<()>` (`is_write_result_type`).
    pub(in crate::lower) fn is_write_result(&self, ty: Ty<'tcx>) -> bool {
        is_write_result_type(self.tcx, ty)
    }

    /// `io::Result<()>`, a write's result (`is_write_result_type`).
    pub(in crate::lower) fn is_io_unit_result(&self, ty: Ty<'tcx>) -> bool {
        is_io_unit_result_type(self.tcx, ty)
    }

    /// The `fmt` trait a placeholder of `kind` calls, `{:x}`'s `LowerHex`,
    /// where it's one of `OTHER_FMT_TRAITS`.
    pub(in crate::lower) fn other_fmt_trait(&self, kind: Std) -> Option<DefId> {
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
pub(crate) enum WriteCall {
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
pub(in crate::lower) enum SkipPredicate {
    None,
    Some,
    Empty,
}

impl<'a, 'tcx> Recognition<'a, 'tcx> {
    /// `&mut Formatter<'_>`.
    /// `fmt::Write`'s `write_str(f, s)` of a `Formatter`, called through the
    /// trait: its `&mut Self` is the `Formatter`, as `f.write_str(s)`'s is.
    pub(in crate::lower) fn fmt_write_on_formatter(&self, def_id: DefId, args: ty::GenericArgsRef<'tcx>) -> bool {
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
    pub(in crate::lower) fn formatter_write(&self, def_id: DefId, args: ty::GenericArgsRef<'tcx>) -> bool {
        let inherent = (self.tcx.inherent_impl_of_assoc(def_id)).is_some_and(|imp| {
            self.is_std_type(
                self.tcx.type_of(imp).instantiate_identity().skip_normalization(),
                StdItem::Formatter,
            )
        });
        (inherent && matches!(self.tcx.item_name(def_id).as_str(), "write_str" | "write_fmt"))
            || self.fmt_write_on_formatter(def_id, args)
    }

    /// `Display::fmt(&x, f)`, `Debug::fmt` or another `fmt` trait's, called
    /// as a function, on a `Formatter` held where it isn't its writer's
    /// (ADR 0461): what it writes.
    pub(in crate::lower) fn formatter_fmt_to(&self, def_id: DefId) -> Option<WriteCall> {
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
    pub(in crate::lower) fn is_formatter_type(&self, ty: Ty<'tcx>) -> bool {
        self.is_std_type(ty, StdItem::Formatter)
    }

    fn is_formatter(&self, ty: Ty<'tcx>) -> bool {
        matches!(ty.kind(), ty::Ref(_, inner, Mutability::Mut)
            if self.is_std_type(*inner, StdItem::Formatter))
    }
    pub(in crate::lower) fn display_trait(&self) -> DefId {
        self.tcx
            .get_diagnostic_item(Symbol::intern("Display"))
            .expect("std has `Display`")
    }
    pub(in crate::lower) fn is_fmt_result(&self, ty: Ty<'tcx>) -> bool {
        is_fmt_result_type(self.tcx, ty)
    }
    /// Is `ty` a `fmt::Error`, a `fmt::Result`'s `Err` (ADR 0187)?
    pub(in crate::lower) fn is_fmt_error(&self, ty: Ty<'tcx>) -> bool {
        matches!(fmt_result(self.tcx).map(|result| *result.kind()), Some(ty::Adt(_, args))
            if self.tcx.erase_and_anonymize_regions(ty) == self.tcx.erase_and_anonymize_regions(args.type_at(1)))
    }
    /// Whether `def_id`, a function or a closure, writes (ADR 0054): which
    /// of its parameters is its `Formatter`, given it returns a `fmt::Result`.
    pub(in crate::lower) fn formatter_param(&self, def_id: DefId) -> Option<usize> {
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
    pub(in crate::lower) fn callable_formatter(
        &self,
        def_id: DefId,
        args: ty::GenericArgsRef<'tcx>,
        output: Ty<'tcx>,
    ) -> Option<usize> {
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
    pub(in crate::lower) fn pointer_formatter(&self, callee: Ty<'tcx>) -> Option<usize> {
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

    pub(in crate::lower) fn write_call(&self, def_id: DefId, known_function: bool) -> WriteCall {
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
            _ if trait_id.is_some_and(|t| t.is_local()) => WriteCall::Trait,
            _ => WriteCall::Unsupported,
        }
    }

    pub(in crate::lower) fn skip_predicate(&self, function: DefId, ty: Ty<'tcx>) -> Option<SkipPredicate> {
        let standard = matches!(self.tcx.crate_name(function.krate).as_str(), "core" | "alloc");
        match self.tcx.item_name(function).as_str() {
            "is_none" if standard && self.option_of(ty).is_some() => Some(SkipPredicate::None),
            "is_some" if standard && self.option_of(ty).is_some() => Some(SkipPredicate::Some),
            "is_empty" if standard && (self.is_vec_like(ty.peel_refs()) || self.is_string_like(ty)) => {
                Some(SkipPredicate::Empty)
            }
            _ => None,
        }
    }

    pub(in crate::lower) fn skips_none(&self, function: DefId) -> bool {
        self.tcx.crate_name(function.krate).as_str() == "core" && self.tcx.item_name(function).as_str() == "is_none"
    }
}

/// The `fmt` traits of a placeholder's other than `{}` and `{:?}`: `{:x}`'s
/// `LowerHex` and the like, and `{:p}`'s `Pointer`, most with no diagnostic
/// item (ADR 0165).
const OTHER_FMT_TRAITS: [&str; 7] = [
    "LowerHex", "UpperHex", "Octal", "Binary", "LowerExp", "UpperExp", "Pointer",
];

/// Is `id` one of core's `OTHER_FMT_TRAITS`?
pub(crate) fn is_other_fmt_trait(tcx: TyCtxt<'_>, id: DefId) -> bool {
    tcx.crate_name(id.krate) == sym::core
        && tcx.def_kind(id) == DefKind::Trait
        && OTHER_FMT_TRAITS.contains(&tcx.item_name(id).as_str())
        && std_path(tcx, id).contains("fmt::")
}

impl<'a, 'tcx> Recognition<'a, 'tcx> {
    /// Of the call's own types, as `json::stringify_with` of an `Unknown`
    /// may throw where one of a `JsonText` can't (ADR 0310).
    /// `Display::fmt`, whose impl says how `format!` shows a value.
    pub(in crate::lower) fn display_fmt(&self) -> Option<DefId> {
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

/// `io::Result<()>`, a write's result.
pub(crate) fn is_io_unit_result_type<'tcx>(tcx: TyCtxt<'tcx>, ty: Ty<'tcx>) -> bool {
    matches!(ty.kind(), ty::Adt(result, args) if is_std_def(tcx, result.did(), StdItem::Result)
        && args.type_at(0).is_unit()
        && matches!(args.type_at(1).kind(), ty::Adt(error, _) if std_path(tcx, error.did()) == "std::io::Error"))
}

pub(crate) fn is_fmt_result_type<'tcx>(tcx: TyCtxt<'tcx>, ty: Ty<'tcx>) -> bool {
    fmt_result(tcx).is_some_and(|result| tcx.erase_and_anonymize_regions(ty) == tcx.erase_and_anonymize_regions(result))
}

/// Is a result of `declared` type, a generic one, `Result<(), S::Error>`, a
/// `fmt::Result` once `args` are given, in `typing_env`? Generic code has it
/// as a `Result`, `{ TAG }`, where a `fmt::Result` is `undefined` or its error
/// thrown (ADRs 0187, 0461).
pub(crate) fn is_generic_fmt_result<'tcx>(
    tcx: TyCtxt<'tcx>,
    typing_env: ty::TypingEnv<'tcx>,
    declared: Ty<'tcx>,
    args: ty::GenericArgsRef<'tcx>,
) -> bool {
    // A `Result<(), E>` of a generic `E`: its `E` given `args`, a `fmt::Error`.
    let Some(ty::Adt(result, result_args)) = fmt_result(tcx).map(|result| *result.kind()) else {
        return false;
    };
    let ty::Adt(adt, declared_args) = *declared.kind() else {
        return false;
    };
    if adt != result
        || !declared_args.type_at(0).is_unit()
        || !ty::TypeVisitableExt::has_param(&declared_args.type_at(1))
    {
        return false;
    }
    let error = ty::EarlyBinder::bind(tcx, declared_args.type_at(1))
        .instantiate(tcx, args)
        .skip_normalization();
    let error = (tcx.try_normalize_erasing_regions(typing_env, ty::Unnormalized::new_wip(error))).unwrap_or(error);
    tcx.erase_and_anonymize_regions(error) == tcx.erase_and_anonymize_regions(result_args.type_at(1))
}

/// `fmt::Result`, what `Display::fmt` gives, if std's `Display` is loaded.
fn fmt_result(tcx: TyCtxt<'_>) -> Option<Ty<'_>> {
    let display = tcx.get_diagnostic_item(Symbol::intern("Display"))?;
    let fmt = tcx.associated_item_def_ids(display)[0];
    Some(tcx.fn_sig(fmt).skip_binder().skip_binder().output())
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
pub(in crate::lower) enum FormatterQuery {
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
pub(in crate::lower) enum FmtResultAnswer {
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
}

/// The answer of the `fmt::Result` method `def_id`, if it's one of these.
pub(in crate::lower) fn fmt_result_answer(tcx: TyCtxt<'_>, def_id: DefId) -> Option<FmtResultAnswer> {
    Some(match tcx.item_name(def_id).as_str() {
        "unwrap" => FmtResultAnswer::Unwrap,
        "expect" => FmtResultAnswer::Expect,
        "is_ok" => FmtResultAnswer::Is(true),
        "is_err" => FmtResultAnswer::Is(false),
        "ok" => FmtResultAnswer::Ok,
        "map_err" => FmtResultAnswer::MapErr,
        "map" => FmtResultAnswer::Map,
        "unwrap_or" | "unwrap_or_default" => FmtResultAnswer::UnwrapOr,
        _ => return None,
    })
}

/// A `Debug` builder of std's, by its type, which a `Formatter`'s
/// `debug_struct(..)` and the like make (ADR 0136).
#[derive(Clone, Copy, PartialEq)]
pub(crate) enum DebugBuilder {
    Struct,
    Tuple,
    List,
    Set,
    Map,
}

/// A step of a `Debug` builder, by its method: what it's given, then what
/// it writes.
#[derive(Clone, Copy, PartialEq)]
pub(crate) enum BuilderStep {
    Field,
    Entry,
    Entries,
    Finish { non_exhaustive: bool },
}

/// The `Debug` builder `did`, a type, is: std's `DebugStruct` and the like.
pub(in crate::lower) fn debug_builder(tcx: TyCtxt<'_>, did: DefId) -> Option<DebugBuilder> {
    if !std_path(tcx, did).starts_with("std::fmt::") {
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
pub(in crate::lower) fn builder_step(tcx: TyCtxt<'_>, def_id: DefId) -> Option<BuilderStep> {
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
pub(in crate::lower) fn makes_debug_builder(tcx: TyCtxt<'_>, def_id: DefId) -> bool {
    let name = tcx.item_name(def_id);
    name.as_str().starts_with("debug_") && !name.as_str().ends_with("_finish")
}

/// Whether `def_id` is a method of std's `Formatter`, its own.
fn is_formatter_method(tcx: TyCtxt<'_>, def_id: DefId) -> bool {
    tcx.inherent_impl_of_assoc(def_id).is_some_and(|imp| {
        matches!(tcx.type_of(imp).skip_binder().kind(),
            ty::Adt(adt, _) if is_std_def(tcx, adt.did(), StdItem::Formatter))
    })
}

/// The question `def_id` asks of a `Formatter`: `f.width()` and the like.
pub(in crate::lower) fn formatter_query(tcx: TyCtxt<'_>, def_id: DefId) -> Option<FormatterQuery> {
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

/// `fmt::Arguments::new` and its kin, what `format_args!` makes.
pub(crate) fn is_arguments_new(tcx: TyCtxt<'_>, id: DefId) -> bool {
    tcx.item_name(id).as_str() == "new" && std_path(tcx, id).starts_with("std::fmt::Arguments")
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
