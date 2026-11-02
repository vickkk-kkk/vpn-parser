//! Development-fork distribution policy, separate from upstream source syncing.
//! No release channel is approved for this fork. Do not infer one from version tags.

pub const BINARY_UPDATES_ENABLED: bool = false;
pub const DISABLED_REASON: &str = "此 fork 开发版本已禁用应用二进制更新；请通过明确审查的源码同步和构建更新。内核更新不受影响。";

pub fn ensure_binary_updates_enabled() -> Result<(), &'static str> {
    if BINARY_UPDATES_ENABLED {
        Ok(())
    } else {
        Err(DISABLED_REASON)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn development_fork_has_no_binary_update_channel() {
        assert_eq!(ensure_binary_updates_enabled(), Err(DISABLED_REASON));
        assert!(!DISABLED_REASON.is_empty());
    }
}
