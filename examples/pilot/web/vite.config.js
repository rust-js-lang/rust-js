import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

// The client is the Cargo workspace's `frontend` (ADR 0101), its JS beside
// its Rust, which `rust-js build --package frontend` writes (ADR 0412), and
// `/api` is the native server's: `cargo run -p server`, or `PILOT_API` for
// another.
export default defineConfig({
  plugins: [react()],
  server: {
    proxy: { "/api": process.env.PILOT_API ?? "http://127.0.0.1:3000" },
  },
});
