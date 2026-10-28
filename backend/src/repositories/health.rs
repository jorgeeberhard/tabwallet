use sqlx::PgPool;

pub struct HealthRepository {
    pool: PgPool,
}

impl HealthRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Succeeds when the database answers a trivial query.
    pub async fn ping(&self) -> Result<(), sqlx::Error> {
        sqlx::query("SELECT 1").execute(&self.pool).await?;
        Ok(())
    }
}
