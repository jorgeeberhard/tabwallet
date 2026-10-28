/** Largest amount the API accepts, in cents (R$ 1 billion). Keep in sync with the back-end. */
export const MAX_AMOUNT_CENTS = 100_000_000_000

/**
 * Reads a non-negative amount (zero allowed) into cents, or `null` if it is
 * not a valid amount. Accepts "1180,4", "1.180,40" and "1180.40"; never uses
 * floating point, so cents are exact.
 */
function parseMagnitude(text: string): number | null {
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
  return cents <= MAX_AMOUNT_CENTS ? cents : null
}

/**
 * Turns what the person typed in a money field into cents, or `null` if it is
 * not a valid amount greater than zero (Transaction amounts are positive).
 */
export function parseAmountToCents(text: string): number | null {
  const cents = parseMagnitude(text)
  return cents !== null && cents > 0 ? cents : null
}

/**
 * Like `parseAmountToCents`, but for a balance: zero and a leading "-" (or the
 * true minus "−") are accepted, so "-1.180,40" is -118040. Returns `null` if
 * it is not a valid amount.
 */
export function parseSignedAmountToCents(text: string): number | null {
  const trimmed = text.trim()
  const negative = /^[-−]/.test(trimmed)
  const cents = parseMagnitude(negative ? trimmed.slice(1) : trimmed)
  if (cents === null) return null
  return negative && cents !== 0 ? -cents : cents
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
