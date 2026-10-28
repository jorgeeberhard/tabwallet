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

export type SignUpValues = z.infer<typeof signUpSchema>
export type LogInValues = z.infer<typeof logInSchema>
