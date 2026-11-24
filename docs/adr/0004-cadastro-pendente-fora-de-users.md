# O cadastro pendente fica numa tabela própria, fora de `users`

Quem pede o cadastro vira um Cadastro pendente (e-mail, nome, hash da senha e hash do link), numa tabela própria; só a confirmação do e-mail cria a Conta, com as categorias padrão, numa única transação. A tabela `users` guarda apenas contas confirmadas, então nenhuma rota precisa checar "esta conta confirmou?", e um cadastro pendente nunca ocupa o e-mail de ninguém: um novo cadastro com o mesmo e-mail o substitui. A troca de e-mail segue a mesma ideia, com uma tabela de Troca de e-mail pendente.

## Considered Options

- **Coluna "confirmado" em `users`:** menos tabelas, mas a conta não confirmada existiria de verdade (com categorias padrão), cada rota teria de lembrar de recusá-la, e alguém poderia cadastrar o e-mail de outra pessoa e impedi-la de se cadastrar.

## Consequences

- Pendentes expirados não são apagados por tarefa agendada: são ignorados e removidos quando chega um novo cadastro (mesma linha do ADR-0002).
- A confirmação precisa recusar o caso em que o e-mail virou Conta de outra pessoa entre o cadastro e o clique.
- O login só enxerga Contas; para dizer "Confirme seu e-mail" a API consulta o cadastro pendente, e só quando e-mail e senha batem, para não revelar quais e-mails têm cadastro pendente.
