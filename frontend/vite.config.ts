import { defineConfig } from 'vite'
import preact from '@preact/preset-vite'

// https://vite.dev/config/
export default defineConfig({
  plugins: [preact()],
  define: {
    // @solana/web3.js expects the Node-style `global` object in the browser.
    global: 'globalThis',
  },
  resolve: {
    // Without this, the bare `buffer` specifier resolves to Vite's Node-builtin
    // stub instead of the browser polyfill package that @solana/web3.js needs.
    alias: {
      buffer: 'buffer/index.js',
    },
  },
  optimizeDeps: {
    include: ['buffer'],
  },
})
