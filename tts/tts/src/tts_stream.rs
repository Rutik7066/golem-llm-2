use std::{cell::RefCell, collections::VecDeque};

use crate::{
    client::TtsClient,
    durability::ExtendedGuest,
    golem::tts::{
        streaming::{AudioChunk, GuestSynthesisStream, StreamStatus, SynthesisOptions, TextInput},
        types::{TextType, TtsError},
    },
};

pub struct TtsStream<T: TtsClient + ExtendedGuest> {
    pub client: T,
    pub voice: String,
    pub options: Option<SynthesisOptions>,
    pub input_buffer: RefCell<String>,
    pub result: RefCell<VecDeque<u8>>,
    pub sequence_counter: RefCell<u32>,
    pub is_input_finished: RefCell<bool>,
    pub first_text: RefCell<Option<TextInput>>,
    pub status: RefCell<Option<StreamStatus>>,
    // guard so we synthesize only once per finish()
    pub synth_started: RefCell<bool>,
    pub is_final : RefCell<bool>
}

impl<T: TtsClient + ExtendedGuest> TtsStream<T> {
    pub fn new(client: T, voice: String, options: Option<SynthesisOptions>) -> Self {
        Self {
            client,
            voice,
            options,
            input_buffer: RefCell::new(String::new()),
            result: RefCell::new(VecDeque::new()),
            sequence_counter: RefCell::new(0),
            is_input_finished: RefCell::new(false),
            first_text: RefCell::new(None),
            status: RefCell::new(None),
            synth_started: RefCell::new(false),
            is_final:RefCell::new(false),            
        }
    }
}

impl<T: TtsClient + ExtendedGuest> GuestSynthesisStream for TtsStream<T> {
    fn send_text(&self, input: TextInput) -> Result<(), TtsError> {
        // disallow new text once status was ever set
        if self.status.borrow().is_some() {
            return Err(TtsError::RequestError(
                "Stream is already in use".to_string(),
            ));
        }
        if self.first_text.borrow().is_none() {
            *self.first_text.borrow_mut() = Some(input.clone());
        }
        self.input_buffer
            .borrow_mut()
            .push_str(input.content.as_str());
        Ok(())
    }

    fn finish(&self) -> Result<(), TtsError> {
        *self.status.borrow_mut() = Some(StreamStatus::Ready); // Input is complete and ready to consume result
        *self.is_input_finished.borrow_mut() = true;
        Ok(())
    }

    fn receive_chunk(&self) -> Result<Option<AudioChunk>, TtsError> {
        // enforce finishing semantics
        if !*self.is_input_finished.borrow() {
            *self.status.borrow_mut() = Some(StreamStatus::Error);
            return Err(TtsError::RequestError(
                "Call finish() before calling receive_chunk()".to_string(),
            ));
        }

        const CHUNK_SIZE: usize = 1024;

        // 1) If we have queued bytes, safely drain up to CHUNK_SIZE
        if !self.result.borrow().is_empty() {
            // mutably borrow once
            let mut buf = self.result.borrow_mut();

            let take = std::cmp::min(CHUNK_SIZE, buf.len());
            
            if take  < CHUNK_SIZE{
                *self.is_final.borrow_mut() = true;
            }   

            let audio_data: Vec<u8> = buf.drain(..take).collect();

            // Using sequence_counter to determine range to drain for durability compability 
            // this way after rebuild new state. first receive_chunk call we fech synthesis data. 
            // and use sequence_counter to determine offset to create new result data as is was before crash.
            *self.sequence_counter.borrow_mut() += 1;

            let is_final = audio_data.len() < CHUNK_SIZE || buf.is_empty();
            if is_final {
                *self.status.borrow_mut() = Some(StreamStatus::Finished);
            }

            return Ok(Some(AudioChunk {
                data: audio_data,
                sequence_number: *self.sequence_counter.borrow(),
                timing_info: None,
                is_final,
            }));
        }

        // 2) No queued data: kick off synthesis once (guarded by synth_started)
        if !*self.synth_started.borrow() {
            // if there was never any text provided, return a clear error
            let first = match self.first_text.borrow().as_ref() {
                Some(v) => v.clone(),
                None => {
                    *self.status.borrow_mut() = Some(StreamStatus::Error);
                    return Err(TtsError::RequestError("No input text provided".to_string()));
                }
            };

            // mark started to prevent repeated synth attempts
            *self.synth_started.borrow_mut() = true;
            *self.status.borrow_mut() = Some(StreamStatus::Processing);

            // build TextInput from buffer and metadata
            let input_text = TextInput {
                content: self.input_buffer.borrow().clone(),
                text_type: first.text_type, // safe because we matched Some above
                language: first.language.clone(),
            };

            // IMPORTANT: this is a blocking call in your current client API.
            // Consider making it async or running it off the main event loop.
            match self
                .client
                .synthesize(input_text, self.voice.clone(), self.options.clone())
            {
                Ok(response) => {

                    *self.result.borrow_mut() = response.audio_data.into();
                    

                    // optionally clear input buffer to free memory and avoid accidental reuse
                    self.input_buffer.borrow_mut().clear();
                    // audio is queued, status Ready -> consumer will drain on next calls
                    *self.status.borrow_mut() = Some(StreamStatus::Ready);
                    return Ok(None);
                }
                Err(err) => {
                    *self.status.borrow_mut() = Some(StreamStatus::Error);
                    return Err(err);
                }
            }
        }

        // 3) synth already started but result not yet produced/available
        Ok(None)
    }

    fn has_pending_audio(&self) -> bool {
        !*self.is_final.borrow()
    }

    fn get_status(&self) -> StreamStatus {
        self.status.borrow().unwrap_or(StreamStatus::Ready)
    }

    fn close(&self) {
        *self.status.borrow_mut() = Some(StreamStatus::Closed);
        self.result.borrow_mut().clear();
        self.input_buffer.borrow_mut().clear();
        self.first_text.borrow_mut().take();
        *self.synth_started.borrow_mut() = false;
    }
}
