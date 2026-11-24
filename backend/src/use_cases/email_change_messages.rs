//! The e-mails of the e-mail change (plain text, in Portuguese). When the new
//! address already has an account, the person who asked is told nothing and
//! that address gets `sign_up_messages::account_already_exists` instead of a
//! link.

use crate::email::EmailMessage;

use super::sign_up_messages::{CONFIRMATION_LINK_TTL_HOURS, greeting};

/// The e-mail with the Confirmation link of a Pending e-mail change, sent to
/// the NEW address.
pub fn confirmation(to: &str, name: &str, app_url: &str, token: &str) -> EmailMessage {
    EmailMessage {
        to: to.to_string(),
        subject: "Confirme o novo e-mail do TabWallet".to_string(),
        text: format!(
            "{greeting}\n\n\
             Recebemos um pedido para usar este e-mail como o e-mail de acesso de uma conta do \
             TabWallet. Para confirmar que o e-mail é seu e concluir a troca, abra o link abaixo. \
             Ele vale por {CONFIRMATION_LINK_TTL_HOURS} horas e só pode ser usado uma vez:\n\n\
             {app_url}/confirm-email-change?token={token}\n\n\
             Até você abrir o link, a conta continua com o e-mail de antes.\n\n\
             Se você não pediu isso, ignore este e-mail: nada será alterado.\n",
            greeting = greeting(name),
        ),
    }
}

/// The notice to the OLD address when the change is done. It carries no link
/// that changes anything.
pub fn old_address_notice(to: &str, name: &str, new_email: &str, app_url: &str) -> EmailMessage {
    EmailMessage {
        to: to.to_string(),
        subject: "Seu e-mail de acesso ao TabWallet foi alterado".to_string(),
        text: format!(
            "{greeting}\n\n\
             O e-mail de acesso da sua conta no TabWallet foi alterado para {new_email}. \
             A partir de agora, entre com o novo e-mail e a mesma senha; este endereço não \
             serve mais para entrar.\n\n\
             Se foi você, não precisa fazer nada. Se não foi você, outra pessoa conhece a sua \
             senha: entre com o novo e-mail, se ele for seu, e troque a senha no perfil, ou use \
             \"Esqueci a senha\" em {app_url}/forgot-password.\n",
            greeting = greeting(name),
        ),
    }
}
