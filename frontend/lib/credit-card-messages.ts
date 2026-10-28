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
    "Esta compra pertence a uma fatura já paga e não pode ser alterada. A data escolhida também não pode cair numa fatura paga.",
  card_purchase_not_found: "Esta compra não existe mais. Atualize a página.",
  name_length: "Digite um nome de até 60 caracteres.",
  day_range: "Escolha um dia de 1 a 31.",
  amount_range: "Digite um valor maior que zero.",
  description_length: "Digite uma descrição de até 200 caracteres.",
  category_not_found: "Esta categoria não existe mais. Escolha outra.",
  category_type_mismatch: "Escolha uma categoria de despesa.",
  category_inactive: "Esta categoria está desativada. Escolha outra ou reative-a.",
}

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
