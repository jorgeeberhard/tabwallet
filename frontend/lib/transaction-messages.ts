import { ApiError } from "@/lib/api"
import { genericFailure } from "@/lib/auth-messages"

/** What the person sees for each transaction error code or field rule (see backend/src/error.rs). */
const MESSAGES: Record<string, string> = {
  transaction_not_found: "Esta transação não existe mais. Atualize a página.",
  amount_range: "Digite um valor maior que zero.",
  description_length: "Digite uma descrição de até 200 caracteres.",
  category_not_found: "Esta categoria não existe mais. Escolha outra.",
  category_type_mismatch: "A categoria precisa ser do mesmo tipo da transação.",
  category_inactive: "Esta categoria está desativada. Escolha outra ou reative-a.",
}

/** The message of one field rule of a validation error, if we know it. */
export function transactionFieldMessage(error: unknown, field: string): string | null {
  if (error instanceof ApiError) {
    return MESSAGES[error.fields[field] ?? ""] ?? null
  }
  return null
}

/** Message for a failed transaction action; never shows a code. */
export function transactionFailure(error: unknown, action: string): string {
  if (error instanceof ApiError) {
    const specific =
      MESSAGES[error.code] ?? Object.values(error.fields).map((rule) => MESSAGES[rule]).find(Boolean)
    if (specific) return specific
  }
  return genericFailure(action)
}
