// Streaming feature: durability and passthrough implementations

use std::cell::RefCell;
use crate::durability::{DurableTTS, ExtendedGuest};
use crate::golem::tts::streaming::{
    AudioChunk, Guest as StreamingGuest, GuestSynthesisStream, GuestVoiceConversionStream,
    StreamStatus, SynthesisStream, VoiceConversionStream,
};
use crate::golem::tts::synthesis::SynthesisOptions;
use crate::golem::tts::types::{TextInput, TtsError};
use crate::golem::tts::voices::VoiceBorrow;
use crate::init_logging;

// ============================
// Passthrough implementation
// ============================
#[cfg(not(feature = "durability"))]
mod passthrough_impl {
    use super::*;

    impl<Impl: ExtendedGuest> StreamingGuest for DurableTTS<Impl> {
        type SynthesisStream = Impl::SynthesisStream;
        type VoiceConversionStream = Impl::VoiceConversionStream;

        fn create_stream(
            voice: VoiceBorrow<'_>,
            options: Option<SynthesisOptions>,
        ) -> Result<SynthesisStream, TtsError> {
            init_logging();
            Impl::create_stream(voice, options)
        }

        fn create_voice_conversion_stream(
            target_voice: VoiceBorrow<'_>,
            options: Option<SynthesisOptions>,
        ) -> Result<VoiceConversionStream, TtsError> {
            init_logging();
            Impl::create_voice_conversion_stream(target_voice, options)
        }
    }
}

// ============================
// Durability implementation
// ============================
#[cfg(feature = "durability")]
mod durable_impl {
    use super::*;
    use golem_rust::durability::Durability;
    use golem_rust::bindings::golem::durability::durability::DurableFunctionType;
    use golem_rust::{with_persistence_level, PersistenceLevel};

    #[derive(Debug, Clone, golem_rust::IntoValue)]
    struct CreateStreamInput { options: Option<SynthesisOptions> }

    #[derive(Debug, Clone, golem_rust::IntoValue)]
    struct CreateVoiceConversionStreamInput { options: Option<SynthesisOptions> }

    #[derive(Debug, Clone, golem_rust::IntoValue)]
    struct SendTextInput { input: TextInput }

    #[derive(Debug, Clone, golem_rust::IntoValue)]
    struct SendAudioInput { audio_data: Vec<u8> }

    #[derive(Debug, golem_rust::FromValueAndType, golem_rust::IntoValue)]
    struct NoInput;

    #[derive(Debug, Clone, golem_rust::FromValueAndType, golem_rust::IntoValue)]
    struct NoOutput;

    #[derive(Debug, golem_rust::FromValueAndType, golem_rust::IntoValue)]
    struct UnusedError;

    impl std::fmt::Display for UnusedError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { write!(f, "UnusedError") }
    }

    enum DurableSynthesisStreamState<Impl: ExtendedGuest> {
        Live { stream: Impl::SynthesisStream },
        Replay { _options: Option<SynthesisOptions> },
    }

    pub struct DurableSynthesisStream<Impl: ExtendedGuest> {
        state: RefCell<Option<DurableSynthesisStreamState<Impl>>>,
    }

    impl<Impl: ExtendedGuest> DurableSynthesisStream<Impl> {
        fn live(stream: Impl::SynthesisStream) -> Self { Self { state: RefCell::new(Some(DurableSynthesisStreamState::Live { stream })) } }
        fn replay(options: Option<SynthesisOptions>) -> Self { Self { state: RefCell::new(Some(DurableSynthesisStreamState::Replay { _options: options })) } }
    }

    enum DurableVoiceConversionStreamState<Impl: ExtendedGuest> {
        Live { stream: Impl::VoiceConversionStream },
        Replay { _options: Option<SynthesisOptions> },
    }

    pub struct DurableVoiceConversionStream<Impl: ExtendedGuest> {
        state: RefCell<Option<DurableVoiceConversionStreamState<Impl>>>,
    }

    impl<Impl: ExtendedGuest> DurableVoiceConversionStream<Impl> {
        fn live(stream: Impl::VoiceConversionStream) -> Self { Self { state: RefCell::new(Some(DurableVoiceConversionStreamState::Live { stream })) } }
        fn replay(options: Option<SynthesisOptions>) -> Self { Self { state: RefCell::new(Some(DurableVoiceConversionStreamState::Replay { _options: options })) } }
    }

    impl<Impl: ExtendedGuest> GuestVoiceConversionStream for DurableVoiceConversionStream<Impl> {
        fn send_audio(&self, audio_data: Vec<u8>) -> Result<(), TtsError> {
            let durability = Durability::<Result<(), TtsError>, UnusedError>::new(
                "golem_tts", "voice_conversion_stream_send_audio", DurableFunctionType::WriteRemote,
            );
            if durability.is_live() {
                let state = self.state.borrow();
                let result = match &*state {
                    Some(DurableVoiceConversionStreamState::Live { stream }) => with_persistence_level(PersistenceLevel::PersistNothing, || stream.send_audio(audio_data.clone())),
                    _ => unreachable!("Should not be in replay mode during live execution"),
                };
                durability.persist_infallible(SendAudioInput { audio_data }, result.clone())?;
                result
            } else {
                durability.replay_infallible()
            }
        }

        fn receive_converted(&self) -> Result<Option<AudioChunk>, TtsError> {
            let durability = Durability::<Result<Option<AudioChunk>, TtsError>, UnusedError>::new(
                "golem_tts", "voice_conversion_stream_receive_converted", DurableFunctionType::ReadRemote,
            );
            if durability.is_live() {
                let state = self.state.borrow();
                let result = match &*state {
                    Some(DurableVoiceConversionStreamState::Live { stream }) => with_persistence_level(PersistenceLevel::PersistNothing, || stream.receive_converted()),
                    _ => unreachable!("Should not be in replay mode during live execution"),
                };
                durability.persist_infallible(NoInput, result.clone())?;
                result
            } else {
                durability.replay_infallible()
            }
        }

        fn finish(&self) -> Result<(), TtsError> {
            let durability = Durability::<Result<(), TtsError>, UnusedError>::new(
                "golem_tts", "voice_conversion_stream_finish", DurableFunctionType::WriteRemote,
            );
            if durability.is_live() {
                let state = self.state.borrow();
                let result = match &*state {
                    Some(DurableVoiceConversionStreamState::Live { stream }) => with_persistence_level(PersistenceLevel::PersistNothing, || stream.finish()),
                    _ => unreachable!("Should not be in replay mode during live execution"),
                };
                durability.persist_infallible(NoInput, result.clone())?;
                result
            } else {
                durability.replay_infallible()
            }
        }

        fn close(&self) {
            let state = self.state.borrow();
            if let Some(DurableVoiceConversionStreamState::Live { stream }) = &*state { stream.close(); }
        }
    }


    impl<Impl: ExtendedGuest> GuestSynthesisStream for DurableSynthesisStream<Impl> {
        fn send_text(&self, input: TextInput) -> Result<(), TtsError> {
            let durability = Durability::<Result<(), TtsError>, UnusedError>::new(
                "golem_tts", "synthesis_stream_send_text", DurableFunctionType::WriteRemote,
            );
            if durability.is_live() {
                let state = self.state.borrow();
                let result = match &*state {
                    Some(DurableSynthesisStreamState::Live { stream }) => with_persistence_level(PersistenceLevel::PersistNothing, || stream.send_text(input.clone())),
                    Some(DurableSynthesisStreamState::Replay { .. }) => Ok(()),
                    _ => unreachable!(),
                };
                durability.persist_infallible(SendTextInput { input }, result.clone())?;
                result
            } else {
                durability.replay_infallible()
            }
        }

        fn receive_chunk(&self) -> Result<Option<AudioChunk>, TtsError> {
            let durability = Durability::<Result<Option<AudioChunk>, TtsError>, UnusedError>::new(
                "golem_tts", "synthesis_stream_receive_chunk", DurableFunctionType::ReadRemote,
            );
            if durability.is_live() {
                let state = self.state.borrow();
                let result = match &*state {
                    Some(DurableSynthesisStreamState::Live { stream }) => with_persistence_level(PersistenceLevel::PersistNothing, || stream.receive_chunk()),
                    Some(DurableSynthesisStreamState::Replay { .. }) => Ok(None),
                    _ => unreachable!(),
                };
                durability.persist_infallible(NoInput, result.clone())?;
                result
            } else {
                durability.replay_infallible()
            }
        }

        fn finish(&self) -> Result<(), TtsError> {
            let durability = Durability::<Result<(), TtsError>, UnusedError>::new(
                "golem_tts", "synthesis_stream_finish", DurableFunctionType::WriteRemote,
            );
            if durability.is_live() {
                let state = self.state.borrow();
                let result = match &*state {
                    Some(DurableSynthesisStreamState::Live { stream }) => with_persistence_level(PersistenceLevel::PersistNothing, || stream.finish()),
                    Some(DurableSynthesisStreamState::Replay { .. }) => Ok(()),
                    _ => unreachable!(),
                };
                durability.persist_infallible(NoInput, result.clone())?;
                result
            } else {
                durability.replay_infallible()
            }
        }

        fn close(&self) {
            let state = self.state.borrow();
            if let Some(DurableSynthesisStreamState::Live { stream }) = &*state { stream.close(); }
        }

        fn has_pending_audio(&self) -> bool {
            let durability = Durability::<bool, UnusedError>::new(
                "golem_tts", "synthesis_stream_has_pending_audio", DurableFunctionType::ReadRemote,
            );
            if durability.is_live() {
                let state = self.state.borrow();
                let result = match &*state {
                    Some(DurableSynthesisStreamState::Live { stream }) => with_persistence_level(PersistenceLevel::PersistNothing, || stream.has_pending_audio()),
                    Some(DurableSynthesisStreamState::Replay { .. }) => false,
                    _ => unreachable!(),
                };
                durability.persist_infallible(NoInput, result)
            } else {
                durability.replay_infallible()
            }
        }

        fn get_status(&self) -> StreamStatus {
            let durability = Durability::<StreamStatus, UnusedError>::new(
                "golem_tts", "synthesis_stream_get_status", DurableFunctionType::ReadRemote,
            );
            if durability.is_live() {
                let state = self.state.borrow();
                let result = match &*state {
                    Some(DurableSynthesisStreamState::Live { stream }) => with_persistence_level(PersistenceLevel::PersistNothing, || stream.get_status()),
                    Some(DurableSynthesisStreamState::Replay { .. }) => StreamStatus::Ready,
                    _ => unreachable!(),
                };
                durability.persist_infallible(NoInput, result)
            } else {
                durability.replay_infallible()
            }
        }
    }

    impl<Impl: ExtendedGuest> StreamingGuest for DurableTTS<Impl> {
        type SynthesisStream = DurableSynthesisStream<Impl>;
        type VoiceConversionStream = DurableVoiceConversionStream<Impl>;

        fn create_stream(
            voice: VoiceBorrow<'_>,
            options: Option<SynthesisOptions>,
        ) -> Result<SynthesisStream, TtsError> {
            init_logging();
            let durability = Durability::<NoOutput, UnusedError>::new(
                "golem_tts", "create_stream", DurableFunctionType::WriteRemote,
            );
            if durability.is_live() {
                let inner_stream = with_persistence_level(PersistenceLevel::PersistNothing, || {
                    match Impl::create_stream(voice, options.clone()) {
                        Ok(stream) => Ok(stream.into_inner()),
                        Err(e) => Err(e),
                    }
                });
                let _ = durability.persist_infallible(CreateStreamInput { options: options.clone() }, NoOutput);
                match inner_stream {
                    Ok(stream) => Ok(SynthesisStream::new(DurableSynthesisStream::<Impl>::live(stream))),
                    Err(e) => Err(e),
                }
            } else {
                let _: NoOutput = durability.replay_infallible();
                Ok(SynthesisStream::new(DurableSynthesisStream::<Impl>::replay(options)))
            }
        }

        fn create_voice_conversion_stream(
            target_voice: VoiceBorrow<'_>,
            options: Option<SynthesisOptions>,
        ) -> Result<VoiceConversionStream, TtsError> {
            init_logging();
            let durability = Durability::<NoOutput, UnusedError>::new(
                "golem_tts", "create_voice_conversion_stream", DurableFunctionType::WriteRemote,
            );
            if durability.is_live() {
                let inner_stream = with_persistence_level(PersistenceLevel::PersistNothing, || {
                    match Impl::create_voice_conversion_stream(target_voice, options.clone()) {
                        Ok(stream) => Ok(stream.into_inner()),
                        Err(e) => Err(e),
                    }
                });
                let _ = durability.persist_infallible(CreateVoiceConversionStreamInput { options: options.clone() }, NoOutput);
                match inner_stream {
                    Ok(stream) => Ok(VoiceConversionStream::new(DurableVoiceConversionStream::<Impl>::live(stream))),
                    Err(e) => Err(e),
                }
            } else {
                let _: NoOutput = durability.replay_infallible();
                Ok(VoiceConversionStream::new(DurableVoiceConversionStream::<Impl>::replay(options)))
            }
        }
    }
}

