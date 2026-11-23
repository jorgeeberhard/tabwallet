use std::sync::Arc;

use chrono::NaiveDate;

use crate::{clock::Clock, repositories::health::HealthRepository};

pub struct HealthReport {
    pub database_up: bool,
    /// The day the service considers "today" (America/Sao_Paulo).
    pub today: NaiveDate,
}

pub struct CheckHealthUseCase {
    repository: HealthRepository,
    clock: Arc<dyn Clock>,
}

impl CheckHealthUseCase {
    pub fn new(repository: HealthRepository, clock: Arc<dyn Clock>) -> Self {
        Self { repository, clock }
    }

    pub async fn execute(&self) -> HealthReport {
        HealthReport {
            database_up: self.repository.ping().await.is_ok(),
            today: self.clock.today(),
        }
    }
}
