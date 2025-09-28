// Advanced feature: durability and passthrough implementations

use crate::durability::{DurableTTS, ExtendedGuest};
use crate::golem::tts::advanced::{
    AudioSample, Guest as AdvancedGuest, GuestLongFormOperation, GuestPronunciationLexicon,
    LanguageCode, LongFormOperation, LongFormResult, OperationStatus, PronunciationEntry,
    PronunciationLexicon, VoiceDesignParams,
};
use crate::golem::tts::types::TtsError;
use crate::golem::tts::voices::{Voice, VoiceBorrow};
use crate::init_logging;
use std::cell::RefCell;

// ============================
// Passthrough implementation
// ============================
#[cfg(not(feature = "durability"))]
mod passthrough_impl {
    use super::*;

    impl<Impl: ExtendedGuest> AdvancedGuest for DurableTTS<Impl> {
        type PronunciationLexicon = Impl::PronunciationLexicon;
        type LongFormOperation = Impl::LongFormOperation;

        fn create_voice_clone(
            name: String,
            audio_samples: Vec<AudioSample>,
            description: Option<String>,
        ) -> Result<Voice, TtsError> {
            init_logging();
            Impl::create_voice_clone(name, audio_samples, description)
        }

        fn design_voice(
            name: String,
            characteristics: VoiceDesignParams,
        ) -> Result<Voice, TtsError> {
            init_logging();
            Impl::design_voice(name, characteristics)
        }

        fn convert_voice(
            input_audio: Vec<u8>,
            target_voice: VoiceBorrow<'_>,
            preserve_timing: Option<bool>,
        ) -> Result<Vec<u8>, TtsError> {
            init_logging();
            Impl::convert_voice(input_audio, target_voice, preserve_timing)
        }

        fn generate_sound_effect(
            description: String,
            duration_seconds: Option<f32>,
            style_influence: Option<f32>,
        ) -> Result<Vec<u8>, TtsError> {
            init_logging();
            Impl::generate_sound_effect(description, duration_seconds, style_influence)
        }

        fn create_lexicon(
            name: String,
            language: LanguageCode,
            entries: Option<Vec<PronunciationEntry>>,
        ) -> Result<PronunciationLexicon, TtsError> {
            init_logging();
            Impl::create_lexicon(name, language, entries)
        }

        fn synthesize_long_form(
            content: String,
            voice: VoiceBorrow<'_>,
            output_location: String,
            chapter_breaks: Option<Vec<u32>>,
        ) -> Result<LongFormOperation, TtsError> {
            init_logging();
            Impl::synthesize_long_form(content, voice, output_location, chapter_breaks)
        }
    }
}

// ============================
// Durability implementation
// ============================
#[cfg(feature = "durability")]
mod durable_impl {
    use super::*;
    use golem_rust::bindings::golem::durability::durability::DurableFunctionType;
    use golem_rust::durability::Durability;
    use golem_rust::{with_persistence_level, PersistenceLevel};

    // Input payloads
    #[derive(Debug, Clone, golem_rust::IntoValue)]
    struct CreateVoiceCloneInput {
        name: String,
        audio_samples: Vec<AudioSample>,
        description: Option<String>,
    }

    #[derive(Debug, Clone, golem_rust::IntoValue)]
    struct DesignVoiceInput {
        name: String,
        characteristics: VoiceDesignParams,
    }

    #[derive(Debug, Clone, golem_rust::IntoValue)]
    struct ConvertVoiceInput {
        input_audio: Vec<u8>,
        preserve_timing: Option<bool>,
    }

    #[derive(Debug, Clone, golem_rust::IntoValue)]
    struct GenerateSoundEffectInput {
        description: String,
        duration_seconds: Option<f32>,
        style_influence: Option<f32>,
    }

    #[derive(Debug, Clone, golem_rust::IntoValue)]
    struct CreateLexiconInput {
        name: String,
        language: LanguageCode,
        entries: Option<Vec<PronunciationEntry>>,
    }

    #[derive(Debug, Clone, golem_rust::IntoValue)]
    struct SynthesizeLongFormInput {
        content: String,
        output_location: String,
        chapter_breaks: Option<Vec<u32>>,
    }

    #[derive(Debug, Clone, golem_rust::IntoValue)]
    struct AddEntryInput {
        word: String,
        pronunciation: String,
    }

    #[derive(Debug, Clone, golem_rust::IntoValue)]
    struct RemoveEntryInput {
        word: String,
    }

    #[derive(Debug, golem_rust::FromValueAndType, golem_rust::IntoValue)]
    struct NoInput;

    #[derive(Debug, Clone, golem_rust::FromValueAndType, golem_rust::IntoValue)]
    struct NoOutput;

    #[derive(Debug, golem_rust::FromValueAndType, golem_rust::IntoValue)]
    struct UnusedError;

    impl std::fmt::Display for UnusedError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "UnusedError")
        }
    }

    enum DurablePronunciationLexiconState<Impl: ExtendedGuest> {
        Live {
            lexicon: Impl::PronunciationLexicon,
        },
        Replay {
            name: String,
            language: LanguageCode,
            entries: Option<Vec<PronunciationEntry>>,
        },
    }

    pub struct DurablePronunciationLexicon<Impl: ExtendedGuest> {
        state: RefCell<Option<DurablePronunciationLexiconState<Impl>>>,
    }

    impl<Impl: ExtendedGuest> DurablePronunciationLexicon<Impl> {
        fn live(
            lexicon: Impl::PronunciationLexicon,
            _name: String,
            _language: LanguageCode,
            _entries: Option<Vec<PronunciationEntry>>,
        ) -> Self {
            Self {
                state: RefCell::new(Some(DurablePronunciationLexiconState::Live { lexicon })),
            }
        }
        fn replay(
            name: String,
            language: LanguageCode,
            entries: Option<Vec<PronunciationEntry>>,
        ) -> Self {
            Self {
                state: RefCell::new(Some(DurablePronunciationLexiconState::Replay {
                    name,
                    language,
                    entries,
                })),
            }
        }
    }

    enum DurableLongFormOperationState<Impl: ExtendedGuest> {
        Live {
            operation: Impl::LongFormOperation,
        },
        Replay {
            _content: String,
            _output_location: String,
            _chapter_breaks: Option<Vec<u32>>,
        },
    }

    pub struct DurableLongFormOperation<Impl: ExtendedGuest> {
        state: RefCell<Option<DurableLongFormOperationState<Impl>>>,
    }

    impl<Impl: ExtendedGuest> DurableLongFormOperation<Impl> {
        fn live(
            operation: Impl::LongFormOperation,
            _content: String,
            _output_location: String,
            _chapter_breaks: Option<Vec<u32>>,
        ) -> Self {
            Self {
                state: RefCell::new(Some(DurableLongFormOperationState::Live { operation })),
            }
        }
        fn replay(
            content: String,
            output_location: String,
            chapter_breaks: Option<Vec<u32>>,
        ) -> Self {
            Self {
                state: RefCell::new(Some(DurableLongFormOperationState::Replay {
                    _content: content,
                    _output_location: output_location,
                    _chapter_breaks: chapter_breaks,
                })),
            }
        }
    }

    impl<Impl: ExtendedGuest> GuestPronunciationLexicon for DurablePronunciationLexicon<Impl> {
        fn get_name(&self) -> String {
            let durability = Durability::<String, UnusedError>::new(
                "golem_tts",
                "pronunciation_lexicon_get_name",
                DurableFunctionType::ReadRemote,
            );
            if durability.is_live() {
                let state = self.state.borrow();
                let result = match &*state {
                    Some(DurablePronunciationLexiconState::Live { lexicon }) => {
                        with_persistence_level(PersistenceLevel::PersistNothing, || {
                            lexicon.get_name()
                        })
                    }
                    Some(DurablePronunciationLexiconState::Replay { name, .. }) => name.clone(),
                    _ => unreachable!(),
                };
                durability.persist_infallible(NoInput, result)
            } else {
                durability.replay_infallible()
            }
        }

        fn get_language(&self) -> LanguageCode {
            let durability = Durability::<LanguageCode, UnusedError>::new(
                "golem_tts",
                "pronunciation_lexicon_get_language",
                DurableFunctionType::ReadRemote,
            );
            if durability.is_live() {
                let state = self.state.borrow();
                let result = match &*state {
                    Some(DurablePronunciationLexiconState::Live { lexicon }) => {
                        with_persistence_level(PersistenceLevel::PersistNothing, || {
                            lexicon.get_language()
                        })
                    }
                    Some(DurablePronunciationLexiconState::Replay { language, .. }) => {
                        language.clone()
                    }
                    _ => unreachable!(),
                };
                durability.persist_infallible(NoInput, result)
            } else {
                durability.replay_infallible()
            }
        }

        fn get_entry_count(&self) -> u32 {
            let durability = Durability::<u32, UnusedError>::new(
                "golem_tts",
                "pronunciation_lexicon_get_entry_count",
                DurableFunctionType::ReadRemote,
            );
            if durability.is_live() {
                let state = self.state.borrow();
                let result = match &*state {
                    Some(DurablePronunciationLexiconState::Live { lexicon }) => {
                        with_persistence_level(PersistenceLevel::PersistNothing, || {
                            lexicon.get_entry_count()
                        })
                    }
                    Some(DurablePronunciationLexiconState::Replay { entries, .. }) => {
                        entries.as_ref().map(|e| e.len() as u32).unwrap_or(0)
                    }
                    _ => unreachable!(),
                };
                durability.persist_infallible(NoInput, result)
            } else {
                durability.replay_infallible()
            }
        }

        fn add_entry(&self, word: String, pronunciation: String) -> Result<(), TtsError> {
            let durability = Durability::<Result<(), TtsError>, UnusedError>::new(
                "golem_tts",
                "pronunciation_lexicon_add_entry",
                DurableFunctionType::WriteRemote,
            );
            if durability.is_live() {
                let state = self.state.borrow();
                let result = match &*state {
                    Some(DurablePronunciationLexiconState::Live { lexicon }) => {
                        with_persistence_level(PersistenceLevel::PersistNothing, || {
                            lexicon.add_entry(word.clone(), pronunciation.clone())
                        })
                    }
                    _ => unreachable!("Should not be in replay mode during live execution"),
                };
                durability.persist_infallible(
                    AddEntryInput {
                        word,
                        pronunciation,
                    },
                    result.clone(),
                )?;
                result
            } else {
                durability.replay_infallible()
            }
        }

        fn remove_entry(&self, word: String) -> Result<(), TtsError> {
            let durability = Durability::<Result<(), TtsError>, UnusedError>::new(
                "golem_tts",
                "pronunciation_lexicon_remove_entry",
                DurableFunctionType::WriteRemote,
            );
            if durability.is_live() {
                let state = self.state.borrow();
                let result = match &*state {
                    Some(DurablePronunciationLexiconState::Live { lexicon }) => {
                        with_persistence_level(PersistenceLevel::PersistNothing, || {
                            lexicon.remove_entry(word.clone())
                        })
                    }
                    _ => unreachable!("Should not be in replay mode during live execution"),
                };
                durability.persist_infallible(RemoveEntryInput { word }, result.clone())?;
                result
            } else {
                durability.replay_infallible()
            }
        }

        fn export_content(&self) -> Result<String, TtsError> {
            let durability = Durability::<Result<String, TtsError>, UnusedError>::new(
                "golem_tts",
                "pronunciation_lexicon_export_content",
                DurableFunctionType::ReadRemote,
            );
            if durability.is_live() {
                let state = self.state.borrow();
                let result = match &*state {
                    Some(DurablePronunciationLexiconState::Live { lexicon }) => {
                        with_persistence_level(PersistenceLevel::PersistNothing, || {
                            lexicon.export_content()
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
    }

    impl<Impl: ExtendedGuest> GuestLongFormOperation for DurableLongFormOperation<Impl> {
        fn get_status(&self) -> OperationStatus {
            let durability = Durability::<OperationStatus, UnusedError>::new(
                "golem_tts",
                "long_form_operation_get_status",
                DurableFunctionType::ReadRemote,
            );
            if durability.is_live() {
                let state = self.state.borrow();
                let result = match &*state {
                    Some(DurableLongFormOperationState::Live { operation }) => {
                        with_persistence_level(PersistenceLevel::PersistNothing, || {
                            operation.get_status()
                        })
                    }
                    Some(DurableLongFormOperationState::Replay { .. }) => {
                        OperationStatus::Completed
                    }
                    _ => unreachable!(),
                };
                durability.persist_infallible(NoInput, result)
            } else {
                durability.replay_infallible()
            }
        }

        fn get_progress(&self) -> f32 {
            let durability = Durability::<f32, UnusedError>::new(
                "golem_tts",
                "long_form_operation_get_progress",
                DurableFunctionType::ReadRemote,
            );
            if durability.is_live() {
                let state = self.state.borrow();
                let result = match &*state {
                    Some(DurableLongFormOperationState::Live { operation }) => {
                        with_persistence_level(PersistenceLevel::PersistNothing, || {
                            operation.get_progress()
                        })
                    }
                    Some(DurableLongFormOperationState::Replay { .. }) => 100.0,
                    _ => unreachable!(),
                };
                durability.persist_infallible(NoInput, result)
            } else {
                durability.replay_infallible()
            }
        }

        fn cancel(&self) -> Result<(), TtsError> {
            let durability = Durability::<Result<(), TtsError>, UnusedError>::new(
                "golem_tts",
                "long_form_operation_cancel",
                DurableFunctionType::WriteRemote,
            );
            if durability.is_live() {
                let state = self.state.borrow();
                let result = match &*state {
                    Some(DurableLongFormOperationState::Live { operation }) => {
                        with_persistence_level(PersistenceLevel::PersistNothing, || {
                            operation.cancel()
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

        fn get_result(&self) -> Result<LongFormResult, TtsError> {
            let durability = Durability::<Result<LongFormResult, TtsError>, UnusedError>::new(
                "golem_tts",
                "long_form_operation_get_result",
                DurableFunctionType::ReadRemote,
            );
            if durability.is_live() {
                let state = self.state.borrow();
                let result = match &*state {
                    Some(DurableLongFormOperationState::Live { operation }) => {
                        with_persistence_level(PersistenceLevel::PersistNothing, || {
                            operation.get_result()
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
    }

    impl<Impl: ExtendedGuest> AdvancedGuest for DurableTTS<Impl> {
        type PronunciationLexicon = DurablePronunciationLexicon<Impl>;
        type LongFormOperation = DurableLongFormOperation<Impl>;

        fn create_voice_clone(
            name: String,
            audio_samples: Vec<AudioSample>,
            description: Option<String>,
        ) -> Result<Voice, TtsError> {
            init_logging();
            let durability = Durability::<NoOutput, UnusedError>::new(
                "golem_tts",
                "create_voice_clone",
                DurableFunctionType::WriteRemote,
            );
            if durability.is_live() {
                let result = with_persistence_level(PersistenceLevel::PersistNothing, || {
                    Impl::create_voice_clone(
                        name.clone(),
                        audio_samples.clone(),
                        description.clone(),
                    )
                });
                let _ = durability.persist_infallible(
                    CreateVoiceCloneInput {
                        name,
                        audio_samples,
                        description,
                    },
                    NoOutput,
                );
                result
            } else {
                let _: NoOutput = durability.replay_infallible();
                Impl::create_voice_clone(name, audio_samples, description)
            }
        }

        fn design_voice(
            name: String,
            characteristics: VoiceDesignParams,
        ) -> Result<Voice, TtsError> {
            init_logging();
            let durability = Durability::<NoOutput, UnusedError>::new(
                "golem_tts",
                "design_voice",
                DurableFunctionType::WriteRemote,
            );
            if durability.is_live() {
                let result = with_persistence_level(PersistenceLevel::PersistNothing, || {
                    Impl::design_voice(name.clone(), characteristics.clone())
                });
                let _ = durability.persist_infallible(
                    DesignVoiceInput {
                        name,
                        characteristics,
                    },
                    NoOutput,
                );
                result
            } else {
                let _: NoOutput = durability.replay_infallible();
                Impl::design_voice(name, characteristics)
            }
        }

        fn convert_voice(
            input_audio: Vec<u8>,
            target_voice: VoiceBorrow<'_>,
            preserve_timing: Option<bool>,
        ) -> Result<Vec<u8>, TtsError> {
            init_logging();
            let durability = Durability::<NoOutput, UnusedError>::new(
                "golem_tts",
                "convert_voice",
                DurableFunctionType::WriteRemote,
            );
            if durability.is_live() {
                let result = with_persistence_level(PersistenceLevel::PersistNothing, || {
                    Impl::convert_voice(input_audio.clone(), target_voice, preserve_timing)
                });
                let _ = durability.persist_infallible(
                    ConvertVoiceInput {
                        input_audio,
                        preserve_timing,
                    },
                    NoOutput,
                );
                result
            } else {
                let _: NoOutput = durability.replay_infallible();
                Impl::convert_voice(input_audio, target_voice, preserve_timing)
            }
        }

        fn generate_sound_effect(
            description: String,
            duration_seconds: Option<f32>,
            style_influence: Option<f32>,
        ) -> Result<Vec<u8>, TtsError> {
            init_logging();
            let durability = Durability::<Result<Vec<u8>, TtsError>, UnusedError>::new(
                "golem_tts",
                "generate_sound_effect",
                DurableFunctionType::WriteRemote,
            );
            if durability.is_live() {
                let result = with_persistence_level(PersistenceLevel::PersistNothing, || {
                    Impl::generate_sound_effect(
                        description.clone(),
                        duration_seconds,
                        style_influence,
                    )
                });
                durability.persist_infallible(
                    GenerateSoundEffectInput {
                        description,
                        duration_seconds,
                        style_influence,
                    },
                    result.clone(),
                )?;
                result
            } else {
                durability.replay_infallible()
            }
        }

        fn create_lexicon(
            name: String,
            language: LanguageCode,
            entries: Option<Vec<PronunciationEntry>>,
        ) -> Result<PronunciationLexicon, TtsError> {
            init_logging();
            let durability = Durability::<NoOutput, UnusedError>::new(
                "golem_tts",
                "create_lexicon",
                DurableFunctionType::WriteRemote,
            );
            if durability.is_live() {
                let inner_lexicon =
                    with_persistence_level(PersistenceLevel::PersistNothing, || {
                        match Impl::create_lexicon(name.clone(), language.clone(), entries.clone())
                        {
                            Ok(lexicon) => Ok(lexicon.into_inner()),
                            Err(e) => Err(e),
                        }
                    });
                let _ = durability.persist_infallible(
                    CreateLexiconInput {
                        name: name.clone(),
                        language: language.clone(),
                        entries: entries.clone(),
                    },
                    NoOutput,
                );
                match inner_lexicon {
                    Ok(lexicon) => Ok(PronunciationLexicon::new(
                        DurablePronunciationLexicon::<Impl>::live(lexicon, name, language, entries),
                    )),
                    Err(e) => Err(e),
                }
            } else {
                let _: NoOutput = durability.replay_infallible();
                Ok(PronunciationLexicon::new(
                    DurablePronunciationLexicon::<Impl>::replay(name, language, entries),
                ))
            }
        }

        fn synthesize_long_form(
            content: String,
            voice: VoiceBorrow<'_>,
            output_location: String,
            chapter_breaks: Option<Vec<u32>>,
        ) -> Result<LongFormOperation, TtsError> {
            init_logging();
            let durability = Durability::<NoOutput, UnusedError>::new(
                "golem_tts",
                "synthesize_long_form",
                DurableFunctionType::WriteRemote,
            );
            if durability.is_live() {
                let inner_operation =
                    with_persistence_level(PersistenceLevel::PersistNothing, || {
                        match Impl::synthesize_long_form(
                            content.clone(),
                            voice,
                            output_location.clone(),
                            chapter_breaks.clone(),
                        ) {
                            Ok(operation) => Ok(operation.into_inner()),
                            Err(e) => Err(e),
                        }
                    });
                let _ = durability.persist_infallible(
                    SynthesizeLongFormInput {
                        content: content.clone(),
                        output_location: output_location.clone(),
                        chapter_breaks: chapter_breaks.clone(),
                    },
                    NoOutput,
                );
                match inner_operation {
                    Ok(operation) => Ok(LongFormOperation::new(
                        DurableLongFormOperation::<Impl>::live(
                            operation,
                            content,
                            output_location,
                            chapter_breaks,
                        ),
                    )),
                    Err(e) => Err(e),
                }
            } else {
                let _: NoOutput = durability.replay_infallible();
                Ok(LongFormOperation::new(
                    DurableLongFormOperation::<Impl>::replay(
                        content,
                        output_location,
                        chapter_breaks,
                    ),
                ))
            }
        }
    }
}
