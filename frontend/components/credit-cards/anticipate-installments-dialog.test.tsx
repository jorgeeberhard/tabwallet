import { QueryClient, QueryClientProvider } from "@tanstack/react-query"
import { render, screen, within } from "@testing-library/react"
import userEvent from "@testing-library/user-event"
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest"

import { AnticipateInstallmentsDialog } from "@/components/credit-cards/anticipate-installments-dialog"

const toast = vi.hoisted(() => ({ success: vi.fn(), error: vi.fn() }))
vi.mock("sonner", () => ({ toast }))

function cycle(month: string, state = "open") {
  return {
    closing_month: month,
    closing_date: `${month}-10`,
    due_date: `${month}-17`,
    reference_month: month,
    state,
  }
}

function installment(id: string, number: number, month: string, amountCents: number) {
  return {
    id,
    installment_purchase_id: "ip1",
    number,
    installment_count: 4,
    amount_cents: amountCents,
    total_cents: 10_000,
    date: "2026-02-05",
    description: "Geladeira",
    category: { id: "c1", name: "Outros", type: "expense", active: true },
    credit_card: { id: "k1", name: "Nubank", active: true },
    invoice: cycle(month),
    reference_month: month,
    purchase_has_paid_installments: false,
    purchase_has_anticipated_installments: false,
    anticipated: false,
    discount_cents: 0,
    original_amount_cents: amountCents,
    original_invoice_month: null,
  }
}

const listBody = {
  open_invoice: cycle("2026-02"),
  installments: [
    installment("i3", 3, "2026-04", 2_500),
    installment("i4", 4, "2026-05", 2_500),
  ],
}

function json(status: number, body?: unknown) {
  return Promise.resolve(
    new Response(body === undefined ? null : JSON.stringify(body), {
      status,
      headers: { "Content-Type": "application/json" },
    }),
  )
}

describe("AnticipateInstallmentsDialog", () => {
  const fetchMock = vi.fn()
  const onClose = vi.fn()

  beforeEach(() => {
    vi.stubGlobal("fetch", fetchMock)
    // jsdom has none; the Radix checkbox measures itself with it.
    vi.stubGlobal(
      "ResizeObserver",
      class {
        observe() {}
        unobserve() {}
        disconnect() {}
      },
    )
    onClose.mockReset()
    toast.success.mockReset()
  })

  afterEach(() => {
    vi.unstubAllGlobals()
    fetchMock.mockReset()
  })

  function renderDialog() {
    const client = new QueryClient({ defaultOptions: { queries: { retry: false } } })
    render(
      <QueryClientProvider client={client}>
        <AnticipateInstallmentsDialog cardId="k1" cardName="Nubank" onClose={onClose} />
      </QueryClientProvider>,
    )
  }

  it("lists the future installments by invoice and shows the resulting total", async () => {
    fetchMock.mockImplementation(() => json(200, listBody))
    renderDialog()

    expect(await screen.findByText("Fatura de abril de 2026")).toBeInTheDocument()
    expect(screen.getByText("Fatura de maio de 2026")).toBeInTheDocument()
    expect(screen.getByText(/fatura aberta de fevereiro de 2026/)).toBeInTheDocument()

    const boxes = screen.getAllByRole("checkbox")
    await userEvent.click(boxes[1])
    await userEvent.click(boxes[2])
    await userEvent.type(screen.getByLabelText("Desconto (opcional)"), "10")

    expect(screen.getByText("2 parcelas escolhidas")).toBeInTheDocument()
    expect(screen.getByTestId("anticipation-total")).toHaveTextContent("R$ 40,00")
  })

  it("warns that it cannot be undone and only sends after the confirmation, with the discount in cents", async () => {
    fetchMock.mockImplementation((url: string, init?: RequestInit) =>
      init?.method === "POST"
        ? json(200, {
            anticipation: {
              original_total_cents: 5_000,
              discount_cents: 1_000,
              total_cents: 4_000,
              invoice: { closing_month: "2026-02", reference_month: "2026-02", total_cents: 4_000 },
            },
          })
        : json(200, listBody),
    )
    renderDialog()
    await screen.findByText("Fatura de abril de 2026")

    await userEvent.click(screen.getByRole("checkbox", { name: "Selecionar todas as parcelas" }))
    await userEvent.type(screen.getByLabelText("Desconto (opcional)"), "10")
    await userEvent.click(screen.getByRole("button", { name: "Continuar" }))

    const confirmation = await screen.findByRole("alertdialog")
    expect(within(confirmation).getByText("Essa operação não pode ser desfeita.")).toBeInTheDocument()
    expect(fetchMock).not.toHaveBeenCalledWith(
      expect.stringContaining("installment-anticipations"),
      expect.anything(),
    )

    await userEvent.click(within(confirmation).getByRole("button", { name: "Antecipar parcelas" }))

    await vi.waitFor(() => expect(onClose).toHaveBeenCalled())
    expect(fetchMock).toHaveBeenCalledWith(
      "/api/credit-cards/k1/installment-anticipations",
      expect.objectContaining({
        method: "POST",
        body: JSON.stringify({
          installment_ids: ["i3", "i4"],
          discount_cents: 1000,
          confirm: true,
        }),
      }),
    )
    expect(toast.success).toHaveBeenCalled()
  })

  it("sends no discount when the field is empty", async () => {
    fetchMock.mockImplementation((url: string, init?: RequestInit) =>
      init?.method === "POST"
        ? json(200, {
            anticipation: {
              original_total_cents: 2_500,
              discount_cents: 0,
              total_cents: 2_500,
              invoice: { closing_month: "2026-02", reference_month: "2026-02", total_cents: 5_000 },
            },
          })
        : json(200, listBody),
    )
    renderDialog()
    await screen.findByText("Fatura de abril de 2026")

    await userEvent.click(screen.getAllByRole("checkbox")[1])
    await userEvent.click(screen.getByRole("button", { name: "Continuar" }))
    await userEvent.click(await screen.findByRole("button", { name: "Antecipar parcelas" }))

    await vi.waitFor(() => expect(onClose).toHaveBeenCalled())
    expect(fetchMock).toHaveBeenCalledWith(
      "/api/credit-cards/k1/installment-anticipations",
      expect.objectContaining({
        body: JSON.stringify({ installment_ids: ["i3"], confirm: true }),
      }),
    )
  })

  it("needs at least one installment and a discount below their total, without calling the API", async () => {
    fetchMock.mockImplementation(() => json(200, listBody))
    renderDialog()
    await screen.findByText("Fatura de abril de 2026")

    await userEvent.click(screen.getByRole("button", { name: "Continuar" }))
    expect(await screen.findByText("Escolha pelo menos uma parcela.")).toBeInTheDocument()

    await userEvent.click(screen.getAllByRole("checkbox")[1])
    await userEvent.type(screen.getByLabelText("Desconto (opcional)"), "25")
    await userEvent.click(screen.getByRole("button", { name: "Continuar" }))
    expect(
      await screen.findByText("O desconto precisa ser menor que o total das parcelas escolhidas."),
    ).toBeInTheDocument()
    expect(screen.queryByRole("alertdialog")).not.toBeInTheDocument()
    expect(fetchMock).toHaveBeenCalledTimes(1)
  })

  it("explains a refusal of the API and goes back to the choice", async () => {
    fetchMock.mockImplementation((url: string, init?: RequestInit) =>
      init?.method === "POST"
        ? json(409, { error: { code: "installments_not_anticipable", message: "x" } })
        : json(200, listBody),
    )
    renderDialog()
    await screen.findByText("Fatura de abril de 2026")

    await userEvent.click(screen.getAllByRole("checkbox")[1])
    await userEvent.click(screen.getByRole("button", { name: "Continuar" }))
    await userEvent.click(await screen.findByRole("button", { name: "Antecipar parcelas" }))

    expect(await screen.findByText(/não pode mais ser antecipada/)).toBeInTheDocument()
    expect(onClose).not.toHaveBeenCalled()
  })

  it("says so when there is nothing to anticipate", async () => {
    fetchMock.mockImplementation(() => json(200, { open_invoice: cycle("2026-02"), installments: [] }))
    renderDialog()

    expect(await screen.findByText(/não tem parcelas em faturas futuras/)).toBeInTheDocument()
    expect(screen.getByRole("button", { name: "Continuar" })).toBeDisabled()
  })
})
