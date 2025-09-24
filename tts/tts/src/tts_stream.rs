use std::{cell::RefCell, collections::VecDeque};

use crate::{client::TtsClient, golem::tts::{types::{TextType, TtsError}, streaming::{AudioChunk, GuestSynthesisStream, StreamStatus, SynthesisOptions, TextInput}}};


pub struct TtsStream<T: TtsClient> {
    pub client: T,
    pub voice: String,
    pub options: Option<SynthesisOptions>,
    pub buffer: RefCell<String>,
    pub result: RefCell<VecDeque<u8>>,
    pub is_active: RefCell<bool>,
    pub sequence_counter: RefCell<u32>,
}
impl<T: TtsClient> TtsStream<T> {
    pub fn new(client:T, voice: String, options: Option<SynthesisOptions>) -> Self {
        Self {
            client,
            voice,
            options,
            buffer: RefCell::new(String::new()),
            result: RefCell::new(VecDeque::new()),
            is_active: RefCell::new(true),
            sequence_counter: RefCell::new(0),
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
            
            let response = self.client.synthesize(input, self.voice.clone(), self.options.clone())?;
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
            let mut buffer = self.result.borrow_mut();
            let audio_data: Vec<u8> = buffer.drain(..).collect();
            let is_final = !*self.is_active.borrow();
            let mut counter = self.sequence_counter.borrow_mut();
            let seq_num = *counter;
            *counter += 1;
            Ok(Some(AudioChunk {
                data: audio_data,
                sequence_number: seq_num,
                timing_info: None,
                is_final,
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