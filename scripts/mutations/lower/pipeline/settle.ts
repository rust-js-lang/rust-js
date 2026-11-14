// Mutations of src/lower/pipeline/settle.rs (ADR 0093): the rounds that
// settle what lowering finds, bounded (ADR 1148).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "settle-unbounded",
    breaks: "a protocol that never settles is lowered again without end, never refused (#1671 change 4, M2)",
    file: "src/lower/pipeline/settle.rs",
    find: "        if *taken > limit {\n",
    replace: "        if false && *taken > limit {\n",
    unitTests: "lower::pipeline::settle::tests::a_protocol_that_never_settles_is_refused_by_name",
  },
  {
    name: "settle-codecs-bounded-as-the-others",
    breaks: "a chain of codecs deeper than the bound, a type in a type in a type, each serde's, is refused as unsettled, though each round lowers one more (#1671 change 4, M2)",
    file: "src/lower/pipeline/settle.rs",
    find: "            Protocol::Codecs => LIMIT.max(self.codec_count),\n",
    replace: "            Protocol::Codecs => LIMIT,\n",
    unitTests: "lower::pipeline::settle::tests::the_codecs_take_a_round_each",
  },
];
