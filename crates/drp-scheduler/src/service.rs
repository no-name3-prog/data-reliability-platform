//! Scheduler service.

use std::sync::Arc;
use std::time::Duration;

use tracing::{error, info, instrument, warn};

use crate::handler::{JobHandlerRegistry, NoopHandler};
use drp_common::{Error, JobId, Result, RunId};
use drp_core::{EventBus, JobDefinition, JobRun, JobStatus, PlatformEvent};
use drp_storage::Store;

/// Job scheduling and execution service.
#[derive(Clone)]
pub struct SchedulerService {
    store: Arc<dyn Store>,
    handlers: JobHandlerRegistry,
    events: EventBus,
    max_concurrent: usize,
}

impl SchedulerService {
    /// Create a scheduler with a built-in `noop` handler.
    pub fn new(store: Arc<dyn Store>, events: EventBus, max_concurrent: usize) -> Self {
        let handlers = JobHandlerRegistry::new();
        handlers.register(Arc::new(NoopHandler));
        Self {
            store,
            handlers,
            events,
            max_concurrent,
        }
    }

    /// Access the handler registry for extension.
    pub fn handlers(&self) -> &JobHandlerRegistry {
        &self.handlers
    }

    /// Create or update a job definition.
    pub async fn upsert_job(&self, job: JobDefinition) -> Result<JobDefinition> {
        self.store.upsert_job(job).await
    }

    /// Get a job.
    pub async fn get_job(&self, id: &JobId) -> Result<JobDefinition> {
        self.store
            .get_job(id)
            .await?
            .ok_or_else(|| Error::not_found(format!("job {id}")))
    }

    /// List jobs.
    pub async fn list_jobs(&self) -> Result<Vec<JobDefinition>> {
        self.store.list_jobs().await
    }

    /// Enqueue and immediately execute a job run.
    #[instrument(skip(self), fields(job_id = %job_id))]
    pub async fn run_job(&self, job_id: &JobId) -> Result<JobRun> {
        let job = self.get_job(job_id).await?;
        if !job.enabled {
            return Err(Error::scheduler(format!("job {job_id} is disabled")));
        }

        let mut run = JobRun::pending(job.id);
        run.mark_running();
        self.store.save_job_run(run.clone()).await?;

        let handler = match self.handlers.get(&job.kind) {
            Ok(h) => h,
            Err(e) => {
                run.mark_failed(e.to_string());
                return self.store.save_job_run(run).await;
            }
        };

        match handler.execute(&job).await {
            Ok(payload) => run.mark_succeeded(payload),
            Err(e) => {
                error!(error = %e, "job handler failed");
                run.mark_failed(e.to_string());
            }
        }

        let saved = self.store.save_job_run(run).await?;
        self.events
            .publish(PlatformEvent::JobCompleted {
                job_id: saved.job_id,
                run_id: saved.id,
                success: saved.status == JobStatus::Succeeded,
            })
            .await;
        Ok(saved)
    }

    /// Get a job run.
    pub async fn get_run(&self, id: &RunId) -> Result<JobRun> {
        self.store
            .get_job_run(id)
            .await?
            .ok_or_else(|| Error::not_found(format!("job run {id}")))
    }

    /// List runs for a job.
    pub async fn list_runs(&self, job_id: &JobId, limit: Option<usize>) -> Result<Vec<JobRun>> {
        self.store.list_job_runs(job_id, limit).await
    }

    /// Background tick loop for scheduled jobs.
    pub async fn run_loop(
        self,
        tick_interval: Duration,
        mut shutdown: tokio::sync::watch::Receiver<bool>,
    ) {
        info!(
            interval_secs = tick_interval.as_secs(),
            max_concurrent = self.max_concurrent,
            "scheduler loop started"
        );
        let sem = Arc::new(tokio::sync::Semaphore::new(self.max_concurrent));

        loop {
            tokio::select! {
                _ = shutdown.changed() => {
                    if *shutdown.borrow() {
                        info!("scheduler shutting down");
                        break;
                    }
                }
                _ = tokio::time::sleep(tick_interval) => {
                    if let Err(e) = self.tick_once(sem.clone()).await {
                        warn!(error = %e, "scheduler tick failed");
                    }
                }
            }
        }
    }

    async fn tick_once(&self, sem: Arc<tokio::sync::Semaphore>) -> Result<()> {
        let jobs = self.list_jobs().await?;
        let mut handles = Vec::new();
        for job in jobs {
            if !job.enabled || job.schedule.is_none() {
                continue;
            }
            let Ok(permit) = sem.clone().acquire_owned().await else {
                break;
            };
            let this = self.clone();
            let job_id = job.id;
            handles.push(tokio::spawn(async move {
                let _permit = permit;
                if let Err(e) = this.run_job(&job_id).await {
                    warn!(%job_id, error = %e, "scheduled job run failed");
                }
            }));
        }
        for h in handles {
            let _ = h.await;
        }
        Ok(())
    }
}
