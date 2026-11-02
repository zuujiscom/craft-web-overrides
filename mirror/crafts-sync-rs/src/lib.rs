//! crafts-sync: mirrors the browser builds of the storytold/ArtCraft apps (see main.rs and the README).
#![forbid(unsafe_code)]

pub mod http;
pub mod manifest;
pub mod page;
pub mod sync;
pub mod util;

pub type R<T> = Result<T, Box<dyn std::error::Error>>;
