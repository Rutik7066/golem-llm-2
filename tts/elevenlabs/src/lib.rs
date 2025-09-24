use std::cell::RefCell;

use golem_tts::{durability::{DurableTTS, ExtendedGuest}, golem::tts::{advanced::{TtsError, VoiceBorrow}, streaming::SynthesisOptions}};

use crate::{client::{synthesis_stream::ElevenLabsSynthesisStream, voices::{ElVoice, ElVoices}, ElevenLabsClient}, voices::ElevenLabsVoiceResults};

pub mod advanced;
mod client;
pub mod synthesis;
pub mod synthesis_stream;
pub mod voices;

pub struct ElevenLabsTtsComponent;

impl ExtendedGuest for ElevenLabsTtsComponent {
    fn unwrapped_list_voices(filter: Option<golem_tts::golem::tts::voices::VoiceFilter>) -> Result<Self::VoiceResults, TtsError> {
        let client = ElevenLabsClient::new()?;
        let result: ElVoices = client.list_voices(filter.clone(), None)?;
        let voice_result = ElevenLabsVoiceResults {
            has_more: RefCell::new(result.has_more),
            voices: RefCell::new(result.voices),
            total_count: RefCell::new(result.total_count),
            next_page_token: RefCell::new(result.next_page_token),
            options: RefCell::new(filter),
        };
        Ok(voice_result)
    }
    
    fn unwrapped_create_sythesis_stream(
        voice: VoiceBorrow<'_>,
        options: Option<SynthesisOptions>,
    ) -> Result<Self::SynthesisStream, TtsError> {
        let client = ElevenLabsClient::new()?;
        let el_voice = voice.get::<ElVoice>();
        Ok(ElevenLabsSynthesisStream::new(client, el_voice.clone(), options))
    }

    
}

type DurableElevenLabsTtsComponent = DurableTTS<ElevenLabsTtsComponent>;

golem_tts::export_tts!(DurableElevenLabsTtsComponent with_types_in golem_tts);
