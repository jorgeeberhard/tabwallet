# TabWallet com shadcn/ui

O front-end usa [shadcn/ui](https://ui.shadcn.com) (Radix + Tailwind CSS) como base de componentes, com o tema do Design System TabWallet. Decisão registrada em [ADR-0001](../../adr/0001-shadcn-ui-no-front-end.md).

O shadcn copia o código dos componentes para `front-end/components/ui/`. Esses arquivos são nossos: ajuste-os ao Design System em vez de sobrescrever estilos de fora.

## 1. Instalação

Na pasta `front-end/` (Next.js com App Router, TypeScript e Tailwind CSS v4):

```bash
npx shadcn@latest init
npx shadcn@latest add button input label form select calendar popover toggle-group \
  card badge table dialog alert-dialog alert progress sonner sheet dropdown-menu \
  skeleton separator tooltip chart
npm install next-themes lucide-react
```

Formulários usam o `form` do shadcn com `react-hook-form` + `zod`. Nas versões recentes do shadcn também existem `field`, `input-group`, `empty` e `spinner`. Se estiverem disponíveis, prefira-os para `TextField`, `MoneyInput`, `EmptyState` e o estado de carregamento do botão.

## 2. Tema

1. Substitua o conteúdo de `front-end/app/globals.css` por [`shadcn/globals.css`](shadcn/globals.css). Ele contém todas as variáveis do shadcn nos temas claro (`:root`) e escuro (`.dark`), mais as cores próprias do TabWallet (`income`, `expense-soft`, `warning`…), já expostas como utilitários do Tailwind (`bg-income-soft`, `text-warning`).
2. No `components.json` do shadcn, mantenha `"cssVariables": true` e `"iconLibrary": "lucide"`.
3. O `tokens.css` deste Design System **não** é importado no app: o `globals.css` o substitui. O `tokens.json` continua sendo a fonte de verdade. Se um valor mudar, atualize os dois.

### Mapeamento

| Variável shadcn | Token TabWallet | Claro | Escuro |
| --- | --- | --- | --- |
| `--background` | `bg-canvas` | `#f4f5f8` | `#161a23` |
| `--foreground` | `text-primary` | `#161a23` | `#f4f5f8` |
| `--card`, `--popover` | `bg-surface` | `#ffffff` | `#1f2330` |
| `--primary` | `brand-primary` | `#4b3fd6` | `#8b83f0` |
| `--primary-foreground` | `on-brand` | `#ffffff` | `#161a23` |
| `--secondary`, `--muted`, `--accent` | `bg-subtle` | `#eceef3` | `#272c3a` |
| `--muted-foreground` | `text-secondary` | `#5b6475` | `#a9b0bf` |
| `--destructive` | `expense` | `#c22a42` | `#ff6b7f` |
| `--destructive-foreground` | `on-expense` | `#ffffff` | `#161a23` |
| `--border` | `border` | `#dcdfe6` | `#343a4a` |
| `--input` | `border-strong` | `#808a9c` | `#737d92` |
| `--ring` | `focus-ring` | `#4b3fd6` | `#8b83f0` |
| `--chart-1…5` | marca, receitas, despesas, alerta, neutro | | |
| `--radius` | `radius-md` | `10px` | |

Raios derivados: `rounded-sm` 6px (campos), `rounded-lg` 10px (botões, alertas), `rounded-xl` 16px (cards, diálogos), `rounded-full` (badges, barras).

## 3. Fontes e tema escuro

Copie os `.woff2` de [`fonts/`](fonts) para `front-end/app/fonts/` e carregue-os com `next/font/local`. Use `next-themes` com `attribute="class"`: o tema escuro é a classe `.dark`, não mais `data-theme`.

```tsx
// front-end/app/layout.tsx
import localFont from "next/font/local";
import { ThemeProvider } from "next-themes";
import { Toaster } from "@/components/ui/sonner";
import "./globals.css";

const inter = localFont({ src: "./fonts/inter-latin-wght-normal.woff2", variable: "--font-inter", weight: "100 900", display: "swap" });
const manrope = localFont({ src: "./fonts/manrope-latin-wght-normal.woff2", variable: "--font-manrope", weight: "200 800", display: "swap" });
const mono = localFont({ src: "./fonts/jetbrains-mono-latin-wght-normal.woff2", variable: "--font-jetbrains-mono", weight: "100 800", display: "swap" });

export default function RootLayout({ children }: { children: React.ReactNode }) {
  return (
    <html lang="pt-BR" suppressHydrationWarning className={`${inter.variable} ${manrope.variable} ${mono.variable}`}>
      <body>
        <ThemeProvider attribute="class" defaultTheme="system" enableSystem disableTransitionOnChange>
          {children}
          <Toaster />
        </ThemeProvider>
      </body>
    </html>
  );
}
```

## 4. Ajustes nos componentes gerados

O tema sozinho não basta: alguns padrões do shadcn contrariam o Design System. Faça estes ajustes uma vez, logo depois do `add`.

| Arquivo | Ajuste | Motivo |
| --- | --- | --- |
| `ui/button.tsx` | `default`: `hover:bg-primary-hover` no lugar de `hover:bg-primary/90`. `destructive`: `text-destructive-foreground` no lugar de `text-white`, e remova `dark:bg-destructive/60`. Alturas: `sm` `h-8`, `default` `h-10`, `lg` `h-12`. Rótulo `font-semibold`. | Hover definido pela marca. Texto branco sobre despesa no escuro fica abaixo de AA. Alturas `control-sm/md/lg`. |
| Todos (`button`, `input`, `select`, `toggle-group`, `checkbox`…) | Troque `focus-visible:ring-ring/50 focus-visible:ring-[3px]` por `focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 focus-visible:ring-offset-background`. | O anel de foco precisa ser sólido (3:1) em todas as superfícies. |
| `ui/input.tsx`, `ui/select.tsx` | Altura `h-10`, `bg-card`, `rounded-sm`. Remova `dark:bg-input/30`. `placeholder:text-muted-foreground`. | Campos em `bg-surface` nos dois temas, contorno `border-strong`. |
| `ui/card.tsx` | `rounded-xl` e `shadow-sm` mantidos. Padding `p-6` (`gap-6` entre partes). Título com `font-display text-xl font-semibold`. | `radius-lg` 16px e `space-6`. |
| `ui/dialog.tsx`, `ui/alert-dialog.tsx` | Conteúdo `rounded-xl`, `max-w-[440px]`. | Mesmo raio dos cards. Largura da diretriz de Dialog. |
| `ui/badge.tsx` | `rounded-full`. Novas variantes: `brand` (`bg-brand-soft text-primary`), `income` (`bg-income-soft text-income`), `expense` (`bg-expense-soft text-expense`), `warning` (`bg-warning-soft text-warning`). O neutro é `secondary` com `text-muted-foreground`. | Status de fatura e badges de categoria. |
| `ui/alert.tsx` | Novas variantes `info` (`bg-brand-soft`), `success` (`bg-income-soft`), `warning` (`bg-warning-soft`), e `destructive` com `bg-expense-soft`. Ícone na cor cheia, texto em `foreground`. | Alertas de orçamento (UC13). |
| `ui/progress.tsx` | Aceite `indicatorClassName` para pintar o indicador (`bg-income`, `bg-warning`, `bg-expense`). Altura `h-2`. | Base do `BudgetBar`. |
| `ui/chart.tsx` | Use `var(--chart-1…5)` no `ChartConfig`. Sempre com legenda escrita. | O gráfico do shadcn usa Recharts, já previsto no termo de abertura. |

## 5. Componentes do Design System → shadcn

| TabWallet | Base shadcn | Observação |
| --- | --- | --- |
| `Button` | `button` | `primary` vira `default`, `secondary` vira `outline`, `ghost` continua `ghost`, `danger` vira `destructive` |
| `TextField` | `form` (ou `field`) + `label` + `input` | Erros pelo `FormMessage`, validação com `zod` |
| `MoneyInput` | `input-group` (ou `input` com prefixo) | Prefixo "R$", `inputMode="decimal"`, `tabular-nums` |
| `Select` | `select` | Listas longas de categorias: `popover` + `command` (combobox) |
| `SegmentedControl` | `toggle-group` `type="single"` | Use `tabs` quando a troca mostra outro painel |
| `Badge` | `badge` | Variantes novas da seção 4 |
| `Card` | `card` | `CardHeader`, `CardTitle`, `CardDescription`, `CardAction`, `CardContent` |
| `Dialog` | `dialog`, `alert-dialog` | Exclusões (UC07) sempre em `alert-dialog` |
| `Alert` | `alert` + `sonner` | Inline: `alert`. Confirmação passageira ("Transação salva."): `toast` do `sonner` |
| `EmptyState` | `empty` (ou composição própria) | |
| `Icon` | `lucide-react` | `strokeWidth={1.75}` |
| `Money` | próprio | `formatBRL()` + `span` com `tabular-nums` |
| `TransactionRow` | próprio, com `badge`, `button`, `dropdown-menu` | No celular, as ações vão para um `dropdown-menu` |
| `BalanceCard` | próprio, sobre `card` | |
| `BudgetBar` | próprio, sobre `progress` | Estado sempre escrito, nunca só cor |
| `InvoiceCard` | próprio, sobre `card` + `badge` + `button` + `dialog` | |
| `AppHeader` | próprio, com `sheet` (menu no celular) e `dropdown-menu` (conta) | |
| `Logo` | próprio | SVG de [`logos/`](logos) com `fill` nas variáveis `primary`, `logo-tab`, `logo-clasp` |
| — | `calendar` + `popover` | Data da transação |
| — | `skeleton` | Carregamento de listas e cards (TanStack Query) |

Os componentes próprios ficam em `front-end/components/` (fora de `ui/`), com os nomes e props de [`components/index.d.ts`](components/index.d.ts). As diretrizes de uso de cada um continuam em [`components/`](components).

## 6. Regras que o shadcn não garante

- Uma ação `default` (primária) por tela ou diálogo.
- `income` e `expense` nunca falam sozinhas: "+" e "−" (U+2212), seta, ou texto.
- Valores sempre com `tabular-nums` e `formatBRL()`.
- Textos em pt-BR, conforme a seção de voz e tom do [README](README.md).
- Não use cores do Tailwind (`bg-blue-500`, `text-red-600`) nem hex nas telas. Use só as variáveis do tema.
