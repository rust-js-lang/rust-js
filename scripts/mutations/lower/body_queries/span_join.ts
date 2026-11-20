// Mutations of src/lower/body_queries/span_join.rs (ADR 0093): the joins
// of MIR to THIR by a span, rustc's only key (#1671, #2158, #2175).
import type { Mutation } from "../../../mutations";

const held = ["test/corpus.test.ts", "-t", "^held_chain_beside_another_value_stepped_from_its_back.rs$"];

export const mutations: Mutation[] = [
  {
    name: "var-local-unjoined",
    breaks: "a variable's MIR local isn't found, so the held flip takes a variable no one steps from its back, `apart`, as made knowing it and leaves its chain hand-written (#2158)",
    file: "src/lower/body_queries/span_join.rs",
    find: "        mir::VarDebugInfoContents::Place(place) if place.projection.is_empty() && info.source_info.span == span => {\n",
    replace: "        mir::VarDebugInfoContents::Place(place) if false => {\n",
    snapshots: true,
    tests: held,
  },
  {
    name: "call-destination-of-any-callee",
    breaks: "a call's destination is joined by its span alone, where a `for` loop's `into_iter` shares it, so the chain it iterates is no one local's, made knowing, and hand-written where std's struct was (#2158)",
    file: "src/lower/body_queries/span_join.rs",
    find: "                } if *fn_span == span && func.const_fn_def().is_some_and(|(id, _)| id == callee) => Some(*destination),\n",
    replace: "                } if *fn_span == span && func.const_fn_def().is_some() => Some(*destination),\n",
    snapshots: true,
    tests: ["test/corpus.test.ts", "-t", "^btree_nodes_unwinds.rs$"],
  },
  {
    name: "assigned-local-unjoined",
    breaks: "a value made where it's written, `Ends(1, 4)`, isn't joined to the local it's made in, so it's made knowing and its chain hand-written beside another `Ends` stepped from its back (#2158)",
    file: "src/lower/body_queries/span_join.rs",
    find: "                    if statement.source_info.span == span && assign.0.projection.is_empty() =>\n",
    replace: "                    if false =>\n",
    snapshots: true,
    tests: held,
  },
];
