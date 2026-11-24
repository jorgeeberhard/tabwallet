// The Password policy, as far as the browser knows it: the length and the
// composition (keep both in step with backend/src/auth/password_policy.rs).
// Common passwords, sequences and the e-mail or name in the password are
// decided by the API alone, and shown from its error codes (auth-messages.ts).
export const MIN_PASSWORD_LENGTH = 10
export const MAX_PASSWORD_LENGTH = 128
/** A password needs at least this many of the four character types. */
export const MIN_CHARACTER_TYPES = 3

export const PASSWORD_TOO_SHORT_MESSAGE = `A senha precisa ter pelo menos ${MIN_PASSWORD_LENGTH} caracteres.`
export const PASSWORD_TOO_LONG_MESSAGE = `A senha pode ter no máximo ${MAX_PASSWORD_LENGTH} caracteres.`
export const PASSWORD_COMPOSITION_MESSAGE = `A senha precisa ter pelo menos ${MIN_CHARACTER_TYPES} destes tipos de caractere: letras minúsculas, letras maiúsculas, números e símbolos.`

/** The short hint shown next to every new-password field. */
export const PASSWORD_HINT = `Use de ${MIN_PASSWORD_LENGTH} a ${MAX_PASSWORD_LENGTH} caracteres, com pelo menos ${MIN_CHARACTER_TYPES} destes tipos: letras minúsculas, letras maiúsculas, números e símbolos. Evite senhas comuns, sequências e o seu e-mail ou nome.`

/**
 * How many of the four character types the password has: a lowercase letter,
 * an uppercase letter, a digit (0 to 9) and a symbol, which is any character
 * that is neither a letter nor a digit (punctuation, a space, non-ASCII
 * punctuation). Accented letters are letters, lowercase or uppercase by their
 * case. A letter without case counts for none. Same counting as the API.
 */
export function countCharacterTypes(password: string): number {
  let lowercase = false
  let uppercase = false
  let digit = false
  let symbol = false
  for (const character of password) {
    if (/\p{Lowercase}/u.test(character)) lowercase = true
    else if (/\p{Uppercase}/u.test(character)) uppercase = true
    else if (/[0-9]/.test(character)) digit = true
    else if (!/\p{Alphabetic}/u.test(character)) symbol = true
  }
  return [lowercase, uppercase, digit, symbol].filter(Boolean).length
}
