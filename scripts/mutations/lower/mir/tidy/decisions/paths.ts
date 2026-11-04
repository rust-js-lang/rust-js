// Mutations of src/lower/mir/tidy/decisions/paths.rs (ADR 0093).
import type { Mutation } from "../../../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "written-order-of-the-alternative-alone",
    breaks:
      "a tuple's parts' tests are ordered by where their alternative is alone, all alike, so MIR's `b === 3 && a === 2` stays, not the order of the tuple's parts, `a === 2 && b === 3` (#632)",
    file: "src/lower/mir/tidy/decisions/paths.rs",
    find: "    let mut keyed: Vec<((u32, f64), Expr)> = (parts.into_iter().map(|(at, _, n)| (at, n)))\n",
    replace: "    let mut keyed: Vec<((u32, f64), Expr)> = (parts.into_iter().map(|(at, _, n)| (at, n * 0.0)))\n",
    tests: ["test/lowering-match-paths.test.ts", "-t", "alternatives test a tuple's parts in the order written"],
  },
  {
    name: "or-pattern-alternatives-say-nothing",
    breaks:
      "a later arm of a tuple's or-pattern after `(1, _) | (_, 1)` keeps `a !== 1` before `(b === 3 && a === 2) || ..`, which says it, more tests than the tree's, so the arms are MIR's `if (a !== 1 && b !== 1) { .. continue; } hit = true;`, not the chain as written (#617)",
    file: "src/lower/mir/tidy/decisions/paths.rs",
    find: "        ExprKind::Binary(Op::Or, a, b) => says_within(a, subject, set) && says_within(b, subject, set),\n",
    replace: "        ExprKind::Binary(Op::Or, a, b) => false,\n",
    tests: ["test/lowering-comments-inside.test.ts", "-t", "tuple's or-pattern is the arm's"],
  },
  {
    name: "or-pattern-alternative-tests-unread",
    breaks:
      "an or-pattern's alternative, `b === 3 && a === 2`, says nothing of `a` past its first test, so `a !== 1` before them stays and the arms are MIR's tree, not the chain as written (#617)",
    file: "src/lower/mir/tidy/decisions/paths.rs",
    find: "        ExprKind::Binary(Op::And, a, b) => says_within(a, subject, set) || says_within(b, subject, set),\n",
    replace: "        ExprKind::Binary(Op::And, a, b) => says_within(a, subject, set),\n",
    tests: ["test/lowering-comments-inside.test.ts", "-t", "tuple's or-pattern is the arm's"],
  },
  {
    name: "decisions-impossible-path-kept",
    breaks:
      'a path no value takes, `c < "0" && c >= "A"`, is one of an arm\'s tests',
    file: "src/lower/mir/tidy/decisions/paths.rs",
    find: "        if now.is_empty() {\n            return None;\n        }\n",
    replace: "",
    tests: ["test/lowering-matches.test.ts", "-t", "arms of more than one pattern"],
  },
  {
    name: "decisions-stronger-test-not-first",
    breaks:
      "`(2, 2)`'s path is `a !== 1 && b === 2 && a === 2`, longer than the tree, which stays",
    file: "src/lower/mir/tidy/decisions/paths.rs",
    find: "            Some(j) => out[i] = out.remove(j),\n",
    replace: "            Some(_) => i += 1,\n",
    tests: ["test/corpus.test.ts", "-t", "^default_value_exits.rs$"],
    snapshots: true,
  },
  {
    name: "paths-alike-but-for-two-values-one",
    breaks:
      "paths that differ in two tests, `a === 0 && b === 1` and `a === 1`, are one of the first's value's union, `(a === 0 || a === 1) && b === 1`, so `(1, 4)` takes the `_` arm, not native Rust's (#631)",
    file: "src/lower/mir/tidy/decisions/paths.rs",
    find: "            if p.len() != q.len() || p.len() < 2 {\n                return None;\n            }\n            let differ: Vec<usize> = (0..p.len()).filter(|&k| !p[k].written_alike(&q[k])).collect();\n            let [k] = differ.as_slice() else { return None };\n",
    replace: "            if p.len() < 2 || q.is_empty() {\n                return None;\n            }\n            let differ: Vec<usize> = (0..p.len().min(q.len())).filter(|&k| !p[k].written_alike(&q[k])).collect();\n            let [k, ..] = differ.as_slice() else { return None };\n",
    tests: ["test/corpus.test.ts", "-t", "^or_pattern_paths_absorbed.rs$"],
  },
  {
    name: "or-pattern-excluding-path-kept",
    breaks:
      "a path whose or-pattern's alternatives none can hold, `a === 0 && ((b === 3 && a === 2) || ..)`, is one of an arm's, more tests than the tree's, so the `match` is MIR's labeled block (#631)",
    file: "src/lower/mir/tidy/decisions/paths.rs",
    find: "    (!excluded).then_some(out)\n",
    replace: "    Some(out).filter(|_| excluded || !excluded)\n",
    tests: ["test/lowering-match-paths.test.ts", "-t", "after an arm that tests both values"],
  },
  {
    name: "or-pattern-excluding-one-alternative",
    breaks:
      "a path one of whose or-pattern's alternatives can hold, `a === 2 && ((b === 3 && a === 2) || (b === 2 && a === 3))`, is none, so `(2, 3)` after `(2, 0)` takes the `_` arm, not native Rust's (#631)",
    file: "src/lower/mir/tidy/decisions/paths.rs",
    find: "        ExprKind::Binary(Op::Or, a, b) => excludes(a, subject, set) && excludes(b, subject, set),\n",
    replace: "        ExprKind::Binary(Op::Or, a, b) => excludes(a, subject, set) || excludes(b, subject, set),\n",
    tests: ["test/corpus.test.ts", "-t", "^or_pattern_paths_absorbed.rs$"],
  },
  {
    name: "or-pattern-said-atom-kept",
    breaks:
      "a test an or-pattern's alternatives before it each say, `(..) && b !== 1`, stays, more tests than the tree's, so the `match` is MIR's labeled block (#631)",
    file: "src/lower/mir/tidy/decisions/paths.rs",
    find: "            None if said => drop(out.remove(i)),\n",
    replace: "            None if said && false => drop(out.remove(i)),\n",
    tests: ["test/lowering-match-paths.test.ts", "-t", "after an arm that tests both values"],
  },
];
