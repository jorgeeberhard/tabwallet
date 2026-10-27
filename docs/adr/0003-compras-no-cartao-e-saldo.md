# Compras no cartão não alteram o saldo; a Fatura é debitada no vencimento

Uma Compra no cartão (ou Parcela) pertence a uma Fatura e não muda o Saldo atual. O saldo só cai por Pagamento de fatura: o usuário pode pagar antes, total ou parcialmente, e no Dia de vencimento o sistema debita sozinho o que sobrou. Relatórios e Orçamentos contam cada item no Mês de referência do vencimento da Fatura, o mesmo mês em que o dinheiro sai do saldo, e o Pagamento de fatura não é uma Despesa categorizada para não duplicar o gasto. Quem não pagou tudo no banco corrige o pagamento, e a diferença vira Saldo rolado na próxima Fatura, sem juros calculados pelo sistema.

## Considered Options

- **Compra reduz o saldo na data da compra:** simples, mas o saldo mostraria dinheiro que ainda está na conta e a Fatura deixaria de ter sentido.
- **Usuário confirma manualmente o pagamento de cada Fatura:** reflete melhor a realidade, mas faz o saldo errar toda vez que o usuário esquecer de registrar o pagamento.

## Consequences

- A regra de que o usuário pagou tudo é uma suposição que ele corrige, não um fato registrado; por isso existe a edição do Pagamento de fatura (Saldo rolado).
- Relatório por categoria e Saldo atual usam fórmulas diferentes (Despesas por Mês de referência x saldo por data de saída), e as duas precisam ser testadas juntas.
