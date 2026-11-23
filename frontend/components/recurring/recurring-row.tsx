"use client"

import {
  ArrowDownLeftIcon,
  ArrowUpRightIcon,
  CircleSlashIcon,
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
import { formatDate, formatMonth } from "@/lib/format"
import type { RecurringTransaction } from "@/lib/recurring-api"

interface RecurringRowProps {
  recurring: RecurringTransaction
  onEdit: (recurring: RecurringTransaction) => void
  onEnd: (recurring: RecurringTransaction) => void
}

/**
 * One Gasto fixo or Receita fixa: description, the day it falls on, Category,
 * the next Occurrence and the amount in force. An ended one is history: it only
 * shows when it ended and offers no actions.
 */
export function RecurringRow({ recurring, onEdit, onEnd }: RecurringRowProps) {
  const isIncome = recurring.type === "income"
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
        <span className="sr-only">{isIncome ? "Receita fixa" : "Gasto fixo"}</span>
      </span>

      <div className="flex min-w-0 flex-1 flex-col gap-1">
        <span className="truncate font-medium">{recurring.description}</span>
        <span className="flex flex-wrap items-center gap-x-2 gap-y-1 text-sm text-muted-foreground">
          <Badge variant="brand">
            <RepeatIcon strokeWidth={1.75} aria-hidden />
            Todo dia {recurring.dayOfMonth}
          </Badge>
          <Badge variant={recurring.category.active ? "secondary" : "outline"}>
            {recurring.category.name}
            {!recurring.category.active && " (desativada)"}
          </Badge>
          {recurring.ended && recurring.endMonth ? (
            <span>Encerrado em {formatMonth(`${recurring.endMonth}-01`)}</span>
          ) : recurring.nextDate ? (
            <span>
              Próxima: <time dateTime={recurring.nextDate}>{formatDate(recurring.nextDate)}</time>
            </span>
          ) : (
            <span>Sem próximas ocorrências</span>
          )}
        </span>
      </div>

      <Money value={recurring.amount} kind={isIncome ? "income" : "expense"} />

      {!recurring.ended && (
        <>
          <div className="hidden items-center sm:flex">
            <Button
              variant="ghost"
              size="icon"
              aria-label={`Editar ${recurring.description}`}
              onClick={() => onEdit(recurring)}
            >
              <PencilIcon strokeWidth={1.75} />
            </Button>
            <Button
              variant="ghost"
              size="icon"
              aria-label={`Encerrar ${recurring.description}`}
              title="Encerrar"
              onClick={() => onEnd(recurring)}
            >
              <CircleSlashIcon strokeWidth={1.75} />
            </Button>
          </div>

          <div className="sm:hidden">
            <DropdownMenu>
              <DropdownMenuTrigger asChild>
                <Button
                  variant="ghost"
                  size="icon"
                  aria-label={`Ações de ${recurring.description}`}
                >
                  <EllipsisIcon strokeWidth={1.75} />
                </Button>
              </DropdownMenuTrigger>
              <DropdownMenuContent align="end">
                <DropdownMenuItem onSelect={() => onEdit(recurring)}>
                  <PencilIcon strokeWidth={1.75} />
                  Editar
                </DropdownMenuItem>
                <DropdownMenuItem variant="destructive" onSelect={() => onEnd(recurring)}>
                  <CircleSlashIcon strokeWidth={1.75} />
                  Encerrar
                </DropdownMenuItem>
              </DropdownMenuContent>
            </DropdownMenu>
          </div>
        </>
      )}
    </li>
  )
}
