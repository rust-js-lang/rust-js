// Mutations of src/lower/analysis/nulled_params/read_alike.rs (ADR 0861,
// amended, #665): which nullable thread-locals nothing reads apart.
import type { Mutation } from "../../../../mutations";

const tested = ["test/lowering-nullable-reads.test.ts", "-t", "a nullable thread-local nothing reads apart"];
const corpus = ["test/corpus.test.ts", "-t", "^nullable_local_read_alike.rs$"];

export const mutations: Mutation[] = [
  {
    name: "alike-of-a-public-key",
    breaks: "an exported nullable thread-local, which a JS module may read `=== null`, stores what may be `undefined` as it is (#665)",
    file: "src/lower/analysis/nulled_params/read_alike.rs",
    find: "            let elsewhere = tcx.visibility(key).is_public()\n                || !at.is_local()\n",
    replace: "            let elsewhere = !at.is_local()\n",
    tests: tested,
  },
  {
    name: "alike-read-in-another-module",
    breaks: "a nullable thread-local another module reads, which its module then exports, stores what may be `undefined` as it is (#665)",
    file: "src/lower/analysis/nulled_params/read_alike.rs",
    find: "                || tcx.parent_module_from_def_id(at.expect_local())\n                    != tcx.parent_module_from_def_id(key.expect_local())\n",
    replace: "",
    tests: tested,
  },
  {
    name: "alike-read-by-generic-code",
    breaks: "a nullable thread-local generic code reads, which a consumer may compile, stores what may be `undefined` as it is (#665)",
    file: "src/lower/analysis/nulled_params/read_alike.rs",
    find: "                || tcx.generics_of(at).requires_monomorphization(tcx);\n",
    replace: ";\n",
    tests: tested,
  },
  {
    name: "alike-handed-on",
    breaks: "a nullable thread-local handed on, `is_set(&HANDED)`, whose reads there aren't seen, stores what may be `undefined` as it is (#665)",
    file: "src/lower/analysis/nulled_params/read_alike.rs",
    find: "            match elsewhere || !accessed.contains(&id) {\n",
    replace: "            match elsewhere {\n",
    tests: tested,
  },
  {
    name: "alike-any-lend",
    breaks: "a nullable thread-local lent, `LENT.with_borrow_mut(f)`, whose `f` may do anything, stores what may be `undefined` as it is (#665)",
    file: "src/lower/analysis/nulled_params/read_alike.rs",
    find: "                // A lend, `with_borrow_mut(f)`, whose `f` may do anything.\n                _ => false,\n",
    replace: "                _ => true,\n",
    tests: tested,
  },
  {
    name: "alike-borrow-of-any-function",
    breaks: "a nullable thread-local lent to a closure, `BORROWED.with_borrow(|v| seen(*v))`, stores what may be `undefined` as it is, so the binding is given `undefined` (#665)",
    file: "src/lower/analysis/nulled_params/read_alike.rs",
    find: "                    clone.is_some_and(|clone| tcx.is_lang_item(clone, LangItem::Clone))\n",
    replace: "                    true\n",
    tests: tested,
  },
  {
    name: "alike-stored",
    breaks: "a nullable thread-local whose read is kept in a nullable field, tested `=== null`, stores what may be `undefined` as it is, so `None` reads as a `Some` (#665)",
    file: "src/lower/analysis/nulled_params/read_alike.rs",
    find: "                    Rvalue::Ref(_, mir::BorrowKind::Shared, of) if is(of, whole) => {\n                        into.is_some_and(|into| self.alike_through(key, into, seen))\n                    }\n                    _ => false,\n",
    replace: "                    Rvalue::Ref(_, mir::BorrowKind::Shared, of) if is(of, whole) => {\n                        into.is_some_and(|into| self.alike_through(key, into, seen))\n                    }\n                    _ => true,\n",
    tests: corpus,
  },
  {
    name: "alike-given-to-any-call",
    breaks: "a nullable thread-local whose read is given to a binding or a function of the crate's, which may tell `null` from `undefined`, stores what may be `undefined` as it is (#665)",
    file: "src/lower/analysis/nulled_params/read_alike.rs",
    find: "            Some(Std::Drop) => at == 0,\n",
    replace: "            Some(Std::Drop) => at == 0,\n            _ if true => true,\n",
    tests: tested,
  },
  {
    name: "alike-returned",
    breaks: "a nullable thread-local a nullable function returns, whose callers take it as `null` or a value, stores what may be `undefined` as it is (#665)",
    file: "src/lower/analysis/nulled_params/read_alike.rs",
    find: "            _ => return false,\n        };\n        let given",
    replace: "            _ => return true,\n        };\n        let given",
    tests: corpus,
  },
  {
    name: "alike-set-into-another",
    breaks: "a nullable thread-local whose read is set into another, which stores it as it is, stores what may be `undefined` as it is, so the other holds `undefined` (#665)",
    file: "src/lower/analysis/nulled_params/read_alike.rs",
    find: "                    && self.key_called(func, args) == Some(key)\n",
    replace: "                    && self.key_called(func, args).is_some()\n",
    tests: tested,
  },
  {
    name: "alike-cloned-unfollowed",
    breaks: "a nullable thread-local whose read, `with_borrow(Clone::clone)`, is only tested and set back is taken as told apart, so its write keeps `?? null` (#665)",
    file: "src/lower/analysis/nulled_params/read_alike.rs",
    find: "                Some(Std::Pointee | Std::Comb(Comb::AsDeref)) => {\n",
    replace: "                Some(Std::Comb(Comb::AsDeref)) => {\n",
    tests: corpus,
    snapshots: true,
  },
];
