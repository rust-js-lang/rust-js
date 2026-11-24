// Mutations of crates/rust-js-program/src/library/registry_sites.rs (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "registry-site-of-any-panic",
    breaks: "a registry crate's site is the first `panic!` of its file, whatever its message (#2325)",
    file: "crates/rust-js-program/src/library/registry_sites.rs",
    find: "        (args[..end].trim_start().starts_with(&quoted)).then(|| {\n",
    replace: "        true.then(|| {\n",
    unitTests: "library::registry_sites",
  },
  {
    name: "registry-site-column-from-0",
    breaks: "a registry crate's site's column counts from 0, one before rustc's hook's (#2325)",
    file: "crates/rust-js-program/src/library/registry_sites.rs",
    find: "                source[line_start..at].chars().count() + 1,\n",
    replace: "                source[line_start..at].chars().count(),\n",
    unitTests: "library::registry_sites",
  },
  {
    name: "registry-site-under-this-machine",
    breaks: "a fetched crate's file, a registry crate's site's or its own panic's, is named under this machine's `$CARGO_HOME`, so the JS differs by machine and ships the builder's home (#2325, #2435)",
    file: "crates/rust-js-program/src/library/registry_sites.rs",
    find: "    Some(format!(\"/cargo/{}\", parts[at..].join(\"/\")))\n",
    replace: "    Some(path.display().to_string())\n",
    unitTests: "library::registry_sites",
  },
  {
    name: "git-checkout-named-as-the-machine-keeps-it",
    breaks: "a git dependency's file is named under this machine's `$CARGO_HOME/git/checkouts/`, the builder's home in an app's JS (#2435)",
    file: "crates/rust-js-program/src/library/registry_sites.rs",
    find: " || pair == [\"git\", \"checkouts\"]",
    replace: "",
    unitTests: "library::registry_sites",
  },
  {
    name: "a-git-checkout-taken-for-a-registry",
    breaks: "a crate of a git checkout is taken for crates.io's, and its sites are looked for there (#2435)",
    file: "crates/rust-js-program/src/library/registry_sites.rs",
    find: ".filter(|under| under.starts_with(\"/cargo/registry/\"))",
    replace: "",
    unitTests: "library::registry_sites",
  },
  {
    name: "registry-site-strings-counted",
    breaks: "a format's `{{` in a `panic!`'s message is counted as a bracket, and its site isn't found (#2325)",
    file: "crates/rust-js-program/src/library/registry_sites.rs",
    find: "                (false, '\"') => quoted_text = true,\n",
    replace: "",
    unitTests: "library::registry_sites",
  },
];
