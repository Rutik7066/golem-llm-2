use golem_tts::{
    client::TtsClient,
    durability::{DurableTTS, ExtendedGuest},
    golem::tts::{
        advanced::{
            AudioSample, Guest as AdvancedGuest, LanguageCode, LongFormOperation,
            PronunciationEntry, PronunciationLexicon, TtsError, Voice, VoiceBorrow,
            VoiceDesignParams,
        },
        streaming::{
            Guest as StreamingGuest, SynthesisOptions, SynthesisStream, TextInput, TimingInfo,
            VoiceConversionStream,
        },
        synthesis::{Guest as SynthesisGuest, SynthesisResult, ValidationResult},
        voices::{Guest as VoicesGuest, LanguageInfo, VoiceFilter, VoiceInfo, VoiceResults},
    },
    tts_stream::TtsStream,
};

use crate::{
    google::Google,
    error::unsupported,
    resources::{
        GoogleLongFormOperation, GooglePronunciationLexicon, GoogleVoice,
        GoogleVoiceConversionStream, GoogleVoiceResults,
    },
};

pub mod auth;
pub mod google;
pub mod error;
pub mod resources;
pub mod types;
pub mod utils;

pub struct GoogleTtsComponent;

impl SynthesisGuest for GoogleTtsComponent {
    #[doc = " Convert text to speech (removed async)"]
    fn synthesize(
        input: TextInput,
        voice: VoiceBorrow<'_>,
        options: Option<SynthesisOptions>,
    ) -> Result<SynthesisResult, TtsError> {
        let google = Google::new()?;
        let voice_name = voice.get::<GoogleVoice>().clone().name;
        google.synthesize(input, voice_name, options)
    }

    #[doc = " Batch synthesis for multiple inputs (removed async)"]
    fn synthesize_batch(
        inputs: Vec<TextInput>,
        voice: VoiceBorrow<'_>,
        options: Option<SynthesisOptions>,
    ) -> Result<Vec<SynthesisResult>, TtsError> {
        let google = Google::new()?;
        let voice_name = voice.get::<GoogleVoice>().clone().name;
        google.synthesize_batch(inputs, voice_name, options)
    }

    #[doc = " Get timing information without audio synthesis"]
    fn get_timing_marks(
        input: TextInput,
        voice: VoiceBorrow<'_>,
    ) -> Result<Vec<TimingInfo>, TtsError> {
        let google = Google::new()?;
        let voice_name = voice.get::<GoogleVoice>().clone().name;
        google.get_timing_marks(input, voice_name)
    }

    #[doc = " Validate text before synthesis"]
    fn validate_input(
        input: TextInput,
        voice: VoiceBorrow<'_>,
    ) -> Result<ValidationResult, TtsError> {
        let google = Google::new()?;
        let voice_name = voice.get::<GoogleVoice>().clone().name;
        google.validate_input(input, voice_name)
    }
}

impl StreamingGuest for GoogleTtsComponent {
    type SynthesisStream = TtsStream<Google>;

    type VoiceConversionStream = GoogleVoiceConversionStream;

    #[doc = " Create streaming synthesis session"]
    fn create_stream(
        voice: VoiceBorrow<'_>,
        options: Option<SynthesisOptions>,
    ) -> Result<SynthesisStream, TtsError> {
        let voice_name = voice.get::<GoogleVoice>().clone().name;
        Google::create_stream(voice_name, options)
    }

    #[doc = " Real-time voice conversion streaming"]
    fn create_voice_conversion_stream(
        target_voice: VoiceBorrow<'_>,
        options: Option<SynthesisOptions>,
    ) -> Result<VoiceConversionStream, TtsError> {
        let voice_name = target_voice.get::<GoogleVoice>().clone().name;
        Google::create_voice_conversion_stream(voice_name, options)
    }
}

impl VoicesGuest for GoogleTtsComponent {
    type Voice = GoogleVoice;

    type VoiceResults = GoogleVoiceResults;

    #[doc = " List available voices with filtering and pagination"]
    fn list_voices(filter: Option<VoiceFilter>) -> Result<VoiceResults, TtsError> {
        let google = Google::new()?;
        google.list_voices(filter)
    }

    #[doc = " Get specific voice by ID"]
    fn get_voice(voice_id: String) -> Result<Voice, TtsError> {
        let google = Google::new()?;
        google.get_voice(voice_id)
    }

    #[doc = " Search voices by characteristics"]
    fn search_voices(
        query: String,
        filter: Option<VoiceFilter>,
    ) -> Result<Vec<VoiceInfo>, TtsError> {
      unsupported("Google does not supports voice searching use list_voices instead.")
    }

    #[doc = " Get supported languages"]
    fn list_languages() -> Result<Vec<LanguageInfo>, TtsError> {
        let google = Google::new()?;
        google.list_languages()
    }
}

impl AdvancedGuest for GoogleTtsComponent {
    type PronunciationLexicon = GooglePronunciationLexicon;

    type LongFormOperation = GoogleLongFormOperation;

    #[doc = " Voice cloning and creation (removed async)"]
    fn create_voice_clone(
        name: String,
        audio_samples: Vec<AudioSample>,
        description: Option<String>,
    ) -> Result<Voice, TtsError> {
        unsupported("Google TTS does not support voice cloning")
    }

    #[doc = " Design synthetic voice (removed async)"]
    fn design_voice(name: String, characteristics: VoiceDesignParams) -> Result<Voice, TtsError> {
        let google = Google::new()?;
        google.design_voice(name, characteristics)
    }

    #[doc = " Voice-to-voice conversion (removed async)"]
    fn convert_voice(
        input_audio: Vec<u8>,
        target_voice: VoiceBorrow<'_>,
        preserve_timing: Option<bool>,
    ) -> Result<Vec<u8>, TtsError> {
        unsupported("Google TTS does not support voice conversion")
    }

    #[doc = " Generate sound effects from text description (removed async)"]
    fn generate_sound_effect(
        description: String,
        duration_seconds: Option<f32>,
        style_influence: Option<f32>,
    ) -> Result<Vec<u8>, TtsError> {
        unsupported("Google TTS does not support sound effect generation")
    }

    #[doc = " Create custom pronunciation lexicon"]
    fn create_lexicon(
        name: String,
        language: LanguageCode,
        entries: Option<Vec<PronunciationEntry>>,
    ) -> Result<PronunciationLexicon, TtsError> {
        unsupported("Google TTS does not support custom pronunciation lexicons")
    }

    #[doc = " Long-form content synthesis with optimization (removed async)"]
    fn synthesize_long_form(
        content: String,
        voice: VoiceBorrow<'_>,
        output_location: String,
        chapter_breaks: Option<Vec<u32>>,
    ) -> Result<LongFormOperation, TtsError> {
        let google = Google::new()?;
        let voice_name = voice.get::<GoogleVoice>().clone().name;
        google.synthesize_long_form(content, voice_name, output_location, chapter_breaks)
    }
}

impl ExtendedGuest for GoogleTtsComponent {
    fn unwrapped_list_voices(
        filter: Option<golem_tts::golem::tts::voices::VoiceFilter>,
    ) -> Result<Self::VoiceResults, golem_tts::golem::tts::types::TtsError> {
        let google = Google::new()?;
        let result = google.list_voices(filter)?;
        Ok(result.into_inner::<GoogleVoiceResults>())
    }

    fn unwrapped_create_sythesis_stream(
        voice: VoiceBorrow<'_>,
        options: Option<SynthesisOptions>,
    ) -> Result<Self::SynthesisStream, TtsError> {
        let voice_name = voice.get::<GoogleVoice>().clone().name;
        let google = Google::new()?;

        Ok(TtsStream::new(google, voice_name, options))
    }
}

type DurableGoogleTtsComponent = DurableTTS<GoogleTtsComponent>;

golem_tts::export_tts!(DurableGoogleTtsComponent with_types_in golem_tts);
