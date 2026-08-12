import { BACKEND_URL } from './constants'

export interface OracleResponse {
  admin: string
  price: number
  decimals: number
  last_updated_slot: number
  is_stale: boolean
}

async function getJson<T>(path: string): Promise<T> {
  const res = await fetch(`${BACKEND_URL}${path}`)
  if (!res.ok) {
    const body = await res.json().catch(() => null)
    throw new Error(body?.error ?? `${res.status} ${res.statusText}`)
  }
  return res.json()
}

export function fetchHealth(): Promise<{ status: string }> {
  return getJson('/health')
}

export function fetchOracle(): Promise<OracleResponse> {
  return getJson('/oracle')
}
