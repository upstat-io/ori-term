//! `impl crate::Perform for ObservedPerformer` — records raw actions via
//! `PerformObserver` then delegates to the canonical shared dispatch
//! functions (no algorithmic duplication).

use crate::Params;

use super::super::handler::Handler;
use super::super::observer::{PerformAction, PerformObserver};
use super::super::processor::{ObservedPerformer, Timeout};

impl<'a, H, T, O> crate::Perform for ObservedPerformer<'a, H, T, O>
where
    H: Handler + 'a,
    T: Timeout,
    O: PerformObserver,
{
    #[inline]
    fn print(&mut self, c: char) {
        self.observer.observe(&PerformAction::Print { c });
        super::dispatch_print(self.state, self.handler, c);
    }

    #[inline]
    fn execute(&mut self, byte: u8) {
        self.observer.observe(&PerformAction::Execute { byte });
        super::dispatch_execute(self.handler, byte);
    }

    #[inline]
    fn hook(&mut self, params: &Params, intermediates: &[u8], ignore: bool, action: char) {
        self.observer.observe(&PerformAction::Hook {
            params: params.iter().map(|s| s.to_vec()).collect(),
            intermediates: intermediates.to_vec(),
            ignore,
            action,
        });
        super::dispatch_hook(self.state, self.handler, params, intermediates, ignore, action);
    }

    #[inline]
    fn put(&mut self, byte: u8) {
        self.observer.observe(&PerformAction::Put { byte });
        super::dispatch_put(self.state, self.handler, byte);
    }

    #[inline]
    fn unhook(&mut self) {
        self.observer.observe(&PerformAction::Unhook);
        super::dispatch_unhook(self.state, self.handler);
    }

    #[inline]
    fn apc_start(&mut self) {
        self.observer.observe(&PerformAction::ApcStart);
        super::dispatch_apc_start(self.state);
    }

    #[inline]
    fn apc_put(&mut self, byte: u8) {
        self.observer.observe(&PerformAction::ApcPut { byte });
        super::dispatch_apc_put(self.state, byte);
    }

    #[inline]
    fn apc_end(&mut self) {
        self.observer.observe(&PerformAction::ApcEnd);
        super::dispatch_apc_end(self.state, self.handler);
    }

    #[inline]
    fn osc_dispatch(&mut self, params: &[&[u8]], bell_terminated: bool) {
        self.observer.observe(&PerformAction::OscDispatch {
            params: params.iter().map(|p| p.to_vec()).collect(),
            bell_terminated,
        });
        super::osc::dispatch(&mut *self.handler, params, bell_terminated);
    }

    #[allow(clippy::cognitive_complexity)]
    #[inline]
    fn csi_dispatch(
        &mut self,
        params: &Params,
        intermediates: &[u8],
        has_ignored_intermediates: bool,
        action: char,
    ) {
        self.observer.observe(&PerformAction::CsiDispatch {
            params: params.iter().map(|s| s.to_vec()).collect(),
            intermediates: intermediates.to_vec(),
            ignore: has_ignored_intermediates,
            action,
        });
        super::csi::dispatch(
            &mut *self.handler,
            &mut self.state.preceding_char,
            &mut self.state.sync_state.timeout,
            &mut self.terminated,
            params,
            intermediates,
            has_ignored_intermediates,
            action,
        );
    }

    #[inline]
    fn esc_dispatch(&mut self, intermediates: &[u8], _ignore: bool, byte: u8) {
        self.observer.observe(&PerformAction::EscDispatch {
            intermediates: intermediates.to_vec(),
            ignore: _ignore,
            byte,
        });
        super::dispatch_esc(self.handler, intermediates, byte);
    }

    #[inline]
    fn terminated(&self) -> bool {
        self.terminated
    }
}
