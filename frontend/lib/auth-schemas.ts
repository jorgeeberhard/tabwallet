import { z } from "zod"

import {
  MAX_PASSWORD_LENGTH,
  MIN_CHARACTER_TYPES,
  MIN_PASSWORD_LENGTH,
  PASSWORD_COMPOSITION_MESSAGE,
  PASSWORD_TOO_LONG_MESSAGE,
  PASSWORD_TOO_SHORT_MESSAGE,
  countCharacterTypes,
} from "@/lib/password-rule"

const email = z
  .string()
  .trim()
  .min(1, "Digite seu e-mail.")
  .pipe(z.email("Digite um e-mail válido, como nome@dominio.com."))

// A new password: the browser checks the length and the composition (at least
// 3 of the 4 character types), in the API's order, and reports only the first
// that fails. Everything else of the Password policy (common passwords,
// sequences, e-mail, name) is decided by the API and shown from its error codes.
const newPassword = z.string().superRefine((value, context) => {
  if (value.length < MIN_PASSWORD_LENGTH) {
    context.addIssue({ code: "custom", message: PASSWORD_TOO_SHORT_MESSAGE })
  } else if (value.length > MAX_PASSWORD_LENGTH) {
    context.addIssue({ code: "custom", message: PASSWORD_TOO_LONG_MESSAGE })
  } else if (countCharacterTypes(value) < MIN_CHARACTER_TYPES) {
    context.addIssue({ code: "custom", message: PASSWORD_COMPOSITION_MESSAGE })
  }
})

export const MAX_NAME_LENGTH = 100

const name = z
  .string()
  .trim()
  .min(1, "Digite seu nome.")
  .max(MAX_NAME_LENGTH, `Use até ${MAX_NAME_LENGTH} caracteres.`)

export const signUpSchema = z.object({
  email,
  name,
  password: newPassword,
})

// At login the password is only checked for presence: the API decides.
export const logInSchema = z.object({
  email,
  password: z.string().min(1, "Digite sua senha."),
})

export const forgotPasswordSchema = z.object({ email })

/** Asks for the confirmation e-mail again, for an address the person types. */
export const resendConfirmationSchema = z.object({ email })

export const resetPasswordSchema = z
  .object({
    password: newPassword,
    confirmation: z.string().min(1, "Repita a nova senha."),
  })
  .refine((values) => values.password === values.confirmation, {
    path: ["confirmation"],
    message: "As senhas não são iguais.",
  })

export const profileNameSchema = z.object({
  name: z.string().trim().max(MAX_NAME_LENGTH, `Use até ${MAX_NAME_LENGTH} caracteres.`),
})

export const profileEmailSchema = z.object({
  email,
  currentPassword: z.string().min(1, "Digite sua senha atual para trocar o e-mail."),
})

export const profilePasswordSchema = z
  .object({
    currentPassword: z.string().min(1, "Digite sua senha atual."),
    newPassword,
    confirmation: z.string().min(1, "Repita a nova senha."),
  })
  .refine((values) => values.newPassword === values.confirmation, {
    path: ["confirmation"],
    message: "As senhas não são iguais.",
  })

export type ForgotPasswordValues = z.infer<typeof forgotPasswordSchema>
export type ResendConfirmationValues = z.infer<typeof resendConfirmationSchema>
export type ResetPasswordValues = z.infer<typeof resetPasswordSchema>
export type ProfileNameValues = z.infer<typeof profileNameSchema>
export type ProfileEmailValues = z.infer<typeof profileEmailSchema>
export type ProfilePasswordValues = z.infer<typeof profilePasswordSchema>
export type SignUpValues = z.infer<typeof signUpSchema>
export type LogInValues = z.infer<typeof logInSchema>
