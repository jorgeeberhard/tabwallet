import { Money } from "@/components/money"
import { Badge } from "@/components/ui/badge"
import {
  Table,
  TableBody,
  TableCaption,
  TableCell,
  TableFooter,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table"
import { formatMonthOfYear } from "@/lib/format"
import { centsToReais } from "@/lib/amount"
import type { IncomeVsExpenses } from "@/lib/reports-api"

export interface IncomeExpenseTableProps {
  report: IncomeVsExpenses
}

/** Income x expenses per month as a table: the accessible way to read the chart. */
export function IncomeExpenseTable({ report }: IncomeExpenseTableProps) {
  return (
    <div
      className="max-h-96 overflow-auto rounded-lg border"
      role="region"
      aria-label="Tabela de receitas e despesas por mês"
      tabIndex={0}
    >
      <Table>
        <TableCaption className="sr-only">
          Receitas e despesas de cada mês de referência, com o resultado do mês.
        </TableCaption>
        <TableHeader className="sticky top-0 bg-card">
          <TableRow>
            <TableHead scope="col">Mês</TableHead>
            <TableHead scope="col" className="text-right">
              Receitas
            </TableHead>
            <TableHead scope="col" className="text-right">
              Despesas
            </TableHead>
            <TableHead scope="col" className="text-right">
              Resultado
            </TableHead>
          </TableRow>
        </TableHeader>
        <TableBody>
          {report.months.map((month) => (
            <TableRow key={month.month}>
              <TableCell>
                {formatMonthOfYear(month.month)}
                {(month.projectedIncomeCents > 0 || month.projectedExpenseCents > 0) && (
                  <Badge variant="outline" className="ml-2">
                    Com projeções
                  </Badge>
                )}
              </TableCell>
              <TableCell className="text-right">
                <Money value={month.income} kind="income" showSign={false} size="sm" />
              </TableCell>
              <TableCell className="text-right">
                <Money value={month.expense} kind="expense" showSign={false} size="sm" />
              </TableCell>
              <TableCell className="text-right">
                <Money value={month.balance} showSign={false} size="sm" />
              </TableCell>
            </TableRow>
          ))}
        </TableBody>
        <TableFooter>
          <TableRow>
            <TableCell className="font-semibold">Total do período</TableCell>
            <TableCell className="text-right">
              <Money value={centsToReais(report.incomeCents)} kind="income" showSign={false} size="sm" />
            </TableCell>
            <TableCell className="text-right">
              <Money value={centsToReais(report.expenseCents)} kind="expense" showSign={false} size="sm" />
            </TableCell>
            <TableCell className="text-right">
              <Money
                value={centsToReais(report.incomeCents - report.expenseCents)}
                showSign={false}
                size="sm"
              />
            </TableCell>
          </TableRow>
        </TableFooter>
      </Table>
    </div>
  )
}
