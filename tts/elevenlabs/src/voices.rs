use std::cell::RefCell;

use golem_tts::golem::tts::{
    types::{LanguageCode, TtsError},
    voices::{
        Guest, GuestVoice, GuestVoiceResults, LanguageInfo, Voice, VoiceFilter, VoiceGender,
        VoiceInfo, VoiceResults, VoiceSettings,
    },
};
use reqwest::Client;

use crate::{
    client::{
        error::{from_http_error, unsupported},
        voices::{ElVoice, ElVoices},
        ElevenLabsClient,
    },
    ElevenLabsTtsComponent,
};

pub struct ElevenLabsVoiceResults {
    pub voices: RefCell<Vec<ElVoice>>,
    pub has_more: RefCell<bool>,
    pub total_count: RefCell<Option<u32>>,
    pub next_page_token: RefCell<Option<String>>,
    pub options: RefCell<Option<VoiceFilter>>,
}

impl Guest for ElevenLabsTtsComponent {
    type Voice = ElVoice;
    type VoiceResults = ElevenLabsVoiceResults;

    fn list_voices(filter: Option<VoiceFilter>) -> Result<VoiceResults, TtsError> {
        let client = ElevenLabsClient::new()?;
        let result: ElVoices = client.list_voices(filter.clone(), None)?;
        let voice_result = ElevenLabsVoiceResults {
            has_more: RefCell::new(result.has_more),
            voices: RefCell::new(result.voices),
            total_count: RefCell::new(result.total_count),
            next_page_token: RefCell::new(result.next_page_token),
            options: RefCell::new(filter),
        };
        Ok(VoiceResults::new(voice_result))
    }

    fn get_voice(voice_id: String) -> Result<Voice, TtsError> {
        let client = ElevenLabsClient::new()?;
        let result: ElVoice = client.get_voice(&voice_id)?;
        Ok(Voice::new(result))
    }

    fn search_voices(
        query: String,
        filter: Option<VoiceFilter>,
    ) -> Result<Vec<VoiceInfo>, TtsError> {
        let client = ElevenLabsClient::new()?;
        let result = client.list_voices(filter, Some(query))?;
        Ok(result.voices.iter().map(VoiceInfo::from).collect())
    }

    fn list_languages() -> Result<Vec<LanguageInfo>, TtsError> {
        let client = ElevenLabsClient::new()?;
        client.list_languages()
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

        let client = ElevenLabsClient::new()?;
        let result: ElVoices = client.list_voices(
            self.options.borrow().clone(),
            self.next_page_token.borrow().clone(),
        )?;

        *self.has_more.borrow_mut() = result.has_more;
        *self.next_page_token.borrow_mut() = result.next_page_token;
        *self.total_count.borrow_mut() = result.total_count;
        self.voices.borrow_mut().extend(result.voices.clone());

        Ok(result.voices.iter().map(VoiceInfo::from).collect())
    }

    fn get_total_count(&self) -> Option<u32> {
        *self.total_count.borrow()
    }
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
        let client = ElevenLabsClient::new()?;
        let _ = client.delete_voice(&self.voice_id)?;
        Ok(())
    }

    fn clone(&self) -> Result<Voice, TtsError> {
        todo!()
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

impl From<&ElVoice> for VoiceInfo {
    fn from(v: &ElVoice) -> Self {
        let mut languages = vec!["English".to_string()];
        if let Some(vl) = &v.verified_languages {
            for l in vl.iter() {
                languages.push(l.language.clone());
            }
        };

        VoiceInfo {
            id: v.voice_id.clone(),
            name: v.name.clone(),
            language: languages[0].clone(),
            additional_languages: languages,
            gender: VoiceGender::Neutral,
            quality: "standard".to_string(),
            description: v.description.clone(),
            provider: "ElevenLabs".to_string(),
            sample_rate: 0,
            is_custom: v.is_owner.unwrap_or(false),
            is_cloned: v.category.clone().unwrap_or_default().contains("cloned"),
            preview_url: v.preview_url.clone(),
            use_cases: vec![],
        }
    }
}
