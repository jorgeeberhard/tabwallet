import { ApiError } from "@/lib/api"
import { genericFailure } from "@/lib/auth-messages"

/** What the person sees for each balance error code or field rule (see backend/src/error.rs). */
const MESSAGES: Record<string, string> = {
  opening_balance_already_set:
    "O saldo inicial já foi informado e não pode ser alterado.",
  amount_range: "Digite um valor entre -R$ 1.000.000.000,00 e R$ 1.000.000.000,00.",
}

/** Message for a failed balance action; never shows a code. */
export function balanceFailure(error: unknown, action: string): string {
  if (error instanceof ApiError) {
    const specific =
      MESSAGES[error.code] ?? Object.values(error.fields).map((rule) => MESSAGES[rule]).find(Boolean)
    if (specific) return specific
  }
  return genericFailure(action)
}
