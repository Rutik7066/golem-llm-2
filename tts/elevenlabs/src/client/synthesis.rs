use golem_tts::golem::tts::{
    streaming::{SynthesisOptions, TextInput},
    types::{SynthesisMetadata, SynthesisResult, TtsError},
};
use reqwest::Method;
use serde::Serialize;

use crate::client::{voices::ElVoice, ElevenLabsClient};

#[derive(Serialize, Clone, Debug)]
pub struct SynthesisRequest {
    pub text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_format: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language_code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub voice_settings: Option<SynthesisVoiceSettings>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pronunciation_dictionary_locators: Option<Vec<PronunciationDictionaryLocator>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seed: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub previous_text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub apply_text_normalization: Option<String>, // "auto", "on", "off"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub previous_request_ids: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_request_ids: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub apply_language_text_normalization: Option<bool>,
}

#[derive(Serialize, Clone, Debug)]
pub struct SynthesisVoiceSettings {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stability: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub similarity_boost: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub style: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub use_speaker_boost: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub speed: Option<f32>,
}

#[derive(Serialize, Clone, Debug)]
pub struct PronunciationDictionaryLocator {
    pub pronunciation_dictionary_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version_id: Option<String>,
}

impl ElevenLabsClient {
    pub fn synthesize(
        &self,
        input: TextInput,
        voice: &ElVoice,
        options: Option<SynthesisOptions>,
        previous_text: Option<String>,
        next_text: Option<String>,
    ) -> Result<SynthesisResult, TtsError> {
        let text_content = input.content.clone();
        let mut body = SynthesisRequest {
            text: text_content,
            model_id: None,
            output_format: None,
            language_code: input.language,
            voice_settings: None,
            pronunciation_dictionary_locators: None,
            seed: None,
            previous_text,
            next_text,
            apply_text_normalization: None,
            previous_request_ids: None,
            next_request_ids: None,
            apply_language_text_normalization: None,
        };

        if let Some(opt) = options {
            body.model_id = opt.model_version;
            body.seed = opt.seed.map(|s| s as i32);

            if let Some(audio_config) = opt.audio_config {
                body.output_format = Some(audio_config.format);
            }

            if let Some(voice_settings) = opt.voice_settings {
                body.voice_settings = Some(SynthesisVoiceSettings {
                    stability: voice_settings.stability,
                    similarity_boost: voice_settings.similarity,
                    style: voice_settings.style,
                    use_speaker_boost: voice_settings.volume.map(|v| v > 0.0),
                    speed: voice_settings.speed,
                });
            }
        }

        let path = format!("/v1/text-to-speech/{}", voice.voice_id);

        let result = self.request_binary(Method::POST, &path, Some(body), None::<String>)?;
        Ok(SynthesisResult {
            audio_data: result.clone(),
            metadata: SynthesisMetadata {
                duration_seconds: 0.0,
                character_count: input.content.len() as u32,
                word_count: 0,
                audio_size_bytes: result.len() as u32,
                request_id: "".to_string(),
                provider_info: Some("ElevenLabs".to_string()),
            },
        })
    }

    pub fn synthesize_batch(
        &self,
        inputs: Vec<TextInput>,
        voice: &ElVoice,
        options: Option<SynthesisOptions>,
    ) -> Result<Vec<SynthesisResult>, TtsError> {
        let mut results = Vec::with_capacity(inputs.len());

        for input in inputs.clone() {
            let result = self.synthesize(input, voice, options.clone(), None, None)?;
            results.push(result);
        }

        Ok(results)
    }
}
