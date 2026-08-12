import { Buffer } from 'buffer'
import { render } from 'preact'
import { AddressType, PhantomProvider } from '@phantom/react-sdk'
import './index.css'
import { App } from './app.tsx'

// @solana/web3.js relies on the Node `Buffer` global, which isn't present in the browser.
const globalWithBuffer = globalThis as unknown as { Buffer?: typeof Buffer }
globalWithBuffer.Buffer ??= Buffer

render(
  <PhantomProvider
    config={{ providers: ['injected'], addressTypes: [AddressType.solana] }}
    appName="Mini-Launchpad"
  >
    <App />
  </PhantomProvider>,
  document.getElementById('app')!,
)
