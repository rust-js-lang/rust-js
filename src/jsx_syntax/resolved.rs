//! Which of a crate's components' props are built by their type's companion,
//! where the syntax pass couldn't tell (ADR 0213, amended; Codex's
//! prototype, rust-js-lang/vgpu-in-rust@143d724). Before rustc resolves a
//! name the pass predicts it of the module's `Scope`; a props type a glob
//! or a prelude imports, `use ui::*;`, it predicts is a struct literal. After
//! expansion rustc's names are the truth: the component's props type, its
//! signature's, has a companion where a macro of its name beside it is one
//! rust-js made, `#[rust_js::jsx_props]`. A prediction rustc's names prove
//! wrong stops the compile there, before a literal is checked, and the
//! crate is compiled once more with that component's answer. A compile so
//! stopped says nothing: what it would have said is held, and dropped, so
//! the replay's diagnostics are each said once.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, Mutex};

use rustc_errors::DiagInner;
use rustc_errors::emitter::{DynEmitter, Emitter};
use rustc_hir::def::DefKind;
use rustc_middle::ty::{self, TyCtxt};
use rustc_session::Session;
use rustc_span::def_id::DefId;
use rustc_span::source_map::SourceMap;
use rustc_span::{FileName, Span, Symbol};

/// A component, by where its name is written: its file and offset, what a
/// replay reads again, not rustc's ids, which are of one compile alone.
type Key = (FileName, u32);

/// Where a run of rustc is: its first compile, that compile stopped for a
/// replay, or the replay.
#[derive(Clone, Copy, Default, PartialEq)]
enum Phase {
    #[default]
    First,
    Stopped,
    Replaying,
}

#[derive(Default)]
struct State {
    /// The components predicted a struct literal, by their names.
    predicted: BTreeMap<Key, String>,
    /// Those a replay builds by their companion, rustc's names said so.
    flipped: BTreeSet<Key>,
    phase: Phase,
    /// The components a replay built by their companion, in order.
    replayed: Vec<String>,
}

static STATE: Mutex<Option<State>> = Mutex::new(None);

fn state<T>(f: impl FnOnce(&mut State) -> T) -> T {
    f(STATE
        .lock()
        .expect("JSX props' resolution")
        .get_or_insert_with(State::default))
}

fn key(sess: &Session, span: Span) -> Key {
    let file = sess.source_map().lookup_source_file(span.lo());
    (file.name.clone(), span.lo().0 - file.start_pos.0)
}

/// The component named at `name`, whose props the syntax pass predicts are
/// a struct literal: whether rustc's names said they're built by their
/// companion, a replay's answer; else recorded, to verify.
pub(super) fn predicted_literal(sess: &Session, name: rustc_span::Ident) -> bool {
    let at = key(sess, name.span);
    state(|state| match state.flipped.contains(&at) {
        true => true,
        false => {
            state.predicted.insert(at, name.to_string());
            false
        }
    })
}

/// Whether the struct `props` has a companion rust-js made: a macro of its
/// name in its module, `#[rust_js::jsx_props]`, of this crate or another's.
fn owns_companion(tcx: TyCtxt<'_>, props: DefId) -> bool {
    let module = tcx.parent(props);
    let marker = [Symbol::intern("rust_js"), Symbol::intern("jsx_props")];
    let owns = |def: DefId| {
        matches!(tcx.def_kind(def), DefKind::Macro(_))
            && tcx.opt_parent(def) == Some(module)
            && tcx.item_name(def) == tcx.item_name(props)
            && tcx.get_attrs_by_path(def, &marker).next().is_some()
    };
    match module.is_local() {
        true => tcx.hir_crate_items(()).definitions().any(|id| owns(id.to_def_id())),
        false => (tcx.module_children(module).iter())
            .filter_map(|child| child.res.opt_def_id())
            .any(owns),
    }
}

/// Each prediction of this compile, against rustc's names: `true` where one
/// was wrong, so the compile stops for a replay that builds those props by
/// their companion. Its held diagnostics are dropped then, else said. A
/// replay whose predictions are wrong again is an error, what it compiled
/// having changed.
pub fn verify(tcx: TyCtxt<'_>) -> bool {
    let predicted = state(|state| std::mem::take(&mut state.predicted));
    // Nothing predicted a literal, nothing to ask rustc's names of.
    if predicted.is_empty() {
        held::release();
        return false;
    }
    let mut wrong = Vec::new();
    for id in tcx.hir_crate_items(()).definitions() {
        if tcx.def_kind(id) != DefKind::Fn {
            continue;
        }
        let Some(span) = tcx.def_ident_span(id) else { continue };
        let at = key(tcx.sess, span);
        let Some(name) = predicted.get(&at) else { continue };
        // The props type its signature takes, an alias's target, by
        // reference or not.
        let sig = tcx.fn_sig(id).instantiate_identity().skip_normalization().skip_binder();
        let Some(props) = sig.inputs().first().map(|ty| ty.peel_refs()) else {
            continue;
        };
        if let ty::Adt(adt, _) = props.kind()
            && adt.is_struct()
            && owns_companion(tcx, adt.did())
        {
            wrong.push((at, name.clone()));
        }
    }
    let phase = state(|state| state.phase);
    if wrong.is_empty() {
        held::release();
        return false;
    }
    if phase == Phase::Replaying {
        held::release();
        tcx.dcx().err("rust-js: a component's props were resolved differently when compiled again: compile its unchanged sources again");
        return true;
    }
    held::discard();
    state(|state| {
        for (at, name) in wrong {
            state.flipped.insert(at);
            state.replayed.push(name);
        }
        state.phase = Phase::Stopped;
    });
    true
}

/// A run of rustc begun: no prediction of another's. The first compile of
/// a run holds its diagnostics until `verify` says it goes on.
pub fn begin() {
    *STATE.lock().expect("JSX props' resolution") = Some(State::default());
}

/// Whether the compile that ran stopped for a replay: then the next is it,
/// which says what it says as it goes.
pub fn replay() -> bool {
    state(|state| match state.phase {
        Phase::Stopped => {
            state.phase = Phase::Replaying;
            true
        }
        Phase::First | Phase::Replaying => false,
    })
}

/// Whether this compile is its run's replay.
fn replaying() -> bool {
    state(|state| state.phase == Phase::Replaying)
}

/// Whether this compile stopped for a replay.
fn stopped() -> bool {
    state(|state| state.phase == Phase::Stopped)
}

/// The components whose props a replay built by their companion, in order:
/// what `--props-replayed <file>` writes, a line each, its test's hook.
pub fn replayed() -> Vec<String> {
    state(|state| state.replayed.clone())
}

/// A compile's diagnostics held until `verify` says whether it goes on.
pub mod held {
    use super::*;

    /// What a compile does with what it would say: holds it, until `verify`
    /// asks; says it, what's held first; or drops it, a compile stopped for
    /// a replay saying nothing.
    enum Hold {
        Holding(Vec<DiagInner>),
        Saying(Vec<DiagInner>),
        Dropping,
    }

    static HOLD: Mutex<Option<Arc<Mutex<Hold>>>> = Mutex::new(None);

    fn current() -> Option<Arc<Mutex<Hold>>> {
        HOLD.lock().expect("held diagnostics").clone()
    }

    /// What's held said, and what follows said as it comes.
    pub(super) fn release() {
        if let Some(hold) = current() {
            let mut hold = hold.lock().expect("held diagnostics");
            if let Hold::Holding(held) = &mut *hold {
                *hold = Hold::Saying(std::mem::take(held));
            }
        }
    }

    /// What's held dropped, and what follows too.
    pub(super) fn discard() {
        if let Some(hold) = current() {
            *hold.lock().expect("held diagnostics") = Hold::Dropping;
        }
    }

    /// An emitter that holds what `inner` would say until it's released.
    pub struct Holder {
        inner: Box<DynEmitter>,
        hold: Arc<Mutex<Hold>>,
    }

    impl Holder {
        /// `inner`, held from the start of a run's first compile; a replay
        /// says what it says as it goes.
        fn new(inner: Box<DynEmitter>) -> Holder {
            let hold = Arc::new(Mutex::new(match super::replaying() {
                true => Hold::Saying(Vec::new()),
                false => Hold::Holding(Vec::new()),
            }));
            *HOLD.lock().expect("held diagnostics") = Some(hold.clone());
            Holder { inner, hold }
        }

        /// What's held to be said, said, in its order.
        fn flush(&mut self) {
            let held = match &mut *self.hold.lock().expect("held diagnostics") {
                Hold::Saying(held) => std::mem::take(held),
                Hold::Holding(_) | Hold::Dropping => Vec::new(),
            };
            for diag in held {
                self.inner.emit_diagnostic(diag);
            }
        }
    }

    impl Emitter for Holder {
        fn emit_diagnostic(&mut self, diag: DiagInner) {
            self.flush();
            match &mut *self.hold.lock().expect("held diagnostics") {
                Hold::Holding(held) => return held.push(diag),
                Hold::Dropping => return,
                Hold::Saying(_) => {}
            }
            self.inner.emit_diagnostic(diag);
        }

        fn emit_artifact_notification(&mut self, path: &std::path::Path, artifact_type: &str) {
            self.flush();
            self.inner.emit_artifact_notification(path, artifact_type);
        }

        fn emit_future_breakage_report(&mut self, diags: Vec<DiagInner>) {
            self.inner.emit_future_breakage_report(diags);
        }

        fn emit_unused_externs(&mut self, lint_level: rustc_lint_defs::Level, unused_externs: &[&str]) {
            self.inner.emit_unused_externs(lint_level, unused_externs);
        }

        fn should_show_explain(&self) -> bool {
            self.inner.should_show_explain()
        }

        fn supports_color(&self) -> bool {
            self.inner.supports_color()
        }

        fn source_map(&self) -> Option<&SourceMap> {
            self.inner.source_map()
        }
    }

    /// rustc's own emitter, as its session builds it of `opts`
    /// (`rustc_session::session::default_emitter`, private), then held: a
    /// human's, or Cargo's JSON (`--error-format=json`).
    pub fn of(opts: &rustc_session::config::Options, source_map: Arc<SourceMap>) -> Box<DynEmitter> {
        use rustc_errors::TerminalUrl;
        use rustc_errors::annotate_snippet_emitter_writer::AnnotateSnippetEmitter;
        use rustc_errors::emitter::{HumanReadableErrorType, OutputTheme, stderr_destination};
        use rustc_errors::json::JsonEmitter;
        use rustc_session::config::ErrorOutputType;
        let unstable = &opts.unstable_opts;
        let terminal_url = match unstable.terminal_urls {
            TerminalUrl::Auto => match (std::env::var("COLORTERM").as_deref(), std::env::var("TERM").as_deref()) {
                (Ok("truecolor"), Ok("xterm-256color")) if opts.unstable_features.is_nightly_build() => {
                    TerminalUrl::Yes
                }
                _ => TerminalUrl::No,
            },
            url => url,
        };
        let source_map = (!unstable.link_only).then_some(source_map);
        let inner: Box<DynEmitter> = match opts.error_format {
            ErrorOutputType::HumanReadable {
                kind: HumanReadableErrorType { short, unicode },
                color_config,
            } => Box::new(
                AnnotateSnippetEmitter::new(stderr_destination(color_config))
                    .sm(source_map)
                    .short_message(short)
                    .diagnostic_width(opts.diagnostic_width)
                    .macro_backtrace(unstable.macro_backtrace)
                    .track_diagnostics(unstable.track_diagnostics)
                    .terminal_url(terminal_url)
                    .theme(if unicode {
                        OutputTheme::Unicode
                    } else {
                        OutputTheme::Ascii
                    })
                    .ignored_directories_in_source_blocks(
                        unstable.ignore_directory_in_diagnostics_source_blocks.clone(),
                    )
                    .ui_testing(unstable.ui_testing),
            ),
            ErrorOutputType::Json {
                pretty,
                json_rendered,
                color_config,
            } => Box::new(
                JsonEmitter::new(
                    Box::new(std::io::BufWriter::new(std::io::stderr())),
                    source_map,
                    pretty,
                    json_rendered,
                    color_config,
                )
                .ui_testing(unstable.ui_testing)
                .ignored_directories_in_source_blocks(unstable.ignore_directory_in_diagnostics_source_blocks.clone())
                .diagnostic_width(opts.diagnostic_width)
                .macro_backtrace(unstable.macro_backtrace)
                .track_diagnostics(unstable.track_diagnostics)
                .terminal_url(terminal_url),
            ),
        };
        Box::new(Holder::new(inner))
    }

    /// `config`'s session's diagnostics held, `of`'s, whatever else its
    /// session is given first.
    pub fn install(config: &mut rustc_interface::interface::Config) {
        let opts = config.opts.clone();
        let before = config.psess_created.take();
        config.psess_created = Some(Box::new(move |psess| {
            if let Some(before) = before {
                before(psess);
            }
            psess.dcx().set_emitter(of(&opts, psess.clone_source_map()));
        }));
    }

    /// A compile that ends with diagnostics still held, one whose errors
    /// came before `verify` could ask, says them; not one stopped for a
    /// replay.
    impl Drop for Holder {
        fn drop(&mut self) {
            {
                let mut hold = self.hold.lock().expect("held diagnostics");
                if let Hold::Holding(held) = &mut *hold
                    && !super::stopped()
                {
                    *hold = Hold::Saying(std::mem::take(held));
                }
            }
            self.flush();
        }
    }
}
