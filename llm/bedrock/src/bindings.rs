// Generated by `wit-bindgen` 0.36.0. DO NOT EDIT!
// Options used:
//   * runtime_path: "wit_bindgen_rt"
//   * with "golem:llm/llm@1.0.0" = "golem_llm::golem::llm::llm"
//   * generate_unused_types
use golem_llm::golem::llm::llm as __with_name0;
#[cfg(target_arch = "wasm32")]
#[link_section = "component-type:wit-bindgen:0.36.0:golem:llm-bedrock@1.0.0:llm-library:encoded world"]
#[doc(hidden)]
pub static __WIT_BINDGEN_COMPONENT_TYPE: [u8; 1760] = *b"\
\0asm\x0d\0\x01\0\0\x19\x16wit-component-encoding\x04\0\x07\xde\x0c\x01A\x02\x01\
A\x02\x01BO\x01m\x04\x04user\x09assistant\x06system\x04tool\x04\0\x04role\x03\0\0\
\x01m\x06\x0finvalid-request\x15authentication-failed\x13rate-limit-exceeded\x0e\
internal-error\x0bunsupported\x07unknown\x04\0\x0aerror-code\x03\0\x02\x01m\x06\x04\
stop\x06length\x0atool-calls\x0econtent-filter\x05error\x05other\x04\0\x0dfinish\
-reason\x03\0\x04\x01m\x03\x03low\x04high\x04auto\x04\0\x0cimage-detail\x03\0\x06\
\x01k\x07\x01r\x02\x03urls\x06detail\x08\x04\0\x09image-url\x03\0\x09\x01p}\x01r\
\x03\x04data\x0b\x09mime-types\x06detail\x08\x04\0\x0cimage-source\x03\0\x0c\x01\
q\x02\x03url\x01\x0a\0\x06inline\x01\x0d\0\x04\0\x0fimage-reference\x03\0\x0e\x01\
q\x02\x04text\x01s\0\x05image\x01\x0f\0\x04\0\x0ccontent-part\x03\0\x10\x01ks\x01\
p\x11\x01r\x03\x04role\x01\x04name\x12\x07content\x13\x04\0\x07message\x03\0\x14\
\x01r\x03\x04names\x0bdescription\x12\x11parameters-schemas\x04\0\x0ftool-defini\
tion\x03\0\x16\x01r\x03\x02ids\x04names\x0earguments-jsons\x04\0\x09tool-call\x03\
\0\x18\x01ky\x01r\x04\x02ids\x04names\x0bresult-jsons\x11execution-time-ms\x1a\x04\
\0\x0ctool-success\x03\0\x1b\x01r\x04\x02ids\x04names\x0derror-messages\x0aerror\
-code\x12\x04\0\x0ctool-failure\x03\0\x1d\x01q\x02\x07success\x01\x1c\0\x05error\
\x01\x1e\0\x04\0\x0btool-result\x03\0\x1f\x01r\x02\x03keys\x05values\x04\0\x02kv\
\x03\0!\x01kv\x01ps\x01k$\x01p\x17\x01p\"\x01r\x07\x05models\x0btemperature#\x0a\
max-tokens\x1a\x0estop-sequences%\x05tools&\x0btool-choice\x12\x10provider-optio\
ns'\x04\0\x06config\x03\0(\x01r\x03\x0cinput-tokens\x1a\x0doutput-tokens\x1a\x0c\
total-tokens\x1a\x04\0\x05usage\x03\0*\x01k\x05\x01k+\x01r\x05\x0dfinish-reason,\
\x05usage-\x0bprovider-id\x12\x09timestamp\x12\x16provider-metadata-json\x12\x04\
\0\x11response-metadata\x03\0.\x01p\x19\x01r\x04\x02ids\x07content\x13\x0atool-c\
alls0\x08metadata/\x04\0\x11complete-response\x03\01\x01r\x03\x04code\x03\x07mes\
sages\x13provider-error-json\x12\x04\0\x05error\x03\03\x01q\x03\x07message\x012\0\
\x0ctool-request\x010\0\x05error\x014\0\x04\0\x0achat-event\x03\05\x01k\x13\x01k\
0\x01r\x02\x07content7\x0atool-calls8\x04\0\x0cstream-delta\x03\09\x01q\x03\x05d\
elta\x01:\0\x06finish\x01/\0\x05error\x014\0\x04\0\x0cstream-event\x03\0;\x04\0\x0b\
chat-stream\x03\x01\x01h=\x01p<\x01k?\x01@\x01\x04self>\0\xc0\0\x04\0\x1c[method\
]chat-stream.get-next\x01A\x01@\x01\x04self>\0?\x04\0%[method]chat-stream.blocki\
ng-get-next\x01B\x01p\x15\x01@\x02\x08messages\xc3\0\x06config)\06\x04\0\x04send\
\x01D\x01o\x02\x19\x20\x01p\xc5\0\x01@\x03\x08messages\xc3\0\x0ctool-results\xc6\
\0\x06config)\06\x04\0\x08continue\x01G\x01i=\x01@\x02\x08messages\xc3\0\x06conf\
ig)\0\xc8\0\x04\0\x06stream\x01I\x04\0\x13golem:llm/llm@1.0.0\x05\0\x04\0#golem:\
llm-bedrock/llm-library@1.0.0\x04\0\x0b\x11\x01\0\x0bllm-library\x03\0\0\0G\x09p\
roducers\x01\x0cprocessed-by\x02\x0dwit-component\x070.220.1\x10wit-bindgen-rust\
\x060.36.0";
#[inline(never)]
#[doc(hidden)]
pub fn __link_custom_section_describing_imports() {
    wit_bindgen_rt::maybe_link_cabi_realloc();
}
