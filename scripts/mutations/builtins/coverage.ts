import type {Mutation} from "../../mutations";
const tests = ["test/builtins-coverage.test.ts", "-t", "the js crate binds what it bound"];
export const mutations: Mutation[] = [
  {name: "builtins-object-constructor-unread", breaks: "object-shaped constructor signatures disappear from native coverage", file: "builtins/coverage.ts", find: 'const ctor = checkedMembers(d.type.members).some((m: any) => m.kind === "construct");', replace: 'const ctor = false;', tests},
  {name: "builtins-interface-constructor-unread", breaks: "interface constructor signatures disappear from native coverage", file: "builtins/coverage.ts", find: 'constructSignatures.add(name);', replace: 'constructSignatures.delete(name);', tests},
];
