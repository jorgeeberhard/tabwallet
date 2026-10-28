import { api } from "@/lib/api"
import { centsToReais } from "@/lib/amount"

export interface CreditCard {
  id: string
  name: string
  /** 1 to 31; the days the card has from now on. */
  closingDay: number
  dueDay: number
  /** A deactivated card takes no new purchases but keeps its history. */
  active: boolean
}

interface CreditCardPayload {
  id: string
  name: string
  closing_day: number
  due_day: number
  active: boolean
}

function toCreditCard(payload: CreditCardPayload): CreditCard {
  return {
    id: payload.id,
    name: payload.name,
    closingDay: payload.closing_day,
    dueDay: payload.due_day,
    active: payload.active,
  }
}

/** Query key of the list of cards; mutations invalidate it. */
export const CREDIT_CARDS_KEY = ["credit-cards"] as const

/** Query key prefix of every invoice query; purchase mutations invalidate it. */
export const INVOICES_KEY = ["invoices"] as const

export async function listCreditCards(): Promise<CreditCard[]> {
  const { credit_cards } = await api<{ credit_cards: CreditCardPayload[] }>("/credit-cards")
  return credit_cards.map(toCreditCard)
}

export interface CreditCardInput {
  name: string
  closingDay: number
  dueDay: number
}

export async function createCreditCard(input: CreditCardInput): Promise<CreditCard> {
  const { credit_card } = await api<{ credit_card: CreditCardPayload }>("/credit-cards", {
    method: "POST",
    body: { name: input.name, closing_day: input.closingDay, due_day: input.dueDay },
  })
  return toCreditCard(credit_card)
}

/** New days only affect the invoices still open. */
export async function updateCreditCard(
  id: string,
  changes: Partial<CreditCardInput>,
): Promise<CreditCard> {
  const { credit_card } = await api<{ credit_card: CreditCardPayload }>(`/credit-cards/${id}`, {
    method: "PATCH",
    body: { name: changes.name, closing_day: changes.closingDay, due_day: changes.dueDay },
  })
  return toCreditCard(credit_card)
}

export async function deactivateCreditCard(id: string): Promise<CreditCard> {
  const { credit_card } = await api<{ credit_card: CreditCardPayload }>(
    `/credit-cards/${id}/deactivate`,
    { method: "POST" },
  )
  return toCreditCard(credit_card)
}

export async function reactivateCreditCard(id: string): Promise<CreditCard> {
  const { credit_card } = await api<{ credit_card: CreditCardPayload }>(
    `/credit-cards/${id}/reactivate`,
    { method: "POST" },
  )
  return toCreditCard(credit_card)
}

/** `open` receives purchases, `closed` waits for its due date, `paid` is from the due date on. */
export type InvoiceState = "open" | "closed" | "paid"

/** The dates and state of the invoice a purchase belongs to. */
export interface InvoiceCycle {
  /** `YYYY-MM`: the month the invoice closes; identifies it within the card. */
  closingMonth: string
  /** `YYYY-MM-DD`. */
  closingDate: string
  dueDate: string
  /** `YYYY-MM`: the month of the due date, where its purchases count. */
  referenceMonth: string
  state: InvoiceState
}

interface InvoiceCyclePayload {
  closing_month: string
  closing_date: string
  due_date: string
  reference_month: string
  state: InvoiceState
}

function toCycle(payload: InvoiceCyclePayload): InvoiceCycle {
  return {
    closingMonth: payload.closing_month,
    closingDate: payload.closing_date,
    dueDate: payload.due_date,
    referenceMonth: payload.reference_month,
    state: payload.state,
  }
}

export interface Invoice extends InvoiceCycle {
  creditCardId: string
  totalCents: number
  /** In reais, for the Design System components. */
  total: number
  remainingCents: number
  remaining: number
}

interface InvoicePayload extends InvoiceCyclePayload {
  credit_card_id: string
  total_cents: number
  remaining_cents: number
}

function toInvoice(payload: InvoicePayload): Invoice {
  return {
    ...toCycle(payload),
    creditCardId: payload.credit_card_id,
    totalCents: payload.total_cents,
    total: centsToReais(payload.total_cents),
    remainingCents: payload.remaining_cents,
    remaining: centsToReais(payload.remaining_cents),
  }
}

export interface CardPurchase {
  id: string
  /** Always greater than zero. */
  amountCents: number
  /** In reais, for the Design System components. */
  amount: number
  /** A calendar day, `YYYY-MM-DD`. */
  date: string
  description: string
  category: { id: string; name: string; active: boolean }
  creditCard: { id: string; name: string; active: boolean }
  /** The invoice the purchase enters, by the closing rule. */
  invoice: InvoiceCycle
  /** `YYYY-MM`: the month of the invoice's due date. */
  referenceMonth: string
  isProjection: boolean
}

export interface CardPurchasePayload {
  id: string
  amount_cents: number
  date: string
  description: string
  category: { id: string; name: string; type: "expense"; active: boolean }
  credit_card: { id: string; name: string; active: boolean }
  invoice: InvoiceCyclePayload
  reference_month: string
  is_projection: boolean
}

export function toCardPurchase(payload: CardPurchasePayload): CardPurchase {
  return {
    id: payload.id,
    amountCents: payload.amount_cents,
    amount: centsToReais(payload.amount_cents),
    date: payload.date,
    description: payload.description,
    category: {
      id: payload.category.id,
      name: payload.category.name,
      active: payload.category.active,
    },
    creditCard: payload.credit_card,
    invoice: toCycle(payload.invoice),
    referenceMonth: payload.reference_month,
    isProjection: payload.is_projection,
  }
}

export function invoicesKey(cardId: string) {
  return [...INVOICES_KEY, cardId] as const
}

export async function listInvoices(cardId: string): Promise<Invoice[]> {
  const { invoices } = await api<{ invoices: InvoicePayload[] }>(
    `/credit-cards/${cardId}/invoices`,
  )
  return invoices.map(toInvoice)
}

export interface InvoiceDetail {
  invoice: Invoice
  purchases: CardPurchase[]
}

export function invoiceKey(cardId: string, closingMonth: string) {
  return [...INVOICES_KEY, cardId, closingMonth] as const
}

export async function getInvoice(cardId: string, closingMonth: string): Promise<InvoiceDetail> {
  const payload = await api<{ invoice: InvoicePayload; purchases: CardPurchasePayload[] }>(
    `/credit-cards/${cardId}/invoices/${closingMonth}`,
  )
  return {
    invoice: toInvoice(payload.invoice),
    purchases: payload.purchases.map(toCardPurchase),
  }
}

export interface CardPurchaseInput {
  creditCardId: string
  amountCents: number
  date: string
  description: string
  categoryId: string
}

export async function createCardPurchase(input: CardPurchaseInput): Promise<CardPurchase> {
  const { card_purchase } = await api<{ card_purchase: CardPurchasePayload }>("/card-purchases", {
    method: "POST",
    body: {
      credit_card_id: input.creditCardId,
      amount_cents: input.amountCents,
      date: input.date,
      description: input.description,
      category_id: input.categoryId,
    },
  })
  return toCardPurchase(card_purchase)
}

/** What an edit may change: the card never changes. */
export type CardPurchaseChanges = Partial<Omit<CardPurchaseInput, "creditCardId">>

export async function updateCardPurchase(
  id: string,
  changes: CardPurchaseChanges,
): Promise<CardPurchase> {
  const { card_purchase } = await api<{ card_purchase: CardPurchasePayload }>(
    `/card-purchases/${id}`,
    {
      method: "PATCH",
      body: {
        amount_cents: changes.amountCents,
        date: changes.date,
        description: changes.description,
        category_id: changes.categoryId,
      },
    },
  )
  return toCardPurchase(card_purchase)
}

export async function deleteCardPurchase(id: string): Promise<void> {
  await api(`/card-purchases/${id}`, { method: "DELETE" })
}
