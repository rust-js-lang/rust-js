// Mutations of src/lower/library.rs (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "library-crate-hash-unchecked",
    breaks: "a library's manifest is taken beside the metadata of another build of it",
    file: "src/lower/library.rs",
    find: "            if let Some(library) = self.dependencies.libraries.get(name.as_str())\n",
    replace: "            if let Some(library) = self.dependencies.libraries.get(name.as_str()).filter(|_| false)\n",
    tests: ["test/crates.test.ts", "-t", "another build"],
  },
  {
    name: "library-fails-unlisted",
    breaks: "a library's writer that may fail is listed as one that never does",
    file: "src/lower/library.rs",
    find: "            fails: failing.contains(&id),",
    replace: "            fails: false && failing.contains(&id),",
    tests: ["test/crates.test.ts","-t","may fail is one its consumer"],
  },
  {
    name: "library-no-drops-unlisted",
    breaks: "a library's generic fold is given a destructor by its consumer",
    file: "src/lower/library.rs",
    find: "            no_drops: no_drops\n                .get(&id)\n                .map(|params| params.iter().copied().collect())\n                .unwrap_or_default(),",
    replace: "            no_drops: Vec::new(),",
    tests: ["test/crates.test.ts","-t","no destructor by its consumers"],
  },
  {
    name: "library-drops-unlisted",
    breaks: "a library's manifest says none of its functions takes a drop, so a consumer gives `Sink.put` none, and its value isn't dropped",
    file: "src/lower/library.rs",
    find: "            drops: drop_params.get(&id).cloned().unwrap_or_default(),",
    replace: "            drops: Vec::new(),",
    tests: ["test/crates.test.ts", "-t", "a library's generic trait method drops a consumer's value where Rust does"],
  },
];
