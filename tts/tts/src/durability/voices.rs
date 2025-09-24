// Voices feature: durability and passthrough implementations

use std::cell::RefCell;

use crate::durability::{DurableTTS, ExtendedGuest};
use crate::golem::tts::voices::{
    Guest as VoicesGuest, GuestVoiceResults, LanguageInfo, Voice, VoiceBorrow, VoiceFilter, VoiceInfo,
    VoiceResults,
};
use crate::golem::tts::types::TtsError;
use crate::init_logging;

// ============================
// Passthrough implementation
// ============================
#[cfg(not(feature = "durability"))]
mod passthrough_impl {
    use super::*;

    impl<Impl: ExtendedGuest> VoicesGuest for DurableTTS<Impl> {
        type Voice = Impl::Voice;
        type VoiceResults = Impl::VoiceResults;

        fn list_voices(filter: Option<VoiceFilter>) -> Result<Self::VoiceResults, TtsError> {
            init_logging();
            Impl::list_voices(filter)
        }

        fn get_voice(voice_id: String) -> Result<Voice, TtsError> {
            init_logging();
            Impl::get_voice(voice_id)
        }

        fn search_voices(
            query: String,
            filter: Option<VoiceFilter>,
        ) -> Result<Vec<VoiceInfo>, TtsError> {
            init_logging();
            Impl::search_voices(query, filter)
        }

        fn list_languages() -> Result<Vec<LanguageInfo>, TtsError> {
            init_logging();
            Impl::list_languages()
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

    // Input payloads
    #[derive(Debug, Clone, golem_rust::IntoValue)]
    struct ListVoicesInput {
        filter: Option<VoiceFilter>,
    }

    #[derive(Debug, Clone, golem_rust::IntoValue)]
    struct GetVoiceInput {
        voice_id: String,
    }

    #[derive(Debug, Clone, golem_rust::IntoValue)]
    struct SearchVoicesInput {
        query: String,
        filter: Option<VoiceFilter>,
    }

    #[derive(Debug, Clone, golem_rust::FromValueAndType, golem_rust::IntoValue)]
    struct UnwrappedVoiceResults {
        voice_infos: Vec<VoiceInfo>,
        has_more: bool,
        total_count: Option<u32>,
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

    // Durable voice results wrapper
    enum DurableVoiceResultsState {
        Live {
            results_buffer: Vec<VoiceInfo>,
            current_index: usize,
            has_more: bool,
            total_count: Option<u32>,
        },
        Replay {
            results_buffer: Vec<Vec<VoiceInfo>>,
            current_batch_index: usize,
            finished: bool,
            total_count: Option<u32>,
        },
    }

    pub struct DurableVoiceResults {
        state: RefCell<Option<DurableVoiceResultsState>>,
    }

    impl DurableVoiceResults {
        pub fn live(
            voice_infos: Vec<VoiceInfo>,
            has_more: bool,
            total_count: Option<u32>,
        ) -> DurableVoiceResults {
            Self {
                state: RefCell::new(Some(DurableVoiceResultsState::Live {
                    results_buffer: voice_infos,
                    current_index: 0,
                    has_more,
                    total_count,
                })),
            }
        }

        pub fn replay(
            results_buffer: Vec<Vec<VoiceInfo>>,
            has_more: bool,
            total_count: Option<u32>,
        ) -> DurableVoiceResults {
            Self {
                state: RefCell::new(Some(DurableVoiceResultsState::Replay {
                    results_buffer,
                    current_batch_index: 0,
                    finished: !has_more,
                    total_count,
                })),
            }
        }
    }

    impl GuestVoiceResults for DurableVoiceResults {
        fn has_more(&self) -> bool {
            let durability = Durability::<bool, UnusedError>::new(
                "golem_tts",
                "voice_results_has_more",
                DurableFunctionType::ReadRemote,
            );

            if durability.is_live() {
                let state = self.state.borrow();
                let result = match &*state {
                    Some(DurableVoiceResultsState::Live { has_more, .. }) => *has_more,
                    Some(DurableVoiceResultsState::Replay {
                        results_buffer,
                        current_batch_index,
                        finished,
                        ..
                    }) => !*finished && *current_batch_index < results_buffer.len(),
                    None => unreachable!(),
                };
                durability.persist_infallible(NoInput, result)
            } else {
                let state = self.state.borrow();
                match &*state {
                    Some(DurableVoiceResultsState::Replay {
                        results_buffer,
                        current_batch_index,
                        finished,
                        ..
                    }) => !*finished && *current_batch_index < results_buffer.len(),
                    _ => false,
                }
            }
        }

        fn get_next(&self) -> Result<Vec<VoiceInfo>, TtsError> {
            let durability = Durability::<Result<Vec<VoiceInfo>, TtsError>, UnusedError>::new(
                "golem_tts",
                "voice_results_get_next",
                DurableFunctionType::ReadRemote,
            );

            if durability.is_live() {
                let mut state = self.state.borrow_mut();
                let result = match &mut *state {
                    Some(DurableVoiceResultsState::Live {
                        results_buffer,
                        current_index,
                        has_more,
                        ..
                    }) => {
                        if *current_index < results_buffer.len() {
                            let remaining = results_buffer.len() - *current_index;
                            let batch_size = remaining.min(10);
                            let start = *current_index;
                            let end = start + batch_size;
                            let batch = results_buffer[start..end].to_vec();
                            *current_index = end;
                            Ok(batch)
                        } else if *has_more {
                            Err(TtsError::InternalError(
                                "Unexpected end of buffered results".to_string(),
                            ))
                        } else {
                            Ok(Vec::new())
                        }
                    }
                    Some(DurableVoiceResultsState::Replay { .. }) => {
                        unreachable!("Should not be in replay mode during live execution")
                    }
                    None => unreachable!(),
                };

                durability.persist_infallible(NoInput, result.clone())?;
                result
            } else {
                let mut state = self.state.borrow_mut();
                match &mut *state {
                    Some(DurableVoiceResultsState::Replay {
                        results_buffer,
                        current_batch_index,
                        finished,
                        ..
                    }) => {
                        if *finished {
                            Ok(Vec::new())
                        } else if *current_batch_index < results_buffer.len() {
                            let batch = results_buffer[*current_batch_index].clone();
                            *current_batch_index += 1;
                            if *current_batch_index >= results_buffer.len() {
                                *finished = true;
                            }
                            Ok(batch)
                        } else {
                            *finished = true;
                            Ok(Vec::new())
                        }
                    }
                    _ => Ok(Vec::new()),
                }
            }
        }

        fn get_total_count(&self) -> Option<u32> {
            let durability = Durability::<Option<u32>, UnusedError>::new(
                "golem_tts",
                "voice_results_get_total_count",
                DurableFunctionType::ReadRemote,
            );

            if durability.is_live() {
                let state = self.state.borrow();
                let result = match &*state {
                    Some(DurableVoiceResultsState::Live { total_count, .. }) => *total_count,
                    Some(DurableVoiceResultsState::Replay { .. }) => {
                        unreachable!("Should not be in replay mode during live execution")
                    }
                    None => unreachable!(),
                };
                durability.persist_infallible(NoInput, result)
            } else {
                let state = self.state.borrow();
                match &*state {
                    Some(DurableVoiceResultsState::Replay { total_count, .. }) => *total_count,
                    _ => None,
                }
            }
        }
    }

    impl<Impl: ExtendedGuest> VoicesGuest for DurableTTS<Impl> {
        type Voice = Impl::Voice;
        type VoiceResults = DurableVoiceResults;

        fn list_voices(filter: Option<VoiceFilter>) -> Result<VoiceResults, TtsError> {
            init_logging();

            let durability = Durability::<UnwrappedVoiceResults, UnusedError>::new(
                "golem_tts",
                "list_voices",
                DurableFunctionType::ReadRemote,
            );

            if durability.is_live() {
                // Fetch all results upfront for durability
                let (voice_infos, has_more, total_count) = with_persistence_level(
                    PersistenceLevel::PersistNothing,
                    || {
                        let result = Impl::unwrapped_list_voices(filter.clone())?;
                        let mut all_results = Vec::new();
                        let temp_results = result;
                        let mut has_more = temp_results.has_more();

                        while has_more {
                            match temp_results.get_next() {
                                Ok(batch) => {
                                    all_results.extend(batch);
                                    has_more = temp_results.has_more();
                                }
                                Err(_) => break,
                            }
                        }

                        let total_count = temp_results.get_total_count();
                        Ok((all_results, has_more, total_count))
                    },
                )?;

                let unwrapped_result = UnwrappedVoiceResults {
                    voice_infos: voice_infos.clone(),
                    has_more,
                    total_count,
                };

                durability.persist_infallible(ListVoicesInput { filter }, unwrapped_result);

                Ok(VoiceResults::new(DurableVoiceResults::live(
                    voice_infos, has_more, total_count,
                )))
            } else {
                let unwrapped_result: UnwrappedVoiceResults = durability.replay_infallible();
                Ok(VoiceResults::new(DurableVoiceResults::replay(
                    vec![unwrapped_result.voice_infos],
                    unwrapped_result.has_more,
                    unwrapped_result.total_count,
                )))
            }
        }

        fn get_voice(voice_id: String) -> Result<Voice, TtsError> {
            init_logging();

            let durability = Durability::<NoOutput, UnusedError>::new(
                "golem_tts",
                "get_voice",
                DurableFunctionType::ReadRemote,
            );

            if durability.is_live() {
                let result = with_persistence_level(PersistenceLevel::PersistNothing, || {
                    Impl::get_voice(voice_id.clone())
                });
                let _ = durability.persist_infallible(GetVoiceInput { voice_id }, NoOutput);
                result
            } else {
                let _: NoOutput = durability.replay_infallible();
                Impl::get_voice(voice_id)
            }
        }

        fn search_voices(
            query: String,
            filter: Option<VoiceFilter>,
        ) -> Result<Vec<VoiceInfo>, TtsError> {
            init_logging();

            let durability = Durability::<Result<Vec<VoiceInfo>, TtsError>, UnusedError>::new(
                "golem_tts",
                "search_voices",
                DurableFunctionType::ReadRemote,
            );

            if durability.is_live() {
                let result = with_persistence_level(PersistenceLevel::PersistNothing, || {
                    Impl::search_voices(query.clone(), filter.clone())
                });
                durability.persist_infallible(SearchVoicesInput { query, filter }, result.clone())?;
                result
            } else {
                durability.replay_infallible()
            }
        }

        fn list_languages() -> Result<Vec<LanguageInfo>, TtsError> {
            init_logging();

            let durability = Durability::<Result<Vec<LanguageInfo>, TtsError>, UnusedError>::new(
                "golem_tts",
                "list_languages",
                DurableFunctionType::ReadRemote,
            );

            if durability.is_live() {
                let result = with_persistence_level(PersistenceLevel::PersistNothing, || {
                    Impl::list_languages()
                });
                durability.persist_infallible(NoInput, result.clone())?;
                result
            } else {
                durability.replay_infallible()
            }
        }
    }
}

