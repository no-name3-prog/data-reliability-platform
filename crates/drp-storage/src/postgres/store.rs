//! SQL implementation of the store trait.

use async_trait::async_trait;
use sqlx::postgres::PgPoolOptions;
use sqlx::{PgPool, Row};
use tracing::info;

use drp_common::{AssetId, CheckId, Error, JobId, Result, RunId};
use drp_core::{
    Asset, CheckDefinition, CheckResult, DatasetProfile, JobDefinition, JobRun, ValidationRun,
};

use crate::traits::Store;

/// PostgreSQL metadata store.
pub struct PostgresStore {
    pool: PgPool,
}

impl PostgresStore {
    /// Connect and run migrations.
    pub async fn connect(database_url: &str, max_connections: u32) -> Result<Self> {
        let pool = PgPoolOptions::new()
            .max_connections(max_connections.max(1))
            .acquire_timeout(std::time::Duration::from_secs(15))
            .connect(database_url)
            .await
            .map_err(|e| Error::storage(format!("postgres connect: {e}")))?;
        let store = Self { pool };
        store.migrate().await?;
        Ok(store)
    }

    async fn migrate(&self) -> Result<()> {
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS assets (
                id TEXT PRIMARY KEY,
                fqn TEXT NOT NULL UNIQUE,
                payload JSONB NOT NULL,
                updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
            );
            CREATE TABLE IF NOT EXISTS checks (
                id TEXT PRIMARY KEY,
                asset_id TEXT,
                payload JSONB NOT NULL,
                updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
            );
            CREATE TABLE IF NOT EXISTS check_results (
                run_id TEXT PRIMARY KEY,
                check_id TEXT NOT NULL,
                payload JSONB NOT NULL,
                created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
            );
            -- Append-only validation suite runs.
            CREATE TABLE IF NOT EXISTS validation_runs (
                run_id TEXT PRIMARY KEY,
                asset_id TEXT,
                payload JSONB NOT NULL,
                created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
            );
            -- Append-only profile history (one row per profile run).
            CREATE TABLE IF NOT EXISTS profile_history (
                run_id TEXT PRIMARY KEY,
                asset_id TEXT NOT NULL,
                payload JSONB NOT NULL,
                created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
            );
            -- Legacy single-row profiles table (kept for older installs; unused by new code).
            CREATE TABLE IF NOT EXISTS profiles (
                asset_id TEXT PRIMARY KEY,
                payload JSONB NOT NULL,
                updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
            );
            CREATE TABLE IF NOT EXISTS jobs (
                id TEXT PRIMARY KEY,
                payload JSONB NOT NULL,
                updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
            );
            CREATE TABLE IF NOT EXISTS job_runs (
                id TEXT PRIMARY KEY,
                job_id TEXT NOT NULL,
                payload JSONB NOT NULL,
                created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
            );
            CREATE INDEX IF NOT EXISTS idx_check_results_check ON check_results(check_id);
            CREATE INDEX IF NOT EXISTS idx_job_runs_job ON job_runs(job_id);
            CREATE INDEX IF NOT EXISTS idx_profile_history_asset ON profile_history(asset_id);
            CREATE INDEX IF NOT EXISTS idx_validation_runs_asset ON validation_runs(asset_id);
            "#,
        )
        .execute(&self.pool)
        .await
        .map_err(|e| Error::storage(format!("migrate: {e}")))?;
        info!("postgres metadata schema ready");
        Ok(())
    }
}

#[async_trait]
impl Store for PostgresStore {
    async fn upsert_asset(&self, asset: Asset) -> Result<Asset> {
        let payload = serde_json::to_value(&asset)
            .map_err(|e| Error::storage(format!("serialize asset: {e}")))?;
        sqlx::query(
            r#"
            INSERT INTO assets (id, fqn, payload)
            VALUES ($1, $2, $3)
            ON CONFLICT (id) DO UPDATE SET fqn = EXCLUDED.fqn, payload = EXCLUDED.payload, updated_at = NOW()
            "#,
        )
        .bind(asset.id.to_string())
        .bind(&asset.fqn)
        .bind(payload)
        .execute(&self.pool)
        .await
        .map_err(|e| Error::storage(format!("upsert asset: {e}")))?;
        Ok(asset)
    }

    async fn get_asset(&self, id: &AssetId) -> Result<Option<Asset>> {
        let row = sqlx::query("SELECT payload FROM assets WHERE id = $1")
            .bind(id.to_string())
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| Error::storage(e.to_string()))?;
        match row {
            Some(r) => {
                let v: serde_json::Value = r
                    .try_get("payload")
                    .map_err(|e| Error::storage(e.to_string()))?;
                Ok(Some(
                    serde_json::from_value(v).map_err(|e| Error::storage(e.to_string()))?,
                ))
            }
            None => Ok(None),
        }
    }

    async fn get_asset_by_fqn(&self, fqn: &str) -> Result<Option<Asset>> {
        let row = sqlx::query("SELECT payload FROM assets WHERE fqn = $1")
            .bind(fqn)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| Error::storage(e.to_string()))?;
        match row {
            Some(r) => {
                let v: serde_json::Value = r
                    .try_get("payload")
                    .map_err(|e| Error::storage(e.to_string()))?;
                Ok(Some(
                    serde_json::from_value(v).map_err(|e| Error::storage(e.to_string()))?,
                ))
            }
            None => Ok(None),
        }
    }

    async fn list_assets(&self, limit: Option<usize>) -> Result<Vec<Asset>> {
        let lim = limit.unwrap_or(1000) as i64;
        let rows = sqlx::query("SELECT payload FROM assets ORDER BY fqn LIMIT $1")
            .bind(lim)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| Error::storage(e.to_string()))?;
        rows.into_iter()
            .map(|r| {
                let v: serde_json::Value = r
                    .try_get("payload")
                    .map_err(|e| Error::storage(e.to_string()))?;
                serde_json::from_value(v).map_err(|e| Error::storage(e.to_string()))
            })
            .collect()
    }

    async fn delete_asset(&self, id: &AssetId) -> Result<bool> {
        let res = sqlx::query("DELETE FROM assets WHERE id = $1")
            .bind(id.to_string())
            .execute(&self.pool)
            .await
            .map_err(|e| Error::storage(e.to_string()))?;
        Ok(res.rows_affected() > 0)
    }

    async fn upsert_check(&self, check: CheckDefinition) -> Result<CheckDefinition> {
        let payload = serde_json::to_value(&check).map_err(|e| Error::storage(e.to_string()))?;
        sqlx::query(
            r#"
            INSERT INTO checks (id, asset_id, payload)
            VALUES ($1, $2, $3)
            ON CONFLICT (id) DO UPDATE SET asset_id = EXCLUDED.asset_id, payload = EXCLUDED.payload, updated_at = NOW()
            "#,
        )
        .bind(check.id.to_string())
        .bind(check.asset_id.to_string())
        .bind(payload)
        .execute(&self.pool)
        .await
        .map_err(|e| Error::storage(e.to_string()))?;
        Ok(check)
    }

    async fn get_check(&self, id: &CheckId) -> Result<Option<CheckDefinition>> {
        let row = sqlx::query("SELECT payload FROM checks WHERE id = $1")
            .bind(id.to_string())
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| Error::storage(e.to_string()))?;
        Ok(match row {
            Some(r) => Some(
                serde_json::from_value(
                    r.try_get("payload")
                        .map_err(|e| Error::storage(e.to_string()))?,
                )
                .map_err(|e| Error::storage(e.to_string()))?,
            ),
            None => None,
        })
    }

    async fn list_checks(&self, asset_id: Option<&AssetId>) -> Result<Vec<CheckDefinition>> {
        let rows = if let Some(aid) = asset_id {
            sqlx::query("SELECT payload FROM checks WHERE asset_id = $1")
                .bind(aid.to_string())
                .fetch_all(&self.pool)
                .await
        } else {
            sqlx::query("SELECT payload FROM checks")
                .fetch_all(&self.pool)
                .await
        }
        .map_err(|e| Error::storage(e.to_string()))?;
        rows.into_iter()
            .map(|r| {
                serde_json::from_value(
                    r.try_get("payload")
                        .map_err(|e| Error::storage(e.to_string()))?,
                )
                .map_err(|e| Error::storage(e.to_string()))
            })
            .collect()
    }

    async fn save_check_result(&self, result: CheckResult) -> Result<CheckResult> {
        let payload = serde_json::to_value(&result).map_err(|e| Error::storage(e.to_string()))?;
        sqlx::query(
            r#"
            INSERT INTO check_results (run_id, check_id, payload)
            VALUES ($1, $2, $3)
            ON CONFLICT (run_id) DO UPDATE SET payload = EXCLUDED.payload
            "#,
        )
        .bind(result.run_id.to_string())
        .bind(result.check_id.to_string())
        .bind(payload)
        .execute(&self.pool)
        .await
        .map_err(|e| Error::storage(e.to_string()))?;
        Ok(result)
    }

    async fn list_check_results(
        &self,
        check_id: &CheckId,
        limit: Option<usize>,
    ) -> Result<Vec<CheckResult>> {
        let lim = limit.unwrap_or(50) as i64;
        let rows = sqlx::query(
            "SELECT payload FROM check_results WHERE check_id = $1 ORDER BY created_at DESC LIMIT $2",
        )
        .bind(check_id.to_string())
        .bind(lim)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| Error::storage(e.to_string()))?;
        rows.into_iter()
            .map(|r| {
                serde_json::from_value(
                    r.try_get("payload")
                        .map_err(|e| Error::storage(e.to_string()))?,
                )
                .map_err(|e| Error::storage(e.to_string()))
            })
            .collect()
    }

    async fn save_validation_run(&self, run: ValidationRun) -> Result<ValidationRun> {
        let payload = serde_json::to_value(&run).map_err(|e| Error::storage(e.to_string()))?;
        let asset_id = run.asset_id.map(|id| id.to_string());
        sqlx::query(
            r#"
            INSERT INTO validation_runs (run_id, asset_id, payload)
            VALUES ($1, $2, $3)
            ON CONFLICT (run_id) DO UPDATE SET payload = EXCLUDED.payload, asset_id = EXCLUDED.asset_id
            "#,
        )
        .bind(run.id.to_string())
        .bind(asset_id)
        .bind(payload)
        .execute(&self.pool)
        .await
        .map_err(|e| Error::storage(e.to_string()))?;
        Ok(run)
    }

    async fn get_validation_run(&self, id: &RunId) -> Result<Option<ValidationRun>> {
        let row = sqlx::query("SELECT payload FROM validation_runs WHERE run_id = $1")
            .bind(id.to_string())
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| Error::storage(e.to_string()))?;
        Ok(match row {
            Some(r) => Some(
                serde_json::from_value(
                    r.try_get("payload")
                        .map_err(|e| Error::storage(e.to_string()))?,
                )
                .map_err(|e| Error::storage(e.to_string()))?,
            ),
            None => None,
        })
    }

    async fn list_validation_runs(
        &self,
        asset_id: Option<&AssetId>,
        limit: Option<usize>,
    ) -> Result<Vec<ValidationRun>> {
        let lim = limit.unwrap_or(100) as i64;
        let rows = if let Some(aid) = asset_id {
            sqlx::query(
                r#"
                SELECT payload FROM validation_runs
                WHERE asset_id = $1
                ORDER BY created_at DESC
                LIMIT $2
                "#,
            )
            .bind(aid.to_string())
            .bind(lim)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| Error::storage(e.to_string()))?
        } else {
            sqlx::query(
                r#"
                SELECT payload FROM validation_runs
                ORDER BY created_at DESC
                LIMIT $1
                "#,
            )
            .bind(lim)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| Error::storage(e.to_string()))?
        };
        rows.into_iter()
            .map(|r| {
                serde_json::from_value(
                    r.try_get("payload")
                        .map_err(|e| Error::storage(e.to_string()))?,
                )
                .map_err(|e| Error::storage(e.to_string()))
            })
            .collect()
    }

    async fn save_profile(&self, profile: DatasetProfile) -> Result<DatasetProfile> {
        let payload = serde_json::to_value(&profile).map_err(|e| Error::storage(e.to_string()))?;
        sqlx::query(
            r#"
            INSERT INTO profile_history (run_id, asset_id, payload)
            VALUES ($1, $2, $3)
            ON CONFLICT (run_id) DO UPDATE SET payload = EXCLUDED.payload
            "#,
        )
        .bind(profile.run_id.to_string())
        .bind(profile.asset_id.to_string())
        .bind(payload)
        .execute(&self.pool)
        .await
        .map_err(|e| Error::storage(e.to_string()))?;
        Ok(profile)
    }

    async fn latest_profile(&self, asset_id: &AssetId) -> Result<Option<DatasetProfile>> {
        let row = sqlx::query(
            r#"
            SELECT payload FROM profile_history
            WHERE asset_id = $1
            ORDER BY created_at DESC
            LIMIT 1
            "#,
        )
        .bind(asset_id.to_string())
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| Error::storage(e.to_string()))?;
        Ok(match row {
            Some(r) => Some(
                serde_json::from_value(
                    r.try_get("payload")
                        .map_err(|e| Error::storage(e.to_string()))?,
                )
                .map_err(|e| Error::storage(e.to_string()))?,
            ),
            None => None,
        })
    }

    async fn list_profile_history(
        &self,
        asset_id: &AssetId,
        limit: Option<usize>,
    ) -> Result<Vec<DatasetProfile>> {
        let lim = limit.unwrap_or(100) as i64;
        let rows = sqlx::query(
            r#"
            SELECT payload FROM profile_history
            WHERE asset_id = $1
            ORDER BY created_at DESC
            LIMIT $2
            "#,
        )
        .bind(asset_id.to_string())
        .bind(lim)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| Error::storage(e.to_string()))?;
        rows.into_iter()
            .map(|r| {
                serde_json::from_value(
                    r.try_get("payload")
                        .map_err(|e| Error::storage(e.to_string()))?,
                )
                .map_err(|e| Error::storage(e.to_string()))
            })
            .collect()
    }

    async fn get_profile_by_run(
        &self,
        asset_id: &AssetId,
        run_id: &RunId,
    ) -> Result<Option<DatasetProfile>> {
        let row = sqlx::query(
            r#"
            SELECT payload FROM profile_history
            WHERE asset_id = $1 AND run_id = $2
            "#,
        )
        .bind(asset_id.to_string())
        .bind(run_id.to_string())
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| Error::storage(e.to_string()))?;
        Ok(match row {
            Some(r) => Some(
                serde_json::from_value(
                    r.try_get("payload")
                        .map_err(|e| Error::storage(e.to_string()))?,
                )
                .map_err(|e| Error::storage(e.to_string()))?,
            ),
            None => None,
        })
    }

    async fn upsert_job(&self, job: JobDefinition) -> Result<JobDefinition> {
        let payload = serde_json::to_value(&job).map_err(|e| Error::storage(e.to_string()))?;
        sqlx::query(
            r#"
            INSERT INTO jobs (id, payload) VALUES ($1, $2)
            ON CONFLICT (id) DO UPDATE SET payload = EXCLUDED.payload, updated_at = NOW()
            "#,
        )
        .bind(job.id.to_string())
        .bind(payload)
        .execute(&self.pool)
        .await
        .map_err(|e| Error::storage(e.to_string()))?;
        Ok(job)
    }

    async fn get_job(&self, id: &JobId) -> Result<Option<JobDefinition>> {
        let row = sqlx::query("SELECT payload FROM jobs WHERE id = $1")
            .bind(id.to_string())
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| Error::storage(e.to_string()))?;
        Ok(match row {
            Some(r) => Some(
                serde_json::from_value(
                    r.try_get("payload")
                        .map_err(|e| Error::storage(e.to_string()))?,
                )
                .map_err(|e| Error::storage(e.to_string()))?,
            ),
            None => None,
        })
    }

    async fn list_jobs(&self) -> Result<Vec<JobDefinition>> {
        let rows = sqlx::query("SELECT payload FROM jobs")
            .fetch_all(&self.pool)
            .await
            .map_err(|e| Error::storage(e.to_string()))?;
        rows.into_iter()
            .map(|r| {
                serde_json::from_value(
                    r.try_get("payload")
                        .map_err(|e| Error::storage(e.to_string()))?,
                )
                .map_err(|e| Error::storage(e.to_string()))
            })
            .collect()
    }

    async fn save_job_run(&self, run: JobRun) -> Result<JobRun> {
        let payload = serde_json::to_value(&run).map_err(|e| Error::storage(e.to_string()))?;
        sqlx::query(
            r#"
            INSERT INTO job_runs (id, job_id, payload) VALUES ($1, $2, $3)
            ON CONFLICT (id) DO UPDATE SET payload = EXCLUDED.payload
            "#,
        )
        .bind(run.id.to_string())
        .bind(run.job_id.to_string())
        .bind(payload)
        .execute(&self.pool)
        .await
        .map_err(|e| Error::storage(e.to_string()))?;
        Ok(run)
    }

    async fn get_job_run(&self, id: &RunId) -> Result<Option<JobRun>> {
        let row = sqlx::query("SELECT payload FROM job_runs WHERE id = $1")
            .bind(id.to_string())
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| Error::storage(e.to_string()))?;
        Ok(match row {
            Some(r) => Some(
                serde_json::from_value(
                    r.try_get("payload")
                        .map_err(|e| Error::storage(e.to_string()))?,
                )
                .map_err(|e| Error::storage(e.to_string()))?,
            ),
            None => None,
        })
    }

    async fn list_job_runs(&self, job_id: &JobId, limit: Option<usize>) -> Result<Vec<JobRun>> {
        let lim = limit.unwrap_or(50) as i64;
        let rows = sqlx::query(
            "SELECT payload FROM job_runs WHERE job_id = $1 ORDER BY created_at DESC LIMIT $2",
        )
        .bind(job_id.to_string())
        .bind(lim)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| Error::storage(e.to_string()))?;
        rows.into_iter()
            .map(|r| {
                serde_json::from_value(
                    r.try_get("payload")
                        .map_err(|e| Error::storage(e.to_string()))?,
                )
                .map_err(|e| Error::storage(e.to_string()))
            })
            .collect()
    }
}
