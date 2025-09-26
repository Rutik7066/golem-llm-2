use std::{
    cell::RefCell,
    collections::{HashMap, VecDeque},
};

use golem_tts::{
    config::get_env,
    golem::tts::{
        advanced::{
            GuestLongFormOperation, GuestPronunciationLexicon, LongFormResult, OperationStatus,
        },
        streaming::{GuestVoiceConversionStream, SynthesisOptions},
        types::{AudioChunk, LanguageCode, TtsError, VoiceGender},
        voices::{GuestVoice, GuestVoiceResults, Voice, VoiceFilter, VoiceInfo, VoiceSettings},
    },
};
use log::trace;
use reqwest::{ Client, InputStream, Method};
use serde::Deserialize;
use uuid::Uuid;

use crate::{
    elevenlabs::Elevenlabs,
    error::{from_http_error, unsupported},
    types::{
        AddRulesRequest, CreateLexiconResponse, ElVoiceSettings, ElVoices, ListVoicesQuery,
        PronunciationRule, RemoveRulesRequest, UpdateLexiconRuleResponse, VerifiedLanguage,
    },
    utils::{add_file_field, add_form_field},
};

pub struct ElevenLabsVoiceResults {
    pub voices: RefCell<Vec<ElVoice>>,
    pub has_more: RefCell<bool>,
    pub total_count: RefCell<u32>,
    pub next_page_token: RefCell<Option<String>>,
    pub filter: RefCell<Option<VoiceFilter>>,
}

impl ElevenLabsVoiceResults {
    pub fn new(
        voices: Vec<ElVoice>,
        next_page_token: Option<String>,
        filter: Option<VoiceFilter>,
    ) -> Self {
        let count = voices.len() as u32;
        Self {
            voices: RefCell::new(voices),
            has_more: RefCell::new(next_page_token.is_some()),
            total_count: RefCell::new(count),
            next_page_token: RefCell::new(next_page_token),
            filter: RefCell::new(filter),
        }
    }
}

impl GuestVoiceResults for ElevenLabsVoiceResults {
    fn has_more(&self) -> bool {
        *self.has_more.borrow()
    }

    fn get_next(&self) -> Result<Vec<VoiceInfo>, TtsError> {
        if !self.has_more() {
            return Ok(vec![]);
        }

        let body = ListVoicesQuery {
            next_page_token: None,
            page_size: Some(100),
            search: self
                .filter
                .borrow()
                .as_ref()
                .and_then(|f| f.search_query.clone()),
            sort: Some("name".to_string()),
            sort_direction: Some("asc".to_string()),
            voice_type: None,
            category: None,
            fine_tuning_state: None,
            collection_id: None,
            include_total_count: Some(true),
            voice_ids: None,
        };
        let elevenlabs = Elevenlabs::new()?;

        let result = elevenlabs
            .client
            .retry_request::<ElVoices, _, ListVoicesQuery, _>(
                reqwest::Method::GET,
                "/v1/voices",
                "",
                Some(&body),
                from_http_error,
            )?;

        *self.has_more.borrow_mut() = result.has_more;
        *self.next_page_token.borrow_mut() = result.next_page_token;
        *self.total_count.borrow_mut() = result.total_count.unwrap_or_default();
        self.voices.borrow_mut().extend(result.voices.clone());

        Ok(result.voices.iter().map(VoiceInfo::from).collect())
    }

    fn get_total_count(&self) -> Option<u32> {
        Some(*self.total_count.borrow())
    }
}

#[derive(Deserialize, Clone, Debug)]
pub struct ElVoice {
    pub voice_id: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub labels: Option<HashMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preview_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub settings: Option<ElVoiceSettings>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verified_languages: Option<Vec<VerifiedLanguage>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_owner: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_legacy: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_mixed: Option<bool>,
}

impl GuestVoice for ElVoice {
    fn get_id(&self) -> String {
        self.voice_id.clone()
    }

    fn get_name(&self) -> String {
        self.name.clone()
    }

    fn get_provider_id(&self) -> Option<String> {
        Some("elevenlabs".to_string())
    }

    fn get_language(&self) -> LanguageCode {
        self.verified_languages
            .as_ref()
            .and_then(|langs| langs.first())
            .map_or_else(|| "English".to_string(), |lang| lang.language.clone())
    }

    fn get_additional_languages(&self) -> Vec<LanguageCode> {
        self.verified_languages.as_ref().map_or_else(
            || vec!["English".to_string()],
            |langs| langs.iter().map(|l| l.language.clone()).collect(),
        )
    }

    fn get_gender(&self) -> VoiceGender {
        VoiceGender::Neutral
    }

    fn get_quality(&self) -> String {
        "standard".to_string()
    }

    fn get_description(&self) -> Option<String> {
        self.description.clone()
    }

    fn supports_ssml(&self) -> bool {
        true
    }

    fn get_sample_rates(&self) -> Vec<u32> {
        vec![22050, 44100]
    }

    fn get_supported_formats(&self) -> Vec<String> {
        vec![
            "Mp3".to_string(),
            "Alaw".to_string(),
            "OggOpus".to_string(),
            "Pcm".to_string(),
        ]
    }

    fn update_settings(&self, _settings: VoiceSettings) -> Result<(), TtsError> {
        unsupported("Updating voice settings is not supported")
    }

    fn delete(&self) -> Result<(), TtsError> {
        let elevenlabs = Elevenlabs::new()?;
        elevenlabs.client.retry_request::<(), _, (), _>(
            Method::DELETE,
            &format!("/v1/voices/{}", self.voice_id),
            "",
            None,
            from_http_error,
        )
    }

    fn clone(&self) -> Result<Voice, TtsError> {
        unsupported("Voice to voice cloning is not supported by Elevenlabs")
    }

    fn preview(&self, _text: String) -> Result<Vec<u8>, TtsError> {
        if self.preview_url.is_none() {
            return Err(TtsError::UnsupportedOperation(
                "Preview is not supported for this voice".to_string(),
            ));
        }
        let response = Client::new()
            .get(self.preview_url.as_ref().unwrap().clone())
            .send()
            .map_err(|err| {
                TtsError::InternalError(format!("Failed to create HTTP client: {err}"))
            })?;

        if response.status().is_success() {
            let audio = response.bytes().map_err(|err| {
                TtsError::InternalError(format!("Failed to read response body: {err}"))
            })?;

            Ok(audio.to_vec())
        } else {
            Err(from_http_error(response))
        }
    }
}

pub struct ElevenLabsVoiceConversionStream {
    pub target_voice: String,
    pub options: Option<SynthesisOptions>,
    data: RefCell<Vec<u8>>,
    audio_data: RefCell<Option<VecDeque<u8>>>,
    sequence_number: RefCell<u32>,
}

impl ElevenLabsVoiceConversionStream {
    pub fn new(target_voice: String, options: Option<SynthesisOptions>) -> Self {
        Self {
            target_voice,
            options,
            data: RefCell::new(Vec::new()),
            audio_data: RefCell::new(None),
            sequence_number: RefCell::new(0),
        }
    }
}

impl GuestVoiceConversionStream for ElevenLabsVoiceConversionStream {
    #[doc = " Send input audio chunks"]
    fn send_audio(&self, audio_data: Vec<u8>) -> Result<(), TtsError> {
        self.data.borrow_mut().extend(audio_data);
        Ok(())
    }

    #[doc = " Receive converted audio chunks"]
    fn receive_converted(&self) -> Result<Option<AudioChunk>, TtsError> {
        if let Some(audio_data) = self.audio_data.borrow_mut().as_mut() {
            let data = audio_data.drain(..1024).collect::<Vec<u8>>();
            *self.sequence_number.borrow_mut() = *self.sequence_number.borrow() + 1;
            Ok(Some(AudioChunk {
                data,
                sequence_number: *self.sequence_number.borrow(),
                is_final: audio_data.is_empty(),
                timing_info: None,
            }))
        } else {
            Ok(None)
        }
    }

    /// Creating multipart form data manually. golemcloud/reqwest fails to compile with multipart
    fn finish(&self) -> Result<(), TtsError> {
        if self.data.borrow().is_empty() {
            return Err(TtsError::InvalidText(
                "Input audio data cannot be empty".to_string(),
            ));
        }

        let api_key = get_env("ELEVENLABS_API_KEY")?;
        let base_url = get_env("TTS_PROVIDER_ENDPOINT")
            .ok()
            .unwrap_or("https://api.elevenlabs.io".to_string());

        // Create multipart form data manually
        let boundary = format!(
            "----boundary{}",
            Uuid::new_v4().to_string().replace("-", "")
        );
        let mut body = Vec::new();

        // Add audio file
        let _ = add_file_field(
            &mut body,
            &boundary,
            "audio",
            "input_audio.wav",
            "audio/wav",
            self.data.borrow().as_slice(),
        );

        let output_format = self
            .options
            .as_ref()
            .and_then(|o| o.audio_config.as_ref().map(|c| c.format.clone()))
            .unwrap_or_default();
        let _ = add_form_field(
            &mut body,
            &boundary,
            "output_format",
            output_format.as_str(),
        );

        let model_id = self
            .options
            .as_ref()
            .and_then(|o| o.model_id.clone())
            .unwrap_or_default();
        let _ = add_form_field(&mut body, &boundary, "model_id", model_id.as_str());

        body.extend_from_slice(format!("--{}--\r\n", boundary).as_bytes());

        let url = format!(
            "{}/v1/speech-to-speech/{}",
            base_url,
            self.target_voice.clone()
        );

        let request = Client::new()
            .post(&url)
            .header("xi-api-key", api_key)
            .header(
                "Content-Type",
                format!("multipart/form-data; boundary={}", boundary),
            )
            .body(body);

        let result = match request.send() {
            Ok(response) => {
                if response.status().is_success() {
                    response
                        .bytes()
                        .map_err(|e| {
                            TtsError::InternalError(format!(
                                "Failed to read binary response: {}",
                                e
                            ))
                        })
                        .map(|bytes| bytes.to_vec())
                } else {
                    Err(from_http_error(response))
                }
            }
            Err(err) => Err(TtsError::NetworkError(format!("Request failed: {}", err))),
        }?;
        *self.audio_data.borrow_mut() = Some(result.into());
        Ok(())
    }

    fn close(&self) {
        // Nothing to close
    }
}

pub struct ElPronunciationLexicon {
    pub id: String,
    pub name: String,
    pub language: LanguageCode,
    pub version_id: std::cell::RefCell<String>,
    pub rules_count: std::cell::RefCell<u32>,
}

impl GuestPronunciationLexicon for ElPronunciationLexicon {
    fn get_name(&self) -> String {
        self.name.clone()
    }

    fn get_language(&self) -> LanguageCode {
        self.language.clone()
    }

    fn get_entry_count(&self) -> u32 {
        *self.rules_count.borrow()
    }

    #[doc = " Add pronunciation rule"]
    fn add_entry(&self, word: String, pronunciation: String) -> Result<(), TtsError> {
        let rule = if pronunciation
            .chars()
            .any(|c| "əɪɛɔʊʌɑɒæɜɪʏøœɯɤɐɞɘɵɨɵʉɪʊ".contains(c))
        {
            PronunciationRule {
                string_to_replace: word,
                rule_type: "phoneme".to_string(),
                alias: None,
                phoneme: Some(pronunciation),
                alphabet: Some("ipa".to_string()),
            }
        } else {
            PronunciationRule {
                string_to_replace: word,
                rule_type: "alias".to_string(),
                alias: Some(pronunciation),
                phoneme: None,
                alphabet: None,
            }
        };

        let request = AddRulesRequest { rules: vec![rule] };
        let path = format!("/v1/pronunciation-dictionaries/{}/add-rules", self.id);
        let elevenlabs = Elevenlabs::new()?;

        let response = elevenlabs
            .client
            .make_request::<UpdateLexiconRuleResponse, AddRulesRequest, (), _>(
                Method::POST,
                &path,
                request,
                None,
                from_http_error,
            )?;
        *self.rules_count.borrow_mut() += 1;
        trace!("Add Entry response : {response:?}");
        Ok(())
    }

    #[doc = " Remove pronunciation rule"]
    fn remove_entry(&self, word: String) -> Result<(), TtsError> {
        let request = RemoveRulesRequest {
            rule_strings: vec![word],
        };
        let path = format!(
            "/v1/pronunciation-dictionaries/{}/remove-rules",
            self.id
        );
        let elevenlabs = Elevenlabs::new()?;

        let reuslt = elevenlabs
            .client
            .make_request::<UpdateLexiconRuleResponse, RemoveRulesRequest, (), _>(
                Method::POST,
                &path,
                request,
                None,
                from_http_error,
            )?;
        *self.rules_count.borrow_mut() -= 1;
        trace!("Remove Entry response : {reuslt:?}");
        Ok(())
    }

    #[doc = " Export lexicon content"]
    fn export_content(&self) -> Result<String, TtsError> {
        let base_url = get_env("TTS_PROVIDER_ENDPOINT")
            .ok()
            .unwrap_or("https://api.elevenlabs.io".to_string());
        let api_key = get_env("ELEVENLABS_API_KEY")?;
        let path = format!(
            "/v1/pronunciation-dictionaries/{}/{}/download",
            self.id, self.version_id.borrow()
        );
        let url = format!("{}{}", base_url, path);
        let request = Client::new()
            .request(Method::GET, &url)
            .header("xi-api-key", api_key);

        match request.send() {
            Ok(response) => {
                if response.status().is_success() {
                    response.text().map_err(|e| {
                        TtsError::InternalError(format!("Failed to read PLS content: {}", e))
                    })
                } else {
                    Err(from_http_error(response))
                }
            }
            Err(err) => Err(TtsError::NetworkError(format!("Request failed: {}", err))),
        }
    }
}

pub struct ElLongFormSynthesis;

impl GuestLongFormOperation for ElLongFormSynthesis {
    fn get_status(&self) -> OperationStatus {
        OperationStatus::Cancelled
    }

    fn get_progress(&self) -> f32 {
        100.0
    }

    fn cancel(&self) -> Result<(), TtsError> {
        unsupported("Long-form synthesis not yet implemented for ElevenLabs TTS")
    }

    fn get_result(&self) -> Result<LongFormResult, TtsError> {
        unsupported("Long-form synthesis not yet implemented for ElevenLabs TTS")
    }
}
