//! Pure logic for QR Code Generator: the encoder and the persisted model.
//! No Xous dependencies; everything here is unit-tested on the host.

#![no_std]
extern crate alloc;

pub mod encode;
pub mod model;

pub use encode::{encode, module_px, EcLevel, Mode, QrCode, QrError};
pub use model::{Codes, IndexRecord, LegacyCode, SaveError, SavedCode, Settings, Template, TextField};

#[cfg(test)]
mod tests;
