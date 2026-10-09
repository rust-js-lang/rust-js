// Mutations of src/lower/effects.rs (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "operators-impure",
    breaks: "an operator on references to numbers is taken as a call that may do anything: a pure chain is lazy",
    file: "src/lower/effects.rs",
    find: "                Some(_) => generic_args.types().all(simple),",
    replace: "                Some(_) => false,",
    tests: ["test/corpus.test.ts", "-t", "lazy_chains"],
    snapshots: true,
  },
  {
    name: "pure-if-impure",
    breaks: "an `if` of what does nothing is taken as doing something: a pure chain is lazy",
    file: "src/lower/effects.rs",
    find: "        } => pure(*cond) && pure(*then) && else_opt.is_none_or(pure),",
    replace: "        } => false,",
    tests: ["test/snapshots.test.ts"],
    snapshots: true,
  },
  {
    name: "cannot-throw-ignored",
    breaks: "a binding marked `cannot_throw` is taken as one that may throw, so `count` keeps a drop",
    file: "src/lower/effects.rs",
    find: "                    None => bindings::is_binding(tcx, id) && bindings::cannot_throw(tcx, id),\n",
    replace: "                    None => false,\n",
    tests: ["test/bindings.test.ts", "-t", "can't throw"],
  },
  {
    name: "coercion-may-leave",
    breaks: "a pointer coercion, an `&dyn Fn` read for a narrower lifetime, is taken as what may leave, so `call` keeps a drop",
    file: "src/lower/effects.rs",
    find: "        ExprKind::PointerCoercion { source, .. } => pure(*source),\n",
    replace: "",
    tests: ["test/bindings.test.ts", "-t", "can't throw"],
  },
];
