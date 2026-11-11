//! Whether tidy's stages run again (#779, #784).

/// Tidy's stages run again (`stages::Stage::again`): whether the stage
/// running found they must, once it's done, `must`, set by its passes and
/// taken by `tidy`; and whether they're running, `running`, after the
/// body's own `written` stage, which isn't run again (#779, #784).
#[derive(Default)]
pub(in crate::lower::mir) struct Again {
    pub(in crate::lower::mir) must: std::cell::Cell<bool>,
    pub(in crate::lower::mir) running: std::cell::Cell<bool>,
}
