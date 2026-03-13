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
    const INPUT: &[u8] = b"\x00\x1f\x80\x90\x98\x9b\x9c\x9d\x9e\x9fa";

    let mut dispatcher = Dispatcher::default();
    let mut parser = Parser::new();

    parser.advance(&mut dispatcher, INPUT);

    assert_eq!(dispatcher.dispatched.len(), 11);
    assert_eq!(dispatcher.dispatched[0], Sequence::Execute(0));
    assert_eq!(dispatcher.dispatched[1], Sequence::Execute(31));
    assert_eq!(dispatcher.dispatched[2], Sequence::Execute(128));
    assert_eq!(dispatcher.dispatched[3], Sequence::Execute(144));
    assert_eq!(dispatcher.dispatched[4], Sequence::Execute(152));
    assert_eq!(dispatcher.dispatched[5], Sequence::Execute(155));
    assert_eq!(dispatcher.dispatched[6], Sequence::Execute(156));
    assert_eq!(dispatcher.dispatched[7], Sequence::Execute(157));
    assert_eq!(dispatcher.dispatched[8], Sequence::Execute(158));
    assert_eq!(dispatcher.dispatched[9], Sequence::Execute(159));
    assert_eq!(dispatcher.dispatched[10], Sequence::Print('a'));
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
