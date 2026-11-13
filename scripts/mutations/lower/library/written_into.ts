// Mutations of src/lower/library/written_into.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "library-writer-fn-unframed-here",
    breaks: "a library's writer function a consumer's framed writer gives its `Formatter` is refused, though its library frames it",
    file: "src/lower/library/written_into.rs",
    find: "                .is_some_and(|library| !library.builtin && library.framed_writers.contains(&key(self.tcx, id)))\n",
    replace: "                .is_some_and(|_| false)\n",
    tests: ["test/crates-compiled-here.test.ts", "-t", "a library's writer given as a value is its import"],
  },
  {
    name: "trait-written-arguments-unread",
    breaks: "a library's trait's method that writes the `Arguments` it's given, called through a `dyn`, is given a template, so a writer it shows claims the sink and its own pieces are lost (#1735)",
    file: "src/lower/library/written_into.rs",
    find: "                    .and_then(|library| library.written_trait_arguments.get(&key(self.tcx, id)).cloned())\n",
    replace: "                    .and_then(|_| None::<Vec<u32>>)\n",
    tests: ["test/writer-sink.test.ts", "-t", "a consumer's Arguments given a library's trait's method through a dyn"],
  },
];
