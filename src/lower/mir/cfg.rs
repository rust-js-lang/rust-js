//! A MIR body's control flow as it runs without a panic: the blocks reached
//! from the start, their order, and what dominates what (ADR 0364). A panic
//! unwinds as a JS exception does, so cleanup blocks and unwind edges are
//! left out, and so are the edges rustc adds for borrowck alone.

use rustc_middle::mir::{BasicBlock, Body, TerminatorKind};
use std::collections::HashSet;

pub(super) struct Graph {
    /// Each block's successors, in the order its terminator names them.
    pub(super) succ: Vec<Vec<BasicBlock>>,
    /// The blocks reached from the start, in reverse postorder.
    pub(super) rpo: Vec<BasicBlock>,
    /// Each block's place in `rpo`, `usize::MAX` for one never reached.
    pub(super) order: Vec<usize>,
    /// Each block's immediate dominator; the start's is itself.
    pub(super) idom: Vec<Option<BasicBlock>>,
    /// The targets of edges back to a block that dominates their source.
    pub(super) headers: HashSet<BasicBlock>,
    /// The blocks more than one forward edge reaches.
    pub(super) merges: HashSet<BasicBlock>,
}

/// What `block` goes on to when nothing panics.
pub(super) fn successors(body: &Body<'_>, block: BasicBlock) -> Vec<BasicBlock> {
    use TerminatorKind::*;
    let mut next = match &body.basic_blocks[block].terminator().kind {
        Goto { target } => vec![*target],
        SwitchInt { targets, .. } => targets.all_targets().to_vec(),
        Drop { target, .. } | Assert { target, .. } => vec![*target],
        Call { target, .. } => target.iter().copied().collect(),
        FalseEdge { real_target, .. } | FalseUnwind { real_target, .. } => vec![*real_target],
        InlineAsm { targets, .. } => targets.to_vec(),
        Yield { resume, .. } => vec![*resume],
        Return | Unreachable | UnwindResume | UnwindTerminate(_) | CoroutineDrop | TailCall { .. } => Vec::new(),
    };
    let mut seen = HashSet::new();
    next.retain(|b| seen.insert(*b));
    next
}

impl Graph {
    pub(super) fn of(body: &Body<'_>) -> Graph {
        let n = body.basic_blocks.len();
        let succ: Vec<Vec<BasicBlock>> = (0..n).map(|i| successors(body, BasicBlock::from_usize(i))).collect();
        // Postorder by an explicit stack, children in their order.
        let mut post = Vec::with_capacity(n);
        let mut visited = vec![false; n];
        let start = BasicBlock::from_usize(0);
        let mut stack: Vec<(BasicBlock, usize)> = vec![(start, 0)];
        visited[0] = true;
        while let Some((block, i)) = stack.pop() {
            if let Some(&next) = succ[block.as_usize()].get(i) {
                stack.push((block, i + 1));
                if !visited[next.as_usize()] {
                    visited[next.as_usize()] = true;
                    stack.push((next, 0));
                }
            } else {
                post.push(block);
            }
        }
        let rpo: Vec<BasicBlock> = post.into_iter().rev().collect();
        let mut order = vec![usize::MAX; n];
        for (i, b) in rpo.iter().enumerate() {
            order[b.as_usize()] = i;
        }
        let mut preds: Vec<Vec<BasicBlock>> = vec![Vec::new(); n];
        for &b in &rpo {
            for &s in &succ[b.as_usize()] {
                preds[s.as_usize()].push(b);
            }
        }
        // Cooper, Harvey and Kennedy's: each block's dominator, in
        // reverse postorder until nothing changes.
        let mut idom: Vec<Option<BasicBlock>> = vec![None; n];
        idom[0] = Some(start);
        let intersect = |idom: &[Option<BasicBlock>], mut a: BasicBlock, mut b: BasicBlock| {
            while a != b {
                while order[a.as_usize()] > order[b.as_usize()] {
                    a = idom[a.as_usize()].expect("a processed block");
                }
                while order[b.as_usize()] > order[a.as_usize()] {
                    b = idom[b.as_usize()].expect("a processed block");
                }
            }
            a
        };
        let mut changed = true;
        while changed {
            changed = false;
            for &b in rpo.iter().skip(1) {
                let mut new = None;
                for &p in &preds[b.as_usize()] {
                    if idom[p.as_usize()].is_some() {
                        new = Some(match new {
                            None => p,
                            Some(other) => intersect(&idom, p, other),
                        });
                    }
                }
                if new != idom[b.as_usize()] {
                    idom[b.as_usize()] = new;
                    changed = true;
                }
            }
        }
        let mut graph = Graph {
            succ,
            rpo,
            order,
            idom,
            headers: HashSet::new(),
            merges: HashSet::new(),
        };
        let mut forward_in = vec![0usize; n];
        for &b in &graph.rpo {
            for &s in &graph.succ[b.as_usize()] {
                if graph.dominates(s, b) {
                    graph.headers.insert(s);
                } else {
                    forward_in[s.as_usize()] += 1;
                }
            }
        }
        graph.merges = (0..n)
            .filter(|&i| forward_in[i] > 1)
            .map(BasicBlock::from_usize)
            .collect();
        graph
    }

    /// Whether `a` dominates `b`: every way from the start to `b` passes it.
    pub(super) fn dominates(&self, a: BasicBlock, mut b: BasicBlock) -> bool {
        loop {
            if a == b {
                return true;
            }
            match self.idom[b.as_usize()] {
                Some(up) if up != b => b = up,
                _ => return false,
            }
        }
    }

    /// Whether the edge `from` to `to` goes back to a loop's header.
    pub(super) fn backward(&self, from: BasicBlock, to: BasicBlock) -> bool {
        self.headers.contains(&to) && self.dominates(to, from)
    }

    /// The blocks `block` immediately dominates that more than one forward
    /// edge reaches, the latest first: each is what follows a labeled
    /// block, the latest the outermost (Ramsey's "Beyond Relooper").
    pub(super) fn merge_children(&self, block: BasicBlock) -> Vec<BasicBlock> {
        let mut children: Vec<BasicBlock> = self
            .rpo
            .iter()
            .copied()
            .filter(|&c| c != block && self.idom[c.as_usize()] == Some(block) && self.merges.contains(&c))
            .collect();
        children.sort_by_key(|c| std::cmp::Reverse(self.order[c.as_usize()]));
        children
    }
}
