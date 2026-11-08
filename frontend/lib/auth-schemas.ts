import { z } from "zod"

// Keep these limits in step with the API (backend/src/handlers/auth.rs).
export const MIN_PASSWORD_LENGTH = 8
export const MAX_PASSWORD_LENGTH = 128

const email = z
  .string()
  .trim()
  .min(1, "Digite seu e-mail.")
  .pipe(z.email("Digite um e-mail válido, como nome@dominio.com."))

export const signUpSchema = z.object({
  email,
  password: z
    .string()
    .min(MIN_PASSWORD_LENGTH, `Use uma senha com pelo menos ${MIN_PASSWORD_LENGTH} caracteres.`)
    .max(MAX_PASSWORD_LENGTH, `Use uma senha com no máximo ${MAX_PASSWORD_LENGTH} caracteres.`),
})

// At login the password is only checked for presence: the API decides.
export const logInSchema = z.object({
  email,
  password: z.string().min(1, "Digite sua senha."),
})

const newPassword = z
  .string()
  .min(MIN_PASSWORD_LENGTH, `Use uma senha com pelo menos ${MIN_PASSWORD_LENGTH} caracteres.`)
  .max(MAX_PASSWORD_LENGTH, `Use uma senha com no máximo ${MAX_PASSWORD_LENGTH} caracteres.`)

export const forgotPasswordSchema = z.object({ email })

export const resetPasswordSchema = z
  .object({
    password: newPassword,
    confirmation: z.string().min(1, "Repita a nova senha."),
  })
  .refine((values) => values.password === values.confirmation, {
    path: ["confirmation"],
    message: "As senhas não são iguais.",
  })

export const MAX_NAME_LENGTH = 100

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
export type ResetPasswordValues = z.infer<typeof resetPasswordSchema>
export type ProfileNameValues = z.infer<typeof profileNameSchema>
export type ProfileEmailValues = z.infer<typeof profileEmailSchema>
export type ProfilePasswordValues = z.infer<typeof profilePasswordSchema>
export type SignUpValues = z.infer<typeof signUpSchema>
export type LogInValues = z.infer<typeof logInSchema>
