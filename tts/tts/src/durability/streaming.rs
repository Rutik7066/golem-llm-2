use crate::durability::DurableTTS;
use crate::golem::tts::streaming::{
    AudioChunk, Guest as StreamingGuest, GuestSynthesisStream, GuestVoiceConversionStream,
    StreamStatus, SynthesisStream, VoiceConversionStream,
};
use crate::golem::tts::synthesis::SynthesisOptions;
use crate::golem::tts::types::{TextInput, TtsError};
use crate::golem::tts::voices::VoiceBorrow;
use crate::init_logging;
use std::cell::RefCell;

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

#[cfg(feature = "durability")]
mod durable_impl {
    use std::ops::Deref;

    use crate::durability::ExtendedGuest;
    use crate::golem::tts::streaming::TextInput;
    use crate::golem::tts::types::TextType;
    use crate::golem::tts::voices::{self, Guest as VoiceGuest, GuestVoice};

    use super::*;
    use golem_rust::bindings::golem::durability::durability::DurableFunctionType;
    use golem_rust::durability::Durability;
    use golem_rust::{with_persistence_level, FromValueAndType, IntoValue, PersistenceLevel};

    #[derive(Debug, Clone, golem_rust::IntoValue)]
    struct CreateStreamInput {
        options: Option<SynthesisOptions>,
    }

    #[derive(Debug, Clone, golem_rust::IntoValue)]
    struct CreateVoiceConversionStreamInput {
        options: Option<SynthesisOptions>,
    }

    #[derive(Debug, Clone, golem_rust::IntoValue)]
    struct SendAudioInput {
        audio_data: Vec<u8>,
    }

    #[derive(Debug, Clone, golem_rust::FromValueAndType, golem_rust::IntoValue)]
    struct NoInput;

    enum DurableSynthesisStreamState<Impl: ExtendedGuest> {
        Live {
            stream: Impl::SynthesisStream,
            input_buffer: TextInput,
        },
        Replay {
            voice_id: String,
            options: Option<SynthesisOptions>,
            replayed_buffer: TextInput,
            sequence_counter: u32,
        },
    }

    pub struct DurableSynthesisStream<Impl: ExtendedGuest> {
        state: RefCell<Option<DurableSynthesisStreamState<Impl>>>,
    }

    impl<Impl: ExtendedGuest> DurableSynthesisStream<Impl> {
        fn live(stream: Impl::SynthesisStream) -> Self {
            Self {
                state: RefCell::new(Some(DurableSynthesisStreamState::Live {
                    stream,
                    input_buffer: TextInput {
                        content: String::new(),
                        text_type: TextType::Plain,
                        language: None,
                    },
                })),
            }
        }
        fn replay(voice_id: String, options: Option<SynthesisOptions>) -> Self {
            Self {
                state: RefCell::new(Some(DurableSynthesisStreamState::Replay {
                    replayed_buffer: TextInput {
                        content: String::new(),
                        text_type: TextType::Plain,
                        language: None,
                    },
                    sequence_counter: 0,
                    voice_id,
                    options,
                })),
            }
        }
    }

    #[derive(Debug, Clone, FromValueAndType, IntoValue)]
    struct NoOutput;

    #[derive(Debug, FromValueAndType, IntoValue)]
    struct UnusedError;

    impl std::fmt::Display for UnusedError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "UnusedError")
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
                "golem_tts",
                "create_stream",
                DurableFunctionType::WriteRemote,
            );
            if durability.is_live() {
                let result = with_persistence_level(PersistenceLevel::PersistNothing, || {
                    let voice_id = voice.get::<<Impl as VoiceGuest>::Voice>().get_id();
                    let stream =
                        Impl::unwrapped_create_sythesis_stream(voice_id, options.clone(), None);
                    SynthesisStream::new(DurableSynthesisStream::<Impl>::live(stream))
                });

                let _ = durability.persist_infallible(CreateStreamInput { options }, NoOutput);
                Ok(result)
            } else {
                let _: NoOutput = durability.replay_infallible();
                let voice_id = voice.get::<<Impl as VoiceGuest>::Voice>().get_id();
                Ok(SynthesisStream::new(
                    DurableSynthesisStream::<Impl>::replay(voice_id, options),
                ))
            }
        }

        fn create_voice_conversion_stream(
            target_voice: VoiceBorrow<'_>,
            options: Option<SynthesisOptions>,
        ) -> Result<VoiceConversionStream, TtsError> {
            init_logging();
            let durability = Durability::<NoOutput, UnusedError>::new(
                "golem_tts",
                "create_voice_conversion_stream",
                DurableFunctionType::WriteRemote,
            );
            if durability.is_live() {
                let inner_stream = with_persistence_level(PersistenceLevel::PersistNothing, || {
                    match Impl::create_voice_conversion_stream(target_voice, options.clone()) {
                        Ok(stream) => Ok(stream.into_inner()),
                        Err(e) => Err(e),
                    }
                });
                let _ = durability.persist_infallible(
                    CreateVoiceConversionStreamInput {
                        options: options.clone(),
                    },
                    NoOutput,
                );
                match inner_stream {
                    Ok(stream) => Ok(VoiceConversionStream::new(DurableVoiceConversionStream::<
                        Impl,
                    >::live(
                        stream
                    ))),
                    Err(e) => Err(e),
                }
            } else {
                let _: NoOutput = durability.replay_infallible();
                Ok(VoiceConversionStream::new(DurableVoiceConversionStream::<
                    Impl,
                >::replay(
                    options
                )))
            }
        }
    }

    #[derive(Debug, Clone, FromValueAndType, IntoValue)]
    pub struct SendTextInput {
        input: TextInput,
    }

    impl<Impl: ExtendedGuest> GuestSynthesisStream for DurableSynthesisStream<Impl> {
        fn send_text(&self, input: TextInput) -> Result<(), TtsError> {
            let durability = Durability::<TextInput, UnusedError>::new(
                "golem_tts",
                "send_text",
                DurableFunctionType::WriteRemote,
            );
            if durability.is_live() {
                let mut state = self.state.borrow_mut();
                let (result, current_input_buffer, new_stream) = match &mut *state {
                    Some(DurableSynthesisStreamState::Live {
                        stream,
                        input_buffer,
                        ..
                    }) => {
                        let result =
                            with_persistence_level(PersistenceLevel::PersistNothing, || {
                                stream.send_text(input.clone())
                            });

                        input_buffer.content.push_str(&input.content.clone());
                        if input_buffer.language.is_none() {
                            input_buffer.language = input.language;
                            input_buffer.text_type = input.text_type;
                        }
                        (result, input_buffer, None)
                    }
                    Some(DurableSynthesisStreamState::Replay {
                        replayed_buffer,
                        voice_id,
                        options,
                        ..
                    }) => {
                        replayed_buffer.content.push_str(&input.content.clone());

                        if replayed_buffer.language.is_none() {
                            replayed_buffer.language = input.language;
                            replayed_buffer.text_type = input.text_type;
                        }
                        let new_stream = Impl::unwrapped_create_sythesis_stream(
                            voice_id.clone(),
                            options.clone(),
                            None,
                        );
                        let result =
                            with_persistence_level(PersistenceLevel::PersistNothing, || {
                                new_stream.send_text(replayed_buffer.clone())
                            });

                        // after adding this input text to replayed_buffer. it will be used as current_input_buffer.
                        (result, replayed_buffer, Some(new_stream))
                    }

                    _ => unreachable!(),
                };

                if let Some(stream) = new_stream {
                    let mut state = self.state.borrow_mut();
                    *state = Some(DurableSynthesisStreamState::Live {
                        stream,
                        input_buffer: current_input_buffer.clone(),
                    });
                }

                let _ = durability.persist_infallible(NoInput, current_input_buffer.clone());

                result
            } else {
                let replay: TextInput = durability.replay_infallible();
                let mut state = self.state.borrow_mut();
                match &mut *state {
                    Some(DurableSynthesisStreamState::Live { .. }) => {
                        unreachable!(
                            "Durable stream cannot be in live mode during offline replay!!"
                        )
                    }
                    Some(DurableSynthesisStreamState::Replay {
                        replayed_buffer, ..
                    }) => {
                        *replayed_buffer = replay;
                    }

                    None => unreachable!("State is not set in offline"),
                };

                Ok(())
            }
        }

        fn receive_chunk(&self) -> Result<Option<AudioChunk>, TtsError> {
            let durability = Durability::<Option<AudioChunk>, UnusedError>::new(
                "golem_tts",
                "receive_chunk",
                DurableFunctionType::ReadRemote,
            );
            if durability.is_live() {
                let mut state = self.state.borrow_mut();
                let (result, new_stream) = match &mut *state {
                    Some(DurableSynthesisStreamState::Live { stream, .. }) => {
                        let result =
                            with_persistence_level(PersistenceLevel::PersistNothing, || {
                                stream.receive_chunk()
                            });

                        (result, None)
                    }
                    Some(DurableSynthesisStreamState::Replay {
                        voice_id,
                        options,
                        sequence_counter,
                        ..
                    }) => {
                        let new_stream = Impl::unwrapped_create_sythesis_stream(
                            voice_id.clone(),
                            options.clone(),
                            Some(sequence_counter.clone()),
                        );

                        let result =
                            with_persistence_level(PersistenceLevel::PersistNothing, || {
                                new_stream.receive_chunk()
                            });

                        (result, Some(new_stream))
                    }
                    _ => unreachable!(),
                };

                if let Some(stream) = new_stream {
                    let mut state = self.state.borrow_mut();
                    *state = Some(DurableSynthesisStreamState::Live {
                        stream,
                        // at this point input is no longer needed
                        input_buffer: TextInput {
                            content: String::new(),
                            text_type: TextType::Plain,
                            language: None,
                        },
                    });
                }

                let log_entry = result.clone().ok().unwrap_or_default();
                let _ = durability.persist_infallible(NoInput, log_entry);
                result
            } else {
                let replay: Option<AudioChunk> = durability.replay_infallible();
                let mut state = self.state.borrow_mut();
                match &mut *state {
                    Some(DurableSynthesisStreamState::Live { .. }) => unreachable!(
                        "Durable stream cannot be in live mode during offline replay!!"
                    ),
                    Some(DurableSynthesisStreamState::Replay {
                        sequence_counter, ..
                    }) => {
                        if let Some(chunk) = replay.clone() {
                            *sequence_counter = chunk.sequence_number;
                        }
                    }
                    _ => unreachable!(),
                };

                Ok(replay)
            }
        }

        fn finish(&self) -> Result<(), TtsError> {
            let durability = Durability::<TextInput, UnusedError>::new(
                "golem_tts",
                "finish",
                DurableFunctionType::WriteRemote,
            );
            if durability.is_live() {
                let mut state = self.state.borrow_mut();
                let (result, current_input_buffer, new_stream) = match &mut *state {
                    Some(DurableSynthesisStreamState::Live {
                        stream,
                        input_buffer,
                    }) => {
                        let result =
                            with_persistence_level(PersistenceLevel::PersistNothing, || {
                                stream.finish()
                            });
                        // in case of crash internal buffer might get destroyed. so input_buffer needs to be saved to rebuild state.
                        (result, input_buffer.clone(), None)
                    }
                    Some(DurableSynthesisStreamState::Replay {
                        voice_id,
                        options,
                        replayed_buffer,
                        ..
                    }) => with_persistence_level(PersistenceLevel::PersistNothing, || {
                        let new_stream = Impl::unwrapped_create_sythesis_stream(
                            voice_id.clone(),
                            options.clone(),
                            None,
                        );

                        // lost buffered input
                      let _ =  new_stream.send_text(replayed_buffer.clone());

                        // after setting replayed_buffer to internal state we can use it as current_buffer
                        (
                            new_stream.finish(),
                            replayed_buffer.clone(),
                            Some(new_stream),
                        )
                    }),
                    _ => unreachable!(),
                };

                if let Some(stream) = new_stream {
                    let mut state = self.state.borrow_mut();
                    *state = Some(DurableSynthesisStreamState::Live {
                        stream,
                        input_buffer: current_input_buffer.clone(),
                    });
                }

                durability.persist_infallible(NoInput, current_input_buffer.clone());

                result
            } else {
                let replay: TextInput = durability.replay_infallible();
                let mut state = self.state.borrow_mut();
                match &mut *state {
                    Some(DurableSynthesisStreamState::Live { .. }) => {
                        unreachable!(
                            "Durable stream cannot be in live mode during offline replay!!"
                        )
                    }
                    Some(DurableSynthesisStreamState::Replay {
                        replayed_buffer, ..
                    }) => {
                        *replayed_buffer = replay;
                    }
                    _ => unreachable!(),
                };

                Ok(())
            }
        }

        fn close(&self) {
            let durability = Durability::<NoOutput, UnusedError>::new(
                "golem_tts",
                "close",
                DurableFunctionType::WriteRemote,
            );
            if durability.is_live() {
                let mut state = self.state.borrow_mut();
                match &mut *state {
                    Some(DurableSynthesisStreamState::Live { stream, .. }) => {
                        with_persistence_level(PersistenceLevel::PersistNothing, || stream.close());
                    }
                    _ => unreachable!(),
                };

                durability.persist_infallible(NoInput, NoOutput);
            } else {
                let _: NoOutput = durability.replay_infallible();
            }
        }

        fn has_pending_audio(&self) -> bool {
            let durability = Durability::<NoOutput, UnusedError>::new(
                "golem_tts",
                "synthesis_stream_has_pending_audio",
                DurableFunctionType::ReadRemote,
            );
            if durability.is_live() {
                let state = self.state.borrow();
                let result = match &*state {
                    Some(DurableSynthesisStreamState::Live { stream,.. }) => {
                        with_persistence_level(PersistenceLevel::PersistNothing, || {
                            stream.has_pending_audio()
                        })
                    }
                    Some(DurableSynthesisStreamState::Replay { .. }) => {
                        /// This this case we dont have way to rebuild internal state.
                        /// we will have to depend on other calls later to has_pending_audio(). 
                        /// e.g commonly called will be receive_chunk() 
                        /// receive_chunk() return Ok(None) as value for safely avoiding bugs and errors.
                        /// So even if we have pending audio we will return true.
                        true
                    },
                    _ => unreachable!(),
                };
                durability.persist_infallible(NoInput, NoOutput);
                result
            } else {
                let _: NoOutput = durability.replay_infallible();
                false
            }
        }

        fn get_status(&self) -> StreamStatus {
            let durability = Durability::<StreamStatus, UnusedError>::new(
                "golem_tts",
                "synthesis_stream_get_status",
                DurableFunctionType::ReadRemote,
            );
            if durability.is_live() {
                let state = self.state.borrow();
                let result = match &*state {
                    Some(DurableSynthesisStreamState::Live { stream, .. }) => {
                        with_persistence_level(PersistenceLevel::PersistNothing, || {
                            stream.get_status()
                        })
                    }
                    Some(DurableSynthesisStreamState::Replay { .. }) => StreamStatus::Ready,
                    _ => unreachable!(),
                };
                durability.persist_infallible(NoInput, result)
            } else {
                durability.replay_infallible()
            }
        }
    }

    enum DurableVoiceConversionStreamState<Impl: ExtendedGuest> {
        Live { stream: Impl::VoiceConversionStream },
        Replay { _options: Option<SynthesisOptions> },
    }

    pub struct DurableVoiceConversionStream<Impl: ExtendedGuest> {
        state: RefCell<Option<DurableVoiceConversionStreamState<Impl>>>,
    }

    impl<Impl: ExtendedGuest> DurableVoiceConversionStream<Impl> {
        fn live(stream: Impl::VoiceConversionStream) -> Self {
            Self {
                state: RefCell::new(Some(DurableVoiceConversionStreamState::Live { stream })),
            }
        }
        fn replay(options: Option<SynthesisOptions>) -> Self {
            Self {
                state: RefCell::new(Some(DurableVoiceConversionStreamState::Replay {
                    _options: options,
                })),
            }
        }
    }

    impl<Impl: ExtendedGuest> GuestVoiceConversionStream for DurableVoiceConversionStream<Impl> {
        fn send_audio(&self, audio_data: Vec<u8>) -> Result<(), TtsError> {
            let durability = Durability::<Result<(), TtsError>, UnusedError>::new(
                "golem_tts",
                "voice_conversion_stream_send_audio",
                DurableFunctionType::WriteRemote,
            );
            if durability.is_live() {
                let state = self.state.borrow();
                let result = match &*state {
                    Some(DurableVoiceConversionStreamState::Live { stream }) => {
                        with_persistence_level(PersistenceLevel::PersistNothing, || {
                            stream.send_audio(audio_data.clone())
                        })
                    }
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
                "golem_tts",
                "voice_conversion_stream_receive_converted",
                DurableFunctionType::ReadRemote,
            );
            if durability.is_live() {
                let state = self.state.borrow();
                let result = match &*state {
                    Some(DurableVoiceConversionStreamState::Live { stream }) => {
                        with_persistence_level(PersistenceLevel::PersistNothing, || {
                            stream.receive_converted()
                        })
                    }
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
                "golem_tts",
                "voice_conversion_stream_finish",
                DurableFunctionType::WriteRemote,
            );
            if durability.is_live() {
                let state = self.state.borrow();
                let result = match &*state {
                    Some(DurableVoiceConversionStreamState::Live { stream }) => {
                        with_persistence_level(PersistenceLevel::PersistNothing, || stream.finish())
                    }
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
            if let Some(DurableVoiceConversionStreamState::Live { stream }) = &*state {
                stream.close();
            }
        }
    }
}
