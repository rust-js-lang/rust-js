// Mutations of src/lower/mir/census.rs (#1461).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "label-census-loops-uncrossed",
    breaks: "the label census says a jump to a kept label crosses no loop, though it leaves one, so a row's reach is lost (#1461)",
    file: "src/lower/mir/census.rs",
    find: "                    found.loops = found\n                        .loops\n                        .max(crossed.iter().filter(|(a, _)| *a == Around::Loop).count());\n",
    replace: "                    found.loops = 0;\n",
    unitTests: "lower::mir::census::tests::each_compiler_label_kept_and_how_far_its_jumps_reach",
  },
  {
    name: "label-census-written-labels-counted",
    breaks: "the label census counts a label the program wrote, `outer`, among the compiler's, which no fix takes away (#1461)",
    file: "src/lower/mir/census.rs",
    find: "        let label = |label: &Option<String>| label.clone().filter(|name| super::queries::is_block_label(name));\n",
    replace: "        let label = |label: &Option<String>| label.clone();\n",
    unitTests: "lower::mir::census::tests::each_compiler_label_kept_and_how_far_its_jumps_reach",
  },
];
