//! Spec-chain scenarios for DEC private rectangular-area ops.
//!
//! Section 09A.5 landed `DECRQCRA`; §09A.6 adds the six mutation ops
//! (DECSACE + DECCARA + DECRARA + DECCRA + DECFRA + DECERA + DECSERA).
//! One file per sequence keeps the matrix shape obvious.

mod deccara;
mod deccra;
mod decera;
mod decfra;
mod decrara;
mod decrqcra;
mod decsace;
mod decsera;
