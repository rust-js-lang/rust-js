// Mutations of src/lower/classes/prototypes.rs (ADR 0093): a class's
// prototype its load replaces, set right after its constructor (#448).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "class-prototype-set-of-the-module",
    breaks: "a class's prototype set at load of a function of its module moves to its constructor, where what it reads may not be made yet (#448)",
    file: "src/lower/classes/prototypes.rs",
    find: "                if read.is_none() && made.contains(name) {\n",
    replace: "                if false {\n",
    tests: [
      "test/classes.test.ts",
      "-t",
      "prototype replaced"
    ]
  },
  {
    name: "class-prototype-set-past-a-load",
    breaks: "a class's prototype set at load moves past a thread-local made as its module loads, which would see it replaced first (#448)",
    file: "src/lower/classes/prototypes.rs",
    find: "            js::Item::Const(constant) => Some(format!(\"`{}`\", constant.name)),\n",
    replace: "            js::Item::Const(_) => None,\n",
    tests: [
      "test/classes.test.ts",
      "-t",
      "prototype replaced"
    ]
  },
  {
    name: "class-prototype-set-left-in-place",
    breaks: "a class's prototype set at load stays where its `on_load!` is, after the methods assigned to the prototype it replaces, `chunk.then` undefined (#448)",
    file: "src/lower/classes/prototypes.rs",
    find: "            class.prototype_set = true;\n",
    replace: "",
    tests: [
      "test/classes.test.ts",
      "-t",
      "prototype replaced"
    ]
  },
  {
    name: "class-prototype-set-past-a-later-load",
    breaks: "a class's prototype set at load is refused of a thread-local written after its `on_load!`, which is made after it either way (#448)",
    file: "src/lower/classes/prototypes.rs",
    find: "            .filter(|item| written_between(item))\n",
    replace: "",
    tests: [
      "test/classes.test.ts",
      "-t",
      "prototype replaced at load"
    ]
  },
  {
    name: "class-prototype-set-beside-others",
    breaks: "a class's prototype set by an `on_load!` of other statements moves alone to its constructor, apart from the `let` it reads (#448)",
    file: "src/lower/classes/prototypes.rs",
    find: "    if let [set] = body {\n",
    replace: "    if let [set, ..] = body {\n",
    tests: [
      "test/classes.test.ts",
      "-t",
      "prototype replaced"
    ]
  },
];
