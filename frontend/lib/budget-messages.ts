import { ApiError } from "@/lib/api"
import { genericFailure } from "@/lib/auth-messages"

/** What the person sees for each budget error code or field rule (see backend/src/error.rs). */
const MESSAGES: Record<string, string> = {
  budget_not_found: "Esta categoria não tem mais orçamento. Atualize a página.",
  category_not_found: "Esta categoria não existe mais. Atualize a página.",
  category_type_mismatch: "Só categorias de despesa têm orçamento.",
  category_inactive: "Esta categoria está desativada. Reative-a para definir um orçamento.",
  amount_range: "Digite um limite maior que zero.",
}

/** Message for a failed budget action; never shows a code. */
export function budgetFailure(error: unknown, action: string): string {
  if (error instanceof ApiError) {
    const rule = error.fields.limit_cents ?? error.fields.category_id ?? ""
    const specific = MESSAGES[error.code] ?? MESSAGES[rule]
    if (specific) return specific
  }
  return genericFailure(action)
}
