"use client"

import {
  ArrowDownLeftIcon,
  ArrowUpRightIcon,
  CalendarXIcon,
  EllipsisIcon,
  PencilIcon,
  RepeatIcon,
} from "lucide-react"

import { Money } from "@/components/money"
import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu"
import { formatDate } from "@/lib/format"
import type { Occurrence } from "@/lib/recurring-api"

interface OccurrenceRowProps {
  occurrence: Occurrence
  onEditAmount: (occurrence: Occurrence) => void
  onSkip: (occurrence: Occurrence) => void
}

/**
 * One Occurrence of a Gasto fixo or Receita fixa in the statement (Design
 * System `TransactionRow`, with the `repeat` icon for fixed items). A future
 * one is marked "Projeção". The actions act on this month only: the other
 * Occurrences are not affected.
 */
export function OccurrenceRow({ occurrence, onEditAmount, onSkip }: OccurrenceRowProps) {
  const isIncome = occurrence.type === "income"
  const Icon = isIncome ? ArrowDownLeftIcon : ArrowUpRightIcon

  return (
    <li className="flex items-center gap-3 px-4 py-3">
      <span
        className={
          isIncome
            ? "flex size-9 shrink-0 items-center justify-center rounded-full bg-income-soft text-income"
            : "flex size-9 shrink-0 items-center justify-center rounded-full bg-expense-soft text-expense"
        }
      >
        <Icon strokeWidth={1.75} className="size-4" aria-hidden />
      </span>

      <div className="flex min-w-0 flex-1 flex-col gap-1">
        <span className="truncate font-medium">{occurrence.description}</span>
        <span className="flex flex-wrap items-center gap-x-2 gap-y-1 text-sm text-muted-foreground">
          <time dateTime={occurrence.date}>{formatDate(occurrence.date)}</time>
          <Badge variant={occurrence.category.active ? "secondary" : "outline"}>
            {occurrence.category.name}
            {!occurrence.category.active && " (desativada)"}
          </Badge>
          <Badge variant="brand">
            <RepeatIcon strokeWidth={1.75} aria-hidden />
            {isIncome ? "Receita fixa" : "Gasto fixo"}
          </Badge>
          {occurrence.isProjection && <Badge variant="warning">Projeção</Badge>}
          {occurrence.isEdited && <Badge variant="outline">Valor editado</Badge>}
        </span>
      </div>

      <Money value={occurrence.amount} kind={isIncome ? "income" : "expense"} />

      <div className="hidden items-center sm:flex">
        <Button
          variant="ghost"
          size="icon"
          aria-label={`Editar valor de ${occurrence.description} só deste mês`}
          title="Editar valor só deste mês"
          onClick={() => onEditAmount(occurrence)}
        >
          <PencilIcon strokeWidth={1.75} />
        </Button>
        <Button
          variant="ghost"
          size="icon"
          aria-label={`Excluir ${occurrence.description} só deste mês`}
          title="Excluir só este mês"
          onClick={() => onSkip(occurrence)}
        >
          <CalendarXIcon strokeWidth={1.75} />
        </Button>
      </div>

      <div className="sm:hidden">
        <DropdownMenu>
          <DropdownMenuTrigger asChild>
            <Button
              variant="ghost"
              size="icon"
              aria-label={`Ações de ${occurrence.description}`}
            >
              <EllipsisIcon strokeWidth={1.75} />
            </Button>
          </DropdownMenuTrigger>
          <DropdownMenuContent align="end">
            <DropdownMenuItem onSelect={() => onEditAmount(occurrence)}>
              <PencilIcon strokeWidth={1.75} />
              Editar valor só deste mês
            </DropdownMenuItem>
            <DropdownMenuItem variant="destructive" onSelect={() => onSkip(occurrence)}>
              <CalendarXIcon strokeWidth={1.75} />
              Excluir só este mês
            </DropdownMenuItem>
          </DropdownMenuContent>
        </DropdownMenu>
      </div>
    </li>
  )
}
