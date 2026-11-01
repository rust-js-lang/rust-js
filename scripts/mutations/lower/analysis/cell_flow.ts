// Mutations of src/lower/analysis/cell_flow.rs (ADR 0093): which cells a
// borrow can be of, by their flow (ADR 0863, amended, #445).
import type { Mutation } from "../../../mutations";

const corpus = (name: string) => ["test/corpus.test.ts", "-t", `^${name}.rs$`];
const flow = (name: string) => ["test/lowering-cell-flow.test.ts", "-t", name];

export const mutations: Mutation[] = [
  {
    name: "flow-std-moves-nothing",
    breaks:
      "a call of std's makes no value of a type it's given one with another it gives, so a cell found again by `get`, `cloned` and `map` is a cell of its own, unchecked where its held borrow's is, and runs on where Rust panics",
    file: "src/lower/analysis/cell_flow.rs",
    find: "                Some(&other) => self.graph.unify(other, node),",
    replace: "                Some(_) => {}",
    tests: corpus("refcell_flow_through_std"),
  },
  {
    name: "flow-closure-arguments-apart",
    breaks:
      "a closure called by an `Fn` trait's method isn't given its arguments, so a cell it's given is a cell of its own, unchecked, and runs on where Rust panics",
    file: "src/lower/analysis/cell_flow.rs",
    find: "                        self.graph.unify(item, param);",
    replace: "                        let _ = (item, param);",
    tests: corpus("refcell_flow_through_std"),
  },
  {
    name: "flow-key-init-apart",
    breaks:
      "a thread-local's storage isn't what its initializer gives, so one made of another's `Rc` is a cell apart, unchecked, and runs on where Rust panics",
    file: "src/lower/analysis/cell_flow.rs",
    find: "            self.graph.unify(given, held);",
    replace: "            let _ = (given, held);",
    tests: corpus("refcell_flow_thread_local_init"),
  },
  {
    name: "flow-binding-keeps-in-collection",
    breaks:
      "every binding is read as a collection that keeps what it's given in itself, so a cell given to JS is no cell JS gives back, unchecked, and runs on where Rust panics",
    file: "src/lower/analysis/cell_flow.rs",
    find: "                false => self.unseen(&ports),",
    replace: "                false => self.by_type(at, &ports, true),",
    tests: flow("a cell given to JS may be any cell JS gives back"),
  },
  {
    name: "flow-collection-unseen",
    breaks:
      "a JS collection's methods are code rust-js can't see, so a cell kept in a `Map` is any cell of its type, and Fizz's batch, of the same type as a tag cache held across a getter, is checked (ADR 1048)",
    file: "src/lower/analysis/cell_flow.rs",
    find: "                true => self.by_type(at, &ports, true),",
    replace: "                true => self.unseen(&ports),",
    tests: flow("a held borrow counts the cells that can be its own"),
  },
  {
    name: "flow-exports-closed",
    breaks:
      "what JS gives an exported function is the crate's own, so a borrow held of a cell JS gives meets none of a cell a binding gives, unchecked, and runs on where Rust panics",
    file: "src/lower/analysis/cell_flow.rs",
    find: "            if crate::lower::library::reachable(tcx, id) || method || tcx.is_coroutine(id) {",
    replace: "            if method || tcx.is_coroutine(id) {",
    tests: flow("a cell JS gives may be any cell of its type JS reaches"),
  },
  {
    name: "flow-given-functions-closed",
    breaks:
      "a closure given to what may call it anything isn't opened, so what it captures is the crate's own, a borrow of it unchecked, and runs on where Rust panics",
    file: "src/lower/analysis/cell_flow.rs",
    find: "            if more.is_empty() {\n                break;\n            }\n            for id in more {",
    replace: "            if true {\n                break;\n            }\n            for id in more {",
    tests: corpus("refcell_flow_reaches_unseen"),
  },
  {
    name: "flow-unresolved-call-closed",
    breaks:
      "a call rustc can't resolve, a generic `f()`, runs nothing of what it's given, so a closure it calls borrows a cell of its own, unchecked, and runs on where Rust panics",
    file: "src/lower/analysis/cell_flow.rs",
    find: "args) else {\n            return self.unseen(&ports);\n        };\n        let ty::InstanceKind::Item(callee)",
    replace: "args) else {\n            return;\n        };\n        let ty::InstanceKind::Item(callee)",
    tests: corpus("refcell_flow_reaches_unseen"),
  },
  {
    name: "flow-destructor-closed",
    breaks:
      "a value whose destructor is the crate's isn't what code rust-js can't see is given, so a `Drop` that borrows a cell it holds borrows one of its own, unchecked while a borrow of it is held, and runs on where Rust panics",
    file: "src/lower/analysis/cell_flow.rs",
    find: "            && adt.has_dtor(tcx)\n",
    replace: "            && adt.has_dtor(tcx)\n            && false\n",
    tests: corpus("refcell_let_guard_meets"),
  },
];
