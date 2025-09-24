use std::{cell::RefCell, collections::VecDeque};

use golem_tts::golem::tts::{
    streaming::{SynthesisOptions, TtsError},
    types::{TextInput, TextType},
};

use crate::client::{voices::ElVoice, ElevenLabsClient};

#[derive(Clone)]
pub struct ElevenLabsSynthesisStream {
    client: ElevenLabsClient,
    voice: ElVoice,
    options: Option<SynthesisOptions>,
    pub text_buffer: RefCell<String>,
    pub audio_buffer: RefCell<VecDeque<u8>>,
    pub is_active: RefCell<bool>,
    pub sequence_counter: RefCell<u32>,
}

#[derive(Clone)]
pub struct ElevenLabsVoiceConversionStream {
    // ElevenLabs doesn't support real-time voice conversion
    // This is a placeholder that will return UnsupportedOperation
}

impl ElevenLabsSynthesisStream {
    pub fn new(
        client: ElevenLabsClient,
        voice: ElVoice,
        options: Option<SynthesisOptions>,
    ) -> Self {
        Self {
            client,
            voice,
            options,
            text_buffer: RefCell::new(String::new()),
            audio_buffer: RefCell::new(VecDeque::new()),
            is_active: RefCell::new(true),
            sequence_counter: RefCell::new(0),
        }
    }

    pub fn add_text(&self, text: &str) {
        let mut text_buffer = self.text_buffer.borrow_mut();
        text_buffer.push_str(text);
    }

    pub fn synthesize_buffered_text(&self) -> Result<(), TtsError> {
        let text_buffer = self.text_buffer.borrow();
        if text_buffer.is_empty() {
            return Ok(());
        }

        let input = TextInput {
            content: text_buffer.clone(),
            text_type: TextType::Plain,
            language: None,
        };

        let response =
            self.client
                .synthesize(input, &self.voice, self.options.clone(), None, None)?;

        let mut audio_buffer = self.audio_buffer.borrow_mut();
        audio_buffer.extend(response.audio_data.iter());

        Ok(())
    }

    pub fn get_buffered_audio(&self) -> Vec<u8> {
        let mut buffer = self.audio_buffer.borrow_mut();
        buffer.drain(..).collect()
    }

    pub fn is_active(&self) -> bool {
        *self.is_active.borrow()
    }

    pub fn stop(&self) {
        *self.is_active.borrow_mut() = false;
    }
}
