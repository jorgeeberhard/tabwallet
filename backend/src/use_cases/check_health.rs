use crate::repositories::health::HealthRepository;

pub struct HealthReport {
    pub database_up: bool,
}

pub struct CheckHealthUseCase {
    repository: HealthRepository,
}

impl CheckHealthUseCase {
    pub fn new(repository: HealthRepository) -> Self {
        Self { repository }
    }

    pub async fn execute(&self) -> HealthReport {
        HealthReport {
            database_up: self.repository.ping().await.is_ok(),
        }
    }
}
