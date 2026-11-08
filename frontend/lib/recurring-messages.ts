import { ApiError } from "@/lib/api"
import { transactionFailure, transactionFieldMessage } from "@/lib/transaction-messages"

/** What the person sees for each recurring-transaction error code or field rule (see backend/src/error.rs). */
const MESSAGES: Record<string, string> = {
  recurring_transaction_not_found: "Este lançamento fixo não existe mais. Atualize a página.",
  occurrence_not_found: "Esta ocorrência não existe mais. Atualize a página.",
  recurring_transaction_ended: "Este lançamento fixo já foi encerrado e não pode mais ser alterado.",
  day_of_month_range: "Escolha um dia do mês de 1 a 31.",
  start_month_past: "O primeiro mês não pode ser anterior ao mês atual.",
  last_month_before_history: "Não é possível encerrar antes de ocorrências que já aconteceram.",
}

/** The message of one field rule of a validation error, if we know it. */
export function recurringFieldMessage(error: unknown, field: string): string | null {
  if (error instanceof ApiError) {
    return MESSAGES[error.fields[field] ?? ""] ?? transactionFieldMessage(error, field)
  }
  return null
}

/** Message for a failed action on a recurring transaction or Occurrence; never shows a code. */
export function recurringFailure(error: unknown, action: string): string {
  if (error instanceof ApiError) {
    const specific =
      MESSAGES[error.code] ?? Object.values(error.fields).map((rule) => MESSAGES[rule]).find(Boolean)
    if (specific) return specific
  }
  return transactionFailure(error, action)
}
