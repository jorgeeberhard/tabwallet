# TabWallet

Sistema web de controle financeiro pessoal em que o próprio usuário registra receitas, despesas e compras no cartão de crédito, sem integração bancária. Cada termo está em português; o nome usado no código (em inglês) aparece entre crases.

## Language

### Transações

**Transação** (`Transaction`):
Registro de uma Receita ou de uma Despesa do usuário, com valor, data, descrição e Categoria. O valor é sempre maior que zero, em reais; o tipo dá o sinal.
_Avoid_: gasto, movimentação, lançamento

**Receita** (`Income`):
Tipo de Transação que representa dinheiro recebido. Estornos recebidos da loja são lançados como Receita: o sistema não tem conceito de estorno.
_Avoid_: entrada, ganho

**Despesa** (`Expense`):
Tipo de Transação que representa dinheiro gasto.
_Avoid_: gasto, saída

**Transação fixa** (`RecurringTransaction`):
Transação que se repete todo mês, definida por um valor, um tipo (Receita ou Despesa) e o dia do mês em que ocorre. Em meses mais curtos que o dia definido, vale o último dia do mês. Pode ser encerrada: as Projeções futuras somem e o histórico fica. Debita ou credita direto no Saldo atual, nunca numa Fatura.
_Avoid_: transação recorrente, assinatura

**Gasto fixo** (`FixedExpense`):
Transação fixa do tipo Despesa (ex.: aluguel).

**Receita fixa** (`FixedIncome`):
Transação fixa do tipo Receita (ex.: salário).

**Ocorrência** (`Occurrence`):
A Transação de um mês específico gerada por uma Transação fixa. Mudar o valor da Transação fixa vale a partir do mês da alteração; meses passados não mudam. O usuário pode excluir ou editar o valor de uma única Ocorrência sem afetar as futuras.

**Projeção** (`Projection`):
Transação com data futura, inclusive as Ocorrências futuras de Gastos e Receitas fixos. Aparece nas listagens, mas não entra no Saldo atual até a data chegar.
_Avoid_: transação pendente, agendada

**Mês de referência** (`ReferenceMonth`):
Mês em que uma Transação conta nos relatórios e orçamentos. Para Despesas fora do cartão é o mês da própria data; para Despesas vinculadas a uma Fatura (Compras no cartão, Parcelas e Encargos) é o mês do Dia de vencimento da Fatura.
_Avoid_: mês de competência

### Categorias

**Categoria** (`Category`):
Classificação de Transações que pertence a exatamente um tipo (Receita ou Despesa). Uma Transação só aceita Categoria do mesmo tipo.
_Avoid_: tag, rótulo

**Categoria padrão** (`DefaultCategory`):
Categoria criada automaticamente para todo usuário novo; pode ser renomeada e desativada como qualquer outra. A única exceção é "Juros e encargos", que não pode ser desativada enquanto houver Encargos lançados.

**Categoria desativada** (`InactiveCategory`):
Categoria removida pelo usuário. Continua associada às Transações existentes, mas não pode ser escolhida em novos lançamentos. Pode ser reativada. O nome é único por tipo entre ativas e desativadas. O Orçamento dela fica guardado, sem gerar Alerta, e volta a valer na reativação.
_Avoid_: excluída, apagada, removida

### Orçamento

**Orçamento** (`Budget`):
Limite de gasto de uma Categoria de Despesa que vale todo mês até o usuário alterá-lo. O gasto da categoria é a soma das Despesas do Mês de referência, incluindo as Projeções.
_Avoid_: meta, teto

**Alerta de orçamento** (`BudgetAlert`):
Aviso exibido dentro do sistema quando o gasto de uma Categoria passa do Orçamento.
_Avoid_: notificação

### Saldo

**Saldo atual** (`CurrentBalance`):
Soma das Receitas menos as Despesas não vinculadas a uma Fatura e menos os Pagamentos de fatura, todos com data até hoje, mais o Saldo inicial. Pode ficar negativo. Existe um único saldo por usuário: não há múltiplas contas ou carteiras. A data de uma Transação é só o dia, sem horário, e "hoje" é sempre o dia em America/Sao_Paulo.
_Avoid_: total, caixa

**Saldo projetado** (`ProjectedBalance`):
Saldo esperado numa data futura de até 12 meses: o Saldo atual mais as Receitas e Despesas não vinculadas a uma Fatura com data até ela (Projeções), menos o valor restante de toda Fatura com vencimento até essa data. Compras no cartão ainda não lançadas não entram.
_Avoid_: previsão

**Saldo inicial** (`OpeningBalance`):
Valor opcional que o usuário informa uma vez, representando o dinheiro que já possuía ao começar a usar o sistema.

### Cartão de crédito

**Cartão de crédito** (`CreditCard`):
Cartão cadastrado pelo usuário, com Dia de fechamento e Dia de vencimento. Não tem limite de crédito. Mudar os dias vale só para Faturas ainda abertas. Pode ser desativado, quando não há Fatura com valor restante, e some das opções para novas compras.
_Avoid_: cartão, crédito

**Dia de fechamento** (`ClosingDay`):
Dia do mês em que a Fatura de um Cartão de crédito fecha. Compras feitas até esse dia (inclusive) entram na fatura que fecha no mês; depois dele, na do mês seguinte.

**Dia de vencimento** (`DueDay`):
Dia do mês em que a Fatura fechada deve ser paga.

**Fatura** (`Invoice`):
Conjunto das Compras no cartão e Parcelas de um Cartão de crédito em um ciclo de fechamento. Só afeta o Saldo atual por meio de Pagamentos de fatura. Passa por três estados: **Aberta** (recebe compras), **Fechada** (após o Dia de fechamento) e **Paga** (valor restante zerado). Compras e Parcelas de uma Fatura Paga não podem ser editadas nem excluídas.
_Avoid_: extrato, conta do cartão

**Compra no cartão** (`CardPurchase`):
Despesa vinculada a uma Fatura. Não altera o Saldo atual na data da compra.
_Avoid_: gasto no cartão, compra no crédito

**Compra parcelada** (`InstallmentPurchase`):
Compra no cartão dividida em várias Parcelas. O usuário informa o valor total já com juros, se houver; o sistema não calcula nem registra juros. Enquanto nenhuma Parcela estiver numa Fatura paga, o usuário pode editar valor total, número de Parcelas, data e Categoria, e todas as Parcelas são recalculadas e realocadas. Se já houver Parcelas em Faturas pagas, o usuário só pode mudar a Categoria e cancelar as Parcelas restantes.
_Avoid_: parcelamento

**Parcela** (`Installment`):
Fração de uma Compra parcelada, alocada a uma Fatura. O valor é dividido igualmente e o centavo que sobra da divisão fica na primeira Parcela.
_Avoid_: prestação

**Pagamento de fatura** (`InvoicePayment`):
Saída do Saldo atual que quita uma Fatura, total ou parcialmente. No Dia de vencimento, o sistema registra sozinho um Pagamento de fatura com o valor ainda não pago. Não é uma Despesa categorizada, para não contar o gasto duas vezes nos relatórios. O usuário pode editá-lo para o valor realmente pago; a diferença vira Saldo rolado na próxima Fatura, e a Fatura original passa a Paga. Ele continua editável até a Fatura que recebeu o Saldo rolado ficar Paga.
_Avoid_: quitação

**Saldo rolado** (`CarriedBalance`):
Parte de uma Fatura que o usuário não pagou no vencimento e que passa como item para a próxima Fatura. Não é Despesa, porque o gasto original já foi contado.
_Avoid_: rotativo, dívida

**Encargos** (`Charges`):
Despesa lançada pelo usuário com os juros, multa e IOF cobrados pelo banco numa Fatura, na Categoria padrão "Juros e encargos". O sistema não calcula juros, só registra o valor informado.
_Avoid_: juros, taxas

**Pagamento antecipado** (`EarlyPayment`):
Pagamento de fatura feito antes do Dia de vencimento, na Fatura aberta ou fechada, no valor integral ou em parte. Compras lançadas depois apenas aumentam o valor restante.
_Avoid_: adiantamento de fatura

**Antecipação de parcelas** (`InstallmentAnticipation`):
Operação que traz Parcelas escolhidas pelo usuário, de faturas futuras do mesmo Cartão de crédito, para a Fatura aberta, opcionalmente com um Desconto. As Parcelas passam a contar no Mês de referência da Fatura aberta. É irreversível: exige confirmação do usuário e um aviso de que não pode ser desfeita.
_Avoid_: adiantar parcelas

**Desconto** (`Discount`):
Valor que o usuário informa, abatido do total das Parcelas ao fazer uma Antecipação de parcelas, refletindo o benefício oferecido por alguns bancos. É dividido proporcionalmente entre as Parcelas, reduzindo cada uma e a Despesa na Categoria da compra original.
