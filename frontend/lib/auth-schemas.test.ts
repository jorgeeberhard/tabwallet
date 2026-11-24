import { describe, expect, it } from "vitest"

import {
  logInSchema,
  profilePasswordSchema,
  resetPasswordSchema,
  signUpSchema,
} from "@/lib/auth-schemas"

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
      password: "A senha precisa ter pelo menos 10 caracteres.",
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

describe("the new-password length rule", () => {
  const password = (value: string) =>
    messages(signUpSchema.safeParse({ email: "ana@example.com", password: value })).password

  it("accepts from 10 to 128 characters", () => {
    expect(password("a".repeat(9))).toBe("A senha precisa ter pelo menos 10 caracteres.")
    expect(password("x7".repeat(5))).toBeUndefined()
    expect(password("x7".repeat(64))).toBeUndefined()
    expect(password("x".repeat(129))).toBe("A senha pode ter no máximo 128 caracteres.")
  })

  it("leaves everything else to the API: no composition rule and no common-password list here", () => {
    expect(password("12345678901")).toBeUndefined()
    expect(password("aaaaaaaaaa")).toBeUndefined()
    expect(password("senha12345")).toBeUndefined()
    expect(password("purplegiraffesdancequietly")).toBeUndefined()
  })

  it("applies to the reset and the profile forms as well", () => {
    expect(
      messages(resetPasswordSchema.safeParse({ password: "short", confirmation: "short" })).password,
    ).toBe("A senha precisa ter pelo menos 10 caracteres.")
    expect(
      messages(
        profilePasswordSchema.safeParse({
          currentPassword: "x",
          newPassword: "short",
          confirmation: "short",
        }),
      ).newPassword,
    ).toBe("A senha precisa ter pelo menos 10 caracteres.")
  })
})
