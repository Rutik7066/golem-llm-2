// Root of the durability module
// Keep public API stable here (DurableTTS, ExtendedGuest) so providers do not need to change imports

use crate::golem::tts::{
    self,
    advanced::{TtsError, VoiceBorrow},
    streaming::SynthesisOptions,
};
use std::marker::PhantomData;

// Public wrapper type used by provider implementations
pub struct DurableTTS<Impl> {
    pub(crate) phantom: PhantomData<Impl>,
}

impl<Impl> Default for DurableTTS<Impl> {
    fn default() -> Self {
        Self {
            phantom: PhantomData,
        }
    }
}

// Trait to be implemented in addition to the TTS `Guest` trait when wrapping it with `DurableTTS`.
pub trait ExtendedGuest:
    tts::advanced::Guest + tts::streaming::Guest + tts::synthesis::Guest + tts::voices::Guest + 'static
{
    /// Creates an instance of the TTS specific `VoiceResults` without wrapping it in a `Resource`
    fn unwrapped_list_voices(
        filter: Option<tts::voices::VoiceFilter>,
    ) -> Result<Self::VoiceResults, tts::types::TtsError>;


    /// Synthesis streaming helper functoion to be used by the durable implementation. 
    fn unwrapped_create_sythesis_stream(
        voice_id: String,
        options: Option<SynthesisOptions>,
        sequence_counter:Option<u32>,
    ) -> Self::SynthesisStream;


}


// Feature modules (private). They define impls and wrappers.
pub mod advanced;
pub mod streaming;
pub mod synthesis;
pub mod voices;
