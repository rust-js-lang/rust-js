// Mutations of src/lower/analysis/unwinding/eager_futures.rs (ADR 0093, #965).
import type { Mutation } from "../../../../mutations";

const held = ["test/corpus.test.ts", "-t", "^future_held_past_what_is_seen_refused.rs$"];
const dropped = ["test/corpus.test.ts", "-t", "^future_dropped_unawaited_refused.rs$"];
const kept = ["test/corpus.test.ts", "-t", "^(future_held_where_nothing_shows|future_sizes|future_size_unwinds).rs$"];

export const mutations: Mutation[] = [
  {
    name: "held-future-awaited-after-seen-kept",
    breaks: "a future of the crate's held while something that prints runs, then awaited, compiles, and its body runs before the print in JS, where Rust runs it at its `.await` (a wrong answer, #965)",
    file: "src/lower/analysis/unwinding/eager_futures.rs",
    find: "                        if let Some(seen) = seen {\n                            return Some(Fate::AwaitedAfter(seen));\n                        }\n",
    replace: "                        let _ = seen;\n",
    tests: held,
  },
  {
    name: "future-dropped-by-drop-kept",
    breaks: "a future of the crate's given to `drop` unawaited compiles, and its body runs in JS, where Rust's never does (a wrong answer, #965)",
    file: "src/lower/analysis/unwinding/eager_futures.rs",
    find: "                        if crate::lower::recognition::is_mem_drop(self.tcx, def) {\n                            return Some(Fate::Dropped(span));\n                        }\n",
    replace: "",
    tests: dropped,
  },
  {
    name: "every-future-followed",
    breaks: "a future whose body can't be seen is refused where it's held or dropped unawaited, though running it as it's made can't be told from Rust's (#965)",
    file: "src/lower/analysis/unwinding/eager_futures.rs",
    find: "        let noisy = |ty: Ty<'tcx>| self.future_body(ty).is_some() && !self.quiet_instance(ty, 0);\n",
    replace: "        let noisy = |ty: Ty<'tcx>| self.future_body(ty).is_some();\n",
    tests: kept,
  },
  {
    name: "pinned-future-lost",
    breaks: "a future `pin!`ned, written into its helper's field, is refused as given where it can't be followed (#965)",
    file: "src/lower/analysis/unwinding/eager_futures.rs",
    find: "                    if !to.is_indirect() {\n                        holding.insert(to.local);\n",
    replace: "                    if to.projection.is_empty() {\n                        holding.insert(to.local);\n",
    tests: ["test/corpus.test.ts", "-t", "^future_held_in_a_field_refused.rs$"],
  },
  {
    name: "future-carried-by-the-crates-fn",
    breaks: "a future given to the crate's `async fn later(f: F)`, which prints before it awaits `f`, is followed as what `later`'s future holds, so it compiles, and JS prints the future's body before `later`'s print, where Rust prints it after (a wrong answer, #965)",
    file: "src/lower/analysis/unwinding/eager_futures.rs",
    find: "                        let carried = crate::lower::recognition::is_std_crate(self.tcx, def)\n                            && held.walk()",
    replace: "                        let carried = held.walk()",
    tests: ["test/limitations.test.ts", "-t", "may await it later"],
  },
];
