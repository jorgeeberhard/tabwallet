"use client"

import * as React from "react"
import { cn } from "cn"

import { InputGroup, InputGroupAddon, InputGroupInput } from "@/components/ui/input-group"
import { formatAmountInput, parseAmountToCents, parseSignedAmountToCents } from "@/lib/amount"

export interface MoneyInputProps
  extends Omit<React.ComponentProps<"input">, "value" | "onChange" | "type"> {
  /** What the person typed ("1.180,40"). */
  value: string
  onChange: (value: string) => void
  /**
   * For balances: accepts zero and a leading "-". Phones then get the text
   * keyboard, because the numeric one has no minus key.
   */
  signed?: boolean
}

/**
 * An amount in reais (Design System `MoneyInput`): fixed "R$" prefix, tabular
 * figures and the numeric keyboard on phones. Amounts are always positive: the
 * Transaction type, not a minus sign, gives the direction. On blur a valid
 * amount is rewritten as "1.180,40". With `signed` it also takes zero and a
 * negative amount ("-1.180,40"), for balances.
 */
export function MoneyInput({
  value,
  onChange,
  onBlur,
  className,
  signed = false,
  ...props
}: MoneyInputProps) {
  return (
    <InputGroup>
      <InputGroupAddon>R$</InputGroupAddon>
      <InputGroupInput
        inputMode={signed ? "text" : "decimal"}
        autoComplete="off"
        placeholder="0,00"
        className={cn("font-semibold tabular-nums", className)}
        value={value}
        onChange={(event) => onChange(event.target.value)}
        onBlur={(event) => {
          const cents = signed
            ? parseSignedAmountToCents(event.target.value)
            : parseAmountToCents(event.target.value)
          if (cents !== null) onChange(formatAmountInput(cents))
          onBlur?.(event)
        }}
        {...props}
      />
    </InputGroup>
  )
}
