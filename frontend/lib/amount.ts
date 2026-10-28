/** Largest amount the API accepts, in cents (R$ 1 billion). Keep in sync with the back-end. */
export const MAX_AMOUNT_CENTS = 100_000_000_000

/**
 * Turns what the person typed in a money field into cents, or `null` if it is
 * not a valid amount greater than zero. Accepts "1180,4", "1.180,40" and
 * "1180.40"; never uses floating point, so cents are exact.
 */
export function parseAmountToCents(text: string): number | null {
  const cleaned = text.replace(/R\$|\s/g, "")
  let integerPart: string
  let fraction: string

  if (cleaned.includes(",")) {
    // Brazilian format: dots group thousands, the comma starts the cents.
    const match = /^(\d{1,3}(?:\.\d{3})+|\d+)(?:,(\d{1,2}))?$/.exec(cleaned)
    if (!match) return null
    integerPart = match[1].replace(/\./g, "")
    fraction = match[2] ?? ""
  } else if (/^\d{1,3}(\.\d{3})+$/.test(cleaned)) {
    // "1.180" is one thousand one hundred eighty, not 1.18.
    integerPart = cleaned.replace(/\./g, "")
    fraction = ""
  } else {
    const match = /^(\d+)(?:\.(\d{1,2}))?$/.exec(cleaned)
    if (!match) return null
    integerPart = match[1]
    fraction = match[2] ?? ""
  }

  if (integerPart.length > 12) return null
  const cents = Number(integerPart) * 100 + Number(fraction.padEnd(2, "0"))
  return cents > 0 && cents <= MAX_AMOUNT_CENTS ? cents : null
}

/** Cents as the text of a money field: 118040 becomes "1.180,40". */
export function formatAmountInput(cents: number): string {
  return new Intl.NumberFormat("pt-BR", {
    minimumFractionDigits: 2,
    maximumFractionDigits: 2,
  }).format(cents / 100)
}

/** Cents to reais, for the Design System components that take reais. */
export function centsToReais(cents: number): number {
  return cents / 100
}
