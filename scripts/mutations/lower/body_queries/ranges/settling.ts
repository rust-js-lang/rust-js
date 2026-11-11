// Mutations of src/lower/body_queries/ranges/settling.rs (ADR 0093): each
// transition of how what a function only calls reach is told settles (#873).
import type { Mutation } from "../../../../mutations";

const unit = "lower::body_queries::ranges::settling";

export const mutations: Mutation[] = [
  {
    name: "settling-never-widens",
    breaks: "what a function only calls reach is told keeps joining past four times, what still changes never forgotten (#873)",
    file: "src/lower/body_queries/ranges/settling.rs",
    find: "                times if times >= WIDENS_AFTER => Settling::Widening(times),\n",
    replace: "",
    unitTests: unit,
  },
  {
    name: "settling-never-gives-up",
    breaks: "what a function only calls reach is told keeps widening past 64 times, never given up (#873)",
    file: "src/lower/body_queries/ranges/settling.rs",
    find: "                times if times >= GIVES_UP_AFTER => Settling::GivenUp,\n",
    replace: "",
    unitTests: unit,
  },
  {
    name: "settling-given-up-joins-again",
    breaks: "a function given up on is counted back to joining (#873)",
    file: "src/lower/body_queries/ranges/settling.rs",
    find: "            Settling::GivenUp => Settling::GivenUp,\n",
    replace: "            Settling::GivenUp => Settling::Joining(0),\n",
    unitTests: unit,
  },
  {
    name: "settling-given-up-told-its-calls",
    breaks: "a function given up on is told what its calls tell it, not nothing (#873)",
    file: "src/lower/body_queries/ranges/settling.rs",
    find: "            (Settling::GivenUp, _, _) => Some(Known::default()),\n",
    replace: "",
    unitTests: unit,
  },
  {
    name: "settling-widening-keeps-what-changes",
    breaks: "a function told otherwise four times is told all its calls tell it, what still changes kept, so a range that grows each walk is never forgotten (#873)",
    file: "src/lower/body_queries/ranges/settling.rs",
    find: "            (Settling::Widening(_), Some(mut next), Some(was)) => {\n",
    replace: "            (Settling::Widening(_), Some(mut next), Some(was)) if false => {\n",
    tests: ["test/corpus.test.ts", "-t", "^ranges_kept_of_what_stops_changing.rs$"],
    snapshots: true,
  },
];
