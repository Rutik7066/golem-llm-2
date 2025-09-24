use golem_tts::golem::tts::streaming::{
    AudioChunk, Guest, GuestSynthesisStream, GuestVoiceConversionStream, StreamStatus,
    SynthesisOptions, SynthesisStream, TextInput, TtsError, VoiceBorrow, VoiceConversionStream,
};

use crate::{
    client::{
        synthesis_stream::{ElevenLabsSynthesisStream, ElevenLabsVoiceConversionStream},
        voices::ElVoice,
        ElevenLabsClient,
    },
    ElevenLabsTtsComponent,
};

impl Guest for ElevenLabsTtsComponent {
    type SynthesisStream = ElevenLabsSynthesisStream;
    type VoiceConversionStream = ElevenLabsVoiceConversionStream;

    #[doc = " Create streaming synthesis session"]
    fn create_stream(
        voice: VoiceBorrow<'_>,
        options: Option<SynthesisOptions>,
    ) -> Result<SynthesisStream, TtsError> {
        let client = ElevenLabsClient::new()?;
        let v = voice.get::<ElVoice>().clone();
        Ok(SynthesisStream::new(ElevenLabsSynthesisStream::new(
            client, v, options,
        )))
    }

    #[doc = " Real-time voice conversion streaming"]
    fn create_voice_conversion_stream(
        _target_voice: VoiceBorrow<'_>,
        _options: Option<SynthesisOptions>,
    ) -> Result<VoiceConversionStream, TtsError> {
        Err(TtsError::UnsupportedOperation(
            "Real-time voice conversion streaming is not supported by ElevenLabs TTS".to_string(),
        ))
    }
}

impl GuestVoiceConversionStream for ElevenLabsVoiceConversionStream {
    #[doc = " Send audio for voice conversion"]
    fn send_audio(&self, _audio_data: Vec<u8>) -> Result<(), TtsError> {
        Err(TtsError::UnsupportedOperation(
            "Real-time voice conversion streaming is not supported by ElevenLabs TTS".to_string(),
        ))
    }

    #[doc = " Receive converted audio chunks"]
    fn receive_converted(&self) -> Result<Option<AudioChunk>, TtsError> {
        Err(TtsError::UnsupportedOperation(
            "Real-time voice conversion streaming is not supported by ElevenLabs TTS".to_string(),
        ))
    }

    #[doc = " Signal end of input and flush remaining audio"]
    fn finish(&self) -> Result<(), TtsError> {
        Err(TtsError::UnsupportedOperation(
            "Real-time voice conversion streaming is not supported by ElevenLabs TTS".to_string(),
        ))
    }

    #[doc = " Close stream and clean up resources"]
    fn close(&self) {
        // Nothing to close for unsupported operation
    }
}

impl GuestSynthesisStream for ElevenLabsSynthesisStream {
    #[doc = " Send text for synthesis (can be called multiple times)"]
    fn send_text(&self, input: TextInput) -> Result<(), TtsError> {
        let text = input.content;

        self.add_text(&text);
        Ok(())
    }

    #[doc = " Receive next audio chunk (non-blocking)"]
    fn receive_chunk(&self) -> Result<Option<AudioChunk>, TtsError> {
        if self.audio_buffer.borrow().is_empty() {
            Ok(None)
        } else {
            let audio_data = self.get_buffered_audio();
            let is_final = !self.is_active();
            let mut counter = self.sequence_counter.borrow_mut();
            let seq_num = *counter;
            *counter += 1;
            Ok(Some(AudioChunk {
                data: audio_data,
                sequence_number: seq_num,
                timing_info: None,
                is_final,
            }))
        }
    }

    #[doc = " Check if more chunks are available"]
    fn has_pending_audio(&self) -> bool {
        !self.audio_buffer.borrow().is_empty()
    }

    #[doc = " Get current stream status"]
    fn get_status(&self) -> StreamStatus {
        if self.is_active() {
            StreamStatus::Ready
        } else {
            StreamStatus::Closed
        }
    }

    #[doc = " Signal end of input and flush remaining audio"]
    fn finish(&self) -> Result<(), TtsError> {
        self.synthesize_buffered_text()?;
        self.stop();
        Ok(())
    }

    #[doc = " Close stream and clean up resources"]
    fn close(&self) {
        self.stop();
    }
}
