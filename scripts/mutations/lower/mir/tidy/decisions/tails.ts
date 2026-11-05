// Mutations of src/lower/mir/tidy/decisions/tails.rs (#523, ADR 0364).
import type { Mutation } from "../../../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "arm-trees-unchained",
    breaks: "an arm's own `match` whose arms give the outer's value, `value = ..; break bb195;`, isn't chained there, so the interpreter's `eval` keeps `bb195:` (#523, C1)",
    file: "src/lower/mir/tidy/decisions/tails.rs",
    find: "    tails_in(label, list, told, true)\n",
    replace: "    None\n",
    tests: ["test/corpus.test.ts", "-t", "^(interpreter).rs$"],
    snapshots: true,
  },
  {
    name: "arm-trees-unkept",
    breaks: "an arm's own `match` is chained only where it reads no more tests than its tree, so the interpreter's keeps `bb195:` (#523)",
    file: "src/lower/mir/tidy/decisions/tails.rs",
    find: "            && let Some((chain, _)) = chain((label, list[k].span), &list[k..], &[], told, (n == 0, &[]))\n",
    replace: "            && let Some((chain, _)) = chain((label, list[k].span), &list[k..], &[], told, (false, &[]))\n",
    tests: ["test/corpus.test.ts", "-t", "^(interpreter).rs$"],
    snapshots: true,
  },
  {
    name: "arm-trees-from-the-first-leaving",
    breaks: "an arm's own `match` is chained only from the first statement that leaves the block, `if (e[0] === 0 && e[1] === 0) { tmp = 6; break bb39; }`, not past it where the inner tree is, so `inner_after_an_arm` keeps its labeled block (#771)",
    file: "src/lower/mir/tidy/decisions/tails.rs",
    find: "    for (n, k) in leaving.enumerate() {\n",
    replace: "    for (n, k) in leaving.enumerate().take(1) {\n",
    tests: ["test/corpus.test.ts", "-t", "^(match_trees_chained).rs$"],
    snapshots: true,
  },
  {
    name: "arm-trees-past-the-first-read-again",
    breaks: "an arm's own tree past the first statement that leaves the block is chained though it reads its tests again, so std's `Skip::next_back` is `if (tmp === value && value > 0) { .. } if (tmp === value && value <= 0) { return; }`, `tmp === value` read three times, not its labeled block (#771)",
    file: "src/lower/mir/tidy/decisions/tails.rs",
    find: "            && let Some((chain, _)) = chain((label, list[k].span), &list[k..], &[], told, (n == 0, &[]))\n",
    replace: "            && let Some((chain, _)) = chain((label, list[k].span), &list[k..], &[], told, (true, &[]))\n",
    tests: ["test/std-package.test.ts"],
  },
];
