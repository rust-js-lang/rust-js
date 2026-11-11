//! Whether a call of a library's function gives it what may fail that it
//! writes or formats through (#1392): a writer of the crate's whose `write`
//! may, a file opened to read, a value whose `Display` may. A library none
//! of whose own writes fail lowers each of its writes as one that never
//! does: its JS passes a writer's throw on, but tests it, `is_ok()`,
//! `match`, `let _ =`, by its constant answer, and its manifest says it
//! never fails, so its caller here catches nothing. The crate compiles its
//! own of it, lowered as the crate's writes are (ADR 0914, amended). What
//! no consumer compiles, a write through a `dyn` or a trait method's own
//! type parameter, the library takes to fail itself (`fmt_failures`).

use super::needed::compiled_here;
use super::*;
use crate::lower::analysis::{Failing, call_may_fail};
use crate::lower::library::Foreign;
use crate::lower::recognition::{fmt_trait_called, formatted_parts, is_write_result_type};

/// Whether `caller`'s call of `callee`, a library's function its export
/// says never fails, of `args`, may fail as the crate's writes may: what
/// its MIR calls, given what the call gives it, of a write or a format the
/// crate's analysis says may fail, or a function of its library's that
/// does, asked in turn.
pub(super) fn fails_given<'tcx>(
    tcx: TyCtxt<'tcx>,
    foreign: &Foreign<'_, 'tcx>,
    failing: Failing<'_>,
    (caller, callee, args): (DefId, DefId, ty::GenericArgsRef<'tcx>),
) -> bool {
    if !failing.any || foreign.fails(callee) || !args.types().next().is_some() {
        return false;
    }
    let typing_env = ty::TypingEnv::post_analysis(tcx, tcx.typeck_root_def_id(caller));
    let given = Given {
        tcx,
        typing_env,
        caller,
        foreign,
        failing,
    };
    given.fails((callee, args), &mut IndexSet::default())
}

/// The crate's answers a library's body is asked in, given a call's types.
struct Given<'a, 'tcx> {
    tcx: TyCtxt<'tcx>,
    typing_env: ty::TypingEnv<'tcx>,
    caller: DefId,
    foreign: &'a Foreign<'a, 'tcx>,
    failing: Failing<'a>,
}

impl<'tcx> Given<'_, 'tcx> {
    /// Whether `id`'s body, of `args`, calls what may fail here: of each
    /// call that depends on its type parameters, what the library knows
    /// whole being its own, its manifest's answer.
    fn fails(&self, (id, args): (DefId, ty::GenericArgsRef<'tcx>), seen: &mut IndexSet<DefId>) -> bool {
        let tcx = self.tcx;
        if !seen.insert(id) || !tcx.is_mir_available(id) {
            return false;
        }
        called(tcx.optimized_mir(id)).into_iter().any(|(callee, generic)| {
            if !generic.has_param() {
                return false;
            }
            let given = ty::EarlyBinder::bind(tcx, generic).instantiate(tcx, args).skip_normalization();
            let given = (tcx.try_normalize_erasing_regions(self.typing_env, ty::Unnormalized::new_wip(given)))
                .unwrap_or(given);
            self.call_fails(callee, given)
                // A closure it makes, which its body's calls step, `filter(|x| w.write(x).is_ok())`'s.
                || (given.types().flat_map(|ty| ty.walk())).any(|part| match part.as_type().map(|ty| *ty.kind()) {
                    Some(ty::Closure(closure, of)) => !closure.is_local() && self.fails((closure, of), seen),
                    _ => false,
                })
                || (compiled_here(self.foreign, tcx, callee) && !self.foreign.fails(callee) && self.fails((callee, given), seen))
        })
    }

    /// Whether a call of `callee` of `args` is a write or a format that may
    /// fail here: `{}` of a value whose `fmt` may, of each type it holds,
    /// or a call that gives back a write's result and may give its error.
    fn call_fails(&self, callee: DefId, args: ty::GenericArgsRef<'tcx>) -> bool {
        let tcx = self.tcx;
        let may_fail = |id: DefId, args| {
            call_may_fail(
                tcx,
                (self.typing_env, self.caller),
                self.foreign,
                self.failing,
                (id, args),
            )
        };
        if let Some(fmt_trait) = fmt_trait_called(tcx, callee) {
            let fmt = tcx.associated_item_def_ids(fmt_trait)[0];
            return (args.types().next().into_iter().flat_map(formatted_parts))
                .any(|part| may_fail(fmt, tcx.mk_args(&[part.into()])));
        }
        let output = tcx
            .fn_sig(callee)
            .instantiate(tcx, args)
            .skip_normalization()
            .skip_binder()
            .output();
        let output =
            (tcx.try_normalize_erasing_regions(self.typing_env, ty::Unnormalized::new_wip(output))).unwrap_or(output);
        is_write_result_type(tcx, output) && may_fail(callee, args)
    }
}
