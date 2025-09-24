pub mod advanced;
pub mod error;
pub mod lexicon;
pub mod synthesis;
pub mod synthesis_stream;
pub mod voices;

use golem_tts::{
    config::{get_env, get_parsed_env},
    golem::tts::types::TtsError,
};
use log::trace;
use reqwest::{Client, Method};
use serde::{de::DeserializeOwned, Serialize};
use std::time::Duration;

use crate::client::error::from_http_error;

#[derive(Clone)]
pub struct ElevenLabsClient {
    api_key: String,
    client: Client,
    base_url: String,
    rate_limit_config: RateLimitConfig,
}

#[derive(Clone)]
pub struct RateLimitConfig {
    pub max_retries: u32,
    pub initial_delay: Duration,
    pub max_delay: Duration,
    pub backoff_multiplier: f64,
}

impl ElevenLabsClient {
    pub fn new() -> Result<Self, TtsError> {
        let api_key = get_env("ELEVENLABS_API_KEY")?;
        let base_url = get_env("TTS_PROVIDER_ENDPOINT")
            .ok()
            .unwrap_or("https://api.elevenlabs.io".to_string());
        let timeout = get_parsed_env("TTS_PROVIDER_TIMEOUT", 30_u64);
        let max_retries = get_parsed_env("TTS_PROVIDER_MAX_RETRIES", 3_u32);

        let rate_limit_config = RateLimitConfig {
            max_retries,
            initial_delay: Duration::from_millis(1000),
            max_delay: Duration::from_secs(30),
            backoff_multiplier: 2.0,
        };

        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(timeout))
            .build()
            .map_err(|err| {
                TtsError::InternalError(format!("Failed to create HTTP client: {err}"))
            })?;

        Ok(Self {
            api_key,
            client,
            base_url,
            rate_limit_config,
        })
    }

    pub fn make_request<T: DeserializeOwned, B: Serialize + Clone, Q: Serialize + Clone>(
        &self,
        method: Method,
        path: &str,
        body: Option<B>,
        query: Option<Q>,
    ) -> Result<T, TtsError> {
        let url = format!("{}{}", self.base_url, path);
        let mut request = self.client.request(method.clone(), &url);

        request = request.header("xi-api-key", &self.api_key);

        if let Some(q) = query {
            request = request.query(&q);
        }

        if let Some(body_content) = body {
            request = request.json(&body_content);
        }

        match request.send() {
            Ok(response) => {
                if response.status().is_success() {
                    response.json::<T>().map_err(|e| {
                        TtsError::InternalError(format!("Failed to parse response: {}", e))
                    })
                } else {
                    Err(from_http_error(response))
                }
            }
            Err(err) => Err(TtsError::NetworkError(format!("Request failed: {}", err))),
        }
    }

    pub fn retry_request<T: DeserializeOwned, B: Serialize + Clone>(
        &self,
        method: Method,
        path: &str,
        body: Option<B>,
    ) -> Result<T, TtsError> {
        let mut delay = self.rate_limit_config.initial_delay;

        for attempt in 0..=self.rate_limit_config.max_retries {
            trace!("Retrying request. Attempt: #{attempt}");
            match self.make_request::<T, B, ()>(method.clone(), path, body.clone(), None) {
                Ok(result) => return Ok(result),
                Err(TtsError::RateLimited(_)) if attempt < self.rate_limit_config.max_retries => {
                    std::thread::sleep(delay);
                    delay = std::cmp::min(
                        Duration::from_millis(
                            (delay.as_millis() as f64 * self.rate_limit_config.backoff_multiplier)
                                as u64,
                        ),
                        self.rate_limit_config.max_delay,
                    );
                }
                Err(TtsError::ServiceUnavailable(_))
                    if attempt < self.rate_limit_config.max_retries =>
                {
                    std::thread::sleep(delay);
                    delay = std::cmp::min(
                        Duration::from_millis(
                            (delay.as_millis() as f64 * self.rate_limit_config.backoff_multiplier)
                                as u64,
                        ),
                        self.rate_limit_config.max_delay,
                    );
                }
                Err(e) => return Err(e),
            }
        }

        // If we get here, all retries failed
        Err(TtsError::RateLimited(429))
    }

    pub fn request_binary<B: Serialize + Clone, Q: Serialize + Clone>(
        &self,
        method: Method,
        path: &str,
        body: Option<B>,
        query: Option<Q>,
    ) -> Result<Vec<u8>, TtsError> {
        let mut delay = self.rate_limit_config.initial_delay;
        let url = format!("{}{}", self.base_url, path);
        for attempt in 0..=self.rate_limit_config.max_retries {
            trace!("Binary request attempt: #{attempt}");

            let mut request = self.client.request(method.clone(), &url);
            request = request
                .header("xi-api-key", &self.api_key)
                .header("Content-Type", "application/json")
                .json(&body)
                .query(&query);

            match request.send() {
                Ok(response) => {
                    if response.status().is_success() {
                        return response
                            .bytes()
                            .map_err(|e| {
                                TtsError::InternalError(format!(
                                    "Failed to read binary response: {}",
                                    e
                                ))
                            })
                            .map(|bytes| bytes.to_vec());
                    } else {
                        let error = from_http_error(response);
                        match error {
                            TtsError::RateLimited(_) | TtsError::ServiceUnavailable(_)
                                if attempt < self.rate_limit_config.max_retries =>
                            {
                                std::thread::sleep(delay);
                                delay = std::cmp::min(
                                    Duration::from_millis(
                                        (delay.as_millis() as f64
                                            * self.rate_limit_config.backoff_multiplier)
                                            as u64,
                                    ),
                                    self.rate_limit_config.max_delay,
                                );
                                continue;
                            }
                            _ => return Err(error),
                        }
                    }
                }
                Err(err) => {
                    if attempt < self.rate_limit_config.max_retries {
                        std::thread::sleep(delay);
                        delay = std::cmp::min(
                            Duration::from_millis(
                                (delay.as_millis() as f64
                                    * self.rate_limit_config.backoff_multiplier)
                                    as u64,
                            ),
                            self.rate_limit_config.max_delay,
                        );
                        continue;
                    }
                    return Err(TtsError::NetworkError(format!("Request failed: {}", err)));
                }
            }
        }

        Err(TtsError::RateLimited(429))
    }
}
