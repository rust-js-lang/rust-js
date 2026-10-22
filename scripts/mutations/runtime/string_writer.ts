// Mutations of src/runtime/string_writer.js (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "mut-writer-not-through",
    breaks: "`$mutWriter` gives its writer the box a `&mut` to it is given in, not what's in it",
    file: "src/runtime/string_writer.js",
    find: "    writeFmt: (w, text) => writer.writeFmt(w.value, text),\n",
    replace: "    writeFmt: (w, text) => writer.writeFmt(w, text),\n",
    tests: ["test/corpus.test.ts", "-t", "^(generic_writers|mut_given_as_type_param).rs$"],
  },
];
