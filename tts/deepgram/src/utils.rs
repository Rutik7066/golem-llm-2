use golem_tts::golem::tts::{types::VoiceGender, voices::VoiceInfo};

use crate::resources::DeepgramVoice;

impl From<&DeepgramVoice> for VoiceInfo {
    fn from(v: &DeepgramVoice) -> Self {
        let tags = v.metadata.tags.clone();
        let mut gender = VoiceGender::Neutral;
        for t in tags {
            if t.contains("feminine") {
                gender = VoiceGender::Female;
            } else if t.contains("masculine") {
                gender = VoiceGender::Male;
            }
        }

        let use_case = v.metadata.use_cases.clone().join(",");
        let name = v.name.clone();

        let mut quality = "standard".to_string();
        for q in ["standard", "premium", "neural", "studio"] {
            if v.metadata.tags.contains(&q.to_string()) {
                quality = q.to_string();
                break;
            }
        }

        VoiceInfo {
            id: v.uuid.clone(),
            name: name.clone(),
            language: v.languages[0].clone(),
            additional_languages: v.languages.clone(),
            gender,
            quality: quality.to_string(),
            description: Some(format!("I am {}. I can help you with {}", name, use_case)),
            provider: "Deepgram".to_string(),
            sample_rate: 00, // Sample rate is determined by encoding format
            is_custom: false,
            is_cloned: false,
            preview_url: Some(v.metadata.sample.clone()),
            use_cases: v.metadata.use_cases.clone(),
        }
    }
}

pub fn estimate_duration(text: &str) -> f32 {
    // Rough estimate: ~150 words per minute, ~5 characters per word
    let char_count = text.len() as f32;
    let estimated_words = char_count / 5.0;
    let duration_minutes = estimated_words / 150.0;
    duration_minutes * 60.0 // Convert to seconds
}
