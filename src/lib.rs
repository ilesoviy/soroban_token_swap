//! This contract implements swap of one token pair between one offeror and
//! multiple acceptors.
//! It demonstrates one of the ways of how swap might be implemented.
#![no_std]

mod storage_types;
mod fee;
mod allow;
mod offer;

mod test;
