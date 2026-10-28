import { ApiError } from "@/lib/api"

/** What the person sees for each stable API error code (see backend/src/error.rs). */
const MESSAGES: Record<string, string> = {
  email_taken: "Já existe uma conta com este e-mail. Entre ou use outro e-mail.",
  invalid_credentials: "E-mail ou senha incorretos. Confira os dados e tente de novo.",
  invalid_email: "Digite um e-mail válido, como nome@dominio.com.",
  password_length: "Use uma senha de 8 a 128 caracteres.",
  required: "Preencha este campo.",
  name_length: "Use um nome com até 100 caracteres.",
  invalid_current_password: "A senha atual não confere. Confira e tente de novo.",
  invalid_reset_token:
    "Este link de redefinição é inválido, expirou ou já foi usado. Peça um novo link.",
}

/** Message for an API error code, or `undefined` when it has no specific text. */
export function messageForCode(code: string): string | undefined {
  return MESSAGES[code]
}

/** Fallback for anything unexpected: say what to do, never show a code. */
export function genericFailure(action: string): string {
  return `Não foi possível ${action}. Verifique a conexão e tente de novo.`
}

export function isApiError(error: unknown): error is ApiError {
  return error instanceof ApiError
}
