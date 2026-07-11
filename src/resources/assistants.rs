//! Assistants (beta v2) resources, mirroring `resources/beta/assistants.py`
//! and `resources/beta/threads/`.
//!
//! Every request in this module sends the `OpenAI-Beta: assistants=v2` header,
//! injected via [`RequestOptions::extra_headers`]. Nested resources take their
//! parent ids as method arguments rather than storing them (KISS).
//!
//! Streaming runs (`create_and_run`, run streaming) are intentionally out of
//! scope; use the non-streaming `create`/`retrieve` methods and poll.

use reqwest::Method;
use serde::de::DeserializeOwned;

use crate::client::Client;
use crate::error::OpenAIError;
use crate::pagination::{HasId, List};
use crate::request::RequestOptions;
use crate::types::assistants::{
    Assistant, AssistantCreateParams, AssistantDeleted, AssistantUpdateParams, ListParams,
    MessageCreateParams, MessageDeleted, MessageUpdateParams, Run, RunCreateParams, RunStep,
    RunUpdateParams, SubmitToolOutputsParams, Thread, ThreadCreateParams, ThreadDeleted,
    ThreadMessage, ThreadUpdateParams,
};

/// The beta header sent on every request in this module.
const BETA_HEADER: (&str, &str) = ("OpenAI-Beta", "assistants=v2");

/// Prepend the beta header so a caller-supplied header of the same name (added
/// later) would still win, matching the Python header-merge semantics.
fn with_beta(mut options: RequestOptions) -> RequestOptions {
    options
        .extra_headers
        .insert(0, (BETA_HEADER.0.to_string(), BETA_HEADER.1.to_string()));
    options
}

/// Fetch every item from a beta cursor-paginated endpoint, injecting the beta
/// header on each page request.
async fn paginate_all_beta<T>(
    client: &Client,
    path: &str,
    query: Vec<(String, String)>,
) -> Result<Vec<T>, OpenAIError>
where
    T: HasId + DeserializeOwned,
{
    client
        .paginate_all_with_headers(
            path,
            query,
            vec![(BETA_HEADER.0.to_string(), BETA_HEADER.1.to_string())],
        )
        .await
}

// ---------------------------------------------------------------------------
// Assistants
// ---------------------------------------------------------------------------

/// The assistants resource: `client.assistants()`.
#[derive(Debug, Clone)]
pub struct Assistants {
    client: Client,
}

impl Assistants {
    pub(crate) fn new(client: Client) -> Self {
        Self { client }
    }

    /// Create an assistant.
    pub async fn create(&self, params: AssistantCreateParams) -> Result<Assistant, OpenAIError> {
        let body = serde_json::to_value(&params)?;
        self.client
            .execute(
                Method::POST,
                "/assistants",
                with_beta(RequestOptions::json(body)),
            )
            .await
    }

    /// Retrieve an assistant by id.
    pub async fn retrieve(&self, assistant_id: &str) -> Result<Assistant, OpenAIError> {
        self.client
            .execute(
                Method::GET,
                &format!("/assistants/{assistant_id}"),
                with_beta(RequestOptions::default()),
            )
            .await
    }

    /// Update an assistant.
    pub async fn update(
        &self,
        assistant_id: &str,
        params: AssistantUpdateParams,
    ) -> Result<Assistant, OpenAIError> {
        let body = serde_json::to_value(&params)?;
        self.client
            .execute(
                Method::POST,
                &format!("/assistants/{assistant_id}"),
                with_beta(RequestOptions::json(body)),
            )
            .await
    }

    /// Delete an assistant.
    pub async fn delete(&self, assistant_id: &str) -> Result<AssistantDeleted, OpenAIError> {
        self.client
            .execute(
                Method::DELETE,
                &format!("/assistants/{assistant_id}"),
                with_beta(RequestOptions::default()),
            )
            .await
    }

    /// List assistants (a single page).
    pub async fn list(&self, params: Option<ListParams>) -> Result<List<Assistant>, OpenAIError> {
        let query = params.unwrap_or_default().to_query();
        self.client
            .execute(
                Method::GET,
                "/assistants",
                with_beta(RequestOptions::query(query)),
            )
            .await
    }

    /// List all assistants, following pagination cursors.
    pub async fn list_all(
        &self,
        params: Option<ListParams>,
    ) -> Result<Vec<Assistant>, OpenAIError> {
        let query = params.unwrap_or_default().to_query();
        paginate_all_beta(&self.client, "/assistants", query).await
    }
}

// ---------------------------------------------------------------------------
// Threads
// ---------------------------------------------------------------------------

/// The threads resource: `client.threads()`.
#[derive(Debug, Clone)]
pub struct Threads {
    client: Client,
}

impl Threads {
    pub(crate) fn new(client: Client) -> Self {
        Self { client }
    }

    /// Nested messages resource: `client.threads().messages()`.
    pub fn messages(&self) -> ThreadMessages {
        ThreadMessages::new(self.client.clone())
    }

    /// Nested runs resource: `client.threads().runs()`.
    pub fn runs(&self) -> Runs {
        Runs::new(self.client.clone())
    }

    /// Create a thread.
    pub async fn create(&self, params: ThreadCreateParams) -> Result<Thread, OpenAIError> {
        let body = serde_json::to_value(&params)?;
        self.client
            .execute(
                Method::POST,
                "/threads",
                with_beta(RequestOptions::json(body)),
            )
            .await
    }

    /// Retrieve a thread by id.
    pub async fn retrieve(&self, thread_id: &str) -> Result<Thread, OpenAIError> {
        self.client
            .execute(
                Method::GET,
                &format!("/threads/{thread_id}"),
                with_beta(RequestOptions::default()),
            )
            .await
    }

    /// Update a thread.
    pub async fn update(
        &self,
        thread_id: &str,
        params: ThreadUpdateParams,
    ) -> Result<Thread, OpenAIError> {
        let body = serde_json::to_value(&params)?;
        self.client
            .execute(
                Method::POST,
                &format!("/threads/{thread_id}"),
                with_beta(RequestOptions::json(body)),
            )
            .await
    }

    /// Delete a thread.
    pub async fn delete(&self, thread_id: &str) -> Result<ThreadDeleted, OpenAIError> {
        self.client
            .execute(
                Method::DELETE,
                &format!("/threads/{thread_id}"),
                with_beta(RequestOptions::default()),
            )
            .await
    }
}

// ---------------------------------------------------------------------------
// Thread messages
// ---------------------------------------------------------------------------

/// The thread messages resource: `client.threads().messages()`. Methods take
/// the owning `thread_id` as their first argument.
#[derive(Debug, Clone)]
pub struct ThreadMessages {
    client: Client,
}

impl ThreadMessages {
    pub(crate) fn new(client: Client) -> Self {
        Self { client }
    }

    /// Create a message on a thread.
    pub async fn create(
        &self,
        thread_id: &str,
        params: MessageCreateParams,
    ) -> Result<ThreadMessage, OpenAIError> {
        let body = serde_json::to_value(&params)?;
        self.client
            .execute(
                Method::POST,
                &format!("/threads/{thread_id}/messages"),
                with_beta(RequestOptions::json(body)),
            )
            .await
    }

    /// Retrieve a message.
    pub async fn retrieve(
        &self,
        thread_id: &str,
        message_id: &str,
    ) -> Result<ThreadMessage, OpenAIError> {
        self.client
            .execute(
                Method::GET,
                &format!("/threads/{thread_id}/messages/{message_id}"),
                with_beta(RequestOptions::default()),
            )
            .await
    }

    /// Update a message.
    pub async fn update(
        &self,
        thread_id: &str,
        message_id: &str,
        params: MessageUpdateParams,
    ) -> Result<ThreadMessage, OpenAIError> {
        let body = serde_json::to_value(&params)?;
        self.client
            .execute(
                Method::POST,
                &format!("/threads/{thread_id}/messages/{message_id}"),
                with_beta(RequestOptions::json(body)),
            )
            .await
    }

    /// Delete a message.
    pub async fn delete(
        &self,
        thread_id: &str,
        message_id: &str,
    ) -> Result<MessageDeleted, OpenAIError> {
        self.client
            .execute(
                Method::DELETE,
                &format!("/threads/{thread_id}/messages/{message_id}"),
                with_beta(RequestOptions::default()),
            )
            .await
    }

    /// List messages on a thread (a single page).
    pub async fn list(
        &self,
        thread_id: &str,
        params: Option<ListParams>,
    ) -> Result<List<ThreadMessage>, OpenAIError> {
        let query = params.unwrap_or_default().to_query();
        self.client
            .execute(
                Method::GET,
                &format!("/threads/{thread_id}/messages"),
                with_beta(RequestOptions::query(query)),
            )
            .await
    }

    /// List all messages on a thread, following pagination cursors.
    pub async fn list_all(
        &self,
        thread_id: &str,
        params: Option<ListParams>,
    ) -> Result<Vec<ThreadMessage>, OpenAIError> {
        let query = params.unwrap_or_default().to_query();
        paginate_all_beta(
            &self.client,
            &format!("/threads/{thread_id}/messages"),
            query,
        )
        .await
    }
}

// ---------------------------------------------------------------------------
// Runs
// ---------------------------------------------------------------------------

/// The runs resource: `client.threads().runs()`. Methods take the owning
/// `thread_id` as their first argument.
#[derive(Debug, Clone)]
pub struct Runs {
    client: Client,
}

impl Runs {
    pub(crate) fn new(client: Client) -> Self {
        Self { client }
    }

    /// Nested run steps resource: `client.threads().runs().steps()`.
    pub fn steps(&self) -> RunSteps {
        RunSteps::new(self.client.clone())
    }

    /// Create (start) a run on a thread. Streaming is out of scope.
    pub async fn create(
        &self,
        thread_id: &str,
        params: RunCreateParams,
    ) -> Result<Run, OpenAIError> {
        let body = serde_json::to_value(&params)?;
        self.client
            .execute(
                Method::POST,
                &format!("/threads/{thread_id}/runs"),
                with_beta(RequestOptions::json(body)),
            )
            .await
    }

    /// Retrieve a run.
    pub async fn retrieve(&self, thread_id: &str, run_id: &str) -> Result<Run, OpenAIError> {
        self.client
            .execute(
                Method::GET,
                &format!("/threads/{thread_id}/runs/{run_id}"),
                with_beta(RequestOptions::default()),
            )
            .await
    }

    /// Update a run.
    pub async fn update(
        &self,
        thread_id: &str,
        run_id: &str,
        params: RunUpdateParams,
    ) -> Result<Run, OpenAIError> {
        let body = serde_json::to_value(&params)?;
        self.client
            .execute(
                Method::POST,
                &format!("/threads/{thread_id}/runs/{run_id}"),
                with_beta(RequestOptions::json(body)),
            )
            .await
    }

    /// List runs on a thread (a single page).
    pub async fn list(
        &self,
        thread_id: &str,
        params: Option<ListParams>,
    ) -> Result<List<Run>, OpenAIError> {
        let query = params.unwrap_or_default().to_query();
        self.client
            .execute(
                Method::GET,
                &format!("/threads/{thread_id}/runs"),
                with_beta(RequestOptions::query(query)),
            )
            .await
    }

    /// List all runs on a thread, following pagination cursors.
    pub async fn list_all(
        &self,
        thread_id: &str,
        params: Option<ListParams>,
    ) -> Result<Vec<Run>, OpenAIError> {
        let query = params.unwrap_or_default().to_query();
        paginate_all_beta(&self.client, &format!("/threads/{thread_id}/runs"), query).await
    }

    /// Cancel a run that is in progress.
    pub async fn cancel(&self, thread_id: &str, run_id: &str) -> Result<Run, OpenAIError> {
        self.client
            .execute(
                Method::POST,
                &format!("/threads/{thread_id}/runs/{run_id}/cancel"),
                with_beta(RequestOptions::default()),
            )
            .await
    }

    /// Submit tool outputs for a run awaiting action. Streaming is out of scope.
    pub async fn submit_tool_outputs(
        &self,
        thread_id: &str,
        run_id: &str,
        params: SubmitToolOutputsParams,
    ) -> Result<Run, OpenAIError> {
        let body = serde_json::to_value(&params)?;
        self.client
            .execute(
                Method::POST,
                &format!("/threads/{thread_id}/runs/{run_id}/submit_tool_outputs"),
                with_beta(RequestOptions::json(body)),
            )
            .await
    }
}

// ---------------------------------------------------------------------------
// Run steps
// ---------------------------------------------------------------------------

/// The run steps resource: `client.threads().runs().steps()`. Methods take the
/// owning `thread_id` and `run_id` as their first arguments.
#[derive(Debug, Clone)]
pub struct RunSteps {
    client: Client,
}

impl RunSteps {
    pub(crate) fn new(client: Client) -> Self {
        Self { client }
    }

    /// Retrieve a run step.
    pub async fn retrieve(
        &self,
        thread_id: &str,
        run_id: &str,
        step_id: &str,
    ) -> Result<RunStep, OpenAIError> {
        self.client
            .execute(
                Method::GET,
                &format!("/threads/{thread_id}/runs/{run_id}/steps/{step_id}"),
                with_beta(RequestOptions::default()),
            )
            .await
    }

    /// List steps for a run (a single page).
    pub async fn list(
        &self,
        thread_id: &str,
        run_id: &str,
        params: Option<ListParams>,
    ) -> Result<List<RunStep>, OpenAIError> {
        let query = params.unwrap_or_default().to_query();
        self.client
            .execute(
                Method::GET,
                &format!("/threads/{thread_id}/runs/{run_id}/steps"),
                with_beta(RequestOptions::query(query)),
            )
            .await
    }

    /// List all steps for a run, following pagination cursors.
    pub async fn list_all(
        &self,
        thread_id: &str,
        run_id: &str,
        params: Option<ListParams>,
    ) -> Result<Vec<RunStep>, OpenAIError> {
        let query = params.unwrap_or_default().to_query();
        paginate_all_beta(
            &self.client,
            &format!("/threads/{thread_id}/runs/{run_id}/steps"),
            query,
        )
        .await
    }
}
