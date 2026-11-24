//! `ParamForm`'s shadow, Campaign 6's step 0 (#2539, ADR 1203): of each
//! value converted where it crosses into or out of generic code, the one
//! form the compile takes there (`form_given`, `form_back`), beside every
//! form generic code's view differs in, the plan `ParamForm`'s one record
//! would derive. Where the plan has more than one step, or another one,
//! `ParamForm` would decide otherwise than today's forms. A `param-form`
//! shadow line each (`--shadow param-form=<file>`, ADR 1199); no output
//! changes. Each form is asked of its owner as today's crossing asks it,
//! `form_differs`, never decided here.

use super::{Crossing, Form};
use crate::FnCx;
use crate::session::ShadowKind;

/// Where a value is converted, which a shadow line names.
#[derive(Clone, Copy)]
pub(crate) enum Route {
    /// Given to generic code, `form_given`'s.
    Given,
    /// Given back by it, `form_back`'s.
    Back,
}

/// Each form, in the order `ParamForm`'s plan applies them: what holds the
/// value first, then what reaches it.
const PLAN_ORDER: [Form; 6] = [
    Form::Placed,
    Form::Box,
    Form::HeldBox,
    Form::Handle,
    Form::Tagged,
    Form::Valued,
];

/// A form's word in a shadow line.
fn word(form: Form) -> &'static str {
    match form {
        Form::Handle => "handle",
        Form::Box => "box",
        Form::HeldBox => "held-box",
        Form::Tagged => "tagged",
        Form::Valued => "valued",
        Form::Placed => "placed",
    }
}

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// A `param-form` shadow line of the value converted at `crossing` to
    /// `taken`, at `route`, where the session records one: every form that
    /// differs there, the plan; `agrees` where the plan is `taken` alone,
    /// `drops` where it has more steps than the one taken, `other` where
    /// its one step isn't `taken`. A value given is never `Form::Valued`'s,
    /// the same form on both sides.
    pub(crate) fn shadow_param_form(&self, route: Route, crossing: Crossing<'tcx>, taken: Form) {
        let tcx = self.tcx;
        if !crate::session::shadowing(tcx, ShadowKind::ParamForm) {
            return;
        }
        let (def_id, declared, instantiated) = crossing;
        let plan: Vec<Form> = (PLAN_ORDER.into_iter())
            .filter(|&form| !matches!((route, form), (Route::Given, Form::Valued)))
            .filter(|&form| self.form_differs(form, crossing))
            .collect();
        let verdict = match plan.as_slice() {
            [only] if *only == taken => "agrees",
            [_, _, ..] => "drops",
            _ => "other",
        };
        let steps = plan.iter().map(|&form| word(form)).collect::<Vec<_>>().join("+");
        let route = match route {
            Route::Given => "given",
            Route::Back => "back",
        };
        let erase = |ty| tcx.erase_and_anonymize_regions(ty);
        let line = format!(
            "site\t{route}\t{verdict}\t{}\t{}\t{}\t{}\t{}\t{}",
            word(taken),
            if steps.is_empty() { "-" } else { &steps },
            tcx.def_path_str(def_id),
            erase(declared),
            erase(instantiated),
            tcx.def_path_str(self.frame.owner),
        );
        crate::session::shadow(tcx, ShadowKind::ParamForm, || line);
    }
}
