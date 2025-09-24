use reqwest::Method;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use golem_tts::golem::tts::advanced::{AudioSample, TtsError, VoiceDesignParams};

use super::ElevenLabsClient;

// Voice cloning request/response structures
#[derive(Serialize, Debug, Clone)]
pub struct AddVoiceRequest {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remove_background_noise: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub labels: Option<String>,
}

#[derive(Deserialize, Debug)]
pub struct AddVoiceResponse {
    pub voice_id: String,
    pub requires_verification: bool,
}

// Sound effects request/response structures
#[derive(Serialize, Debug, Clone)]
pub struct SoundEffectsRequest {
    pub text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_seconds: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_influence: Option<f32>,
}

// PVC create request/response structures
#[derive(Serialize, Debug, Clone)]
pub struct PvcCreateRequest {
    pub name: String,
    pub language: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub labels: Option<std::collections::HashMap<String, String>>,
}

#[derive(Deserialize, Debug)]
pub struct PvcCreateResponse {
    pub voice_id: String,
}

impl ElevenLabsClient {
    /// Create voice clone using instant voice cloning (IVC)
    pub fn create_voice_clone(
        &self,
        name: String,
        audio_samples: Vec<AudioSample>,
        description: Option<String>,
    ) -> Result<AddVoiceResponse, TtsError> {
        if audio_samples.is_empty() {
            return Err(TtsError::InvalidText(
                "At least one audio sample is required for voice cloning".to_string(),
            ));
        }

        // Create multipart form data manually
        let boundary = format!(
            "----boundary{}",
            Uuid::new_v4().to_string().replace("-", "")
        );
        let mut body = Vec::new();

        // Add name field
        self.add_form_field(&mut body, &boundary, "name", &name)?;

        // Add description if provided
        if let Some(desc) = description {
            self.add_form_field(&mut body, &boundary, "description", &desc)?;
        }

        // Add remove_background_noise field
        self.add_form_field(&mut body, &boundary, "remove_background_noise", "false")?;

        // Add audio files
        for (i, sample) in audio_samples.iter().enumerate() {
            let filename = format!("sample_{}.wav", i);
            self.add_file_field(
                &mut body,
                &boundary,
                "files[]",
                &filename,
                "audio/wav",
                &sample.data,
            )?;
        }

        // Add final boundary
        body.extend_from_slice(format!("--{}--\r\n", boundary).as_bytes());

        let url = format!("{}/v1/voices/add", self.base_url);
        let request = self
            .client
            .post(&url)
            .header("xi-api-key", &self.api_key)
            .header(
                "Content-Type",
                format!("multipart/form-data; boundary={}", boundary),
            )
            .body(body);

        match request.send() {
            Ok(response) => {
                if response.status().is_success() {
                    response.json::<AddVoiceResponse>().map_err(|e| {
                        TtsError::InternalError(format!("Failed to parse response: {}", e))
                    })
                } else {
                    Err(crate::client::error::from_http_error(response))
                }
            }
            Err(err) => Err(TtsError::NetworkError(format!("Request failed: {}", err))),
        }
    }

    /// Generate sound effects from text description
    pub fn generate_sound_effect(
        &self,
        description: String,
        duration_seconds: Option<f32>,
        style_influence: Option<f32>,
    ) -> Result<Vec<u8>, TtsError> {
        let request = SoundEffectsRequest {
            text: description,
            duration_seconds,
            prompt_influence: style_influence,
        };

        self.request_binary::<SoundEffectsRequest, ()>(
            Method::POST,
            "/v1/sound-generation",
            Some(request),
            None,
        )
    }

    /// Create a PVC voice using the characteristics
    pub fn design_voice(
        &self,
        name: String,
        characteristics: VoiceDesignParams,
    ) -> Result<PvcCreateResponse, TtsError> {
        // Create voice description from characteristics
        let mut description_parts = Vec::new();

        // Add gender
        match characteristics.gender {
            golem_tts::golem::tts::types::VoiceGender::Male => {
                description_parts.push("male voice".to_string())
            }
            golem_tts::golem::tts::types::VoiceGender::Female => {
                description_parts.push("female voice".to_string())
            }
            golem_tts::golem::tts::types::VoiceGender::Neutral => {
                description_parts.push("neutral voice".to_string())
            }
        }

        // Add age category
        let age_desc = match characteristics.age_category {
            golem_tts::golem::tts::advanced::AgeCategory::Child => "young child",
            golem_tts::golem::tts::advanced::AgeCategory::YoungAdult => "young adult",
            golem_tts::golem::tts::advanced::AgeCategory::MiddleAged => "middle-aged",
            golem_tts::golem::tts::advanced::AgeCategory::Elderly => "elderly",
        };
        description_parts.push(age_desc.to_string());

        // Add accent if provided
        if !characteristics.accent.is_empty() {
            description_parts.push(format!("with {} accent", characteristics.accent));
        }

        // Add personality traits
        if !characteristics.personality_traits.is_empty() {
            let traits = characteristics.personality_traits.join(", ");
            description_parts.push(format!("personality: {}", traits));
        }

        let description = description_parts.join(", ");

        // Create labels from characteristics
        let mut labels = std::collections::HashMap::new();
        labels.insert(
            "gender".to_string(),
            format!("{:?}", characteristics.gender),
        );
        labels.insert(
            "age".to_string(),
            format!("{:?}", characteristics.age_category),
        );
        if !characteristics.accent.is_empty() {
            labels.insert("accent".to_string(), characteristics.accent.clone());
        }

        let request = PvcCreateRequest {
            name: name.clone(),
            language: "en".to_string(), // Default to English
            description: Some(description),
            labels: Some(labels),
        };

        self.make_request::<PvcCreateResponse, PvcCreateRequest, ()>(
            Method::POST,
            "/v1/voices/pvc",
            Some(request),
            None,
        )
    }

    /// Convert voice using speech-to-speech endpoint
    pub fn convert_voice(
        &self,
        input_audio: Vec<u8>,
        target_voice_id: &str,
        preserve_timing: Option<bool>,
    ) -> Result<Vec<u8>, TtsError> {
        if input_audio.is_empty() {
            return Err(TtsError::InvalidText(
                "Input audio data cannot be empty".to_string(),
            ));
        }

        // Create multipart form data manually
        let boundary = format!(
            "----boundary{}",
            Uuid::new_v4().to_string().replace("-", "")
        );
        let mut body = Vec::new();

        // Add audio file
        self.add_file_field(
            &mut body,
            &boundary,
            "audio",
            "input_audio.wav",
            "audio/wav",
            &input_audio,
        )?;

        // Add model_id field
        self.add_form_field(&mut body, &boundary, "model_id", "eleven_english_sts_v2")?;

        // Add output_format field
        self.add_form_field(&mut body, &boundary, "output_format", "mp3_44100_128")?;

        // Add enable_logging field
        self.add_form_field(&mut body, &boundary, "enable_logging", "false")?;

        // Add remove_background_noise field
        self.add_form_field(&mut body, &boundary, "remove_background_noise", "false")?;

        // Add preserve_timing if specified (this might not be a real ElevenLabs parameter)
        if let Some(preserve) = preserve_timing {
            self.add_form_field(
                &mut body,
                &boundary,
                "preserve_timing",
                &preserve.to_string(),
            )?;
        }

        // Add final boundary
        body.extend_from_slice(format!("--{}--\r\n", boundary).as_bytes());

        let url = format!("{}/v1/speech-to-speech/{}", self.base_url, target_voice_id);
        let request = self
            .client
            .post(&url)
            .header("xi-api-key", &self.api_key)
            .header(
                "Content-Type",
                format!("multipart/form-data; boundary={}", boundary),
            )
            .body(body);

        match request.send() {
            Ok(response) => {
                if response.status().is_success() {
                    response
                        .bytes()
                        .map_err(|e| {
                            TtsError::InternalError(format!(
                                "Failed to read binary response: {}",
                                e
                            ))
                        })
                        .map(|bytes| bytes.to_vec())
                } else {
                    Err(crate::client::error::from_http_error(response))
                }
            }
            Err(err) => Err(TtsError::NetworkError(format!("Request failed: {}", err))),
        }
    }

    /// Add a text form field to multipart body
    fn add_form_field(
        &self,
        body: &mut Vec<u8>,
        boundary: &str,
        name: &str,
        value: &str,
    ) -> Result<(), TtsError> {
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
    fn add_file_field(
        &self,
        body: &mut Vec<u8>,
        boundary: &str,
        name: &str,
        filename: &str,
        content_type: &str,
        data: &[u8],
    ) -> Result<(), TtsError> {
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
}
