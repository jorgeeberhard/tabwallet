import { ApiError } from "@/lib/api"
import {
  PASSWORD_COMPOSITION_MESSAGE,
  PASSWORD_TOO_LONG_MESSAGE,
  PASSWORD_TOO_SHORT_MESSAGE,
} from "@/lib/password-rule"

/** What the person sees for each stable API error code (see backend/src/error.rs). */
const MESSAGES: Record<string, string> = {
  email_taken: "Já existe uma conta com este e-mail. Entre ou use outro e-mail.",
  invalid_credentials: "E-mail ou senha incorretos. Confira os dados e tente de novo.",
  invalid_email: "Digite um e-mail válido, como nome@dominio.com.",
  // The Password policy refusals (backend/src/auth/password_policy.rs).
  password_too_short: PASSWORD_TOO_SHORT_MESSAGE,
  password_too_long: PASSWORD_TOO_LONG_MESSAGE,
  password_composition: PASSWORD_COMPOSITION_MESSAGE,
  password_common: "Essa senha é muito comum e fácil de adivinhar. Escolha outra, de preferência uma frase.",
  password_sequence: "Evite sequências e repetições, como Abcdefghij1 ou Ab1Ab1Ab1Ab1. Escolha outra senha.",
  password_contains_email: "A senha não pode conter o seu e-mail. Escolha outra.",
  password_contains_name: "A senha não pode conter o seu nome. Escolha outra.",
  required: "Preencha este campo.",
  name_length: "Use um nome com até 100 caracteres.",
  invalid_current_password: "A senha atual não confere. Confira e tente de novo.",
  too_many_requests: "Muitas tentativas seguidas. Aguarde alguns minutos e tente de novo.",
  email_not_confirmed:
    "Confirme seu e-mail para entrar. Enviamos um link de confirmação para o endereço do cadastro.",
  invalid_confirmation_token:
    "Este link de confirmação é inválido, expirou ou já foi usado. Peça um novo link.",
  no_pending_email_change:
    "Não há troca de e-mail pendente: ela já foi concluída ou o link expirou. Peça a troca de novo, se ainda quiser.",
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
