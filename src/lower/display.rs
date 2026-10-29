//! `Display` (ADR 0054): a function that writes to a `Formatter` returns the
//! string it writes. Its formatter is a local string, each write is `f += s`,
//! and an `Ok` `fmt::Result` is nothing at all; an `Err`, which chrono's
//! formatting returns, is thrown, with what was written before it (ADR 0187).

mod debug;
mod written;

use super::bindings;

use super::format_spec::{Options, Radix};
use super::passed_on::{Taker, passed_on};
use super::recognition::{BuilderStep, ChannelError, DebugBuilder, FmtResultAnswer, FormatterQuery, Std};
use super::recognition::{StdItem, WriteCall, opt_std_item, std_item};
use super::representation::{self, Num};
use super::{FnCx, R};
use crate::js::result;
use crate::js::{self, Expr, Op, Stmt, StmtKind};
use rustc_hir::attrs::lang_items::LangItem;
use rustc_middle::ty::{self, Ty, TypeVisitableExt};
use rustc_span::Span;
use rustc_span::def_id::DefId;

/// What writing to a `Formatter` knows (ADRs 0054, 0137).
#[derive(Default)]
pub(super) struct Writing {
    /// In such a function of a crate that gives a `Formatter` options (ADRs
    /// 0058, 0137): the parameter that has them, `{ alternate: true }`.
    given: Option<Expr>,
    /// How a `&dyn Debug` made here shows its value: with its writer's
    /// options, while a derived `Debug`'s or a builder's
    /// arguments are lowered (ADR 0137), plain anywhere else.
    dyn_debug: Pretty,
    /// The options a `{:?}` being shown gives each part of it (ADR 0058).
    options: Option<Options>,
}

/// The options a `fmt` is given (ADRs 0058, 0137): none, `{:#?}`'s, a
/// writer's own, which say at run time, or a placeholder's, `{ width: 6 }`,
/// which say whether it's pretty.
#[derive(Clone, Default)]
pub(super) enum Pretty {
    #[default]
    Plain,
    Always,
    When(Expr),
    Given(Expr, bool),
}

impl Pretty {
    /// Whether it's pretty, if that's known.
    fn known(&self) -> Option<bool> {
        match self {
            Pretty::Plain => Some(false),
            Pretty::Always => Some(true),
            Pretty::When(_) => None,
            Pretty::Given(_, alternate) => Some(*alternate),
        }
    }

    /// The JS value that says whether it's pretty: `false`, `true`, or
    /// `options?.alternate`.
    pub(super) fn alternate(&self) -> Expr {
        match (self, self.known()) {
            (Pretty::When(options), _) => Expr::optional_member(options.clone(), "alternate"),
            (_, known) => Expr::bool(known == Some(true)),
        }
    }

    /// `pretty_form` or `plain`, as it says.
    fn choose(&self, pretty_form: Expr, plain: Expr) -> Expr {
        match self.known() {
            Some(true) => pretty_form,
            Some(false) => plain,
            None => Expr::cond(self.alternate(), pretty_form, plain),
        }
    }

    /// The options object a `fmt` is given: `undefined`, `{ alternate: true }`,
    /// the writer's own, or a placeholder's.
    fn options(&self) -> Expr {
        match self {
            Pretty::Plain => Expr::undefined(),
            Pretty::Always => Expr::object(vec![js::Prop::Field("alternate".into(), Expr::bool(true))]),
            Pretty::When(options) | Pretty::Given(options, _) => options.clone(),
        }
    }
}

/// What a step of a `Debug` builder adds: one text, or an array of them,
/// as `entries` may give none (ADR 0136).
#[derive(Clone)]
pub(super) enum BuilderPart {
    One(Expr),
    Many(Expr),
}

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// May calling `def_id` of `args` return `Err(fmt::Error)` (ADR 0187):
    /// the crate's or a library's function that may, or a dictionary's,
    /// which may be any, where any of the crate's or its libraries' may.
    pub(super) fn call_may_fail(&self, def_id: DefId, args: ty::GenericArgsRef<'tcx>) -> bool {
        if !self.krate.any_failing {
            return false;
        }
        match self.resolve_instance(def_id, args) {
            Ok(Some(instance)) if matches!(instance.def, ty::InstanceKind::Virtual(..)) => true,
            Ok(Some(instance)) => {
                self.krate.failing.contains(&instance.def_id()) || self.krate.foreign.fails(instance.def_id())
            }
            Ok(None) => args.has_param(),
            Err(_) => false,
        }
    }

    /// May formatting a `ty` by `fmt_trait` fail: its impl's `fmt`, or that of
    /// a type it holds, as std's `Vec<T>` calls `T`'s (ADR 0187)?
    pub(super) fn fmt_may_fail(&self, fmt_trait: DefId, ty: Ty<'tcx>) -> bool {
        if !self.krate.any_failing {
            return false;
        }
        let fmt = self.tcx.associated_item_def_ids(fmt_trait)[0];
        ty.walk()
            .filter_map(|part| part.as_type())
            .any(|part| self.call_may_fail(fmt, self.tcx.mk_args(&[part.into()])))
    }

    /// A call's value, of a function that returns a `fmt::Result` and may
    /// fail, the crate's or a library's: `$writeTry(() => show(s, h))`, the
    /// `fmt::Error` it threw caught (ADR 0187). A writer's write, given its
    /// `Formatter`, is its writer's to pass on.
    pub(super) fn fmt_result_value(&mut self, def_id: DefId, args: ty::GenericArgsRef<'tcx>, value: Expr) -> Expr {
        let output = self
            .tcx
            .fn_sig(def_id)
            .instantiate(self.tcx, args)
            .skip_normalization()
            .skip_binder()
            .output();
        if !self.is_write_result(output) || self.formatter_param(def_id).is_some() || !self.call_may_fail(def_id, args)
        {
            return value;
        }
        self.fmt_try(vec![StmtKind::Return(Some(value)).at(js::Span::NONE)])
    }

    /// `?` of a `fmt::Result`: its error, on, thrown (ADR 0187). What
    /// `$writeTry` would catch runs as it is; another may be the error.
    pub(super) fn fmt_check(&mut self, value: Expr, js_span: js::Span, out: &mut Vec<Stmt>) {
        if matches!(value.kind, js::ExprKind::Undefined) {
            return;
        }
        let tried = matches!(&value.kind, js::ExprKind::Call(callee, args)
            if callee.helper_name() == Some("$writeTry")
                && matches!(args.first().map(|a| &a.kind), Some(js::ExprKind::Arrow(params, _)) if params.is_empty()));
        if !tried {
            out.push(StmtKind::Expr(Expr::call(Expr::helper("$writeCheck"), vec![value])).at(js_span));
            return;
        }
        let js::ExprKind::Call(_, mut args) = value.kind else {
            unreachable!("checked")
        };
        let js::ExprKind::Arrow(_, body) = args.remove(0).kind else {
            unreachable!("checked")
        };
        for stmt in body {
            match stmt.kind {
                StmtKind::Return(Some(returned)) => out.push(StmtKind::Expr(returned).at(stmt.span)),
                kind => out.push(kind.at(stmt.span)),
            }
        }
    }

    /// A method of a `fmt::Result`, `result`, in a crate where one may be
    /// an `Err` (ADR 0187): of what `$writeTry` caught, or a variable it's in,
    /// tested against `undefined`; another's, its constant answer. `message`
    /// is `expect`'s, or `map`'s or `map_err`'s function.
    pub(in crate::lower) fn fmt_result_values(
        &mut self,
        answer: FmtResultAnswer,
        result: Expr,
        message: Option<Expr>,
        may_fail: bool,
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> Expr {
        let constant = match answer {
            FmtResultAnswer::Unwrap | FmtResultAnswer::Expect | FmtResultAnswer::UnwrapOr => Expr::undefined(),
            FmtResultAnswer::Is(ok) => Expr::bool(ok),
            FmtResultAnswer::Ok => self.some(Expr::undefined()),
            FmtResultAnswer::MapErr => crate::js::result::ok(Expr::undefined()),
            // Its function given `()`, `undefined`, which one that takes
            // it as a value reads, `map(Ok)`'s.
            FmtResultAnswer::Map => {
                let f = message.clone().expect("`map`'s function");
                crate::js::result::ok(crate::lower::calls::apply(f, vec![Expr::undefined()]))
            }
        };
        // Only what `$writeTry` caught may be an `Err`, or a variable it's in,
        // and only where one may fail.
        let may_be_err = may_fail
            && match &result.kind {
                js::ExprKind::Call(callee, _) => callee.helper_name() == Some("$writeTry"),
                js::ExprKind::Var(_) => true,
                _ => false,
            };
        if !may_be_err {
            for value in std::iter::once(result).chain(message) {
                if value.has_effects() {
                    out.push(StmtKind::Expr(value).at(self.js_span(span)));
                }
            }
            return constant;
        }
        let result = if result.reads_same() {
            result
        } else {
            self.spill("result", result, out)
        };
        let ok = Expr::bin(Op::Eq, result.clone(), Expr::undefined());
        match answer {
            FmtResultAnswer::Ok => Expr::cond(ok, self.some(Expr::undefined()), Expr::undefined()),
            FmtResultAnswer::UnwrapOr => Expr::undefined(),
            FmtResultAnswer::Map => Expr::cond(ok, constant, crate::js::result::err(result)),
            FmtResultAnswer::MapErr => {
                let f = message.expect("`map_err`'s function");
                let mapped = crate::js::result::err(self.applied(f, result));
                Expr::cond(ok, crate::js::result::ok(Expr::undefined()), mapped)
            }
            FmtResultAnswer::Is(true) => ok,
            FmtResultAnswer::Is(false) => Expr::bin(Op::Ne, result, Expr::undefined()),
            FmtResultAnswer::Unwrap | FmtResultAnswer::Expect => {
                let message = message.unwrap_or_else(|| Expr::str("called `Result::unwrap()` on an `Err` value"));
                Expr::call(Expr::helper("$writeExpect"), vec![result, message])
            }
        }
    }

    /// `$writeTry(() => { .. })`: what `body` writes, as a `fmt::Result`,
    /// `undefined` or the `fmt::Error` it failed with (ADR 0187).
    pub(super) fn fmt_try(&mut self, body: Vec<Stmt>) -> Expr {
        Expr::call(Expr::helper("$writeTry"), vec![Expr::arrow(Vec::new(), body)])
    }

    /// `is_generic_fmt_result` in this body's typing environment.
    pub(super) fn generic_fmt_result(&self, declared: Ty<'tcx>, generic_args: ty::GenericArgsRef<'tcx>) -> bool {
        super::recognition::is_generic_fmt_result(self.tcx, self.typing_env, declared, generic_args)
    }

    /// Is the function being lowered one that may return `Err(fmt::Error)`
    /// (ADR 0187)?
    pub(super) fn writer_fails(&self) -> bool {
        self.krate.failing.contains(&self.item)
    }

    /// In a writer given options (ADRs 0137, 0143): what `f.alternate()`,
    /// `f.width()` and the like answer, `options?.alternate === true` and
    /// `options?.width`, which is `None` where it's not given.
    pub(super) fn formatter_answer(&self, query: FormatterQuery) -> Option<Expr> {
        let options = self.writing.given.clone()?;
        let member = |name: &str| Expr::optional_member(options.clone(), name);
        let is_true = |name: &str| Expr::bin(Op::Eq, member(name), Expr::bool(true));
        Some(match query {
            FormatterQuery::Alternate => is_true("alternate"),
            FormatterQuery::Width => member("width"),
            FormatterQuery::Precision => member("precision"),
            FormatterQuery::Fill => Expr::bin(Op::Coalesce, member("fill"), Expr::str(" ")),
            FormatterQuery::Align => member("align"),
            FormatterQuery::SignPlus => is_true("plus"),
            FormatterQuery::SignAwareZeroPad => is_true("zero"),
        })
    }

    /// Do the crate's writers take a `Formatter`'s options (ADRs 0058, 0137):
    /// does it show anything pretty, or give a placeholder's options to what
    /// isn't a number, a `bool` or a string?
    /// Does `writer`, a function that writes, take its `Formatter`'s
    /// options after its arguments? Its crate's answer (ADR 0137), a
    /// library's in its manifest, but one the crate compiles itself, its own
    /// (ADR 0914).
    pub(super) fn takes_options(&self, writer: DefId) -> bool {
        match writer.is_local() || self.krate.foreign.is_instance(writer) {
            true => self.writers_take_options(),
            false => self.krate.foreign.writers_take_options(writer.krate),
        }
    }

    /// What another writer this one calls, `writer`, is given after its
    /// arguments: this one's `Formatter`'s options where it takes them,
    /// `undefined` where this one has none. A callable, `None`, any writer,
    /// is given this one's.
    pub(super) fn given_to_writer(&self, writer: Option<DefId>) -> Option<Expr> {
        match writer {
            Some(writer) if !self.takes_options(writer) => None,
            Some(_) => Some(self.writing.given.clone().unwrap_or_else(Expr::undefined)),
            None => self.writing.given.clone(),
        }
    }

    pub(super) fn writers_take_options(&self) -> bool {
        self.krate.writers_take_options()
    }

    /// `lower`, a writer's body, given its `Formatter`'s options, `given`,
    /// the parameter that has them, where the crate gives any (ADR 0137).
    pub(super) fn with_given<T>(&mut self, given: Option<Expr>, lower: impl FnOnce(&mut Self) -> T) -> T {
        let outer = std::mem::replace(&mut self.writing.given, given);
        let made = lower(self);
        self.writing.given = outer;
        made
    }

    /// `lower`, during which each part of a `{:?}` shown is given `options`.
    pub(super) fn with_options<T>(&mut self, options: Option<Options>, lower: impl FnOnce(&mut Self) -> T) -> T {
        let outer = std::mem::replace(&mut self.writing.options, options);
        let made = lower(self);
        self.writing.options = outer;
        made
    }

    /// A part of a `{:?}` that applies its options itself, as std's `fmt`
    /// does: a number, a `bool` and `()` pad, and a string doesn't.
    pub(super) fn is_debug_leaf(&self, ty: Ty<'tcx>) -> bool {
        Num::of(ty).is_some() || ty.is_bool() || ty.is_unit() || self.is_string_like(ty)
    }

    /// A number, a `bool`, `()` or a string shown in a writer given options
    /// at run time (ADR 0058): `$formatted(text, options)`, which applies
    /// them as std's `fmt` does. A string's and a `char`'s `{:?}` don't, and
    /// a crate that gives none but `{:#?}`'s has none to apply.
    fn formatted(&mut self, value: Expr, (kind, ty): (Std, Ty<'tcx>), pretty: &Pretty) -> Option<Expr> {
        let Pretty::When(options) = pretty else { return None };
        if !self.krate.format_options {
            return None;
        }
        let num = Num::of(ty);
        // A precision is a float's digits.
        if let Some(num) = num.filter(|n| n.float()) {
            let show = match (kind, num) {
                (Std::FmtDebug, Num::F32) => "$debugF32",
                (Std::FmtDebug, _) => "$debugF64",
                (_, Num::F32) => "$displayF32",
                _ => "$displayF64",
            };
            return Some(Expr::call(
                Expr::helper("$formatFloat"),
                vec![value, options.clone(), Expr::var(show)],
            ));
        }
        let text = match kind {
            // An integer's `{:?}` in a crate that asks `{:x?}`: in hex where
            // the options say, as `{:x}` shows it, `0x` and all where it's
            // alternate (ADR 0559).
            Std::FmtDebug
                if self.krate.debug_hex
                    && let Some(num) = num.filter(|n| !n.float()) =>
            {
                let hex = Radix::LowerHex.digits(value.clone(), num);
                Expr::call_helper("$debugInt", vec![shown_number(value), hex, options.clone()])
            }
            _ if num.is_some() || ty.is_bool() => shown_number(value),
            Std::FmtDebug if ty.is_unit() => Expr::str("()"),
            Std::FmtDisplay if self.is_string_like(ty) => value,
            _ => return None,
        };
        let mut args = vec![text, options.clone()];
        if num.is_some() {
            args.push(Expr::bool(true));
        }
        Some(Expr::call(Expr::helper("$formatted"), args))
    }

    /// An error if a placeholder's options are given to what can't apply
    /// them (ADR 0058): serde_json's types, whose `fmt`s are rust-js's own,
    /// and a `&dyn Debug`, the string it shows already (ADR 0060).
    fn can_apply(&self, ty: Ty<'tcx>, pretty: &Pretty, span: Span) -> R<()> {
        if matches!(pretty, Pretty::Given(..))
            && (self.json_type(ty).is_some()
                || self.is_json_error(ty)
                || self.recognition().is_js_error(ty)
                || self.is_dyn_debug(ty))
        {
            return Err(self.unsupported(span, &format!("options for a `{ty}`")));
        }
        Ok(())
    }

    /// `value`, of type `ty`, made a `&dyn Debug` here: the string it shows
    /// (ADR 0060), pretty or plain as `with_dyn_debug` says.
    pub(super) fn dyn_debug_string(&mut self, value: Expr, ty: Ty<'tcx>, span: Span) -> R<Expr> {
        let pretty = self.writing.dyn_debug.clone();
        self.debug_string_with(value, ty, span, &pretty)
    }

    /// `{:x}` or `{:p}` of a generic `T`, `LowerHex::fmt(t, f)` too: its
    /// dictionary's `fmt`, given the placeholder's options as a `T: Display`'s
    /// is (ADR 0174).
    pub(super) fn other_fmt_dictionary(
        &mut self,
        trait_id: DefId,
        ty: Ty<'tcx>,
        value: Expr,
        pretty: &Pretty,
        span: Span,
    ) -> R<Expr> {
        let tr = ty::TraitRef::new(self.tcx, trait_id, [ty]);
        let dictionary = self.evidence_for(tr).ok_or_else(|| self.no_evidence(span, tr))?;
        let mut list = vec![value];
        list.extend(self.options_arg(pretty));
        Ok(Expr::call(Expr::member(dictionary, "fmt"), list))
    }

    /// std's `LowerHex`, `UpperHex`, `Octal` or `Binary` of an integer, given a
    /// `Formatter`'s options as it runs: its digits, its prefix where it's
    /// alternate, padded as a number is (ADR 0185).
    pub(super) fn radix_text(&mut self, value: Expr, num: Num, radix: Radix, pretty: &Pretty) -> Expr {
        let digits = radix.digits(value, num);
        let text = match pretty.known() {
            Some(true) => join(vec![Expr::str(radix.prefix()), digits]),
            Some(false) => digits,
            None => {
                let prefix = Expr::cond(pretty.alternate(), Expr::str(radix.prefix()), Expr::str(""));
                Expr::bin(Op::Add, prefix, digits)
            }
        };
        match pretty {
            Pretty::When(options) if self.krate.format_options => Expr::call(
                Expr::helper("$formatted"),
                vec![text, options.clone(), Expr::bool(true)],
            ),
            _ => text,
        }
    }

    /// Whether std's `LowerExp`, `false`, or `UpperExp`, `true`, is
    /// `trait_id`.
    pub(super) fn exp_trait(&self, trait_id: DefId) -> Option<bool> {
        [false, true]
            .into_iter()
            .find(|&upper| self.recognition().other_fmt_trait(Std::FmtExp(upper)) == Some(trait_id))
    }

    /// The radix std's `LowerHex`, `UpperHex`, `Octal` or `Binary` shows.
    pub(super) fn radix_trait(&self, trait_id: DefId) -> Option<Radix> {
        [Radix::LowerHex, Radix::UpperHex, Radix::Octal, Radix::Binary]
            .into_iter()
            .find(|&radix| self.recognition().other_fmt_trait(Std::FmtRadix(radix)) == Some(trait_id))
    }

    /// The type `ty` shows: what a `Box` or an `Rc` holds, as they are it in
    /// JS, and `ty` itself otherwise.
    pub(super) fn shown_type(&self, ty: Ty<'tcx>) -> Ty<'tcx> {
        // What an `impl Display` a function gives hides: its type, whose
        // own `Display` it is (#171).
        let ty = self.reveal(ty);
        match ty.kind() {
            ty::Adt(_, args) if self.shows_inside(ty) => args.types().next().expect("what it holds").peel_refs(),
            _ => ty,
        }
    }

    /// Whether `ty`'s `Display::fmt` is a binding of the value itself,
    /// `#[link_name = "this"]`, `js::shown`'s: its text is what JS's
    /// template makes of the value, `shown_text` (ADR 0312).
    pub(super) fn displays_itself(&self, ty: Ty<'tcx>) -> bool {
        // A class's, its prototype's `toString`, given no `Formatter`'s
        // options, which JS's template gives it none of (#358).
        if self.is_shown_class(ty) && !self.writers_take_options() {
            return true;
        }
        let Some(fmt) = self.recognition().display_fmt() else {
            return false;
        };
        let args = self.tcx.mk_args(&[self.tcx.erase_and_anonymize_regions(ty).into()]);
        matches!(self.resolve_instance(fmt, args), Ok(Some(instance))
            if bindings::is_binding(self.tcx, instance.def_id())
                && matches!(bindings::js_form(self.tcx, instance.def_id()), bindings::JsForm::This))
    }

    /// Whether `ty` is a class with a `Display`, its prototype's `toString`
    /// (#358), no dictionary's.
    pub(super) fn is_shown_class(&self, ty: Ty<'tcx>) -> bool {
        matches!(ty.kind(), ty::Adt(adt, _) if crate::lower::classes::is_class(self.tcx, adt.did()))
            && self.has_user_impl(self.display_trait(), ty)
    }

    /// `ToString`, which is `alloc`'s: a `#![no_std]` crate that doesn't load
    /// `alloc` has none, as bitflags and num-traits don't.
    pub(super) fn to_string_trait(&self) -> Option<DefId> {
        opt_std_item(self.tcx, StdItem::ToString)
    }

    pub(super) fn debug_trait(&self) -> DefId {
        std_item(self.tcx, StdItem::Debug)
    }

    /// Is `ty` `dyn Debug`, which rust-js holds as the string it shows
    /// (ADR 0060), behind a reference, a `Box` or an uncounted `Rc`, each
    /// its value (ADR 0023)? A derived `Debug` hands its fields to the
    /// formatter so.
    pub(super) fn is_dyn_debug(&self, ty: Ty<'tcx>) -> bool {
        let mut ty = ty.peel_refs();
        while let ty::Adt(_, args) = ty.kind()
            && (ty.is_box() || self.is_rc(ty) && self.counted_rc(ty).is_none())
        {
            ty = args.type_at(0).peel_refs();
        }
        matches!(ty.kind(), ty::Dynamic(traits, ..) if traits.principal_def_id() == Some(self.debug_trait()))
    }

    /// How pretty what this writer shows is (ADR 0137): as its `alternate`
    /// says, or plain where the crate's writers don't take one.
    pub(super) fn writer_pretty(&self) -> Pretty {
        match &self.writing.given {
            Some(options) => Pretty::When(options.clone()),
            None => Pretty::Plain,
        }
    }

    /// `value`, in a `const` first if it may run code, which a plain and a
    /// pretty form would each run.
    fn once(&mut self, value: Expr, out: &mut Vec<Stmt>) -> Expr {
        if self.writing.given.is_none() || value.reads_same() {
            value
        } else {
            self.spill("shown", value, out)
        }
    }

    /// `pretty_or`, of an `open` and a `close` that are expressions.
    fn pretty_or_parts(&mut self, pretty: &Pretty, plain: Expr, open: Expr, items: Expr, close: Expr) -> Expr {
        if pretty.known() == Some(false) {
            return plain;
        }
        pretty.choose(Expr::call(Expr::helper("$pretty"), vec![open, items, close]), plain)
    }

    /// `plain`, or, pretty, `$pretty(open, items, close)` of the same parts:
    /// each on a line of its own, indented, and ended by a comma (ADR 0137).
    fn pretty_or(&mut self, pretty: &Pretty, plain: Expr, open: &str, items: Expr, close: &str) -> Expr {
        self.pretty_or_parts(pretty, plain, Expr::str(open), items, Expr::str(close))
    }

    /// What a writer function is given after its value, where the crate's
    /// take options (ADRs 0058, 0137): `undefined` for a plain one, which
    /// the printer leaves out where it's last.
    pub(super) fn options_arg(&self, pretty: &Pretty) -> Option<Expr> {
        self.writers_take_options().then(|| pretty.options())
    }

    /// A call of the crate's own `fmt`, `Debug`'s or `Display`'s, given
    /// whether it's pretty where it takes it (ADR 0137), but left out of a
    /// plain one that takes nothing after it.
    pub(super) fn writer_call(
        &mut self,
        method: DefId,
        args: ty::GenericArgsRef<'tcx>,
        value: Expr,
        pretty: &Pretty,
        span: Span,
    ) -> R<Expr> {
        let args = self.tcx.erase_and_anonymize_regions(args);
        // Through a library's dictionary, its entry, given what the method
        // takes (ADR 0914, amended).
        if let Some((instance, entry)) = self.dictionary_entry(method, args, 1, span)? {
            let mut values = vec![value];
            if self.takes_options(instance.def_id()) {
                values.push(pretty.options());
            }
            return Ok(Expr::call(entry, values));
        }
        let instance = self
            .resolve_instance(method, args)?
            .filter(|i| self.is_rust_fn(i.def_id()))
            .ok_or_else(|| self.unsupported(span, "this implementation"))?;
        let evidence = self.evidence_args(instance.def_id(), instance.args, span)?;
        let mut values = vec![value];
        // A class's `toString` given no options is called with none,
        // `p.toString()` (#358).
        let plain_to_string = matches!(pretty, Pretty::Plain)
            && crate::lower::classes::shown_class(self.tcx, instance.def_id()).is_some();
        if self.takes_options(instance.def_id()) && !plain_to_string {
            values.push(pretty.options());
        }
        values.extend(evidence);
        // A class's `toString` is called on the value (#358).
        Ok(self.crate_called(instance.def_id(), values))
    }

    /// `Box<T>` and `Rc<T>`: shown as their `T`, which is the value they are
    /// in JS (ADR 0023).
    pub(in crate::lower) fn shows_inside(&self, ty: Ty<'tcx>) -> bool {
        ty.is_box() || self.is_rc(ty)
    }

    /// `f(value)`, with a function that only returns written in place when
    /// `value` is a variable: `value == null ? "None" : ..`.
    pub(super) fn applied(&mut self, f: Expr, value: Expr) -> Expr {
        if let js::ExprKind::Arrow(params, body) = &f.kind
            && let [js::Pattern::Name(name)] = params.as_slice()
            && let [
                Stmt {
                    kind: StmtKind::Return(Some(result)),
                    ..
                },
            ] = body.as_slice()
            && value.reads_same()
            && let Some(inlined) = result.substitute_in_callbacks(&|n: &str| (n == name).then(|| value.clone()))
        {
            return inlined;
        }
        Expr::call(f, vec![value])
    }

    /// `(value) => <its string>` for a dictionary's `fmt`, or the function
    /// itself: `String`, `$displayF64`.
    pub(super) fn display_fn(&mut self, ty: Ty<'tcx>, span: Span) -> R<Expr> {
        // Given a `Formatter`'s options, where the crate's writers are (ADR 0058).
        let (params, pretty): (Vec<js::Pattern>, _) = match self.writers_take_options() {
            true => (
                vec!["value".into(), "options".into()],
                Pretty::When(Expr::var("options")),
            ),
            false => (vec!["value".into()], Pretty::Plain),
        };
        let shown = self.display_string_with(Expr::var("value"), ty, span, &pretty)?;
        // `String(value)` of a number is the function `String`, and so is
        // JS's text of the value, \`${value}\`, a class's `toString`'s (#358).
        let text_of = match &shown.kind {
            js::ExprKind::Stringed(value) => Some(&**value),
            js::ExprKind::Template(_, values, _) if is_shown_text(&shown) && values.len() == 1 => values.first(),
            _ => None,
        };
        if let Some(value) = text_of
            && params.len() == 1
            && is_var(value, "value")
        {
            return Ok(Expr::var("String"));
        }
        let fmt = Expr::arrow(params, vec![StmtKind::Return(Some(shown)).at(js::Span::NONE)]);
        // One that calls a function with what it's given, `$displayF64`.
        Ok(passed_on(&fmt, Taker::Made, &[]).unwrap_or(fmt))
    }
}

/// Strings joined: text alone is a string, text and values a template
/// literal, \`Some(${x})\`, and strings alone `a + b`. A part that's itself
/// strings joined is taken apart, so templates don't nest needlessly; but
/// not a template of values alone, \`${x}\`, `shown_text`'s, whose values
/// only a template makes text of, which makes what it's in a template,
/// \`${x}${s}\` (ADR 0066).
pub(super) fn join(parts: Vec<Expr>) -> Expr {
    joined(parts.into_iter().map(|part| (part, false)).collect(), false)
}

/// `join`, or, `separable`, of what's written piece by piece, as what may
/// fail is (`append_written`): strings alone a template, \`${a}${b}\`,
/// whose pieces are there to take apart, where `a + b` is one (ADR 0187).
/// Each part is a format argument written across lines, or not, which a
/// template's value of it is (`TemplateLayout::across`).
fn joined(parts: Vec<(Expr, bool)>, separable: bool) -> Expr {
    let pieces = merged(parts);
    let is_text = |(p, _): &(Expr, bool)| matches!(p.kind, js::ExprKind::Str(_));
    let strings_alone = !pieces.iter().any(|p| is_text(p) || is_shown_text(&p.0));
    if pieces.iter().all(is_text) || strings_alone && !(separable && pieces.len() > 1) {
        return (pieces.into_iter())
            .map(|(piece, _)| piece)
            .reduce(|a, b| Expr::bin(Op::Add, a, b))
            .unwrap_or_else(|| Expr::str(""));
    }
    let (texts, values, across) = texts_and_values(pieces);
    Expr::template_across(texts, values, across)
}

/// The text JS's template makes of `value`, \`${value}\`, as `js::shown`'s
/// `Display` writes it (ADR 0312): a string, where the value itself may be
/// a number or an object. It stays a template inside a template, so that
/// what takes one apart, `append_written`'s `+=` piece by piece, appends
/// text, not the value; preparation splices it into the outer one.
pub(super) fn shown_text(value: Expr) -> Expr {
    Expr::template(vec![String::new(), String::new()], vec![value])
}

/// Whether `e` is a template of values alone, \`${x}\`, which is text only
/// as a template: `shown_text`'s, or one made of such.
fn is_shown_text(e: &Expr) -> bool {
    matches!(&e.kind, js::ExprKind::Template(texts, ..) if texts.iter().all(String::is_empty))
}

/// A format string's pieces as it's written, each its parts, each one and
/// whether it's an argument written across lines, and whether the piece is
/// (`join_written`).
pub(super) type WrittenParts = Vec<(Vec<(Expr, bool)>, bool)>;

/// Strings joined as their format string is written, `pieces` of it, each
/// its parts and whether it's written across lines: one, as `join` joins
/// it; `concat!`'s, each a template where it holds a value and otherwise
/// a string, joined by `+` (ADR 0641). `separable`, of what's written
/// piece by piece, as `joined` says.
pub(super) fn join_written(mut pieces: WrittenParts, separable: bool) -> Expr {
    if pieces.len() == 1 {
        let (parts, lines) = pieces.pop().expect("a piece");
        return joined(parts, separable).with_lines(lines);
    }
    let pieces = (pieces.into_iter())
        .map(|(parts, lines)| {
            let (texts, values, across) = texts_and_values(merged(parts));
            (texts, values, across, lines)
        })
        .collect();
    Expr::template_pieces(pieces)
}

/// `parts`, each strings joined taken apart, each piece of one written
/// across lines as it is, and texts next to each other one.
fn merged(parts: Vec<(Expr, bool)>) -> Vec<(Expr, bool)> {
    let mut pieces: Vec<(Expr, bool)> = Vec::new();
    let all = (parts.into_iter())
        .flat_map(|(part, across)| joined_pieces(part).into_iter().map(move |piece| (piece, across)));
    for (piece, across) in all {
        match (pieces.last_mut(), &piece.kind) {
            (
                Some((
                    Expr {
                        kind: js::ExprKind::Str(before),
                        ..
                    },
                    _,
                )),
                js::ExprKind::Str(after),
            ) => before.push_str(after),
            _ => pieces.push((piece, across)),
        }
    }
    pieces
}

/// A template's texts and values of `pieces`, its text and values in turn:
/// a `shown_text` among them a value of it, a template in a template.
fn texts_and_values(pieces: Vec<(Expr, bool)>) -> (Vec<String>, Vec<Expr>, Vec<bool>) {
    let (mut texts, mut values, mut across) = (vec![String::new()], Vec::new(), Vec::new());
    for (piece, lines) in pieces {
        match piece.kind {
            js::ExprKind::Str(text) => texts.last_mut().expect("a text").push_str(&text),
            _ => {
                values.push(unstringed(piece));
                across.push(lines);
                texts.push(String::new());
            }
        }
    }
    (texts, values, across)
}

/// `String(n)` of a Rust number is `n` in a template, which makes it a
/// string the same way. Not `String(x)` the program wrote, of any value,
/// which names a symbol a template throws on (ADR 0066).
fn unstringed(value: Expr) -> Expr {
    match value.kind {
        js::ExprKind::Stringed(value) => *value,
        _ => value,
    }
}

/// The pieces of strings joined: of a template, its texts and values, which
/// go back into one, as there's text among them; of `"(" + a + ")"`, which
/// starts with a string, so that each `+` in it concatenates, its operands.
/// Anything else is one piece, a template of values alone too, whose values
/// may not be text (`shown_text`).
fn joined_pieces(e: Expr) -> Vec<Expr> {
    if is_shown_text(&e) {
        return vec![e];
    }
    if let js::ExprKind::Template(texts, values, _) = e.kind {
        let mut pieces = Vec::new();
        let mut values = values.into_iter();
        for text in texts {
            if !text.is_empty() {
                pieces.push(Expr::str(text));
            }
            pieces.extend(values.next());
        }
        return pieces;
    }
    vec![e]
}

fn is_var(e: &Expr, name: &str) -> bool {
    matches!(&e.kind, js::ExprKind::Var(n) if n == name)
}

/// `String(n)`, or the text itself for a constant: `"4096"`.
fn shown_number(value: Expr) -> Expr {
    match value.as_int() {
        Some(n) => Expr::str(n.to_string()),
        None => Expr::stringed(value),
    }
}

/// `target += written`: piece by piece, `pieces`, where what it writes may
/// fail, so what came before it is written (ADR 0187).
pub(super) fn append_written(target: &Expr, written: Expr, pieces: bool, js_span: js::Span, out: &mut Vec<Stmt>) {
    let parts = if pieces && let js::ExprKind::Template(texts, values, _) = &written.kind {
        let mut parts = Vec::new();
        let mut values = values.iter().cloned();
        for text in texts {
            if !text.is_empty() {
                parts.push(Expr::str(text));
            }
            parts.extend(values.next());
        }
        parts
    } else {
        vec![written]
    };
    for part in parts {
        out.push(StmtKind::Assign(target.clone(), Expr::text_joined(target.clone(), part)).at(js_span));
    }
}

/// A `VarError` shown, `value`: of `NotPresent`, its text; of `NotUnicode`,
/// the text around its `OsString`'s `{:?}` (ADR 0391). A value shown is held
/// where it's read twice (ADR 0377).
fn var_error_shown(value: Expr, (present, before, after): (&str, &str, &str)) -> Expr {
    let shown = Expr::call(
        Expr::helper("$debugStr"),
        vec![Expr::member(value.clone(), js::positional(0))],
    );
    let unicode = Expr::template(vec![before.to_string(), after.to_string()], vec![shown]);
    Expr::cond(
        Expr::bin(Op::Eq, value, Expr::str("NotPresent")),
        Expr::str(present),
        unicode,
    )
}
