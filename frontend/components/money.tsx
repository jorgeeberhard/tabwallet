import { cn } from "cn"

import { formatBRL, MINUS } from "@/lib/format"

type MoneyKind = "auto" | "income" | "expense" | "neutral"

export interface MoneyProps {
  /** Amount in reais. The sign picks income/expense when `kind` is `auto`. */
  value: number
  kind?: MoneyKind
  size?: "sm" | "md" | "lg"
  /** Set to `false` for balances and totals: only a negative value shows "−". */
  showSign?: boolean
  className?: string
}

const NBSP = " "

const sizeClass = {
  sm: "text-sm leading-5",
  md: "text-base leading-6",
  lg: "text-[2rem] leading-10 tracking-[-0.01em]",
} as const

const kindClass = {
  income: "text-income",
  expense: "text-expense",
  neutral: "text-foreground",
} as const

function resolveKind(value: number, kind: MoneyKind): keyof typeof kindClass {
  if (kind !== "auto") return kind
  if (value > 0) return "income"
  if (value < 0) return "expense"
  return "neutral"
}

/**
 * A BRL amount (Design System `Money`): tabular figures, with sign and colour
 * taken from its meaning. Colour never carries the meaning alone: income gets
 * "+" and expense gets "−" (U+2212).
 */
export function Money({
  value,
  kind = "auto",
  size = "md",
  showSign = true,
  className,
}: MoneyProps) {
  // Without sign display (balances, totals) only a negative value is marked,
  // and it turns into an expense.
  const resolved = showSign
    ? resolveKind(value, kind)
    : value < 0
      ? "expense"
      : kind === "auto"
        ? "neutral"
        : kind

  let prefix = ""
  if (showSign && resolved === "income") prefix = `+${NBSP}`
  else if (resolved === "expense" || value < 0) prefix = `${MINUS}${NBSP}`

  return (
    <span
      className={cn(
        "font-sans font-semibold whitespace-nowrap tabular-nums",
        sizeClass[size],
        kindClass[resolved],
        className
      )}
    >
      {prefix}
      {formatBRL(value)}
    </span>
  )
}
