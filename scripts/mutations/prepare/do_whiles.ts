// Mutations of src/prepare/do_whiles.rs (ADR 0093).
import type { Mutation } from "../../mutations";

const corpus = ["test/corpus.test.ts", "-t", "^do_while_advances.rs$"];

export const mutations: Mutation[] = [
  {
    name: "do-while-never",
    breaks: "React's updateReducerImpl's walk stays `while (true) { .. if (..) { update = update.next; continue; } .. break; }`, not upstream's `do { .. } while (..)` (ADR 0978)",
    file: "src/prepare/do_whiles.rs",
    find: "    if !declared\n",
    replace: "    if true || !declared\n",
    tests: corpus,
    snapshots: true,
  },
  {
    name: "do-while-read-after",
    breaks: "a `do` whose variable is read after it, `return total + update.lane`, reads the value its test refused (ADR 0978)",
    file: "src/prepare/do_whiles.rs",
    find: "        || js::mentions_in(after, variable) > 0\n",
    replace: "",
    tests: corpus,
  },
  {
    name: "do-while-tests-unlike",
    breaks: "a `loop` whose ways advance on two tests is a `do` of the first's, which the other way never asked (ADR 0978)",
    file: "src/prepare/do_whiles.rs",
    find: "a.variable != *variable || !same(&a.test, &first.test)",
    replace: "a.variable != *variable",
    tests: corpus,
  },
  {
    name: "do-while-test-reads-variable",
    breaks: "a test that reads the variable apart from its next value, `next.lane > update.lane`, is the `do`'s test of it given that value, `update.lane > update.lane` (ADR 0978)",
    file: "src/prepare/do_whiles.rs",
    find: "    if holed.mentions_var(variable) || !holed.mentions_var(HOLE) {\n",
    replace: "    if !holed.mentions_var(HOLE) {\n",
    tests: corpus,
  },
];
