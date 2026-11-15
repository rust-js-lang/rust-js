import type {Mutation} from "../../mutations";
export const mutations: Mutation[] = [
  {name: "webapi-constructor-unread", breaks: "native DOM constructors disappear from the coverage inventory", file: "webapi/coverage.ts", find: 'if (x.kind === "construct") m.ctor = true;', replace: 'if (x.kind === "construct") m.ctor = false;', tests: ["test/webapi-coverage.test.ts", "-t", "the webapi crate binds what it bound"]},
  {
    "name": "webapi-symbol-key-unformatted",
    "breaks": "the browser coverage consumer treats a structured symbol key as a string",
    "file": "webapi/coverage.ts",
    "find": "m.members.add(memberKey(member.name));",
    "replace": "m.members.add(member.name);",
    "tests": [
      "test/webapi-coverage.test.ts",
      "-t",
      "the webapi crate binds what it bound"
    ]
  },
];
