import { withRustJs } from "@rust-js/next-plugin";

/** @type {import('next').NextConfig} */
const nextConfig = {
  /* config options here */
};

// The app's Rust, compiled as `next dev` and `next build` load this
// (ADR 0380): its JS beside it, which Next.js serves as the app's own.
export default withRustJs(nextConfig);
