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
        voices::LanguageInfo,
        voices::{Guest as VoicesGuest, VoiceFilter, VoiceInfo, VoiceResults},
    },
    tts_stream::TtsStream,
};

use crate::{
    deepgram::Deepgram,
    error::unsupported,
    resources::{
        DeepgramLongFormOperation, DeepgramPronunciationLexicon, DeepgramVoice,
        DeepgramVoiceConversionStream, DeepgramVoiceResults,
    },
};

pub mod deepgram;
pub mod error;
pub mod resources;
pub mod utils;

pub struct DeepgramComponent;

impl SynthesisGuest for DeepgramComponent {
    #[doc = " Convert text to speech (removed async)"]
    fn synthesize(
        input: TextInput,
        voice: VoiceBorrow<'_>,
        options: Option<SynthesisOptions>,
    ) -> Result<SynthesisResult, TtsError> {
        let deepgram = Deepgram::new()?;
        let voice = voice.get::<DeepgramVoice>().canonical_name.clone();
        deepgram.synthesize(input, voice, options)
    }

    #[doc = " Batch synthesis for multiple inputs (removed async)"]
    fn synthesize_batch(
        inputs: Vec<TextInput>,
        voice: VoiceBorrow<'_>,
        options: Option<SynthesisOptions>,
    ) -> Result<Vec<SynthesisResult>, TtsError> {
        let deepgram = Deepgram::new()?;
        let voice = voice.get::<DeepgramVoice>().canonical_name.clone();
        deepgram.synthesize_batch(inputs, voice, options)
    }

    #[doc = " Get timing information without audio synthesis"]
    fn get_timing_marks(
        input: TextInput,
        voice: VoiceBorrow<'_>,
    ) -> Result<Vec<TimingInfo>, TtsError> {
        let deepgram = Deepgram::new()?;
        let voice = voice.get::<DeepgramVoice>().canonical_name.clone();
        deepgram.get_timing_marks(input, voice)
    }

    #[doc = " Validate text before synthesis"]
    fn validate_input(
        input: TextInput,
        voice: VoiceBorrow<'_>,
    ) -> Result<ValidationResult, TtsError> {
        let deepgram = Deepgram::new()?;
        let voice = voice.get::<DeepgramVoice>().canonical_name.clone();
        deepgram.validate_input(input, voice)
    }
}

impl StreamingGuest for DeepgramComponent {
    type SynthesisStream = TtsStream<Deepgram>;

    type VoiceConversionStream = DeepgramVoiceConversionStream;

    #[doc = " Create streaming synthesis session"]
    fn create_stream(
        voice: VoiceBorrow<'_>,
        options: Option<SynthesisOptions>,
    ) -> Result<SynthesisStream, TtsError> {
        let voice = voice.get::<DeepgramVoice>().canonical_name.clone();
        Deepgram::create_stream(voice, options)
    }

    #[doc = " Real-time voice conversion streaming"]
    fn create_voice_conversion_stream(
        target_voice: VoiceBorrow<'_>,
        options: Option<SynthesisOptions>,
    ) -> Result<VoiceConversionStream, TtsError> {
        let target_voice = target_voice.get::<DeepgramVoice>().canonical_name.clone();
        Deepgram::create_voice_conversion_stream(target_voice, options)
    }
}

impl AdvancedGuest for DeepgramComponent {
    type PronunciationLexicon = DeepgramPronunciationLexicon;

    type LongFormOperation = DeepgramLongFormOperation;

    #[doc = " Voice cloning and creation (removed async)"]
    fn create_voice_clone(
        name: String,
        audio_samples: Vec<AudioSample>,
        description: Option<String>,
    ) -> Result<Voice, TtsError> {
        let deepgram = Deepgram::new()?;
        deepgram.create_voice_clone(name, audio_samples, description)
    }

    #[doc = " Design synthetic voice (removed async)"]
    fn design_voice(name: String, characteristics: VoiceDesignParams) -> Result<Voice, TtsError> {
        let deepgram = Deepgram::new()?;
        deepgram.design_voice(name, characteristics)
    }

    #[doc = " Voice-to-voice conversion (removed async)"]
    fn convert_voice(
        input_audio: Vec<u8>,
        target_voice: VoiceBorrow<'_>,
        preserve_timing: Option<bool>,
    ) -> Result<Vec<u8>, TtsError> {
        let deepgram = Deepgram::new()?;
        let target_voice = target_voice.get::<DeepgramVoice>().canonical_name.clone();
        deepgram.convert_voice(input_audio, target_voice, preserve_timing)
    }

    #[doc = " Generate sound effects from text description (removed async)"]
    fn generate_sound_effect(
        description: String,
        duration_seconds: Option<f32>,
        style_influence: Option<f32>,
    ) -> Result<Vec<u8>, TtsError> {
        let deepgram = Deepgram::new()?;
        deepgram.generate_sound_effect(description, duration_seconds, style_influence)
    }

    #[doc = " Create custom pronunciation lexicon"]
    fn create_lexicon(
        name: String,
        language: LanguageCode,
        entries: Option<Vec<PronunciationEntry>>,
    ) -> Result<PronunciationLexicon, TtsError> {
        let deepgram = Deepgram::new()?;
        deepgram.create_lexicon(name, language, entries)
    }

    #[doc = " Long-form content synthesis with optimization (removed async)"]
    fn synthesize_long_form(
        content: String,
        voice: VoiceBorrow<'_>,
        output_location: String,
        chapter_breaks: Option<Vec<u32>>,
    ) -> Result<LongFormOperation, TtsError> {
        let deepgram = Deepgram::new()?;
        let voice = voice.get::<DeepgramVoice>().canonical_name.clone();
        deepgram.synthesize_long_form(content, voice, output_location, chapter_breaks)
    }
}

impl VoicesGuest for DeepgramComponent {
    type Voice = DeepgramVoice;

    type VoiceResults = DeepgramVoiceResults;

    #[doc = " List available voices with filtering and pagination"]
    fn list_voices(filter: Option<VoiceFilter>) -> Result<VoiceResults, TtsError> {
        let deepgram = Deepgram::new()?;
        deepgram.list_voices(filter)
    }

    #[doc = " Get specific voice by ID"]
    fn get_voice(voice_id: String) -> Result<Voice, TtsError> {
        let deepgram = Deepgram::new()?;
        deepgram.get_voice(voice_id)
    }

    #[doc = " Search voices by characteristics"]
    fn search_voices(
        query: String,
        filter: Option<VoiceFilter>,
    ) -> Result<Vec<VoiceInfo>, TtsError> {
        unsupported("Deepgram does not supports voice searching use list_voices instead.")
    }

    #[doc = " Get supported languages"]
    fn list_languages() -> Result<Vec<LanguageInfo>, TtsError> {
        let deepgram = Deepgram::new()?;
        deepgram.list_languages()
    }
}

impl ExtendedGuest for DeepgramComponent {
    fn unwrapped_list_voices(filter: Option<VoiceFilter>) -> Result<Self::VoiceResults, TtsError> {
        let google = Deepgram::new()?;
        let result = google.list_voices(filter)?;
        Ok(result.into_inner::<DeepgramVoiceResults>())
    }

    fn unwrapped_create_sythesis_stream(
        voice: VoiceBorrow<'_>,
        options: Option<SynthesisOptions>,
    ) -> Result<Self::SynthesisStream, TtsError> {
        let google = Deepgram::new()?;
        let deepgram_voice = voice.get::<DeepgramVoice>().canonical_name.clone();
        Ok(TtsStream::new(google, deepgram_voice.clone(), options))
    }
}

type DurableDeepgramComponent = DurableTTS<DeepgramComponent>;

golem_tts::export_tts!(DurableDeepgramComponent with_types_in golem_tts);
