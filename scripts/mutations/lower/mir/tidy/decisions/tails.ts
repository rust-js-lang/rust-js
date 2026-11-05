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
    find: "        && let Some((chain, _)) = chain((label, list[k].span), &list[k..], &[], told, true)\n",
    replace: "        && let Some((chain, _)) = chain((label, list[k].span), &list[k..], &[], told, false)\n",
    tests: ["test/corpus.test.ts", "-t", "^(interpreter).rs$"],
    snapshots: true,
  },
];
