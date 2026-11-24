// Mutations of crates/rust-js-lower/src/analysis/caller_locations.rs (ADR 0427).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "caller-read-unseen",
    breaks:
      "a tracked function that reads `Location::caller()` takes no location, and reads none",
    file: "crates/rust-js-lower/src/analysis/caller_locations.rs",
    find: "            if location_of(tcx, callee) == Some(LocationOp::Caller) {\n                taking.insert(owner);\n",
    replace: "            if location_of(tcx, callee) == Some(LocationOp::Caller) {\n                let _ = owner;\n",
    tests: ["test/corpus.test.ts", "-t", "^track_caller.rs$"],
  },
  {
    name: "caller-passed-on-unseen",
    breaks:
      "a tracked function that calls one taking its location takes none, and gives its own place instead",
    file: "crates/rust-js-lower/src/analysis/caller_locations.rs",
    find: "            if takes_location(tcx, &taking, foreign, callee) && taking.insert(caller) {\n",
    replace: "            if false && taking.insert(caller) {\n",
    tests: ["test/corpus.test.ts", "-t", "^track_caller.rs$"],
  },
  {
    name: "dictionary-location-untaken",
    breaks:
      "a trait method one of whose impls reads its caller's location is called through a dictionary without it",
    file: "crates/rust-js-lower/src/analysis/caller_locations.rs",
    find: "            if takes {\n                changed |= taking.insert(declared);\n            }\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^track_caller_traits.rs$"],
  },
  {
    name: "dictionary-call-not-passed-on",
    breaks:
      "a tracked function calling a trait method through a dictionary passes on no location",
    file: "crates/rust-js-lower/src/analysis/caller_locations.rs",
    find: "                _ => callee,\n",
    replace: "                _ => continue,\n",
    tests: ["test/corpus.test.ts", "-t", "^track_caller_traits.rs$"],
  },
  {
    name: "tracked-assert-raises-at-its-own",
    breaks: "a tracked function whose own `Assert` raises a panic, a division's, takes no location, so its panic names its own line where native Rust names its caller's (#2350)",
    file: "crates/rust-js-lower/src/analysis/caller_locations.rs",
    find: "                        TerminatorKind::Assert { .. } => {\n                            taking.insert(owner);\n                        }\n",
    replace: "                        TerminatorKind::Assert { .. } => {}\n",
    tests: ["test/corpus.test.ts", "-t", "^track_caller_raises.rs$"],
    snapshots: true,
  },
  {
    name: "tracked-std-call-raises-at-its-own",
    breaks: "a tracked function calling std's that tracks its caller, `panic!`'s, `unwrap`, a `Vec`'s index, takes no location, so its panic names its own line (#2350)",
    file: "crates/rust-js-lower/src/analysis/caller_locations.rs",
    find: "            if tracks_caller(tcx, resolved) && !compiled.contains(&resolved)",
    replace: "            if false && tracks_caller(tcx, resolved) && !compiled.contains(&resolved)",
    tests: ["test/corpus.test.ts", "-t", "^track_caller_raises.rs$"],
  },
  {
    name: "compiled-callee-taken-as-std",
    breaks: "a tracked function calling the crate's that raises nothing takes a location it doesn't read, `calm(x, location)` (#2350)",
    file: "crates/rust-js-lower/src/analysis/caller_locations.rs",
    find: " && !compiled.contains(&resolved) && foreign.item(resolved).is_none() {",
    replace: " {",
    tests: ["test/corpus.test.ts", "-t", "^track_caller.rs$"],
    snapshots: true,
  },
  {
    name: "tracked-calls-unscanned",
    breaks: "what a tracked function calls by its MIR isn't seen, so one that passes its location on, or reads `Location::caller()`, takes none",
    file: "crates/rust-js-lower/src/analysis/caller_locations.rs",
    find: "                        TerminatorKind::Call { func, .. } => called.extend(func.const_fn_def()),\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^track_caller.rs$"],
  },
  {
    name: "tracked-fn-dividing-generically-untaken",
    breaks: "a `#[track_caller]` function that divides through a dictionary takes no location, so its `/` names its own line, not where it was called (#2457)",
    file: "crates/rust-js-lower/src/analysis/caller_locations.rs",
    find: "            if crate::recognition::model_raises_at_caller(tcx, resolved) {\n",
    replace: "            if false && crate::recognition::model_raises_at_caller(tcx, resolved) {\n",
    tests: ["test/binaries.test.ts", "-t", "through a dictionary, names generic"],
  },
];
