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
    elevenlabs::Elevenlabs,
    error::unsupported,
    resources::{
        ElLongFormSynthesis, ElPronunciationLexicon, ElVoice, ElevenLabsVoiceConversionStream,
        ElevenLabsVoiceResults,
    },
};

pub mod elevenlabs;
pub mod error;
pub mod resources;
pub mod types;
pub mod utils;

pub struct ElevenLabsTtsComponent;

impl SynthesisGuest for ElevenLabsTtsComponent {
    #[doc = " Convert text to speech (removed async)"]
    fn synthesize(
        input: TextInput,
        voice: VoiceBorrow<'_>,
        options: Option<SynthesisOptions>,
    ) -> Result<SynthesisResult, TtsError> {
        let client = Elevenlabs::new()?;
        let voice_name = voice.get::<ElVoice>().name.clone();
        client.synthesize(input, voice_name, options)
    }

    #[doc = " Batch synthesis for multiple inputs (removed async)"]
    fn synthesize_batch(
        inputs: Vec<TextInput>,
        voice: VoiceBorrow<'_>,
        options: Option<SynthesisOptions>,
    ) -> Result<Vec<SynthesisResult>, TtsError> {
        let client = Elevenlabs::new()?;
        let voice_name = voice.get::<ElVoice>().name.clone();
        client.synthesize_batch(inputs, voice_name, options)
    }

    #[doc = " Get timing information without audio synthesis"]
    fn get_timing_marks(
        input: TextInput,
        voice: VoiceBorrow<'_>,
    ) -> Result<Vec<TimingInfo>, TtsError> {
        let client = Elevenlabs::new()?;
        let voice_name = voice.get::<ElVoice>().name.clone();
        client.get_timing_marks(input, voice_name)
    }

    #[doc = " Validate text before synthesis"]
    fn validate_input(
        input: TextInput,
        voice: VoiceBorrow<'_>,
    ) -> Result<ValidationResult, TtsError> {
        let client = Elevenlabs::new()?;
        let voice_name: String = voice.get::<ElVoice>().name.clone();
        client.validate_input(input, voice_name)
    }
}

impl StreamingGuest for ElevenLabsTtsComponent {
    type SynthesisStream = TtsStream<Elevenlabs>;

    type VoiceConversionStream = ElevenLabsVoiceConversionStream;

    #[doc = " Create streaming synthesis session"]
    fn create_stream(
        voice: VoiceBorrow<'_>,
        options: Option<SynthesisOptions>,
    ) -> Result<SynthesisStream, TtsError> {
        let client = Elevenlabs::new()?;
        let voice_name = voice.get::<ElVoice>().name.clone();
        Ok(SynthesisStream::new(TtsStream::new(
            client, voice_name, options,
        )))
    }

    #[doc = " Real-time voice conversion streaming"]
    fn create_voice_conversion_stream(
        target_voice: VoiceBorrow<'_>,
        options: Option<SynthesisOptions>,
    ) -> Result<VoiceConversionStream, TtsError> {
        let voice_name = target_voice.get::<ElVoice>().name.clone();
        Ok(VoiceConversionStream::new(
            ElevenLabsVoiceConversionStream::new(voice_name, options),
        ))
    }
}

impl AdvancedGuest for ElevenLabsTtsComponent {
    type PronunciationLexicon = ElPronunciationLexicon;

    type LongFormOperation = ElLongFormSynthesis;

    #[doc = " Voice cloning and creation (removed async)"]
    fn create_voice_clone(
        name: String,
        audio_samples: Vec<AudioSample>,
        description: Option<String>,
    ) -> Result<Voice, TtsError> {
        let client = Elevenlabs::new()?;
        client.create_voice_clone(name, audio_samples, description)
    }

    #[doc = " Design synthetic voice (removed async)"]
    fn design_voice(name: String, characteristics: VoiceDesignParams) -> Result<Voice, TtsError> {
        let client = Elevenlabs::new()?;
        client.design_voice(name, characteristics)
    }

    #[doc = " Voice-to-voice conversion (removed async)"]
    fn convert_voice(
        input_audio: Vec<u8>,
        target_voice: VoiceBorrow<'_>,
        preserve_timing: Option<bool>,
    ) -> Result<Vec<u8>, TtsError> {
        let client = Elevenlabs::new()?;
        let target_voice_name = target_voice.get::<ElVoice>().name.clone();
        client.convert_voice(input_audio, target_voice_name, preserve_timing)
    }

    #[doc = " Generate sound effects from text description (removed async)"]
    fn generate_sound_effect(
        description: String,
        duration_seconds: Option<f32>,
        style_influence: Option<f32>,
    ) -> Result<Vec<u8>, TtsError> {
        let client = Elevenlabs::new()?;
        client.generate_sound_effect(description, duration_seconds, style_influence)
    }

    #[doc = " Create custom pronunciation lexicon"]
    fn create_lexicon(
        name: String,
        language: LanguageCode,
        entries: Option<Vec<PronunciationEntry>>,
    ) -> Result<PronunciationLexicon, TtsError> {
        let client = Elevenlabs::new()?;
        client.create_lexicon(name, language, entries)
    }

    #[doc = " Long-form content synthesis with optimization (removed async)"]
    fn synthesize_long_form(
        content: String,
        voice: VoiceBorrow<'_>,
        output_location: String,
        chapter_breaks: Option<Vec<u32>>,
    ) -> Result<LongFormOperation, TtsError> {
        unsupported("Long-form content synthesis is not supported by Elvenlabs")
    }
}

impl VoicesGuest for ElevenLabsTtsComponent {
    type Voice = ElVoice;

    type VoiceResults = ElevenLabsVoiceResults;

    #[doc = " List available voices with filtering and pagination"]
    fn list_voices(filter: Option<VoiceFilter>) -> Result<VoiceResults, TtsError> {
        let client = Elevenlabs::new()?;
        client.list_voices(filter)
    }

    #[doc = " Get specific voice by ID"]
    fn get_voice(voice_id: String) -> Result<Voice, TtsError> {
        let client = Elevenlabs::new()?;
        client.get_voice(voice_id)
    }

    #[doc = " Search voices by characteristics"]
    fn search_voices(
        query: String,
        filter: Option<VoiceFilter>,
    ) -> Result<Vec<VoiceInfo>, TtsError> {
        unsupported("Voice search is not supported by elevenlabs")
    }

    #[doc = " Get supported languages"]
    fn list_languages() -> Result<Vec<LanguageInfo>, TtsError> {
        let client = Elevenlabs::new()?;
        client.list_languages()
    }
}

impl ExtendedGuest for ElevenLabsTtsComponent {
    fn unwrapped_list_voices(
        filter: Option<golem_tts::golem::tts::voices::VoiceFilter>,
    ) -> Result<Self::VoiceResults, TtsError> {
        let client = Elevenlabs::new()?;
        let list = client.list_voices(filter)?;
        Ok(list.into_inner::<ElevenLabsVoiceResults>())
    }

    fn unwrapped_create_sythesis_stream(
        voice: VoiceBorrow<'_>,
        options: Option<SynthesisOptions>,
    ) -> Result<Self::SynthesisStream, TtsError> {
        let client = Elevenlabs::new()?;
        let el_voice = voice.get::<ElVoice>().name.clone();
        Ok(TtsStream::new(client, el_voice, options))
    }
}

type DurableElevenLabsTtsComponent = DurableTTS<ElevenLabsTtsComponent>;

golem_tts::export_tts!(DurableElevenLabsTtsComponent with_types_in golem_tts);
