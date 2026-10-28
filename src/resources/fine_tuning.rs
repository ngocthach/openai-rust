//! `/fine_tuning` resource, mirroring `resources/fine_tuning/jobs/`:
//! create, retrieve, list, cancel, pause, resume, list events, and list
//! checkpoints for fine-tuning jobs.

use reqwest::Method;

use crate::client::Client;
use crate::error::OpenAIError;
use crate::pagination::List;
use crate::request::{api_path, RequestOptions};
use crate::types::fine_tuning::{
    FineTuningJob, FineTuningJobCheckpoint, FineTuningJobEvent, FineTuningJobListParams,
    FineTuningJobRequest, FineTuningPageParams,
};

/// Accessor for fine-tuning endpoints: `client.fine_tuning().jobs()`.
#[derive(Debug, Clone)]
pub struct FineTuning {
    client: Client,
}

impl FineTuning {
    pub(crate) fn new(client: Client) -> Self {
        Self { client }
    }

    /// The fine-tuning jobs resource.
    pub fn jobs(&self) -> FineTuningJobs {
        FineTuningJobs {
            client: self.client.clone(),
        }
    }
}

/// The `/fine_tuning/jobs` resource.
#[derive(Debug, Clone)]
pub struct FineTuningJobs {
    client: Client,
}

impl FineTuningJobs {
    /// Create a fine-tuning job.
    pub async fn create(
        &self,
        request: FineTuningJobRequest,
    ) -> Result<FineTuningJob, OpenAIError> {
        let body = serde_json::to_value(&request)?;
        self.client
            .execute(
                Method::POST,
                "/fine_tuning/jobs",
                RequestOptions::json(body),
            )
            .await
    }

    /// Retrieve a fine-tuning job.
    pub async fn retrieve(&self, job_id: &str) -> Result<FineTuningJob, OpenAIError> {
        self.client
            .execute(
                Method::GET,
                &api_path!("/fine_tuning/jobs/{}", job_id),
                RequestOptions::default(),
            )
            .await
    }

    /// List fine-tuning jobs (a single page).
    pub async fn list(
        &self,
        params: Option<FineTuningJobListParams>,
    ) -> Result<List<FineTuningJob>, OpenAIError> {
        let query = params.unwrap_or_default().to_query();
        self.client
            .execute(
                Method::GET,
                "/fine_tuning/jobs",
                RequestOptions::query(query),
            )
            .await
    }

    /// List all fine-tuning jobs, following pagination cursors.
    pub async fn list_all(
        &self,
        params: Option<FineTuningJobListParams>,
    ) -> Result<Vec<FineTuningJob>, OpenAIError> {
        let query = params.unwrap_or_default().to_query();
        self.client.paginate_all("/fine_tuning/jobs", query).await
    }

    /// Cancel a fine-tuning job.
    pub async fn cancel(&self, job_id: &str) -> Result<FineTuningJob, OpenAIError> {
        self.client
            .execute(
                Method::POST,
                &api_path!("/fine_tuning/jobs/{}/cancel", job_id),
                RequestOptions::default(),
            )
            .await
    }

    /// Pause a fine-tuning job.
    pub async fn pause(&self, job_id: &str) -> Result<FineTuningJob, OpenAIError> {
        self.client
            .execute(
                Method::POST,
                &api_path!("/fine_tuning/jobs/{}/pause", job_id),
                RequestOptions::default(),
            )
            .await
    }

    /// Resume a paused fine-tuning job.
    pub async fn resume(&self, job_id: &str) -> Result<FineTuningJob, OpenAIError> {
        self.client
            .execute(
                Method::POST,
                &api_path!("/fine_tuning/jobs/{}/resume", job_id),
                RequestOptions::default(),
            )
            .await
    }

    /// List status events for a fine-tuning job (a single page).
    pub async fn list_events(
        &self,
        job_id: &str,
        params: Option<FineTuningPageParams>,
    ) -> Result<List<FineTuningJobEvent>, OpenAIError> {
        let query = params.unwrap_or_default().to_query();
        self.client
            .execute(
                Method::GET,
                &api_path!("/fine_tuning/jobs/{}/events", job_id),
                RequestOptions::query(query),
            )
            .await
    }

    /// List checkpoints for a fine-tuning job (a single page).
    pub async fn list_checkpoints(
        &self,
        job_id: &str,
        params: Option<FineTuningPageParams>,
    ) -> Result<List<FineTuningJobCheckpoint>, OpenAIError> {
        let query = params.unwrap_or_default().to_query();
        self.client
            .execute(
                Method::GET,
                &api_path!("/fine_tuning/jobs/{}/checkpoints", job_id),
                RequestOptions::query(query),
            )
            .await
    }
}
