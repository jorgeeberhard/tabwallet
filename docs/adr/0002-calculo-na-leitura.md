# Ocorrências e pagamentos automáticos são calculados na leitura

As Ocorrências de Transações fixas, o Pagamento de fatura automático do vencimento e o Saldo rolado não são gravados por um job agendado: o sistema os calcula quando uma tela ou a API os pede. Só se grava algo quando o usuário muda alguma coisa (edita ou exclui uma Ocorrência, corrige um Pagamento de fatura, lança Encargos). A API de back-end roda em um único serviço sem processo em segundo plano, o prazo é de 9 semanas, e um job que falhasse deixaria Saldo atual e Faturas errados sem o usuário perceber.

## Considered Options

- **Job agendado (cron) que grava Ocorrências e pagamentos:** o saldo é uma simples soma, mas exige um agendador confiável no Fly.io/Railway, reprocessamento se o job falhar e regeneração quando o usuário edita uma Transação fixa.

## Consequences

- Consultas de saldo, listagem e relatório precisam aplicar as regras de Ocorrência e vencimento (e a janela de 12 meses do Saldo projetado), então essa lógica fica concentrada nos use cases do back-end e é a principal candidata a testes automatizados.
- Trocar para gravação por job no futuro exige uma migração de dados e é a parte cara de reverter.
