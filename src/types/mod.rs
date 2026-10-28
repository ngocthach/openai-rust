//! Request/response types, one module per endpoint group (mirrors
//! `openai-python/src/openai/types/`).

pub mod assistants;
pub mod audio;
pub mod batches;
pub mod chat;
pub mod common;
pub mod completions;
pub mod conversations;
pub mod embeddings;
pub mod files;
pub mod fine_tuning;
pub mod images;
pub mod models;
pub mod moderations;
pub mod responses;
pub mod uploads;
pub mod vector_stores;
