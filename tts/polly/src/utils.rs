use std::cell::RefCell;

use golem_tts::golem::tts::{
    advanced::{PronunciationEntry, TtsError, Voice},
    types::VoiceGender,
    voices::{VoiceFilter, VoiceInfo},
};

use crate::{
    resources::{AwsVoice, AwsVoiceResults},
    types::{ListVoiceParam, ListVoiceResponse},
};

impl From<ListVoiceResponse> for AwsVoiceResults {
    fn from(value: ListVoiceResponse) -> Self {
        let voices = value
            .voices
            .iter()
            .map(|voice| {
                let voice_gender = voice.gender.to_lowercase();

                let gender = if voice_gender == "male" {
                    VoiceGender::Male
                } else if voice_gender == "female" {
                    VoiceGender::Female
                } else {
                    VoiceGender::Neutral
                };

                let quality = voice.supported_engines.join(",");

                VoiceInfo {
                    id: voice.id.clone(),
                    name: voice.name.clone(),
                    language: voice.language_code.clone(),
                    additional_languages: voice.additional_language_codes.clone(),
                    gender,
                    quality,
                    description: None,
                    provider: "AWS Polly".to_string(),
                    sample_rate: 0,
                    is_custom: false,
                    is_cloned: false,
                    preview_url: None,
                    use_cases: vec![],
                }
            })
            .collect::<Vec<VoiceInfo>>();

        Self {
            next_token: RefCell::new(value.next_token),
            voices: RefCell::new(voices),
            total_count: RefCell::new(None),
        }
    }
}

impl From<&AwsVoice> for VoiceInfo {
    fn from(voice: &AwsVoice) -> Self {
        let voice_gender = voice.gender.to_lowercase();

        let gender = if voice_gender == "male" {
            VoiceGender::Male
        } else if voice_gender == "female" {
            VoiceGender::Female
        } else {
            VoiceGender::Neutral
        };

        let quality = voice.supported_engines.join(",");

        VoiceInfo {
            id: voice.id.clone(),
            name: voice.name.clone(),
            language: voice.language_code.clone(),
            additional_languages: voice.additional_language_codes.clone(),
            gender,
            quality,
            description: None,
            provider: "AWS Polly".to_string(),
            sample_rate: 0,
            is_custom: false,
            is_cloned: false,
            preview_url: None,
            use_cases: vec![],
        }
    }
}

pub fn create_pls_content(language_code: &str, entries: &[PronunciationEntry]) -> String {
    let mut pls = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<lexicon version="1.0" 
         xmlns="http://www.w3.org/2005/01/pronunciation-lexicon"
         xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" 
         xsi:schemaLocation="http://www.w3.org/2005/01/pronunciation-lexicon 
                             http://www.w3.org/TR/2007/CR-pronunciation-lexicon-20071212/pls.xsd"
         alphabet="ipa" xml:lang="{}">"#,
        language_code
    );

    for entry in entries {
        pls.push_str(&format!(
            r#"
    <lexeme>
        <grapheme>{}</grapheme>
        <phoneme>{}</phoneme>
    </lexeme>"#,
            entry.word, entry.pronunciation
        ));
    }

    pls.push_str("\n</lexicon>");
    pls
}

pub fn parse_s3_location(location: &str) -> Result<(String, String), TtsError> {
    if !location.starts_with("s3://") {
        return Err(TtsError::InvalidConfiguration(
            "AWS Polly requires S3 location for long-form synthesis (s3://bucket/key)".to_string(),
        ));
    }

    let without_prefix = &location[5..];
    let parts: Vec<&str> = without_prefix.splitn(2, '/').collect();

    if parts.len() != 2 {
        return Err(TtsError::InvalidConfiguration(
            "Invalid S3 location format. Expected: s3://bucket/key".to_string(),
        ));
    }

    Ok((parts[0].to_string(), parts[1].to_string()))
}

pub fn estimate_audio_duration(audio_data: &[u8], content_type: &str) -> Option<f32> {
    match content_type {
        "audio/mpeg" | "audio/mp3" => Some(audio_data.len() as f32 / 16000.0),
        "audio/wav" | "audio/pcm" => Some(audio_data.len() as f32 / 44100.0),
        "audio/ogg" | "audio/ogg;codecs=opus" => Some(audio_data.len() as f32 / 48000.0),
        _ => None,
    }
}

pub fn estimate_text_duration(text: &str) -> f32 {
    let word_count = text.split_whitespace().count() as f32;
    word_count / 175.0 * 60.0
}
