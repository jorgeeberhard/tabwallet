import { TriangleAlertIcon } from "lucide-react"

import { Logo } from "@/components/logo"
import { Money } from "@/components/money"
import { ThemeToggle } from "@/components/theme-toggle"
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert"
import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card"
import { Progress } from "@/components/ui/progress"
import { formatBRL } from "@/lib/format"

export default function Home() {
  return (
    <>
      <header className="border-b bg-card">
        <div className="mx-auto flex h-16 w-full max-w-[1200px] items-center justify-between px-4 md:px-6">
          <Logo height={28} />
          <ThemeToggle />
        </div>
      </header>

      <main className="mx-auto flex w-full max-w-[1200px] flex-1 flex-col gap-8 px-4 py-8 md:px-6">
        <section className="flex flex-col gap-2">
          <h1 className="font-display text-3xl font-bold tracking-tight">
            Seu dinheiro, em uma aba.
          </h1>
          <p className="max-w-prose text-muted-foreground">
            Registre receitas, despesas e faturas de cartão, e acompanhe saldo e
            orçamentos.
          </p>
        </section>

        <section aria-labelledby="preview-title" className="flex flex-col gap-4">
          <div>
            <h2
              id="preview-title"
              className="font-display text-xl font-semibold"
            >
              Prévia do tema
            </h2>
            <p className="text-sm text-muted-foreground">
              Valores de exemplo, só para conferir o Design System nos temas
              claro e escuro.
            </p>
          </div>

          <div className="grid gap-6 sm:grid-cols-2 lg:grid-cols-3">
            <Card>
              <CardHeader>
                <CardDescription>Saldo atual</CardDescription>
                <CardTitle>
                  <Money value={4812.37} kind="neutral" size="lg" />
                </CardTitle>
              </CardHeader>
              <CardContent className="flex flex-col gap-1">
                <div className="flex items-center justify-between text-sm">
                  <span className="text-muted-foreground">Receitas</span>
                  <Money value={3200} kind="income" size="sm" />
                </div>
                <div className="flex items-center justify-between text-sm">
                  <span className="text-muted-foreground">Despesas</span>
                  <Money value={1180.4} kind="expense" size="sm" />
                </div>
              </CardContent>
            </Card>

            <Card>
              <CardHeader>
                <CardTitle>Orçamento de Lazer</CardTitle>
                <CardDescription>setembro de 2026</CardDescription>
              </CardHeader>
              <CardContent className="flex flex-col gap-3">
                <Progress
                  value={85}
                  indicatorClassName="bg-warning"
                  aria-label="85% do limite de Lazer"
                />
                <p className="text-sm text-muted-foreground">
                  85% do limite
                </p>
                <Alert variant="warning">
                  <TriangleAlertIcon strokeWidth={1.75} />
                  <AlertTitle>Lazer perto do limite</AlertTitle>
                  <AlertDescription>
                    Faltam {formatBRL(90)} para o limite do mês.
                  </AlertDescription>
                </Alert>
              </CardContent>
            </Card>

            <Card>
              <CardHeader>
                <CardTitle>Fatura</CardTitle>
                <CardDescription>Cartão principal</CardDescription>
              </CardHeader>
              <CardContent className="flex flex-col gap-4">
                <div className="flex flex-wrap gap-2">
                  <Badge variant="brand">Aberta</Badge>
                  <Badge variant="warning">Fechada</Badge>
                  <Badge variant="income">Paga</Badge>
                  <Badge variant="expense">Vencida</Badge>
                  <Badge variant="secondary">Alimentação</Badge>
                </div>
                <div className="flex flex-wrap gap-2">
                  <Button>Adiantar pagamento</Button>
                  <Button variant="outline">Ver fatura</Button>
                  <Button variant="ghost">Cancelar</Button>
                </div>
              </CardContent>
            </Card>
          </div>
        </section>
      </main>
    </>
  )
}
