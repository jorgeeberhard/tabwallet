# Dialog

A modal that asks for one decision — confirmations of deletion (UC07), advance payment of an invoice (UC17) or a short form.

- Props: `open`, `title` (a question for confirmations), `tone` `danger` for destructive ones, `actions` (buttons, the primary last), `onClose` (backdrop click and Escape), children.
- Destructive confirmations name the item and the consequence, and their confirm button is `danger` with the same verb as the title ("Excluir").
- Focus moves into the dialog on open and returns to the trigger on close (handle it in the app, e.g. with Radix Dialog). `inline` only exists for documentation previews.

## shadcn/ui

`dialog` for forms (advance payment, quick edit); `alert-dialog` for every destructive confirmation (UC07), with `AlertDialogAction` styled as `buttonVariants({ variant: "destructive" })`. Content `rounded-xl max-w-[440px]`. Radix handles focus trap, Escape and focus return.
