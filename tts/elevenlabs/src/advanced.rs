use golem_tts::golem::tts::advanced::{
    AudioSample, Guest, GuestLongFormOperation, GuestPronunciationLexicon, LanguageCode,
    LongFormOperation, LongFormResult, OperationStatus, PronunciationEntry, PronunciationLexicon,
    TtsError, Voice, VoiceBorrow, VoiceDesignParams,
};

use crate::{
    client::{error::unsupported, voices::ElVoice, ElevenLabsClient},
    ElevenLabsTtsComponent,
};

impl Guest for ElevenLabsTtsComponent {
    type PronunciationLexicon = ElPronunciationLexicon;

    type LongFormOperation = ElLongFormSynthesis;

    #[doc = " Voice cloning and creation (removed async)"]
    fn create_voice_clone(
        name: String,
        audio_samples: Vec<AudioSample>,
        description: Option<String>,
    ) -> Result<Voice, TtsError> {
        let client = ElevenLabsClient::new()?;
        let response = client.create_voice_clone(name, audio_samples, description)?;

        let voice = client.get_voice(&response.voice_id)?;
        Ok(Voice::new(voice))
    }

    #[doc = " Design synthetic voice (removed async)"]
    fn design_voice(name: String, characteristics: VoiceDesignParams) -> Result<Voice, TtsError> {
        let client = ElevenLabsClient::new()?;
        let response = client.design_voice(name, characteristics)?;

        let voice = client.get_voice(&response.voice_id)?;
        Ok(Voice::new(voice))
    }

    #[doc = " Voice-to-voice conversion (removed async)"]
    fn convert_voice(
        input_audio: Vec<u8>,
        target_voice: VoiceBorrow<'_>,
        preserve_timing: Option<bool>,
    ) -> Result<Vec<u8>, TtsError> {
        let client = ElevenLabsClient::new()?;
        let target_voice_data = target_voice.get::<ElVoice>();
        client.convert_voice(input_audio, &target_voice_data.voice_id, preserve_timing)
    }

    #[doc = " Generate sound effects from text description (removed async)"]
    fn generate_sound_effect(
        description: String,
        duration_seconds: Option<f32>,
        style_influence: Option<f32>,
    ) -> Result<Vec<u8>, TtsError> {
        let client = ElevenLabsClient::new()?;
        client.generate_sound_effect(description, duration_seconds, style_influence)
    }

    #[doc = " Create custom pronunciation lexicon"]
    fn create_lexicon(
        name: String,
        language: LanguageCode,
        entries: Option<Vec<PronunciationEntry>>,
    ) -> Result<PronunciationLexicon, TtsError> {
        let client = ElevenLabsClient::new()?;
        let description = Some(format!(
            "Pronunciation dictionary for {} language",
            match language.as_str() {
                "en" => "English",
                "es" => "Spanish",
                "fr" => "French",
                "de" => "German",
                _ => "multilingual",
            }
        ));

        let response = client.create_lexicon_from_rules(name.clone(), entries, description)?;

        Ok(PronunciationLexicon::new(ElPronunciationLexicon {
            id: response.id,
            name: response.name,
            language,
            version_id: std::cell::RefCell::new(response.version_id),
            rules_count: std::cell::RefCell::new(response.version_rules_num),
        }))
    }

    #[doc = " Long-form content synthesis with optimization (removed async)"]
    fn synthesize_long_form(
        _content: String,
        _voice: VoiceBorrow<'_>,
        _output_location: String,
        _chapter_breaks: Option<Vec<u32>>,
    ) -> Result<LongFormOperation, TtsError> {
        unsupported("Long-form synthesis not yet implemented for ElevenLabs TTS")
    }
}

pub struct ElLongFormSynthesis {}

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
        let client = crate::client::ElevenLabsClient::new()?;
        let response = client.add_lexicon_rule(&self.id, word, pronunciation)?;

        // Update version and rules count
        *self.version_id.borrow_mut() = response.version_id;
        *self.rules_count.borrow_mut() = response.version_rules_num;

        Ok(())
    }

    #[doc = " Remove pronunciation rule"]
    fn remove_entry(&self, word: String) -> Result<(), TtsError> {
        let client = crate::client::ElevenLabsClient::new()?;
        let response = client.remove_lexicon_rule(&self.id, word)?;

        // Update version and rules count
        *self.version_id.borrow_mut() = response.version_id;
        *self.rules_count.borrow_mut() = response.version_rules_num;

        Ok(())
    }

    #[doc = " Export lexicon content"]
    fn export_content(&self) -> Result<String, TtsError> {
        let client = crate::client::ElevenLabsClient::new()?;
        let version_id = self.version_id.borrow();
        client.export_lexicon(&self.id, &version_id)
    }
}
