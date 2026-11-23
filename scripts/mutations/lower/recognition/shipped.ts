// Mutations of crates/rust-js-lower/src/recognition/shipped.rs (ADR 1185).
import type { Mutation } from "../../../mutations";

const file = "crates/rust-js-lower/src/recognition/shipped.rs";

export const mutations: Mutation[] = [
  {
    name: "qualified-version-unread",
    breaks: "a qualified getrandom's version is never read, so each of its calls is refused as one of a getrandom rust-js doesn't know",
    file,
    find: "    let text = std::fs::read_to_string(rust_js_program::library::qualified(&metadata)).ok()?;\n",
    replace: "    let text: String = std::fs::read_to_string(&metadata).ok().filter(|_| false)?;\n",
    tests: ["test/shipped-crates.test.ts", "-t", "getrandom's API runs"],
  },
  {
    name: "getrandom-unqualified-read-as-qualified",
    breaks: "a getrandom no qualified check checked, a plain Cargo build's, is read as 0.1's, its version a guess",
    file,
    find: "    GetrandomVersion::of(&qualified_version(tcx, krate)?)\n",
    replace: "    GetrandomVersion::of(&qualified_version(tcx, krate).unwrap_or_else(|| \"0.1.16\".to_string()))\n",
    tests: ["test/shipped-crates.test.ts", "-t", "getrandom rust-js doesn't write"],
  },
  {
    name: "getrandom-from-code-unrecognized",
    breaks: "0.1's and 0.2's `Error::from(code)` is refused, a call of getrandom's `From` impl",
    file,
    find: "                .filter(|&version| version.has_code())\n",
    replace: "                .filter(|_| false)\n",
    tests: ["test/shipped-crates.test.ts", "-t", "getrandom's API runs"],
  },
];
