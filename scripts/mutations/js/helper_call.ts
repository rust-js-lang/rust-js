// Mutations of crates/rust-js-program/src/js/helper_call.rs (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "padded-taken-as-given",
    breaks: "an `undefined` a located call is padded with, `$count(it, undefined, ..)`'s, is read as its `drop` given (#2334)",
    file: "crates/rust-js-program/src/js/helper_call.rs",
    find: "    args.get(at).filter(|arg| !arg.facts.has(Known::Padded))\n",
    replace: "    args.get(at)\n",
    unitTests: "js::helper_call",
  },
  {
    name: "helper-call-of-any-helper",
    breaks: "a call of another helper is read as one of `$lent`'s (#2334)",
    file: "crates/rust-js-program/src/js/helper_call.rs",
    find: "        let name = *names.iter().find(|&&name| callee.helper_name() == Some(name))?;\n",
    replace: "        callee.helper_name()?;\n        let name = names[0];\n",
    unitTests: "js::helper_call",
  },
];
