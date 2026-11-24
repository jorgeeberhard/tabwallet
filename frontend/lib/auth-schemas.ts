import { z } from "zod"

import {
  MAX_PASSWORD_LENGTH,
  MIN_PASSWORD_LENGTH,
  PASSWORD_TOO_LONG_MESSAGE,
  PASSWORD_TOO_SHORT_MESSAGE,
} from "@/lib/password-rule"

const email = z
  .string()
  .trim()
  .min(1, "Digite seu e-mail.")
  .pipe(z.email("Digite um e-mail válido, como nome@dominio.com."))

// A new password: the browser checks only the length. Everything else of the
// Password policy (common passwords, sequences, e-mail, name) is decided by the
// API and shown from its error codes.
const newPassword = z
  .string()
  .min(MIN_PASSWORD_LENGTH, PASSWORD_TOO_SHORT_MESSAGE)
  .max(MAX_PASSWORD_LENGTH, PASSWORD_TOO_LONG_MESSAGE)

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
