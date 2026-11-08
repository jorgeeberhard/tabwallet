/**
 * Automated accessibility check (axe-core) of the forms and dialogs of the main
 * flows: labels, names of buttons, dialog roles and ARIA use. jsdom has no
 * layout or colors, so contrast and focus order are verified in a real browser
 * (see docs/nao-funcionais.md).
 */
import { QueryClient, QueryClientProvider } from "@tanstack/react-query"
import { render } from "@testing-library/react"
import axe from "axe-core"
import type { ReactElement } from "react"
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest"

import { ForgotPasswordForm } from "@/components/auth/forgot-password-form"
import { LoginForm } from "@/components/auth/login-form"
import { ResetPasswordForm } from "@/components/auth/reset-password-form"
import { SignupForm } from "@/components/auth/signup-form"
import { OpeningBalanceDialog } from "@/components/balance/opening-balance-dialog"
import { BudgetDialog } from "@/components/budgets/budget-dialog"
import { CategoryDialog } from "@/components/categories/category-dialog"
import { CardPurchaseDialog } from "@/components/credit-cards/card-purchase-dialog"
import { CreditCardDialog } from "@/components/credit-cards/credit-card-dialog"
import { InstallmentPurchaseDialog } from "@/components/credit-cards/installment-purchase-dialog"
import { ProfileForms } from "@/components/profile/profile-forms"
import { RecurringDialog } from "@/components/recurring/recurring-dialog"
import { TransactionDialog } from "@/components/transactions/transaction-dialog"

vi.mock("next/navigation", () => ({
  useRouter: () => ({ replace: vi.fn(), refresh: vi.fn(), push: vi.fn() }),
}))
vi.mock("sonner", () => ({ toast: { success: vi.fn(), error: vi.fn() } }))

const categories = [
  { id: "c1", name: "Alimentação", type: "expense", active: true },
  { id: "c2", name: "Salário", type: "income", active: true },
]
const cards = [{ id: "k1", name: "Nubank", closing_day: 10, due_day: 17, active: true }]
const user = { id: "u1", email: "ana@example.com", name: "Ana", created_at: "2026-01-01T00:00:00Z" }

/** Answers the reads the dialogs make when they open. */
function fakeApi(input: RequestInfo | URL) {
  const path = String(input)
  let body: unknown = {}
  if (path.endsWith("/categories")) body = { categories }
  else if (path.endsWith("/credit-cards")) body = { credit_cards: cards }
  else if (path.endsWith("/auth/me")) body = { user }
  return Promise.resolve(
    new Response(JSON.stringify(body), {
      status: 200,
      headers: { "Content-Type": "application/json" },
    }),
  )
}

async function violationsIn(container: HTMLElement) {
  const results = await axe.run(container, {
    // jsdom does not lay anything out, so these cannot be judged here.
    rules: { "color-contrast": { enabled: false }, region: { enabled: false } },
  })
  return results.violations.map(
    (v) => `${v.id} (${v.impact}): ${v.help} -> ${v.nodes.map((n) => n.target).join(", ")}`,
  )
}

const noop = () => {}

const screens: Array<[string, () => ReactElement]> = [
  ["login form", () => <LoginForm />],
  ["sign-up form", () => <SignupForm />],
  ["forgot-password form", () => <ForgotPasswordForm />],
  ["reset-password form", () => <ResetPasswordForm token="abc" />],
  ["profile forms", () => <ProfileForms />],
  ["transaction dialog", () => <TransactionDialog mode={{ kind: "create", type: "expense" }} onClose={noop} />],
  ["opening balance dialog", () => <OpeningBalanceDialog onClose={noop} />],
  ["category dialog", () => <CategoryDialog mode={{ kind: "create", type: "expense" }} onClose={noop} onSaved={noop} />],
  ["recurring dialog", () => <RecurringDialog mode={{ kind: "create", type: "expense" }} onClose={noop} />],
  ["credit card dialog", () => <CreditCardDialog mode={{ kind: "create" }} onClose={noop} />],
  ["card purchase dialog", () => <CardPurchaseDialog mode={{ kind: "create" }} onClose={noop} />],
  ["installment purchase dialog", () => <InstallmentPurchaseDialog mode={{ kind: "create" }} onClose={noop} />],
  [
    "budget dialog",
    () => (
      <BudgetDialog
        target={{ categoryId: "c1", categoryName: "Alimentação", currentLimitCents: null }}
        currentMonth="2026-01"
        onClose={noop}
      />
    ),
  ],
]

describe("accessibility of forms and dialogs (axe)", () => {
  beforeEach(() => {
    vi.stubGlobal("fetch", vi.fn(fakeApi))
  })

  afterEach(() => {
    vi.unstubAllGlobals()
  })

  it("really catches a field without a label (the check itself works)", async () => {
    render(
      <form>
        <input type="text" />
        <button type="submit" />
      </form>,
    )

    const violations = await violationsIn(document.body)

    expect(violations.join("\n")).toContain("label")
    expect(violations.join("\n")).toContain("button-name")
  })

  it.each(screens)("%s has no axe violations", async (_name, element) => {
    const client = new QueryClient({ defaultOptions: { queries: { retry: false } } })
    render(<QueryClientProvider client={client}>{element()}</QueryClientProvider>)
    // Let the queries answer so the fields that depend on them are in the DOM.
    await new Promise((resolve) => setTimeout(resolve, 50))

    // Dialogs render in a portal, outside the render container.
    expect(await violationsIn(document.body)).toEqual([])
  })
})
