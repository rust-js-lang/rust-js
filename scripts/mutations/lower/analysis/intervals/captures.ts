// Mutations of src/lower/analysis/intervals/captures.rs, what the MIR
// intervals know of a closure's captures where it's made (ADR 1137, #1701).
import type { Mutation } from "../../../../mutations";

const file = "src/lower/analysis/intervals/captures.rs";
const tests = ["test/lowering-integer-ranges.test.ts", "-t", "what it was made with"];

export const mutations: Mutation[] = [
  {
    name: "captures-never-given",
    breaks: "a closure isn't given what its captures were made with, so `move || at * 2` of `at = 10` keeps its wrap",
    file,
    find: "            given.entry(closure).or_default().captures = captures;\n",
    replace: "            let _ = (closure, captures);\n",
    tests,
  },
  {
    name: "captures-written-read",
    breaks: "a capture its closure's body writes is what it was made with, so `at -= 1` of a `move` closure's own `at = 2`, called a third time, is taken as 1 and gives `-1` for `u32::MAX`",
    file,
    find: "        if referent.is_some() || ty_is_value(upvar) && !written.contains(&k) {\n",
    replace: "        if referent.is_some() || ty_is_value(upvar) {\n",
    tests,
  },
  {
    name: "captures-shared-reference-unfollowed",
    breaks: "a shared reference a closure captures is of nothing known, so `n - step` of `step = 7` past `n > 9` keeps its wrap",
    file,
    find: "        self.operand(&Operand::Copy(self.referred(operand)?.into()), state)\n",
    replace: "        let _ = (self.referred(operand)?, state);\n        None\n",
    tests,
  },
  {
    name: "captures-not-locals",
    breaks: "a closure's captures are no locals of the body the walk reads, so `from_fn(|| ..)`'s `n -= 1` past `n == 0` of a `&mut n` capture is left to the bridge again, test/intervals-bridged.txt's 5 sites of #1701 B",
    file,
    find: "    if !tcx.is_closure_like(body.source.def_id()) || body.arg_count == 0 {\n",
    replace: "    if true || !tcx.is_closure_like(body.source.def_id()) || body.arg_count == 0 {\n",
    tests: ["test/lowering-unwind-arms.test.ts"],
  },
  {
    name: "closure-results-never-given",
    breaks: "no closure's result tells its caller anything, so `i += 1` past `while let Some(_) = ITEMS.with_borrow(|items| items.get(i).copied())` keeps its wrap",
    file,
    find: "            results.insert(closure, result);\n",
    replace: "            let _ = (closure, result);\n",
    tests: ["test/lowering-integer-ranges.test.ts", "-t", "tells its caller the index"],
  },
  {
    name: "closure-copied-capture-unread",
    breaks: "an `i` a closure made where it's given the call captured by copy is none of its caller's, so `i += 1` past `with_borrow(move |items| items.get(i).copied())` keeps its wrap",
    file,
    find: "                .map(|operand| self.referred(operand).or_else(|| copied(operand)))\n",
    replace: "                .map(|operand| self.referred(operand).or_else(|| copied(operand).filter(|_| false)))\n",
    tests: ["test/lowering-integer-ranges.test.ts", "-t", "tells its caller the index"],
  },
];
