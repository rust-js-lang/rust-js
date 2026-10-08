//! `Display` (ADR 0054): a function that writes to a `Formatter` returns the
//! string it writes. Its formatter is a local string, each write is `f += s`,
//! and an `Ok` `fmt::Result` is nothing at all; an `Err`, which chrono's
//! formatting returns, is thrown, with what was written before it (ADR 0187).

use super::fn_def;
use super::format_spec::{Options, Radix};
use super::recognition::{ChannelError, FormatterQuery, Std};
use super::recognition::{StdItem, WriteCall, fmt_trait_called, opt_std_item, std_item, trait_method};
use super::representation::{self, Num};
use super::{Dest, FnCx, R};
use crate::js::{self, Expr, Op, Stmt, StmtKind};
use crate::runtime::Helper;
use rustc_hir::attrs::lang_items::LangItem;
use rustc_middle::thir::visit::{self, Visitor};
use rustc_middle::thir::{self, ExprId, ExprKind, LocalVarId, PatKind};
use rustc_middle::ty::{self, Ty, TypeVisitableExt};
use rustc_span::Span;
use rustc_span::def_id::DefId;

/// What writing to a `Formatter` knows (ADRs 0054, 0137).
#[derive(Default)]
pub(super) struct Writing {
    /// In a function that writes to a `Formatter` (ADR 0054): its variable,
    /// and the JS string that stands for it.
    writer: Option<(Option<LocalVarId>, String)>,
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

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// May calling `def_id` of `args` return `Err(fmt::Error)` (ADR 0187):
    /// the crate's or a library's function that may, or a dictionary's,
    /// which may be any, where any of the crate's or its libraries' may.
    pub(super) fn call_may_fail(&self, def_id: DefId, args: ty::GenericArgsRef<'tcx>) -> bool {
        if !self.krate.any_failing {
            return false;
        }
        match ty::Instance::try_resolve(self.tcx, self.typing_env, def_id, args) {
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

    /// May what `e` formats fail: a `format_args!`'s arguments, or a
    /// `to_string()`'s value, by their traits (ADR 0187)?
    pub(super) fn formats_may_fail(&self, e: ExprId) -> bool {
        if !self.krate.any_failing {
            return false;
        }
        let mut found = Called {
            thir: self.thir,
            calls: Vec::new(),
        };
        found.visit_expr(&self.thir[e]);
        found.calls.into_iter().any(|(id, args)| {
            fmt_trait_called(self.tcx, id)
                .is_some_and(|fmt_trait| args.types().next().is_some_and(|ty| self.fmt_may_fail(fmt_trait, ty)))
        })
    }

    /// A call's value, of a function that returns a `fmt::Result` and may
    /// fail, the crate's or a library's: `$fmtTry(() => show(s, h))`, the
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
        if !self.is_fmt_result(output) || self.formatter_param(def_id).is_some() || !self.call_may_fail(def_id, args) {
            return value;
        }
        self.fmt_try(vec![StmtKind::Return(Some(value)).at(js::Span::NONE)])
    }

    /// `?` of a `fmt::Result`: its error, on, thrown (ADR 0187). What
    /// `$fmtTry` would catch runs as it is; another may be the error.
    pub(super) fn fmt_check(&mut self, value: Expr, js_span: js::Span, out: &mut Vec<Stmt>) {
        if matches!(value.kind, js::ExprKind::Undefined) {
            return;
        }
        let tried = matches!(&value.kind, js::ExprKind::Call(callee, args)
            if matches!(&callee.kind, js::ExprKind::Var(name) if name == "$fmtTry")
                && matches!(args.first().map(|a| &a.kind), Some(js::ExprKind::Arrow(params, _)) if params.is_empty()));
        if !tried {
            self.runtime.insert(Helper::FmtError);
            out.push(StmtKind::Expr(Expr::call(Expr::var("$fmtCheck"), vec![value])).at(js_span));
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

    /// `$fmtTry(() => { .. })`: what `body` writes, as a `fmt::Result`,
    /// `undefined` or the `fmt::Error` it failed with (ADR 0187).
    pub(super) fn fmt_try(&mut self, body: Vec<Stmt>) -> Expr {
        self.runtime.insert(Helper::FmtError);
        Expr::call(Expr::var("$fmtTry"), vec![Expr::arrow(Vec::new(), body)])
    }

    /// Is the function being lowered one that may return `Err(fmt::Error)`
    /// (ADR 0187)?
    pub(super) fn writer_fails(&self) -> bool {
        self.krate.failing.contains(&self.item)
    }

    /// In a function that writes to a `Formatter` (ADR 0054): the JS string
    /// it's written, which a `return` gives back.
    pub(super) fn written(&self) -> Option<String> {
        self.writing.writer.as_ref().map(|(_, name)| name.clone())
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
    pub(super) fn writers_take_options(&self) -> bool {
        self.krate.pretty_debug || self.krate.format_options
    }

    /// `lower`, during which a `&dyn Debug` made shows its value as `pretty`
    /// says: a derived `Debug`'s or a builder's arguments (ADR 0137).
    pub(super) fn with_dyn_debug<T>(&mut self, pretty: Pretty, lower: impl FnOnce(&mut Self) -> T) -> T {
        let outer = std::mem::replace(&mut self.writing.dyn_debug, pretty);
        let made = lower(self);
        self.writing.dyn_debug = outer;
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
            let (helper, show) = match (kind, num) {
                (Std::FmtDebug, Num::F32) => (Helper::DebugF32, "$debugF32"),
                (Std::FmtDebug, _) => (Helper::DebugF64, "$debugF64"),
                (_, Num::F32) => (Helper::DisplayF32, "$displayF32"),
                _ => (Helper::DisplayF64, "$displayF64"),
            };
            self.runtime.insert(helper);
            self.runtime.insert(Helper::FormatFloat);
            return Some(Expr::call(
                Expr::var("$formatFloat"),
                vec![value, options.clone(), Expr::var(show)],
            ));
        }
        let text = match kind {
            _ if num.is_some() || ty.is_bool() => shown_number(value),
            Std::FmtDebug if ty.is_unit() => Expr::str("()"),
            Std::FmtDisplay if self.is_string_like(ty) => value,
            _ => return None,
        };
        self.runtime.insert(Helper::Formatted);
        let mut args = vec![text, options.clone()];
        if num.is_some() {
            args.push(Expr::bool(true));
        }
        Some(Expr::call(Expr::var("$formatted"), args))
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

    /// The JS parameters of a function that writes to a formatter, and its
    /// body in `out`: `let f = ""`, the writes, and `return f`. Just
    /// `return s` if it writes once.
    pub(super) fn lower_writer(
        &mut self,
        params: &[thir::Param<'tcx>],
        formatter: usize,
        body: ExprId,
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> R<Vec<js::Pattern>> {
        let mut rest = params.to_vec();
        let param = rest.remove(formatter);
        let js_params = self.lower_params(&rest, span, out)?;
        let (var, name) = match param.pat.as_deref().map(|p| &p.kind) {
            Some(PatKind::Binding { name, var, .. }) => (Some(*var), self.bind(*var, name.as_str(), true)),
            // `_: &mut Formatter`: nothing is written.
            Some(PatKind::Wild) => (None, self.fresh("f")),
            _ => return Err(self.unsupported(span, "this `Formatter` parameter")),
        };
        // Its `Formatter`'s options, where the crate gives any (ADRs 0058, 0137).
        let mut js_params = js_params;
        let given = self.writers_take_options().then(|| {
            let options = self.fresh("options");
            js_params.push(options.as_str().into());
            Expr::var(&options)
        });
        let mut body_out = Vec::new();
        let previous = self.writing.writer.replace((var, name.clone()));
        let previous_given = std::mem::replace(&mut self.writing.given, given);
        let lowered = self.stmt(body, &Dest::Discard, &mut body_out);
        self.writing.writer = previous;
        self.writing.given = previous_given;
        lowered?;
        // One that may fail gives what it wrote before, in front of what
        // what it called did, to the writer or the consumer it fails to
        // (ADR 0187): `let f = ""; try { .. } catch (error) { .. } return f;`.
        if self.writer_fails() {
            self.runtime.insert(Helper::FmtError);
            let error = self.fresh("error");
            let written = Expr::call(Expr::var("$fmtWritten"), vec![Expr::var(&error), Expr::var(&name)]);
            out.push(StmtKind::Let(name.clone(), Some(Expr::str(""))).at(js::Span::NONE));
            out.push(
                StmtKind::TryCatch(body_out, Some(error), vec![StmtKind::Throw(written).at(js::Span::NONE)])
                    .at(js::Span::NONE),
            );
            out.push(StmtKind::Return(Some(Expr::var(&name))).at(js::Span::NONE));
            return Ok(js_params);
        }
        // Each way through writes once: each is a `return` of what it writes.
        if let Some(returns) = as_returns(&body_out, &name) {
            out.extend(returns);
            return Ok(js_params);
        }
        // Declarations, then one write: they, then `return` of what it writes.
        if let Some((last, before)) = body_out.split_last()
            && before
                .iter()
                .all(|s| matches!(s.kind, StmtKind::Const(..) | StmtKind::Let(..)))
            && let Some(returns) = as_returns(std::slice::from_ref(last), &name)
        {
            out.extend(before.iter().cloned());
            out.extend(returns);
            return Ok(js_params);
        }
        body_out.push(StmtKind::Return(Some(Expr::var(&name))).at(js::Span::NONE));
        // A first write starts the string: `let f = s;`.
        let first = match body_out.first().map(|s| &s.kind) {
            Some(StmtKind::Assign(target, value)) if is_var(target, &name) => match &value.kind {
                js::ExprKind::Binary(Op::Add, lhs, rhs) if is_var(lhs, &name) => Some((**rhs).clone()),
                _ => None,
            },
            _ => None,
        };
        match first {
            Some(s) => {
                out.push(StmtKind::Let(name, Some(s)).at(js::Span::NONE));
                out.extend(body_out.into_iter().skip(1));
            }
            None => {
                out.push(StmtKind::Let(name, Some(Expr::str(""))).at(js::Span::NONE));
                out.extend(body_out);
            }
        }
        Ok(js_params)
    }

    /// A call that writes to this function's formatter: `write!(f, ..)`,
    /// `f.write_str(s)`, `x.fmt(f)`, or a function of ours that writes. It's
    /// `f += s`, with `s` the string it writes. `None` if it isn't one.
    pub(super) fn write_call(
        &mut self,
        def_id: DefId,
        generic_args: ty::GenericArgsRef<'tcx>,
        args: &[ExprId],
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> R<Option<Expr>> {
        let i = match self.formatter_param(def_id) {
            Some(i) => i,
            // `Write::write_str(f, s)` of a `Formatter`: `f` is the first.
            None if self.recognition().fmt_write_on_formatter(def_id, generic_args) => 0,
            None => return Ok(None),
        };
        let Some((var, name)) = self.writing.writer.clone() else {
            return Err(self.unsupported(span, "a `Formatter` outside a `fmt`"));
        };
        if var.is_none() || self.formatter_var(args[i]) != var {
            return Err(self.unsupported(self.thir[args[i]].span, "this `Formatter`"));
        }
        let others: Vec<ExprId> = args
            .iter()
            .enumerate()
            .filter(|&(j, _)| j != i)
            .map(|(_, &a)| a)
            .collect();
        let operation = self
            .recognition()
            .write_call(def_id, self.krate.fns.contains_key(&def_id));
        // Its `&dyn Debug` arguments show as pretty as this writer is (ADR 0137).
        let pretty = self.writer_pretty();
        let outer = std::mem::replace(&mut self.writing.dyn_debug, pretty.clone());
        let values = self.operands(&others, out);
        self.writing.dyn_debug = outer;
        let mut values = values?;
        let written = match operation {
            // `write!(f, ..)` is `f.write_fmt(format_args!(..))`, which is a string (ADR 0034).
            WriteCall::Text => values.remove(0),
            // A string, padded and cut as the `Formatter`'s options say, as a
            // `str`'s `Display` is (ADR 0143).
            WriteCall::Pad => {
                let text = values.remove(0);
                let str_ty = self.tcx.types.str_;
                self.formatted(text.clone(), (Std::FmtDisplay, str_ty), &pretty)
                    .unwrap_or(text)
            }
            WriteCall::Display => {
                let ty = generic_args.type_at(0);
                self.display_string_with(values.remove(0), ty, span, &pretty)?
            }
            WriteCall::Debug => {
                let ty = generic_args.type_at(0);
                self.debug_string_with(values.remove(0), ty, span, &pretty)?
            }
            // The crate's impl, given this `Formatter`'s options, or std's of a
            // number, padded by them as it runs (ADR 0185).
            WriteCall::OtherFmt => {
                let ty = generic_args.type_at(0).peel_refs();
                let trait_id = self.tcx.trait_of_assoc(def_id).expect("a trait's `fmt`");
                let std_radix = (!self.has_user_impl(trait_id, ty))
                    .then(|| Num::of(ty).filter(|n| !n.float()).zip(self.radix_trait(trait_id)))
                    .flatten();
                // A generic `T`'s: its dictionary's (ADR 0174).
                if self.is_unknown(ty) {
                    let value = values.remove(0);
                    self.other_fmt_dictionary(trait_id, ty, value, &pretty, span)?
                } else if let Some((num, radix)) = std_radix {
                    self.radix_text(values.remove(0), num, radix, &pretty)
                } else {
                    if !self.has_user_impl(trait_id, ty) {
                        let path = self.tcx.def_path_str(def_id);
                        return Err(self.unsupported(span, &format!("calling `{path}` of a `{ty}`")));
                    }
                    let args = self.tcx.mk_args(&[self.tcx.erase_and_anonymize_regions(ty).into()]);
                    self.writer_call(def_id, args, values.remove(0), &pretty, span)?
                }
            }
            // More than five fields: arrays of their names and strings.
            WriteCall::StructFields => {
                self.runtime.insert(Helper::DebugFields);
                values.extend(self.writing.given.is_some().then(|| pretty.alternate()));
                Expr::call(Expr::var("$debugFields"), values)
            }
            WriteCall::TupleFields => {
                let (type_name, items) = (values.remove(0), values.remove(0));
                let items = self.once(items, out);
                let joined = Expr::call(Expr::member(items.clone(), "join"), vec![Expr::str(", ")]);
                let plain = join(vec![type_name.clone(), Expr::str("("), joined, Expr::str(")")]);
                let open = join(vec![type_name, Expr::str("(")]);
                self.pretty_or_parts(&pretty, plain, open, items, Expr::str(")"))
            }
            // A derived `Debug`'s body (ADR 0060): its fields are strings
            // already, each a `&dyn Debug` (`debug_dyn`).
            WriteCall::Struct => {
                let type_name = values.remove(0);
                let mut parts = vec![type_name.clone(), Expr::str(" { ")];
                let mut shown = Vec::new();
                let mut first = true;
                while values.len() >= 2 {
                    let (field, value) = (values.remove(0), values.remove(0));
                    let value = self.once(value, out);
                    if !first {
                        parts.push(Expr::str(", "));
                    }
                    first = false;
                    parts.extend([field.clone(), Expr::str(": "), value.clone()]);
                    shown.push(join(vec![field, Expr::str(": "), value]));
                }
                parts.push(Expr::str(" }"));
                let open = join(vec![type_name, Expr::str(" {")]);
                self.pretty_or_parts(&pretty, join(parts), open, Expr::array(shown), Expr::str("}"))
            }
            WriteCall::Tuple => {
                let type_name = values.remove(0);
                let mut parts = vec![type_name.clone(), Expr::str("(")];
                let mut shown = Vec::new();
                for (i, value) in values.drain(..).enumerate() {
                    let value = self.once(value, out);
                    if i > 0 {
                        parts.push(Expr::str(", "));
                    }
                    parts.push(value.clone());
                    shown.push(value);
                }
                parts.push(Expr::str(")"));
                let open = join(vec![type_name, Expr::str("(")]);
                self.pretty_or_parts(&pretty, join(parts), open, Expr::array(shown), Expr::str(")"))
            }
            // Another writer, given this one's `Formatter`: its options.
            WriteCall::Function => {
                values.extend(self.writing.given.clone());
                values.extend(self.evidence_args(def_id, generic_args, span)?);
                Expr::call(self.fn_ref(def_id), values)
            }
            WriteCall::Trait => {
                values.extend(self.writing.given.clone());
                match self.trait_call(def_id, generic_args, values, span, out)? {
                    Some(call) => call,
                    None => return Err(self.unsupported(span, "this call")),
                }
            }
            _ => {
                let what = format!("calling `{}`", self.tcx.def_path_str(def_id));
                return Err(self.unsupported(span, &what));
            }
        };
        let target = Expr::var(&name);
        let js_span = self.js_span(span);
        let pieces = others.iter().any(|&other| self.formats_may_fail(other));
        append_written(&target, written, pieces, js_span, out);
        Ok(Some(Expr::undefined()))
    }

    /// `f.debug_struct("P").field("x", &self.x).finish()`, and the other
    /// builders' `finish` (ADR 0136): what the chain writes, as one string,
    /// as a derived `Debug`'s is: `P { x: 1 }`. A builder kept in a variable,
    /// which its calls write to in turn, is an error.
    pub(super) fn debug_builder(
        &mut self,
        def_id: DefId,
        args: &[ExprId],
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> R<Option<Expr>> {
        let finish = self.tcx.item_name(def_id);
        let non_exhaustive = match finish.as_str() {
            "finish" => false,
            "finish_non_exhaustive" => true,
            _ => return Ok(None),
        };
        let Some(owner) = self.tcx.inherent_impl_of_assoc(def_id) else {
            return Ok(None);
        };
        let owner = self.tcx.type_of(owner).instantiate_identity().skip_normalization();
        let ty::Adt(owner, _) = owner.kind() else {
            return Ok(None);
        };
        let kind = match self.tcx.def_path_str(owner.did()).as_str() {
            "std::fmt::DebugStruct" => "DebugStruct",
            "std::fmt::DebugTuple" => "DebugTuple",
            "std::fmt::DebugList" => "DebugList",
            "std::fmt::DebugSet" => "DebugSet",
            "std::fmt::DebugMap" => "DebugMap",
            _ => return Ok(None),
        };
        // From `finish`'s receiver back to the `Formatter`'s method that made it.
        let mut steps: Vec<(String, Vec<ExprId>, ty::GenericArgsRef<'tcx>)> = Vec::new();
        let mut at = args[0];
        let start = loop {
            let mut e = self.strip(at);
            while let ExprKind::Borrow { arg, .. } | ExprKind::Deref { arg } = self.thir[e].kind {
                e = self.strip(arg);
            }
            let ExprKind::Call { fun, ref args, .. } = self.thir[e].kind else {
                return Err(self.unsupported(span, "a `Debug` builder kept in a variable"));
            };
            let Some((id, generic_args)) = fn_def(self.thir[self.strip(fun)].ty) else {
                return Err(self.unsupported(span, "this `Debug` builder"));
            };
            let name = self.tcx.item_name(id).to_string();
            if name.starts_with("debug_") {
                break (args.to_vec(), id);
            }
            at = args[0];
            steps.push((name, args.to_vec(), generic_args));
        };
        steps.reverse();
        let (start_args, _) = start;
        let Some((var, written_to)) = self.writing.writer.clone() else {
            return Err(self.unsupported(span, "a `Formatter` outside a `fmt`"));
        };
        if var.is_none() || self.formatter_var(start_args[0]) != var {
            return Err(self.unsupported(span, "this `Formatter`"));
        }
        // Each part, in the order Rust writes them: a name, then one string for
        // each field or entry, or an array of them for `entries(items)`.
        let mut list = start_args[1..].to_vec();
        for (_, args, _) in &steps {
            list.extend(args[1..].iter().copied());
        }
        // As pretty as this writer is, `&dyn Debug`s and entries too (ADR 0137).
        let pretty = self.writer_pretty();
        let outer = std::mem::replace(&mut self.writing.dyn_debug, pretty.clone());
        let values = self.operands(&list, out);
        self.writing.dyn_debug = outer;
        let mut values = values?.into_iter();
        let type_name = match kind {
            "DebugStruct" | "DebugTuple" => Some(values.next().expect("a type's name")),
            _ => None,
        };
        #[derive(Clone)]
        enum Part {
            One(Expr),
            Many(Expr),
        }
        let mut parts = Vec::new();
        for (name, args, generic_args) in &steps {
            // A `&dyn Debug` is the string it shows already (ADR 0060).
            let debug_of = |this: &mut Self, value: Expr, at: ExprId| -> R<Expr> {
                match this.is_dyn_debug(this.thir[at].ty) {
                    true => Ok(value),
                    false => this.debug_string(value, this.thir[at].ty, span),
                }
            };
            let part = match (kind, name.as_str()) {
                ("DebugStruct", "field") => {
                    let (field, value) = (values.next().expect("a name"), values.next().expect("a value"));
                    let shown = debug_of(self, value, args[2])?;
                    Part::One(join(vec![field, Expr::str(": "), shown]))
                }
                ("DebugTuple", "field") | ("DebugList" | "DebugSet", "entry") => {
                    let value = values.next().expect("a value");
                    Part::One(debug_of(self, value, args[1])?)
                }
                ("DebugMap", "entry") => {
                    let (key, value) = (values.next().expect("a key"), values.next().expect("a value"));
                    let key = debug_of(self, key, args[1])?;
                    let value = debug_of(self, value, args[2])?;
                    Part::One(join(vec![key, Expr::str(": "), value]))
                }
                ("DebugList" | "DebugSet" | "DebugMap", "entries") => {
                    let items = values.next().expect("the entries");
                    let tys: Vec<Ty<'tcx>> = generic_args.types().collect();
                    let iterable = *tys.last().expect("the entries' type");
                    let items = self.iter_source(items, iterable, span, out)?;
                    let shown = if kind == "DebugMap" {
                        let pair = Expr::var("entry");
                        let key =
                            self.debug_string_with(Expr::index(pair.clone(), Expr::int(0)), tys[0], span, &pretty)?;
                        let value = self.debug_string_with(Expr::index(pair, Expr::int(1)), tys[1], span, &pretty)?;
                        join(vec![key, Expr::str(": "), value])
                    } else {
                        self.debug_string_with(Expr::var("entry"), tys[0], span, &pretty)?
                    };
                    let each = Expr::arrow(
                        vec!["entry".into()],
                        vec![StmtKind::Return(Some(shown)).at(js::Span::NONE)],
                    );
                    Part::Many(Expr::call(Expr::member(Expr::var("Array"), "from"), vec![items, each]))
                }
                _ => return Err(self.unsupported(span, &format!("a `Debug` builder's `{name}`"))),
            };
            parts.push(part);
        }
        // The parts, comma separated: in place when each is one string, else
        // an array of them joined, as `entries` may give none.
        let separated = |parts: Vec<Part>, extra: Option<&str>| -> Expr {
            let mut parts = parts;
            if let Some(extra) = extra {
                parts.push(Part::One(Expr::str(extra)));
            }
            if parts.iter().all(|p| matches!(p, Part::One(_))) {
                let mut pieces = Vec::new();
                for (i, part) in parts.into_iter().enumerate() {
                    if i > 0 {
                        pieces.push(Expr::str(", "));
                    }
                    if let Part::One(e) = part {
                        pieces.push(e);
                    }
                }
                return join(pieces);
            }
            let mut arrays: Vec<Expr> = Vec::new();
            let mut ones: Vec<Expr> = Vec::new();
            for part in parts {
                match part {
                    Part::One(e) => ones.push(e),
                    Part::Many(e) => {
                        if !ones.is_empty() {
                            arrays.push(Expr::array(std::mem::take(&mut ones)));
                        }
                        arrays.push(e);
                    }
                }
            }
            if !ones.is_empty() {
                arrays.push(Expr::array(ones));
            }
            let mut arrays = arrays.into_iter();
            let first = arrays.next().expect("a part");
            let all = arrays.fold(first, |all, next| Expr::call(Expr::member(all, "concat"), vec![next]));
            Expr::call(Expr::member(all, "join"), vec![Expr::str(", ")])
        };
        // Pretty: each part on a line of its own, an array of them, as `entries`
        // may give none (ADR 0137).
        let array = {
            let mut arrays: Vec<Expr> = Vec::new();
            let mut ones: Vec<Expr> = Vec::new();
            for part in parts.clone() {
                match part {
                    Part::One(e) => ones.push(e),
                    Part::Many(e) => {
                        if !ones.is_empty() {
                            arrays.push(Expr::array(std::mem::take(&mut ones)));
                        }
                        arrays.push(e);
                    }
                }
            }
            if !ones.is_empty() || arrays.is_empty() {
                arrays.push(Expr::array(ones));
            }
            let mut arrays = arrays.into_iter();
            let first = arrays.next().expect("a part");
            arrays.fold(first, |all, next| Expr::call(Expr::member(all, "concat"), vec![next]))
        };
        let pretty_open = match kind {
            "DebugStruct" => type_name.clone().map(|name| join(vec![name, Expr::str(" {")])),
            "DebugTuple" => type_name.clone().map(|name| join(vec![name, Expr::str("(")])),
            "DebugList" => Some(Expr::str("[")),
            _ => Some(Expr::str("{")),
        };
        let pretty_close = match kind {
            "DebugTuple" => ")",
            "DebugList" => "]",
            _ => "}",
        };
        let has_parts = !parts.is_empty();
        let rest = non_exhaustive.then_some("..");
        let empty = parts.is_empty() && !non_exhaustive;
        let written = match kind {
            "DebugStruct" | "DebugTuple" if empty => type_name.expect("a type's name"),
            "DebugStruct" => {
                let inside = separated(parts, rest);
                join(vec![
                    type_name.expect("a type's name"),
                    Expr::str(" { "),
                    inside,
                    Expr::str(" }"),
                ])
            }
            "DebugTuple" => {
                let inside = separated(parts, rest);
                join(vec![
                    type_name.expect("a type's name"),
                    Expr::str("("),
                    inside,
                    Expr::str(")"),
                ])
            }
            _ => {
                let (open, close) = if kind == "DebugList" { ("[", "]") } else { ("{", "}") };
                if empty {
                    Expr::str(format!("{open}{close}"))
                } else {
                    let inside = separated(parts, rest);
                    join(vec![Expr::str(open), inside, Expr::str(close)])
                }
            }
        };
        let written = match (&pretty, has_parts, pretty_open) {
            (Pretty::Plain, _, _) | (_, false, _) | (_, _, None) => written,
            (_, true, Some(open)) => {
                self.runtime.insert(Helper::Pretty);
                let mut list = vec![open, array, Expr::str(pretty_close)];
                if non_exhaustive {
                    list.push(Expr::bool(true));
                }
                let pretty_form = Expr::call(Expr::var("$pretty"), list);
                pretty.choose(pretty_form, written)
            }
        };
        let target = Expr::var(&written_to);
        let js_span = self.js_span(span);
        out.push(StmtKind::Assign(target.clone(), Expr::bin(Op::Add, target, written)).at(js_span));
        Ok(Some(Expr::undefined()))
    }

    /// The JS string a place is, if it's the `Formatter` being written, `*f`:
    /// given where a writer goes, it's boxed, and taken back (ADR 0180).
    pub(super) fn formatter_text(&self, place: ExprId) -> Option<String> {
        let (writer, text) = self.writing.writer.as_ref()?;
        (writer.is_some() && self.formatter_var(place) == *writer).then(|| text.clone())
    }

    /// The variable a `Formatter` argument is: `f`, or `&mut *f`.
    fn formatter_var(&self, e: ExprId) -> Option<thir::LocalVarId> {
        match self.thir[self.strip(e)].kind {
            ExprKind::Borrow { arg, .. } | ExprKind::Deref { arg } => self.formatter_var(arg),
            ExprKind::VarRef { id } | ExprKind::UpvarRef { var_hir_id: id, .. } => Some(id),
            _ => None,
        }
    }

    /// `{}` of a `ty` value: the string itself, `String(x)`, `$displayF64(x)`,
    /// a hand-written `fmt`'s string, or `TDisplay.fmt(x)` in generic code.
    pub(super) fn display_string(&mut self, value: Expr, ty: Ty<'tcx>, span: Span) -> R<Expr> {
        self.display_string_with(value, ty, span, &Pretty::Plain)
    }

    /// `display_string`, of a `{:#}` or not, which only the crate's own
    /// `fmt` can tell (ADR 0137).
    pub(super) fn display_string_with(&mut self, value: Expr, ty: Ty<'tcx>, span: Span, pretty: &Pretty) -> R<Expr> {
        let (value, ty) = self.through_refs(value, ty);
        let ty = self.shown_type(ty);
        if let Some(shown) = self.formatted(value.clone(), (Std::FmtDisplay, ty), pretty) {
            return Ok(shown);
        }
        self.can_apply(ty, pretty, span)?;
        // A UTF-8 error's message, a `FromUtf8Error`'s its `Utf8Error`'s (ADR 0172).
        if let Some(owned) = self.recognition().utf8_error(ty) {
            self.runtime.insert(Helper::Utf8);
            let error = if owned { Expr::member(value, "error") } else { value };
            return Ok(Expr::call(Expr::var("$utf8ErrorMessage"), vec![error]));
        }
        if self.recognition().is_cow_str(ty) {
            return Ok(Expr::member(value, "_0"));
        }
        // A path's `display()`, its text (ADR 0173).
        if self.recognition().is_path_like(ty) {
            return Ok(value);
        }
        if super::recognition::is_try_from_slice_error(self.tcx, ty) {
            return Ok(Expr::str("could not convert slice to array"));
        }
        // `format_args!`'s text (ADR 0034), which its `Display` writes as it
        // is, whatever width the `Formatter` has, as strum's derive asks.
        if self.is_lang_adt(ty, LangItem::FormatArguments) {
            return Ok(value);
        }
        // A `Wrapping` shows its number, as both its `Display` and its
        // `Debug` do (ADR 0175).
        if let Some(inner) = self.recognition().wrapping_of(ty) {
            return self.display_string_with(Expr::index(value, Expr::int(0)), inner, span, pretty);
        }
        if self.is_string_like(ty) || self.is_parse_error(ty) {
            return Ok(value);
        }
        if self.is_json_error(ty) {
            self.runtime.insert(Helper::JsonError);
            return Ok(Expr::call(Expr::var("$displayJsonError"), vec![value]));
        }
        // A `Value`'s JSON, or a `Number`'s (ADR 0083).
        if let Some(shown) = self.json_value_display(value.clone(), ty, false) {
            return Ok(shown);
        }
        if ty.is_bool() || Num::of(ty).is_some_and(|n| !n.float()) {
            return Ok(shown_number(value));
        }
        if Num::of(ty) == Some(Num::F64) {
            self.runtime.insert(Helper::DisplayF64);
            return Ok(Expr::call(Expr::var("$displayF64"), vec![value]));
        }
        if Num::of(ty) == Some(Num::F32) {
            self.runtime.insert(Helper::DisplayF32);
            return Ok(Expr::call(Expr::var("$displayF32"), vec![value]));
        }
        let display = self.display_trait();
        let options = self.options_arg(pretty);
        if let Some(shown) = self.dyn_written(value.clone(), (ty, display), options, span)? {
            return Ok(shown);
        }
        if self.is_unknown(ty) {
            let tr = ty::TraitRef::new(self.tcx, display, [ty]);
            let dictionary = self.evidence_for(tr).ok_or_else(|| self.no_evidence(span, tr))?;
            let mut list = vec![value];
            list.extend(self.options_arg(pretty));
            return Ok(Expr::call(Expr::member(dictionary, "fmt"), list));
        }
        if self.has_user_impl(display, ty) {
            let fmt = self.tcx.associated_item_def_ids(display)[0];
            let args = self.tcx.mk_args(&[self.tcx.erase_and_anonymize_regions(ty).into()]);
            return self.writer_call(fmt, args, value, pretty, span);
        }
        Err(self.unsupported(span, &format!("`{{}}` of a `{ty}`")))
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
            Some(true) => Expr::bin(Op::Add, Expr::str(radix.prefix()), digits),
            Some(false) => digits,
            None => {
                let prefix = Expr::cond(pretty.alternate(), Expr::str(radix.prefix()), Expr::str(""));
                Expr::bin(Op::Add, prefix, digits)
            }
        };
        match pretty {
            Pretty::When(options) if self.krate.format_options => {
                self.runtime.insert(Helper::Formatted);
                Expr::call(Expr::var("$formatted"), vec![text, options.clone(), Expr::bool(true)])
            }
            _ => text,
        }
    }

    /// The radix std's `LowerHex`, `UpperHex`, `Octal` or `Binary` shows.
    pub(super) fn radix_trait(&self, trait_id: DefId) -> Option<Radix> {
        [Radix::LowerHex, Radix::UpperHex, Radix::Octal, Radix::Binary]
            .into_iter()
            .find(|&radix| self.recognition().other_fmt_trait(Std::FmtRadix(radix)) == Some(trait_id))
    }

    /// `write!(w, ..)` or `w.write_char(c)` of a writer of the crate's that
    /// keeps std's: its own `write_str`, given the text whole, where Rust's
    /// gives it a piece at a time (ADR 0166). Its text is Rust's; how many
    /// calls carry it isn't.
    pub(super) fn user_write(
        &mut self,
        def_id: DefId,
        args: &[ExprId],
        generic_args: ty::GenericArgsRef<'tcx>,
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> R<Expr> {
        let trait_id = self.tcx.trait_of_assoc(def_id).expect("`fmt::Write`'s method");
        let write_str = trait_method(self.tcx, trait_id, "write_str");
        let values = self.operands(args, out)?;
        self.trait_call(write_str, generic_args, values, span, out)?
            .ok_or_else(|| self.unsupported(span, "this writer's `write_str`"))
    }

    /// The type `ty` shows: what a `Box`, an `Rc` or a `RefCell`'s
    /// `borrow()` holds, as they are it in JS, and `ty` itself otherwise.
    pub(super) fn shown_type(&self, ty: Ty<'tcx>) -> Ty<'tcx> {
        match ty.kind() {
            ty::Adt(_, args) if self.shows_inside(ty) => args.types().next().expect("what it holds").peel_refs(),
            _ => ty,
        }
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
    /// (ADR 0060)? A derived `Debug` hands its fields to the formatter so.
    pub(super) fn is_dyn_debug(&self, ty: Ty<'tcx>) -> bool {
        matches!(ty.peel_refs().kind(), ty::Dynamic(traits, ..)
            if traits.principal_def_id() == Some(self.debug_trait()))
    }

    /// `{:?}` of a `ty` value (ADR 0060), as Rust shows it: `Some(1)`,
    /// `(1, "a")`, `[1.0, 2.5]`, a call of a `Debug` impl of the crate's own,
    /// derived or not, or `TDebug.fmt(x)` in generic code.
    pub(super) fn debug_string(&mut self, value: Expr, ty: Ty<'tcx>, span: Span) -> R<Expr> {
        self.debug_string_with(value, ty, span, &Pretty::Plain)
    }

    /// `debug_string`, plain or pretty, `{:#?}` (ADR 0137): a part of a
    /// value is shown as the value is.
    pub(super) fn debug_string_with(&mut self, value: Expr, ty: Ty<'tcx>, span: Span, pretty: &Pretty) -> R<Expr> {
        let (value, ty) = self.through_refs(value, ty);
        // A `fmt::Result`: `undefined`, `Ok`, or the `fmt::Error` it caught
        // (ADR 0187), and a `fmt::Error`, which holds nothing.
        if self.is_fmt_result(ty) {
            let ok = Expr::str("Ok(())");
            return Ok(match value.kind {
                js::ExprKind::Undefined => ok,
                _ => Expr::cond(Expr::bin(Op::Eq, value, Expr::undefined()), ok, Expr::str("Err(Error)")),
            });
        }
        if self.recognition().is_fmt_error(ty) {
            return Ok(Expr::str("Error"));
        }
        if super::recognition::is_try_from_slice_error(self.tcx, ty) {
            return Ok(Expr::str("TryFromSliceError(())"));
        }
        // A `Duration`'s, in its largest whole unit, `1.5s` (ADR 0188). With
        // options, std rounds and pads it by its own rules: not yet.
        if super::recognition::is_duration_ty(ty) {
            if self.writing.options.is_some() || matches!(pretty, Pretty::Given(..)) {
                return Err(self.unsupported(span, "options for a `Duration`'s `{:?}`"));
            }
            self.runtime.insert(Helper::Duration);
            return Ok(Expr::call(Expr::var("$debugDuration"), vec![value]));
        }
        // A part of a `{:?}` given its options (ADR 0058): a leaf applies them.
        if self.writing.options.is_some() && self.is_debug_leaf(ty) {
            let options = self.writing.options.take().expect("checked");
            let shown = self.format_value(
                value,
                (Std::FmtDebug, ty),
                options.spec,
                (options.width.clone(), options.precision.clone()),
                span,
            );
            self.writing.options = Some(options);
            return shown;
        }
        if let Some(shown) = self.formatted(value.clone(), (Std::FmtDebug, ty), pretty) {
            return Ok(shown);
        }
        self.can_apply(ty, pretty, span)?;
        // serde_json's own, `Object {"a": Number(1)}` (ADR 0083).
        if let Some(shown) = self.json_value_debug(value.clone(), ty, pretty) {
            return Ok(shown);
        }
        let std = |item: StdItem| self.is_std_type(ty, item);
        let num = Num::of(ty);
        if self.is_dyn_debug(ty) {
            return Ok(value);
        }
        // An enum with no variants, `Infallible`: no value of it exists, so
        // this never runs, as Rust's `match *self {}` doesn't (ADR 0161).
        if let ty::Adt(adt, _) = ty.kind()
            && adt.is_enum()
            && adt.variants().is_empty()
        {
            return Ok(Expr::str(""));
        }
        // A channel's errors (ADR 0142), as std shows them: `TryRecvError` is
        // its variant's name already.
        if let Some(error) = self.recognition().channel_error(ty) {
            return Ok(match error {
                ChannelError::Recv => Expr::str("RecvError"),
                ChannelError::TryRecv => value,
                ChannelError::Send => Expr::str("SendError { .. }"),
            });
        }
        let (shown, options) = (self.shown_type(ty), self.options_arg(pretty));
        if let Some(shown) = self.dyn_written(value.clone(), (shown, self.debug_trait()), options, span)? {
            return Ok(shown);
        }
        if num == Some(Num::F64) {
            self.runtime.insert(Helper::DebugF64);
            return Ok(Expr::call(Expr::var("$debugF64"), vec![value]));
        }
        if num == Some(Num::F32) {
            self.runtime.insert(Helper::DebugF32);
            return Ok(Expr::call(Expr::var("$debugF32"), vec![value]));
        }
        if num.is_some() || ty.is_bool() {
            return Ok(shown_number(value));
        }
        if ty.is_unit() {
            return Ok(Expr::str("()"));
        }
        if ty.is_char() {
            self.runtime.insert(Helper::DebugStr);
            return Ok(Expr::call(Expr::var("$debugStr"), vec![value, Expr::str("'")]));
        }
        if self.is_string_like(ty) {
            self.runtime.insert(Helper::DebugStr);
            return Ok(Expr::call(Expr::var("$debugStr"), vec![value]));
        }
        // What a JS function threw, as JS shows it: `SyntaxError: ..`.
        if self.recognition().is_js_error(ty) {
            return Ok(Expr::call(Expr::var("String"), vec![value]));
        }
        if self.is_json_error(ty) {
            self.runtime.insert(Helper::JsonError);
            return Ok(Expr::call(Expr::var("$debugJsonError"), vec![value]));
        }
        // Their derived `Debug`s (ADR 0172).
        if let Some(owned) = self.recognition().utf8_error(ty) {
            self.runtime.insert(Helper::Utf8);
            let mut list = vec![value];
            if owned {
                list.push(Expr::bool(true));
            }
            return Ok(Expr::call(Expr::var("$debugUtf8Error"), list));
        }
        if let Some(inner) = self.recognition().wrapping_of(ty) {
            return self.debug_string_with(Expr::index(value, Expr::int(0)), inner, span, pretty);
        }
        // A path shows its text as a string does (ADR 0173).
        if self.recognition().is_path_like(ty) {
            self.runtime.insert(Helper::DebugStr);
            return Ok(Expr::call(Expr::var("$debugStr"), vec![value]));
        }
        // A `Cow` shows its text, borrowed or owned.
        if self.recognition().is_cow_str(ty) {
            self.runtime.insert(Helper::DebugStr);
            return Ok(Expr::call(Expr::var("$debugStr"), vec![Expr::member(value, "_0")]));
        }
        // A parse error is its message (ADR 0063), which says its kind.
        if self.is_parse_error(ty) {
            let ty::Adt(adt, _) = ty.kind() else {
                unreachable!("a struct")
            };
            let name = self.tcx.item_name(adt.did());
            self.runtime.insert(Helper::DebugParseError);
            return Ok(Expr::call(
                Expr::var("$debugParseError"),
                vec![value, Expr::str(name.as_str())],
            ));
        }
        if self.is_lang_adt(ty, LangItem::OrderingEnum) {
            let names = ["Less", "Equal", "Greater"];
            if let Some(n) = value.as_int().filter(|n| (-1..=1).contains(n)) {
                return Ok(Expr::str(names[(n + 1) as usize]));
            }
            let names = Expr::array(names.into_iter().map(Expr::str).collect());
            return Ok(Expr::index(names, Expr::bin(Op::Add, value, Expr::int(1))));
        }
        let debug = self.debug_trait();
        if self.is_unknown(ty) {
            let tr = ty::TraitRef::new(self.tcx, debug, [ty]);
            let dictionary = self.evidence_for(tr).ok_or_else(|| self.no_evidence(span, tr))?;
            let mut list = vec![value];
            list.extend(self.options_arg(pretty));
            return Ok(Expr::call(Expr::member(dictionary, "fmt"), list));
        }
        // A fieldless enum is its variant's name (ADR 0013), which is what a
        // derived `Debug` shows.
        if let ty::Adt(adt, _) = ty.kind()
            && representation::is_fieldless_enum(*adt)
            && self.is_derived_impl(debug, ty)
        {
            return Ok(value);
        }
        // The crate's own, hand-written or derived.
        if self.has_user_impl(debug, ty) {
            let fmt = self.tcx.associated_item_def_ids(debug)[0];
            let args = self.args_of(debug, ty);
            return self.writer_call(fmt, args, value, pretty, span);
        }
        if self.range_kind(ty).is_some() {
            return self.range_debug(value, ty, span, pretty);
        }
        // `PhantomData<u8>`, its type's name as `type_name` gives it (ADR 0132).
        if let ty::Adt(_, args) = ty.kind()
            && self.is_lang_adt(ty, LangItem::PhantomData)
            && !args.type_at(0).has_param()
        {
            let of = self
                .tcx
                .normalize_erasing_regions(self.typing_env, ty::Unnormalized::new_wip(args.type_at(0)));
            let name = rustc_const_eval::util::type_name(self.tcx, of);
            return Ok(Expr::str(format!("PhantomData<{name}>")));
        }
        match ty.kind() {
            // A constant is known: `Some(1.0)` is `"Some(1.0)"`, with no test.
            _ if let Some(inner) = self.option_of(ty)
                && value.is_constant()
                && !self.boxed_payload(inner) =>
            {
                if matches!(value.kind, js::ExprKind::Undefined | js::ExprKind::Null) {
                    return Ok(Expr::str("None"));
                }
                let shown = self.debug_string_with(value, inner, span, pretty)?;
                let plain = join(vec![Expr::str("Some("), shown.clone(), Expr::str(")")]);
                Ok(self.pretty_or(pretty, plain, "Some(", Expr::array(vec![shown]), ")"))
            }
            _ if let Some(inner) = self.option_of(ty) => {
                let inside = if self.boxed_payload(inner) {
                    self.some_value(Expr::var("value"))
                } else {
                    Expr::var("value")
                };
                let shown = self.debug_string_with(inside, inner, span, pretty)?;
                let plain = join(vec![Expr::str("Some("), shown.clone(), Expr::str(")")]);
                let some = self.pretty_or(pretty, plain, "Some(", Expr::array(vec![shown]), ")");
                let none = Expr::bin(Op::LooseEq, Expr::var("value"), Expr::null());
                let f = Expr::arrow(
                    vec!["value".into()],
                    vec![StmtKind::Return(Some(Expr::cond(none, Expr::str("None"), some))).at(js::Span::NONE)],
                );
                Ok(self.applied(f, value))
            }
            ty::Tuple(tys) => {
                let tys: Vec<Ty<'tcx>> = tys.to_vec();
                let mut parts = vec![Expr::str("(")];
                let mut items = Vec::new();
                for (i, &t) in tys.iter().enumerate() {
                    if i > 0 {
                        parts.push(Expr::str(", "));
                    }
                    let item =
                        self.debug_string_with(Expr::index(Expr::var("tuple"), Expr::int(i as i128)), t, span, pretty)?;
                    parts.push(item.clone());
                    items.push(item);
                }
                if tys.len() == 1 {
                    parts.push(Expr::str(","));
                }
                parts.push(Expr::str(")"));
                let shown = self.pretty_or(pretty, join(parts), "(", Expr::array(items), ")");
                let f = Expr::arrow(
                    vec!["tuple".into()],
                    vec![StmtKind::Return(Some(shown)).at(js::Span::NONE)],
                );
                Ok(self.applied(f, value))
            }
            ty::Array(item, _) | ty::Slice(item) => self.debug_items(value, *item, "[", "]", span, pretty),
            ty::Adt(_, args) if self.is_vec_like(ty) => {
                self.debug_items(value, args.type_at(0), "[", "]", span, pretty)
            }
            ty::Adt(_, args) if self.is_reverse(ty) => {
                let shown = self.debug_string_with(Expr::index(value, Expr::int(0)), args.type_at(0), span, pretty)?;
                let plain = join(vec![Expr::str("Reverse("), shown.clone(), Expr::str(")")]);
                Ok(self.pretty_or(pretty, plain, "Reverse(", Expr::array(vec![shown]), ")"))
            }
            ty::Adt(_, args) if self.shows_inside(ty) => {
                self.debug_string_with(value, args.types().next().expect("what it holds"), span, pretty)
            }
            // An atomic shows what it holds.
            ty::Adt(_, args) if std(StdItem::Atomic) => {
                self.debug_string_with(Expr::member(value, "value"), args.type_at(0), span, pretty)
            }
            ty::Adt(_, args) if std(StdItem::Cell) || std(StdItem::RefCell) => {
                let name = if std(StdItem::Cell) { "Cell" } else { "RefCell" };
                let shown = self.debug_string_with(Expr::member(value, "value"), args.type_at(0), span, pretty)?;
                let plain = join(vec![
                    Expr::str(format!("{name} {{ value: ")),
                    shown.clone(),
                    Expr::str(" }"),
                ]);
                let field = join(vec![Expr::str("value: "), shown]);
                Ok(self.pretty_or(pretty, plain, &format!("{name} {{"), Expr::array(vec![field]), "}"))
            }
            ty::Adt(_, args) if self.is_set(ty) => {
                let items = self.in_order_of(value, ty, span)?;
                self.debug_items(items, args.type_at(0), "{", "}", span, pretty)
            }
            ty::Adt(_, args) if self.is_map(ty) => {
                let value = self.in_order_of(value, ty, span)?;
                let (key, item) = (args.type_at(0), args.type_at(1));
                let key = self.debug_string_with(Expr::var("key"), key, span, pretty)?;
                let item = self.debug_string_with(Expr::var("value"), item, span, pretty)?;
                let pair = join(vec![key, Expr::str(": "), item]);
                let f = Expr::arrow(
                    vec![js::Pattern::Array(vec![Some("key".into()), Some("value".into())])],
                    vec![StmtKind::Return(Some(pair)).at(js::Span::NONE)],
                );
                let entries = Expr::call(Expr::member(Expr::var("Array"), "from"), vec![value]);
                let mapped = Expr::call(Expr::member(entries, "map"), vec![f]);
                let shown = Expr::call(Expr::member(mapped.clone(), "join"), vec![Expr::str(", ")]);
                let plain = join(vec![Expr::str("{"), shown, Expr::str("}")]);
                Ok(self.pretty_or(pretty, plain, "{", mapped, "}"))
            }
            ty::Adt(_, args) if std(StdItem::Result) => {
                let inside = || Expr::member(Expr::var("result"), "_0");
                let ok = self.debug_string_with(inside(), args.type_at(0), span, pretty)?;
                let err = self.debug_string_with(inside(), args.type_at(1), span, pretty)?;
                let f = Expr::arrow(
                    vec!["result".into()],
                    vec![
                        StmtKind::Return(Some(Expr::cond(
                            Expr::bin(Op::Eq, Expr::member(Expr::var("result"), "TAG"), Expr::str("Ok")),
                            self.pretty_or(
                                pretty,
                                join(vec![Expr::str("Ok("), ok.clone(), Expr::str(")")]),
                                "Ok(",
                                Expr::array(vec![ok]),
                                ")",
                            ),
                            self.pretty_or(
                                pretty,
                                join(vec![Expr::str("Err("), err.clone(), Expr::str(")")]),
                                "Err(",
                                Expr::array(vec![err]),
                                ")",
                            ),
                        )))
                        .at(js::Span::NONE),
                    ],
                );
                Ok(self.applied(f, value))
            }
            // Another crate's enum without fields, std's `IntErrorKind` say:
            // its variant's name, which it is (ADR 0013), as a derived `Debug`
            // shows it.
            ty::Adt(adt, _)
                if representation::is_fieldless_enum(*adt)
                    && !adt.did().is_local()
                    && !self.krate.foreign.in_library(adt.did())
                    && self.recognition().derives(std_item(self.tcx, StdItem::Debug), ty) =>
            {
                Ok(value)
            }
            _ => Err(self.unsupported(span, &format!("`{{:?}}` of a `{ty}`"))),
        }
    }

    /// A sequence's `{:?}`: `"[" + items.map((item) => ..).join(", ") + "]"`.
    fn debug_items(
        &mut self,
        items: Expr,
        item: Ty<'tcx>,
        open: &str,
        close: &str,
        span: Span,
        pretty: &Pretty,
    ) -> R<Expr> {
        let shown = self.debug_string_with(Expr::var("item"), item, span, pretty)?;
        let f = Expr::arrow(
            vec!["item".into()],
            vec![StmtKind::Return(Some(shown)).at(js::Span::NONE)],
        );
        let items = if self.is_map(item) || open == "{" {
            Expr::call(Expr::member(Expr::var("Array"), "from"), vec![items])
        } else {
            items
        };
        let mapped = Expr::call(Expr::member(items, "map"), vec![f]);
        let joined = Expr::call(Expr::member(mapped.clone(), "join"), vec![Expr::str(", ")]);
        let plain = join(vec![Expr::str(open), joined, Expr::str(close)]);
        Ok(self.pretty_or(pretty, plain, open, mapped, close))
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
        self.runtime.insert(Helper::Pretty);
        pretty.choose(Expr::call(Expr::var("$pretty"), vec![open, items, close]), plain)
    }

    /// `plain`, or, pretty, `$pretty(open, items, close)` of the same parts:
    /// each on a line of its own, indented, and ended by a comma (ADR 0137).
    fn pretty_or(&mut self, pretty: &Pretty, plain: Expr, open: &str, items: Expr, close: &str) -> Expr {
        self.pretty_or_parts(pretty, plain, Expr::str(open), items, Expr::str(close))
    }

    /// What a writer function is given after its value, where the crate's
    /// take options (ADRs 0058, 0137): none for a plain one, which takes
    /// nothing after it.
    pub(super) fn options_arg(&self, pretty: &Pretty) -> Option<Expr> {
        (self.writers_take_options() && !matches!(pretty, Pretty::Plain)).then(|| pretty.options())
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
        let instance = self
            .resolve_instance(method, args)?
            .filter(|i| self.is_rust_fn(i.def_id()))
            .ok_or_else(|| self.unsupported(span, "this implementation"))?;
        let evidence = self.evidence_args(instance.def_id(), instance.args, span)?;
        let mut values = vec![value];
        match self.options_arg(pretty) {
            Some(options) => values.push(options),
            None if self.writers_take_options() && !evidence.is_empty() => values.push(Expr::undefined()),
            None => {}
        }
        values.extend(evidence);
        Ok(Expr::call(self.fn_ref(instance.def_id()), values))
    }

    /// `Box<T>`, `Rc<T>`, `Ref<T>` and `RefMut<T>`: shown as their `T`,
    /// which is the value they are in JS (ADR 0023).
    fn shows_inside(&self, ty: Ty<'tcx>) -> bool {
        ty.is_box() || self.is_rc(ty) || self.is_guard(ty)
    }

    /// Does `{:?}` of a `ty` read the value more than once? An `Option`, a
    /// `Result` and a tuple are shown by their parts.
    pub(super) fn debug_reads_parts(&self, ty: Ty<'tcx>) -> bool {
        let ty = ty.peel_refs();
        if self.is_dyn_debug(ty) || self.is_unknown(ty) || self.has_user_impl(self.debug_trait(), ty) {
            return false;
        }
        match ty.kind() {
            ty::Tuple(tys) => !tys.is_empty(),
            ty::Adt(_, args) if ty.is_box() || self.is_rc(ty) => self.debug_reads_parts(args.type_at(0)),
            _ => self.option_of(ty).is_some() || self.is_std_type(ty, StdItem::Result),
        }
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
        let names: Vec<&str> = ["value", "options"].into_iter().take(params.len()).collect();
        if let js::ExprKind::Call(callee, args) = &shown.kind
            && matches!(callee.kind, js::ExprKind::Var(_))
            && args.len() == names.len()
            && args.iter().zip(&names).all(|(arg, name)| is_var(arg, name))
        {
            return Ok((**callee).clone());
        }
        Ok(Expr::arrow(
            params,
            vec![StmtKind::Return(Some(shown)).at(js::Span::NONE)],
        ))
    }
}

/// `f += s`, as a `return s`, for a body that writes once whichever way
/// it goes: `if (c) { f += a } else { f += b }` is `if (c) { return a } else
/// { return b }`. `None` if some way writes more, or does anything else.
fn as_returns(body: &[Stmt], name: &str) -> Option<Vec<Stmt>> {
    let [stmt] = body else { return None };
    let kind = match &stmt.kind {
        StmtKind::Assign(target, value) if is_var(target, name) => match &value.kind {
            js::ExprKind::Binary(Op::Add, lhs, rhs) if is_var(lhs, name) => StmtKind::Return(Some((**rhs).clone())),
            _ => return None,
        },
        StmtKind::If(test, then, Some(els)) => {
            StmtKind::If(test.clone(), as_returns(then, name)?, Some(as_returns(els, name)?))
        }
        _ => return None,
    };
    Some(vec![kind.at(stmt.span)])
}

/// Strings joined: text alone is a string, text and values a template
/// literal, \`Some(${x})\`, and values alone `a + b`. A part that's itself
/// strings joined is taken apart, so templates don't nest needlessly.
pub(super) fn join(parts: Vec<Expr>) -> Expr {
    let mut pieces: Vec<Expr> = Vec::new();
    for piece in parts.into_iter().flat_map(joined_pieces) {
        match (pieces.last_mut(), &piece.kind) {
            (
                Some(Expr {
                    kind: js::ExprKind::Str(before),
                    ..
                }),
                js::ExprKind::Str(after),
            ) => before.push_str(after),
            _ => pieces.push(piece),
        }
    }
    let is_text = |p: &Expr| matches!(p.kind, js::ExprKind::Str(_));
    if pieces.iter().all(is_text) || !pieces.iter().any(is_text) {
        return pieces
            .into_iter()
            .reduce(|a, b| Expr::bin(Op::Add, a, b))
            .unwrap_or_else(|| Expr::str(""));
    }
    let (mut texts, mut values) = (vec![String::new()], Vec::new());
    for piece in pieces {
        match piece.kind {
            js::ExprKind::Str(text) => texts.last_mut().expect("a text").push_str(&text),
            _ => {
                values.push(unstringed(piece));
                texts.push(String::new());
            }
        }
    }
    Expr::template(texts, values)
}

/// `String(x)` is `x` in a template, which makes it a string the same way.
fn unstringed(value: Expr) -> Expr {
    match value.kind {
        js::ExprKind::Call(ref callee, ref args)
            if matches!(&callee.kind, js::ExprKind::Var(name) if name == "String") && args.len() == 1 =>
        {
            args[0].clone()
        }
        _ => value,
    }
}

/// The pieces of strings joined: of a template, its texts and values; of
/// `"(" + a + ")"`, which starts with a string, so that each `+` in it
/// concatenates, its operands. Anything else is one piece.
fn joined_pieces(e: Expr) -> Vec<Expr> {
    if let js::ExprKind::Template(texts, values) = e.kind {
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
    let mut pieces = Vec::new();
    let mut rest = e;
    while let js::ExprKind::Binary(Op::Add, left, right) = rest.kind {
        pieces.push(*right);
        rest = *left;
    }
    let starts_with_string = matches!(rest.kind, js::ExprKind::Str(_));
    pieces.push(rest);
    pieces.reverse();
    if starts_with_string || pieces.len() == 1 {
        pieces
    } else {
        vec![
            pieces
                .into_iter()
                .reduce(|a, b| Expr::bin(Op::Add, a, b))
                .expect("a piece"),
        ]
    }
}

fn is_var(e: &Expr, name: &str) -> bool {
    matches!(&e.kind, js::ExprKind::Var(n) if n == name)
}

/// `String(n)`, or the text itself for a constant: `"4096"`.
fn shown_number(value: Expr) -> Expr {
    match value.as_int() {
        Some(n) => Expr::str(n.to_string()),
        None => Expr::call(Expr::var("String"), vec![value]),
    }
}

/// The functions an expression calls, each with its arguments: not those its
/// calls' arguments call, as a `write!` inside a `format_args!`, another
/// consumer, which takes its own `fmt::Error` (ADR 0187).
struct Called<'a, 'tcx> {
    thir: &'a thir::Thir<'tcx>,
    calls: Vec<(DefId, ty::GenericArgsRef<'tcx>)>,
}

impl<'a, 'tcx> Visitor<'a, 'tcx> for Called<'a, 'tcx> {
    fn thir(&self) -> &'a thir::Thir<'tcx> {
        self.thir
    }

    fn visit_expr(&mut self, expr: &'a thir::Expr<'tcx>) {
        match expr.kind {
            ExprKind::Call { fun, .. } => {
                if let Some((id, args)) = fn_def(self.thir[fun].ty) {
                    self.calls.push((id, args));
                }
            }
            _ => visit::walk_expr(self, expr),
        }
    }
}

/// `target += written`: piece by piece, `pieces`, where what it writes may
/// fail, so what came before it is written (ADR 0187).
pub(super) fn append_written(target: &Expr, written: Expr, pieces: bool, js_span: js::Span, out: &mut Vec<Stmt>) {
    let parts = if pieces && let js::ExprKind::Template(texts, values) = &written.kind {
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
        out.push(StmtKind::Assign(target.clone(), Expr::bin(Op::Add, target.clone(), part)).at(js_span));
    }
}
