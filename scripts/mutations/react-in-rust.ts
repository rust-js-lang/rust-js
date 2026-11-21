// React's port's checkout (scripts/react-in-rust.ts): one regen's at a
// time, and made again where it's unreadable (#2279).
import type { Mutation } from "../mutations";

export const mutations: Mutation[] = [
  {
    name: "port-bounded-under-its-regen",
    breaks: "another checkout's React port is bounded while a regen holds it, so a worktree's regen removes the port a main checkout's is using (#2279)",
    file: "scripts/react-in-rust.ts",
    find: "    if (at === own || lockState(portLock(at)) === \"held\") continue;\n",
    replace: "    if (at === own) continue;\n",
    tests: ["test/react-port-checkout.test.ts", "-t", "isn't bounded from under it"],
  },
  {
    name: "corrupt-port-checkout-not-remade",
    breaks: "a React port checkout whose objects are unreadable fails each push at its React gate, never made again from nothing (#2279)",
    file: "scripts/react-in-rust.ts",
    find: "  rmSync(port, { recursive: true, force: true });\n  const again = checkoutPin(port, repository, pin);\n",
    replace: "  const again = checkoutPin(port, repository, pin);\n",
    tests: ["test/react-port-checkout.test.ts", "-t", "made again once"],
  },
  {
    name: "port-git-step-unnamed",
    breaks: "a git step of React's port names no repository, so in a port whose `.git` is unreadable it reaches one above it, the checkout's own, and cleans or stages it (#2279)",
    file: "scripts/react-in-rust.ts",
    find: "  must([\"git\", \"clean\", \"-q\", \"-fdx\", \"-e\", \"node_modules\"], port, portRepository(port));\n",
    replace: "  must([\"git\", \"clean\", \"-q\", \"-fdx\", \"-e\", \"node_modules\"], port);\n",
    tests: ["test/react-port-checkout.test.ts", "-t", "names the port's own repository"],
  },
];
