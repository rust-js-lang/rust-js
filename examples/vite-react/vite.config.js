import babel from '@rolldown/plugin-babel'
import tailwindcss from '@tailwindcss/vite'
import react, { reactCompilerPreset } from '@vitejs/plugin-react'
import { defineConfig } from 'vite'

// https://vite.dev/config/
// The app's crate, src/App.rs, is compiled by `rust-js build` (ADR 0412) to
// src/App.jsx, which plugin-react serves with Fast Refresh, and React
// Compiler memoizes.
export default defineConfig({
  plugins: [react(), babel({ presets: [reactCompilerPreset()] }), tailwindcss()],
})
