//! Fine-tuning job types, mirroring
//! `openai-python/src/openai/types/fine_tuning/`.

use std::collections::HashMap;

use serde::{Deserialize, Deserializer, Serialize};

use crate::pagination::HasId;

/// A hyperparameter value that is either the string `"auto"` or a concrete
/// number, mirroring the `"auto" | number` unions in the fine-tuning API.
///
/// Serializes [`AutoOr::Auto`] as the literal string `"auto"` and
/// [`AutoOr::Value`] as the wrapped value.
#[derive(Debug, Clone, PartialEq)]
pub enum AutoOr<T> {
    /// Let the platform choose the value.
    Auto,
    /// An explicit value.
    Value(T),
}

impl<T: Serialize> Serialize for AutoOr<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            AutoOr::Auto => serializer.serialize_str("auto"),
            AutoOr::Value(value) => value.serialize(serializer),
        }
    }
}

impl<'de, T: Deserialize<'de>> Deserialize<'de> for AutoOr<T> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = serde_json::Value::deserialize(deserializer)?;
        if value.as_str() == Some("auto") {
            Ok(AutoOr::Auto)
        } else {
            T::deserialize(value)
                .map(AutoOr::Value)
                .map_err(serde::de::Error::custom)
        }
    }
}

// ---------------------------------------------------------------------------
// Hyperparameters
// ---------------------------------------------------------------------------

/// Supervised (and deprecated top-level) hyperparameters. Each field is
/// `"auto"` or a number.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Hyperparameters {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub batch_size: Option<AutoOr<i64>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub learning_rate_multiplier: Option<AutoOr<f64>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub n_epochs: Option<AutoOr<i64>>,
}

/// DPO method hyperparameters.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DpoHyperparameters {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub batch_size: Option<AutoOr<i64>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub beta: Option<AutoOr<f64>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub learning_rate_multiplier: Option<AutoOr<f64>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub n_epochs: Option<AutoOr<i64>>,
}

/// Reinforcement method hyperparameters.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ReinforcementHyperparameters {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub batch_size: Option<AutoOr<i64>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub compute_multiplier: Option<AutoOr<f64>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub eval_interval: Option<AutoOr<i64>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub eval_samples: Option<AutoOr<i64>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub learning_rate_multiplier: Option<AutoOr<f64>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub n_epochs: Option<AutoOr<i64>>,
    /// `"default" | "low" | "medium" | "high"`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reasoning_effort: Option<String>,
}

// ---------------------------------------------------------------------------
// Method
// ---------------------------------------------------------------------------

/// Supervised fine-tuning method configuration.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SupervisedMethod {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hyperparameters: Option<Hyperparameters>,
}

/// DPO fine-tuning method configuration.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DpoMethod {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hyperparameters: Option<DpoHyperparameters>,
}

/// Reinforcement fine-tuning method configuration. The `grader` is a deeply
/// polymorphic object left as raw JSON.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReinforcementMethod {
    pub grader: serde_json::Value,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hyperparameters: Option<ReinforcementHyperparameters>,
}

/// The fine-tuning method: a discriminated `type` plus its matching config.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FineTuningMethod {
    /// `"supervised" | "dpo" | "reinforcement"`.
    #[serde(rename = "type")]
    pub method_type: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub supervised: Option<SupervisedMethod>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dpo: Option<DpoMethod>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reinforcement: Option<ReinforcementMethod>,
}

impl FineTuningMethod {
    /// A supervised method.
    pub fn supervised(config: SupervisedMethod) -> Self {
        Self {
            method_type: "supervised".into(),
            supervised: Some(config),
            dpo: None,
            reinforcement: None,
        }
    }

    /// A DPO method.
    pub fn dpo(config: DpoMethod) -> Self {
        Self {
            method_type: "dpo".into(),
            supervised: None,
            dpo: Some(config),
            reinforcement: None,
        }
    }

    /// A reinforcement method.
    pub fn reinforcement(config: ReinforcementMethod) -> Self {
        Self {
            method_type: "reinforcement".into(),
            supervised: None,
            dpo: None,
            reinforcement: Some(config),
        }
    }
}

// ---------------------------------------------------------------------------
// Integrations
// ---------------------------------------------------------------------------

/// Weights & Biases integration settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WandbIntegration {
    pub project: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub entity: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
}

/// A training integration (currently only `wandb`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Integration {
    /// Integration type, e.g. `"wandb"`.
    #[serde(rename = "type")]
    pub integration_type: String,
    pub wandb: WandbIntegration,
}

impl Integration {
    /// A `wandb` integration for the given project.
    pub fn wandb(project: impl Into<String>) -> Self {
        Self {
            integration_type: "wandb".into(),
            wandb: WandbIntegration {
                project: project.into(),
                name: None,
                entity: None,
                tags: None,
            },
        }
    }
}

// ---------------------------------------------------------------------------
// Create request
// ---------------------------------------------------------------------------

/// Request body for `POST /fine_tuning/jobs`
/// (`job_create_params.py`).
#[derive(Debug, Clone, Serialize)]
pub struct FineTuningJobRequest {
    pub model: String,
    pub training_file: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suffix: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub validation_file: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seed: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, String>>,
    /// Deprecated: prefer [`FineTuningJobRequest::method`].
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hyperparameters: Option<Hyperparameters>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub method: Option<FineTuningMethod>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub integrations: Option<Vec<Integration>>,
}

impl FineTuningJobRequest {
    /// Create a request with the required `model` and `training_file`.
    pub fn new(model: impl Into<String>, training_file: impl Into<String>) -> Self {
        Self {
            model: model.into(),
            training_file: training_file.into(),
            suffix: None,
            validation_file: None,
            seed: None,
            metadata: None,
            hyperparameters: None,
            method: None,
            integrations: None,
        }
    }

    pub fn suffix(mut self, suffix: impl Into<String>) -> Self {
        self.suffix = Some(suffix.into());
        self
    }

    pub fn validation_file(mut self, validation_file: impl Into<String>) -> Self {
        self.validation_file = Some(validation_file.into());
        self
    }

    pub fn seed(mut self, seed: i64) -> Self {
        self.seed = Some(seed);
        self
    }

    pub fn metadata(mut self, metadata: HashMap<String, String>) -> Self {
        self.metadata = Some(metadata);
        self
    }

    pub fn hyperparameters(mut self, hyperparameters: Hyperparameters) -> Self {
        self.hyperparameters = Some(hyperparameters);
        self
    }

    pub fn method(mut self, method: FineTuningMethod) -> Self {
        self.method = Some(method);
        self
    }

    pub fn integrations(mut self, integrations: Vec<Integration>) -> Self {
        self.integrations = Some(integrations);
        self
    }
}

// ---------------------------------------------------------------------------
// List params
// ---------------------------------------------------------------------------

/// Query parameters for listing fine-tuning jobs.
#[derive(Debug, Clone, Default)]
pub struct FineTuningJobListParams {
    pub after: Option<String>,
    pub limit: Option<u32>,
    /// Metadata filters, serialized as `metadata[key]=value`.
    pub metadata: Option<HashMap<String, String>>,
}

impl FineTuningJobListParams {
    pub(crate) fn to_query(&self) -> Vec<(String, String)> {
        let mut query =
            crate::pagination::cursor_query(self.after.as_deref(), None, self.limit, None);
        if let Some(metadata) = &self.metadata {
            for (key, value) in metadata {
                query.push((format!("metadata[{key}]"), value.clone()));
            }
        }
        query
    }
}

/// Query parameters for listing events or checkpoints (cursor + limit).
#[derive(Debug, Clone, Default)]
pub struct FineTuningPageParams {
    pub after: Option<String>,
    pub limit: Option<u32>,
}

impl FineTuningPageParams {
    pub(crate) fn to_query(&self) -> Vec<(String, String)> {
        crate::pagination::cursor_query(self.after.as_deref(), None, self.limit, None)
    }
}

// ---------------------------------------------------------------------------
// Responses
// ---------------------------------------------------------------------------

/// Failure details attached to a fine-tuning job.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct FineTuningJobError {
    #[serde(default)]
    pub code: Option<String>,
    #[serde(default)]
    pub message: Option<String>,
    #[serde(default)]
    pub param: Option<String>,
}

/// The lifecycle status of a fine-tuning job.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FineTuningJobStatus {
    ValidatingFiles,
    Queued,
    Running,
    Succeeded,
    Failed,
    Cancelled,
    /// A status not known to this client version.
    #[serde(other)]
    Unknown,
}

/// A fine-tuning job (`fine_tuning_job.py`).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct FineTuningJob {
    pub id: String,
    #[serde(default)]
    pub created_at: i64,
    #[serde(default)]
    pub error: Option<FineTuningJobError>,
    #[serde(default)]
    pub fine_tuned_model: Option<String>,
    #[serde(default)]
    pub finished_at: Option<i64>,
    #[serde(default)]
    pub hyperparameters: Option<Hyperparameters>,
    #[serde(default)]
    pub model: String,
    #[serde(default)]
    pub object: String,
    #[serde(default)]
    pub organization_id: Option<String>,
    #[serde(default)]
    pub result_files: Vec<String>,
    #[serde(default)]
    pub seed: Option<i64>,
    #[serde(default)]
    pub status: Option<FineTuningJobStatus>,
    #[serde(default)]
    pub trained_tokens: Option<i64>,
    #[serde(default)]
    pub training_file: String,
    #[serde(default)]
    pub validation_file: Option<String>,
    #[serde(default)]
    pub estimated_finish: Option<i64>,
    #[serde(default)]
    pub integrations: Option<Vec<serde_json::Value>>,
    #[serde(default)]
    pub metadata: Option<HashMap<String, String>>,
    #[serde(default)]
    pub method: Option<FineTuningMethod>,
}

impl HasId for FineTuningJob {
    fn id(&self) -> Option<&str> {
        Some(&self.id)
    }
}

/// An event emitted while a fine-tuning job runs.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct FineTuningJobEvent {
    pub id: String,
    #[serde(default)]
    pub created_at: i64,
    /// `"info" | "warn" | "error"`.
    #[serde(default)]
    pub level: Option<String>,
    #[serde(default)]
    pub message: Option<String>,
    #[serde(default)]
    pub object: String,
    /// `"message" | "metrics"`.
    #[serde(default, rename = "type")]
    pub event_type: Option<String>,
    #[serde(default)]
    pub data: Option<serde_json::Value>,
}

impl HasId for FineTuningJobEvent {
    fn id(&self) -> Option<&str> {
        Some(&self.id)
    }
}

/// Metrics captured at a fine-tuning checkpoint.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[non_exhaustive]
pub struct FineTuningJobCheckpointMetrics {
    #[serde(default)]
    pub full_valid_loss: Option<f64>,
    #[serde(default)]
    pub full_valid_mean_token_accuracy: Option<f64>,
    #[serde(default)]
    pub step: Option<f64>,
    #[serde(default)]
    pub train_loss: Option<f64>,
    #[serde(default)]
    pub train_mean_token_accuracy: Option<f64>,
    #[serde(default)]
    pub valid_loss: Option<f64>,
    #[serde(default)]
    pub valid_mean_token_accuracy: Option<f64>,
}

/// A checkpoint produced during a fine-tuning job.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct FineTuningJobCheckpoint {
    pub id: String,
    #[serde(default)]
    pub created_at: i64,
    #[serde(default)]
    pub fine_tuned_model_checkpoint: String,
    #[serde(default)]
    pub fine_tuning_job_id: String,
    #[serde(default)]
    pub metrics: Option<FineTuningJobCheckpointMetrics>,
    #[serde(default)]
    pub object: String,
    #[serde(default)]
    pub step_number: i64,
}

impl HasId for FineTuningJobCheckpoint {
    fn id(&self) -> Option<&str> {
        Some(&self.id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auto_or_serializes_auto_as_string() {
        assert_eq!(
            serde_json::to_value(AutoOr::<i64>::Auto).unwrap(),
            serde_json::json!("auto")
        );
    }

    #[test]
    fn auto_or_serializes_value_as_number() {
        assert_eq!(
            serde_json::to_value(AutoOr::Value(3_i64)).unwrap(),
            serde_json::json!(3)
        );
        assert_eq!(
            serde_json::to_value(AutoOr::Value(0.5_f64)).unwrap(),
            serde_json::json!(0.5)
        );
    }

    #[test]
    fn auto_or_deserializes_auto_and_number() {
        let auto: AutoOr<i64> = serde_json::from_value(serde_json::json!("auto")).unwrap();
        assert_eq!(auto, AutoOr::Auto);
        let value: AutoOr<i64> = serde_json::from_value(serde_json::json!(4)).unwrap();
        assert_eq!(value, AutoOr::Value(4));
        let float: AutoOr<f64> = serde_json::from_value(serde_json::json!(0.25)).unwrap();
        assert_eq!(float, AutoOr::Value(0.25));
    }

    #[test]
    fn status_unknown_falls_back() {
        let status: FineTuningJobStatus =
            serde_json::from_value(serde_json::json!("brand_new_status")).unwrap();
        assert_eq!(status, FineTuningJobStatus::Unknown);
        let running: FineTuningJobStatus =
            serde_json::from_value(serde_json::json!("running")).unwrap();
        assert_eq!(running, FineTuningJobStatus::Running);
    }
}
