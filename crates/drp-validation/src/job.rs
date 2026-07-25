//! Scheduler job handler for kind `validation`.
//!
//! Job params:
//! - `asset_id` (string, optional if `check_ids` set)
//! - `connector_id` (string, default `mock`)
//! - `check_ids` (array of strings, optional)

use async_trait::async_trait;
use serde_json::{json, Value};
use tracing::info;

use drp_common::{AssetId, CheckId, Error, Result};
use drp_core::JobDefinition;
use drp_scheduler::JobHandler;

use crate::ValidationService;

/// Job kind identifier registered with the scheduler.
pub const VALIDATION_JOB_KIND: &str = "validation";

/// Runs a validation suite when the scheduler fires a `validation` job.
pub struct ValidationJobHandler {
    validation: ValidationService,
}

impl ValidationJobHandler {
    /// Create a handler bound to a validation service.
    pub fn new(validation: ValidationService) -> Self {
        Self { validation }
    }
}

#[async_trait]
impl JobHandler for ValidationJobHandler {
    fn kind(&self) -> &str {
        VALIDATION_JOB_KIND
    }

    async fn execute(&self, job: &JobDefinition) -> Result<Option<Value>> {
        let connector_id = job
            .params
            .get("connector_id")
            .and_then(|v| v.as_str())
            .unwrap_or("mock");

        let asset_id = match job.params.get("asset_id").and_then(|v| v.as_str()) {
            Some(s) => Some(s.parse::<AssetId>().map_err(|e| {
                Error::validation(format!("invalid asset_id in validation job: {e}"))
            })?),
            None => None,
        };

        let check_ids =
            match job.params.get("check_ids") {
                Some(Value::Array(arr)) => {
                    let mut ids = Vec::new();
                    for v in arr {
                        let s = v
                            .as_str()
                            .ok_or_else(|| Error::validation("check_ids must be strings"))?;
                        ids.push(s.parse::<CheckId>().map_err(|e| {
                            Error::validation(format!("invalid check_id in job: {e}"))
                        })?);
                    }
                    Some(ids)
                }
                None => None,
                _ => {
                    return Err(Error::validation(
                        "validation job param check_ids must be an array",
                    ))
                }
            };

        info!(
            job_id = %job.id,
            asset_id = ?asset_id,
            "running scheduled validation suite"
        );

        let run = self
            .validation
            .run_suite(asset_id, connector_id, check_ids, Some(job.id))
            .await?;

        Ok(Some(json!({
            "suite_run_id": run.id.to_string(),
            "status": run.status,
            "passed": run.passed,
            "failed": run.failed,
            "warned": run.warned,
            "skipped": run.skipped,
            "errored": run.errored,
            "asset_id": run.asset_id.map(|id| id.to_string()),
        })))
    }
}
