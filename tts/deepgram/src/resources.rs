use std::cell::RefCell;

use golem_tts::golem::tts::{
    advanced::{
        GuestLongFormOperation, GuestPronunciationLexicon, LanguageCode, LongFormResult,
        OperationStatus, TtsError, Voice,
    },
    streaming::{AudioChunk, GuestVoiceConversionStream, VoiceSettings},
    types::VoiceGender,
    voices::{GuestVoice, GuestVoiceResults, VoiceInfo},
};
use serde::{Deserialize, Serialize};

use crate::error::unsupported;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeepgramVoice {
    pub name: String,
    pub canonical_name: String,
    pub architecture: String,
    pub languages: Vec<String>,
    pub version: String,
    pub uuid: String,
    pub metadata: DeepgramVoiceMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeepgramVoiceMetadata {
    pub accent: String,
    pub age: String,
    pub color: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    pub image: String,
    pub sample: String,
    pub tags: Vec<String>,
    pub use_cases: Vec<String>,
}

impl GuestVoice for DeepgramVoice {
    #[doc = " Get voice identification"]
    fn get_id(&self) -> String {
        self.uuid.clone()
    }

    fn get_name(&self) -> String {
        self.metadata
            .display_name
            .clone()
            .unwrap_or_else(|| self.name.clone())
    }

    fn get_provider_id(&self) -> Option<String> {
        Some("Deepgram".to_string())
    }

    #[doc = " Get voice characteristics"]
    fn get_language(&self) -> LanguageCode {
        self.languages
            .first()
            .cloned()
            .unwrap_or_else(|| "en-US".to_string())
    }

    fn get_additional_languages(&self) -> Vec<LanguageCode> {
        if self.languages.len() > 1 {
            self.languages[1..].to_vec()
        } else {
            vec![]
        }
    }

    fn get_gender(&self) -> VoiceGender {
        let tags = self.metadata.tags.clone();
        for t in tags {
            if t.contains("feminine") {
                return VoiceGender::Female;
            } else if t.contains("masculine") {
                return VoiceGender::Male;
            }
        }
        VoiceGender::Neutral
    }

    fn get_quality(&self) -> String {
        "standard".to_string()
    }

    fn get_description(&self) -> Option<String> {
        let use_case = self.metadata.use_cases.clone().join(",");
        let name = self.name.clone();
        Some(format!("I am {}. I can help you with {}", name, use_case))
    }

    #[doc = " Voice capabilities"]
    fn supports_ssml(&self) -> bool {
        false
    }

    fn get_sample_rates(&self) -> Vec<u32> {
        vec![]
    }

    fn get_supported_formats(&self) -> Vec<String> {
        vec!["Mp3".to_string(), "Wav".to_string(), "OggOpus".to_string()]
    }

    #[doc = " Voice management (may return unsupported-operation)"]
    fn update_settings(&self, _settings: VoiceSettings) -> Result<(), TtsError> {
        unsupported("Voice settings update is not supported by Deepgram TTS")
    }

    fn delete(&self) -> Result<(), TtsError> {
        unsupported("Voice delete is not supported by Deepgram TTS")
    }

    fn clone(&self) -> Result<Voice, TtsError> {
        unsupported("Voice clone is not supported by Deepgram TTS")
    }

    #[doc = " Preview voice with sample text"]
    fn preview(&self, _text: String) -> Result<Vec<u8>, TtsError> {
        unsupported("Preview is not supported by Deepgram TTS")
    }
}

#[derive(Debug, Clone)]
pub struct DeepgramVoiceResults {
    pub next_token: RefCell<Option<String>>,
    pub voices: RefCell<Vec<VoiceInfo>>,
    pub total_count: RefCell<Option<u32>>,
}

impl DeepgramVoiceResults {
    pub fn new(value: Vec<DeepgramVoice>) -> Self {
        let voices: Vec<VoiceInfo> = value.iter().map(VoiceInfo::from).collect();
        let count = voices.len() as u32;
        Self {
            next_token: RefCell::new(None),
            voices: RefCell::new(voices),
            total_count: RefCell::new(Some(count)),
        }
    }
}

impl GuestVoiceResults for DeepgramVoiceResults {
    #[doc = " Check if more voices are available"]
    fn has_more(&self) -> bool {
        false
    }

    #[doc = " Get next batch of voices"]
    fn get_next(&self) -> Result<Vec<VoiceInfo>, TtsError> {
        unsupported(
            "Pagination is not supported by Deepgram TTS. Use list_voices to get all voices.",
        )
    }

    #[doc = " Get total count if available"]
    fn get_total_count(&self) -> Option<u32> {
        *self.total_count.borrow()
    }
}

pub struct DeepgramPronunciationLexicon;

impl GuestPronunciationLexicon for DeepgramPronunciationLexicon {
    fn get_name(&self) -> String {
        "Deepgram does not support pronunciation lexicon".to_string()
    }

    fn get_language(&self) -> LanguageCode {
        "Deepgram does not support pronunciation lexicon".to_string()
    }

    fn get_entry_count(&self) -> u32 {
        0
    }

    #[doc = " Add pronunciation rule"]
    fn add_entry(&self, _word: String, _pronunciation: String) -> Result<(), TtsError> {
        unsupported("Deepgram does not support pronunciation lexicon")
    }

    #[doc = " Remove pronunciation rule"]
    fn remove_entry(&self, _word: String) -> Result<(), TtsError> {
        unsupported("Deepgram does not support pronunciation lexicon")
    }

    #[doc = " Export lexicon content"]
    fn export_content(&self) -> Result<String, TtsError> {
        unsupported("Deepgram does not support pronunciation lexicon")
    }
}

pub struct DeepgramLongFormOperation;

impl GuestLongFormOperation for DeepgramLongFormOperation {
    fn get_status(&self) -> OperationStatus {
        OperationStatus::Failed
    }

    fn get_progress(&self) -> f32 {
        100.0
    }

    fn cancel(&self) -> Result<(), TtsError> {
        unsupported("Deepgram does not support long form operations")
    }

    fn get_result(&self) -> Result<LongFormResult, TtsError> {
        unsupported("Deepgram does not support long form operations")
    }
}

pub struct DeepgramVoiceConversionStream;

impl GuestVoiceConversionStream for DeepgramVoiceConversionStream {
    #[doc = " Send input audio chunks"]
    fn send_audio(&self, _audio_data: Vec<u8>) -> Result<(), TtsError> {
        unsupported("Deepgram does not support voice conversion streaming")
    }

    #[doc = " Receive converted audio chunks"]
    fn receive_converted(&self) -> Result<Option<AudioChunk>, TtsError> {
        unsupported("Deepgram does not support voice conversion streaming")
    }

    fn finish(&self) -> Result<(), TtsError> {
        unsupported("Deepgram does not support voice conversion streaming")
    }

    fn close(&self) {}
}
