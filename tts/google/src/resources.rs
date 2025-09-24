use std::cell::RefCell;

use golem_tts::{client::TtsClient, golem::tts::{advanced::{GuestLongFormOperation, GuestPronunciationLexicon, LanguageCode, LongFormResult, OperationStatus, Voice}, streaming::{AudioChunk, GuestVoiceConversionStream, TextInput, VoiceSettings}, types::{TextType, TtsError, VoiceGender}, voices::{GuestVoice, GuestVoiceResults, VoiceInfo}}};
use serde::Deserialize;

use crate::{google::Google, error::unsupported};




pub  struct GoogleLongFormOperation;


impl GuestLongFormOperation for GoogleLongFormOperation {
    fn get_status(&self,) -> OperationStatus {
        todo!()
    }

    fn get_progress(&self,) -> f32 {
        todo!()
    }

    fn cancel(&self,) -> Result<(),TtsError> {
        todo!()
    }

    fn get_result(&self,) -> Result<LongFormResult,TtsError> {
        todo!()
    }
}



#[derive(Deserialize, Clone)]
pub struct GoogleVoice {
    #[serde(rename = "languageCodes")]
    pub language_codes: Vec<String>,
    pub name: String,
    #[serde(rename = "ssmlGender")]
    pub ssml_gender: String,
    #[serde(rename = "naturalSampleRateHertz")]
    pub natural_sample_rate_hertz: u32,
}



impl GuestVoice for GoogleVoice {
    fn get_id(&self) -> String {
        self.name.clone()
    }

    fn get_name(&self) -> String {
        self.name.clone()
    }

    fn get_provider_id(&self) -> Option<String> {
        Some("google".to_string())
    }

    fn get_language(&self) -> LanguageCode {
        self.language_codes[0].clone()
    }

    fn get_additional_languages(&self) -> Vec<LanguageCode> {
        self.language_codes.clone()
    }

    fn get_gender(&self) -> VoiceGender {
        match self.ssml_gender.as_str() {
            "MALE" => VoiceGender::Male,
            "FEMALE" => VoiceGender::Female,
            _ => VoiceGender::Neutral,
        }
    }

    fn get_quality(&self) -> String {
        "neural".to_string()
    }

    fn get_description(&self) -> Option<String> {
        Some(format!("Google Text-to-Speech voice: {}", self.name))
    }

    fn supports_ssml(&self) -> bool {
        true // Google TTS supports SSML
    }

    fn get_sample_rates(&self) -> Vec<u32> {
        vec![self.natural_sample_rate_hertz]
    }

    fn get_supported_formats(&self) -> Vec<String> {
        vec![
            "MP3".to_string(),
            "OGG_OPUS".to_string(),
            "MULAW".to_string(),
            "ALAW".to_string(),
            "LINEAR16".to_string(),
        ]
    }

    fn update_settings(&self, _settings: VoiceSettings) -> Result<(), TtsError> {
        unsupported("Voice settings update not supported by Google TTS")
    }

    fn delete(&self) -> Result<(), TtsError> {
        unsupported("Voice deletion not supported by Google TTS")
    }

    fn clone(&self) -> Result<Voice, TtsError> {
        unsupported("Voice cloning not supported by Google TTS")
    }

    fn preview(&self, text: String) -> Result<Vec<u8>, TtsError> {
        let client = Google::new()?;
        let result = client.synthesize(
            TextInput {
                content: text,
                text_type: TextType::Plain,
                language: None,
            },
            self.name.clone(),
            None,
        )?;
        Ok(result.audio_data)
    }
}



#[derive(Clone, Debug)]
pub struct GoogleVoiceResults {
    voices: RefCell<Vec<VoiceInfo>>,
    has_more: RefCell<bool>,
}

impl GoogleVoiceResults {
    pub fn new(voices: Vec<VoiceInfo>) -> Self {
        Self {
            voices: RefCell::new(voices),
            has_more: RefCell::new(false),
        }
    }
}

impl GuestVoiceResults for GoogleVoiceResults {
    fn has_more(&self) -> bool {
        *self.has_more.borrow()
    }

    fn get_next(&self) -> Result<Vec<VoiceInfo>, TtsError> {
        unsupported(
            "Pagination is not supported by Google Cloud TTS. Use list_voices to get all voices.",
        )
    }

    fn get_total_count(&self) -> Option<u32> {
        Some(self.voices.borrow().len() as u32)
    }
}




#[derive(Clone)]
pub struct GoogleVoiceConversionStream ;


impl GuestVoiceConversionStream for GoogleVoiceConversionStream {
    #[doc = " Send input audio chunks"]
    fn send_audio(&self, _audio_data: Vec<u8>) -> Result<(), TtsError> {
        unsupported("Google TTS does not support voice conversion")
    }

    #[doc = " Receive converted audio chunks"]
    fn receive_converted(&self) -> Result<Option<AudioChunk>, TtsError> {
        unsupported("Google TTS does not support voice conversion")
    }

    fn finish(&self) -> Result<(), TtsError> {
        unsupported("Google TTS does not support voice conversion")
    }

    fn close(&self) {}
}

pub struct GooglePronunciationLexicon;

impl GuestPronunciationLexicon for GooglePronunciationLexicon {
    fn get_name(&self) -> String {
        "Unsupported".to_string()
    }

    fn get_language(&self) -> LanguageCode {
        "en".to_string()
    }

    fn get_entry_count(&self) -> u32 {
        0
    }

    #[doc = " Add pronunciation rule"]
    fn add_entry(&self, _word: String, _pronunciation: String) -> Result<(), TtsError> {
        unsupported("Google TTS does not support custom pronunciation lexicons")
    }

    #[doc = " Remove pronunciation rule"]
    fn remove_entry(&self, _word: String) -> Result<(), TtsError> {
        unsupported("Google TTS does not support custom pronunciation lexicons")
    }

    #[doc = " Export lexicon content"]
    fn export_content(&self) -> Result<String, TtsError> {
        unsupported("Google TTS does not support custom pronunciation lexicons")
    }
}
