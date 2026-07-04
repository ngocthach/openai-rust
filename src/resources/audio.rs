//! `/audio` resources, mirroring `resources/audio/speech.py` and
//! `resources/audio/transcriptions.py`.

use bytes::Bytes;
use reqwest::Method;

use crate::client::Client;
use crate::error::OpenAIError;
use crate::request::{MultipartField, RequestOptions};
use crate::types::audio::{SpeechRequest, Transcription, TranscriptionRequest};
use crate::types::files::FileUpload;

/// Accessor for audio endpoints: `client.audio().speech(...)`.
#[derive(Debug, Clone)]
pub struct Audio {
    client: Client,
}

impl Audio {
    pub(crate) fn new(client: Client) -> Self {
        Self { client }
    }

    /// Text-to-speech: returns the raw audio bytes.
    pub async fn speech(&self, request: SpeechRequest) -> Result<Bytes, OpenAIError> {
        let body = serde_json::to_value(&request)?;
        self.client
            .execute_bytes(Method::POST, "/audio/speech", RequestOptions::json(body))
            .await
    }

    /// Speech-to-text: transcribe an audio file (multipart upload).
    pub async fn transcriptions(
        &self,
        file: FileUpload,
        request: TranscriptionRequest,
    ) -> Result<Transcription, OpenAIError> {
        let mut fields = vec![
            MultipartField::File {
                name: "file".into(),
                filename: file.filename,
                bytes: file.bytes,
                content_type: file.content_type,
            },
            MultipartField::Text {
                name: "model".into(),
                value: request.model,
            },
        ];
        if let Some(language) = request.language {
            fields.push(MultipartField::Text {
                name: "language".into(),
                value: language,
            });
        }
        if let Some(prompt) = request.prompt {
            fields.push(MultipartField::Text {
                name: "prompt".into(),
                value: prompt,
            });
        }
        if let Some(response_format) = request.response_format {
            fields.push(MultipartField::Text {
                name: "response_format".into(),
                value: response_format,
            });
        }
        if let Some(temperature) = request.temperature {
            fields.push(MultipartField::Text {
                name: "temperature".into(),
                value: temperature.to_string(),
            });
        }
        self.client
            .execute(
                Method::POST,
                "/audio/transcriptions",
                RequestOptions::multipart(fields),
            )
            .await
    }
}
