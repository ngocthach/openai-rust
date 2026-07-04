# Phase 01 — Chat Multimodal Content Parts

**Owner:** main agent (IN PROGRESS — do not assign to a subagent).
**Parallel group:** A (serial, runs first). Edits shared `src/types/chat.rs`, so it
must merge before phase-08 wiring; other parallel phases do not touch this file.

## Context Links
- Python: `types/chat/chat_completion_content_part_param.py` (union L40-45),
  `chat_completion_content_part_text_param.py`, `..._image_param.py` (L10-28),
  `..._input_audio_param.py` (L10-24).
- Rust target: `src/types/chat.rs` (already has `MessageContent`, `ContentPart`,
  re-exported in `lib.rs`).

## Overview
- **Priority:** P1 (unblocks vision/audio chat; already re-exported in `lib.rs`).
- **Status:** in progress by main agent.
- Extend chat message `content` to be `String | Vec<ContentPart>` with typed parts.

## Key Insights
- Discriminator field is `"type"` on every part (untagged at the alias level in
  Python; model as internally-tagged `#[serde(tag = "type")]` enum in Rust).
- 4 Python variants: `text`, `image_url`, `input_audio`, `file`. Port the first 3
  in scope; `file` optional (KISS — add only if trivial).
- Enum values (verbatim): image `detail` = `auto|low|high`; audio `format` = `wav|mp3`.

## Requirements
- `MessageContent` serializes untagged as either a bare string or an array.
- `ContentPart::Text { text }`, `Image { image_url: ImageUrl { url, detail? } }`,
  `InputAudio { input_audio: InputAudio { data, format } }`.
- `detail`/`format` as `Option<String>` or small enums; `#[serde(skip_serializing_if)]`
  on optional fields.

## Architecture
```
MessageContent (untagged enum): Text(String) | Parts(Vec<ContentPart>)
ContentPart (#[serde(tag="type", rename_all="snake_case")]):
  Text { text: String }
  ImageUrl { image_url: ImageUrl }   // rename to "image_url"
  InputAudio { input_audio: InputAudio }
ImageUrl { url: String, detail: Option<String> }
InputAudio { data: String, format: String }  // "wav" | "mp3"
```

## Related Code Files
- Modify: `src/types/chat.rs` (extend existing `ContentPart`/`MessageContent`).
- No new accessors; `lib.rs` re-exports already list `ContentPart`, `MessageContent`.

## Implementation Steps
1. Define/extend `ContentPart` internally-tagged enum with `Text`/`ImageUrl`/`InputAudio`.
2. Add `ImageUrl` and `InputAudio` sub-structs with skip-none optionals.
3. Ensure `MessageContent` is `#[serde(untagged)]` over `String` and `Vec<ContentPart>`.
4. Add constructor helpers (`ContentPart::text`, `::image_url`, `::input_audio`).
5. `cargo build` + round-trip serde unit tests in `src/types/chat.rs` `#[cfg(test)]`.

## Todo List
- [ ] Extend `ContentPart` enum + sub-structs
- [ ] `MessageContent` untagged String|Vec
- [ ] Constructor helpers
- [ ] Serde round-trip unit tests (string form + parts form)
- [ ] `cargo build` clean

## Success Criteria
- A chat request with `content` = vec of text+image_url serializes to the exact
  OpenAI JSON shape; a plain-string content still serializes as a bare string.

## Risk Assessment
- Untagged enum ordering: `String` variant must be tried before `Vec` — order fields so serde disambiguates. Mitigate with round-trip tests.

## Security Considerations
- None beyond existing (no new I/O).

## Next Steps
- Must be merged before phase-08. Independent of phases 02–07.
