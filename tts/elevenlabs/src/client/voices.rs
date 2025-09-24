use golem_tts::golem::tts::{
    advanced::TtsError,
    voices::{LanguageInfo, VoiceFilter},
};
use log::trace;
use reqwest::Method;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::client::ElevenLabsClient;

#[derive(Deserialize, Clone, Debug)]
pub struct ElVoices {
    pub voices: Vec<ElVoice>,
    pub has_more: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_count: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_page_token: Option<String>,
}

#[derive(Deserialize, Clone, Debug)]
pub struct ElVoice {
    pub voice_id: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub labels: Option<HashMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preview_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub settings: Option<VoiceSettings>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verified_languages: Option<Vec<VerifiedLanguage>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_owner: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_legacy: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_mixed: Option<bool>,
}

#[derive(Deserialize, Clone, Debug)]
pub struct VoiceSettings {
    pub stability: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub use_speaker_boost: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub similarity_boost: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub style: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub speed: Option<f64>,
}

#[derive(Deserialize, Clone, Debug)]
pub struct VerifiedLanguage {
    pub language: String,
    pub model_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub accent: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub locale: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preview_url: Option<String>,
}

impl ElevenLabsClient {
    pub fn list_voices(
        &self,
        filter: Option<VoiceFilter>,
        next_page_token: Option<String>,
    ) -> Result<ElVoices, TtsError> {
        #[derive(Serialize, Clone, Debug)]
        pub struct ListVoicesQuery {
            #[serde(skip_serializing_if = "Option::is_none")]
            pub next_page_token: Option<String>,
            #[serde(skip_serializing_if = "Option::is_none")]
            pub page_size: Option<i32>,
            #[serde(skip_serializing_if = "Option::is_none")]
            pub search: Option<String>,
            #[serde(skip_serializing_if = "Option::is_none")]
            pub sort: Option<String>,
            #[serde(skip_serializing_if = "Option::is_none")]
            pub sort_direction: Option<String>,
            #[serde(skip_serializing_if = "Option::is_none")]
            pub voice_type: Option<String>,
            #[serde(skip_serializing_if = "Option::is_none")]
            pub category: Option<String>,
            #[serde(skip_serializing_if = "Option::is_none")]
            pub fine_tuning_state: Option<String>,
            #[serde(skip_serializing_if = "Option::is_none")]
            pub collection_id: Option<String>,
            #[serde(skip_serializing_if = "Option::is_none")]
            pub include_total_count: Option<bool>,
            #[serde(skip_serializing_if = "Option::is_none")]
            pub voice_ids: Option<Vec<String>>,
        }

        // Docs: https://elevenlabs.io/docs/api-reference/voices/search
        let path = "/v1/voices";
        let query = if let Some(options) = filter {
            Some(ListVoicesQuery {
                next_page_token,
                page_size: Some(100),
                search: options.search_query,
                sort: Some("name".to_string()),
                sort_direction: Some("asc".to_string()),
                voice_type: None,
                category: None,
                fine_tuning_state: None,
                collection_id: None,
                include_total_count: Some(true),
                voice_ids: None,
            })
        } else {
            None
        };
        trace!("Listing available voices.");
        self.make_request::<ElVoices, (), ListVoicesQuery>(Method::GET, path, None, query)
    }

    pub fn get_voice(&self, voice_id: &str) -> Result<ElVoice, TtsError> {
        // Docs: https://elevenlabs.io/docs/api-reference/voices/get
        let path = format!("/v1/voices/{}", voice_id);
        self.make_request::<ElVoice, (), ()>(Method::GET, &path, None, None)
    }

    pub fn delete_voice(&self, voice_id: &str) -> Result<String, TtsError> {
        let path = format!("/v1/voices/{voice_id}");
        self.make_request::<String, (), ()>(Method::DELETE, &path, None, None)
    }

    pub fn list_languages(&self) -> Result<Vec<LanguageInfo>, TtsError> {
        // English (USA, UK, Australia, Canada), Japanese, Chinese,
        // German, Hindi, French (France, Canada), Korean, Portuguese (Brazil, Portugal),
        // Italian, Spanish (Spain, Mexico), Indonesian, Dutch, Turkish,
        // Filipino, Polish, Swedish, Bulgarian, Romanian,
        // Arabic (Saudi Arabia, UAE), Czech, Greek, Finnish, Croatian, Malay,
        // Slovak, Danish, Tamil, Ukrainian & Russian.
        // Return hardcoded list of LanguageInfo using above Language
        Ok(vec![
            LanguageInfo {
                code: "en-US".to_string(),
                name: "English (US)".to_string(),
                native_name: "English (US)".to_string(),
                voice_count: 100,
            },
            LanguageInfo {
                code: "en-GB".to_string(),
                name: "English (UK)".to_string(),
                native_name: "English (UK)".to_string(),
                voice_count: 80,
            },
            LanguageInfo {
                code: "en-AU".to_string(),
                name: "English (Australia)".to_string(),
                native_name: "English (Australia)".to_string(),
                voice_count: 50,
            },
            LanguageInfo {
                code: "en-CA".to_string(),
                name: "English (Canada)".to_string(),
                native_name: "English (Canada)".to_string(),
                voice_count: 40,
            },
            LanguageInfo {
                code: "ja".to_string(),
                name: "Japanese".to_string(),
                native_name: "日本語".to_string(),
                voice_count: 30,
            },
            LanguageInfo {
                code: "zh-CN".to_string(),
                name: "Chinese (Mandarin)".to_string(),
                native_name: "中文（普通话）".to_string(),
                voice_count: 25,
            },
            LanguageInfo {
                code: "de".to_string(),
                name: "German".to_string(),
                native_name: "Deutsch".to_string(),
                voice_count: 35,
            },
            LanguageInfo {
                code: "hi".to_string(),
                name: "Hindi".to_string(),
                native_name: "हिंदी".to_string(),
                voice_count: 20,
            },
            LanguageInfo {
                code: "fr".to_string(),
                name: "French (France)".to_string(),
                native_name: "Français (France)".to_string(),
                voice_count: 30,
            },
            LanguageInfo {
                code: "fr-CA".to_string(),
                name: "French (Canada)".to_string(),
                native_name: "Français (Canada)".to_string(),
                voice_count: 15,
            },
            LanguageInfo {
                code: "ko".to_string(),
                name: "Korean".to_string(),
                native_name: "한국어".to_string(),
                voice_count: 20,
            },
            LanguageInfo {
                code: "pt-BR".to_string(),
                name: "Portuguese (Brazil)".to_string(),
                native_name: "Português (Brasil)".to_string(),
                voice_count: 25,
            },
            LanguageInfo {
                code: "pt".to_string(),
                name: "Portuguese (Portugal)".to_string(),
                native_name: "Português (Portugal)".to_string(),
                voice_count: 15,
            },
            LanguageInfo {
                code: "it".to_string(),
                name: "Italian".to_string(),
                native_name: "Italiano".to_string(),
                voice_count: 25,
            },
            LanguageInfo {
                code: "es".to_string(),
                name: "Spanish (Spain)".to_string(),
                native_name: "Español (España)".to_string(),
                voice_count: 30,
            },
            LanguageInfo {
                code: "es-MX".to_string(),
                name: "Spanish (Mexico)".to_string(),
                native_name: "Español (México)".to_string(),
                voice_count: 20,
            },
            LanguageInfo {
                code: "id".to_string(),
                name: "Indonesian".to_string(),
                native_name: "Bahasa Indonesia".to_string(),
                voice_count: 10,
            },
            LanguageInfo {
                code: "nl".to_string(),
                name: "Dutch".to_string(),
                native_name: "Nederlands".to_string(),
                voice_count: 15,
            },
            LanguageInfo {
                code: "tr".to_string(),
                name: "Turkish".to_string(),
                native_name: "Türkçe".to_string(),
                voice_count: 15,
            },
            LanguageInfo {
                code: "fil".to_string(),
                name: "Filipino".to_string(),
                native_name: "Filipino".to_string(),
                voice_count: 10,
            },
            LanguageInfo {
                code: "pl".to_string(),
                name: "Polish".to_string(),
                native_name: "Polski".to_string(),
                voice_count: 15,
            },
            LanguageInfo {
                code: "sv".to_string(),
                name: "Swedish".to_string(),
                native_name: "Svenska".to_string(),
                voice_count: 10,
            },
            LanguageInfo {
                code: "bg".to_string(),
                name: "Bulgarian".to_string(),
                native_name: "Български".to_string(),
                voice_count: 8,
            },
            LanguageInfo {
                code: "ro".to_string(),
                name: "Romanian".to_string(),
                native_name: "Română".to_string(),
                voice_count: 8,
            },
            LanguageInfo {
                code: "ar-SA".to_string(),
                name: "Arabic (Saudi Arabia)".to_string(),
                native_name: "العربية (السعودية)".to_string(),
                voice_count: 12,
            },
            LanguageInfo {
                code: "ar-AE".to_string(),
                name: "Arabic (UAE)".to_string(),
                native_name: "العربية (الإمارات)".to_string(),
                voice_count: 10,
            },
            LanguageInfo {
                code: "cs".to_string(),
                name: "Czech".to_string(),
                native_name: "Čeština".to_string(),
                voice_count: 8,
            },
            LanguageInfo {
                code: "el".to_string(),
                name: "Greek".to_string(),
                native_name: "Ελληνικά".to_string(),
                voice_count: 8,
            },
            LanguageInfo {
                code: "fi".to_string(),
                name: "Finnish".to_string(),
                native_name: "Suomi".to_string(),
                voice_count: 8,
            },
            LanguageInfo {
                code: "hr".to_string(),
                name: "Croatian".to_string(),
                native_name: "Hrvatski".to_string(),
                voice_count: 8,
            },
            LanguageInfo {
                code: "ms".to_string(),
                name: "Malay".to_string(),
                native_name: "Bahasa Melayu".to_string(),
                voice_count: 8,
            },
            LanguageInfo {
                code: "sk".to_string(),
                name: "Slovak".to_string(),
                native_name: "Slovenčina".to_string(),
                voice_count: 8,
            },
            LanguageInfo {
                code: "da".to_string(),
                name: "Danish".to_string(),
                native_name: "Dansk".to_string(),
                voice_count: 8,
            },
            LanguageInfo {
                code: "ta".to_string(),
                name: "Tamil".to_string(),
                native_name: "தமிழ்".to_string(),
                voice_count: 10,
            },
            LanguageInfo {
                code: "uk".to_string(),
                name: "Ukrainian".to_string(),
                native_name: "Українська".to_string(),
                voice_count: 10,
            },
            LanguageInfo {
                code: "ru".to_string(),
                name: "Russian".to_string(),
                native_name: "Русский".to_string(),
                voice_count: 15,
            },
        ])
    }
}
