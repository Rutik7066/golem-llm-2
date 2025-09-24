use reqwest::Method;
use serde::{Deserialize, Serialize};

use golem_tts::golem::tts::advanced::{PronunciationEntry, TtsError};

use super::ElevenLabsClient;

#[derive(Serialize, Debug, Clone)]
pub struct PronunciationRule {
    pub string_to_replace: String,
    #[serde(rename = "type")]
    pub rule_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alias: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub phoneme: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alphabet: Option<String>,
}

#[derive(Serialize, Debug, Clone)]
pub struct CreateLexiconFromRulesRequest {
    pub rules: Vec<PronunciationRule>,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workspace_access: Option<String>,
}

#[derive(Serialize, Debug, Clone)]
pub struct AddRulesRequest {
    pub rules: Vec<PronunciationRule>,
}

#[derive(Serialize, Debug, Clone)]
pub struct RemoveRulesRequest {
    pub rule_strings: Vec<String>,
}

#[derive(Deserialize, Debug)]
pub struct CreateLexiconResponse {
    pub id: String,
    pub name: String,
    pub created_by: String,
    pub creation_time_unix: i64,
    pub version_id: String,
    pub version_rules_num: u32,
    pub permission_on_resource: String,
    pub description: Option<String>,
}

impl ElevenLabsClient {
    pub fn create_lexicon_from_rules(
        &self,
        name: String,
        entries: Option<Vec<PronunciationEntry>>,
        description: Option<String>,
    ) -> Result<CreateLexiconResponse, TtsError> {
        let rules = self.convert_entries_to_rules(entries);

        let request = CreateLexiconFromRulesRequest {
            rules,
            name: name.clone(),
            description: description
                .or_else(|| Some(format!("Pronunciation dictionary for {}", name))),
            workspace_access: Some("admin".to_string()),
        };

        self.make_request::<CreateLexiconResponse, CreateLexiconFromRulesRequest, ()>(
            Method::POST,
            "/v1/pronunciation-dictionaries/add-from-rules",
            Some(request),
            None,
        )
    }

    pub fn add_lexicon_rule(
        &self,
        dictionary_id: &str,
        word: String,
        pronunciation: String,
    ) -> Result<CreateLexiconResponse, TtsError> {
        let rule = if pronunciation
            .chars()
            .any(|c| "əɪɛɔʊʌɑɒæɜɪʏøœɯɤɐɞɘɵɨɵʉɪʊ".contains(c))
        {
            PronunciationRule {
                string_to_replace: word,
                rule_type: "phoneme".to_string(),
                alias: None,
                phoneme: Some(pronunciation),
                alphabet: Some("ipa".to_string()),
            }
        } else {
            PronunciationRule {
                string_to_replace: word,
                rule_type: "alias".to_string(),
                alias: Some(pronunciation),
                phoneme: None,
                alphabet: None,
            }
        };

        let request = AddRulesRequest { rules: vec![rule] };
        let path = format!("/v1/pronunciation-dictionaries/{}/add-rules", dictionary_id);

        self.make_request::<CreateLexiconResponse, AddRulesRequest, ()>(
            Method::POST,
            &path,
            Some(request),
            None,
        )
    }

    pub fn remove_lexicon_rule(
        &self,
        dictionary_id: &str,
        word: String,
    ) -> Result<CreateLexiconResponse, TtsError> {
        let request = RemoveRulesRequest {
            rule_strings: vec![word],
        };
        let path = format!(
            "/v1/pronunciation-dictionaries/{}/remove-rules",
            dictionary_id
        );

        self.make_request::<CreateLexiconResponse, RemoveRulesRequest, ()>(
            Method::POST,
            &path,
            Some(request),
            None,
        )
    }

    pub fn export_lexicon(
        &self,
        dictionary_id: &str,
        version_id: &str,
    ) -> Result<String, TtsError> {
        let path = format!(
            "/v1/pronunciation-dictionaries/{}/{}/download",
            dictionary_id, version_id
        );

        let url = format!("{}{}", self.base_url, path);
        let request = self
            .client
            .request(Method::GET, &url)
            .header("xi-api-key", &self.api_key);

        match request.send() {
            Ok(response) => {
                if response.status().is_success() {
                    response.text().map_err(|e| {
                        TtsError::InternalError(format!("Failed to read PLS content: {}", e))
                    })
                } else {
                    Err(crate::client::error::from_http_error(response))
                }
            }
            Err(err) => Err(TtsError::NetworkError(format!("Request failed: {}", err))),
        }
    }

    fn convert_entries_to_rules(
        &self,
        entries: Option<Vec<PronunciationEntry>>,
    ) -> Vec<PronunciationRule> {
        match entries {
            Some(entries) => entries
                .into_iter()
                .map(|entry| {
                    // Check if pronunciation looks like IPA (contains special characters)
                    if entry
                        .pronunciation
                        .chars()
                        .any(|c| "əɪɛɔʊʌɑɒæɜɪʏøœɯɤɐɞɘɵɨɵʉɪʊ".contains(c))
                    {
                        PronunciationRule {
                            string_to_replace: entry.word,
                            rule_type: "phoneme".to_string(),
                            alias: None,
                            phoneme: Some(entry.pronunciation),
                            alphabet: Some("ipa".to_string()),
                        }
                    } else {
                        // Treat as alias if no IPA characters detected
                        PronunciationRule {
                            string_to_replace: entry.word,
                            rule_type: "alias".to_string(),
                            alias: Some(entry.pronunciation),
                            phoneme: None,
                            alphabet: None,
                        }
                    }
                })
                .collect(),
            None => vec![],
        }
    }
}
