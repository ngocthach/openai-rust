//! Audio types, mirroring `openai-python/src/openai/types/audio/`.

use serde::{Deserialize, Serialize};

/// Voice for text-to-speech: a built-in voice name (`"alloy"`, `"nova"`, ...)
/// or a custom voice referenced by id (`{"id": "voice_123"}`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(untagged)]
pub enum SpeechVoice {
    Named(String),
    Custom { id: String },
}

impl SpeechVoice {
    /// A custom voice referenced by id.
    pub fn custom(id: impl Into<String>) -> Self {
        Self::Custom { id: id.into() }
    }
}

impl From<&str> for SpeechVoice {
    fn from(name: &str) -> Self {
        Self::Named(name.to_owned())
    }
}

impl From<&String> for SpeechVoice {
    fn from(name: &String) -> Self {
        Self::Named(name.clone())
    }
}

impl From<String> for SpeechVoice {
    fn from(name: String) -> Self {
        Self::Named(name)
    }
}

/// Request body for `POST /audio/speech` (text-to-speech).
#[derive(Debug, Clone, Serialize)]
pub struct SpeechRequest {
    /// Text to synthesize (max 4096 characters).
    pub input: String,
    /// e.g. `"tts-1"`, `"gpt-4o-mini-tts"`.
    pub model: String,
    /// e.g. `"alloy"`, `"nova"`, `"onyx"`, or [`SpeechVoice::custom`].
    pub voice: SpeechVoice,
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
        voice: impl Into<SpeechVoice>,
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
    /// Words or phrases to guide transcription; sent as repeated
    /// `keywords[]` multipart fields.
    pub keywords: Option<Vec<String>>,
    /// Possible input languages (ISO-639-1); sent as repeated `languages[]`
    /// multipart fields.
    pub languages: Option<Vec<String>>,
}

impl TranscriptionRequest {
    pub fn new(model: impl Into<String>) -> Self {
        Self {
            model: model.into(),
            language: None,
            prompt: None,
            response_format: None,
            temperature: None,
            keywords: None,
            languages: None,
        }
    }

    pub fn keywords<I, S>(mut self, keywords: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.keywords = Some(keywords.into_iter().map(Into::into).collect());
        self
    }

    pub fn languages<I, S>(mut self, languages: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.languages = Some(languages.into_iter().map(Into::into).collect());
        self
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
    /// Languages detected in the audio, when the model reports them.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub languages: Option<Vec<TranscriptionLanguage>>,
}

/// A language detected in transcribed audio.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct TranscriptionLanguage {
    pub code: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transcription_languages_parse_and_default_to_none() {
        let t: Transcription = serde_json::from_value(
            serde_json::json!({"text": "hi", "languages": [{"code": "en"}, {"code": "fr"}]}),
        )
        .unwrap();
        let codes: Vec<_> = t.languages.unwrap().into_iter().map(|l| l.code).collect();
        assert_eq!(codes, ["en", "fr"]);

        let plain: Transcription =
            serde_json::from_value(serde_json::json!({"text": "hi"})).unwrap();
        assert!(plain.languages.is_none());
    }

    #[test]
    fn speech_voice_accepts_string_reference() {
        let voice = String::from("nova");
        let request = SpeechRequest::new("tts-1", "hi", &voice);
        assert_eq!(serde_json::to_value(&request).unwrap()["voice"], "nova");
    }

    #[test]
    fn speech_voice_serializes_as_string_or_id_object() {
        let named = SpeechRequest::new("tts-1", "hi", "alloy");
        assert_eq!(serde_json::to_value(&named).unwrap()["voice"], "alloy");

        let custom = SpeechRequest::new("tts-1", "hi", SpeechVoice::custom("v_1"));
        assert_eq!(
            serde_json::to_value(&custom).unwrap()["voice"],
            serde_json::json!({"id": "v_1"})
        );
    }
}
