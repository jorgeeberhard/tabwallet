# TabWallet — Manual de Identidade Visual

Versão 1.0 · Setembro de 2026 · Jorge Miguel Eberhard da Conceição

Versão em texto do manual de marca (o original é uma apresentação de 16 slides; o PDF exportado fica nesta pasta como `brand-manual.pdf`). As regras aplicadas à interface e os tokens estão em [`../design-system`](../design-system/README.md).

## 1. Apresentação

TabWallet é uma aplicação web gratuita para quem quer enxergar, em um só lugar, para onde vai o próprio dinheiro — sem integração bancária e sem assinatura.

## 2. Conceito

- **Nome:** une *tab*, a aba do navegador onde o produto vive (sem instalação, sem app nativo), e *keep a tab on*, expressão inglesa para "acompanhar de perto".
- **Símbolo:** uma carteira cuja parte superior forma uma aba.
  - **A aba** — o produto vive no navegador.
  - **A carteira** — o dinheiro do dia a dia, receitas e despesas em um só lugar.
  - **O fecho** — controle e segurança.
- **Slogan:** *Seu dinheiro, em uma aba.*
- **Palavras-chave:** CLAREZA · CONTROLE · AUTONOMIA.

## 3. Logo

### Versões

| Versão | Uso | Arquivo |
| --- | --- | --- |
| Principal (horizontal) | Fundos claros: cabeçalho, login, e-mails, documentação | `tabwallet-logo.svg` |
| Símbolo isolado | Favicon, ícone do app (PWA), avatar | `tabwallet-mark.svg` |
| Negativo | Fundos escuros e modo escuro | `tabwallet-logo-negative.svg` |
| Monocromático | Branco sobre Índigo ou Grafite sobre branco | `tabwallet-logo-white.svg`, `tabwallet-logo-mono.svg` |

Não há versão vertical: por ser um produto apenas web, o símbolo isolado cobre os espaços quadrados. Os arquivos estão em [`../design-system/logos`](../design-system/logos).

### Construção

- Símbolo em Índigo (corpo) e Lavanda (aba e fecho).
- Logotipo em Manrope ExtraBold, "Tab" em Índigo e "Wallet" em Grafite, espaçamento entre letras −3%.

### Área de segurança e tamanho mínimo

- Área livre **x = metade da altura do símbolo**, nos quatro lados (também no símbolo isolado).
- Assinatura horizontal: mínimo **160 px** de largura. Abaixo disso, use apenas o símbolo.
- Símbolo: mínimo **16 px**.

### Usos incorretos

Não alterar as cores · não distorcer nem inclinar · não rotacionar · não aplicar contorno · não aplicar sombras ou efeitos · não reorganizar nem redimensionar os elementos.

## 4. Tipografia

| Família | Papel | Pesos |
| --- | --- | --- |
| **Manrope** | Títulos, valores de destaque e logotipo | 600, 700, 800 |
| **Inter** | Interface, textos e tabelas, com algarismos tabulares para alinhar valores | 400, 500, 600 |
| **JetBrains Mono** | Apoio técnico: código e tokens | 400 |

Todas gratuitas, licença SIL Open Font License 1.1 (uso comercial permitido).

### Escala para a interface

| Estilo | Fonte | Peso | Tamanho / linha (px) | Uso |
| --- | --- | --- | --- | --- |
| Display | Manrope | 800 | 40 / 48 | Saldo principal, boas-vindas |
| Título 1 | Manrope | 700 | 32 / 40 | Título de página |
| Título 2 | Manrope | 700 | 24 / 32 | Seções e cards |
| Título 3 | Manrope | 600 | 20 / 28 | Subtítulos, cabeçalho de modal |
| Corpo | Inter | 400 | 16 / 24 | Texto padrão, formulários |
| Corpo pequeno | Inter | 400 | 14 / 20 | Tabelas, textos de apoio |
| Legenda | Inter | 500 | 12 / 16 | Rótulos, datas, metadados |
| Valor | Inter | 600 tabular | 14 a 40 | Valores em R$, sempre tabular |

No código, em rem (1 rem = 16 px). Nenhum texto abaixo de 12 px.

## 5. Cores

### Paleta principal

| Nome | HEX | RGB | Papel |
| --- | --- | --- | --- |
| Índigo | `#4B3FD6` | 75 63 214 | Primária: botões, links e marca |
| Lavanda | `#8B83F0` | 139 131 240 | Apoio e primária no modo escuro |
| Grafite | `#161A23` | 22 26 35 | Texto principal e fundo escuro |
| Ardósia | `#5B6475` | 91 100 117 | Texto secundário e ícones |
| Névoa | `#F4F5F8` | 244 245 248 | Fundo da aplicação |
| Branco | `#FFFFFF` | 255 255 255 | Superfícies: cards e modais |

Proporção sugerida: 60% Névoa e Branco, 30% Grafite e Ardósia, 10% Índigo. Produto 100% digital: cores definidas em HEX e RGB.

### Cores semânticas

| Cor | Modo claro | Modo escuro | Quando usar |
| --- | --- | --- | --- |
| Receita | `#0A7A52` | `#3DD598` | Entradas, saldo positivo, orçamento dentro do limite |
| Despesa | `#C22A42` | `#FF6B7F` | Saídas, saldo negativo, erros e exclusões |
| Alerta | `#9A5B00` | `#F5B544` | Orçamento perto ou acima do limite (UC13) |

**A cor nunca fala sozinha:** receitas levam "+", despesas levam "−" e alertas trazem texto. Todos os pares de texto e fundo passam WCAG AA (≥ 4,5:1).

## 6. Forma

- **Grade de 4 px:** espaçamentos 4, 8, 12, 16, 24, 32, 48 e 64 px.
- **Cantos:** 6 px em campos e tags, 10 px em botões, 16 px em cards e modais, pílula em chips.
- **Ícones:** Lucide (código aberto), contorno de 1,75 px, em 16, 20 ou 24 px, na cor do texto.

## 7. Voz e tom

- **Direto:** frases curtas; botões começam com verbo ("Salvar transação").
- **Calmo:** informa sem alarme e sem julgar os gastos de ninguém.
- **Próximo:** trata por "você", em português do Brasil, sem jargão financeiro.

| Prefira | Evite |
| --- | --- |
| Você passou R$ 120,00 do limite de Alimentação. | ATENÇÃO!!! Você gastou demais! |
| Transação salva. | Operação realizada com sucesso! |
| Não foi possível salvar. Verifique a conexão e tente de novo. | Erro 500: falha na requisição. |
| Nenhuma transação neste período. | Sem dados. |

Formatos: `R$ 1.234,56` · `− R$ 45,90` (sinal de subtração U+2212) · `30/09/2026` · `setembro de 2026`.

## 8. Aplicação na interface

A marca aparece com moderação: o Índigo fica reservado às ações e à identidade, os neutros organizam a informação e as cores semânticas dão sentido aos valores. A aba do navegador — favicon e título — é o primeiro ponto de contato.

## 9. Do manual ao Design System

Os tokens foram nomeados em inglês, como o código do projeto (Termo de Abertura, Anexo I, seção 4), com modo escuro previsto desde o início. A lista completa e as regras de uso estão em [`../design-system`](../design-system/README.md).
