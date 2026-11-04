// Mutations of src/lower/pipeline/forwarders.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "forwarders-copies-unasked",
    breaks:
      "a function whose call needs a copy that boxes what it's given, `some::<Option<u32>>(x)`, is re-exported as the function's own JS (rd575, ADR 0051)",
    file: "src/lower/pipeline/forwarders.rs",
    find: "                && js_given.called(&recognition, &js_given.main(tcx, function), (target, args)) == Ok(None)\n",
    replace: "\n",
    tests: ["test/declarations-libraries.test.ts", "test/lowering-forwarding.test.ts", "-t", "is its re-export"],
  },
  {
    name: "forwarders-arguments-unasked",
    breaks:
      "a function of the crate's that reads `js::arguments()` or `js::this()` is re-exported, counting what JS gives, or reading the `this` JS calls the forwarder with, not what the forwarder gave it (rd575, rd332, rd683)",
    file: "src/lower/pipeline/forwarders.rs",
    find: "            !reads_arguments.reads_its_call(target)\n",
    replace: "            true\n",
    tests: ["test/declarations-libraries.test.ts", "test/lowering-forwarding.test.ts", "test/this.test.ts", "-t", "is its re-export|reads its this"],
  },
  {
    name: "forwarders-library-arguments-unasked",
    breaks:
      "a library's function that reads `js::arguments()` or `js::this()` is re-exported, counting what JS gives, or reading the `this` JS calls the forwarder with (rd575, rd332, rd683)",
    file: "src/lower/pipeline/forwarders.rs",
    find: "            !foreign.reads_its_call(target)\n",
    replace: "            true\n",
    tests: ["test/declarations-libraries.test.ts", "test/lowering-forwarding.test.ts", "test/this.test.ts", "-t", "is its re-export|reads this"],
  },
  {
    name: "forwarders-library-drops-unasked",
    breaks:
      "a library's function that takes a drop its call leaves out is re-exported, given what JS gives past the forwarder's parameters as that drop (rd575, ADR 0098)",
    file: "src/lower/pipeline/forwarders.rs",
    find: " && item.calling.drop_params().is_empty())\n",
    replace: ")\n",
    tests: ["test/declarations-libraries.test.ts", "test/lowering-forwarding.test.ts", "-t", "is its re-export"],
  },
  {
    name: "forwarders-named-at-home",
    breaks:
      "a function its own module calls is re-exported, its module's call naming nothing (rd575)",
    file: "src/lower/pipeline/forwarders.rs",
    find: "                named_at_home.insert(*named);\n",
    replace: "                let _ = named;\n",
    tests: ["test/declarations-libraries.test.ts", "test/lowering-forwarding.test.ts", "-t", "is its re-export"],
  },
  {
    name: "forwarder-default-taken",
    breaks:
      "a module's default that only passes its parameters on is re-exported by its name, and its `export default` names nothing (rd575)",
    file: "src/lower/pipeline/forwarders.rs",
    find: "            || default_export == Some(function)\n",
    replace: "",
    tests: ["test/declarations-libraries.test.ts", "test/lowering-forwarding.test.ts", "-t", "is its re-export"],
  },
  {
    name: "forwarder-alias-taken",
    breaks:
      "a function its module re-exports by another name, `pub use self::renamed as other_name`, is re-exported, and `export { renamed as otherName }` names nothing (rd575)",
    file: "src/lower/pipeline/forwarders.rs",
    find: "            || aliased.iter().any(|&(item, _)| item == function)\n",
    replace: "",
    tests: ["test/declarations-libraries.test.ts", "test/lowering-forwarding.test.ts", "-t", "is its re-export"],
  },
  {
    name: "forwarder-declared-as-reexport",
    breaks:
      "a function re-exported in its JS is re-exported in its declarations too, of the function's own type parameters, not declared at its instantiation (rd575)",
    file: "src/lower/pipeline/forwarders.rs",
    find: "        declared: super::super::reexports::assembled(found.clone(), sources, foreign),\n",
    replace: "        declared: super::super::reexports::assembled(found.iter().cloned().chain(named.iter().cloned()).collect(), sources, foreign),\n",
    tests: ["test/declarations-libraries.test.ts", "test/lowering-forwarding.test.ts", "-t", "is its re-export"],
  },
];
