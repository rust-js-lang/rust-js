// Mutations of src/lower/classes/namespaces.rs (ADR 0093): which `self`
// a namespace's method's call gives names the namespace, and what holds it
// then (ADR 1037).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "namespace-constant-a-value",
    breaks:
      "a namespace named, `Reconciler.createContainer(3)`, is called as a value of it, not as its function, `createContainer(3)` (ADR 1037, rd311)",
    file: "src/lower/classes/namespaces.rs",
    find: "            Operand::Constant(_) => return Some(held),\n",
    replace: "            Operand::Constant(_) => return None,\n",
    tests: ["test/namespaces.test.ts", "-t", "called by name or through the value a binding gives"],
  },
  {
    name: "namespace-own-self-a-value",
    breaks:
      "a namespace's own method's `self` is called as a value, `undefined.updateContainer(tag)` (ADR 1037, rd311)",
    file: "src/lower/classes/namespaces.rs",
    find: "            return (local.as_usize() == 1\n",
    replace: "            return (local.as_usize() == 0\n",
    tests: ["test/namespaces.test.ts", "-t", "called through a value or by name"],
  },
  {
    name: "unread-namespace-written",
    breaks:
      "a library's namespace only calls that name it read, `Reconciler.create_container(3)`, is held all the same, so its module is imported whole, `import * as Reconciler`, which nothing reads (rd310, ADR 1037)",
    file: "src/lower/classes/namespaces.rs",
    find: "            held.extend(locals);\n",
    replace: "",
    tests: ["test/namespaces.test.ts", "-t", "called by name or through the value a binding gives"],
  },
  {
    name: "read-namespace-unwritten",
    breaks:
      "a namespace a named call reads and the function returns, `let reconciler = &Reconciler; ..; reconciler`, is never written, `return reconciler;` of nothing (rd310, ADR 1037)",
    file: "src/lower/classes/namespaces.rs",
    find: "            (reads.1.get(local).into_iter().flatten()).all(|at| named_at.contains(at) || writes.contains(at))\n",
    replace: "            (reads.1.get(local).into_iter().flatten()).all(|_| true)\n",
    tests: ["test/namespaces.test.ts", "-t", "called by name or through the value a binding gives"],
  },
  {
    name: "namespace-let-fake-read-a-read",
    breaks:
      "a `let` of a namespace only calls that name it read, `let reconciler = &Reconciler;`, is held, its fake read taken for one, so its module is imported whole, which nothing reads (rd310, ADR 1037)",
    file: "src/lower/classes/namespaces.rs",
    find: "&& !context.is_storage_marker() && !unwritten {\n",
    replace: "&& !context.is_storage_marker() {\n",
    tests: ["test/namespaces.test.ts", "-t", "called by name or through the value a binding gives"],
  },
  {
    name: "namespace-chain-write-a-read",
    breaks:
      "what holds a namespace only another such holder's write reads, `_3 = &_4`'s `_4`, is held, so its module is imported whole, which nothing reads (rd310, ADR 1037)",
    file: "src/lower/classes/namespaces.rs",
    find: "            (reads.1.get(local).into_iter().flatten()).all(|at| named_at.contains(at) || writes.contains(at))\n",
    replace: "            (reads.1.get(local).into_iter().flatten()).all(|at| named_at.contains(at))\n",
    tests: ["test/namespaces.test.ts", "-t", "called by name or through the value a binding gives"],
  },
];
