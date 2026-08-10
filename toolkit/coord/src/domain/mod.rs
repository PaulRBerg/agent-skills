mod types;

use crate::error::Result;

pub(crate) use types::*;

pub(crate) trait ProcessProbe: Send + Sync {
    fn fingerprint(&self, pid: u32) -> Result<ProcessFingerprint>;
    fn liveness(&self, fingerprint: &ProcessFingerprint) -> ProcessLiveness;
}
