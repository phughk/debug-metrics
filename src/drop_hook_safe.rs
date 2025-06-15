use crate::debug_metrics_safe::DebugMetricsSafeTrait;

pub struct DropHookSafe<DM, CallFn>
where
    DM: DebugMetricsSafeTrait,
    CallFn: Fn(&DM),
{
    pub(crate) debug_metrics: DM,
    pub(crate) call_fn: CallFn,
}

impl<DM, CallFn> Drop for DropHookSafe<DM, CallFn>
where
    DM: DebugMetricsSafeTrait,
    CallFn: Fn(&DM),
{
    fn drop(&mut self) {
        (self.call_fn)(&self.debug_metrics);
    }
}

impl<DM, CallFn> DropHookSafe<DM, CallFn>
where
    DM: DebugMetricsSafeTrait,
    CallFn: Fn(&DM),
{
    pub fn new(debug_metrics: DM, call_fn: CallFn) -> Self {
        DropHookSafe {
            debug_metrics,
            call_fn,
        }
    }
}
