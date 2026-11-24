//! The e-mails of the sign-up (plain text, in Portuguese).

use crate::email::EmailMessage;

/// How long the Confirmation link of a Pending signup works.
pub const CONFIRMATION_LINK_TTL_HOURS: i64 = 24;

fn greeting(name: &str) -> String {
    if name.is_empty() {
        "Olá,".to_string()
    } else {
        format!("Olá, {name},")
    }
}

/// The e-mail with the Confirmation link of a Pending signup.
pub fn confirmation(to: &str, name: &str, app_url: &str, token: &str) -> EmailMessage {
    EmailMessage {
        to: to.to_string(),
        subject: "Confirme seu e-mail no TabWallet".to_string(),
        text: format!(
            "{greeting}\n\n\
             Recebemos um pedido para criar uma conta no TabWallet com este e-mail. \
             Para confirmar que o e-mail é seu e criar a conta, abra o link abaixo. Ele vale \
             por {CONFIRMATION_LINK_TTL_HOURS} horas e só pode ser usado uma vez:\n\n\
             {app_url}/confirm-email?token={token}\n\n\
             Depois de confirmar, entre com o e-mail e a senha que você escolheu.\n\n\
             Se você não pediu isso, ignore este e-mail: nenhuma conta será criada.\n",
            greeting = greeting(name),
        ),
    }
}

/// The notice to the owner of an account when someone asks to sign up with
/// its e-mail. Carries no link that creates or changes anything.
pub fn account_already_exists(to: &str, name: &str, app_url: &str) -> EmailMessage {
    EmailMessage {
        to: to.to_string(),
        subject: "Você já tem uma conta no TabWallet".to_string(),
        text: format!(
            "{greeting}\n\n\
             Alguém pediu para criar uma conta no TabWallet com este e-mail, mas você já tem \
             uma conta. Se foi você, entre ou redefina a senha:\n\n\
             Entrar: {app_url}/login\n\
             Redefinir a senha: {app_url}/forgot-password\n\n\
             Se não foi você, ignore este e-mail: nada foi alterado na sua conta.\n",
            greeting = greeting(name),
        ),
    }
}
