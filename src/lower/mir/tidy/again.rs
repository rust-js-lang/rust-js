//! Whether tidy's stages run again (`stages::Stage::again`, #779, #784):
//! one state of three (#872). None found they must; the stage running
//! found they must, once it's done; or they're running, after the body's
//! own `written` stage, which isn't run again. A pass says what it found,
//! `found`; `tidy` takes it, `take`, and ends it, `done`; a pass of a stage
//! run again asks whether it is, `running`.

use std::cell::Cell;

#[derive(Default)]
pub(in crate::lower::mir) struct Again(Cell<State>);

#[derive(Clone, Copy, Default, PartialEq, Debug)]
enum State {
    #[default]
    Idle,
    Must,
    Running,
}

impl Again {
    /// A pass of the stage running found its stages must run again, or not.
    /// None of the stages run again finds it: none of them makes them.
    pub(in crate::lower::mir) fn found(&self, must: bool) {
        self.0.set(match (self.0.get(), must) {
            (State::Running, _) => unreachable!("a stage run again found its stages must run again (#872)"),
            (state, false) => state,
            (_, true) => State::Must,
        });
    }

    /// Whether the stages run again now, which they do where a pass found
    /// they must: then they're running.
    pub(in crate::lower::mir) fn take(&self) -> bool {
        match self.0.get() {
            State::Idle => false,
            State::Must => {
                self.0.set(State::Running);
                true
            }
            State::Running => unreachable!("tidy's stages run again while they run again (#872)"),
        }
    }

    /// They ran again: none must now.
    pub(in crate::lower::mir) fn done(&self) {
        debug_assert_eq!(self.0.get(), State::Running);
        self.0.set(State::Idle);
    }

    /// Whether the stages are running again.
    pub(in crate::lower::mir) fn running(&self) -> bool {
        self.0.get() == State::Running
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Each transition, as tidy makes them: a pass that finds nothing keeps
    // what an earlier one found; `take` of what's found runs them, and of
    // nothing doesn't; `done` ends the run.
    #[test]
    fn found_is_kept_until_taken_and_done_ends_the_run() {
        let again = Again::default();
        assert!(!again.take(), "nothing found runs nothing again");
        again.found(true);
        again.found(false);
        assert!(!again.running(), "found isn't running");
        assert!(again.take(), "what a pass found, then nothing, runs them again");
        assert!(again.running(), "taken, they're running");
        again.done();
        assert!(!again.running(), "done, they're not");
        assert!(!again.take(), "done, nothing more runs again");
    }
}
