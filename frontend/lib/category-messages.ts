import { ApiError } from "@/lib/api"
import { genericFailure } from "@/lib/auth-messages"

/** What the person sees for each category error code (see backend/src/error.rs). */
const MESSAGES: Record<string, string> = {
  category_name_taken: "Já existe uma categoria com este nome neste tipo. Escolha outro nome.",
  category_inactive_exists: "Já existe uma categoria desativada com este nome.",
  category_has_charges:
    "Esta categoria tem encargos lançados e não pode ser desativada.",
  category_not_found: "Esta categoria não existe mais. Atualize a página.",
  name_length: "Digite um nome de até 60 caracteres.",
}

/** Message for a failed category action; never shows a code. */
export function categoryFailure(error: unknown, action: string): string {
  if (error instanceof ApiError) {
    const specific = MESSAGES[error.code] ?? MESSAGES[error.fields.name ?? ""]
    if (specific) return specific
  }
  return genericFailure(action)
}
