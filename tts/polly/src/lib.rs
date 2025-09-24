
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
    error::unsupported, polly::Polly, resources::{AwsLongFormOperation, AwsPronunciationLexicon, AwsVoice, AwsVoiceConversionStream, AwsVoiceResults}, types::ListVoiceResponse
};

pub mod auth;
pub mod error;
pub mod polly;
pub mod resources;
pub mod types;
pub mod utils;

pub struct AwsPollyComponent;


impl VoicesGuest for AwsPollyComponent {
    type Voice = AwsVoice;

    type VoiceResults =  AwsVoiceResults;

    #[doc = " List available voices with filtering and pagination"]
    fn list_voices(filter:Option<VoiceFilter>,) -> Result<VoiceResults,TtsError> {
        let polly  = Polly::new()?;
         polly.list_voices(filter)
    }

    #[doc = " Get specific voice by ID"]
    fn get_voice(voice_id:String,) -> Result<Voice,TtsError> {
                let polly  = Polly::new()?;
polly.get_voice(voice_id)
    }

    #[doc = " Search voices by characteristics"]
    fn search_voices(query:String,filter:Option<VoiceFilter>,) -> Result<Vec::<VoiceInfo>,TtsError> {
        unsupported("Voice search not supported by aws  polly use list_voices instead")
    }

    #[doc = " Get supported languages"]
    fn list_languages() -> Result<Vec::<LanguageInfo>,TtsError> {
                let polly  = Polly::new()?;
polly.list_languages()
    }
}

impl SynthesisGuest for AwsPollyComponent{
    #[doc = " Convert text to speech (removed async)"]
    fn synthesize(input:TextInput,voice:VoiceBorrow<'_>,options:Option<SynthesisOptions>,) -> Result<SynthesisResult,TtsError> {
                let polly  = Polly::new()?;
                let voice_name = voice.get::<AwsVoice>().name.clone();
                polly.synthesize(input, voice_name, options)
    }
    
    #[doc = " Batch synthesis for multiple inputs (removed async)"]
    fn synthesize_batch(inputs:Vec::<TextInput>,voice:VoiceBorrow<'_>,options:Option<SynthesisOptions>,) -> Result<Vec::<SynthesisResult>,TtsError> {
                let polly  = Polly::new()?;
                let voice_name = voice.get::<AwsVoice>().name.clone();
                polly.synthesize_batch(inputs, voice_name, options)

    }
    
    #[doc = " Get timing information without audio synthesis"]
    fn get_timing_marks(input:TextInput,voice:VoiceBorrow<'_>,) -> Result<Vec::<TimingInfo>,TtsError> {
        let polly  = Polly::new()?;
        let voice_name  = voice.get::<AwsVoice>().name.clone();
        polly.get_timing_marks(input, voice_name)
    }
    
    #[doc = " Validate text before synthesis"]
    fn validate_input(input:TextInput,voice:VoiceBorrow<'_>,) -> Result<ValidationResult,TtsError> {
        let polly  = Polly::new()?;
        let voice_name  = voice.get::<AwsVoice>().name.clone();
        polly.validate_input(input, voice_name)
    }
    
}

impl StreamingGuest for AwsPollyComponent {
    type SynthesisStream = TtsStream<Polly>;

    type VoiceConversionStream = AwsVoiceConversionStream;

    #[doc = " Create streaming synthesis session"]
    fn create_stream(voice:VoiceBorrow<'_>,options:Option<SynthesisOptions>,) -> Result<SynthesisStream,TtsError> {
        let voice_name  = voice.get::<AwsVoice>().name.clone();
        Polly::create_stream(voice_name, options)
    }

    #[doc = " Real-time voice conversion streaming"]
    fn create_voice_conversion_stream(target_voice:VoiceBorrow<'_>,options:Option<SynthesisOptions>,) -> Result<VoiceConversionStream,TtsError> {
        let target_voice_name  = target_voice.get::<AwsVoice>().name.clone();
        Polly::create_voice_conversion_stream(target_voice_name, options)
    }
}


impl AdvancedGuest for AwsPollyComponent {
    type PronunciationLexicon = AwsPronunciationLexicon;

    type LongFormOperation = AwsLongFormOperation;

    #[doc = " Voice cloning and creation (removed async)"]
    fn create_voice_clone(name:String,audio_samples:Vec::<AudioSample>,description:Option<String>,) -> Result<Voice,TtsError> {
        let polly  = Polly::new()?;
        polly.create_voice_clone(name, audio_samples, description)
    }

    #[doc = " Design synthetic voice (removed async)"]
    fn design_voice(name:String,characteristics:VoiceDesignParams,) -> Result<Voice,TtsError> {
        let polly  = Polly::new()?;
        polly.design_voice(name, characteristics)
    }

    #[doc = " Voice-to-voice conversion (removed async)"]
    fn convert_voice(input_audio:Vec::<u8>,target_voice:VoiceBorrow<'_>,preserve_timing:Option<bool>,) -> Result<Vec::<u8>,TtsError> {
        let polly  = Polly::new()?;
        let target_voice_name  = target_voice.get::<AwsVoice>().name.clone();
        polly.convert_voice(input_audio, target_voice_name, preserve_timing)
    }

    #[doc = " Generate sound effects from text description (removed async)"]
    fn generate_sound_effect(description:String,duration_seconds:Option<f32>,style_influence:Option<f32>,) -> Result<Vec::<u8>,TtsError> {
        let polly  = Polly::new()?;
        polly.generate_sound_effect(description, duration_seconds, style_influence)
    }

    #[doc = " Create custom pronunciation lexicon"]
    fn create_lexicon(name:String,language:LanguageCode,entries:Option<Vec::<PronunciationEntry>>,) -> Result<PronunciationLexicon,TtsError> {
        let polly  = Polly::new()?;
        polly.create_lexicon(name, language, entries)
    }

    #[doc = " Long-form content synthesis with optimization (removed async)"]
    fn synthesize_long_form(content:String,voice:VoiceBorrow<'_>,output_location:String,chapter_breaks:Option<Vec::<u32>>,) -> Result<LongFormOperation,TtsError> {
        let polly  = Polly::new()?;
        let voice_name  = voice.get::<AwsVoice>().name.clone();
        polly.synthesize_long_form(content, voice_name, output_location, chapter_breaks)
    }
}

impl ExtendedGuest for AwsPollyComponent {
    fn unwrapped_list_voices(
        filter: Option<golem_tts::golem::tts::voices::VoiceFilter>,
    ) -> Result<Self::VoiceResults, golem_tts::golem::tts::types::TtsError> {
        let client = Polly::new()?;
        let list = client.list_voices(filter)?;
          Ok(list.into_inner::<AwsVoiceResults>())
    }

    fn unwrapped_create_sythesis_stream(
        voice: VoiceBorrow<'_>,
        options: Option<SynthesisOptions>,
    ) -> Result<Self::SynthesisStream, TtsError> {
        let client = Polly::new()?;
        let voice_name = voice.get::<AwsVoice>().name.clone();
        Ok(TtsStream::new(client, voice_name, options))
    }
}

type DurableAwsPollyComponent = DurableTTS<AwsPollyComponent>;

golem_tts::export_tts!(DurableAwsPollyComponent with_types_in golem_tts);
