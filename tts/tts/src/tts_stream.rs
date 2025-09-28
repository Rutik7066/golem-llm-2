use std::{cell::RefCell, collections::VecDeque};

use crate::{
    client::TtsClient,
    golem::tts::{
        streaming::{AudioChunk, GuestSynthesisStream, StreamStatus, SynthesisOptions, TextInput},
        types::{TextType, TtsError},
    },
};

pub struct TtsStream<T: TtsClient> {
    pub client: T,
    pub voice: String,
    pub options: Option<SynthesisOptions>,
    pub buffer: RefCell<String>,
    pub result: RefCell<VecDeque<u8>>,
    pub sequence_counter: RefCell<u32>,
    pub is_active: RefCell<bool>,
}
impl<T: TtsClient> TtsStream<T> {
    pub fn new(client: T, voice: String, options: Option<SynthesisOptions>) -> Self {
        Self {
            client,
            voice,
            options,
            buffer: RefCell::new(String::new()),
            result: RefCell::new(VecDeque::new()),
            sequence_counter: RefCell::new(0),
            is_active: RefCell::new(true),
        }
    }
}

impl<T: TtsClient> GuestSynthesisStream for TtsStream<T> {
    #[doc = " Send text for synthesis (can be called multiple times)"]
    fn send_text(&self, input: TextInput) -> Result<(), TtsError> {
        let mut text_buffer = self.buffer.borrow_mut();
        text_buffer.push_str(&input.content);
        Ok(())
    }

    #[doc = " Signal end of input and flush remaining audio"]
    fn finish(&self) -> Result<(), TtsError> {
        let text_buffer = self.buffer.borrow();
        if !text_buffer.is_empty() {
            let input = TextInput {
                content: text_buffer.clone(),
                text_type: TextType::Plain,
                language: None,
            };

            let response =
                self.client
                    .synthesize(input, self.voice.clone(), self.options.clone())?;
            let mut audio_buffer = self.result.borrow_mut();
            audio_buffer.extend(response.audio_data.iter());
        }
        *self.is_active.borrow_mut() = false;
        Ok(())
    }

    #[doc = " Receive next audio chunk (non-blocking)"]
    fn receive_chunk(&self) -> Result<Option<AudioChunk>, TtsError> {
        if self.result.borrow().is_empty() {
            Ok(None)
        } else {
            let audio_data: Vec<u8> = self.result.borrow_mut().drain(..1024).collect();
            *self.sequence_counter.borrow_mut() = *self.sequence_counter.borrow() + 1;

            Ok(Some(AudioChunk {
                data: audio_data,
                sequence_number: *self.sequence_counter.borrow(),
                timing_info: None,
                is_final: self.result.borrow().is_empty(),
            }))
        }
    }

    #[doc = " Check if more chunks are available"]
    fn has_pending_audio(&self) -> bool {
        !self.result.borrow().is_empty()
    }

    #[doc = " Get current stream status"]
    fn get_status(&self) -> StreamStatus {
        if *self.is_active.borrow() {
            StreamStatus::Ready
        } else {
            StreamStatus::Closed
        }
    }

    #[doc = " Close stream and clean up resources"]
    fn close(&self) {
        *self.is_active.borrow_mut() = false;
    }
}
