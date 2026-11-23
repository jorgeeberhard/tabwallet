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
import type { ExpensesByCategory } from "@/lib/reports-api"

/** The share of `total` that `part` is, as "37,5%"; "—" when there is no total. */
export function formatShare(part: number, total: number): string {
  if (total <= 0) return "—"
  const percent = (part / total) * 100
  return `${percent.toLocaleString("pt-BR", { minimumFractionDigits: 0, maximumFractionDigits: 1 })}%`
}

export interface CategoryTableProps {
  report: ExpensesByCategory
  /** Describes the period, for the caption read by assistive technology. */
  periodLabel: string
}

/** Expenses by Category as a table: the accessible way to read the chart, with the share of each. */
export function CategoryTable({ report, periodLabel }: CategoryTableProps) {
  return (
    <div
      className="overflow-x-auto rounded-lg border"
      role="region"
      aria-label="Tabela das despesas por categoria"
      tabIndex={0}
    >
      <Table>
        <TableCaption className="sr-only">
          Despesas por categoria em {periodLabel}, por mês de referência.
        </TableCaption>
        <TableHeader>
          <TableRow>
            <TableHead scope="col">Categoria</TableHead>
            <TableHead scope="col" className="text-right">
              Total
            </TableHead>
            <TableHead scope="col" className="text-right">
              % do total
            </TableHead>
            <TableHead scope="col" className="text-right">
              Projeção
            </TableHead>
          </TableRow>
        </TableHeader>
        <TableBody>
          {report.categories.map((category) => (
            <TableRow key={category.categoryId}>
              <TableCell>
                {category.name}
                {!category.active && (
                  <Badge variant="outline" className="ml-2">
                    Desativada
                  </Badge>
                )}
              </TableCell>
              <TableCell className="text-right">
                <Money value={category.total} kind="neutral" showSign={false} size="sm" />
              </TableCell>
              <TableCell className="text-right tabular-nums">
                {formatShare(category.totalCents, report.totalCents)}
              </TableCell>
              <TableCell className="text-right">
                {category.projectedCents > 0 ? (
                  <Money value={category.projected} kind="neutral" showSign={false} size="sm" />
                ) : (
                  <span className="text-muted-foreground">—</span>
                )}
              </TableCell>
            </TableRow>
          ))}
        </TableBody>
        <TableFooter>
          <TableRow>
            <TableCell className="font-semibold">Total</TableCell>
            <TableCell className="text-right">
              <Money value={report.total} kind="neutral" showSign={false} size="sm" />
            </TableCell>
            <TableCell className="text-right tabular-nums">
              {formatShare(report.totalCents, report.totalCents)}
            </TableCell>
            <TableCell className="text-right">
              {report.projectedCents > 0 ? (
                <Money value={report.projected} kind="neutral" showSign={false} size="sm" />
              ) : (
                <span className="text-muted-foreground">—</span>
              )}
            </TableCell>
          </TableRow>
        </TableFooter>
      </Table>
    </div>
  )
}
