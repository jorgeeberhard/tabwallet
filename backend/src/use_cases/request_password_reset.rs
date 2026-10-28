use std::sync::Arc;

use chrono::Duration;

use crate::{
    auth::reset_token,
    clock::Clock,
    email::{EmailMessage, EmailSender},
    repositories::{password_resets::PasswordResetRepository, users::UserRepository},
};

use super::AuthError;

/// How long a reset link works.
pub const RESET_LINK_TTL_MINUTES: i64 = 60;

/// UC03, first half: e-mails a reset link to the owner of an address.
pub struct RequestPasswordResetUseCase {
    users: UserRepository,
    resets: PasswordResetRepository,
    sender: Arc<dyn EmailSender>,
    clock: Arc<dyn Clock>,
    app_url: String,
}

impl RequestPasswordResetUseCase {
    pub fn new(
        users: UserRepository,
        resets: PasswordResetRepository,
        sender: Arc<dyn EmailSender>,
        clock: Arc<dyn Clock>,
        app_url: String,
    ) -> Self {
        Self {
            users,
            resets,
            sender,
            clock,
            app_url,
        }
    }

    /// Succeeds, and looks the same to the caller, whether or not the address
    /// has an account, and also when the e-mail provider fails (that is only
    /// logged): any difference would reveal which addresses are registered.
    pub async fn execute(&self, email: &str) -> Result<(), AuthError> {
        let Some(account) = self
            .users
            .find_credentials_by_email(email)
            .await
            .map_err(AuthError::internal)?
        else {
            return Ok(());
        };
        let user = account.user;

        let token = reset_token::generate();
        let now = self.clock.now();
        self.resets
            .replace_token(
                user.id,
                &reset_token::hash(&token),
                now,
                now + Duration::minutes(RESET_LINK_TTL_MINUTES),
            )
            .await
            .map_err(AuthError::internal)?;

        let message = reset_message(&user.email, &user.name, &self.app_url, &token);
        if let Err(error) = self.sender.send(message).await {
            tracing::error!("password reset e-mail was not delivered: {error}");
        }
        Ok(())
    }
}

fn reset_message(to: &str, name: &str, app_url: &str, token: &str) -> EmailMessage {
    let greeting = if name.is_empty() {
        "Olá,".to_string()
    } else {
        format!("Olá, {name},")
    };
    EmailMessage {
        to: to.to_string(),
        subject: "Redefinir sua senha do TabWallet".to_string(),
        text: format!(
            "{greeting}\n\n\
             Recebemos um pedido para redefinir a senha da sua conta no TabWallet. \
             Para escolher uma nova senha, abra o link abaixo. Ele vale por \
             {RESET_LINK_TTL_MINUTES} minutos e só pode ser usado uma vez:\n\n\
             {app_url}/reset-password?token={token}\n\n\
             Se você não pediu isso, ignore este e-mail: sua senha continua a mesma.\n"
        ),
    }
}
