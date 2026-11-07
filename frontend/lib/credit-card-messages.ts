import { ApiError } from "@/lib/api"
import { genericFailure } from "@/lib/auth-messages"

/** What the person sees for each credit-card error code or field rule (see backend/src/error.rs). */
const MESSAGES: Record<string, string> = {
  credit_card_name_taken: "Já existe um cartão com este nome. Escolha outro nome.",
  credit_card_has_remaining_invoice:
    "Este cartão ainda tem fatura com valor a pagar. Ele só pode ser desativado quando todas estiverem pagas.",
  credit_card_not_found: "Este cartão não existe mais. Atualize a página.",
  credit_card_inactive: "Este cartão está desativado. Escolha outro ou reative-o.",
  invoice_not_found: "Esta fatura não existe.",
  invoice_paid:
    "Esta fatura já está paga e não pode mais ser alterada. Uma compra não pode ser editada, excluída nem movida para uma fatura paga, e a fatura paga não recebe pagamento antecipado.",
  amount_exceeds_remaining:
    "O valor não pode passar do que ainda falta pagar nesta fatura. Atualize a página para ver o valor restante.",
  invoice_payment_not_due:
    "Esta fatura ainda não venceu. O pagamento só pode ser corrigido depois do vencimento, quando o sistema debita o valor.",
  invoice_payment_locked:
    "O pagamento desta fatura não pode mais ser corrigido: a fatura que recebeu o saldo rolado já está paga.",
  invoice_nothing_to_pay:
    "Esta fatura foi paga por inteiro antes do vencimento, então não há pagamento do vencimento para corrigir.",
  charge_not_found: "Este encargo não existe mais. Atualize a página.",
  card_purchase_not_found: "Esta compra não existe mais. Atualize a página.",
  installment_purchase_not_found: "Esta compra parcelada não existe mais. Atualize a página.",
  installments_paid:
    "Esta compra já tem parcelas em fatura paga. Só a categoria pode mudar e as parcelas restantes podem ser canceladas.",
  installment_count_range: "Escolha de 2 a 48 parcelas.",
  total_below_installments: "O valor total precisa dar pelo menos R$ 0,01 para cada parcela.",
  name_length: "Digite um nome de até 60 caracteres.",
  day_range: "Escolha um dia de 1 a 31.",
  amount_range: "Digite um valor maior que zero.",
  description_length: "Digite uma descrição de até 200 caracteres.",
  category_not_found: "Esta categoria não existe mais. Escolha outra.",
  category_type_mismatch: "Escolha uma categoria de despesa.",
  category_inactive: "Esta categoria está desativada. Escolha outra ou reative-a.",
}

/**
 * Charges always go to "Juros e encargos", which the person does not pick, so
 * a problem with that Category needs its own wording.
 */
export const CHARGES_CATEGORY_INACTIVE =
  'A categoria "Juros e encargos" está desativada. Reative-a em Categorias para lançar encargos.'

/** The message of one field rule of a validation error, if we know it. */
export function creditCardFieldMessage(error: unknown, field: string): string | null {
  if (error instanceof ApiError) {
    return MESSAGES[error.fields[field] ?? ""] ?? null
  }
  return null
}

/** Message for a failed card or purchase action; never shows a code. */
export function creditCardFailure(error: unknown, action: string): string {
  if (error instanceof ApiError) {
    const specific =
      MESSAGES[error.code] ??
      Object.values(error.fields)
        .map((rule) => MESSAGES[rule])
        .find(Boolean)
    if (specific) return specific
  }
  return genericFailure(action)
}
