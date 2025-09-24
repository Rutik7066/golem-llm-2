use std::cell::RefCell;

use golem_tts::golem::tts::{
    advanced::{
        GuestLongFormOperation, GuestPronunciationLexicon, LongFormResult, OperationStatus,
        TtsError,
    },
    streaming::{AudioChunk, GuestVoiceConversionStream},
    types::{LanguageCode, VoiceGender, VoiceSettings},
    voices::{GuestVoice, GuestVoiceResults, Voice, VoiceInfo},
};
use log::trace;
use reqwest::Method;
use serde::Deserialize;

use crate::{
    error::{from_http_error, unsupported},
    polly::Polly,
    types::{AwsLexicon, LexiconAttributes, ListVoiceParam, ListVoiceResponse, SynthesisTask},
};

#[derive(Clone)]
pub struct AwsVoiceConversionStream;

impl GuestVoiceConversionStream for AwsVoiceConversionStream {
    #[doc = " Send audio for voice conversion"]
    fn send_audio(&self, _audio_data: Vec<u8>) -> Result<(), TtsError> {
        unsupported("Real-time voice conversion streaming is not supported by AWS Polly")
    }

    #[doc = " Receive converted audio chunks"]
    fn receive_converted(&self) -> Result<Option<AudioChunk>, TtsError> {
        unsupported("Real-time voice conversion streaming is not supported by AWS Polly")
    }

    #[doc = " Signal end of input and flush remaining audio"]
    fn finish(&self) -> Result<(), TtsError> {
        unsupported("Real-time voice conversion streaming is not supported by AWS Polly")
    }

    #[doc = " Close stream and clean up resources"]
    fn close(&self) {
        // Nothing to close for unsupported operation
    }
}

#[derive(Deserialize, Debug, Clone)]
pub struct AwsVoice {
    #[serde(rename = "AdditionalLanguageCodes")]
    pub additional_language_codes: Vec<String>,
    #[serde(rename = "Gender")]
    pub gender: String,
    #[serde(rename = "Id")]
    pub id: String,
    #[serde(rename = "LanguageCode")]
    pub language_code: String,
    #[serde(rename = "LanguageName")]
    pub language_name: String,
    #[serde(rename = "Name")]
    pub name: String,
    #[serde(rename = "SupportedEngines")]
    pub supported_engines: Vec<String>,
}

impl GuestVoice for AwsVoice {
    #[doc = " Get voice identification"]
    fn get_id(&self) -> String {
        self.id.clone()
    }

    fn get_name(&self) -> String {
        self.name.clone()
    }

    fn get_provider_id(&self) -> Option<String> {
        Some("AWS Polly".to_string())
    }

    #[doc = " Get voice characteristics"]
    fn get_language(&self) -> LanguageCode {
        self.language_code.clone()
    }

    fn get_additional_languages(&self) -> Vec<LanguageCode> {
        self.additional_language_codes.clone()
    }

    fn get_gender(&self) -> VoiceGender {
        match self.gender.to_lowercase().as_str() {
            "male" => VoiceGender::Male,
            "female" => VoiceGender::Female,
            _ => VoiceGender::Neutral,
        }
    }

    fn get_quality(&self) -> String {
        self.supported_engines.join(", ")
    }

    fn get_description(&self) -> Option<String> {
        Some(format!(
            "AWS Polly {} voice ({}) supporting engines: {}",
            self.name,
            self.language_name,
            self.supported_engines.join(", ")
        ))
    }

    #[doc = " Voice capabilities"]
    fn supports_ssml(&self) -> bool {
        true
    }

    fn get_sample_rates(&self) -> Vec<u32> {
        vec![8000, 16000, 22050, 24000]
    }

    fn get_supported_formats(&self) -> Vec<String> {
        vec!["Mp3".to_string(), "Pcm".to_string(), "OggOpus".to_string()]
    }

    #[doc = " Voice management (may return unsupported-operation)"]
    fn update_settings(&self, _settings: VoiceSettings) -> Result<(), TtsError> {
        unsupported("Voice settings update not supported for AWS Polly voices")
    }

    fn delete(&self) -> Result<(), TtsError> {
        unsupported("Voice deletion not supported for AWS Polly voices")
    }

    fn clone(&self) -> Result<Voice, TtsError> {
        unsupported("Voice cloning not supported for AWS Polly voices")
    }

    #[doc = " Preview voice with sample text"]
    fn preview(&self, _text: String) -> Result<Vec<u8>, TtsError> {
        unsupported("Voice preview not implemented yet")
    }
}

pub struct AwsVoiceResults {
    pub next_token: RefCell<Option<String>>,
    pub voices: RefCell<Vec<VoiceInfo>>,
    pub total_count: RefCell<Option<u32>>,
}

impl GuestVoiceResults for AwsVoiceResults {
    #[doc = " Check if more voices are available"]
    fn has_more(&self) -> bool {
        self.next_token.borrow().is_some()
    }

    #[doc = " Get next batch of voices"]
    fn get_next(&self) -> Result<Vec<VoiceInfo>, TtsError> {
        trace!("Listing available voices.");
        let params = ListVoiceParam {
            engine: None,
            include_additional_language_codes: Some(true),
            language_code: None,
            next_token: self.next_token.borrow().clone(),
        };
        let polly = Polly::new()?;
        let client = polly.get_client::<()>("GET", "/v1/voices", None)?;
        let response = client.make_request::<ListVoiceResponse, (), ListVoiceParam, _>(
            Method::GET,
            "/v1/voices",
            (),
            Some(&params),
            from_http_error,
        )?;
        let voices = response.voices.iter().map(|v| VoiceInfo::from(v)).collect();

        *self.next_token.borrow_mut() = response.next_token;

        Ok(voices)
    }

    #[doc = " Get total count if available"]
    fn get_total_count(&self) -> Option<u32> {
        *self.total_count.borrow()
    }
}

pub struct AwsPronunciationLexicon {
    lexicon: RefCell<AwsLexicon>,
    language_code: RefCell<String>,
    entry_count: RefCell<u32>,
    lexicon_attributes: RefCell<LexiconAttributes>,
}

impl AwsPronunciationLexicon {
    pub fn new(
        lexicon: AwsLexicon,
        language_code: String,
        lexicon_attributes: LexiconAttributes,
    ) -> Self {
        Self {
            lexicon: RefCell::new(lexicon),
            language_code: RefCell::new(language_code),
            entry_count: RefCell::new(0),
            lexicon_attributes: RefCell::new(lexicon_attributes),
        }
    }
}

impl GuestPronunciationLexicon for AwsPronunciationLexicon {
    #[doc = " Get lexicon name"]
    fn get_name(&self) -> String {
        self.lexicon.borrow().name.clone()
    }

    #[doc = " Get supported language"]
    fn get_language(&self) -> LanguageCode {
        self.language_code.borrow().clone()
    }

    #[doc = " Get number of entries"]
    fn get_entry_count(&self) -> u32 {
        *self.entry_count.borrow()
    }

    #[doc = " Add pronunciation entry"]
    fn add_entry(&self, _word: String, _pronunciation: String) -> Result<(), TtsError> {
        unsupported(
            "Adding entries to existing lexicon not supported. Create a new lexicon instead.",
        )
    }

    #[doc = " Export lexicon content"]
    fn export_content(&self) -> Result<String, TtsError> {
        Ok(self.lexicon.borrow().content.clone())
    }

    #[doc = " Remove entry by word"]
    fn remove_entry(&self, _word: String) -> Result<(), TtsError> {
        unsupported("Removing entries from lexicon not supported by AWS Polly")
    }
}

pub struct AwsLongFormOperation {
    task: RefCell<SynthesisTask>,
    output_location: RefCell<String>,
    is_completed: RefCell<bool>,
}

impl AwsLongFormOperation {
    pub fn new(task: SynthesisTask, output_location: String) -> Self {
        Self {
            task: RefCell::new(task),
            output_location: RefCell::new(output_location),
            is_completed: RefCell::new(false),
        }
    }
}

impl GuestLongFormOperation for AwsLongFormOperation {
    #[doc = " Get operation status"]
    fn get_status(&self) -> OperationStatus {
        todo!()
    }

    #[doc = " Get completion percentage (0-100)"]
    fn get_progress(&self) -> f32 {
        todo!()
    }

    #[doc = " Get result when operation is complete"]
    fn get_result(&self) -> Result<LongFormResult, TtsError> {
        todo!()
    }

    #[doc = " Cancel the operation"]
    fn cancel(&self) -> Result<(), TtsError> {
        todo!()
    }
}
