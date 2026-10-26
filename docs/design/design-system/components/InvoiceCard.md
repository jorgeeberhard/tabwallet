# InvoiceCard

One credit-card invoice: card, month, total, status, closing and due dates, and advance payments (UC15–UC17).

- Props: `cardName`, `month` ("outubro"), `total`, `status` `open` | `closed` | `paid`, `closingDay` (UC16), `dueDate` ("DD/MM/AAAA"), `paid` (advanced amount), `onAdvance` (shows "Adiantar pagamento" while not paid).
- Status badges: Aberta `brand`, Fechada `warning`, Paga `income`.
- "Adiantar pagamento" opens a Dialog with a `MoneyInput`, pre-filled with the remaining amount.
- Instalment purchases (UC18) appear inside the invoice as `TransactionRow`s with "Parcela n/N".

## shadcn/ui

Own component on top of `card` + `badge` (`brand` / `warning` / `income` for Aberta / Fechada / Paga) + `button` (`outline`). "Adiantar pagamento" opens a `dialog` with a `MoneyInput`.
