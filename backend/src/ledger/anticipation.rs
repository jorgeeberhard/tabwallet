//! Installment Anticipation with a Discount (UC19, ADR-0003).
//!
//! The user brings Installments of a card that sit in invoices after the open
//! one into the open invoice, optionally with a Discount. This module only
//! plans the operation: which Installments may move, and what each one's
//! amount becomes. Storing the result is for the repository.
//!
//! - Only an Installment in an invoice later than the open one can be
//!   anticipated. That also rules out the closed and the paid invoices, which
//!   come before the open one, and Installments that were already
//!   anticipated (they are in an invoice that is not later any more).
//! - The Discount is split in proportion to the Installments' amounts, in
//!   exact cents. Each share is rounded down and the cents left over go, one
//!   each, to the Installments with the largest fractions of a cent (the
//!   most proportional split); on a tie to the one that comes first, that is,
//!   in the earliest invoice, then in the order the caller listed them.
//! - The Discount can be zero (no Discount), but it can never leave an
//!   Installment with less than one cent: a Discount equal to the whole sum
//!   is refused as well.

use chrono::NaiveDate;

use super::cards::{CardFacts, InvoiceCycle, Month};
use crate::ids::InstallmentId;

/// An Installment the user chose to anticipate, as it is stored.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InstallmentToAnticipate {
    pub id: InstallmentId,
    /// The closing month of the invoice it is in now.
    pub invoice_month: Month,
    /// Cents, greater than zero.
    pub amount_cents: i64,
}

/// What an anticipated Installment becomes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AnticipatedShare {
    pub id: InstallmentId,
    /// The invoice it comes from.
    pub from_month: Month,
    /// Cents before the Discount.
    pub original_cents: i64,
    /// Cents of the Discount this Installment received.
    pub discount_cents: i64,
    /// Cents after the Discount, always at least one.
    pub amount_cents: i64,
}

/// An Installment Anticipation, planned and not yet stored.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AnticipationPlan {
    /// The open invoice, where every chosen Installment goes.
    pub invoice: InvoiceCycle,
    /// The chosen Installments, earliest invoice first.
    pub installments: Vec<AnticipatedShare>,
}

impl AnticipationPlan {
    /// The sum of the Installments before the Discount.
    pub fn original_total_cents(&self) -> i64 {
        self.installments
            .iter()
            .map(|share| share.original_cents)
            .sum()
    }

    pub fn discount_cents(&self) -> i64 {
        self.installments
            .iter()
            .map(|share| share.discount_cents)
            .sum()
    }

    /// What the Installments add up to in the open invoice.
    pub fn total_cents(&self) -> i64 {
        self.original_total_cents() - self.discount_cents()
    }
}

/// Why an Installment Anticipation cannot be planned.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AnticipationError {
    /// The ledger does not know the card.
    UnknownCard,
    /// No Installment was chosen.
    NothingChosen,
    /// This Installment is not in an invoice later than the open one.
    NotAnticipable(InstallmentId),
    /// The Discount is negative, or would leave an Installment with less than
    /// one cent (which includes a Discount of the whole sum or more).
    DiscountTooLarge,
}

/// Splits `discount_cents` between Installments of these `amounts` in
/// proportion to them (see the module docs for where the leftover cents
/// go). `None` if the Discount is negative, or if some Installment would be
/// left with less than one cent.
pub fn split_discount(amounts: &[i64], discount_cents: i64) -> Option<Vec<i64>> {
    let total: i64 = amounts.iter().sum();
    if discount_cents < 0 || total <= 0 {
        return None;
    }
    // Exact fractions: amount * discount / total, kept as (whole, remainder).
    let total_wide = i128::from(total);
    let mut shares = Vec::with_capacity(amounts.len());
    let mut remainders = Vec::with_capacity(amounts.len());
    for &amount in amounts {
        let scaled = i128::from(amount) * i128::from(discount_cents);
        shares.push((scaled / total_wide) as i64);
        remainders.push(scaled % total_wide);
    }
    let leftover = discount_cents - shares.iter().sum::<i64>();
    let mut by_remainder: Vec<usize> = (0..amounts.len()).collect();
    // Stable: equal remainders keep their order, so the earlier one wins.
    by_remainder.sort_by(|&a, &b| remainders[b].cmp(&remainders[a]));
    for &index in by_remainder.iter().take(leftover as usize) {
        shares[index] += 1;
    }
    amounts
        .iter()
        .zip(&shares)
        .all(|(amount, share)| share < amount)
        .then_some(shares)
}

impl CardFacts {
    /// Plans bringing `chosen` into the invoice that is open on `today`, with
    /// a Discount of `discount_cents` (zero for none). `chosen` should be in
    /// the order a tie of the split is to be settled in; the plan lists them
    /// by invoice first.
    pub fn plan_anticipation(
        &self,
        chosen: &[InstallmentToAnticipate],
        discount_cents: i64,
        today: NaiveDate,
    ) -> Result<AnticipationPlan, AnticipationError> {
        if chosen.is_empty() {
            return Err(AnticipationError::NothingChosen);
        }
        let open_month = self.schedule.first_open_month(today);
        if let Some(early) = chosen.iter().find(|item| item.invoice_month <= open_month) {
            return Err(AnticipationError::NotAnticipable(early.id));
        }
        let mut ordered = chosen.to_vec();
        ordered.sort_by_key(|item| item.invoice_month);
        let amounts: Vec<i64> = ordered.iter().map(|item| item.amount_cents).collect();
        let discounts =
            split_discount(&amounts, discount_cents).ok_or(AnticipationError::DiscountTooLarge)?;
        Ok(AnticipationPlan {
            invoice: self.cycle_of(open_month, today),
            installments: ordered
                .iter()
                .zip(discounts)
                .map(|(item, discount)| AnticipatedShare {
                    id: item.id,
                    from_month: item.invoice_month,
                    original_cents: item.amount_cents,
                    discount_cents: discount,
                    amount_cents: item.amount_cents - discount,
                })
                .collect(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::split_discount;

    #[test]
    fn a_discount_is_split_in_proportion_in_exact_cents() {
        assert_eq!(split_discount(&[2_500, 3_000, 2_500], 1_000).unwrap().iter().sum::<i64>(), 1_000);
        assert_eq!(split_discount(&[2_500, 3_000, 2_500], 1_000), Some(vec![313, 375, 312]));
        assert_eq!(split_discount(&[3_333, 3_333], 1), Some(vec![1, 0]));
        assert_eq!(split_discount(&[100, 300], 100), Some(vec![25, 75]));
    }

    #[test]
    fn no_discount_gives_every_installment_zero() {
        assert_eq!(split_discount(&[100, 200], 0), Some(vec![0, 0]));
    }

    #[test]
    fn a_discount_that_leaves_an_installment_without_a_cent_is_refused() {
        assert_eq!(split_discount(&[100, 200], 300), None);
        assert_eq!(split_discount(&[100, 200], 301), None);
        assert_eq!(split_discount(&[1, 10_000], 9_999), None);
        assert_eq!(split_discount(&[100], -1), None);
        assert_eq!(split_discount(&[100], 99), Some(vec![99]));
    }
}
