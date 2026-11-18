//! Evidence for an option that is present when control flow joins.

use super::*;

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// A path proves presence by taking the present edge of a test or by
    /// writing `Some` afterwards. No path may reach the entry or an unknown
    /// write without passing one of those points.
    pub(super) fn present_on_all_paths(&self, state: &State<'_, 'tcx>, local: Local, read: BasicBlock) -> bool {
        let blocks = &state.body.basic_blocks;
        let predecessors = blocks.predecessors();
        let parameter = (1..=state.body.arg_count).contains(&local.as_usize());
        if state.locals.writes[local] <= usize::from(!parameter)
            || predecessors[read].len() < 2
            || self.whole_writes(state, local).is_none()
            || self.last_presence_write(state, read, local).is_some()
        {
            return false;
        }

        let mut seen = IndexSet::default();
        let mut work = vec![read];
        while let Some(to) = work.pop() {
            if !seen.insert(to) {
                continue;
            }
            if to == mir::START_BLOCK || predecessors[to].is_empty() {
                return false;
            }
            for &from in &predecessors[to] {
                let block = &blocks[from];
                let terminator = &block.terminator().kind;
                if block.is_cleanup
                    || matches!(terminator, TerminatorKind::Call { destination, .. } if destination.local == local)
                {
                    return false;
                }
                match self.last_presence_write(state, from, local) {
                    Some(true) => continue,
                    Some(false) => return false,
                    None => {}
                }
                if let TerminatorKind::SwitchInt { discr, targets } = terminator
                    && let Some(flag) = discr.place().filter(|place| place.projection.is_empty())
                    && self.present_edge(state, flag.local, targets, local) == Some(to)
                    && block
                        .terminator()
                        .successors()
                        .filter(|&successor| successor == to)
                        .count()
                        == 1
                    && self.unwritten_since(state, flag.local, local, from)
                {
                    continue;
                }
                work.push(from);
            }
        }
        true
    }

    /// The last statement that sets or ends the local, and whether it writes
    /// a `Some` whose payload cannot itself be JavaScript null.
    fn last_presence_write(&self, state: &State<'_, 'tcx>, block: BasicBlock, local: Local) -> Option<bool> {
        let non_null = self.krate.nulled_params.js_nones();
        state.body[block]
            .statements
            .iter()
            .rev()
            .find_map(|statement| match &statement.kind {
                mir::StatementKind::Assign(assign) if assign.0.local == local => Some(match &assign.1 {
                    Rvalue::Use(operand, _) => self.made_some(state, operand),
                    Rvalue::Aggregate(kind, fields) => {
                        matches!(**kind, AggregateKind::Adt(adt, variant, ..)
                                if self.tcx.is_lang_item(adt, LangItem::Option) && variant.as_u32() == 1)
                            && fields
                                .iter()
                                .filter_map(Operand::place)
                                .all(|place| !non_null.payload_may_be_null(self.tcx, state.body, place))
                    }
                    _ => false,
                }),
                mir::StatementKind::StorageDead(place) | mir::StatementKind::StorageLive(place) if *place == local => {
                    Some(false)
                }
                _ => None,
            })
    }
}
