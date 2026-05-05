//! Catalog rows: ECMA48-C1-8BIT

use std::vec::Vec;

use super::*;

const OSC_BYTES: &[u8] = &[
    0x1B, 0x5D, // Begin OSC
    b'2', b';', b'j', b'w', b'i', b'l', b'm', b'@', b'j', b'w', b'i', b'l', b'm', b'-', b'd',
    b'e', b's', b'k', b':', b' ', b'~', b'/', b'c', b'o', b'd', b'e', b'/', b'a', b'l', b'a',
    b'c', b'r', b'i', b't', b't', b'y', 0x07, // End OSC
];

#[derive(Default)]
struct Dispatcher {
    dispatched: Vec<Sequence>,
}

#[derive(Debug, PartialEq, Eq)]
enum Sequence {
    Osc(Vec<Vec<u8>>, bool),
    Csi(Vec<Vec<u16>>, Vec<u8>, bool, char),
    Esc(Vec<u8>, bool, u8),
    DcsHook(Vec<Vec<u16>>, Vec<u8>, bool, char),
    DcsPut(u8),
    Print(char),
    Execute(u8),
    DcsUnhook,
    ApcStart,
    ApcPut(u8),
    ApcEnd,
}

impl Perform for Dispatcher {
    fn osc_dispatch(&mut self, params: &[&[u8]], bell_terminated: bool) {
        let params = params.iter().map(|p| p.to_vec()).collect();
        self.dispatched.push(Sequence::Osc(params, bell_terminated));
    }

    fn csi_dispatch(&mut self, params: &Params, intermediates: &[u8], ignore: bool, c: char) {
        let params = params.iter().map(|subparam| subparam.to_vec()).collect();
        let intermediates = intermediates.to_vec();
        self.dispatched.push(Sequence::Csi(params, intermediates, ignore, c));
    }

    fn esc_dispatch(&mut self, intermediates: &[u8], ignore: bool, byte: u8) {
        let intermediates = intermediates.to_vec();
        self.dispatched.push(Sequence::Esc(intermediates, ignore, byte));
    }

    fn hook(&mut self, params: &Params, intermediates: &[u8], ignore: bool, c: char) {
        let params = params.iter().map(|subparam| subparam.to_vec()).collect();
        let intermediates = intermediates.to_vec();
        self.dispatched.push(Sequence::DcsHook(params, intermediates, ignore, c));
    }

    fn put(&mut self, byte: u8) {
        self.dispatched.push(Sequence::DcsPut(byte));
    }

    fn unhook(&mut self) {
        self.dispatched.push(Sequence::DcsUnhook);
    }

    fn print(&mut self, c: char) {
        self.dispatched.push(Sequence::Print(c));
    }

    fn execute(&mut self, byte: u8) {
        self.dispatched.push(Sequence::Execute(byte));
    }

    fn apc_start(&mut self) {
        self.dispatched.push(Sequence::ApcStart);
    }

    fn apc_put(&mut self, byte: u8) {
        self.dispatched.push(Sequence::ApcPut(byte));
    }

    fn apc_end(&mut self) {
        self.dispatched.push(Sequence::ApcEnd);
    }
}

#[test]
fn parse_osc() {
    let mut dispatcher = Dispatcher::default();
    let mut parser = Parser::new();

    parser.advance(&mut dispatcher, OSC_BYTES);

    assert_eq!(dispatcher.dispatched.len(), 1);
    match &dispatcher.dispatched[0] {
        Sequence::Osc(params, _) => {
            assert_eq!(params.len(), 2);
            assert_eq!(params[0], &OSC_BYTES[2..3]);
            assert_eq!(params[1], &OSC_BYTES[4..(OSC_BYTES.len() - 1)]);
        },
        _ => panic!("expected osc sequence"),
    }
}

#[test]
fn parse_empty_osc() {
    let mut dispatcher = Dispatcher::default();
    let mut parser = Parser::new();

    parser.advance(&mut dispatcher, &[0x1B, 0x5D, 0x07]);

    assert_eq!(dispatcher.dispatched.len(), 1);
    match &dispatcher.dispatched[0] {
        Sequence::Osc(..) => (),
        _ => panic!("expected osc sequence"),
    }
}

#[test]
fn parse_osc_max_params() {
    let params = ";".repeat(params::MAX_PARAMS + 1);
    let input = format!("\x1b]{}\x1b", &params[..]).into_bytes();
    let mut dispatcher = Dispatcher::default();
    let mut parser = Parser::new();

    parser.advance(&mut dispatcher, &input);

    assert_eq!(dispatcher.dispatched.len(), 1);
    match &dispatcher.dispatched[0] {
        Sequence::Osc(params, _) => {
            assert_eq!(params.len(), MAX_OSC_PARAMS);
            assert!(params.iter().all(Vec::is_empty));
        },
        _ => panic!("expected osc sequence"),
    }
}

#[test]
fn osc_bell_terminated() {
    const INPUT: &[u8] = b"\x1b]11;ff/00/ff\x07";
    let mut dispatcher = Dispatcher::default();
    let mut parser = Parser::new();

    parser.advance(&mut dispatcher, INPUT);

    assert_eq!(dispatcher.dispatched.len(), 1);
    match &dispatcher.dispatched[0] {
        Sequence::Osc(_, true) => (),
        _ => panic!("expected osc with bell terminator"),
    }
}

#[test]
fn osc_c0_st_terminated() {
    const INPUT: &[u8] = b"\x1b]11;ff/00/ff\x1b\\";
    let mut dispatcher = Dispatcher::default();
    let mut parser = Parser::new();

    parser.advance(&mut dispatcher, INPUT);

    assert_eq!(dispatcher.dispatched.len(), 2);
    match &dispatcher.dispatched[0] {
        Sequence::Osc(_, false) => (),
        _ => panic!("expected osc with ST terminator"),
    }
}

#[test]
fn parse_osc_with_utf8_arguments() {
    const INPUT: &[u8] = &[
        0x0D, 0x1B, 0x5D, 0x32, 0x3B, 0x65, 0x63, 0x68, 0x6F, 0x20, 0x27, 0xC2, 0xAF, 0x5C,
        0x5F, 0x28, 0xE3, 0x83, 0x84, 0x29, 0x5F, 0x2F, 0xC2, 0xAF, 0x27, 0x20, 0x26, 0x26,
        0x20, 0x73, 0x6C, 0x65, 0x65, 0x70, 0x20, 0x31, 0x07,
    ];
    let mut dispatcher = Dispatcher::default();
    let mut parser = Parser::new();

    parser.advance(&mut dispatcher, INPUT);

    assert_eq!(dispatcher.dispatched[0], Sequence::Execute(b'\r'));
    let osc_data = INPUT[5..(INPUT.len() - 1)].into();
    assert_eq!(dispatcher.dispatched[1], Sequence::Osc(vec![vec![b'2'], osc_data], true));
    assert_eq!(dispatcher.dispatched.len(), 2);
}

#[test]
fn osc_containing_string_terminator() {
    const INPUT: &[u8] = b"\x1b]2;\xe6\x9c\xab\x1b\\";
    let mut dispatcher = Dispatcher::default();
    let mut parser = Parser::new();

    parser.advance(&mut dispatcher, INPUT);

    assert_eq!(dispatcher.dispatched.len(), 2);
    match &dispatcher.dispatched[0] {
        Sequence::Osc(params, _) => {
            assert_eq!(params[1], &INPUT[4..(INPUT.len() - 2)]);
        },
        _ => panic!("expected osc sequence"),
    }
}

#[test]
fn exceed_max_buffer_size() {
    const NUM_BYTES: usize = MAX_OSC_RAW + 100;
    const INPUT_START: &[u8] = b"\x1b]52;s";
    const INPUT_END: &[u8] = b"\x07";

    let mut dispatcher = Dispatcher::default();
    let mut parser = Parser::new();

    // Create valid OSC escape.
    parser.advance(&mut dispatcher, INPUT_START);

    // Exceed max buffer size.
    parser.advance(&mut dispatcher, &[b'a'; NUM_BYTES]);

    // Terminate escape for dispatch.
    parser.advance(&mut dispatcher, INPUT_END);

    assert_eq!(dispatcher.dispatched.len(), 1);
    match &dispatcher.dispatched[0] {
        Sequence::Osc(params, _) => {
            assert_eq!(params.len(), 2);
            assert_eq!(params[0], b"52");

            #[cfg(feature = "std")]
            assert_eq!(params[1].len(), NUM_BYTES + INPUT_END.len());

            #[cfg(not(feature = "std"))]
            assert_eq!(params[1].len(), MAX_OSC_RAW - params[0].len());
        },
        _ => panic!("expected osc sequence"),
    }
}

#[test]
fn parse_csi_max_params() {
    // This will build a list of repeating '1;'s
    // The length is MAX_PARAMS - 1 because the last semicolon is interpreted
    // as an implicit zero, making the total number of parameters MAX_PARAMS.
    let params = "1;".repeat(params::MAX_PARAMS - 1);
    let input = format!("\x1b[{}p", &params[..]).into_bytes();

    let mut dispatcher = Dispatcher::default();
    let mut parser = Parser::new();

    parser.advance(&mut dispatcher, &input);

    assert_eq!(dispatcher.dispatched.len(), 1);
    match &dispatcher.dispatched[0] {
        Sequence::Csi(params, _, ignore, _) => {
            assert_eq!(params.len(), params::MAX_PARAMS);
            assert!(!ignore);
        },
        _ => panic!("expected csi sequence"),
    }
}

#[test]
fn parse_csi_params_ignore_long_params() {
    // This will build a list of repeating '1;'s
    // The length is MAX_PARAMS because the last semicolon is interpreted
    // as an implicit zero, making the total number of parameters MAX_PARAMS + 1.
    let params = "1;".repeat(params::MAX_PARAMS);
    let input = format!("\x1b[{}p", &params[..]).into_bytes();

    let mut dispatcher = Dispatcher::default();
    let mut parser = Parser::new();

    parser.advance(&mut dispatcher, &input);

    assert_eq!(dispatcher.dispatched.len(), 1);
    match &dispatcher.dispatched[0] {
        Sequence::Csi(params, _, ignore, _) => {
            assert_eq!(params.len(), params::MAX_PARAMS);
            assert!(ignore);
        },
        _ => panic!("expected csi sequence"),
    }
}

#[test]
fn parse_csi_params_trailing_semicolon() {
    let mut dispatcher = Dispatcher::default();
    let mut parser = Parser::new();

    parser.advance(&mut dispatcher, b"\x1b[4;m");

    assert_eq!(dispatcher.dispatched.len(), 1);
    match &dispatcher.dispatched[0] {
        Sequence::Csi(params, ..) => assert_eq!(params, &[[4], [0]]),
        _ => panic!("expected csi sequence"),
    }
}

#[test]
fn parse_csi_params_leading_semicolon() {
    let mut dispatcher = Dispatcher::default();
    let mut parser = Parser::new();

    parser.advance(&mut dispatcher, b"\x1b[;4m");

    assert_eq!(dispatcher.dispatched.len(), 1);
    match &dispatcher.dispatched[0] {
        Sequence::Csi(params, ..) => assert_eq!(params, &[[0], [4]]),
        _ => panic!("expected csi sequence"),
    }
}

#[test]
fn parse_long_csi_param() {
    // The important part is the parameter, which is (i64::MAX + 1).
    const INPUT: &[u8] = b"\x1b[9223372036854775808m";
    let mut dispatcher = Dispatcher::default();
    let mut parser = Parser::new();

    parser.advance(&mut dispatcher, INPUT);

    assert_eq!(dispatcher.dispatched.len(), 1);
    match &dispatcher.dispatched[0] {
        Sequence::Csi(params, ..) => assert_eq!(params, &[[u16::MAX]]),
        _ => panic!("expected csi sequence"),
    }
}

#[test]
fn csi_reset() {
    const INPUT: &[u8] = b"\x1b[3;1\x1b[?1049h";
    let mut dispatcher = Dispatcher::default();
    let mut parser = Parser::new();

    parser.advance(&mut dispatcher, INPUT);

    assert_eq!(dispatcher.dispatched.len(), 1);
    match &dispatcher.dispatched[0] {
        Sequence::Csi(params, intermediates, ignore, _) => {
            assert_eq!(intermediates, b"?");
            assert_eq!(params, &[[1049]]);
            assert!(!ignore);
        },
        _ => panic!("expected csi sequence"),
    }
}

#[test]
fn csi_subparameters() {
    const INPUT: &[u8] = b"\x1b[38:2:255:0:255;1m";
    let mut dispatcher = Dispatcher::default();
    let mut parser = Parser::new();

    parser.advance(&mut dispatcher, INPUT);

    assert_eq!(dispatcher.dispatched.len(), 1);
    match &dispatcher.dispatched[0] {
        Sequence::Csi(params, intermediates, ignore, _) => {
            assert_eq!(params, &[vec![38, 2, 255, 0, 255], vec![1]]);
            assert_eq!(intermediates, &[]);
            assert!(!ignore);
        },
        _ => panic!("expected csi sequence"),
    }
}

#[test]
fn parse_dcs_max_params() {
    let params = "1;".repeat(params::MAX_PARAMS + 1);
    let input = format!("\x1bP{}p", &params[..]).into_bytes();
    let mut dispatcher = Dispatcher::default();
    let mut parser = Parser::new();

    parser.advance(&mut dispatcher, &input);

    assert_eq!(dispatcher.dispatched.len(), 1);
    match &dispatcher.dispatched[0] {
        Sequence::DcsHook(params, _, ignore, _) => {
            assert_eq!(params.len(), params::MAX_PARAMS);
            assert!(params.iter().all(|param| param == &[1]));
            assert!(ignore);
        },
        _ => panic!("expected dcs sequence"),
    }
}

#[test]
fn dcs_reset() {
    const INPUT: &[u8] = b"\x1b[3;1\x1bP1$tx\x9c";
    let mut dispatcher = Dispatcher::default();
    let mut parser = Parser::new();

    parser.advance(&mut dispatcher, INPUT);

    assert_eq!(dispatcher.dispatched.len(), 3);
    match &dispatcher.dispatched[0] {
        Sequence::DcsHook(params, intermediates, ignore, _) => {
            assert_eq!(intermediates, b"$");
            assert_eq!(params, &[[1]]);
            assert!(!ignore);
        },
        _ => panic!("expected dcs sequence"),
    }
    assert_eq!(dispatcher.dispatched[1], Sequence::DcsPut(b'x'));
    assert_eq!(dispatcher.dispatched[2], Sequence::DcsUnhook);
}

#[test]
fn parse_dcs() {
    const INPUT: &[u8] = b"\x1bP0;1|17/ab\x9c";
    let mut dispatcher = Dispatcher::default();
    let mut parser = Parser::new();

    parser.advance(&mut dispatcher, INPUT);

    assert_eq!(dispatcher.dispatched.len(), 7);
    match &dispatcher.dispatched[0] {
        Sequence::DcsHook(params, _, _, c) => {
            assert_eq!(params, &[[0], [1]]);
            assert_eq!(c, &'|');
        },
        _ => panic!("expected dcs sequence"),
    }
    for (i, byte) in b"17/ab".iter().enumerate() {
        assert_eq!(dispatcher.dispatched[1 + i], Sequence::DcsPut(*byte));
    }
    assert_eq!(dispatcher.dispatched[6], Sequence::DcsUnhook);
}

#[test]
fn intermediate_reset_on_dcs_exit() {
    const INPUT: &[u8] = b"\x1bP=1sZZZ\x1b+\x5c";
    let mut dispatcher = Dispatcher::default();
    let mut parser = Parser::new();

    parser.advance(&mut dispatcher, INPUT);

    assert_eq!(dispatcher.dispatched.len(), 6);
    match &dispatcher.dispatched[5] {
        Sequence::Esc(intermediates, ..) => assert_eq!(intermediates, b"+"),
        _ => panic!("expected esc sequence"),
    }
}

#[test]
fn esc_reset() {
    const INPUT: &[u8] = b"\x1b[3;1\x1b(A";
    let mut dispatcher = Dispatcher::default();
    let mut parser = Parser::new();

    parser.advance(&mut dispatcher, INPUT);

    assert_eq!(dispatcher.dispatched.len(), 1);
    match &dispatcher.dispatched[0] {
        Sequence::Esc(intermediates, ignore, byte) => {
            assert_eq!(intermediates, b"(");
            assert_eq!(*byte, b'A');
            assert!(!ignore);
        },
        _ => panic!("expected esc sequence"),
    }
}

#[test]
fn esc_reset_intermediates() {
    const INPUT: &[u8] = b"\x1b[?2004l\x1b#8";
    let mut dispatcher = Dispatcher::default();
    let mut parser = Parser::new();

    parser.advance(&mut dispatcher, INPUT);

    assert_eq!(dispatcher.dispatched.len(), 2);
    assert_eq!(dispatcher.dispatched[0], Sequence::Csi(vec![vec![2004]], vec![63], false, 'l'));
    assert_eq!(dispatcher.dispatched[1], Sequence::Esc(vec![35], false, 56));
}

#[test]
fn params_buffer_filled_with_subparam() {
    const INPUT: &[u8] = b"\x1b[::::::::::::::::::::::::::::::::x\x1b";
    let mut dispatcher = Dispatcher::default();
    let mut parser = Parser::new();

    parser.advance(&mut dispatcher, INPUT);

    assert_eq!(dispatcher.dispatched.len(), 1);
    match &dispatcher.dispatched[0] {
        Sequence::Csi(params, intermediates, ignore, c) => {
            assert_eq!(intermediates, &[]);
            assert_eq!(params, &[[0; 32]]);
            assert_eq!(c, &'x');
            assert!(ignore);
        },
        _ => panic!("expected csi sequence"),
    }
}

#[cfg(not(feature = "std"))]
#[test]
fn build_with_fixed_size() {
    const INPUT: &[u8] = b"\x1b[3;1\x1b[?1049h";
    let mut dispatcher = Dispatcher::default();
    let mut parser: Parser<30> = Parser::new_with_size();

    parser.advance(&mut dispatcher, INPUT);

    assert_eq!(dispatcher.dispatched.len(), 1);
    match &dispatcher.dispatched[0] {
        Sequence::Csi(params, intermediates, ignore, _) => {
            assert_eq!(intermediates, b"?");
            assert_eq!(params, &[[1049]]);
            assert!(!ignore);
        },
        _ => panic!("expected csi sequence"),
    }
}

#[cfg(not(feature = "std"))]
#[test]
fn exceed_fixed_osc_buffer_size() {
    const OSC_BUFFER_SIZE: usize = 32;
    const NUM_BYTES: usize = OSC_BUFFER_SIZE + 100;
    const INPUT_START: &[u8] = b"\x1b]52;";
    const INPUT_END: &[u8] = b"\x07";

    let mut dispatcher = Dispatcher::default();
    let mut parser: Parser<OSC_BUFFER_SIZE> = Parser::new_with_size();

    // Create valid OSC escape.
    parser.advance(&mut dispatcher, INPUT_START);

    // Exceed max buffer size.
    parser.advance(&mut dispatcher, &[b'a'; NUM_BYTES]);

    // Terminate escape for dispatch.
    parser.advance(&mut dispatcher, INPUT_END);

    assert_eq!(dispatcher.dispatched.len(), 1);
    match &dispatcher.dispatched[0] {
        Sequence::Osc(params, _) => {
            assert_eq!(params.len(), 2);
            assert_eq!(params[0], b"52");
            assert_eq!(params[1].len(), OSC_BUFFER_SIZE - params[0].len());
            for item in params[1].iter() {
                assert_eq!(*item, b'a');
            }
        },
        _ => panic!("expected osc sequence"),
    }
}

#[cfg(not(feature = "std"))]
#[test]
fn fixed_size_osc_containing_string_terminator() {
    const INPUT_START: &[u8] = b"\x1b]2;";
    const INPUT_MIDDLE: &[u8] = b"s\xe6\x9c\xab";
    const INPUT_END: &[u8] = b"\x1b\\";

    let mut dispatcher = Dispatcher::default();
    let mut parser: Parser<5> = Parser::new_with_size();

    parser.advance(&mut dispatcher, INPUT_START);
    parser.advance(&mut dispatcher, INPUT_MIDDLE);
    parser.advance(&mut dispatcher, INPUT_END);

    assert_eq!(dispatcher.dispatched.len(), 2);
    match &dispatcher.dispatched[0] {
        Sequence::Osc(params, false) => {
            assert_eq!(params[0], b"2");
            assert_eq!(params[1], INPUT_MIDDLE);
        },
        _ => panic!("expected osc sequence"),
    }
}

#[test]
fn unicode() {
    const INPUT: &[u8] = b"\xF0\x9F\x8E\x89_\xF0\x9F\xA6\x80\xF0\x9F\xA6\x80_\xF0\x9F\x8E\x89";

    let mut dispatcher = Dispatcher::default();
    let mut parser = Parser::new();

    parser.advance(&mut dispatcher, INPUT);

    assert_eq!(dispatcher.dispatched.len(), 6);
    assert_eq!(dispatcher.dispatched[0], Sequence::Print('\u{1f389}'));
    assert_eq!(dispatcher.dispatched[1], Sequence::Print('_'));
    assert_eq!(dispatcher.dispatched[2], Sequence::Print('\u{1f980}'));
    assert_eq!(dispatcher.dispatched[3], Sequence::Print('\u{1f980}'));
    assert_eq!(dispatcher.dispatched[4], Sequence::Print('_'));
    assert_eq!(dispatcher.dispatched[5], Sequence::Print('\u{1f389}'));
}

#[test]
fn invalid_utf8() {
    const INPUT: &[u8] = b"a\xEF\xBCb";

    let mut dispatcher = Dispatcher::default();
    let mut parser = Parser::new();

    parser.advance(&mut dispatcher, INPUT);

    assert_eq!(dispatcher.dispatched.len(), 3);
    assert_eq!(dispatcher.dispatched[0], Sequence::Print('a'));
    assert_eq!(dispatcher.dispatched[1], Sequence::Print('\u{FFFD}'));
    assert_eq!(dispatcher.dispatched[2], Sequence::Print('b'));
}

#[test]
fn partial_utf8() {
    const INPUT: &[u8] = b"\xF0\x9F\x9A\x80";

    let mut dispatcher = Dispatcher::default();
    let mut parser = Parser::new();

    parser.advance(&mut dispatcher, &INPUT[..1]);
    parser.advance(&mut dispatcher, &INPUT[1..2]);
    parser.advance(&mut dispatcher, &INPUT[2..3]);
    parser.advance(&mut dispatcher, &INPUT[3..]);

    assert_eq!(dispatcher.dispatched.len(), 1);
    assert_eq!(dispatcher.dispatched[0], Sequence::Print('\u{1f680}'));
}

#[test]
fn partial_utf8_separating_utf8() {
    // This is different from the `partial_utf8` test since it has a multi-byte UTF8
    // character after the partial UTF8 state, causing a partial byte to be present
    // in the `partial_utf8` buffer after the 2-byte codepoint.

    // "ĸ\u{1f389}"
    const INPUT: &[u8] = b"\xC4\xB8\xF0\x9F\x8E\x89";

    let mut dispatcher = Dispatcher::default();
    let mut parser = Parser::new();

    parser.advance(&mut dispatcher, &INPUT[..1]);
    parser.advance(&mut dispatcher, &INPUT[1..]);

    assert_eq!(dispatcher.dispatched.len(), 2);
    assert_eq!(dispatcher.dispatched[0], Sequence::Print('\u{0138}'));
    assert_eq!(dispatcher.dispatched[1], Sequence::Print('\u{1f389}'));
}

#[test]
fn partial_invalid_utf8() {
    const INPUT: &[u8] = b"a\xEF\xBCb";

    let mut dispatcher = Dispatcher::default();
    let mut parser = Parser::new();

    parser.advance(&mut dispatcher, &INPUT[..1]);
    parser.advance(&mut dispatcher, &INPUT[1..2]);
    parser.advance(&mut dispatcher, &INPUT[2..3]);
    parser.advance(&mut dispatcher, &INPUT[3..]);

    assert_eq!(dispatcher.dispatched.len(), 3);
    assert_eq!(dispatcher.dispatched[0], Sequence::Print('a'));
    assert_eq!(dispatcher.dispatched[1], Sequence::Print('\u{FFFD}'));
    assert_eq!(dispatcher.dispatched[2], Sequence::Print('b'));
}

#[test]
fn partial_invalid_utf8_split() {
    const INPUT: &[u8] = b"\xE4\xBF\x99\xB5";

    let mut dispatcher = Dispatcher::default();
    let mut parser = Parser::new();

    parser.advance(&mut dispatcher, &INPUT[..2]);
    parser.advance(&mut dispatcher, &INPUT[2..]);

    assert_eq!(dispatcher.dispatched[0], Sequence::Print('\u{4FD9}'));
    assert_eq!(dispatcher.dispatched[1], Sequence::Print('\u{FFFD}'));
}

#[test]
fn partial_utf8_into_esc() {
    const INPUT: &[u8] = b"\xD8\x1b012";

    let mut dispatcher = Dispatcher::default();
    let mut parser = Parser::new();

    parser.advance(&mut dispatcher, INPUT);

    assert_eq!(dispatcher.dispatched.len(), 4);
    assert_eq!(dispatcher.dispatched[0], Sequence::Print('\u{FFFD}'));
    assert_eq!(dispatcher.dispatched[1], Sequence::Esc(Vec::new(), false, b'0'));
    assert_eq!(dispatcher.dispatched[2], Sequence::Print('1'));
    assert_eq!(dispatcher.dispatched[3], Sequence::Print('2'));
}

#[test]
fn c1s() {
    // Non-sequence C1 bytes (0x80) are dispatched via execute().
    // Sequence-introducing C1 bytes (0x90, 0x98, 0x9B, 0x9C, 0x9D, 0x9E, 0x9F)
    // transition the parser state instead of being dispatched as execute.
    const INPUT: &[u8] = b"\x00\x1f\x80";

    let mut dispatcher = Dispatcher::default();
    let mut parser = Parser::new();

    parser.advance(&mut dispatcher, INPUT);

    assert_eq!(dispatcher.dispatched.len(), 3);
    assert_eq!(dispatcher.dispatched[0], Sequence::Execute(0));
    assert_eq!(dispatcher.dispatched[1], Sequence::Execute(31));
    assert_eq!(dispatcher.dispatched[2], Sequence::Execute(128));
}

/// C1 sequence introducers enter their respective states rather than
/// being dispatched as execute events.
#[test]
fn c1_sequence_introducers_enter_states() {
    // Each C1 byte tested in isolation to avoid state interference.
    let cases: &[(u8, &str)] = &[
        (0x90, "DcsEntry"),
        (0x98, "SosPmApcString"),
        (0x9B, "CsiEntry"),
        (0x9D, "OscString"),
        (0x9E, "SosPmApcString"),
        (0x9F, "ApcString"),
    ];
    for &(byte, expected_state) in cases {
        let mut d = Dispatcher::default();
        let mut p = Parser::new();
        p.advance(&mut d, &[byte]);
        // Sequence introducers do not produce execute events.
        assert!(
            !d.dispatched.iter().any(|s| matches!(s, Sequence::Execute(b) if *b == byte)),
            "C1 byte 0x{:02X} should enter {} state, not dispatch as Execute",
            byte,
            expected_state,
        );
    }
}

#[test]
fn parse_apc_st_terminated() {
    // ESC _ G payload ESC \ (APC with Kitty-style 'G' command).
    const INPUT: &[u8] = b"\x1b_Gf=32;AAAA\x1b\\";
    let mut dispatcher = Dispatcher::default();
    let mut parser = Parser::new();

    parser.advance(&mut dispatcher, INPUT);

    // ApcStart, then 'G','f','=','3','2',';','A','A','A','A' as ApcPut, then ApcEnd.
    assert_eq!(dispatcher.dispatched[0], Sequence::ApcStart);
    for (i, &byte) in b"Gf=32;AAAA".iter().enumerate() {
        assert_eq!(dispatcher.dispatched[1 + i], Sequence::ApcPut(byte));
    }
    assert_eq!(dispatcher.dispatched[11], Sequence::ApcEnd);
    // ESC \ triggers ApcEnd then esc_dispatch for '\'.
    assert_eq!(dispatcher.dispatched[12], Sequence::Esc(Vec::new(), false, b'\\'));
}

#[test]
fn parse_apc_c1_st_terminated() {
    // ESC _ payload 0x9C (C1 ST terminator).
    const INPUT: &[u8] = b"\x1b_hello\x9c";
    let mut dispatcher = Dispatcher::default();
    let mut parser = Parser::new();

    parser.advance(&mut dispatcher, INPUT);

    assert_eq!(dispatcher.dispatched[0], Sequence::ApcStart);
    for (i, &byte) in b"hello".iter().enumerate() {
        assert_eq!(dispatcher.dispatched[1 + i], Sequence::ApcPut(byte));
    }
    assert_eq!(dispatcher.dispatched[6], Sequence::ApcEnd);
    assert_eq!(dispatcher.dispatched.len(), 7);
}

#[test]
fn parse_apc_cancel() {
    // ESC _ payload CAN (0x18) — cancels APC.
    const INPUT: &[u8] = b"\x1b_data\x18";
    let mut dispatcher = Dispatcher::default();
    let mut parser = Parser::new();

    parser.advance(&mut dispatcher, INPUT);

    assert_eq!(dispatcher.dispatched[0], Sequence::ApcStart);
    for (i, &byte) in b"data".iter().enumerate() {
        assert_eq!(dispatcher.dispatched[1 + i], Sequence::ApcPut(byte));
    }
    assert_eq!(dispatcher.dispatched[5], Sequence::ApcEnd);
    assert_eq!(dispatcher.dispatched[6], Sequence::Execute(0x18));
}

#[test]
fn parse_apc_empty() {
    // ESC _ ESC \ (empty APC string).
    const INPUT: &[u8] = b"\x1b_\x1b\\";
    let mut dispatcher = Dispatcher::default();
    let mut parser = Parser::new();

    parser.advance(&mut dispatcher, INPUT);

    assert_eq!(dispatcher.dispatched[0], Sequence::ApcStart);
    assert_eq!(dispatcher.dispatched[1], Sequence::ApcEnd);
    assert_eq!(dispatcher.dispatched[2], Sequence::Esc(Vec::new(), false, b'\\'));
}

#[test]
fn sos_pm_still_discards() {
    // ESC X (SOS) and ESC ^ (PM) should still discard data (no APC callbacks).
    const SOS: &[u8] = b"\x1bXdata\x1b\\";
    const PM: &[u8] = b"\x1b^data\x1b\\";

    let mut dispatcher = Dispatcher::default();
    let mut parser = Parser::new();

    parser.advance(&mut dispatcher, SOS);
    // SOS goes to SosPmApcString which calls anywhere() — discards data.
    // ESC causes transition to Escape, then '\' dispatches.
    assert_eq!(dispatcher.dispatched.len(), 1);
    assert_eq!(dispatcher.dispatched[0], Sequence::Esc(Vec::new(), false, b'\\'));

    dispatcher.dispatched.clear();
    parser.advance(&mut dispatcher, PM);
    assert_eq!(dispatcher.dispatched.len(), 1);
    assert_eq!(dispatcher.dispatched[0], Sequence::Esc(Vec::new(), false, b'\\'));
}

#[test]
fn execute_anywhere() {
    const INPUT: &[u8] = b"\x18\x1a";

    let mut dispatcher = Dispatcher::default();
    let mut parser = Parser::new();

    parser.advance(&mut dispatcher, INPUT);

    assert_eq!(dispatcher.dispatched.len(), 2);
    assert_eq!(dispatcher.dispatched[0], Sequence::Execute(0x18));
    assert_eq!(dispatcher.dispatched[1], Sequence::Execute(0x1A));
}

// ── 8-bit C1 control detection ──────────────────────────────────────

/// 0x9B enters CSI state — `\x9b0m` dispatches SGR reset.
#[test]
fn c1_0x9b_enters_csi_state() {
    let mut d = Dispatcher::default();
    let mut p = Parser::new();
    // 0x9B is invalid UTF-8 lead byte, triggers C1 detection in error path.
    p.advance(&mut d, &[0x9B, b'0', b'm']);
    assert_eq!(d.dispatched.len(), 1);
    assert_eq!(d.dispatched[0], Sequence::Csi(vec![vec![0]], vec![], false, 'm'));
}

/// 0x90 enters DCS state — `\x90q...ST` dispatches DCS hook.
#[test]
fn c1_0x90_enters_dcs_state() {
    let mut d = Dispatcher::default();
    let mut p = Parser::new();
    // DCS q (sixel introducer) followed by ESC \ (ST).
    p.advance(&mut d, &[0x90, b'q', 0x1B, b'\\']);
    assert!(d.dispatched.iter().any(|s| matches!(s, Sequence::DcsHook(..))));
    assert!(d.dispatched.iter().any(|s| matches!(s, Sequence::DcsUnhook)));
}

/// 0x9D enters OSC state — `\x9d0;title\x07` dispatches OSC with title.
/// Uses BEL (0x07) as terminator because 0x9C conflicts with UTF-8
/// continuation bytes inside OSC payloads.
#[test]
fn c1_0x9d_enters_osc_state() {
    let mut d = Dispatcher::default();
    let mut p = Parser::new();
    p.advance(&mut d, &[0x9D, b'0', b';', b't', b'i', b't', b'l', b'e', 0x07]);
    assert_eq!(d.dispatched.len(), 1);
    match &d.dispatched[0] {
        Sequence::Osc(params, _) => {
            assert_eq!(params.len(), 2);
            assert_eq!(params[0], b"0");
            assert_eq!(params[1], b"title");
        },
        other => panic!("expected Osc, got {:?}", other),
    }
}

/// 0x9F enters APC state — `\x9f...\x9c` captures APC content.
#[test]
fn c1_0x9f_enters_apc_state() {
    let mut d = Dispatcher::default();
    let mut p = Parser::new();
    p.advance(&mut d, &[0x9F, b'h', b'i', 0x9C]);
    assert!(d.dispatched.iter().any(|s| matches!(s, Sequence::ApcStart)));
    assert!(d.dispatched.iter().any(|s| matches!(s, Sequence::ApcPut(b'h'))));
    assert!(d.dispatched.iter().any(|s| matches!(s, Sequence::ApcPut(b'i'))));
    assert!(d.dispatched.iter().any(|s| matches!(s, Sequence::ApcEnd)));
}

/// 0x98 enters SOS discard state — `\x98...\x9c` discards content.
#[test]
fn c1_0x98_enters_sos_discard_state() {
    let mut d = Dispatcher::default();
    let mut p = Parser::new();
    p.advance(&mut d, &[0x98, b'x', b'y', 0x1B, b'\\']);
    // SOS is discarded — no Osc, Csi, Dcs, Apc sequences dispatched.
    // The ESC \ terminates, dispatching esc_dispatch for '\'.
    for s in &d.dispatched {
        assert!(!matches!(s, Sequence::Osc(..) | Sequence::Csi(..) | Sequence::DcsHook(..)));
    }
}

/// 0x9E enters PM discard state — `\x9e...\x9c` discards content.
#[test]
fn c1_0x9e_enters_pm_discard_state() {
    let mut d = Dispatcher::default();
    let mut p = Parser::new();
    p.advance(&mut d, &[0x9E, b'z', 0x1B, b'\\']);
    for s in &d.dispatched {
        assert!(!matches!(s, Sequence::Osc(..) | Sequence::Csi(..) | Sequence::DcsHook(..)));
    }
}

/// 0x9C as ST terminates a DCS sequence mid-stream.
#[test]
fn c1_0x9c_terminates_dcs_sequence() {
    let mut d = Dispatcher::default();
    let mut p = Parser::new();
    // Start DCS via ESC P, then terminate with 8-bit ST (0x9C).
    p.advance(&mut d, &[0x1B, b'P', b'q', b'A', 0x9C]);
    assert!(d.dispatched.iter().any(|s| matches!(s, Sequence::DcsHook(..))));
    assert!(d.dispatched.iter().any(|s| matches!(s, Sequence::DcsUnhook)));
}

/// 0x9C does NOT terminate an OSC sequence — it is treated as data.
/// This is intentional: 0x9C is a valid UTF-8 continuation byte that
/// appears in CJK characters (e.g., '末' = E6 9C AB). Matching upstream
/// Alacritty VTE behavior.
#[test]
fn c1_0x9c_does_not_terminate_osc_sequence() {
    let mut d = Dispatcher::default();
    let mut p = Parser::new();
    // Start OSC via ESC ], then 0x9C. OSC should absorb 0x9C as data, not terminate.
    // Terminate with BEL to close the sequence.
    p.advance(&mut d, &[0x1B, b']', b'0', b';', b'x', 0x9C, 0x07]);
    assert_eq!(d.dispatched.len(), 1);
    match &d.dispatched[0] {
        Sequence::Osc(params, _) => {
            assert_eq!(params.len(), 2);
            // Payload includes the 0x9C byte as data.
            assert_eq!(params[1], &[b'x', 0x9C]);
        },
        other => panic!("expected Osc, got {:?}", other),
    }
}

/// 0x9C as ST terminates a SosPmApc string (via `anywhere` handler).
#[test]
fn c1_0x9c_terminates_sos_pm_string() {
    let mut d = Dispatcher::default();
    let mut p = Parser::new();
    // ESC X (SOS) then 8-bit ST (0x9C), then ESC [ 0 m (CSI SGR).
    // After ST, parser returns to ground and should process the CSI.
    p.advance(&mut d, &[0x1B, b'X', b'a', 0x9C, 0x1B, b'[', b'0', b'm']);
    assert!(d.dispatched.iter().any(|s| matches!(s, Sequence::Csi(..))));
}

// ── C1 ground-state matrix: each byte from ground ───────────────────

/// 0x9B from ground → CSI (duplicate of c1_0x9b_enters_csi_state for matrix).
#[test]
fn c1_matrix_ground_0x9b() {
    let mut d = Dispatcher::default();
    let mut p = Parser::new();
    p.advance(&mut d, &[0x9B, b'1', b'A']);
    assert_eq!(d.dispatched.len(), 1);
    assert_eq!(d.dispatched[0], Sequence::Csi(vec![vec![1]], vec![], false, 'A'));
}

/// 0x90 from ground → DCS.
#[test]
fn c1_matrix_ground_0x90() {
    let mut d = Dispatcher::default();
    let mut p = Parser::new();
    p.advance(&mut d, &[0x90, b'q', 0x1B, b'\\']);
    assert!(d.dispatched.iter().any(|s| matches!(s, Sequence::DcsHook(..))));
}

/// 0x9D from ground → OSC (terminated by BEL).
#[test]
fn c1_matrix_ground_0x9d() {
    let mut d = Dispatcher::default();
    let mut p = Parser::new();
    p.advance(&mut d, &[0x9D, b'0', b';', b'a', 0x07]);
    assert_eq!(d.dispatched.len(), 1);
    match &d.dispatched[0] {
        Sequence::Osc(params, bell) => {
            assert_eq!(params[0], b"0");
            assert_eq!(params[1], b"a");
            assert!(*bell);
        },
        other => panic!("expected Osc, got {:?}", other),
    }
}

/// 0x9F from ground → APC.
#[test]
fn c1_matrix_ground_0x9f() {
    let mut d = Dispatcher::default();
    let mut p = Parser::new();
    p.advance(&mut d, &[0x9F, b'z', 0x1B, b'\\']);
    assert!(d.dispatched.iter().any(|s| matches!(s, Sequence::ApcStart)));
    assert!(d.dispatched.iter().any(|s| matches!(s, Sequence::ApcEnd)));
}

/// 0x98 from ground → SOS (discard).
#[test]
fn c1_matrix_ground_0x98() {
    let mut d = Dispatcher::default();
    let mut p = Parser::new();
    p.advance(&mut d, &[0x98, b'x', 0x1B, b'\\']);
    for s in &d.dispatched {
        assert!(!matches!(s, Sequence::Osc(..) | Sequence::Csi(..) | Sequence::DcsHook(..)));
    }
}

/// 0x9E from ground → PM (discard).
#[test]
fn c1_matrix_ground_0x9e() {
    let mut d = Dispatcher::default();
    let mut p = Parser::new();
    p.advance(&mut d, &[0x9E, b'w', 0x1B, b'\\']);
    for s in &d.dispatched {
        assert!(!matches!(s, Sequence::Osc(..) | Sequence::Csi(..) | Sequence::DcsHook(..)));
    }
}

/// 0x9C from ground → no-op (ST on ground has nothing to terminate).
#[test]
fn c1_matrix_ground_0x9c() {
    let mut d = Dispatcher::default();
    let mut p = Parser::new();
    // 0x9C on ground should be a no-op, then the CSI should parse normally.
    p.advance(&mut d, &[0x9C, 0x1B, b'[', b'0', b'm']);
    assert_eq!(d.dispatched.len(), 1);
    assert!(matches!(&d.dispatched[0], Sequence::Csi(..)));
}

// ── C1 mid-sequence matrix: 0x9C as terminator ──────────────────────

/// 0x9C terminates DCS passthrough (already supported, matrix coverage).
#[test]
fn c1_matrix_midseq_0x9c_in_dcs() {
    let mut d = Dispatcher::default();
    let mut p = Parser::new();
    p.advance(&mut d, &[0x1B, b'P', b'q', 0x9C]);
    assert!(d.dispatched.iter().any(|s| matches!(s, Sequence::DcsUnhook)));
}

/// 0x9C terminates APC string (already supported, matrix coverage).
#[test]
fn c1_matrix_midseq_0x9c_in_apc() {
    let mut d = Dispatcher::default();
    let mut p = Parser::new();
    p.advance(&mut d, &[0x1B, b'_', b'x', 0x9C]);
    assert!(d.dispatched.iter().any(|s| matches!(s, Sequence::ApcEnd)));
}

/// 0x9C inside OSC is treated as data, not as ST (UTF-8 safety).
#[test]
fn c1_matrix_midseq_0x9c_in_osc_is_data() {
    let mut d = Dispatcher::default();
    let mut p = Parser::new();
    // 0x9C mid-OSC is kept as data, BEL terminates.
    p.advance(&mut d, &[0x1B, b']', b'0', b';', b't', 0x9C, 0x07]);
    match &d.dispatched[0] {
        Sequence::Osc(params, _) => assert_eq!(params[1], &[b't', 0x9C]),
        other => panic!("expected Osc, got {:?}", other),
    }
}

/// 0x9C terminates SOS string (gap being fixed via `anywhere`).
#[test]
fn c1_matrix_midseq_0x9c_in_sos() {
    let mut d = Dispatcher::default();
    let mut p = Parser::new();
    // ESC X (SOS), some data, 0x9C (ST), then verify ground state with a print.
    p.advance(&mut d, &[0x1B, b'X', b'z', 0x9C, b'A']);
    assert!(d.dispatched.iter().any(|s| matches!(s, Sequence::Print('A'))));
}

/// 0x9C terminates PM string (gap being fixed via `anywhere`).
#[test]
fn c1_matrix_midseq_0x9c_in_pm() {
    let mut d = Dispatcher::default();
    let mut p = Parser::new();
    p.advance(&mut d, &[0x1B, b'^', b'z', 0x9C, b'B']);
    assert!(d.dispatched.iter().any(|s| matches!(s, Sequence::Print('B'))));
}

// ── Regression guard: BSU/ESU 7-bit form not matched by 8-bit CSI ───────

/// 8-bit CSI `\x9b?2026h` must NOT trigger sync update (BSU matcher
/// expects 7-bit ESC [ form).
#[test]
fn bsu_esu_7bit_not_matched_by_8bit_csi() {
    let mut d = Dispatcher::default();
    let mut p = Parser::new();
    // Feed 8-bit CSI form of BSU: 0x9B ? 2 0 2 6 h.
    // This should parse as a CSI sequence with private marker '?',
    // param 2026, final 'h'. It should NOT be recognized as BSU.
    p.advance(&mut d, &[0x9B, b'?', b'2', b'0', b'2', b'6', b'h']);
    // If it were incorrectly recognized as BSU, the parser would enter
    // sync mode. Verify we got a normal CSI dispatch instead.
    assert_eq!(d.dispatched.len(), 1);
    assert!(matches!(&d.dispatched[0], Sequence::Csi(..)));
}

// ── Property: 8-bit CSI SGR reset only works with C1 support ────

/// `\x9b0m` resets SGR — this test ONLY passes when 8-bit C1 routing
/// correctly enters CSI state from ground via the 0x9B byte.
#[test]
fn c1_csi_sgr_reset_only_passes_with_8bit_support() {
    let mut d = Dispatcher::default();
    let mut p = Parser::new();
    p.advance(&mut d, &[0x9B, b'0', b'm']);
    assert_eq!(d.dispatched.len(), 1);
    // SGR 0 = param [0], no intermediates, final 'm'.
    assert_eq!(d.dispatched[0], Sequence::Csi(vec![vec![0]], vec![], false, 'm'));
}
