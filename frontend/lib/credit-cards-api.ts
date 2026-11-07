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

/**
 * Whether the payment of an invoice can be corrected to the amount really paid
 * at the bank (UC20): `not_due` before the due date, `nothing_to_pay` when
 * early payments covered it all, `editable` until the next invoice (the one
 * that receives the Carried Balance) is paid, and `locked` after that.
 */
export type PaymentCorrectionState = "not_due" | "nothing_to_pay" | "editable" | "locked"

export interface PaymentCorrection {
  state: PaymentCorrectionState
  /** The amount the person said they paid, if they corrected the payment. */
  correctedCents: number | null
  /** The most the payment can be corrected to: what the due date would debit. */
  maxCents: number
}

export interface Invoice extends InvoiceCycle {
  creditCardId: string
  /** Purchases, Charges and the Carried Balance received. */
  totalCents: number
  /** In reais, for the Design System components. */
  total: number
  /** What left the balance for this invoice: Early Payments and, once paid, the due-date payment. */
  paidCents: number
  paid: number
  /** What is still to pay: the total less the Early Payments. Zero once the invoice is paid. */
  remainingCents: number
  remaining: number
  /** Interest, penalty and IOF the person recorded in this invoice. */
  chargesCents: number
  /** Carried Balance received from the previous invoice (not an expense). */
  carriedBalanceCents: number
  /** Part of this invoice that was not paid and moved to the next one. */
  carriedForwardCents: number
  paymentCorrection: PaymentCorrection
}

interface InvoicePayload extends InvoiceCyclePayload {
  credit_card_id: string
  total_cents: number
  paid_cents: number
  remaining_cents: number
  charges_cents: number
  carried_balance_cents: number
  carried_forward_cents: number
  payment_correction: {
    state: PaymentCorrectionState
    corrected_cents: number | null
    max_cents: number
  }
}

function toInvoice(payload: InvoicePayload): Invoice {
  return {
    ...toCycle(payload),
    creditCardId: payload.credit_card_id,
    totalCents: payload.total_cents,
    total: centsToReais(payload.total_cents),
    paidCents: payload.paid_cents,
    paid: centsToReais(payload.paid_cents),
    remainingCents: payload.remaining_cents,
    remaining: centsToReais(payload.remaining_cents),
    chargesCents: payload.charges_cents,
    carriedBalanceCents: payload.carried_balance_cents,
    carriedForwardCents: payload.carried_forward_cents,
    paymentCorrection: {
      state: payload.payment_correction.state,
      correctedCents: payload.payment_correction.corrected_cents,
      maxCents: payload.payment_correction.max_cents,
    },
  }
}

/**
 * A payment of an invoice (an Invoice Payment): `early` is one the person made
 * before the due date, `automatic` is the one the system debits on the due
 * date for what is still unpaid (derived by the API on read), and `corrected`
 * is the due-date payment after the person edited it to the amount really
 * paid. None is an expense: the spending was counted when the purchases were
 * made.
 */
export type InvoicePaymentOrigin = "early" | "automatic" | "corrected"

export interface InvoicePayment {
  /** Only an Early Payment is stored, so only it has an id. */
  id: string | null
  origin: InvoicePaymentOrigin
  amountCents: number
  /** In reais, for the Design System components. */
  amount: number
  /** The day the money leaves (or left) the Current Balance, `YYYY-MM-DD`. */
  date: string
  /** An automatic payment not yet due: the Projected Balance already subtracts it. */
  isProjection: boolean
}

export interface InvoicePaymentPayload {
  id: string | null
  origin: InvoicePaymentOrigin
  amount_cents: number
  date: string
  is_projection: boolean
}

export function toInvoicePayment(payload: InvoicePaymentPayload): InvoicePayment {
  return {
    id: payload.id,
    origin: payload.origin,
    amountCents: payload.amount_cents,
    amount: centsToReais(payload.amount_cents),
    date: payload.date,
    isProjection: payload.is_projection,
  }
}

/** An Invoice Payment in the statement, with the card and the invoice it pays. */
export interface InvoicePaymentItem extends InvoicePayment {
  creditCard: { id: string; name: string; active: boolean }
  invoice: InvoiceCycle
}

export interface InvoicePaymentItemPayload extends InvoicePaymentPayload {
  credit_card: { id: string; name: string; active: boolean }
  invoice: InvoiceCyclePayload
}

export function toInvoicePaymentItem(payload: InvoicePaymentItemPayload): InvoicePaymentItem {
  return {
    ...toInvoicePayment(payload),
    creditCard: payload.credit_card,
    invoice: toCycle(payload.invoice),
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

/**
 * Charges (Encargos): the interest, penalty and IOF the bank charged on an
 * invoice, recorded by the person as an expense of the "Juros e encargos"
 * Category. The system never calculates them. They are part of the invoice
 * total and count in the reference month of its due date.
 */
export interface Charge {
  id: string
  /** Always greater than zero. */
  amountCents: number
  /** In reais, for the Design System components. */
  amount: number
  /** The day the bank charged it, `YYYY-MM-DD`. */
  date: string
  description: string
  category: { id: string; name: string; active: boolean }
  creditCard: { id: string; name: string; active: boolean }
  /** The invoice it was charged on. */
  invoice: InvoiceCycle
  /** `YYYY-MM`: the month of the invoice's due date. */
  referenceMonth: string
  isProjection: boolean
}

export interface ChargePayload {
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

export function toCharge(payload: ChargePayload): Charge {
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

/** Part of an invoice that was not paid on the previous one (not an expense). */
export interface CarriedBalance {
  amountCents: number
  amount: number
  /** `YYYY-MM`: the closing month of the invoice it came from. */
  fromClosingMonth: string
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
  /** The Charges recorded in the invoice, newest first. */
  charges: Charge[]
  /** The Carried Balance this invoice received from the previous one, if any. */
  carriedBalance: CarriedBalance | null
  /** The Early Payments and, when something is unpaid, the due-date one; oldest first. */
  payments: InvoicePayment[]
}

export function invoiceKey(cardId: string, closingMonth: string) {
  return [...INVOICES_KEY, cardId, closingMonth] as const
}

export async function getInvoice(cardId: string, closingMonth: string): Promise<InvoiceDetail> {
  const payload = await api<{
    invoice: InvoicePayload
    purchases: CardPurchasePayload[]
    charges: ChargePayload[]
    carried_balance: { amount_cents: number; from_closing_month: string } | null
    payments: InvoicePaymentPayload[]
  }>(`/credit-cards/${cardId}/invoices/${closingMonth}`)
  return {
    invoice: toInvoice(payload.invoice),
    purchases: payload.purchases.map(toCardPurchase),
    charges: payload.charges.map(toCharge),
    carriedBalance: payload.carried_balance && {
      amountCents: payload.carried_balance.amount_cents,
      amount: centsToReais(payload.carried_balance.amount_cents),
      fromClosingMonth: payload.carried_balance.from_closing_month,
    },
    payments: payload.payments.map(toInvoicePayment),
  }
}

/**
 * UC20: corrects the payment of a paid invoice to the amount really paid at
 * the bank. What is left unpaid becomes the Carried Balance of the next
 * invoice; the correction can be repeated until that invoice is paid.
 */
export async function correctInvoicePayment(
  cardId: string,
  closingMonth: string,
  amountCents: number,
): Promise<{ payment: InvoicePayment; invoice: Invoice }> {
  const payload = await api<{ payment: InvoicePaymentPayload; invoice: InvoicePayload }>(
    `/credit-cards/${cardId}/invoices/${closingMonth}/payment`,
    { method: "PUT", body: { amount_cents: amountCents } },
  )
  return { payment: toInvoicePayment(payload.payment), invoice: toInvoice(payload.invoice) }
}

export interface ChargeInput {
  amountCents: number
  date: string
  description?: string
}

/** UC21: records the interest, penalty and IOF the bank charged on an invoice. */
export async function createCharge(
  cardId: string,
  closingMonth: string,
  input: ChargeInput,
): Promise<Charge> {
  const { charge } = await api<{ charge: ChargePayload }>(
    `/credit-cards/${cardId}/invoices/${closingMonth}/charges`,
    {
      method: "POST",
      body: {
        amount_cents: input.amountCents,
        date: input.date,
        description: input.description,
      },
    },
  )
  return toCharge(charge)
}

export async function updateCharge(id: string, changes: Partial<ChargeInput>): Promise<Charge> {
  const { charge } = await api<{ charge: ChargePayload }>(`/charges/${id}`, {
    method: "PATCH",
    body: {
      amount_cents: changes.amountCents,
      date: changes.date,
      description: changes.description,
    },
  })
  return toCharge(charge)
}

export async function deleteCharge(id: string): Promise<void> {
  await api(`/charges/${id}`, { method: "DELETE" })
}

/**
 * UC17: pays an Open or Closed invoice early, in full or in part, for at most
 * what it still has to pay. The payment is dated today by the API.
 */
export async function payInvoice(
  cardId: string,
  closingMonth: string,
  amountCents: number,
): Promise<{ payment: InvoicePayment; invoice: Invoice }> {
  const payload = await api<{ payment: InvoicePaymentPayload; invoice: InvoicePayload }>(
    `/credit-cards/${cardId}/invoices/${closingMonth}/payments`,
    { method: "POST", body: { amount_cents: amountCents } },
  )
  return { payment: toInvoicePayment(payload.payment), invoice: toInvoice(payload.invoice) }
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
