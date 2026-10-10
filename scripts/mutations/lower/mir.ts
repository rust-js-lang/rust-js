// Mutations of src/lower/mir.rs (ADR 0364).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "mir-fold-out-of-order",
    breaks: "temporaries that do something are folded where they're read, not in the order they were made",
    file: "src/lower/mir.rs",
    find: "        let ordered = acting.windows(2).all(|w| w[0] < w[1]);\n",
    replace: "        let ordered = true;\n",
    tests: ["test/mir.test.ts", "-t", "order"],
  },
  {
    name: "mir-assert-inverted",
    breaks: "a MIR `Assert` throws when its condition holds, not when it fails",
    file: "src/lower/mir.rs",
    find: "                    true => Expr::unary(js::UnaryOp::Not, cond),\n",
    replace: "                    true => cond,\n",
    tests: ["test/mir.test.ts", "-t", "bitwise_operators"],
  },
  {
    name: "mir-statements-inside-labels",
    breaks: "what a block binds before it branches is inside its labeled block, unseen where its branches meet",
    file: "src/lower/mir.rs",
    find: "        let mut out = self.mir_statements(state, block)?;\n        out.extend(self.mir_within(state, block, &merges)?);\n        Ok(out)\n    }\n\n    /// Where `block` goes, inside a labeled block for each of `merges`, each\n    /// followed by its own code (Ramsey's `nodeWithin`). Its statements come\n    /// before, as none of them branches: what they bind is seen where its\n    /// branches meet.\n    fn mir_within(&mut self, state: &mut State<'_, 'tcx>, block: BasicBlock, merges: &[BasicBlock]) -> R<Vec<Stmt>> {\n        let Some((&merge, inner)) = merges.split_first() else {\n            return self.mir_terminator(state, block);\n",
    replace: "        self.mir_within(state, block, &merges)\n    }\n\n    /// Where `block` goes, inside a labeled block for each of `merges`, each\n    /// followed by its own code (Ramsey's `nodeWithin`). Its statements come\n    /// before, as none of them branches: what they bind is seen where its\n    /// branches meet.\n    fn mir_within(&mut self, state: &mut State<'_, 'tcx>, block: BasicBlock, merges: &[BasicBlock]) -> R<Vec<Stmt>> {\n        let Some((&merge, inner)) = merges.split_first() else {\n            let mut out = self.mir_statements(state, block)?;\n            out.extend(self.mir_terminator(state, block)?);\n            return Ok(out);\n",
    tests: ["test/mir.test.ts","-t","bindings_before_branch"],
  },
];
