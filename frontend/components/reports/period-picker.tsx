"use client"

import { Field, FieldLabel } from "@/components/ui/field"
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select"
import {
  CUSTOM_PERIOD_ID,
  MAX_REPORT_MONTHS,
  PERIOD_PRESETS,
  type PeriodChoice,
  type PeriodRange,
  monthOptions,
  periodProblem,
} from "@/lib/report-periods"

export interface PeriodPickerProps {
  /** Unique within the page; prefixes the ids of the fields. */
  idPrefix: string
  label: string
  choice: PeriodChoice
  /** The months the choice covers now. */
  range: PeriodRange
  /** The current month (`YYYY-MM`), for the list of months of a custom period. */
  currentMonth: string
  onChange: (choice: PeriodChoice) => void
}

/**
 * Picks the months a report covers: a preset ("Este mês", "Últimos 6
 * meses"...) or, with "Personalizado", the first and the last month. A period
 * the API would refuse (upside down, or longer than three years) is explained
 * here, and the screen does not ask for it.
 */
export function PeriodPicker({
  idPrefix,
  label,
  choice,
  range,
  currentMonth,
  onChange,
}: PeriodPickerProps) {
  const custom = choice.presetId === CUSTOM_PERIOD_ID
  const options = monthOptions(currentMonth)
  const problem = custom ? periodProblem(range) : null

  function pickPreset(presetId: string) {
    // Starting a custom period from the months the last choice covered.
    onChange(
      presetId === CUSTOM_PERIOD_ID
        ? { presetId, from: range.from, to: range.to }
        : { presetId },
    )
  }

  return (
    <div className="flex flex-col gap-3">
      <div className="grid gap-4 sm:grid-cols-3">
        <Field>
          <FieldLabel htmlFor={`${idPrefix}-period`}>{label}</FieldLabel>
          <Select value={choice.presetId} onValueChange={pickPreset}>
            <SelectTrigger id={`${idPrefix}-period`} className="w-full">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              {PERIOD_PRESETS.map((preset) => (
                <SelectItem key={preset.id} value={preset.id}>
                  {preset.label}
                </SelectItem>
              ))}
              <SelectItem value={CUSTOM_PERIOD_ID}>Personalizado</SelectItem>
            </SelectContent>
          </Select>
        </Field>
        {custom && (
          <>
            <Field>
              <FieldLabel htmlFor={`${idPrefix}-from`}>Do mês</FieldLabel>
              <Select
                value={range.from}
                onValueChange={(from) => onChange({ ...choice, from, to: range.to })}
              >
                <SelectTrigger id={`${idPrefix}-from`} className="w-full">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  {options.map((option) => (
                    <SelectItem key={option.value} value={option.value}>
                      {option.label}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </Field>
            <Field>
              <FieldLabel htmlFor={`${idPrefix}-to`}>Até o mês</FieldLabel>
              <Select
                value={range.to}
                onValueChange={(to) => onChange({ ...choice, from: range.from, to })}
              >
                <SelectTrigger id={`${idPrefix}-to`} className="w-full">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  {options.map((option) => (
                    <SelectItem key={option.value} value={option.value}>
                      {option.label}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </Field>
          </>
        )}
      </div>
      {problem && (
        <p role="alert" className="text-sm text-destructive">
          {problem === "order"
            ? "O mês final precisa ser igual ou depois do mês inicial."
            : `O período pode ter no máximo ${MAX_REPORT_MONTHS} meses.`}
        </p>
      )}
    </div>
  )
}
