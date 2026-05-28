//! DEC private CSI extension `Handler` trait method declarations (Section 09A).
//!
//! Defines `handler_dec_private_methods!()` — a `macro_rules!` that expands
//! to the Section 09A DEC private trait methods:
//!
//! - Rectangular-area ops: DECSACE, DECCARA, DECRARA, DECCRA, DECFRA,
//!   XTCHECKSUM, DECRQCRA, DECERA, DECSERA, XTREPORTSGR.
//! - Presentation ops: DECRQPSR, DECRQUPSS, DECRQDE, DECSCL, DECSCA,
//!   DECSASD, DECSSDT, DECIC, DECDC, DECBI, DECFI.

macro_rules! handler_dec_private_methods {
    () => {
        // ── DEC private rectangular ops (Section 09A) ───────────────────────

        /// DECSACE — Select Attribute Change Extent (CSI Ps * x).
        fn decsace(&mut self, _mode: u16) {}

        /// DECCARA — Change Attributes in Rectangular Area
        /// (CSI Pt;Pl;Pb;Pr;Pm $ r).
        fn deccara(&mut self, _top: u16, _left: u16, _bot: u16, _right: u16, _attrs: &[u16]) {}

        /// DECRARA — Reverse Attributes in Rectangular Area
        /// (CSI Pt;Pl;Pb;Pr;Pm $ t).
        fn decrara(&mut self, _top: u16, _left: u16, _bot: u16, _right: u16, _attrs: &[u16]) {}

        /// DECCRA — Copy Rectangular Area
        /// (CSI Pts;Pls;Pbs;Prs;Pps;Ptd;Pld;Ppd $ v).
        #[expect(
            clippy::too_many_arguments,
            reason = "DECCRA spec encodes 8 distinct coordinates (source rect + dest rect + page nums); collapsing would lose direct param-to-spec mapping"
        )]
        fn deccra(
            &mut self,
            _src_top: u16,
            _src_left: u16,
            _src_bot: u16,
            _src_right: u16,
            _src_page: u16,
            _dst_top: u16,
            _dst_left: u16,
            _dst_page: u16,
        ) {
        }

        /// DECFRA — Fill Rectangular Area (CSI Pc;Pt;Pl;Pb;Pr $ x).
        fn decfra(&mut self, _ch: u16, _top: u16, _left: u16, _bot: u16, _right: u16) {}

        /// XTCHECKSUM — Set checksum-extension flags (CSI Ps # y).
        fn xtchecksum(&mut self, _flags: u16) {}

        /// DECRQCRA — Request Checksum of Rectangular Area
        /// (CSI Pi;Pg;Pt;Pl;Pb;Pr * y).
        fn decrqcra(
            &mut self,
            _id: u16,
            _page: u16,
            _top: u16,
            _left: u16,
            _bot: u16,
            _right: u16,
        ) {
        }

        /// DECERA — Erase Rectangular Area (CSI Pt;Pl;Pb;Pr $ z).
        fn decera(&mut self, _top: u16, _left: u16, _bot: u16, _right: u16) {}

        /// DECSERA — Selective Erase Rectangular Area (CSI Pt;Pl;Pb;Pr $ {).
        fn decsera(&mut self, _top: u16, _left: u16, _bot: u16, _right: u16) {}

        /// XTREPORTSGR — Report SGR attributes of Rectangular Area
        /// (CSI Pt;Pl;Pb;Pr # |).
        fn xtreportsgr(&mut self, _top: u16, _left: u16, _bot: u16, _right: u16) {}

        // ── DEC private presentation ops (Section 09A) ───────────────────────

        /// DECRQPSR — Request Presentation State Report (CSI Ps $ w).
        fn decrqpsr(&mut self, _mode: u16) {}

        /// DECRQUPSS — Request User-Preferred Supplemental Set (CSI & u).
        fn decrqupss(&mut self) {}

        /// DECRQDE — Request Displayed Extent (CSI " v).
        fn decrqde(&mut self) {}

        /// DECSCL — Set Conformance Level (CSI Pl;Pc " p).
        fn decscl(&mut self, _level: u16, _c1_mode: u16) {}

        /// DECSCA — Select Character Protection Attribute (CSI Ps " q).
        fn decsca(&mut self, _protected: u16) {}

        /// DECSASD — Select Active Status Display (CSI Ps $ }).
        fn decsasd(&mut self, _target: u16) {}

        /// DECSSDT — Select Status Display Type (CSI Ps $ ~).
        fn decssdt(&mut self, _line_type: u16) {}

        /// DECIC — Insert Column (CSI Ps ' }).
        fn decic(&mut self, _count: u16) {}

        /// DECDC — Delete Column (CSI Ps ' ~).
        fn decdc(&mut self, _count: u16) {}

        /// DECEFR — Enable Filter Rectangle (CSI Pt;Pl;Pb;Pr ' w).
        /// Defines the coordinates of a filter rectangle for the DEC
        /// Locator subsystem. NOT gated by DECSET 1001 (which is highlight
        /// tracking, a separate protocol per the F1 cure); the DEC Locator
        /// is independently activated by DECELR.
        fn decefr(&mut self, _pt: u16, _pl: u16, _pb: u16, _pr: u16) {}

        /// DECELR — Enable Locator Reporting (CSI Ps;Pu ' z).
        /// Ps: 0 = disabled, 1 = continuous, 2 = one-report-then-disabled.
        /// Pu: 0 or 2 = character cells, 1 = pixels.
        fn decelr(&mut self, _ps: u16, _pu: u16) {}

        /// DECSLE — Select Locator Events (CSI Pm ' {).
        /// Pm: bitmask of event classes the locator should report.
        fn decsle(&mut self, _events: &[u16]) {}

        /// DECRQLP — Request Locator Position (CSI Ps ' |).
        /// Ps: 0, 1, or omitted = transmit a single DECLRP locator report.
        fn decrqlp(&mut self, _ps: u16) {}

        /// DECBI — Back Index (ESC 6). VT420 and up.
        fn decbi(&mut self) {}

        /// DECFI — Forward Index (ESC 9). VT420 and up.
        fn decfi(&mut self) {}
    };
}

pub(crate) use handler_dec_private_methods;
