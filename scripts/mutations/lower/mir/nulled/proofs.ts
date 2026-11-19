// Mutations of src/lower/mir/nulled/proofs.rs (ADR 0093): where an
// `Option` is proved `Some`, split from nulled.rs (#2112), its mutations
// with it.
import type { Mutation } from "../../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "constant-flag-unfollowed",
    breaks: "a `bool` set to constants in a test's arms, `matches!`'s, proves nothing on its edges, so `if !matches!(js::classify(entry), Kind::Undefined) { return entry; }` keeps `?? null`, left for tidy to guess (#1933 step 4 (D))",
    file: "src/lower/mir/nulled/proofs.rs",
    find: "            let Some(written) = self.constant_writes(state, flag.local) else {\n",
    replace: "            let Some(written) = self.constant_writes(state, flag.local).filter(|_| false) else {\n",
    tests: [
      "test/lowering-nullable-tests.test.ts",
      "-t",
      "proved defined"
    ]
  },
  {
    name: "constant-flag-edge-undominated",
    breaks: "a `bool`'s edge proves a read it doesn't lead to, `flag && !matches!(..)`'s past the `if`, so `entry` there is given as `undefined` where Rust's `None` is `null` (#1933 step 4 (D))",
    file: "src/lower/mir/nulled/proofs.rs",
    find: "                entered(taken)\n                    && dominators.dominates(taken, read)\n",
    replace: "                entered(taken)\n",
    tests: [
      "test/lowering-nullable-tests.test.ts",
      "-t",
      "proved defined"
    ]
  },
  {
    name: "constant-flag-values-swapped",
    breaks: "a `bool`'s edge is taken to be the other value's, so the arm that wrote `true` is asked of the edge `false` takes (#1933 step 4 (D))",
    file: "src/lower/mir/nulled/proofs.rs",
    find: "                    && (written.iter().filter(|&&(_, v)| v == value))\n",
    replace: "                    && (written.iter().filter(|&&(_, v)| v != value))\n",
    tests: [
      "test/lowering-nullable-tests.test.ts",
      "-t",
      "proved defined"
    ]
  },
  {
    name: "constant-flag-write-unproved",
    breaks: "a write of a `bool`'s value is taken to prove where it is, past an edge of a test or not, so the arm of `Kind::Undefined` proves `entry` defined (#1933 step 4 (D))",
    file: "src/lower/mir/nulled/proofs.rs",
    find: "                        .all(|&(at, _)| tests.iter().any(|&(tested, targets)| proves(tested, targets, at)))\n",
    replace: "                        .all(|_| true)\n",
    tests: [
      "test/lowering-nullable-tests.test.ts",
      "-t",
      "proved defined"
    ]
  },
  {
    name: "constant-flag-call-written",
    breaks: "a `bool` written by a call as well as by constants, `let mut defined = js::truthy(other)`, is taken for one set to constants alone, so a `true` the call gave proves `entry` defined (#1933 step 4 (D))",
    file: "src/lower/mir/nulled/proofs.rs",
    find: "(!written.is_empty() && self.whole_writes(state, flag)?.len() == written.len()).then_some(written)",
    replace: "(!written.is_empty()).then_some(written)",
    tests: [
      "test/lowering-nullable-tests.test.ts",
      "-t",
      "proved defined"
    ]
  },
  {
    name: "constant-flag-parameter",
    breaks: "a `bool` parameter, which its caller wrote, is taken for one set to constants alone, none of them, so each of its edges proves anything: `value.is_none() && flag`'s `return value` (#1933 step 4 (D))",
    file: "src/lower/mir/nulled/proofs.rs",
    find: "        if (1..=state.body.arg_count).contains(&flag.as_usize()) {\n            return None;\n        }\n        let mut written = Vec::new();\n",
    replace: "        let mut written = Vec::new();\n",
    tests: [
      "test/lowering-nullable-tests.test.ts"
    ]
  },
  {
    name: "constant-flag-negation-kept",
    breaks: "a `bool` written of another's negation, `let defined = !matches!(..)`, is taken for its value, so `Kind::Undefined`'s arm proves it (#1933 step 4 (D))",
    file: "src/lower/mir/nulled/proofs.rs",
    find: "                    flag = of.local;\n                    negated = !negated;\n",
    replace: "                    flag = of.local;\n",
    tests: [
      "test/lowering-nullable-tests.test.ts",
      "-t",
      "proved defined"
    ]
  },
  {
    name: "proved-present-of-any-test",
    breaks: "an Option read where any test's true branch is is taken for one `js::truthy` proved there, `if (!js::truthy(x)) { return x; }` too, its `?? null` gone (#1902)",
    file: "src/lower/mir/nulled/proofs.rs",
    find: "                (self.tested_local(state, &given.node, Some(local))? == local).then_some(present)\n",
    replace: "                Some(present)\n",
    tests: [
      "test/lowering-nullable-truthy.test.ts",
      "test/lowering-nullable-tests.test.ts"
    ]
  },
  {
    name: "proved-present-written-again",
    breaks: "an Option written again after its truthy test is taken for the value it tested, `found = second; return found` given `undefined` where Rust's `None` is `null` (#1902)",
    file: "src/lower/mir/nulled/proofs.rs",
    find: "(once || self.unwritten_since(state, flag, local, read))",
    replace: "true",
    tests: [
      "test/lowering-nullable-truthy.test.ts",
      "test/lowering-nullable-tests.test.ts"
    ]
  },
  {
    name: "proved-present-copies-unfollowed",
    breaks: "the test MIR makes of a copy, `_11 = copy _3; js::truthy(move _11)`, proves nothing of the variable it copies (#1902)",
    file: "src/lower/mir/nulled/proofs.rs",
    find: "                ) if of.projection.is_empty() => {\n                    tested = of.local;\n",
    replace: "                ) if of.projection.is_empty() => {\n                    let _ = of;\n                    break;\n",
    tests: [
      "test/lowering-nullable-truthy.test.ts",
      "test/lowering-nullable-tests.test.ts"
    ]
  },
  {
    name: "proved-present-reborrow-unfollowed",
    breaks: "a test given a reborrow, `&(*_13)` of the promoted `&\"function\"`, says nothing of its text, so `js::type_of(x) == \"function\"` proves nothing (#1933)",
    file: "src/lower/mir/nulled/proofs.rs",
    find: "                    if of.projection[..] == [mir::ProjectionElem::Deref] =>\n                {\n                    tested = of.local;\n",
    replace: "                    if of.projection[..] == [mir::ProjectionElem::Deref] =>\n                {\n                    break;\n",
    tests: [
      "test/lowering-nullable-tests.test.ts"
    ]
  },
  {
    name: "proved-present-is-none-either-edge",
    breaks: "`x.is_none()`'s true edge is taken for one `x` is there on, `if x.is_none() && flag { return x; }` given `undefined` where Rust's `None` is `null` (#1933)",
    file: "src/lower/mir/nulled/proofs.rs",
    find: "                        false => *no,\n",
    replace: "                        false => targets.otherwise(),\n",
    tests: [
      "test/lowering-nullable-tests.test.ts"
    ]
  },
  {
    name: "proved-present-none-arm",
    breaks: "a `match` of an `Option`'s `None` arm is taken for its `Some`'s (#1933)",
    file: "src/lower/mir/nulled/proofs.rs",
    find: "                    [(0, _)] => Some(targets.otherwise()),\n",
    replace: "                    [(0, none)] => Some(*none),\n",
    tests: [
      "test/lowering-nullable-tests.test.ts"
    ]
  },
  {
    name: "proved-present-typeof-object",
    breaks: "`js::type_of(x) == \"object\"`, which `null` passes, is taken to prove `x` there (#1933)",
    file: "src/lower/mir/nulled/proofs.rs",
    find: " && tested.1 != \"object\")",
    replace: ")",
    tests: [
      "test/lowering-nullable-tests.test.ts"
    ]
  },
  {
    name: "proved-present-discriminant-unread",
    breaks: "a `match` or `if let` of an `Option` proves nothing of it, `return value ?? null` where its `Some` arm holds (#1933)",
    file: "src/lower/mir/nulled/proofs.rs",
    find: "            [Write::Assign(Rvalue::Discriminant(of))] if of.projection.is_empty() && of.local == local => {\n",
    replace: "            [Write::Assign(Rvalue::Discriminant(of))] if false && of.local == local => {\n",
    tests: [
      "test/lowering-nullable-tests.test.ts"
    ]
  },
  {
    name: "proved-present-false-edges-counted",
    breaks: "a `match`'s false edge, which no run takes, is counted as a way into an arm, so `None => None, _ if flag => value` proves nothing of `value` (#1933)",
    file: "src/lower/mir/nulled/proofs.rs",
    find: "                    .filter(|&&from| {\n",
    replace: "                    .filter(|&&from| true || {\n",
    tests: [
      "test/lowering-nullable-tests.test.ts"
    ]
  },
  {
    name: "cleanup-write-counted",
    breaks: "a cleanup's write of a local, the value its body's write gives as the old one's drop unwinds, is counted beside it, so a `let mut` written `js::null()` and then a nullable function's result is returned `active ?? null` (#1933)",
    file: "src/lower/mir/nulled/proofs.rs",
    find: "                            if !data.is_cleanup {\n",
    replace: "                            if true {\n",
    tests: [
      "test/lowering-nullable-tests.test.ts",
      "-t",
      "old value and get"
    ]
  },
  {
    name: "proved-many-writes-never",
    breaks: "a variable written more than once, a `match`'s in each arm, is never proved by its test, so `return found ?? null` where nothing writes it after its test (#1933)",
    file: "src/lower/mir/nulled/proofs.rs",
    find: "(once || self.unwritten_since(state, flag, local, read))",
    replace: "once",
    tests: [
      "test/lowering-nullable-tests.test.ts",
      "-t",
      "written more than once"
    ]
  },
  {
    name: "field-read-back-through-the-test",
    breaks: "the ways back from the read are followed through the test again, so a loop's body, `node = parent`, is between, and a `while .. is_none()`'s exit proves nothing (#1933)",
    file: "src/lower/mir/nulled/proofs.rs",
    find: "            if b == test || !behind.insert(b) {\n",
    replace: "            if !behind.insert(b) {\n",
    tests: [
      "test/lowering-nullable-tests.test.ts",
      "-t",
      "a RefCell field tested and read again"
    ]
  },
  {
    name: "kept-lend-never",
    breaks: "any `&mut` lend of an `Option` local makes its writes say nothing, so getParentFragmentInstances' `match &mut found` returns `found ?? null`, tidy's to drop (#1933)",
    file: "src/lower/mir/nulled/proofs.rs",
    find: "                            && !self.lend_keeps_some(state, *lent, assign.0)\n",
    replace: "",
    tests: [
      "test/lowering-nullable-tests.test.ts",
      "-t",
      "a &mut lend that never takes a Some away"
    ]
  },
  {
    name: "classified-present-edge-off",
    breaks: "an arm of a switch on `js::classify(x)` that only a value neither `undefined` nor `null` takes proves nothing, so React's Flight register's `if let Kind::Function(_) = js::classify(exports) { register(exports.unwrap()) }` is `$unwrap(exports)` (#1933)",
    file: "src/lower/mir/nulled/proofs.rs",
    find: "                self.classified_present_edge(state, of.local, targets, local)\n",
    replace: "                None\n",
    tests: [
      "test/lowering-nullable-classified.test.ts",
      "-t",
      "classify arm neither undefined nor null"
    ]
  },
  {
    name: "proved-copy-never",
    breaks: "a temporary that moves what a test proved, React's Flight client's `deferred` moved into one under `deferred.is_some()` before `*value = deferred`, is taken for one nothing proved, `?? null` (#1933)",
    file: "src/lower/mir/nulled/proofs.rs",
    find: "            && written.is_some_and(|at| {\n",
    replace: "            && written.is_some_and(|_| false)\n            && written.is_some_and(|at| {\n",
    tests: [
      "test/lowering-nullable-tests.test.ts",
      "-t",
      "a variable a part of an && shows there"
    ]
  },
  {
    name: "passes-through-no-cast",
    breaks: "`js::cast(found)` of an `Option` isn't the `Option` it's given to what's proved of it, so a nullable function returns it `?? null` under `js::truthy(found)` (#2112 C S4)",
    file: "src/lower/mir/nulled/proofs.rs",
    find: "        let cast = self.is_its_argument(callee)\n",
    replace: "        let cast = false && self.is_its_argument(callee)\n",
    tests: ["test/lowering-nullable-truthy.test.ts"],
  },
  {
    name: "passes-through-no-map",
    breaks: "`Option::map`'s result isn't proved where its receiver is, so `createDetails`' `cancelable.map(js::boolean).unwrap_or(true)` of a parameter JS defaults is coalesced (#2112 C S4)",
    file: "src/lower/mir/nulled/proofs.rs",
    find: "        (mapped || cast).then_some(from)\n",
    replace: "        cast.then_some(from)\n",
    tests: ["test/create-details.test.ts", "-t", "upstream's body"],
  },
  {
    name: "ended-present-unmarked-in-place",
    breaks: "a `last()` proved `Some`, read in place, `keys.at(-1)`, isn't marked proved, so its `unwrap()` keeps `$unwrap` (#1860)",
    file: "src/lower/mir/nulled/proofs.rs",
    find: "        if self.ended_present(state, local) || self.present_on_all_paths(state, local, state.at) {\n",
    replace: "        if self.present_on_all_paths(state, local, state.at) {\n",
    tests: ["test/lowering-collections.test.ts", "-t", "a split collected straight"],
  },
];
