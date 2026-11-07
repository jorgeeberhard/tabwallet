import { Money } from "@/components/money"
import { Badge } from "@/components/ui/badge"
import {
  Table,
  TableBody,
  TableCaption,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table"
import type { Granularity } from "@/lib/balance-series-api"
import type { SeriesRow } from "@/lib/balance-series"
import { formatDate, formatMonth } from "@/lib/format"

export interface BalanceTableProps {
  rows: SeriesRow[]
  granularity: Granularity
  today: string
}

/**
 * What tells apart the two rows of the current month in a monthly table: the
 * balance up to today and the projected balance at the end of the month.
 */
function rowNote(row: SeriesRow, granularity: Granularity, today: string): string | null {
  if (granularity === "day") return row.date === today ? "hoje" : null
  if (row.date.slice(0, 7) !== today.slice(0, 7)) return null
  return row.kind === "projected" ? "fim do mês" : "até hoje"
}

/**
 * Every point of the chart as a table: the accessible way to read the same
 * values. Rows after today are marked "Projetado" in words, not only by style.
 */
export function BalanceTable({ rows, granularity, today }: BalanceTableProps) {
  return (
    <div
      className="max-h-96 overflow-y-auto rounded-lg border"
      role="region"
      aria-label="Tabela da evolução do saldo"
      tabIndex={0}
    >
      <Table>
        <TableCaption className="sr-only">
          Saldo no fim de cada {granularity === "day" ? "dia" : "mês"}, do histórico e da projeção.
        </TableCaption>
        <TableHeader className="sticky top-0 bg-card">
          <TableRow>
            <TableHead scope="col">{granularity === "day" ? "Dia" : "Mês"}</TableHead>
            <TableHead scope="col">Tipo</TableHead>
            <TableHead scope="col" className="text-right">
              Saldo
            </TableHead>
          </TableRow>
        </TableHeader>
        <TableBody>
          {rows.map((row) => {
            const projected = row.kind === "projected"
            const value = projected ? row.projected : row.history
            if (value === undefined) return null
            const note = rowNote(row, granularity, today)
            return (
              <TableRow key={row.date}>
                <TableCell>
                  {granularity === "day" ? formatDate(row.date) : formatMonth(row.date)}
                  {note && <span className="text-muted-foreground"> ({note})</span>}
                </TableCell>
                <TableCell>
                  <Badge variant={projected ? "outline" : "secondary"}>
                    {projected ? "Projetado" : "Histórico"}
                  </Badge>
                </TableCell>
                <TableCell className="text-right">
                  <Money value={value} showSign={false} size="sm" />
                </TableCell>
              </TableRow>
            )
          })}
        </TableBody>
      </Table>
    </div>
  )
}
