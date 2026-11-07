-- Installment Anticipation with a Discount (UC19, ADR-0003).
--
-- Only what the operation changes is stored, on the Installment itself: it
-- moves to another invoice (`invoice_month`), its amount is reduced by its
-- share of the Discount (`amount_cents`), and it is marked as anticipated.
-- The marker matters because an anticipated Installment is history: the
-- re-plan after a change of the card's days leaves it where it is, and its
-- purchase can only change Category or have the remaining Installments
-- cancelled. There is no way back, so nothing here records how to undo it.
--
-- What the Installment was before is kept for display: the invoice it came
-- from, and the Discount it received (the amount it had is `amount_cents +
-- discount_cents`).

ALTER TABLE installments
    ADD COLUMN anticipated_on         DATE,
    ADD COLUMN original_invoice_month DATE
        CHECK (original_invoice_month IS NULL OR EXTRACT(DAY FROM original_invoice_month) = 1),
    ADD COLUMN discount_cents         BIGINT NOT NULL DEFAULT 0 CHECK (discount_cents >= 0),
    ADD CONSTRAINT installments_anticipation_consistent CHECK (
        (anticipated_on IS NULL) = (original_invoice_month IS NULL)
        AND (anticipated_on IS NOT NULL OR discount_cents = 0)
    );
