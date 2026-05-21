//! Pilot scenarios for the verification chain.
//!
//! Each pilot exercises a representative sequence end-to-end through
//! the harness, proving the API works before the schema freeze locks
//! the catalog row format.

mod da1_query;
mod notcurses_info_exact;
mod notcurses_info_handshake;
mod notcurses_startup;
