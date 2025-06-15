use crate::debug_metrics::DebugMetricsTrait;

pub struct DropHook<'a, DM, CallFn>
where
    DM: DebugMetricsTrait + ?Sized,
    CallFn: Fn(&mut DM),
{
    pub(crate) debug_metrics: &'a mut DM,
    pub(crate) call_fn: CallFn,
}

impl<DM, CallFn> Drop for DropHook<'_, DM, CallFn>
where
    DM: DebugMetricsTrait + ?Sized,
    CallFn: Fn(&mut DM),
{
    fn drop(&mut self) {
        (self.call_fn)(self.debug_metrics);
    }
}

impl<'a, DM, CallFn> DropHook<'a, DM, CallFn>
where
    DM: DebugMetricsTrait + ?Sized,
    CallFn: Fn(&mut DM),
{
    pub fn new(debug_metrics: &'a mut DM, call_fn: CallFn) -> Self {
        DropHook {
            debug_metrics,
            call_fn,
        }
    }
}
