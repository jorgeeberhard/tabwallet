import { describe, expect, it } from "vitest"

import { logInSchema, signUpSchema } from "@/lib/auth-schemas"

function messages(result: { success: boolean; error?: { issues: { path: PropertyKey[]; message: string }[] } }) {
  return Object.fromEntries(
    (result.error?.issues ?? []).map((issue) => [String(issue.path[0]), issue.message]),
  )
}

describe("signUpSchema", () => {
  it("accepts a valid e-mail and password and trims the e-mail", () => {
    const result = signUpSchema.safeParse({
      email: "  ana@example.com ",
      password: "correct horse battery",
    })

    expect(result.success).toBe(true)
    expect(result.data?.email).toBe("ana@example.com")
  })

  it("tells the person what to do about an invalid e-mail and a short password", () => {
    const result = signUpSchema.safeParse({ email: "ana", password: "short" })

    expect(messages(result)).toEqual({
      email: "Digite um e-mail válido, como nome@dominio.com.",
      password: "Use uma senha com pelo menos 8 caracteres.",
    })
  })
})

describe("logInSchema", () => {
  it("asks for the missing fields", () => {
    const result = logInSchema.safeParse({ email: "", password: "" })

    expect(messages(result)).toEqual({
      email: "Digite seu e-mail.",
      password: "Digite sua senha.",
    })
  })

  it("does not apply the sign-up password length rule", () => {
    expect(logInSchema.safeParse({ email: "ana@example.com", password: "x" }).success).toBe(true)
  })
})
