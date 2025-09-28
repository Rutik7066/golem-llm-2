use golem_tts::{
    client::{ApiClient, TtsClient},
    config::get_env,
    golem::tts::{
        advanced::{LongFormOperation, PronunciationLexicon, TtsError},
        streaming::SynthesisStream,
        synthesis::ValidationResult,
        types::{SynthesisMetadata, SynthesisResult, TextType},
        voices::{LanguageInfo, Voice, VoiceResults},
    },
    tts_stream::TtsStream,
};
use log::trace;
use reqwest::Method;
use serde::Serialize;

use crate::{
    error::{from_http_error, unsupported},
    resources::{AwsLongFormOperation, AwsPronunciationLexicon, AwsVoiceResults},
    types::{
        GetLexiconResponse, ListVoiceParam, ListVoiceResponse, PutLexiconRequest,
        StartSpeechSynthesisTaskRequest, StartSpeechSynthesisTaskResponse, SynthesizeSpeechParams,
        SynthesizeSpeechResponse,
    },
    utils::{create_pls_content, estimate_text_duration, parse_s3_location},
};

#[derive(Clone)]
pub struct Polly {
    pub access_key_id: String,
    pub secret_access_key: String,
    pub region: String,
    pub base_url: String,
}

impl Polly {
    pub fn new() -> Result<Self, TtsError> {
        let access_key_id = get_env("AWS_ACCESS_KEY_ID")?;
        let secret_access_key = get_env("AWS_SECRET_ACCESS_KEY")?;
        let region = get_env("AWS_REGION")?;
        let base_url = get_env("TTS_PROVIDER_ENDPOINT")
            .ok()
            .unwrap_or(format!("https://polly.{}.amazonaws.com", region));

        Ok(Self {
            access_key_id,
            secret_access_key,
            region,
            base_url,
        })
    }
    pub fn get_client<B: Serialize>(
        &self,
        method: &str,
        path: &str,
        body: Option<&B>,
    ) -> Result<ApiClient, TtsError> {
        let str_body = if let Some(body) = body {
            serde_json::to_string(body)
        } else {
            Ok("".to_string())
        }
        .map_err(|e| TtsError::RequestError(format!("Failed to serialize request: {}", e)))?;

        let auth_headers = self
            .generate_sigv4_headers(method, path, &str_body)
            .map_err(|e| TtsError::InternalError(format!("Failed to sign request: {:?}", e)))?;

        ApiClient::new(self.base_url.clone(), auth_headers)
    }
}

impl TtsClient for Polly {
    fn create_stream(
        voice: String,
        options: Option<golem_tts::golem::tts::streaming::SynthesisOptions>,
    ) -> Result<
        golem_tts::golem::tts::streaming::SynthesisStream,
        golem_tts::golem::tts::types::TtsError,
    > {
        let client = Polly::new()?;
        Ok(SynthesisStream::new(TtsStream::new(client, voice, options)))
    }

    fn create_voice_conversion_stream(
        target_voice: String,
        options: Option<golem_tts::golem::tts::streaming::SynthesisOptions>,
    ) -> Result<
        golem_tts::golem::tts::streaming::VoiceConversionStream,
        golem_tts::golem::tts::types::TtsError,
    > {
        unsupported("Voice conversion is not supported by AWS Polly")
    }

    fn synthesize(
        &self,
        input: golem_tts::golem::tts::streaming::TextInput,
        voice: String,
        options: Option<golem_tts::golem::tts::streaming::SynthesisOptions>,
    ) -> Result<
        golem_tts::golem::tts::synthesis::SynthesisResult,
        golem_tts::golem::tts::types::TtsError,
    > {
        let mut body = if let Some(options) = options {
            let output_format = options.audio_config.as_ref().map(|c| c.format.clone());
            let sample_rate = if let Some(audio_config) = options.audio_config {
                audio_config.sample_rate.map(|rate| rate.to_string())
            } else {
                None
            };
            SynthesizeSpeechParams {
                engine: None,
                language_code: None,
                lexicon_names: None,
                output_format,
                sample_rate,
                speech_mark_types: None,
                text: "".to_string(),
                text_type: None,
                voice_id: voice.clone(),
            }
        } else {
            SynthesizeSpeechParams {
                engine: None,
                language_code: None,
                lexicon_names: None,
                output_format: None,
                sample_rate: None,
                speech_mark_types: None,
                text: "".to_string(),
                text_type: None,
                voice_id: voice.clone(),
            }
        };

        body.text = input.content;
        body.text_type = if input.text_type == TextType::Ssml {
            Some("ssml".to_string())
        } else {
            Some("text".to_string())
        };

        let client = self.get_client("POST", "/v1/speech", Some(&body))?;

        let response = client
            .make_request::<SynthesizeSpeechResponse, SynthesizeSpeechParams, (), _>(
                Method::POST,
                "/v1/speech",
                body,
                None,
                from_http_error,
            )?;

        let audio_size_bytes = response.audio_stream.len() as u32;
        Ok(SynthesisResult {
            audio_data: response.audio_stream,
            metadata: SynthesisMetadata {
                duration_seconds: 0.0,
                character_count: response.request_characters,
                word_count: 0,
                audio_size_bytes,
                request_id: "".to_string(),
                provider_info: Some("AWS Polly".to_string()),
            },
        })
    }

    fn synthesize_batch(
        &self,
        inputs: Vec<golem_tts::golem::tts::streaming::TextInput>,
        voice: String,
        options: Option<golem_tts::golem::tts::streaming::SynthesisOptions>,
    ) -> Result<
        Vec<golem_tts::golem::tts::synthesis::SynthesisResult>,
        golem_tts::golem::tts::types::TtsError,
    > {
        let mut results = Vec::with_capacity(inputs.len());
        for input in inputs {
            let result = self.synthesize(input, voice.clone(), options.clone())?;
            results.push(result);
        }

        Ok(results)
    }

    fn get_timing_marks(
        &self,
        input: golem_tts::golem::tts::streaming::TextInput,
        voice: String,
    ) -> Result<
        Vec<golem_tts::golem::tts::streaming::TimingInfo>,
        golem_tts::golem::tts::types::TtsError,
    > {
        unsupported("Timing marks without audio synthesise is not supported by AWS Polly")
    }

    fn validate_input(
        &self,
        input: golem_tts::golem::tts::streaming::TextInput,
        voice: String,
    ) -> Result<
        golem_tts::golem::tts::synthesis::ValidationResult,
        golem_tts::golem::tts::types::TtsError,
    > {
        let text = input.content;
        let mut warnings = Vec::new();
        let mut errors = Vec::new();

        if text.len() > 3000 {
            errors.push("Text exceeds maximum length of 3000 characters for AWS Polly".to_string());
        }

        if text.trim_start().starts_with('<')
            && (!text.contains("</speak>") || !text.contains("<speak"))
        {
            errors.push("Invalid SSML format - missing speak tags".to_string());
        }

        if text.trim().is_empty() {
            errors.push("Text cannot be empty".to_string());
        }

        if voice.is_empty() {
            errors.push("Voice ID cannot be empty".to_string());
        }

        if text.chars().any(|c| c as u32 > 127) && text.trim_start().starts_with('<') {
            warnings.push("Non-ASCII characters in SSML may cause issues".to_string());
        }

        Ok(ValidationResult {
            is_valid: errors.is_empty(),
            errors,
            warnings,
            character_count: text.len() as u32,
            estimated_duration: Some(estimate_text_duration(&text)),
        })
    }

    fn list_voices(
        &self,
        filter: Option<golem_tts::golem::tts::voices::VoiceFilter>,
    ) -> Result<golem_tts::golem::tts::voices::VoiceResults, golem_tts::golem::tts::types::TtsError>
    {
        trace!("Listing available voices.");
        let params = ListVoiceParam {
            engine: filter.as_ref().and_then(|f| f.quality.clone()),
            include_additional_language_codes: Some(true),
            language_code: filter.as_ref().map(|f| f.language.clone().unwrap()),
            next_token: None,
        };

        let client = self.get_client::<()>("GET", "/v1/voices", None)?;
        let response = client.make_request::<ListVoiceResponse, (), ListVoiceParam, _>(
            Method::GET,
            "/v1/voices",
            (),
            Some(&params),
            from_http_error,
        )?;

        Ok(VoiceResults::new(AwsVoiceResults::from(response)))
    }

    fn get_voice(
        &self,
        voice_id: String,
    ) -> Result<golem_tts::golem::tts::voices::Voice, golem_tts::golem::tts::types::TtsError> {
        let client = self.get_client::<()>("GET", "/v1/voices", None)?;
        let result = client.make_request::<ListVoiceResponse, (), (), _>(
            Method::GET,
            "/v1/voices",
            (),
            None,
            from_http_error,
        )?;
        for voice in result.voices {
            if voice.name == voice_id {
                return Ok(Voice::new(voice));
            }
        }
        Err(TtsError::VoiceNotFound(voice_id.to_string()))
    }

    fn list_languages(
        &self,
    ) -> Result<
        Vec<golem_tts::golem::tts::voices::LanguageInfo>,
        golem_tts::golem::tts::types::TtsError,
    > {
        Ok(vec![
            LanguageInfo {
                code: "en-US".to_string(),
                name: "English (US)".to_string(),
                native_name: "English (United States)".to_string(),
                voice_count: 16,
            },
            LanguageInfo {
                code: "en-GB".to_string(),
                name: "English (UK)".to_string(),
                native_name: "English (United Kingdom)".to_string(),
                voice_count: 5,
            },
            LanguageInfo {
                code: "en-AU".to_string(),
                name: "English (Australia)".to_string(),
                native_name: "English (Australia)".to_string(),
                voice_count: 2,
            },
            LanguageInfo {
                code: "en-IN".to_string(),
                name: "English (India)".to_string(),
                native_name: "English (India)".to_string(),
                voice_count: 3,
            },
            LanguageInfo {
                code: "es-ES".to_string(),
                name: "Spanish (Spain)".to_string(),
                native_name: "Español (España)".to_string(),
                voice_count: 4,
            },
            LanguageInfo {
                code: "es-MX".to_string(),
                name: "Spanish (Mexico)".to_string(),
                native_name: "Español (México)".to_string(),
                voice_count: 2,
            },
            LanguageInfo {
                code: "es-US".to_string(),
                name: "Spanish (US)".to_string(),
                native_name: "Español (Estados Unidos)".to_string(),
                voice_count: 3,
            },
            LanguageInfo {
                code: "fr-FR".to_string(),
                name: "French (France)".to_string(),
                native_name: "Français (France)".to_string(),
                voice_count: 4,
            },
            LanguageInfo {
                code: "fr-CA".to_string(),
                name: "French (Canada)".to_string(),
                native_name: "Français (Canada)".to_string(),
                voice_count: 1,
            },
            LanguageInfo {
                code: "de-DE".to_string(),
                name: "German".to_string(),
                native_name: "Deutsch".to_string(),
                voice_count: 3,
            },
            LanguageInfo {
                code: "it-IT".to_string(),
                name: "Italian".to_string(),
                native_name: "Italiano".to_string(),
                voice_count: 2,
            },
            LanguageInfo {
                code: "pt-PT".to_string(),
                name: "Portuguese (Portugal)".to_string(),
                native_name: "Português (Portugal)".to_string(),
                voice_count: 2,
            },
            LanguageInfo {
                code: "pt-BR".to_string(),
                name: "Portuguese (Brazil)".to_string(),
                native_name: "Português (Brasil)".to_string(),
                voice_count: 3,
            },
            LanguageInfo {
                code: "ja-JP".to_string(),
                name: "Japanese".to_string(),
                native_name: "日本語".to_string(),
                voice_count: 3,
            },
            LanguageInfo {
                code: "ko-KR".to_string(),
                name: "Korean".to_string(),
                native_name: "한국어".to_string(),
                voice_count: 1,
            },
            LanguageInfo {
                code: "zh-CN".to_string(),
                name: "Chinese (Simplified)".to_string(),
                native_name: "中文（简体）".to_string(),
                voice_count: 1,
            },
            LanguageInfo {
                code: "cmn-CN".to_string(),
                name: "Chinese Mandarin".to_string(),
                native_name: "普通话".to_string(),
                voice_count: 1,
            },
            LanguageInfo {
                code: "ar".to_string(),
                name: "Arabic".to_string(),
                native_name: "العربية".to_string(),
                voice_count: 1,
            },
            LanguageInfo {
                code: "hi-IN".to_string(),
                name: "Hindi".to_string(),
                native_name: "हिन्दी".to_string(),
                voice_count: 2,
            },
            LanguageInfo {
                code: "ru-RU".to_string(),
                name: "Russian".to_string(),
                native_name: "Русский".to_string(),
                voice_count: 2,
            },
            LanguageInfo {
                code: "nl-NL".to_string(),
                name: "Dutch".to_string(),
                native_name: "Nederlands".to_string(),
                voice_count: 2,
            },
            LanguageInfo {
                code: "pl-PL".to_string(),
                name: "Polish".to_string(),
                native_name: "Polski".to_string(),
                voice_count: 2,
            },
            LanguageInfo {
                code: "sv-SE".to_string(),
                name: "Swedish".to_string(),
                native_name: "Svenska".to_string(),
                voice_count: 1,
            },
            LanguageInfo {
                code: "nb-NO".to_string(),
                name: "Norwegian".to_string(),
                native_name: "Norsk".to_string(),
                voice_count: 1,
            },
            LanguageInfo {
                code: "da-DK".to_string(),
                name: "Danish".to_string(),
                native_name: "Dansk".to_string(),
                voice_count: 2,
            },
            LanguageInfo {
                code: "tr-TR".to_string(),
                name: "Turkish".to_string(),
                native_name: "Türkçe".to_string(),
                voice_count: 1,
            },
            LanguageInfo {
                code: "ro-RO".to_string(),
                name: "Romanian".to_string(),
                native_name: "Română".to_string(),
                voice_count: 1,
            },
            LanguageInfo {
                code: "cy-GB".to_string(),
                name: "Welsh".to_string(),
                native_name: "Cymraeg".to_string(),
                voice_count: 1,
            },
            LanguageInfo {
                code: "is-IS".to_string(),
                name: "Icelandic".to_string(),
                native_name: "Íslenska".to_string(),
                voice_count: 2,
            },
        ])
    }

    fn create_voice_clone(
        &self,
        name: String,
        audio_samples: Vec<golem_tts::golem::tts::advanced::AudioSample>,
        description: Option<String>,
    ) -> Result<golem_tts::golem::tts::voices::Voice, golem_tts::golem::tts::types::TtsError> {
        unsupported("Voice cloning is not supported by AWS Polly")
    }

    fn design_voice(
        &self,
        name: String,
        characteristics: golem_tts::golem::tts::advanced::VoiceDesignParams,
    ) -> Result<golem_tts::golem::tts::voices::Voice, golem_tts::golem::tts::types::TtsError> {
        unsupported("Voice design is not supported by AWS Polly")
    }

    fn convert_voice(
        &self,
        input_audio: Vec<u8>,
        target_voice: String,
        preserve_timing: Option<bool>,
    ) -> Result<Vec<u8>, golem_tts::golem::tts::types::TtsError> {
        unsupported("Voice-to-voice conversion is not supported by AWS Polly")
    }

    fn generate_sound_effect(
        &self,
        description: String,
        duration_seconds: Option<f32>,
        style_influence: Option<f32>,
    ) -> Result<Vec<u8>, golem_tts::golem::tts::types::TtsError> {
        unsupported("Sound effect generation is not supported by AWS Polly")
    }

    fn create_lexicon(
        &self,
        name: String,
        language: golem_tts::golem::tts::advanced::LanguageCode,
        entries: Option<Vec<golem_tts::golem::tts::advanced::PronunciationEntry>>,
    ) -> Result<
        golem_tts::golem::tts::advanced::PronunciationLexicon,
        golem_tts::golem::tts::types::TtsError,
    > {
        let Some(entries) = entries else {
            return Err(TtsError::RequestError(
                "PronunciationEntry is empty.".to_string(),
            ));
        };

        let pls_content = create_pls_content(language.as_str(), &entries);

        let body = PutLexiconRequest {
            content: pls_content,
            name: name.to_string(),
        };

        let put_path = format!("/v1/lexicons/{}", name);

        let mut client = self.get_client("POST", &put_path, Some(&body))?;

        client.make_request::<(), PutLexiconRequest, (), _>(
            Method::POST,
            &put_path,
            body,
            None,
            from_http_error,
        )?;

        let get_path = format!("/v1/lexicons/{}", name);

        client = self.get_client::<()>("GET", &get_path, None)?;

        let response = client.make_request::<GetLexiconResponse, (), (), _>(
            Method::GET,
            &get_path,
            (),
            None,
            from_http_error,
        )?;

        Ok(PronunciationLexicon::new(AwsPronunciationLexicon::new(
            response.lexicon,
            language,
            response.lexicon_attributes,
        )))
    }

    fn synthesize_long_form(
        &self,
        content: String,
        voice: String,
        output_location: String,
        chapter_breaks: Option<Vec<u32>>,
    ) -> Result<
        golem_tts::golem::tts::advanced::LongFormOperation,
        golem_tts::golem::tts::types::TtsError,
    > {
        let (bucket, key_prefix) = parse_s3_location(&output_location)?;
        let body = StartSpeechSynthesisTaskRequest {
            text: content,
            engine: Some("long-form".to_string()),
            language_code: None,
            lexicon_names: None,
            output_format: "mp3".to_string(),
            output_s3_bucket_name: bucket,
            output_s3_key_prefix: Some(key_prefix),
            sample_rate: None,
            sns_topic_arn: None,
            speech_mark_types: None,
            text_type: None,
            voice_id: voice,
        };
        let path = "/v1/synthesisTasks".to_string();
        let client = self.get_client("POST", &path, Some(&body))?;

        let response = client.make_request::<StartSpeechSynthesisTaskResponse,StartSpeechSynthesisTaskRequest,(),_>(Method::POST, &path, body, None, from_http_error)?;

        Ok(LongFormOperation::new(AwsLongFormOperation::new(
            response.synthesis_task,
            output_location,
        )))
    }
}
