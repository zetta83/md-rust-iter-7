import { useCallback, useEffect, useState } from 'preact/hooks'
import { PublicKey } from '@solana/web3.js'
import { AddressType, useAccounts, useConnect, useDisconnect, useIsExtensionInstalled, useSolana } from '@phantom/react-sdk'

import './app.css'
import { fetchHealth, fetchOracle, type OracleResponse } from './lib/backend'
import { RPC_URL, EXPECTED_DECIMALS } from './lib/constants'
import { buildCreateTokenWithFeeTx } from './lib/instructions'

function explorerTxUrl(signature: string): string {
  if (RPC_URL.includes('devnet')) return `https://explorer.solana.com/tx/${signature}?cluster=devnet`
  if (RPC_URL.includes('mainnet')) return `https://explorer.solana.com/tx/${signature}`
  return `https://explorer.solana.com/tx/${signature}?cluster=custom&customUrl=${encodeURIComponent(RPC_URL)}`
}

function shorten(address: string): string {
  return `${address.slice(0, 4)}…${address.slice(-4)}`
}

interface CreateResult {
  signature: string
  mint: string
}

export function App() {
  const { isInstalled } = useIsExtensionInstalled()
  const { connect, isConnecting } = useConnect()
  const { disconnect } = useDisconnect()
  const { solana, isAvailable } = useSolana()
  const addresses = useAccounts()

  const walletAddress = addresses?.find((a) => a.addressType === AddressType.solana)?.address
  const wallet = walletAddress ? new PublicKey(walletAddress) : null

  const [oracle, setOracle] = useState<OracleResponse | null>(null)
  const [oracleError, setOracleError] = useState<string | null>(null)
  const [backendOk, setBackendOk] = useState<boolean | null>(null)
  const [oracleLoading, setOracleLoading] = useState(false)

  const [initialSupply, setInitialSupply] = useState('1000')
  const [feeUsd, setFeeUsd] = useState('1')
  const [creating, setCreating] = useState(false)
  const [createError, setCreateError] = useState<string | null>(null)
  const [createResult, setCreateResult] = useState<CreateResult | null>(null)

  const refreshOracle = useCallback(async () => {
    setOracleLoading(true)
    setOracleError(null)
    try {
      await fetchHealth()
      setBackendOk(true)
      setOracle(await fetchOracle())
    } catch (err) {
      setBackendOk((prev) => prev ?? false)
      setOracleError(err instanceof Error ? err.message : String(err))
    } finally {
      setOracleLoading(false)
    }
  }, [])

  useEffect(() => {
    refreshOracle()
  }, [refreshOracle])

  const connectWallet = useCallback(async () => {
    if (!isInstalled) {
      window.open('https://phantom.app/', '_blank')
      return
    }
    try {
      await connect({ provider: 'injected' })
    } catch {
      // user closed the Phantom popup — nothing to do
    }
  }, [connect, isInstalled])

  const createToken = useCallback(
    async (e: Event) => {
      e.preventDefault()
      if (!wallet || !isAvailable) return

      setCreating(true)
      setCreateError(null)
      setCreateResult(null)
      try {
        const { transaction, mint } = await buildCreateTokenWithFeeTx({
          payer: wallet,
          decimals: EXPECTED_DECIMALS,
          initialSupply: BigInt(initialSupply),
          feeUsd: BigInt(feeUsd) * 10n ** BigInt(EXPECTED_DECIMALS),
        })

        const { signature } = await solana.signAndSendTransaction(transaction)

        setCreateResult({ signature, mint: mint.publicKey.toBase58() })
        refreshOracle()
      } catch (err) {
        setCreateError(err instanceof Error ? err.message : String(err))
      } finally {
        setCreating(false)
      }
    },
    [wallet, isAvailable, solana, initialSupply, feeUsd, refreshOracle],
  )

  return (
    <div id="page">
      <header>
        <h1>Mini-Launchpad</h1>
        {wallet ? (
          <button type="button" class="wallet-btn connected" onClick={() => disconnect()}>
            {shorten(wallet.toBase58())} · disconnect
          </button>
        ) : (
          <button type="button" class="wallet-btn" onClick={connectWallet} disabled={isConnecting}>
            {isConnecting ? 'Connecting…' : isInstalled ? 'Connect Phantom' : 'Install Phantom'}
          </button>
        )}
      </header>

      <section class="card">
        <div class="card-head">
          <h2>Oracle (front → back → token_factory)</h2>
          <button type="button" class="ghost-btn" onClick={refreshOracle} disabled={oracleLoading}>
            {oracleLoading ? 'Refreshing…' : 'Refresh'}
          </button>
        </div>
        <p class="muted">
          Backend: {backendOk === null ? '…' : backendOk ? 'reachable' : 'unreachable'}
        </p>
        {oracleError && <p class="error">{oracleError}</p>}
        {oracle && (
          <dl class="kv">
            <dt>Price</dt>
            <dd>
              {(oracle.price / 10 ** oracle.decimals).toFixed(oracle.decimals)} ({oracle.decimals} decimals)
            </dd>
            <dt>Admin</dt>
            <dd>{oracle.admin}</dd>
            <dt>Last updated slot</dt>
            <dd>{oracle.last_updated_slot}</dd>
            <dt>Stale</dt>
            <dd class={oracle.is_stale ? 'error' : ''}>{oracle.is_stale ? 'yes' : 'no'}</dd>
          </dl>
        )}
      </section>

      <section class="card">
        <div class="card-head">
          <h2>Create token (front → token_factory)</h2>
        </div>
        <form onSubmit={createToken}>
          <label>
            Initial supply
            <input
              type="number"
              min="1"
              value={initialSupply}
              onInput={(e) => setInitialSupply((e.target as HTMLInputElement).value)}
              required
            />
          </label>
          <label>
            Fee (USD)
            <input
              type="number"
              min="1"
              value={feeUsd}
              onInput={(e) => setFeeUsd((e.target as HTMLInputElement).value)}
              required
            />
          </label>
          <button type="submit" class="wallet-btn" disabled={!wallet || creating}>
            {creating ? 'Creating…' : 'Create token'}
          </button>
          {!wallet && <p class="muted">Connect Phantom to create a token.</p>}
        </form>
        {createError && <p class="error">{createError}</p>}
        {createResult && (
          <dl class="kv">
            <dt>Mint</dt>
            <dd>{createResult.mint}</dd>
            <dt>Signature</dt>
            <dd>
              <a href={explorerTxUrl(createResult.signature)} target="_blank" rel="noreferrer">
                {shorten(createResult.signature)}
              </a>
            </dd>
          </dl>
        )}
      </section>
    </div>
  )
}
