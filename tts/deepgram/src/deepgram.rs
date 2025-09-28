use golem_tts::{
    client::{ApiClient, TtsClient},
    config::get_env,
    golem::tts::{
        advanced::{
            AudioSample, LanguageCode, PronunciationEntry, PronunciationLexicon, Voice,
            VoiceDesignParams,
        },
        streaming::{
            SynthesisOptions, SynthesisStream, TextInput, TimingInfo, VoiceConversionStream,
        },
        synthesis::ValidationResult,
        types::{SynthesisMetadata, SynthesisResult, TtsError},
        voices::{LanguageInfo, VoiceFilter, VoiceGender, VoiceResults},
    },
    tts_stream::TtsStream,
};
use log::trace;
use reqwest::{header::HeaderMap, Method};
use serde::{Deserialize, Serialize};

use crate::{
    error::{from_http_error, unsupported},
    resources::{DeepgramVoice, DeepgramVoiceResults},
    utils::estimate_duration,
};

pub struct Deepgram {
    client: ApiClient,
}

impl Deepgram {
    pub fn new() -> Result<Self, TtsError> {
        let api_key = get_env("DEEPGRAM_API_KEY")?;
        let mut auth_headers = HeaderMap::new();
        auth_headers.insert(
            "Authorization",
            format!("Token {}", api_key).parse().unwrap(),
        );

        let base_url = get_env("TTS_PROVIDER_ENDPOINT")
            .ok()
            .unwrap_or("https://api.deepgram.com".to_string());
        trace!("Using base URL: {base_url}");
        let client = ApiClient::new(base_url, auth_headers)?;

        Ok(Self { client })
    }
}

impl TtsClient for Deepgram {
    /// voice should be canonical name.
    fn synthesize(
        &self,
        input: TextInput,
        voice: String,
        options: Option<SynthesisOptions>,
    ) -> Result<SynthesisResult, TtsError> {
        let query_params = DeepgramQueryParams {
            model: voice,
            encoding: options
                .as_ref()
                .and_then(|o| o.audio_config.as_ref())
                .map(|ac| ac.format.clone()),
            container: None,
            sample_rate: options
                .as_ref()
                .and_then(|o| o.audio_config.as_ref())
                .and_then(|ac| ac.sample_rate),
            bit_rate: options
                .as_ref()
                .and_then(|o| o.audio_config.as_ref())
                .and_then(|ac| ac.bit_rate),
        };

        trace!("Query: {:#?}", query_params);

        let request_body = SpeakRequest {
            text: input.content.clone(),
        };

        let path = "/v1/speak";
        let audio_data = self.client.retry_audio_request(
            Method::POST,
            path,
            &request_body,
            Some(&query_params),
            from_http_error,
        )?;

        let metadata = SynthesisMetadata {
            duration_seconds: estimate_duration(&input.content),
            character_count: input.content.len() as u32,
            word_count: input.content.split_whitespace().count() as u32,
            audio_size_bytes: audio_data.len() as u32,
            request_id: String::new(),
            provider_info: None,
        };

        Ok(SynthesisResult {
            audio_data,
            metadata,
        })
    }

    /// voice should be canonical name.
    fn synthesize_batch(
        &self,
        inputs: Vec<TextInput>,
        voice: String,
        options: Option<SynthesisOptions>,
    ) -> Result<Vec<SynthesisResult>, TtsError> {
        let mut results = Vec::with_capacity(inputs.len());
        for input in inputs {
            let result = self.synthesize(input, voice.clone(), options.clone())?;
            results.push(result);
        }

        Ok(results)
    }

    fn get_timing_marks(
        &self,
        _input: TextInput,
        _voice: String,
    ) -> Result<Vec<TimingInfo>, TtsError> {
        unsupported("Timing marks without audio synthesise is not supported by Deepgram TTS")
    }

    fn validate_input(
        &self,
        input: TextInput,
        voice: String,
    ) -> Result<golem_tts::golem::tts::synthesis::ValidationResult, TtsError> {
        let text = input.content;
        let mut warnings = Vec::new();
        let mut errors = Vec::new();

        // Check if text is empty
        if text.trim().is_empty() {
            errors.push("Text cannot be empty".to_string());
        }

        // Check text length (Deepgram has limits)
        if text.len() > 2000 {
            errors.push(
                "Text exceeds maximum length of 2000 characters for Deepgram TTS".to_string(),
            );
        }

        // Check if SSML is being used (Deepgram doesn't support SSML)
        if input.text_type == golem_tts::golem::tts::types::TextType::Ssml {
            errors
                .push("SSML is not supported by Deepgram TTS. Please use plain text.".to_string());
        }

        // Check voice validity
        if voice.is_empty() {
            errors.push("Voice model name cannot be empty".to_string());
        }

        // Warn about very long text
        if text.len() > 1000 {
            warnings.push("Long text may take significant time to synthesize".to_string());
        }

        Ok(ValidationResult {
            is_valid: errors.is_empty(),
            character_count: text.len() as u32,
            estimated_duration: Some(estimate_duration(&text)),
            warnings,
            errors,
        })
    }

    fn list_voices(&self, filter: Option<VoiceFilter>) -> Result<VoiceResults, TtsError> {
        let path = "/v1/models";
        let result = self.client.make_request::<ListVoiceResponse, (), (), _>(
            Method::GET,
            path,
            (),
            None,
            from_http_error,
        )?;
        let mut voices = result.tts;
        if let Some(fi) = filter {
            voices = voices
                .iter()
                .filter(|v| {
                    if let Some(language) = &fi.language {
                        return v.languages.contains(&language.to_string());
                    }
                    if let Some(gender) = &fi.gender {
                        let g = match gender {
                            VoiceGender::Male => "male",
                            VoiceGender::Female => "female",
                            VoiceGender::Neutral => "neutral",
                        };
                        return v.metadata.tags.contains(&g.to_string());
                    }
                    if let Some(quality) = fi.quality.clone() {
                        return v.metadata.tags.contains(&quality);
                    }

                    false
                })
                .cloned()
                .collect();
        }
        Ok(VoiceResults::new(DeepgramVoiceResults::new(voices)))
    }

    fn get_voice(&self, voice_id: String) -> Result<Voice, TtsError> {
        let path = format!("/v1/models/{}", voice_id);
        let voice = self.client.make_request::<DeepgramVoice, (), (), _>(
            Method::GET,
            &path,
            (),
            None,
            from_http_error,
        )?;
        Ok(Voice::new(voice))
    }

    fn list_languages(&self) -> Result<Vec<golem_tts::golem::tts::voices::LanguageInfo>, TtsError> {
        // Hardcoded languages with actual voice counts from TTS provider
        // Voice counts based on actual available voices as of current data

        let languages = vec![
            LanguageInfo {
                code: "en-US".to_string(),
                name: "English (US)".to_string(),
                native_name: "English (United States)".to_string(),
                voice_count: 45, // Actual count from voice data
            },
            LanguageInfo {
                code: "en-GB".to_string(),
                name: "English (UK)".to_string(),
                native_name: "English (United Kingdom)".to_string(),
                voice_count: 4,
            },
            LanguageInfo {
                code: "en-AU".to_string(),
                name: "English (AU)".to_string(),
                native_name: "English (Australia)".to_string(),
                voice_count: 2,
            },
            LanguageInfo {
                code: "en-IE".to_string(),
                name: "English (IE)".to_string(),
                native_name: "English (Ireland)".to_string(),
                voice_count: 1,
            },
            LanguageInfo {
                code: "en-PH".to_string(),
                name: "English (PH)".to_string(),
                native_name: "English (Philippines)".to_string(),
                voice_count: 1,
            },
            LanguageInfo {
                code: "es-ES".to_string(),
                name: "Spanish (ES)".to_string(),
                native_name: "Español (España)".to_string(),
                voice_count: 4,
            },
            LanguageInfo {
                code: "es-MX".to_string(),
                name: "Spanish (MX)".to_string(),
                native_name: "Español (México)".to_string(),
                voice_count: 3,
            },
            LanguageInfo {
                code: "es-419".to_string(), // Latin America region code
                name: "Spanish (LATAM)".to_string(),
                native_name: "Español (Latinoamérica)".to_string(),
                voice_count: 2,
            },
            LanguageInfo {
                code: "es-CO".to_string(),
                name: "Spanish (CO)".to_string(),
                native_name: "Español (Colombia)".to_string(),
                voice_count: 1,
            },
        ];
        Ok(languages)
    }

    fn create_voice_clone(
        &self,
        name: String,
        audio_samples: Vec<AudioSample>,
        description: Option<String>,
    ) -> Result<Voice, TtsError> {
        unsupported("Deepgram does not support voice cloning")
    }

    fn design_voice(
        &self,
        name: String,
        characteristics: VoiceDesignParams,
    ) -> Result<Voice, TtsError> {
        unsupported("Deepgram does not support voice design")
    }

    fn convert_voice(
        &self,
        input_audio: Vec<u8>,
        target_voice: String,
        preserve_timing: Option<bool>,
    ) -> Result<Vec<u8>, TtsError> {
        unsupported("Deepgram does not support voice conversion")
    }

    fn generate_sound_effect(
        &self,
        description: String,
        duration_seconds: Option<f32>,
        style_influence: Option<f32>,
    ) -> Result<Vec<u8>, TtsError> {
        unsupported("Deepgram does not support sound effect generation")
    }

    fn create_lexicon(
        &self,
        name: String,
        language: LanguageCode,
        entries: Option<Vec<PronunciationEntry>>,
    ) -> Result<PronunciationLexicon, TtsError> {
        unsupported("Deepgram does not support pronunciation lexicon")
    }

    fn synthesize_long_form(
        &self,
        content: String,
        voice: String,
        output_location: String,
        chapter_breaks: Option<Vec<u32>>,
    ) -> Result<golem_tts::golem::tts::advanced::LongFormOperation, TtsError> {
        unsupported("Deepgram does not  supported Async synthesis.")
    }

    fn create_stream(
        voice: String,
        options: Option<SynthesisOptions>,
    ) -> Result<SynthesisStream, TtsError> {
        let client = Deepgram::new()?;
        Ok(SynthesisStream::new(TtsStream::new(client, voice, options)))
    }

    fn create_voice_conversion_stream(
        target_voice: String,
        options: Option<SynthesisOptions>,
    ) -> Result<VoiceConversionStream, TtsError> {
        unsupported("Deepgram does not support voice conversion")
    }
}

#[derive(Serialize, Clone)]
pub struct SpeakRequest {
    pub text: String,
}

#[derive(Serialize, Debug, Clone)]
pub struct DeepgramQueryParams {
    pub model: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub encoding: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub container: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sample_rate: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bit_rate: Option<u32>,
}

#[derive(Clone, Deserialize)]
struct ListVoiceResponse {
    tts: Vec<DeepgramVoice>,
}
