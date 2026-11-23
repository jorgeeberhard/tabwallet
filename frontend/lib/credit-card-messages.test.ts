import { describe, expect, it } from "vitest"

import { ApiError } from "@/lib/api"
import { creditCardFailure } from "@/lib/credit-card-messages"

describe("creditCardFailure", () => {
  it("explains that a change cannot take an invoice below what was paid early", () => {
    const error = new ApiError(409, "invoice_would_be_overpaid")

    expect(creditCardFailure(error, "alterar a compra")).toMatch(/pago nela antecipadamente/)
  })
})
