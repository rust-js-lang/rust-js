// Mutations of src/lower/copies/stored.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

const value = ["test/crates.test.ts", "-t", "three crates: library_value_changed_by_another_library"];

export const mutations: Mutation[] = [
  {
    name: "stored-shared",
    breaks: "a library hands out what it stores itself, `return ORIGIN;`, which another library given it changes in place (#1907)",
    file: "src/lower/copies/stored.rs",
    find: "        self.krate.library\n            && !self.krate.compiled_once\n",
    replace: "        false\n            && !self.krate.compiled_once\n",
    tests: value,
  },
  {
    name: "stored-thread-local-unseen",
    breaks: "a thread-local's `with`'s value isn't what a library stores, so `home()` hands out `HOME` itself, which `mid::marked` changes (#1907)",
    file: "src/lower/copies/stored.rs",
    find: "            return pointer.as_usize() == 2 && self.thread_local_reader(body.source.def_id());\n",
    replace: "            return false;\n",
    tests: value,
  },
  {
    name: "stored-parameters-stored",
    breaks: "what a parameter's reference or a closure's capture reads is taken for what a library stores, so primitives' `computePosition(.., { ...boundary }, ..)` copies again what no crate changes (#1907, #1871)",
    file: "src/lower/copies/stored.rs",
    find: "            return pointer.as_usize() == 2 && self.thread_local_reader(body.source.def_id());\n",
    replace: "            return true;\n",
    tests: ["test/deep-chain.test.ts"],
  },
  {
    name: "stored-parts-read-copied",
    breaks: "a stored value whose number alone is read is copied, `const mark = { ...shared }; return mark.n;` (#1907)",
    file: "src/lower/copies/stored.rs",
    find: "                Rvalue::Use(op, _) if read(op) => self.local_escapes(body, dest.local, 0),\n",
    replace: "                Rvalue::Use(op, _) if read(op) => true,\n",
    tests: ["test/library-kept-copies.test.ts", "-t", "kept value"],
  },
  {
    name: "stored-dictionary-copied",
    breaks: "what a library stores given to a WebIDL operation's dictionary parameter is copied, `decoder.decode(undefined, { ...STREAMING })`, though WebIDL reads it into an IDL value and keeps nothing (#1907)",
    file: "src/lower/copies/stored.rs",
    find: "        matches!(param.kind(), ty::Adt(adt, _) if bindings::is_dictionary(tcx, adt.did()))\n",
    replace: "        matches!(param.kind(), ty::Adt(adt, _) if false && bindings::is_dictionary(tcx, adt.did()))\n",
    tests: ["test/library-kept-copies.test.ts", "-t", "dictionary"],
  },
  {
    name: "stored-any-uncopied",
    breaks: "what a library stores given to a binding's parameter of any value is as it is, `postMessage(SAVED)`, which the platform may keep (#1907)",
    file: "src/lower/copies/stored.rs",
    find: "        if !bindings::is_binding(tcx, callee) {\n            return false;\n        }\n",
    replace: "        if !bindings::is_binding(tcx, callee) {\n            return false;\n        }\n        if true {\n            return true;\n        }\n",
    tests: ["test/library-kept-copies.test.ts", "-t", "dictionary"],
  },
  {
    name: "stored-std-form-copied",
    breaks: "std's value of a form of its own is taken for a plain object, so a library's stored `fmt::Arguments`, a JS function, is spread into an object, `TypeError: args is not a function` (#1907)",
    file: "src/lower/copies/stored.rs",
    find: "            ty::Adt(adt, _) if !crate::lower::recognition::is_std_crate(self.tcx, adt.did()) => {\n",
    replace: "            ty::Adt(adt, _) if true || !crate::lower::recognition::is_std_crate(self.tcx, adt.did()) => {\n",
    tests: ["test/corpus.test.ts", "-t", "formatter_write_fmt_of_held_arguments"],
  },
  {
    name: "stored-guard-unseen",
    breaks: "what a `RefCell`'s guard or an `Rc` points at isn't what a library stores, so `got(&shared)` hands out the `Mark` the consumer's `Rc` holds, which `mid::marked` changes (#1907)",
    file: "src/lower/copies/stored.rs",
    find: "        if holds {\n            return true;\n        }\n",
    replace: "        if false && holds {\n            return true;\n        }\n",
    tests: value,
  },
];
