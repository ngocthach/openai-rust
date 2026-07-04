//! Audio types, mirroring `openai-python/src/openai/types/audio/`.

use serde::{Deserialize, Serialize};

/// Request body for `POST /audio/speech` (text-to-speech).
#[derive(Debug, Clone, Serialize)]
pub struct SpeechRequest {
    /// Text to synthesize (max 4096 characters).
    pub input: String,
    /// e.g. `"tts-1"`, `"gpt-4o-mini-tts"`.
    pub model: String,
    /// e.g. `"alloy"`, `"nova"`, `"onyx"`.
    pub voice: String,
    /// `"mp3"` (default), `"opus"`, `"aac"`, `"flac"`, `"wav"`, `"pcm"`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_format: Option<String>,
    /// 0.25 to 4.0, default 1.0.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub speed: Option<f64>,
    /// Voice direction (newer TTS models only).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instructions: Option<String>,
}

impl SpeechRequest {
    pub fn new(
        model: impl Into<String>,
        input: impl Into<String>,
        voice: impl Into<String>,
    ) -> Self {
        Self {
            input: input.into(),
            model: model.into(),
            voice: voice.into(),
            response_format: None,
            speed: None,
            instructions: None,
        }
    }

    pub fn response_format(mut self, response_format: impl Into<String>) -> Self {
        self.response_format = Some(response_format.into());
        self
    }

    pub fn speed(mut self, speed: f64) -> Self {
        self.speed = Some(speed);
        self
    }

    /// Voice direction, e.g. "speak slowly and warmly" (newer TTS models).
    pub fn instructions(mut self, instructions: impl Into<String>) -> Self {
        self.instructions = Some(instructions.into());
        self
    }
}

/// Parameters for `POST /audio/transcriptions` (multipart; the file itself is
/// passed separately as a [`crate::types::files::FileUpload`]).
#[derive(Debug, Clone)]
pub struct TranscriptionRequest {
    /// e.g. `"whisper-1"`, `"gpt-4o-transcribe"`.
    pub model: String,
    /// ISO-639-1 input language hint.
    pub language: Option<String>,
    /// Optional text to guide style or continue a previous segment.
    pub prompt: Option<String>,
    /// `"json"` (default), `"text"`, `"srt"`, `"verbose_json"`, `"vtt"`.
    /// v0.1 parses the default `json` shape (`{ "text": ... }`).
    pub response_format: Option<String>,
    pub temperature: Option<f64>,
}

impl TranscriptionRequest {
    pub fn new(model: impl Into<String>) -> Self {
        Self {
            model: model.into(),
            language: None,
            prompt: None,
            response_format: None,
            temperature: None,
        }
    }

    pub fn language(mut self, language: impl Into<String>) -> Self {
        self.language = Some(language.into());
        self
    }

    pub fn prompt(mut self, prompt: impl Into<String>) -> Self {
        self.prompt = Some(prompt.into());
        self
    }

    pub fn temperature(mut self, temperature: f64) -> Self {
        self.temperature = Some(temperature);
        self
    }

    /// `"json"` (default), `"verbose_json"`, or — via
    /// `Audio::transcriptions_raw` — `"text"`, `"srt"`, `"vtt"`.
    pub fn response_format(mut self, response_format: impl Into<String>) -> Self {
        self.response_format = Some(response_format.into());
        self
    }
}

/// Response from `POST /audio/transcriptions` (default `json` format).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Transcription {
    pub text: String,
}
