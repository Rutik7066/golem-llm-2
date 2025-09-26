use std::io::Error;

use golem_tts::golem::tts::{types::VoiceGender, voices::VoiceInfo};

use crate::resources::ElVoice;

impl From<&ElVoice> for VoiceInfo {
    fn from(v: &ElVoice) -> Self {
        let mut languages = vec!["English".to_string()];
        if let Some(vl) = &v.verified_languages {
            for l in vl.iter() {
                languages.push(l.language.clone());
            }
        };

        VoiceInfo {
            id: v.voice_id.clone(),
            name: v.name.clone(),
            language: languages[0].clone(),
            additional_languages: languages,
            gender: VoiceGender::Neutral,
            quality: "standard".to_string(),
            description: v.description.clone(),
            provider: "ElevenLabs".to_string(),
            sample_rate: 0,
            is_custom: v.is_owner.unwrap_or(false),
            is_cloned: v.category.clone().unwrap_or_default().contains("cloned"),
            preview_url: v.preview_url.clone(),
            use_cases: vec![],
        }
    }
}

pub fn estimate_text_duration(text: &str) -> f32 {
    let word_count = text.split_whitespace().count() as f32;
    // ElevenLabs typically speaks at around 150-180 words per minute
    word_count / 165.0 * 60.0
}

/// Add a text form field to multipart body
pub fn add_form_field(
    body: &mut Vec<u8>,
    boundary: &str,
    name: &str,
    value: &str,
) -> Result<(), Error> {
    body.extend_from_slice(format!("--{}\r\n", boundary).as_bytes());
    body.extend_from_slice(
        format!("Content-Disposition: form-data; name=\"{}\"\r\n", name).as_bytes(),
    );
    body.extend_from_slice(b"\r\n");
    body.extend_from_slice(value.as_bytes());
    body.extend_from_slice(b"\r\n");
    Ok(())
}

/// Add a file field to multipart body
pub fn add_file_field(
    body: &mut Vec<u8>,
    boundary: &str,
    name: &str,
    filename: &str,
    content_type: &str,
    data: &[u8],
) -> Result<(), Error> {
    body.extend_from_slice(format!("--{}\r\n", boundary).as_bytes());
    body.extend_from_slice(
        format!(
            "Content-Disposition: form-data; name=\"{}\"; filename=\"{}\"\r\n",
            name, filename
        )
        .as_bytes(),
    );
    body.extend_from_slice(format!("Content-Type: {}\r\n", content_type).as_bytes());
    body.extend_from_slice(b"\r\n");
    body.extend_from_slice(data);
    body.extend_from_slice(b"\r\n");
    Ok(())
}
