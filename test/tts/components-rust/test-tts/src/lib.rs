#[allow(static_mut_refs)]
mod bindings;

use std::fs;

use crate::bindings::exports::test;
use crate::bindings::exports::test::tts_exports::test_tts_api::*;
use crate::bindings::golem::tts::advanced::{convert_voice, design_voice, generate_sound_effect, AgeCategory, OperationStatus, VoiceDesignParams};
use crate::bindings::golem::tts::streaming::StreamStatus;
use crate::bindings::golem::tts::synthesis::{AudioEffects, SynthesisResult, VoiceSettings};
use crate::bindings::golem::tts::{
    advanced::{
        create_lexicon, create_voice_clone, synthesize_long_form, AudioSample, PronunciationEntry,
    },
    streaming::{create_stream, create_voice_conversion_stream},
    synthesis::{get_timing_marks, synthesize, synthesize_batch, validate_input, SynthesisOptions},
    types::{AudioConfig, TextInput, TextType, VoiceGender, TtsError},
    voices::{get_voice, list_languages, list_voices, search_voices, Voice, VoiceResults, VoiceFilter},
};
use crate::bindings::test::helper_client::test_helper_client::TestHelperApi;
use golem_rust::atomically;
use log::trace;

use std::thread;
use std::time::Duration;
struct Component;

#[cfg(feature = "deepgram")]
pub const VOICE_UUID: &str = "87f1a83a-8064-465c-ae3d-4e5ab800d4ed"; // aura-2-alvaro-es
#[cfg(feature = "deepgram")]
pub const TARGET_VOICE: &str = "aura-2-amalthea-en";  

#[cfg(feature = "elevenlabs")]
pub const VOICE_UUID: &str = "Ellen";
#[cfg(feature = "elevenlabs")]
pub const TARGET_VOICE: &str = "454";

#[cfg(feature = "polly")]
pub const VOICE_UUID: &str = "Danielle";
#[cfg(feature = "polly")]
pub const TARGET_VOICE: &str = "Joanna";

#[cfg(feature = "google")]
pub const VOICE_UUID: &str = "en-US-Wavenet-A";
#[cfg(feature = "google")]
pub const TARGET_VOICE: &str = "en-US-Wavenet-B"; 


const TEXT:&str = "In a quiet coastal village, mornings begin with the scent of salt in the air and the rhythm of waves meeting the shore. 
Fishermen set out at dawn, their small boats cutting across the glassy water, while children race barefoot along the beach, chasing the tide. 
The old lighthouse, weathered but steadfast, watches over them all, its lantern dim in the morning light, waiting for night to reclaim its glow.";

const SSML: &str = r#"
    <speak>
    <p>
        In a quiet coastal village, mornings begin with the scent of salt in the air
        and the rhythm of waves meeting the shore. <break time="400ms"/>
        Fishermen set out at dawn, their boats tracing silver lines across calm water,
        while children race barefoot along the beach, chasing the tide.
    </p>
    <p>
        The old lighthouse—weathered yet steadfast—watches over them,
        its lantern dim in the early light, waiting for night to reclaim its glow.
        <s>
        It is a place where routines are <emphasis level="moderate">gentle</emphasis>,
        and time seems to breathe.
        </s>
    </p>
    </speak>
"#;

impl Guest for Component {
    // 💥 Test demonstrate Durable listing all voices with pagination
    fn test() -> String {
        trace!("Getting all voices.");
        let filter = VoiceFilter {
            gender: Some(VoiceGender::Female),
            language: Some("en-US".to_string()),
            provider: None,
            quality: Some("standard".to_string()),
            search_query: None,
            supports_ssml: None, // This should be null some provider does not support ssml
        };

        let mut voices = Vec::new();
        let result = list_voices(Some(&filter));
        let mut round = 1;
        let worker_name =
            std::env::var("GOLEM_WORKER_NAME").unwrap_or_else(|_| "test-worker".to_string());
        match result {
            Ok(voice_results) => {
                trace!("Recived {:?} voices", voice_results.get_total_count());
                while voice_results.has_more() {
                    trace!("[{round}]: Fetching more voices.");

                    if round == 2 {
                        trace!("Crashing when getting next batch of voices.");
                        mimic_crash(&worker_name);
                    }

                    voices.append(&mut voice_results.get_next().unwrap());

                    round += 1;
                }
                trace!("Total voices: {}", voices.len());
            }
            Err(err) => {
                return format!("❌ ERROR : {:?}", err);
            }
        }
        format!("✅ Total: {}, Data: {:?}", voices.len(), voices)
    }

    // Test Getting a specific voice
    fn test2() -> String {
        trace!("Getting a specific voice.");
        match get_voice(VOICE_UUID) {
            Ok(voice) => {
                trace!("Recived voice: {:?}", voice);
                format!("✅ Voice ID: {VOICE_UUID} Voice: {:?}", voice)
            }
            Err(err) => {
                return format!("❌ ERROR : {:?}", err);
            }
        }
    }

    // Test Searching voices
    fn test3() -> String {
        let query = "";
        let filter = VoiceFilter {
            gender: Some(VoiceGender::Female),
            language: Some("en-US".to_string()),
            provider: None,
            quality: Some("standard".to_string()),
            search_query: None,
            supports_ssml: None, // This should be null some provider does not support ssml
        };
        match search_voices(query, Some(&filter)) {
            Ok(voices) => {
                trace!("Recived voices: {:?}", voices);
                format!("✅ Voices: {:?}", voices)
            }
            Err(err) => {
                return format!("❌ ERROR : {:?}", err);
            }
        }
    }

    // Test Synthesizing text
    fn test4() -> String {
        let voice = match get_voice(VOICE_UUID) {
            Ok(voices) => voices,
            Err(err) => {
                return format!("❌ ERROR : {:?}", err);
            }
        };
        let mut test_result = String::new();
        test_result.push_str("Test speech synthesis summary:\n");

        trace!("Sending text without options.");
        let test_name = "1. Test without options";

        let text_input = TextInput {
            content: TEXT.to_string(),
            text_type: TextType::Plain,
            language: Some("en-US".to_string()),
        };

        match synthesize(&text_input, &voice, None) {
            Ok(result) => {
                trace!("Recived result: {:?}", result.metadata);
                let dir = "/test-audio-files";
                let name = "test4-without-options.mp3";
                save_audio(&result.audio_data, dir, name);
                push_result(&mut test_result, result, test_name, format!("{dir}/{name}"));
            }
            Err(err) => {
                test_result.push_str(&format!("{test_name} ❌\n"));
                test_result.push_str(&format!("ERROR : {:?}\n", err));
            }
        };

        trace!("Sending text with Audio Config.");
        let test2_name = "2. Test with Audio Config";

        let audio_config_options = SynthesisOptions {
            audio_config: Some(AudioConfig {
                format: "mp3".to_string(),
                sample_rate: Some(22050),
                bit_rate: None,
                channels: Some(1),
            }),
            voice_settings: None,
            audio_effects: None,
            enable_timing: None,
            enable_word_timing: None,
            seed: None,
            model_id: None,
            context: None,
        };

        let text_input = TextInput {
            content: TEXT.to_string(),
            text_type: TextType::Plain,
            language: Some("en-US".to_string()),
        };

        match synthesize(&text_input, &voice, Some(&audio_config_options)) {
            Ok(result) => {
                trace!("Recived result: {:?}", result.metadata);
                let dir = "/test-audio-files";
                let name = "test4-audio-config.mp3";
                save_audio(&result.audio_data, dir, name);
                push_result(
                    &mut test_result,
                    result,
                    test2_name,
                    format!("{dir}/{name}"),
                );
            }
            Err(err) => {
                test_result.push_str(&format!("{test2_name} ❌\n"));
                test_result.push_str(&format!("ERROR : {:?}\n", err));
            }
        };

        trace!("Sending text with Voice Settings.");
        let test3_name = "3. Test with Voice Settings";

        let voice_settings_options = SynthesisOptions {
            audio_config: None,
            voice_settings: Some(VoiceSettings {
                speed: Some(1.2),
                pitch: Some(2.0),
                volume: Some(0.0),
                stability: Some(0.8),
                similarity: Some(0.9),
                style: Some(0.5),
            }),
            audio_effects: None,
            enable_timing: None,
            enable_word_timing: None,
            seed: None,
            model_id: None,
            context: None,
        };

        let text_input = TextInput {
            content: TEXT.to_string(),
            text_type: TextType::Plain,
            language: Some("en-US".to_string()),
        };

        match synthesize(&text_input, &voice, Some(&voice_settings_options)) {
            Ok(result) => {
                trace!("Recived result: {:?}", result.metadata);
                let dir = "/test-audio-files";
                let name = "test4-voice-settings.mp3";
                save_audio(&result.audio_data, dir, name);
                push_result(
                    &mut test_result,
                    result,
                    test3_name,
                    format!("{dir}/{name}"),
                );
            }
            Err(err) => {
                test_result.push_str(&format!("{test3_name} ❌\n"));
                test_result.push_str(&format!("ERROR : {:?}\n", err));
            }
        };

        test_result
    }

    // Test Synthesizing text with SSML
    fn test5() -> String {
        let voice = match get_voice(VOICE_UUID) {
            Ok(voices) => voices,
            Err(err) => {
                return format!("❌ ERROR : {:?}", err);
            }
        };
        let mut test_result = String::new();
        test_result.push_str("Test speech synthesis using SSML summary:\n");

        trace!("Sending SSML without options.");
        let test_name = "1. Test speech synthesis using SSML ";

        let text_input = TextInput {
            content: SSML.to_string(),
            text_type: TextType::Ssml,
            language: Some("en-US".to_string()),
        };

        match synthesize(&text_input, &voice, None) {
            Ok(result) => {
                trace!("Recived result: {:?}", result.metadata);
                let dir = "/test-audio-files";
                let name = "test5-ssml.mp3";
                save_audio(&result.audio_data, dir, name);
                push_result(&mut test_result, result, test_name, format!("{dir}/{name}"));
            }
            Err(err) => {
                test_result.push_str(&format!("{test_name} ❌\n"));
                test_result.push_str(&format!("ERROR : {:?}\n", err));
            }
        };

        test_result
    }

    // Test Batch synthesis for multiple inputs
    fn test6() -> String {
        let voice = match get_voice(VOICE_UUID) {
            Ok(voices) => voices,
            Err(err) => {
                return format!("❌ ERROR : {:?}", err);
            }
        };
        let mut test_result = String::new();
        test_result.push_str("Test batch speech synthesis summary:\n");

        trace!("Sending batch text inputs.");
        let test_name = "1. Test batch speech synthesis";

        let batch_inputs = vec![
            TextInput {
                content: "I am first sentence of the batch inputs.".to_string(),
                text_type: TextType::Plain,
                language: Some("en-US".to_string()),
            },
            TextInput {
                content: "I am seconds sentence of the batch inputs.".to_string(),
                text_type: TextType::Plain,
                language: Some("en-US".to_string()),
            },
        ];

        match synthesize_batch(&batch_inputs, &voice, None) {
            Ok(batch) => {
                test_result.push_str(&format!("{test_name} ✅\n"));
                let mut index = 1;
                for result in batch {
                    trace!("#{index} Recived result: {:?}\n", result.metadata);
                    let dir = "/test-audio-files";
                    let name = format!("test6-batch-{index}.mp3");
                    save_audio(&result.audio_data, dir, &name);
                    test_result.push_str(&format!("Batch Item #{index}:\n"));
                    push_result(&mut test_result, result, test_name, format!("{dir}/{name}"));
                    index += 1;
                }
            }
            Err(err) => {
                test_result.push_str(&format!("{test_name} ❌\n"));
                test_result.push_str(&format!("ERROR : {:?}\n", err));
            }
        };

        test_result
    }

    // Test Validate text before synthesis & Get timing information without audio synthesis
    fn test7() -> String {
        let voice = match get_voice(VOICE_UUID) {
            Ok(voices) => voices,
            Err(err) => {
                return format!("❌ ERROR : {:?}", err);
            }
        };
        let ssml_input = TextInput {
            content: SSML.to_string(),
            text_type: TextType::Ssml,
            language: Some("en-US".to_string()),
        };

        let mut test_result = String::new();
        test_result.push_str("Test input validation & timing marks summary:\n");

        trace!("Testing input validation...");
        let test_name = "1. Test input validation";
        match validate_input(&ssml_input.clone(), &voice) {
            Ok(validation) => {
                trace!("Validation result: {:?}\n", validation);
                test_result.push_str(&format!("{test_name} ✅\n"));
                test_result.push_str(&format!("Is Valid: {:?}\n", validation.is_valid));
                test_result.push_str(&format!(
                    "Character Count: {:?}\n",
                    validation.character_count
                ));
                test_result.push_str(&format!(
                    "Estimated Duration: {:?}\n",
                    validation.estimated_duration
                ));
                test_result.push_str(&format!("Errors: {:?}\n", validation.errors));
                test_result.push_str(&format!("Warnings: {:?}\n", validation.warnings));
            }
            Err(err) => {
                test_result.push_str(&format!("{test_name} ❌\n"));
                test_result.push_str(&format!("ERROR : {:?}\n", err))
            }
        }

        trace!("Testing timing marks.");
        let test_name = "2. Test timing marks";
        match get_timing_marks(&ssml_input, &voice) {
            Ok(timing_marks) => {
                test_result.push_str(&format!("{test_name} ✅\n"));
                let index = 1;
                for mark in timing_marks {
                    trace!("Timing mark #{} : {:?}\n", index, mark);
                    test_result.push_str(&format!("Timing mark #{index}"));
                    test_result.push_str(&format!("{:?}\n", mark));
                }
            }
            Err(err) => {
                test_result.push_str(&format!("{test_name} ❌\n"));
                test_result.push_str(&format!("ERROR : {:?}\n", err))
            }
        };

        test_result
    }

    // 💥💥 Test durable synthesis streaming
    fn test8() -> String {
        let voice = match get_voice(VOICE_UUID) {
            Ok(voices) => voices,
            Err(err) => {
                return format!("❌ ERROR : {:?}", err);
            }
        };
        trace!("Testing durable synthesis streaming...");

        let mut test_result = String::new();
        test_result.push_str("Test synthesis streaming summary:\n");

        trace!("Creating stream for speech synthesis.");
        let test_name = "Test synthesis streaming";
        let stream = match create_stream(&voice, None) {
            Ok(stream) => {
                trace!("Stream created successfully. ✅\n",);
                test_result.push_str("1. Test create stream ✅\n");
                stream
            }
            Err(err) => {
                trace!("Failed to create stream. ❌\n",);
                test_result.push_str("1. Test create stream ❌\n");
                test_result.push_str(&format!("ERROR : {:?}\n", err));
                test_result.push_str(&format!("{test_name} ❌\n"));
                return test_result;
            }
        };

         let worker_name =
            std::env::var("GOLEM_WORKER_NAME").unwrap_or_else(|_| "test-worker".to_string());

        trace!("Sending text as chunks for synthesis.");
        // Sending text as chunks
        for text in "In a quiet coastal village, mornings begin with the scent of salt in the air and the rhythm of waves meeting the shore.".split_whitespace() {
            
            if text.contains("mornings"){
                trace!("Crashing when sending text!");
                mimic_crash(&worker_name);
            }
            
            let input = TextInput {
                content: text.to_string(),
                text_type: TextType::Plain,
                language: Some("en-US".to_string()),
            };

            match stream.send_text(&input) {
                Ok(_) => {
                    trace!("Text sent successfully. ✅\n",);
                }
                Err(err) => {
                    trace!("Failed to send text. ❌\n",);
                    trace!("Sending failed at {text}\n",);
                    test_result.push_str("2. Test send text ❌\n");
                    test_result.push_str(&format!("ERROR : {:?}\n", err));
                    test_result.push_str(&format!("{test_name} ❌\n"));
                    return test_result;
                }
            };
        }
        test_result.push_str("2. Test send text ✅\n");

        match stream.finish() {
            Ok(_) => {
                trace!("Text sending finished successfully. ✅\n",);
                test_result.push_str("3. Test finish stream ✅\n");
            }
            Err(err) => {
                trace!("Failed to finish stream. ❌\n",);
                test_result.push_str("3. Test finish stream ❌\n");
                test_result.push_str(&format!("ERROR : {:?}\n", err));
                test_result.push_str(&format!("{test_name} ❌\n"));
                return test_result;
            }
        }

        trace!("Reciving audio chunks.");
        let mut audio_data = Vec::new();
        let mut chunk_count = 0;
        let max_attempts = 30;
        let mut attempts = 0;

        while attempts < max_attempts {
            if !stream.has_pending_audio() && matches!(stream.get_status(), StreamStatus::Finished)
            {
                break;
            }

            if chunk_count == 7{
                trace!("Crashing when receiving audio chunks #{chunk_count}.");
                mimic_crash(&worker_name);
            }

            match stream.receive_chunk() {
                Ok(Some(chunk)) => {
                    trace!("Recieved chunk #{chunk_count}\n");
                    audio_data.extend_from_slice(&chunk.data);
                    chunk_count += 1;
                    if chunk.is_final {
                        break;
                    }
                }
                Ok(None) => {
                    trace!("No chunk available. Retrying...\n");
                    thread::sleep(Duration::from_millis(100));
                }
                Err(e) => {
                    trace!("Failed to receive chunk #{chunk_count}. ❌\n");
                    test_result.push_str(&format!("4. Test receive chunk ❌\n"));
                    test_result.push_str(&format!("ERROR : {:?}\n", e));
                    test_result.push_str(&format!("{test_name} ❌\n"));
                    break;
                }
            }
            
            // Just as edge case 
            if attempts == 10 {
                trace!("Crashing when receiving audio chunks after 10 attempts.");
                mimic_crash(&worker_name);
            }

            attempts += 1;
        }

        test_result.push_str("4. Test receive chunk ✅\n");


        trace!("Saving audio data.");
        if !audio_data.is_empty() {
            let dir = "/test-audio-files";
            let name = "test8-synthesis-streaming.mp3";
            save_audio(&audio_data, dir, name);
            trace!("Audio saved at {dir}/{name} 💾\n");
            test_result.push_str(&format!("Audio saved at {dir}/{name} 💾\n"));
            
        }

        trace!("Closing stream...\n");
        stream.close();
        trace!("Stream closed successfully. ✅\n",);
        test_result.push_str("5. Test close stream ✅\n");
        test_result.push_str(&format!("{test_name} ✅\n"));

        test_result

    }

    // 💥💥💥 Test durable voice conversion
    fn test9() -> String {
         let voice = match get_voice(VOICE_UUID) {
            Ok(voices) => voices,
            Err(err) => {
                return format!("❌ ERROR : {:?}", err);
            }
        };

        let mut test_result = String::new();
        test_result.push_str("Test synthesis streaming summary:\n");

        trace!("Creating stream for speech synthesis.");
        let test_name = "Test voice conversion streaming";
        let stream = match create_voice_conversion_stream(&voice, None) {
            Ok(stream) => {
                trace!("Stream created successfully. ✅\n",);
                test_result.push_str("1. Test create stream ✅\n");
                stream
            }
            Err(err) => {
                trace!("Failed to create stream. ❌\n",);
                test_result.push_str("1. Test create stream ❌\n");
                test_result.push_str(&format!("ERROR : {:?}\n", err));
                test_result.push_str(&format!("{test_name} ❌\n"));
                return test_result;
            }
        };

         let worker_name =
            std::env::var("GOLEM_WORKER_NAME").unwrap_or_else(|_| "test-worker".to_string());

        let voice_data = fs::read("/test-audio-files/test4-audio-config.mp3").unwrap();
        trace!("Sending voice data.");
        match stream.send_audio(&voice_data) {
            Ok(_) => {
                trace!("Audio sent successfully. ✅\n",);
                test_result.push_str("2. Test send audio data ✅\n");
            }
            Err(err) => {
                trace!("Failed to send audio. ❌\n",);
                test_result.push_str("2. Test send audio data ❌\n");
                test_result.push_str(&format!("ERROR : {:?}\n", err));
                test_result.push_str(&format!("{test_name} ❌\n"));
                return test_result;
            }
        }
        
            trace!("Crashing after sending audio data.");
            mimic_crash(&worker_name);

        trace!("Finishing stream.");
        match stream.finish() {
            Ok(_) => {
                trace!("Stream finished successfully. ✅\n",);
                test_result.push_str("3. Test finish stream ✅\n");
            }
            Err(err) => {
                trace!("Failed to finish stream. ❌\n",);
                test_result.push_str("3. Test finish stream ❌\n");
                test_result.push_str(&format!("ERROR : {:?}\n", err));
                test_result.push_str(&format!("{test_name} ❌\n"));
                return test_result;
            }
        }

        trace!("Crashing after finishing stream before receiving converted audio chunks.");
        mimic_crash(&worker_name);

        trace!("Reciving audio chunks.");
        let mut converted_audio_data = Vec::new();
        let max_attempts = 30;
        let mut attempts = 0;
        while attempts < max_attempts {
            
            if attempts == 7 {
                trace!("Crashing when receiving converted audio chunks after 6th attempts.");
                mimic_crash(&worker_name);
            }

            match stream.receive_converted() {
                Ok(Some(chunk)) => {
                    trace!("Recieved converted voice\n");
                    converted_audio_data.extend_from_slice(&chunk.data);
                }
                Ok(None) => {
                    trace!("No chunk available. Retrying...\n");
                    thread::sleep(Duration::from_millis(100));
                }
                Err(e) => {
                    trace!("Failed to receive converted voice. ❌\n");
                    test_result.push_str(&format!("4. Test receive converted voice ❌\n"));
                    test_result.push_str(&format!("ERROR : {:?}\n", e));
                    test_result.push_str(&format!("{test_name} ❌\n"));
                }
            }
            attempts += 1;   
        }

        test_result.push_str("4. Test receive converted voice ✅\n");

        trace!("Saving converted audio data.");
        if !converted_audio_data.is_empty() {
            let dir = "/test-audio-files";
            let name = "test9-voice-conversion.mp3";
            save_audio(&converted_audio_data, dir, name);
            trace!("Audio saved at {dir}/{name} 💾\n");
            test_result.push_str(&format!("Audio saved at {dir}/{name} 💾\n"));
            
        }

        trace!("Closing stream...\n");
        stream.close();
        trace!("Stream closed successfully. ✅\n",);
        test_result.push_str("5. Test close stream ✅\n");
        test_result.push_str(&format!("{test_name} ✅\n"));

        test_result
    }

    // Test advanced voice operation 
    // 1. Create voice clone
    // 2. Design voice
    // 3. Voice to voice 
    // 4. Generate sound effects
    fn test10() -> String {
        let voice = match get_voice(VOICE_UUID) {
            Ok(voices) => voices,
            Err(err) => {
                return format!("❌ ERROR : {:?}", err);
            }
        };

        let mut test_result = String::new();
        test_result.push_str("Test advanced voice operations summary:\n");

        let text_input = TextInput {
            content: TEXT.to_string(),
            text_type: TextType::Plain,
            language: Some("en-US".to_string()),
        };

        let sample_audio_data = match synthesize(&text_input, &voice, None) {
            Ok(result) => result.audio_data,
            Err(err) => {
                return format!("❌ ERROR generating sample audio: {:?}", err);
            }
        };

        // Test 1: Create voice clone
        trace!("Testing voice cloning...");
        let test_name = "1. Test voice clone";
        
        let audio_samples = vec![
            AudioSample {
                data: sample_audio_data.clone(),
                transcript: Some(TEXT.to_string()),
                quality_rating: Some(8),
            }
        ];

        match create_voice_clone("test-voice-clone", &audio_samples, Some("Test clone description")) {
            Ok(cloned_voice) => {
                let id = cloned_voice.get_id();
                trace!("Voice cloned successfully: {id}") ;
                test_result.push_str(&format!("{test_name} ✅\n"));
                test_result.push_str(&format!("Cloned Voice ID: {id}\n"));
                test_result.push_str(&format!("Cloned Voice Name: {}\n", cloned_voice.get_name()));
            }
            Err(err) => {
                trace!("Failed to clone voice.");
                test_result.push_str(&format!("{test_name} ❌\n"));
                test_result.push_str(&format!("ERROR : {:?}\n", err));
            }
        }

        // Test 2: Design voice
        trace!("Testing voice design...");
        let test2_name = "2. Test voice design";
        let design_params = VoiceDesignParams {
            gender: VoiceGender::Female,
            age_category: AgeCategory::YoungAdult,
            accent: "american".to_string(),
            personality_traits: vec!["friendly".to_string(), "energetic".to_string()],
            reference_voice: Some(VOICE_UUID.to_string()),
        };

        match design_voice("test-designed-voice", &design_params) {
            Ok(designed_voice) => {
                trace!("Voice designed successfully: {:?}", designed_voice.get_id());
                test_result.push_str(&format!("{test2_name} ✅\n"));
                test_result.push_str(&format!("Designed Voice ID: {}\n", designed_voice.get_id()));
                test_result.push_str(&format!("Designed Voice Name: {}\n", designed_voice.get_name()));
            }
            Err(err) => {
                trace!("Failed to design voice.");
                test_result.push_str(&format!("{test2_name} ❌\n"));
                test_result.push_str(&format!("ERROR : {:?}\n", err));
            }
        }

        // Test 3: Voice-to-voice conversion
        trace!("Testing voice-to-voice conversion...");
        let test3_name = "3. Test voice-to-voice conversion";

        let target_voice = match get_voice(TARGET_VOICE) {
            Ok(voices) => voices,
            Err(err) => {
                return format!("❌ ERROR : {:?}", err);
            }
        };

        match convert_voice(&sample_audio_data, &target_voice, Some(true)) {
            Ok(converted_audio) => {
                trace!("Voice conversion successful, audio size: {}", converted_audio.len());
                test_result.push_str(&format!("{test3_name} ✅\n"));
                test_result.push_str(&format!("Converted audio size: {} bytes\n", converted_audio.len()));
                
                let dir = "/test-audio-files";
                let name = "test10-voice-conversion.mp3";
                save_audio(&converted_audio, dir, name);
                test_result.push_str(&format!("💾 Audio saved at {dir}/{name}\n"));
            }
            Err(err) => {
                trace!("Failed to convert voice.");
                test_result.push_str(&format!("{test3_name} ❌\n"));
                test_result.push_str(&format!("ERROR : {:?}\n", err));
            }
        }

        // Test 4: Generate sound effect
        trace!("Testing sound effect generation...");
        let test4_name = "4. Test sound effect generation";
        let description = "Ocean waves crashing on a beach";

        match generate_sound_effect(description, Some(10.0), Some(0.8)) {
            Ok(sound_effect) => {
                trace!("Sound effect generated successfully, audio size: {}", sound_effect.len());
                test_result.push_str(&format!("{test4_name} ✅\n"));
                test_result.push_str(&format!("Sound effect size: {} bytes\n", sound_effect.len()));
                
                let dir = "/test-audio-files";
                let name = "test10-sound-effect.mp3";
                save_audio(&sound_effect, dir, name);
                test_result.push_str(&format!("💾 Audio saved at {dir}/{name}\n"));
            }
            Err(err) => {
                trace!("Failed to generate sound effect.");
                test_result.push_str(&format!("{test4_name} ❌\n"));
                test_result.push_str(&format!("ERROR : {:?}\n", err));
            }
        }

        test_result

    }

    // Text long for synthesis
    fn test11() -> String {
        let voice = match get_voice(VOICE_UUID) {
            Ok(voices) => voices,
            Err(err) => {
                return format!("❌ ERROR : {:?}", err);
            }
        };

        let mut test_result = String::new();
        test_result.push_str("Test long-form synthesis summary:\n");

        trace!("Testing long-form synthesis...");
        let test_name = "Test long-form synthesis";
        
        let long_content = format!("{}\n\n{}\n\n{}", TEXT, TEXT, TEXT);
        let output_location = "/test-audio-files/test11-long-form.mp3";
        let chapter_breaks = [0, TEXT.len() as u32, (TEXT.len() * 2) as u32];

        match synthesize_long_form(&long_content, &voice, output_location, Some(&chapter_breaks)) {
            Ok(operation) => {
                // Monitor the operation progress
                let mut attempts = 0;
                let max_attempts = 30;
                
                while attempts < max_attempts {
                    let status = operation.get_status();
                    let progress: f32 = operation.get_progress();
                    trace!("Operation status: {:?}, progress: {:.2}%", status, progress * 100.0);
                    
                    match status {
                        OperationStatus::Completed => {
                            trace!("Long-form synthesis completed successfully");
                            test_result.push_str(&format!("{test_name} ✅\n"));
                            
                            trace!("Getting long-form synthesis result...");
                            match operation.get_result() {
                                Ok(result) => {
                                    trace!("Recieved Long-form synthesis result");
                                    test_result.push_str("1. Test get operation result ✅\n");
                                    test_result.push_str(&format!("Output location: {}\n", result.output_location));
                                    test_result.push_str(&format!("Total duration: {:.2}s\n", result.total_duration));
                                    test_result.push_str(&format!("Chapter durations: {:?}\n", result.chapter_durations));
                                    test_result.push_str(&format!("Request ID: {}\n", result.metadata.request_id));
                                    test_result.push_str(&format!("💾 Audio saved at {}\n", result.output_location));
                                }
                                Err(err) => {
                                    trace!("Failed to get long-form synthesis result");
                                    test_result.push_str("1. Test get operation result ❌\n");
                                    test_result.push_str(&format!("ERROR getting result: {:?}\n", err));
                                }
                            }
                            break;
                        }
                        OperationStatus::Failed => {
                              trace!("Failed to synthesize long-form content. operation failed !");
                                    test_result.push_str(&format!("{test_name} ❌\n"));
                                    test_result.push_str("operation failed !");
                             match operation.get_result(){
                                Ok(result)=> {
                                    if !result.output_location.is_empty() {
                                        trace!("This should not return result if operation failed: {:?}\n",result);
                                        test_result.push_str("This should not return result if operation failed ❌\n");
                                        test_result.push_str(&format!("Output location: {}\n", result.output_location));
                                        test_result.push_str(&format!("Total duration: {:.2}s\n", result.total_duration));
                                        test_result.push_str(&format!("Chapter durations: {:?}\n", result.chapter_durations));
                                        test_result.push_str(&format!("Request ID: {}\n", result.metadata.request_id));
                                        test_result.push_str(&format!("💾 Audio saved at {}\n", result.output_location));
                                    }
                                },
                                Err(err) => {
                                  
                                    test_result.push_str(&format!("ERROR: {:?}\n", err));
                                }
                            };
                            break;
                        }
                        OperationStatus::Cancelled => {
                                  trace!("Failed to synthesize long-form content. operation cancelled !");
                                    test_result.push_str(&format!("{test_name} ❌\n"));
                                    test_result.push_str("operation cancelled !");
                              match operation.get_result(){
                                Ok(result)=> {
                                    if !result.output_location.is_empty() {
                                        trace!("Operation returned result even if operation cancelled: {:?}\n",result);
                                        test_result.push_str("Operation returned result even if operation cancelled ⚠️\n");
                                        test_result.push_str(&format!("Output location: {}\n", result.output_location));
                                        test_result.push_str(&format!("Total duration: {:.2}s\n", result.total_duration));
                                        test_result.push_str(&format!("Chapter durations: {:?}\n", result.chapter_durations));
                                        test_result.push_str(&format!("Request ID: {}\n", result.metadata.request_id));
                                        test_result.push_str(&format!("💾 Audio saved at {}\n", result.output_location));
                                    }
                                },
                                Err(err) => {
                                
                                    test_result.push_str(&format!("ERROR: {:?}\n", err));
                                }
                            };
                         
                            break;
                        }
                        _ => {
                            // Still processing
                            thread::sleep(Duration::from_millis(500));
                        }
                    }
                    
                    attempts += 1;
                }
                
                if attempts >= max_attempts {
                    test_result.push_str("2. Test long-form synthesis completion ❌\n");
                    test_result.push_str("ERROR: Operation timed out\n");
                }
            }
            Err(err) => {
                test_result.push_str(&format!("{test_name} ❌\n"));
                test_result.push_str(&format!("ERROR : {:?}\n", err));
                return test_result;
            }
        }

        test_result
    }

    // Test pronunciation lexicons
    fn test12() -> String {
        let voice = match get_voice(VOICE_UUID) {
            Ok(voices) => voices,
            Err(err) => {
                return format!("❌ ERROR : {:?}", err);
            }
        };

        let mut test_result = String::new();
        test_result.push_str("Test pronunciation lexicons summary:\n");

        trace!("Creating pronunciation lexicon...");
        let test_name = "1. Test create lexicon";
        
        let pronunciation_entries = [
            PronunciationEntry {
                word: "Golem".to_string(),
                pronunciation: "GOH-lem".to_string(),
                part_of_speech: Some("noun".to_string()),
            },
            PronunciationEntry {
                word: "synthesis".to_string(),
                pronunciation: "SIN-thuh-sis".to_string(),
                part_of_speech: Some("noun".to_string()),
            },
        ];

        let lexicon = match create_lexicon("test-lexicon", "en-US", Some(&pronunciation_entries)) {
            Ok(lexicon) => {
                trace!("Lexicon created successfully: {}", lexicon.get_name());
                test_result.push_str(&format!("{test_name} ✅\n"));
                test_result.push_str(&format!("Lexicon name: {}\n", lexicon.get_name()));
                test_result.push_str(&format!("Lexicon language: {}\n", lexicon.get_language()));
                test_result.push_str(&format!("Entry count: {}\n", lexicon.get_entry_count()));
                lexicon
            }
            Err(err) => {
                trace!("Failed to create lexicon.");
                test_result.push_str(&format!("{test_name} ❌\n"));
                test_result.push_str(&format!("ERROR : {:?}\n", err));
                return test_result;
            }
        };

        trace!("Adding entry to lexicon...");
        let test2_name = "2. Test add lexicon entry";
        
        match lexicon.add_entry("coastal", "KOHS-tuhl") {
            Ok(_) => {
                trace!("Entry added successfully");
                test_result.push_str(&format!("{test2_name} ✅\n"));
                test_result.push_str(&format!("Updated entry count: {}\n", lexicon.get_entry_count()));
            }
            Err(err) => {
                trace!("Failed to add entry to lexicon.");
                test_result.push_str(&format!("{test2_name} ❌\n"));
                test_result.push_str(&format!("ERROR : {:?}\n", err));
            }
        }

        trace!("Exporting lexicon content...");
        let test3_name = "3. Test export lexicon";
        
        match lexicon.export_content() {
            Ok(content) => {
                trace!("Lexicon exported successfully, content length: {}", content.len());
                test_result.push_str(&format!("{test3_name} ✅\n"));
                test_result.push_str(&format!("Exported content length: {} characters\n", content.len()));
                if content.len() < 500 {
                    test_result.push_str(&format!("Content preview: {}\n", content));
                }
            }
            Err(err) => {
                trace!("Failed to export lexicon.");
                test_result.push_str(&format!("{test3_name} ❌\n"));
                test_result.push_str(&format!("ERROR : {:?}\n", err));
            }
        }

        trace!("Removing entry from lexicon...");
        let test4_name = "4. Test remove lexicon entry";
        
        match lexicon.remove_entry("coastal") {
            Ok(_) => {
                trace!("Entry removed successfully");
                test_result.push_str(&format!("{test4_name} ✅\n"));
                test_result.push_str(&format!("Final entry count: {}\n", lexicon.get_entry_count()));
            }
            Err(err) => {
                trace!("Failed to remove entry from lexicon.");
                test_result.push_str(&format!("{test4_name} ❌\n"));
                test_result.push_str(&format!("ERROR : {:?}\n", err));
            }
        }

        test_result
    }
    

}

fn save_audio(audio_data: &[u8], dir: &str, name: &str) {
    match fs::create_dir(dir) {
        Ok(_) => {}
        Err(e) => trace!("❌ ERROR : failled to create directory {e}"),
    };
    match fs::write(format!("/{}/{}", dir, name), audio_data) {
        Ok(_) => {}
        Err(e) => trace!("❌ Failed to save audio {name} ERROR  :{e}"),
    };
}

fn push_result(
    test_result: &mut String,
    result: SynthesisResult,
    test_name: &str,
    audio_file_location: String,
) {
    test_result.push_str(&format!("{test_name} ✅ \n"));
    test_result.push_str(&format!("Request ID: {:?}\n", result.metadata.request_id));
    test_result.push_str(&format!(
        "Audio Size: {:?}\n",
        result.metadata.audio_size_bytes
    ));
    test_result.push_str(&format!(
        "Character Count: {:?}\n",
        result.metadata.character_count
    ));
    test_result.push_str(&format!(
        "Duration in seconds: {:?}\n",
        result.metadata.duration_seconds
    ));
    test_result.push_str(&format!(
        "Provider Info: {:?}\n",
        result.metadata.provider_info
    ));
    test_result.push_str(&format!("Word Count: {:?}\n", result.metadata.word_count));
    test_result.push_str(&format!("💾 Audio saved at {audio_file_location}"));
}

fn mimic_crash(worker_name: &str) {
    atomically(|| {
        let client = TestHelperApi::new(&worker_name);
        let counter = client.blocking_inc_and_get();
        trace!("Devs : Server crashed!!! 😱");
        if counter == 1 {
            panic!("Simulating crash during durability test 💥");
        }
        trace!("CTO (Viggo 😎): Don't worry!, It's Golem 💪");
    });
}

bindings::export!(Component with_types_in bindings);
