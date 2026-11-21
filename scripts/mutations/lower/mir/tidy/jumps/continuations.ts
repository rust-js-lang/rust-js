// Mutations of src/lower/mir/tidy/jumps/continuations.rs: a block MIR made
// whose `break`s jump on to what follows it, each other way given what it
// went on to, each `break` last on its way then and gone (#2215).
import type { Mutation } from "../../../../../mutations";

const leads = "lower::mir::tidy::jumps::continuations::tests::breaks_lead_on_to_the_continuation";
const keeps = "lower::mir::tidy::jumps::continuations::tests::breaks_keep_their_jump_where_a_move_would_differ";
const file = "src/lower/mir/tidy/jumps/continuations.rs";

export const mutations: Mutation[] = [
  {
    name: "continuation-not-led-on",
    breaks: "a block's `break`s that jump on to its continuation past a tail of drops and a `return` keep their labeled block, `bb12:` in std's `try_fold`s (#2215)",
    file,
    find: "        if lead_on(&mut led, label) {\n",
    replace: "        if false && lead_on(&mut led, label) {\n",
    unitTests: leads,
  },
  {
    name: "continuation-moved-past-a-then-that-goes-on",
    breaks: "what follows an `if` whose `then` goes on is moved into its `else`, so the way through the `then` that goes on no longer runs it (#2215)",
    file,
    find: "!guards && js::names_jump(branch, label) && branch.last().is_some_and(js::ends);",
    replace: "!guards && js::names_jump(branch, label);",
    unitTests: keeps,
  },
  {
    name: "continuation-moved-out-of-a-guard",
    breaks: "a guard's `if (c) { break l; } s; return x;`, as a `let .. else` or a `switch`'s case is written, is turned inside out, `if (!c) { s; return x; }` (#2215)",
    file,
    find: "    let guards = rest.last().is_some_and(js::ends);\n",
    replace: "    let guards = false;\n",
    unitTests: keeps,
  },
  {
    name: "continuation-moved-under-a-shadow",
    breaks: "what follows an `if` is moved into a branch that declares a name it reads, which it then reads instead (#2215)",
    file,
    find: "        if shadowed(other, &rest) {\n",
    replace: "        if false {\n",
    unitTests: keeps,
  },
  {
    name: "continuation-copied-under-a-shadow",
    breaks: "a tail of drops and a `return` is written at the end of a branch that declares a name it reads, which the copy then reads instead (#2215)",
    file,
    find: "    if !super::epilogues::copied_tail(&rest) || shadowed(&stmts[at..=at], &rest) {\n",
    replace: "    if !super::epilogues::copied_tail(&rest) {\n",
    unitTests: keeps,
  },
  {
    name: "continuation-copies-what-no-break-may-take",
    breaks: "any tail is written at the end of each way that goes on, a `return` alone, which `copied_tail` keeps to other passes (#2215)",
    file,
    find: "    if !super::epilogues::copied_tail(&rest) || shadowed(",
    replace: "    if shadowed(",
    unitTests: keeps,
  },
  {
    name: "continuation-moved-before-its-reader",
    breaks: "a statement that declares a name read before it, in what it follows, is moved into a branch, which the earlier read then misses (#2215)",
    file,
    find: "    if read_before(&rest, stmts) {\n",
    replace: "    if false {\n",
    unitTests: keeps,
  },
  {
    name: "continuation-else-not-led-on",
    breaks: "an `else` that never goes on but by the `break` doesn't have what follows its `if` moved into the `then`, so the block keeps its label (#2215)",
    file,
    find: "    if leaves(other) && !js::names_jump(then, label) {\n",
    replace: "    if false {\n",
    unitTests: "lower::mir::tidy::jumps::continuations::tests::an_else_that_breaks_leads_on_too",
  },
];
