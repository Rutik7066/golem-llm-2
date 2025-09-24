use base64::{engine::general_purpose, Engine};
use golem_tts::{config::get_env, golem::tts::types::TtsError};
use reqwest::Client;
use rsa::Pkcs1v15Sign;
use rsa::{pkcs8::DecodePrivateKey, RsaPrivateKey};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::time::UNIX_EPOCH;

use crate::google::Google;

impl Google {
    // Needs rework: Use this env  GOOGLE_APPLICATION_CREDENTIALS, GOOGLE_CLOUD_PROJECT
    pub fn get_access_token(&self) -> Result<String, TtsError> {
        // Check if we have a valid token
        {
            let token_data = self.token_data.lock().unwrap();
            if let (Some(token), Some(expires_at)) =
                (&token_data.access_token, &token_data.expires_at)
            {
                let now = std::time::SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs() as i64;

                if now < expires_at - 300 {
                    // 5 minutes buffer
                    return Ok(token.clone());
                }
            }
        }

        let client_email = get_env("GOOGLE_CLIENT_EMAIL").map_err(|_| {
            TtsError::InvalidConfiguration(
                "Missing GOOGLE_CLIENT_EMAIL environment variable".to_string(),
            )
        })?;
        let private_key_pem = get_env("GOOGLE_PRIVATE_KEY").map_err(|_| {
            TtsError::InvalidConfiguration(
                "Missing GOOGLE_PRIVATE_KEY environment variable".to_string(),
            )
        })?;

        let private_key = RsaPrivateKey::from_pkcs8_pem(&private_key_pem).map_err(|e| {
            TtsError::InvalidConfiguration(format!("Failed to parse private key: {e}"))
        })?;

        // Need to refresh token
        let jwt = self.create_jwt(private_key, client_email)?;
        let access_token = self.exchange_jwt_for_token(jwt)?;

        // Update token data
        {
            let mut token_data = self.token_data.lock().unwrap();
            let now = std::time::SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64;

            token_data.access_token = Some(access_token.clone());
            token_data.expires_at = Some(now + 3300); // 55 minutes
        }

        Ok(access_token)
    }

    fn create_jwt(
        &self,
        private_key: RsaPrivateKey,
        client_email: String,
    ) -> Result<String, TtsError> {
        let now = std::time::SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        let exp = now + 3600; // 1 hour

        let header = serde_json::json!({
            "alg": "RS256",
            "typ": "JWT"
        });

        let claims = serde_json::json!({
            "iss": client_email,
            "scope": "https://www.googleapis.com/auth/cloud-platform",
            "aud": "https://oauth2.googleapis.com/token",
            "exp": exp,
            "iat": now
        });

        let header_b64 =
            general_purpose::URL_SAFE_NO_PAD.encode(serde_json::to_vec(&header).unwrap());
        let claims_b64 =
            general_purpose::URL_SAFE_NO_PAD.encode(serde_json::to_vec(&claims).unwrap());
        let to_be_signed = format!("{}.{}", header_b64, claims_b64);

        // Sign with RSA
        let mut hasher = Sha256::new();
        hasher.update(to_be_signed.as_bytes());
        let hash = hasher.finalize();

        let padding = Pkcs1v15Sign::new_unprefixed();
        let mut rng = rand::thread_rng();
        let signature = private_key
            .sign_with_rng(&mut rng, padding, &hash)
            .map_err(|e| TtsError::InternalError(format!("Failed to sign JWT: {e}")))?;

        let signature_b64 = general_purpose::URL_SAFE_NO_PAD.encode(&signature);

        Ok(format!("{}.{}", to_be_signed, signature_b64))
    }

    fn exchange_jwt_for_token(&self, jwt: String) -> Result<String, TtsError> {
        let form_data = format!(
            "grant_type=urn:ietf:params:oauth:grant-type:jwt-bearer&assertion={}",
            urlencoding::encode(&jwt)
        );

        let response = Client::new()
            .post("https://oauth2.googleapis.com/token")
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(form_data)
            .send()
            .map_err(|e| TtsError::NetworkError(format!("Token request failed: {e}")))?;

        if !response.status().is_success() {
            return Err(TtsError::Unauthorized(format!(
                "Token exchange failed: {}",
                response.status()
            )));
        }

        let token_response: Value = response
            .json()
            .map_err(|e| TtsError::InternalError(format!("Failed to parse token response: {e}")))?;

        token_response["access_token"]
            .as_str()
            .ok_or_else(|| TtsError::InternalError("Missing access_token in response".to_string()))
            .map(|s| s.to_string())
    }
}
