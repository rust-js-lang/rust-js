//! A channel on one thread, `mpsc::channel()`: a queue its two ends share
//! (ADR 0142).

use crate::js::{Expr, Stmt};
use crate::lower::calls::Call;
use crate::lower::recognition::{ChannelOp, Std};
use crate::lower::{FnCx, R};
use crate::runtime::Helper;

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// A channel's call (ADR 0142): `None` if `known` is another.
    pub(in crate::lower) fn channel_call(
        &mut self,
        known: Std,
        _call: Call<'_, 'tcx>,
        values: &mut std::vec::IntoIter<Expr>,
        _out: &mut Vec<Stmt>,
    ) -> R<Option<Expr>> {
        let Std::Channel(op) = known else {
            return Ok(None);
        };
        self.runtime.insert(Helper::Channel);
        let (helper, count) = match op {
            ChannelOp::New => ("$channel", 0),
            ChannelOp::Send => ("$send", 2),
            ChannelOp::Recv => ("$recv", 1),
            ChannelOp::TryRecv => ("$tryRecv", 1),
        };
        Ok(Some(Expr::call(Expr::var(helper), values.take(count).collect())))
    }
}
