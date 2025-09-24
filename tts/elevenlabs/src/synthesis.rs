use golem_tts::golem::tts::{
    advanced::VoiceBorrow,
    streaming::{SynthesisOptions, TextInput},
    synthesis::{Guest, SynthesisResult, TimingInfo, TtsError, ValidationResult},
    types::TextType,
};

use crate::{
    client::{error::unsupported, voices::ElVoice, ElevenLabsClient},
    ElevenLabsTtsComponent,
};

impl Guest for ElevenLabsTtsComponent {
    #[doc = " Convert text to speech (removed async)"]
    fn synthesize(
        input: TextInput,
        voice: VoiceBorrow<'_>,
        options: Option<SynthesisOptions>,
    ) -> Result<SynthesisResult, TtsError> {
        let client = ElevenLabsClient::new()?;
        let v = voice.get::<ElVoice>().clone();
        client.synthesize(input, &v, options, None, None)
    }

    #[doc = " Batch synthesis for multiple inputs (removed async)"]
    fn synthesize_batch(
        inputs: Vec<TextInput>,
        voice: VoiceBorrow<'_>,
        options: Option<SynthesisOptions>,
    ) -> Result<Vec<SynthesisResult>, TtsError> {
        let client = ElevenLabsClient::new()?;
        let v = voice.get::<ElVoice>().clone();
        client.synthesize_batch(inputs, &v, options)
    }

    #[doc = " Get timing information without audio synthesis"]
    fn get_timing_marks(
        _input: TextInput,
        _voice: VoiceBorrow<'_>,
    ) -> Result<Vec<TimingInfo>, TtsError> {
        unsupported("Timing marks without audio synthesis is not supported by ElevenLabs")
    }

    #[doc = " Validate text before synthesis"]
    fn validate_input(
        input: TextInput,
        voice: VoiceBorrow<'_>,
    ) -> Result<ValidationResult, TtsError> {
        let text = input.content;
        let mut warnings = Vec::new();
        let mut errors = Vec::new();

        // Check if text is empty
        if text.trim().is_empty() {
            errors.push("Text cannot be empty".to_string());
        }

        // ElevenLabs has a 5000 character limit for most voices
        if text.len() > 5000 {
            errors
                .push("Text exceeds maximum length of 5000 characters for ElevenLabs".to_string());
        }

        // Check voice validity
        let elevenlabs_voice = voice.get::<ElVoice>();
        if elevenlabs_voice.voice_id.is_empty() {
            errors.push("Voice ID cannot be empty".to_string());
        }

        // SSML validation for ElevenLabs
        if input.text_type == TextType::Ssml {
            if text.trim_start().starts_with('<') {
                if !text.contains("</speak>") || !text.contains("<speak") {
                    errors.push("Invalid SSML format - missing speak tags".to_string());
                }
            } else {
                errors.push(
                    "SSML text type specified but content doesn't start with SSML tags".to_string(),
                );
            }
        }

        // Warn about long text that may impact quality
        if text.len() > 2500 {
            warnings.push("Long text may reduce synthesis quality and speed".to_string());
        }

        // Warn about non-ASCII characters
        if text.chars().any(|c| c as u32 > 127) {
            warnings.push("Non-ASCII characters may affect pronunciation quality".to_string());
        }

        Ok(ValidationResult {
            is_valid: errors.is_empty(),
            errors,
            warnings,
            character_count: text.len() as u32,
            estimated_duration: Some(estimate_text_duration(&text)),
        })
    }
}

fn estimate_text_duration(text: &str) -> f32 {
    let word_count = text.split_whitespace().count() as f32;
    // ElevenLabs typically speaks at around 150-180 words per minute
    word_count / 165.0 * 60.0
}
