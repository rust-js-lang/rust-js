// Mutations of src/library.rs (ADR 0093).
import type { Mutation } from "../mutations";

export const mutations: Mutation[] = [
  {
    name: "contract-counts-unread",
    breaks: "a library's contract read as counting every `Rc` whatever it says, so its consumer counts what it hands as a box (ADRs 0844, 0848)",
    file: "src/library.rs",
    find: "                counted_rcs: library.counted_rcs,\n                impls:",
    replace: "                counted_rcs: vec![RcShape { of: None, args: Vec::new() }],\n                impls:",
    tests: ["test/crates.test.ts", "-t", "two crates: rc_per_type"],
  },
  {
    name: "library-libraries-unchecked",
    breaks: "a consumer given a library but not the libraries that one was compiled against is compiled anyway",
    file: "src/library.rs",
    find: "        if let Some((library, used)) = needed.iter().find(|(_, used)| !result.libraries.contains_key(used)) {\n",
    replace: "        if let Some((library, used)) = needed.iter().find(|_| false) {\n",
    tests: ["test/crates.test.ts", "-t", "not the libraries"],
  },
  {
    name: "library-inputs-unchecked",
    breaks: "a library's manifest's inputs, the libraries it was made from, aren't checked: one made from another's old build is refused only by rustc's E0463, which doesn't say it's stale",
    file: "src/library.rs",
    find: "            for artifact in manifest.artifacts.iter().chain(&library.inputs) {",
    replace: "            for artifact in manifest.artifacts.iter() {",
    tests: ["test/crates.test.ts", "-t", "an edited library is what its consumers use once they're rebuilt, and refused before"],
  },
  {
    name: "manifest-writers-options-dropped",
    breaks: "a library's manifest doesn't say its writers take options: a consumer gives them none",
    file: "src/library.rs",
    find: "                writers_take_options: library.writers_take_options,\n",
    replace: "                writers_take_options: false,\n",
    tests: ["test/crates.test.ts", "-t", "as its own crate takes them"],
  },
  {
    name: "library-old-abi-accepted",
    breaks: "a library of the ABI before every `Rc` was counted is loaded as if it counted them",
    file: "src/library.rs",
    find: "            if library.version != VERSION {\n",
    replace: "            if library.version != VERSION && library.version != 2 {\n",
    tests: ["test/cargo-link.test.ts", "-t", "a real Cargo dependency links functions"],
  },
  {
    name: "library-default-imported-by-name",
    breaks: "a library's module's default is imported by its name, `import { noop }`, which its module doesn't export",
    file: "src/library.rs",
    find: "        match self.default {\n",
    replace: "        match false {\n",
    tests: ["test/cross-crate-items.test.ts", "-t", "default export is imported as its default"],
  },
];
