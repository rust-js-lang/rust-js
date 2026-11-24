// Mutations of crates/rust-js-lower/src/analysis/callers_decide.rs (ADR 1014).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "generic-zip-fold-allowed",
    breaks:
      "generic code given a zip of a map it collects steps its first side once more where std's fold stops at the shorter, running the map's closure Rust leaves unrun",
    file: "crates/rust-js-lower/src/analysis/callers_decide.rs",
    find: "                                _ => by_index.is_some(),\n",
    replace: "                                _ => false,\n",
    tests: [
      "test/diagnostics.test.ts",
      "-t",
      "a zip of a map collected in generic code",
    ],
  },
  {
    name: "generic-zip-step-allowed",
    breaks:
      "generic code given a zip of a map of sides std reads by index, stepped by `next()`, steps its first side once more where std's `next()` stops at the shorter",
    file: "crates/rust-js-lower/src/analysis/callers_decide.rs",
    find: "                                Asked::ZipStep(_) => by_index == Some(true),\n",
    replace: "                                Asked::ZipStep(_) => false,\n",
    tests: [
      "test/diagnostics.test.ts",
      "-t",
      "a zip of a map stepped in generic code",
    ],
  },
  {
    name: "generic-trusted-eq-allowed",
    breaks:
      "generic code given two `TrustedLen` maps it compares by `eq` compares them item by item, running their closures where Rust compares their lengths first and runs nothing (#386)",
    file: "crates/rust-js-lower/src/analysis/callers_decide.rs",
    find: "                            if !trusted || !(runs_code(a) || runs_code(b)) {\n",
    replace: "                            if true {\n",
    tests: ["test/diagnostics.test.ts", "-t", "eq in generic code of maps it builds"],
  },
  {
    name: "closure-zip-asks-its-own",
    breaks: "a zip generic code steps in a closure of its own, `s.map(|i| i.zip(..).count())`, is the closure's ask, which no caller calls, so its caller's map closure runs once more than std's (#2542)",
    file: "crates/rust-js-lower/src/analysis/callers_decide.rs",
    find: "        let owner = tcx.typeck_root_def_id(body.def_id);\n",
    replace: "        let owner = body.def_id;\n",
    tests: ["test/diagnostics.test.ts", "-t", "a zip of an item in a closure of generic code"],
  },
  {
    name: "library-zip-asks-unread",
    breaks: "a library's generic zip isn't read of its MIR, so its consumer's map closure runs once more than std's, where one crate's own generic code is refused (#2542)",
    file: "crates/rust-js-lower/src/analysis/callers_decide.rs",
    find: "        .filter(|&id| !id.is_local() && !crate::recognition::is_std_crate(tcx, id) && tcx.generics_of(id).count() > 0)\n",
    replace: "        .filter(|_| false)\n",
    tests: ["test/crates-refused.test.ts", "-t", "zip a closure std stops short"],
  },
  {
    name: "library-chain-asks-uncarried",
    breaks: "a library's function calling another of its own that zips isn't carried to its consumer's call, which runs the map closure once more than std's (#2542)",
    file: "crates/rust-js-lower/src/analysis/callers_decide.rs",
    find: "                    callers.entry(callee).or_default().push((owner, args, *fn_span));\n",
    replace: "",
    tests: ["test/crates-refused.test.ts", "-t", "zip a closure std stops short"],
  },
];
