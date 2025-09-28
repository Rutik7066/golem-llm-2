// Synthesis feature: durability and passthrough implementations

use crate::durability::{DurableTTS, ExtendedGuest};
use crate::golem::tts::streaming::TimingInfo;
use crate::golem::tts::synthesis::{
    Guest as SynthesisGuest, SynthesisOptions, SynthesisResult, ValidationResult,
};
use crate::golem::tts::types::{TextInput, TtsError};
use crate::golem::tts::voices::VoiceBorrow;
use crate::init_logging;

// ============================
// Passthrough implementation
// ============================
#[cfg(not(feature = "durability"))]
mod passthrough_impl {
    use super::*;

    impl<Impl: ExtendedGuest> SynthesisGuest for DurableTTS<Impl> {
        fn synthesize(
            input: TextInput,
            voice: VoiceBorrow<'_>,
            options: Option<SynthesisOptions>,
        ) -> Result<SynthesisResult, TtsError> {
            init_logging();
            Impl::synthesize(input, voice, options)
        }

        fn synthesize_batch(
            inputs: Vec<TextInput>,
            voice: VoiceBorrow<'_>,
            options: Option<SynthesisOptions>,
        ) -> Result<Vec<SynthesisResult>, TtsError> {
            init_logging();
            Impl::synthesize_batch(inputs, voice, options)
        }

        fn get_timing_marks(
            input: TextInput,
            voice: VoiceBorrow<'_>,
        ) -> Result<Vec<TimingInfo>, TtsError> {
            init_logging();
            Impl::get_timing_marks(input, voice)
        }

        fn validate_input(
            input: TextInput,
            voice: VoiceBorrow<'_>,
        ) -> Result<ValidationResult, TtsError> {
            init_logging();
            Impl::validate_input(input, voice)
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
    struct SynthesizeInput {
        input: TextInput,
        options: Option<SynthesisOptions>,
    }

    #[derive(Debug, Clone, golem_rust::IntoValue)]
    struct SynthesizeBatchInput {
        inputs: Vec<TextInput>,
        options: Option<SynthesisOptions>,
    }

    #[derive(Debug, Clone, golem_rust::IntoValue)]
    struct GetTimingMarksInput {
        input: TextInput,
    }

    #[derive(Debug, Clone, golem_rust::IntoValue)]
    struct ValidateInputInput {
        input: TextInput,
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

    impl<Impl: ExtendedGuest> SynthesisGuest for DurableTTS<Impl> {
        fn synthesize(
            input: TextInput,
            voice: VoiceBorrow<'_>,
            options: Option<SynthesisOptions>,
        ) -> Result<SynthesisResult, TtsError> {
            init_logging();

            let durability = Durability::<NoOutput, UnusedError>::new(
                "golem_tts",
                "synthesize",
                DurableFunctionType::WriteRemote,
            );

            if durability.is_live() {
                let result = with_persistence_level(PersistenceLevel::PersistNothing, || {
                    Impl::synthesize(input.clone(), voice, options.clone())
                });
                let _ = durability.persist_infallible(SynthesizeInput { input, options }, NoOutput);
                result
            } else {
                let _: NoOutput = durability.replay_infallible();
                Impl::synthesize(input, voice, options)
            }
        }

        fn synthesize_batch(
            inputs: Vec<TextInput>,
            voice: VoiceBorrow<'_>,
            options: Option<SynthesisOptions>,
        ) -> Result<Vec<SynthesisResult>, TtsError> {
            init_logging();

            let durability = Durability::<NoOutput, UnusedError>::new(
                "golem_tts",
                "synthesize_batch",
                DurableFunctionType::WriteRemote,
            );

            if durability.is_live() {
                let result = with_persistence_level(PersistenceLevel::PersistNothing, || {
                    Impl::synthesize_batch(inputs.clone(), voice, options.clone())
                });
                let _ = durability
                    .persist_infallible(SynthesizeBatchInput { inputs, options }, NoOutput);
                result
            } else {
                let _: NoOutput = durability.replay_infallible();
                Impl::synthesize_batch(inputs, voice, options)
            }
        }

        fn get_timing_marks(
            input: TextInput,
            voice: VoiceBorrow<'_>,
        ) -> Result<Vec<TimingInfo>, TtsError> {
            init_logging();

            let durability = Durability::<Result<Vec<TimingInfo>, TtsError>, UnusedError>::new(
                "golem_tts",
                "get_timing_marks",
                DurableFunctionType::ReadRemote,
            );

            if durability.is_live() {
                let result = with_persistence_level(PersistenceLevel::PersistNothing, || {
                    Impl::get_timing_marks(input.clone(), voice)
                });
                durability.persist_infallible(GetTimingMarksInput { input }, result)
            } else {
                durability.replay_infallible()
            }
        }

        fn validate_input(
            input: TextInput,
            voice: VoiceBorrow<'_>,
        ) -> Result<ValidationResult, TtsError> {
            init_logging();

            let durability = Durability::<Result<ValidationResult, TtsError>, UnusedError>::new(
                "golem_tts",
                "validate_input",
                DurableFunctionType::ReadRemote,
            );

            if durability.is_live() {
                let result = with_persistence_level(PersistenceLevel::PersistNothing, || {
                    Impl::validate_input(input.clone(), voice)
                });
                durability.persist_infallible(ValidateInputInput { input }, result)
            } else {
                durability.replay_infallible()
            }
        }
    }
}
