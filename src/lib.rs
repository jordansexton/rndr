#![deny(missing_docs)]

//! A RNDR program for the Solana blockchain.

pub mod entrypoint;
pub mod error;
pub mod instruction;
pub mod processor;
pub mod state;

mod helpers;

solana_program::declare_id!("RNDR111111111111111111111111111111111111111");