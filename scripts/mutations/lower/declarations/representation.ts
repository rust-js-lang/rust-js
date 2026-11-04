// Mutations of src/lower/declarations/representation.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "mut-object-declared-box",
    breaks: "a `&mut` to an object is declared a box, `tick(counter: { value: Counter })`, where its JS takes the object (#619)",
    file: "src/lower/declarations/representation.rs",
    find: "        (self.representation.mut_ref)(self.owner, pointee)\n",
    replace: "        let _ = (self.representation.mut_ref)(self.owner, pointee);\n        MutRefForm::Box\n",
    tests: ["test/declarations-ungiven.test.ts", "-t", "box is declared"],
  },
  {
    name: "dyn-pair-declared-any",
    breaks: "a `Box<dyn Shape>` of a declared trait is declared `any`, not its pair `{ value; impl }` (#620)",
    file: "src/lower/declarations/representation.rs",
    find: "            Some(t) if self.declares_dictionary(t) => self.pair(t),\n",
    replace: "            Some(t) if self.declares_dictionary(t) => keyword(\"any\"),\n",
    tests: ["test/declarations-ungiven.test.ts", "-t", "box is declared"],
  },
  {
    name: "undeclared-dyn-pair-declared",
    breaks: "a `dyn` of a trait that isn't declared, `Box<dyn Steps>`, is declared a pair of `Steps<unknown>`, which no declaration declares (#620)",
    file: "src/lower/declarations/representation.rs",
    find: "            Some(t) if self.declares_dictionary(t) => self.pair(t),\n",
    replace: "            Some(t) if self.declares_dictionary(t) || t.is_local() => self.pair(t),\n",
    tests: ["test/declarations-ungiven.test.ts", "-t", "box is declared"],
  },
];
