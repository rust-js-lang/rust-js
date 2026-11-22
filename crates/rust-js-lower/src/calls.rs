//! A call and what calls share: `Call`, a call made, `apply`, calling a
//! function value with what's given, `catching`, a JS call that may throw
//! made a `Result` (ADR 0035), and `given_to_js`, a value made what JS
//! takes. Which call it is, MIR's lowering says
//! (`mir/calls.rs`, `mir/std_calls.rs`).

use super::recognition::Catching;

use super::{FnCx, R};
use rust_js_program::js;
use rust_js_program::js::{Expr, Known, StmtKind};

use rustc_middle::ty::{self, Ty};
use rustc_span::Span;
use rustc_span::def_id::DefId;

/// A call being lowered: what's called, which function that is, its
/// arguments, and whether its value is used.
#[derive(Clone, Copy)]
pub struct Call<'c, 'tcx> {
    pub(super) def_id: DefId,
    pub(super) generic_args: ty::GenericArgsRef<'tcx>,
    /// Each argument's type.
    pub(super) tys: &'c [Ty<'tcx>],
    /// Of an argument that's an `Option`, whether it's `null` or a value
    /// already, MIR's `nulled`, where it's asked: a binding's nullable
    /// parameter's, or what a `Cell` is written (ADR 0772); else empty.
    pub(super) nulled: &'c [bool],
    /// Of a call of a `RefCell`'s, `x.f.borrow()`, or of `drop(guard)` of a
    /// guard of one, whether it's a struct's field's no borrow of which can
    /// meet another, never checked (ADR 0863, amended).
    pub(super) quiet: bool,
    /// Of a borrow of a `RefCell`, whether its guard is held only while
    /// nothing can ask whether its cell is borrowed: checked, not counted
    /// (ADR 0328, rd218).
    pub(super) momentary: bool,
    /// Of a call that writes a thread-local's cell, how the thread-local
    /// holds a `None`: of a `#[rust_js::nullable]` one, `null` (ADR 0586,
    /// amended, `NoneForm`).
    pub(super) none_form: super::representation::NoneForm,
    /// Of one, whether nothing reads it apart, `null` from `undefined`, so
    /// what's given is stored as it is, its `None` `null`
    /// (`NulledParams::reads_alike`, #665).
    pub(super) read_alike: bool,
    /// Whether nothing reads what the call gives, a temporary only dropped,
    /// where its drop does nothing: `x.f.replace(v);`'s old value, which
    /// is then no read, as `*x.f.borrow_mut() = v`'s isn't (rd418, ADR
    /// 1022, amended).
    pub(super) unread: bool,
    /// Of a `RefCell`'s `swap`, which of its two cells is a class's
    /// flattened state, its object's keys (ADR 1044, #687).
    pub(super) flattened: [bool; 2],
    /// Whether the reference it's given was made as its place's pointer
    /// (`Pointed::lent_calls`, ADR 1114).
    pub(super) lent: bool,
    /// Of `MaybeUninit::uninit()` of an array, whether what it makes is only
    /// written whole and read whole (`uninit_written_whole`, #1258).
    pub(super) written_whole: bool,
    /// Of `io::Read::read_to_string`, whether the `String` it's given is
    /// provably fresh-empty (`fresh_string_given`, #1297).
    pub(super) fresh_string: bool,
    /// Whether MIR proves the call's indexes in bounds, a `Vec`'s, a
    /// range's or a map's key held (`call_bounds_proved`, #1672 M3).
    pub(super) in_bounds: bool,
    /// Of an `Option`'s `and_then` whose closure gives a slice's
    /// `position`, whether its `Option` is a found index, only tested and
    /// its index taken: `-1` its `None` (`FoundIndexes::through`, #2074).
    pub(super) found_index: bool,
    /// What MIR proves of its first argument where it's read
    /// (`Given::receiver`).
    pub(super) receiver: crate::analysis::mir_reads::presence::Presence,
    /// The one `None` its first argument can be (`Given::receiver_none`).
    pub(super) receiver_none: Option<js::OnlyNone>,
    pub(super) span: Span,
}

/// What MIR says of a call's arguments that their values don't, as `Call`
/// carries it: `nulled`, `quiet`, `momentary`, `none_form`,
/// `read_alike`, `flattened`, `lent` and `fresh_string`; and of what it
/// gives, `unread` and `written_whole`.
#[derive(Clone, Copy, Default)]
pub struct Given<'c> {
    pub nulled: &'c [bool],
    pub quiet: bool,
    pub momentary: bool,
    pub none_form: super::representation::NoneForm,
    pub read_alike: bool,
    pub unread: bool,
    pub flattened: [bool; 2],
    pub lent: bool,
    pub written_whole: bool,
    pub fresh_string: bool,
    /// Whether MIR proves the call's indexes in bounds (`call_bounds_proved`).
    pub in_bounds: bool,
    /// Whether it's an `and_then` whose `Option` is a found index.
    pub found_index: bool,
    /// What MIR proves of its first argument where it's read, an `Option`
    /// receiver's (`mir::Presence`, #2112 C S4).
    pub receiver: crate::analysis::mir_reads::presence::Presence,
    /// The one `None` its first argument, an `Option`, can be where it's
    /// read, `NoneForms::read`'s (#2112 C S5).
    pub receiver_none: Option<js::OnlyNone>,
}

impl<'c, 'tcx> Call<'c, 'tcx> {
    /// The call of `def_id` with `generic_args`, of arguments of `tys`,
    /// MIR's `given`.
    pub fn given(
        (def_id, generic_args): (DefId, ty::GenericArgsRef<'tcx>),
        tys: &'c [Ty<'tcx>],
        given: Given<'c>,
        span: Span,
    ) -> Self {
        let Given {
            nulled,
            quiet,
            momentary,
            none_form,
            read_alike,
            unread,
            flattened,
            lent,
            written_whole,
            fresh_string,
            in_bounds,
            found_index,
            receiver,
            receiver_none,
        } = given;
        Call {
            def_id,
            generic_args,
            tys,
            nulled,
            quiet,
            momentary,
            none_form,
            read_alike,
            unread,
            flattened,
            lent,
            written_whole,
            fresh_string,
            in_bounds,
            found_index,
            receiver,
            receiver_none,
            span,
        }
    }
}

/// A call of `exprs`' first, a function value, of the rest, given to JS
/// (`given_to_js`): a function a variable holds, `js::import!(f)`'s.
pub fn called_given_to_js(mut exprs: Vec<Expr>) -> Expr {
    let callee = exprs.remove(0);
    given_to_js(&mut exprs);
    Expr::call(callee, exprs)
}

/// Arguments given to JS: a trailing `undefined` is one it's given,
/// `GivenUndefined`, which no pass leaves out, as what it calls may count
/// its arguments, `Math.max(1, undefined)` (ADR 0330).
pub fn given_to_js(args: &mut Vec<Expr>) {
    given_to_js_but(args, |_, _| false);
}

/// Arguments given to JS, as `given_to_js` gives them, but of those last
/// each `absent` says is no argument, a `None` of a parameter TypeScript's
/// `x?` leaves out or a `()` (`absent`), `Undefined` marked so,
/// `Known::Absent`, which the one rule, `undefined_arguments::left_out`,
/// leaves out, `cb()` of `cb(None)` (ADR 0313, amended).
pub fn given_to_js_but(args: &mut Vec<Expr>, absent: impl Fn(usize, &Expr) -> bool) {
    marked_absent(args, absent, true);
}

/// The crate's own call's arguments, of a function or a closure of its:
/// of those last, each `absent` says is no argument, `own()` of
/// `own(None)`, marked `Known::Absent`, which prepare leaves out, alone too
/// (ADRs 0443, 0441); any other `undefined` it leaves out only after
/// another argument (ADR 0313).
pub fn own_absent(args: &mut Vec<Expr>, absent: impl Fn(usize, &Expr) -> bool) {
    marked_absent(args, absent, false);
}

/// Whether `e` is `undefined`, given to JS or not.
pub(super) fn is_undefined(e: &Expr) -> bool {
    matches!(e.kind, js::ExprKind::Undefined | js::ExprKind::GivenUndefined)
}

fn marked_absent(args: &mut Vec<Expr>, absent: impl Fn(usize, &Expr) -> bool, to_js: bool) {
    let mut last = true;
    for (i, arg) in args.iter_mut().enumerate().rev() {
        last = last && absent(i, arg);
        if last {
            // A `()` that does nothing to make is `undefined`, a variable's too.
            *arg = Expr {
                kind: js::ExprKind::Undefined,
                span: arg.span,
                facts: arg.facts.with(js::Known::Absent),
                held_as: Default::default(),
            };
            continue;
        }
        if !is_undefined(arg) {
            break;
        }
        if to_js {
            arg.kind = js::ExprKind::GivenUndefined;
        }
    }
    // Left out here too, as prepare leaves them out, so what reads the call
    // before then sees the arguments its JS has.
    js::undefined_arguments::left_out(args);
}

/// Whether argument `i`, `arg`, of a call whose parameters are `tys`, is
/// no argument where it's last: a `()`, which is no value, as JS leaves one
/// out, `resolve()` of `resolve(())` (ADR 0441), one that does nothing to
/// make; and a `None` of an `Option`, `cb()` of `cb(None)`, as a person
/// leaves an optional one out (ADR 0443, amended, ADR 0262's second case),
/// but of a pointer that may be one of `bindings`, whose parameter there
/// isn't optional, which is given it, `max(1, undefined)` of `Math.max`'s
/// (ADR 0330). Only `arguments.length` tells.
pub fn absent(tcx: ty::TyCtxt<'_>, tys: &[Ty<'_>], bindings: &[DefId], i: usize, arg: &Expr) -> bool {
    let Some(ty) = tys.get(i) else {
        return false;
    };
    (ty.is_unit() && !arg.has_effects())
        || matches!(arg.kind, js::ExprKind::Undefined)
            && matches!(ty.kind(), ty::Adt(adt, _) if tcx.is_lang_item(adt.did(), rustc_hir::attrs::lang_items::LangItem::Option))
            && bindings.iter().all(|&f| super::bindings::is_optional_param(tcx, f, i))
}

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// A call that says, in Rust, that it may throw (ADR 0035): one
    /// returning a `Result` runs in a `try`, `$try(() => f(x))`, and one
    /// returning a `Promise<Result<..>>` settles either way, `$settle(p)`;
    /// a binding by its declared result, and a throwing function of the
    /// crate's, as `analysis::call_throws` says (ADR 0644).
    pub fn catching(&mut self, def_id: DefId, args: ty::GenericArgsRef<'tcx>, value: Expr) -> Expr {
        match super::analysis::call_throws(&self.recognition(), self.krate.throwing, def_id, args) {
            Catching::Result => Expr::tried(value),
            Catching::PromiseResult => Expr::call(Expr::helper("$settle"), vec![value]),
            Catching::Direct => value,
        }
    }

    /// Whether `def_id`, the crate's or a library's, takes JS's rest
    /// arguments: its last parameter a slice marked `#[rust_js::rest]`
    /// (ADR 0664), which its calls spread (`spread_rest`).
    pub fn takes_rest(&self, def_id: DefId) -> bool {
        super::bindings::takes_rest(self.tcx, def_id) || self.krate.foreign.takes_rest(def_id)
    }

    /// Whether the value of `def_id`, the crate's function, given `args`,
    /// is the arrow of its call, not the function: a class's constructor
    /// or member's, `(props) => new Component(props)` (ADR 0761), or one
    /// whose call gives an argument other than as it is
    /// (`converts_arguments`, #281).
    pub(super) fn value_is_its_call(&mut self, def_id: DefId, args: ty::GenericArgsRef<'tcx>, span: Span) -> R<bool> {
        use super::classes::{Member, member};
        Ok(matches!(
            member(self.tcx, def_id),
            Some(Member::Constructor | Member::Method | Member::Getter | Member::Own | Member::Namespace)
        ) || self.converts_arguments(def_id, args, span)?)
    }

    /// The function of rest arguments `def_id`, the crate's or a library's,
    /// given `args`, as JS has it, `total` of `function total(...xs)`, where
    /// what it's given to reads it as JS's function and gives no function
    /// of it back for Rust to call with a slice (`functions_themselves`):
    /// not the arrow its value is elsewhere, `(xs) => total(...xs)` (ADR
    /// 0664, amended, #540). Its own JS, the one its module names it by,
    /// whatever its type parameters are: what JS alone reads gives them
    /// nothing, so no copy of it is made for it (`instances::
    /// read_by_js_alone`, #565). `None` of any other, a class's member or
    /// one whose call converts what it's given, which keeps its arrow.
    pub fn rest_function_itself(
        &mut self,
        def_id: DefId,
        args: ty::GenericArgsRef<'tcx>,
        span: Span,
    ) -> R<Option<Expr>> {
        if !self.takes_rest(def_id)
            || self.tcx.trait_of_assoc(def_id).is_some()
            || self.value_is_its_call(def_id, args, span)?
        {
            return Ok(None);
        }
        // Named as its path in the call names it, `Dep.total` of
        // `&dep::total` (rd114, ADR 1018).
        let named = (self.krate.module_paths.iter())
            .find(|(at, through)| through.function == def_id && span.contains(**at))
            .map_or(span, |(at, _)| *at);
        Ok(Some(self.named_fn_ref(def_id, None, named)))
    }

    /// Whether `def_id`, the crate's function or a library's, reads
    /// `js::arguments()`: each call of it gives every argument the Rust
    /// does, `given_to_js`'s, and no closure that forwards to it is it, as
    /// its JS caller's arguments would be its (rd332).
    pub fn reads_arguments(&self, def_id: DefId) -> bool {
        self.krate.reads_arguments.functions.contains(&def_id) || self.krate.foreign.reads_arguments(def_id)
    }

    /// Whether `def_id`, the crate's function or a library's, reads
    /// `js::this()`: each call of it gives none, so one through a member,
    /// a module's namespace's, is `(0, M.f)()` (rd683).
    pub fn reads_this(&self, def_id: DefId) -> bool {
        self.krate.reads_arguments.this.contains(&def_id) || self.krate.foreign.reads_this(def_id)
    }

    /// Whether `def_id`, the crate's function or a library's, reads more
    /// of its call than its parameters, `js::arguments()` or `js::this()`,
    /// so no closure that forwards to it is it, which its JS caller would
    /// give them (rd332, rd683).
    pub(super) fn reads_its_call(&self, def_id: DefId) -> bool {
        self.krate.reads_arguments.reads_its_call(def_id) || self.krate.foreign.reads_its_call(def_id)
    }

    /// Refused, a call each owner says JS can't make as Rust does: of a
    /// string's bytes' pointer (`pointers.rs`, `pointers/into_copies.rs`),
    /// rest arguments as a value
    /// given as a type parameter's (`check_rest_unspread`), a counted
    /// `Rc` or a boxed `RefCell` given to a binding (`std_types/rc.rs`),
    /// and std's lending from inside its receiver where rust-js doesn't
    /// know it, in a crate that asks an address (`lent.rs`, #1114).
    pub fn check_call(
        &self,
        body: &rustc_middle::mir::Body<'tcx>,
        (def_id, generic_args): (DefId, ty::GenericArgsRef<'tcx>),
        args: &[rustc_span::Spanned<rustc_middle::mir::Operand<'tcx>>],
        destination: rustc_middle::mir::Place<'tcx>,
        span: Span,
    ) -> R<()> {
        self.check_text_pointer(body, (def_id, generic_args), destination, span)?;
        self.check_copy_pointer(body, (def_id, generic_args), args, destination, span)?;
        self.check_rest_unspread(def_id, generic_args, span)?;
        self.check_lent((def_id, generic_args), span)?;
        self.check_given_to_js(def_id, generic_args, span)
    }

    /// Refused, a call giving rest arguments as a value, `&dyn js::Rest<T>`,
    /// as a type parameter's value, `call::<&dyn Rest<T>, _>(f, args)`:
    /// what its body calls with one, `f(a)` of an `F: Fn(A)`, gives JS's
    /// rest parameter one array, where only a call through a type that says
    /// so spreads it (ADR 0851). A binding's `impl Rest<T>` takes them spread.
    fn check_rest_unspread(&self, def_id: DefId, generic_args: ty::GenericArgsRef<'tcx>, span: Span) -> R<()> {
        if !super::bindings::is_binding(self.tcx, def_id)
            && generic_args
                .types()
                .any(|ty| super::bindings::is_rest_value(self.tcx, ty))
        {
            let refused = super::refusal::Refused::Because(
                "rest arguments as a value, `&dyn js::Rest<T>`, as a type parameter's value",
                "a call of an `F: Fn(A)` would give them as one array",
            );
            return Err(super::refusal::refuse(self.tcx, span, refused));
        }
        Ok(())
    }

    /// A call of `def_id`'s arguments, its rest's spread where it takes
    /// one: `f(code, ...args)` (ADR 0664).
    pub fn rest_given(&self, def_id: DefId, args: &mut Vec<Expr>) {
        if self.takes_rest(def_id) {
            spread_rest(args);
        }
    }
}

/// A call's last argument given as JS's rest arguments (ADRs 0221, 0640,
/// 0664, 0667): an array written out, a slice's or a tuple's, its items,
/// `f(a, b)` of `&[a, b]`, `log("a", 1)` of `&("a", 1)`, and none of `&[]`;
/// any other value, an array held, spread, `f(...items)`. Of a call of a
/// function taking a rest, a variadic binding, and a `fn` pointer whose
/// last parameter is a `&dyn js::Rest<T>`.
pub fn spread_rest(args: &mut Vec<Expr>) {
    match args.pop() {
        Some(Expr {
            kind: js::ExprKind::Array(items, ..),
            ..
        }) => args.extend(items),
        Some(last) => args.push(Expr::spread(last)),
        None => {}
    }
}

/// `f(args)`, with a closure that only returns put in place, its parameters
/// the arguments: `((s) => s.value)(START)` is `START.value`. Only for
/// arguments that read the same however often they're read; of what
/// lowering knows, which can't tell what changes later, not into a
/// closure inside (`applied`).
pub fn apply(f: Expr, args: Vec<Expr>) -> Expr {
    applied(f, args, &|_| false)
}

/// `apply`'s one rule, asked with what's known never to change,
/// `unchanging`: tidy's, of the body as it is (`changing`). Where each
/// argument is a constant or a variable `unchanging` says nothing sets, a
/// closure inside that reads a parameter, `(e) => $position(DAYS, (d) =>
/// d === e)`, is put in place too, `$position(DAYS, (d) => d === e)`:
/// whenever it runs, it reads the value the parameter was given (the
/// vercel-ui SDK's port, G6; ADR 0705, amended). Else not, as one that
/// runs later would read what the variable holds then.
pub fn applied(f: Expr, args: Vec<Expr>, unchanging: &dyn Fn(&js::Name) -> bool) -> Expr {
    if let js::ExprKind::Arrow(params, body) = &f.kind
        && let [
            js::Stmt {
                kind: StmtKind::Return(Some(value)),
                ..
            },
        ] = body.as_slice()
        && let Some(inlined) = inlined(params, &args, value, unchanging)
    {
        return inlined;
    }
    Expr::call(f, args)
}

/// Each variable `pattern`, a parameter's, binds, and the part of `arg`
/// it is, however deep: `item` of `item`, `[, age]`'s `age` of `item`,
/// `item[1]`, `[[k], x]`'s `k`, `item[0][0]`, and `{ a: { b } }`'s `b`,
/// `item.a.b`. `None` of a pattern no part reads as it binds, a default's
/// or a rest's. The one reading of a pattern as its argument's parts, a
/// closure inlined where it's called and an `Option`'s `map` (#1900).
pub(super) fn pattern_parts<'p>(pattern: &'p js::Pattern, arg: &Expr) -> Option<Vec<(&'p js::Name, Expr)>> {
    match pattern {
        js::Pattern::Name(name) => Some(vec![(name, arg.clone())]),
        js::Pattern::Array(items) => {
            let mut parts = Vec::new();
            for (i, item) in items.iter().enumerate() {
                if let Some(item) = item {
                    parts.extend(pattern_parts(item, &Expr::index(arg.clone(), Expr::int(i as i128)))?);
                }
            }
            Some(parts)
        }
        js::Pattern::Object(fields, None) if fields.iter().all(|(_, _, default)| default.is_none()) => {
            let mut parts = Vec::new();
            for (key, field, _) in fields {
                let part = super::binding_calls::keyed(arg.clone(), Expr::str(key.clone()));
                parts.extend(pattern_parts(field, &part)?);
            }
            Some(parts)
        }
        _ => None,
    }
}

/// `value`, of a closure of `params`, with each the part of `args` it
/// binds, if each reads the same however often it's read.
fn inlined(
    params: &[js::Pattern],
    args: &[Expr],
    value: &Expr,
    unchanging: &dyn Fn(&js::Name) -> bool,
) -> Option<Expr> {
    if params.len() <= args.len() && args.iter().all(Expr::reads_same) {
        let fixed = |arg: &Expr| arg.is_constant() || matches!(&arg.kind, js::ExprKind::Var(v) if unchanging(v));
        let into_callbacks = args.iter().all(fixed);
        // Each variable a parameter binds, and the part of its argument it
        // is (`pattern_parts`).
        let parts: Option<Vec<(&js::Name, Expr)>> = (params.iter().zip(args))
            .map(|(p, arg)| pattern_parts(p, arg))
            .collect::<Option<Vec<_>>>()
            .map(|parts| parts.into_iter().flatten().collect());
        return parts.and_then(|parts| {
            let with = |name: &js::Name| parts.iter().find(|(n, _)| *n == name).map(|(_, part)| part.clone());
            match into_callbacks {
                true => value.substitute_in_callbacks(&with),
                false => value.substitute_names(&with),
            }
        });
    }
    // A thread-local made where it's first used, `x()` (ADR 0037, amended),
    // given a closure that reads it, `|l| l.get()`, is the path of the call
    // once tidy applies the arrow where it's made (`applied arrows`).
    None
}

/// Whether a call of `def_id` gives `null` for `None`, never `undefined`:
/// a function, a binding too, whose result is `#[rust_js::nullable]`, its
/// body writing it so (ADRs 0578, 0638). Not a trait's method, whose impl
/// the call may run, which says nothing of it (ADR 0841, amended).
pub fn gives_null_for_none(tcx: rustc_middle::ty::TyCtxt<'_>, def_id: rustc_span::def_id::DefId) -> bool {
    use super::representation::{NonePlace, none_form};
    none_form(tcx, NonePlace::FnResult(def_id)).stores_null() && tcx.trait_of_assoc(def_id).is_none()
}

/// What a call of `def_id` gives for `None` where it's only one of
/// `undefined` and `null`, as an `Option`'s fact (ADR 0298, amended): of a
/// binding whose result TypeScript types `T | null`,
/// `#[rust_js::nullable]`'s, `null`, and `T | undefined`,
/// `#[rust_js::optional]`'s, `undefined`. Not a trait's method's, as
/// `gives_null_for_none` isn't; nor the crate's own nullable function's,
/// which may give back as it is a nullable field's value that an object
/// JS made holds `undefined` in.
pub fn none_given_by(tcx: rustc_middle::ty::TyCtxt<'_>, def_id: rustc_span::def_id::DefId) -> Option<js::OnlyNone> {
    use super::representation::{NonePlace, Writers, none_form};
    none_form(tcx, NonePlace::CallResult(def_id)).trust(Writers::Exact)
}

/// What an `and_then` of `f` gives, of `then`, `f`'s call: `then`, as
/// `given_by` has it, where what it's of is `Some`, and its `None` where
/// that's `None`, `null` where `f` `gives_null_for_none`, as `f`'s own
/// `None` is, so the whole `Option` is `f`'s, React's `owner ?
/// getComponentNameFromOwner(owner) : null` (ADR 0841, amended); else
/// `undefined`, as a `None` is.
/// Of a closure that MIR says returns `null` or a value at each return,
/// `returns_nulled` (`returns_null_or_value`), `|i|
/// try_hydrate_activity(fiber, i)`'s, the same (ADR 0841, amended, #2112
/// S4d);
/// not a binding's, which TypeScript types `T | null`, whose `undefined`
/// where what it's of is `None` is JS's `labels?.item(0)` (ADR 0298,
/// amended).
pub(super) fn and_then_given<'tcx>(
    tcx: ty::TyCtxt<'tcx>,
    (f, returned, returns_nulled): (Ty<'tcx>, Option<rustc_span::def_id::DefId>, bool),
    then: Expr,
) -> (Expr, Expr) {
    // Not a binding's, `Node::first_child` or `|labels| labels.item(0)`,
    // whose `?.` reads as JS's, `node?.firstChild` (ADR 0298, amended): of a
    // closure, one it gives back the call of, MIR's (`closure_returned_callee`).
    let bindings_null =
        |def_id| super::bindings::is_binding(tcx, def_id) && none_given_by(tcx, def_id) == Some(js::OnlyNone::Null);
    let none = match f.kind() {
        ty::FnDef(def_id, _) if gives_null_for_none(tcx, *def_id) && !super::bindings::is_binding(tcx, *def_id) => {
            Expr::null()
        }
        ty::Closure(..) if returns_nulled && !returned.is_some_and(bindings_null) => Expr::null(),
        _ => Expr::js_undefined(),
    };
    (given_by(tcx, f, then), none)
}

/// `value`, what a call of `f` gave, marked never `undefined` where `f` is
/// a function item that `gives_null_for_none`: `and_then(name_of)`'s, which
/// a nullable place stores as it is, as `map`'s `Some` (ADR 0841, amended).
/// A binding getter's read too, React's `ReactSharedInternals.T`, which its
/// declaration says is `null` for `None` (ADR 0638, amended), whatever
/// else is known of it. A closure's value isn't asked.
/// And, of an `Option` it gives, what its `None` is where it's only one
/// (`none_given_by`), which its test says (ADR 0298, amended); of a `bool`,
/// that it's one. That an `f64` a `#[rust_js::whole]` binding gives is whole
/// is `whole_locals`' to say, of the local it's given to (ADR 0994, amended).
pub fn given_by<'tcx>(tcx: ty::TyCtxt<'tcx>, f: Ty<'tcx>, value: Expr) -> Expr {
    let ty::FnDef(def_id, _) = *f.kind() else {
        return value;
    };
    let mut facts = value.facts;
    let output = tcx.fn_sig(def_id).skip_binder().skip_binder().output();
    // Of a `bool` it gives, that it's one, as a `bool` constant's and a
    // global's read are (ADRs 0031, 0953): `canUseDOM &&
    // isEventSupported("input") && ..`, no `!!` of it (rd145).
    if output.is_bool() {
        facts = facts.with(Known::Boolean);
    }
    Expr { facts, ..value }
}

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// The call of the crate's function `def_id`, given `args`: of a class's
    /// constructor, `new Component(..)`, and of its method, a call of the
    /// prototype's, `component.setState(..)` (ADR 0761); of an object's
    /// method, its own property's, `scope.containsNode(..)` (ADR 0943); of
    /// any other, `f(..)`.
    pub(crate) fn crate_called(&self, def_id: rustc_span::def_id::DefId, args: Vec<Expr>) -> Expr {
        self.crate_called_copy((def_id, None, None), None, args)
    }

    /// `crate_called`'s, of its own JS or of a copy its callers need (ADR
    /// 0051, amended), as the path at `named`, the call's, names it, which
    /// may be through the module that has it (rd114, ADR 1018).
    pub fn crate_called_copy(
        &self,
        (def_id, copy, named): (rustc_span::def_id::DefId, Option<usize>, Option<Span>),
        receiver_named: Option<(bool, bool)>,
        mut args: Vec<Expr>,
    ) -> Expr {
        // A namespace's method (ADR 1037): through the value it's called on,
        // `NoopRenderer.createContainer(..)`, one JS may have made of
        // another instance of its module; of the namespace named, or its own
        // method's `self` (`classes::names_namespace`), its `self` left out,
        // the function itself, `createContainer(..)`. One whose MIR no call
        // gives, a method's as a `fn`, `(a, b) => a.updateContainer(b)`, is
        // of its parameter, a value.
        if crate::classes::member(self.tcx, def_id) == Some(crate::classes::Member::Namespace) && !args.is_empty() {
            let (by_name, flattened) = receiver_named.unwrap_or((false, false));
            let receiver = args.remove(0);
            if !by_name {
                // Of a copy its call needs, the value's function, the
                // module's own JS, only where that copy is alike it
                // (`pipeline::copies::settle`), or its result is that JS's
                // flattened (rd311).
                if let (Some(at), Some(span), false) = (copy, named, flattened) {
                    (self.dependencies.borrow_mut().through_copies).push((def_id, at, span));
                }
                // Its module's function, which reads no `this` (ADR 1037):
                // read apart from the value, it's called as it was (#561).
                return Expr {
                    facts: js::Facts::of(js::Known::ThisUnread).with(js::Known::NamespaceMethod),
                    ..Expr::call(Expr::member(receiver, crate::classes::property(self.tcx, def_id)), args)
                };
            }
        }
        // One that's the function it passes its parameters on to, which its
        // module re-exports by its name (rd292b, ADR 1037 amended): in that
        // module, which has no name of its own for it, that function's call;
        // of its own module's function, which is what its module exports by
        // that name, that function's anywhere, by the name a module imports
        // it by, `closeFlight` (#566).
        let def_id = self.forwarded_fn(def_id);
        let callee = match (copy, named) {
            (_, Some(named)) => self.named_fn_ref(def_id, copy, named),
            (Some(_), None) => self.own_fn_ref(def_id, copy),
            (None, None) => self.fn_ref(def_id),
        };
        match (crate::classes::member(self.tcx, def_id), callee.kind) {
            (Some(crate::classes::Member::Constructor), js::ExprKind::Member(class, _)) => {
                match crate::classes::flattened_parameter(self.tcx, def_id) {
                    Some((at, keys)) if at < args.len() => {
                        let own = crate::signatures::own_parameter_names(self.tcx, def_id);
                        crate::classes::new_of_state(*class, args, (at, &keys), &own)
                    }
                    _ => Expr::new_(*class, args),
                }
            }
            (Some(crate::classes::Member::Method), js::ExprKind::Member(_, name)) if !args.is_empty() => {
                let this = args.remove(0);
                Expr::call(Expr::member(this, name), args)
            }
            // A getter's, its read, `position.leftDiagonal` (#357), which
            // runs its body: never moved past what else runs (rd679).
            (Some(crate::classes::Member::Getter), js::ExprKind::Member(_, name)) if args.len() == 1 => {
                Expr::member(args.remove(0), name).answered_by_a_getter()
            }
            // An object's method, its own property's (ADR 0943).
            (Some(crate::classes::Member::Own), _) if !args.is_empty() => {
                let this = args.remove(0);
                Expr::call(Expr::member(this, crate::classes::property(self.tcx, def_id)), args)
            }
            (_, kind) => Expr::call(Expr { kind, ..callee }, args),
        }
    }

    /// `def_id`, or the function it passes its parameters on to, which its
    /// module re-exports by its name, where a call is of that.
    fn forwarded_fn(&self, def_id: rustc_span::def_id::DefId) -> rustc_span::def_id::DefId {
        let module_of = |id| self.krate.fns.get(&id).map(|info: &crate::FnInfo| info.module);
        match self.krate.forwarded.get(&def_id) {
            Some(&target) if module_of(def_id) == Some(self.module) || module_of(target) == module_of(def_id) => target,
            _ => def_id,
        }
    }

    /// The function `crate_called` calls of `def_id`, given its arguments
    /// apart: of a plain function, the one it forwards to where that's
    /// what its module has (`forwarded_fn`); `None` of a class's or a
    /// namespace's member, called through its value.
    pub(crate) fn crate_callee(&self, def_id: rustc_span::def_id::DefId) -> Option<Expr> {
        if crate::classes::member(self.tcx, def_id).is_some() {
            return None;
        }
        Some(self.fn_ref(self.forwarded_fn(def_id)))
    }
}
