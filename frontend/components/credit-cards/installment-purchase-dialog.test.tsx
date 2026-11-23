import { QueryClient, QueryClientProvider } from "@tanstack/react-query"
import { render, screen } from "@testing-library/react"
import userEvent from "@testing-library/user-event"
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest"

import {
  InstallmentPurchaseDialog,
  type InstallmentPurchaseTarget,
  splitSummary,
} from "@/components/credit-cards/installment-purchase-dialog"

vi.mock("sonner", () => ({ toast: { success: vi.fn(), error: vi.fn() } }))

/** The summary uses the non-breaking space of `formatBRL`. */
function plain(text: string | null) {
  return text?.replaceAll(" ", " ")
}

describe("splitSummary", () => {
  it("shows the first installment apart when the division leaves cents over", () => {
    expect(plain(splitSummary(10_000, 3))).toBe("1ª parcela de R$ 33,34 e 2 de R$ 33,33.")
  })

  it("shows one amount when the division is exact", () => {
    expect(plain(splitSummary(12_000, 4))).toBe("4 parcelas de R$ 30,00.")
  })

  it("has nothing to show without a valid total and count", () => {
    expect(splitSummary(null, 3)).toBeNull()
    expect(splitSummary(10_000, 0)).toBeNull()
    expect(splitSummary(2, 3)).toBeNull()
  })
})

function json(status: number, body?: unknown) {
  return Promise.resolve(
    new Response(body === undefined ? null : JSON.stringify(body), {
      status,
      headers: { "Content-Type": "application/json" },
    }),
  )
}

const target: InstallmentPurchaseTarget = {
  id: "ip1",
  totalCents: 10_000,
  installmentCount: 3,
  date: "2026-01-05",
  description: "Geladeira",
  category: { id: "c1", name: "Outros", active: true },
  creditCard: { id: "k1", name: "Nubank" },
  hasPaidInstallments: false,
  hasAnticipatedInstallments: false,
}

function renderDialog(purchase: InstallmentPurchaseTarget) {
  const client = new QueryClient({ defaultOptions: { queries: { retry: false } } })
  return render(
    <QueryClientProvider client={client}>
      <InstallmentPurchaseDialog mode={{ kind: "edit", purchase }} onClose={vi.fn()} />
    </QueryClientProvider>,
  )
}

describe("InstallmentPurchaseDialog", () => {
  const fetchMock = vi.fn()

  beforeEach(() => {
    vi.stubGlobal("fetch", fetchMock)
    fetchMock.mockImplementation((url: string) => {
      if (url === "/api/categories") {
        return json(200, {
          categories: [
            { id: "c1", name: "Outros", type: "expense", active: true, is_charges: false },
          ],
        })
      }
      if (url === "/api/credit-cards") return json(200, { credit_cards: [] })
      return json(404)
    })
  })

  afterEach(() => {
    vi.unstubAllGlobals()
    fetchMock.mockReset()
  })

  it("previews how the total is split while editing", async () => {
    renderDialog(target)

    expect(screen.getByLabelText("Valor total")).toHaveValue("100,00")
    expect(screen.getByLabelText("Parcelas")).toHaveValue(3)
    expect(screen.getByText("1ª parcela de R$ 33,34 e 2 de R$ 33,33.")).toBeInTheDocument()

    await userEvent.clear(screen.getByLabelText("Parcelas"))
    await userEvent.type(screen.getByLabelText("Parcelas"), "4")

    expect(screen.getByText("4 parcelas de R$ 25,00.")).toBeInTheDocument()
  })

  it("lets the person change every field while no installment is paid", () => {
    renderDialog(target)

    expect(screen.getByLabelText("Valor total")).toBeEnabled()
    expect(screen.getByLabelText("Parcelas")).toBeEnabled()
    expect(screen.getByLabelText("Data da compra")).toBeEnabled()
    expect(screen.getByLabelText("Descrição")).toBeEnabled()
  })

  it("only lets the person change the category when some installment is paid", () => {
    renderDialog({ ...target, hasPaidInstallments: true })

    expect(screen.getByText(/já tem parcelas em fatura paga/)).toBeInTheDocument()
    expect(screen.getByLabelText("Valor total")).toBeDisabled()
    expect(screen.getByLabelText("Parcelas")).toBeDisabled()
    expect(screen.getByLabelText("Data da compra")).toBeDisabled()
    expect(screen.getByLabelText("Descrição")).toBeDisabled()
    expect(screen.getByLabelText("Categoria")).toBeEnabled()
  })

  it("also locks the purchase once some installment was anticipated", () => {
    renderDialog({ ...target, hasAnticipatedInstallments: true })

    expect(
      screen.getByText(/parcelas antecipadas, e a antecipação não pode ser desfeita/),
    ).toBeInTheDocument()
    expect(screen.getByLabelText("Valor total")).toBeDisabled()
    expect(screen.getByLabelText("Parcelas")).toBeDisabled()
    expect(screen.getByLabelText("Categoria")).toBeEnabled()
  })
})
