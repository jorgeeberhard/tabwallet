import { api } from "@/lib/api"
import { centsToReais } from "@/lib/amount"

export interface Balance {
  /** Cents as of today, Projections left out; may be negative. */
  currentBalanceCents: number
  /** Current balance in reais, for the Design System components. */
  currentBalance: number
  /** The Opening Balance is informed once; after that it cannot change. */
  openingBalanceSet: boolean
  /** Cents (zero and negative are valid), or `null` while not informed. */
  openingBalanceCents: number | null
  /** The Opening Balance in reais, or `null` while not informed. */
  openingBalance: number | null
}

interface BalancePayload {
  current_balance_cents: number
  opening_balance_set: boolean
  opening_balance_cents: number | null
}

function toBalance(payload: BalancePayload): Balance {
  return {
    currentBalanceCents: payload.current_balance_cents,
    currentBalance: centsToReais(payload.current_balance_cents),
    openingBalanceSet: payload.opening_balance_set,
    openingBalanceCents: payload.opening_balance_cents,
    openingBalance:
      payload.opening_balance_cents === null ? null : centsToReais(payload.opening_balance_cents),
  }
}

/** Query key of the balance; anything that changes it (a Transaction) invalidates it. */
export const BALANCE_KEY = ["balance"] as const

export async function getBalance(): Promise<Balance> {
  return toBalance(await api<BalancePayload>("/balance"))
}

/** Informs the Opening Balance, which the API accepts only once. */
export async function setOpeningBalance(amountCents: number): Promise<Balance> {
  return toBalance(
    await api<BalancePayload>("/balance/opening", {
      method: "POST",
      body: { amount_cents: amountCents },
    }),
  )
}
