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
      name: "  Ana Souza ",
      password: "Correct horse battery 7",
    })

    expect(result.success).toBe(true)
    expect(result.data?.email).toBe("ana@example.com")
    expect(result.data?.name).toBe("Ana Souza")
  })

  it("tells the person what to do about an invalid e-mail and a short password", () => {
    const result = signUpSchema.safeParse({ email: "ana", name: "Ana", password: "short" })

    expect(messages(result)).toEqual({
      email: "Digite um e-mail válido, como nome@dominio.com.",
      password: "A senha precisa ter pelo menos 10 caracteres.",
    })
  })

  it("requires a name of up to 100 characters", () => {
    const withName = (name: string) =>
      messages(
        signUpSchema.safeParse({
          email: "ana@example.com",
          name,
          password: "Correct horse battery 7",
        }),
      ).name

    expect(withName("   ")).toBe("Digite seu nome.")
    expect(withName("a".repeat(100))).toBeUndefined()
    expect(withName("a".repeat(101))).toBe("Use até 100 caracteres.")
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

// A password of the given length with three character types: A b 1 A b 1 ...
const mixed = (length: number) => "Ab1".repeat(Math.ceil(length / 3)).slice(0, length)

const COMPOSITION_MESSAGE =
  "A senha precisa ter pelo menos 3 destes tipos de caractere: letras minúsculas, letras maiúsculas, números e símbolos."

const password = (value: string) =>
  messages(signUpSchema.safeParse({ email: "ana@example.com", name: "Ana", password: value })).password

describe("the new-password length rule", () => {
  it("accepts from 10 to 128 characters", () => {
    expect(password(mixed(9))).toBe("A senha precisa ter pelo menos 10 caracteres.")
    expect(password(mixed(10))).toBeUndefined()
    expect(password(mixed(128))).toBeUndefined()
    expect(password(mixed(129))).toBe("A senha pode ter no máximo 128 caracteres.")
  })

  it("leaves common passwords, sequences, the e-mail and the name to the API", () => {
    expect(password("Senha12345!")).toBeUndefined()
    expect(password("Abcdefghij1!")).toBeUndefined()
    expect(password("Ab1!Ab1!Ab1!")).toBeUndefined()
    expect(password("Ana.Souza@Example1")).toBeUndefined()
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

describe("the new-password composition rule", () => {
  it.each([
    "1234512345",
    "1234567890",
    "aaaaaaaaaa",
    "abcdefghijkl",
    "ABCDEFGHIJKL",
    "abcdefghij1",
    "ABCDEFGHIJ1",
    "Abcdefghijkl",
    "the quick brown fox jumps",
    "çãõéíóúàê1",
    "ÇÃÕÉÍÓÚÀÊ1",
    "!#$%&/()=?!",
    "1234 5678 90",
  ])("refuses %j: fewer than 3 of the 4 character types", (value) => {
    expect(password(value)).toBe(COMPOSITION_MESSAGE)
  })

  it.each([
    ["lowercase, uppercase and digit", "Zk3mPq9wLx"],
    ["lowercase, digit and symbol", "nf8$kd29xq#w"],
    ["uppercase, digit and symbol", "ZK3MPQ9W!L"],
    ["all four", "Zk3!mPq9wLx"],
    ["accented letters, a space and a symbol", "ação São 2024!"],
    ["a non-ASCII symbol (em dash)", "senha—boa—7xy"],
    ["accented upper and lower case", "Çãõéíóúàê1"],
    ["a long mixed passphrase with spaces", "Li and Ana walk to the old market every morning"],
  ])("accepts %s", (_kind, value) => {
    expect(password(value)).toBeUndefined()
  })

  it("is judged after the length: a short password is told it is short", () => {
    expect(password("short")).toBe("A senha precisa ter pelo menos 10 caracteres.")
    expect(password("a".repeat(129))).toBe("A senha pode ter no máximo 128 caracteres.")
  })

  it("applies to the reset and the profile forms as well", () => {
    expect(
      messages(
        resetPasswordSchema.safeParse({ password: "1234512345", confirmation: "1234512345" }),
      ).password,
    ).toBe(COMPOSITION_MESSAGE)
    expect(
      messages(
        profilePasswordSchema.safeParse({
          currentPassword: "x",
          newPassword: "abcdefghijkl",
          confirmation: "abcdefghijkl",
        }),
      ).newPassword,
    ).toBe(COMPOSITION_MESSAGE)
  })
})
