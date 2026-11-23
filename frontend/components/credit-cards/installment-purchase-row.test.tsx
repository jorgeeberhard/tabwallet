import { render, screen } from "@testing-library/react"
import userEvent from "@testing-library/user-event"
import { describe, expect, it, vi } from "vitest"

import { InstallmentPurchaseRow } from "@/components/credit-cards/installment-purchase-row"
import { InvoiceInstallmentRow } from "@/components/credit-cards/invoice-installment-row"
import type {
  InstallmentPurchase,
  InvoiceCycle,
  InvoiceInstallment,
} from "@/lib/credit-cards-api"

function cycle(month: string, state: InvoiceCycle["state"]): InvoiceCycle {
  return {
    closingMonth: month,
    closingDate: `${month}-10`,
    dueDate: `${month}-17`,
    referenceMonth: month,
    state,
  }
}

const purchase: InstallmentPurchase = {
  id: "ip1",
  totalCents: 10_000,
  total: 100,
  installmentCount: 3,
  date: "2026-01-05",
  description: "Geladeira",
  category: { id: "c1", name: "Outros", active: true },
  creditCard: { id: "k1", name: "Nubank", active: true },
  cancelled: false,
  hasPaidInstallments: false,
  hasAnticipatedInstallments: false,
  discountCents: 0,
  isProjection: false,
  installments: [
    {
      id: "i1",
      number: 1,
      amountCents: 3_334,
      amount: 33.34,
      invoice: cycle("2026-01", "open"),
      referenceMonth: "2026-01",
      anticipated: false,
      discountCents: 0,
      originalAmountCents: 3_333,
      originalInvoiceMonth: null,
    },
    {
      id: "i2",
      number: 2,
      amountCents: 3_333,
      amount: 33.33,
      invoice: cycle("2026-02", "open"),
      referenceMonth: "2026-02",
      anticipated: false,
      discountCents: 0,
      originalAmountCents: 3_333,
      originalInvoiceMonth: null,
    },
    {
      id: "i3",
      number: 3,
      amountCents: 3_333,
      amount: 33.33,
      invoice: cycle("2026-03", "open"),
      referenceMonth: "2026-03",
      anticipated: false,
      discountCents: 0,
      originalAmountCents: 3_333,
      originalInvoiceMonth: null,
    },
  ],
}

function renderRow(value: InstallmentPurchase, handlers = { onEdit: vi.fn(), onCancel: vi.fn() }) {
  render(
    <ul>
      <InstallmentPurchaseRow purchase={value} {...handlers} />
    </ul>,
  )
  return handlers
}

describe("InstallmentPurchaseRow", () => {
  it("shows the total, the number of installments, the card and the invoices", () => {
    renderRow(purchase)

    expect(screen.getByText("Geladeira")).toBeInTheDocument()
    expect(screen.getByText("− R$ 100,00")).toHaveClass("text-expense")
    expect(screen.getByText("3x")).toBeInTheDocument()
    expect(screen.getByText("Nubank")).toBeInTheDocument()
    expect(
      screen.getByText("Faturas de janeiro de 2026 a março de 2026"),
    ).toBeInTheDocument()
  })

  it("lists each installment as number/count with its invoice", () => {
    renderRow(purchase)

    const list = screen.getByRole("list", { name: "Parcelas de Geladeira" })
    expect(list).toHaveTextContent("1/3")
    expect(list).toHaveTextContent("3/3")
    expect(list).toHaveTextContent("fatura de fevereiro de 2026 · aberta")
  })

  it("lets the person edit and delete a purchase with no paid installment", async () => {
    const handlers = renderRow(purchase)

    await userEvent.click(screen.getByRole("button", { name: "Editar Geladeira" }))
    await userEvent.click(screen.getByRole("button", { name: "Excluir Geladeira" }))

    expect(handlers.onEdit).toHaveBeenCalledWith(purchase)
    expect(handlers.onCancel).toHaveBeenCalledWith(purchase)
  })

  it("offers to cancel only the remaining installments when some are paid", async () => {
    const paid: InstallmentPurchase = {
      ...purchase,
      hasPaidInstallments: true,
      installments: purchase.installments.map((installment, index) =>
        index === 0 ? { ...installment, invoice: cycle("2026-01", "paid") } : installment,
      ),
    }
    const handlers = renderRow(paid)

    await userEvent.click(
      screen.getByRole("button", { name: "Cancelar parcelas restantes Geladeira" }),
    )

    expect(handlers.onCancel).toHaveBeenCalledWith(paid)
  })

  it("turns cancelling off when every installment is paid", () => {
    renderRow({
      ...purchase,
      hasPaidInstallments: true,
      installments: purchase.installments.map((installment) => ({
        ...installment,
        invoice: cycle(installment.referenceMonth, "paid"),
      })),
    })

    expect(
      screen.getByRole("button", { name: "Cancelar parcelas restantes Geladeira" }),
    ).toBeDisabled()
  })

  it("shows anticipated installments and cancels only the others", async () => {
    const anticipated: InstallmentPurchase = {
      ...purchase,
      hasAnticipatedInstallments: true,
      installments: purchase.installments.map((installment, index) =>
        index === 2
          ? {
              ...installment,
              anticipated: true,
              discountCents: 100,
              originalInvoiceMonth: "2026-03",
              invoice: cycle("2026-01", "open"),
              referenceMonth: "2026-01",
            }
          : installment,
      ),
    }
    const handlers = renderRow(anticipated)

    expect(screen.getByText("Antecipada · desconto de R$ 1,00")).toBeInTheDocument()
    await userEvent.click(
      screen.getByRole("button", { name: "Cancelar parcelas restantes Geladeira" }),
    )
    expect(handlers.onCancel).toHaveBeenCalledWith(anticipated)
  })

  it("turns cancelling off when what is left is paid or anticipated", () => {
    renderRow({
      ...purchase,
      hasAnticipatedInstallments: true,
      installments: purchase.installments.map((installment) => ({
        ...installment,
        anticipated: true,
      })),
    })

    expect(
      screen.getByRole("button", { name: "Cancelar parcelas restantes Geladeira" }),
    ).toBeDisabled()
  })

  it("says when the remaining installments were cancelled", () => {
    renderRow({
      ...purchase,
      cancelled: true,
      hasPaidInstallments: true,
      installments: purchase.installments.slice(0, 2).map((installment) => ({
        ...installment,
        invoice: cycle(installment.referenceMonth, "paid"),
      })),
    })

    expect(screen.getByText("2 de 3 parcelas · restantes canceladas")).toBeInTheDocument()
  })
})

const line: InvoiceInstallment = {
  id: "i2",
  installmentPurchaseId: "ip1",
  number: 3,
  installmentCount: 10,
  amountCents: 3_333,
  amount: 33.33,
  totalCents: 33_333,
  date: "2026-01-05",
  description: "Geladeira",
  category: { id: "c1", name: "Outros", active: true },
  creditCard: { id: "k1", name: "Nubank", active: true },
  invoice: cycle("2026-03", "open"),
  referenceMonth: "2026-03",
  purchaseHasPaidInstallments: false,
  purchaseHasAnticipatedInstallments: false,
  anticipated: false,
  discountCents: 0,
  originalAmountCents: 3_333,
  originalInvoiceMonth: null,
}

describe("InvoiceInstallmentRow", () => {
  it("shows the installment as number/count with its own amount", () => {
    render(
      <ul>
        <InvoiceInstallmentRow item={line} onEdit={vi.fn()} onCancel={vi.fn()} />
      </ul>,
    )

    expect(screen.getByText("3/10")).toBeInTheDocument()
    expect(screen.getByText("− R$ 33,33")).toHaveClass("text-expense")
    expect(screen.getByText("Fatura de março de 2026 · aberta")).toBeInTheDocument()
  })

  it("edits and cancels through the whole purchase", async () => {
    const onEdit = vi.fn()
    const onCancel = vi.fn()
    render(
      <ul>
        <InvoiceInstallmentRow item={line} onEdit={onEdit} onCancel={onCancel} />
      </ul>,
    )

    await userEvent.click(screen.getByRole("button", { name: "Editar Geladeira 3/10" }))
    await userEvent.click(screen.getByRole("button", { name: "Excluir Geladeira 3/10" }))

    expect(onEdit).toHaveBeenCalledWith(line)
    expect(onCancel).toHaveBeenCalledWith(line)
  })

  it("cannot cancel an installment of a paid invoice", () => {
    render(
      <ul>
        <InvoiceInstallmentRow
          item={{ ...line, invoice: cycle("2026-03", "paid"), purchaseHasPaidInstallments: true }}
          onEdit={vi.fn()}
          onCancel={vi.fn()}
        />
      </ul>,
    )

    expect(
      screen.getByRole("button", { name: "Cancelar parcelas restantes Geladeira 3/10" }),
    ).toBeDisabled()
    expect(screen.getByRole("button", { name: "Editar Geladeira 3/10" })).toBeEnabled()
  })

  it("marks an anticipated installment, where it came from and the discount, and cannot cancel it", () => {
    render(
      <ul>
        <InvoiceInstallmentRow
          item={{
            ...line,
            amountCents: 3_083,
            amount: 30.83,
            anticipated: true,
            discountCents: 250,
            originalAmountCents: 3_333,
            originalInvoiceMonth: "2026-06",
            purchaseHasAnticipatedInstallments: true,
          }}
          onEdit={vi.fn()}
          onCancel={vi.fn()}
        />
      </ul>,
    )

    expect(screen.getByText("Antecipada de junho de 2026")).toBeInTheDocument()
    expect(screen.getByText(/Desconto de R\$ 2,50 \(era R\$ 33,33\)/)).toBeInTheDocument()
    expect(
      screen.getByRole("button", { name: "Cancelar parcelas restantes Geladeira 3/10" }),
    ).toBeDisabled()
    expect(screen.getByRole("button", { name: "Editar Geladeira 3/10" })).toBeEnabled()
  })
})
