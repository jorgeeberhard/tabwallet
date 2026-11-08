"use client"

import {
  ArrowDownLeftIcon,
  ArrowUpRightIcon,
  EllipsisIcon,
  PencilIcon,
  Trash2Icon,
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
import type { Transaction } from "@/lib/transactions-api"

interface TransactionRowProps {
  transaction: Transaction
  onEdit: (transaction: Transaction) => void
  onDelete: (transaction: Transaction) => void
}

/**
 * One line of the statement (Design System `TransactionRow`): direction icon,
 * description, category and date, amount and row actions. A Projection (a
 * date after today) is marked with a badge, so it is not mistaken for money
 * that already moved.
 */
export function TransactionRow({ transaction, onEdit, onDelete }: TransactionRowProps) {
  const isIncome = transaction.type === "income"
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
        <span className="sr-only">{isIncome ? "Receita" : "Despesa"}</span>
      </span>

      <div className="flex min-w-0 flex-1 flex-col gap-1">
        <span className="truncate font-medium">{transaction.description}</span>
        <span className="flex flex-wrap items-center gap-x-2 gap-y-1 text-sm text-muted-foreground">
          <time dateTime={transaction.date}>{formatDate(transaction.date)}</time>
          <Badge variant={transaction.category.active ? "secondary" : "outline"}>
            {transaction.category.name}
            {!transaction.category.active && " (desativada)"}
          </Badge>
          {transaction.isProjection && <Badge variant="warning">Projeção</Badge>}
        </span>
      </div>

      <Money value={transaction.amount} kind={isIncome ? "income" : "expense"} />

      <div className="hidden items-center sm:flex">
        <Button
          variant="ghost"
          size="icon"
          aria-label={`Editar ${transaction.description}`}
          onClick={() => onEdit(transaction)}
        >
          <PencilIcon strokeWidth={1.75} />
        </Button>
        <Button
          variant="ghost"
          size="icon"
          aria-label={`Excluir ${transaction.description}`}
          onClick={() => onDelete(transaction)}
        >
          <Trash2Icon strokeWidth={1.75} />
        </Button>
      </div>

      <div className="sm:hidden">
        <DropdownMenu>
          <DropdownMenuTrigger asChild>
            <Button
              variant="ghost"
              size="icon"
              aria-label={`Ações de ${transaction.description}`}
            >
              <EllipsisIcon strokeWidth={1.75} />
            </Button>
          </DropdownMenuTrigger>
          <DropdownMenuContent align="end">
            <DropdownMenuItem onSelect={() => onEdit(transaction)}>
              <PencilIcon strokeWidth={1.75} />
              Editar
            </DropdownMenuItem>
            <DropdownMenuItem variant="destructive" onSelect={() => onDelete(transaction)}>
              <Trash2Icon strokeWidth={1.75} />
              Excluir
            </DropdownMenuItem>
          </DropdownMenuContent>
        </DropdownMenu>
      </div>
    </li>
  )
}
