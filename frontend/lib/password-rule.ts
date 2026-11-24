// The Password policy, as far as the browser knows it. Only the length is
// mirrored here (keep it in step with backend/src/auth/password_policy.rs):
// common passwords, sequences and the e-mail or name in the password are
// decided by the API alone, and shown from its error codes (auth-messages.ts).
export const MIN_PASSWORD_LENGTH = 10
export const MAX_PASSWORD_LENGTH = 128

export const PASSWORD_TOO_SHORT_MESSAGE = `A senha precisa ter pelo menos ${MIN_PASSWORD_LENGTH} caracteres.`
export const PASSWORD_TOO_LONG_MESSAGE = `A senha pode ter no máximo ${MAX_PASSWORD_LENGTH} caracteres.`

/** The short hint shown next to every new-password field. */
export const PASSWORD_HINT = `Use de ${MIN_PASSWORD_LENGTH} a ${MAX_PASSWORD_LENGTH} caracteres; uma frase longa é uma boa senha. Evite senhas comuns, sequências e o seu e-mail ou nome.`
