//! Exact macOS release-asset selection by CPU architecture. Unknown CPUs never
//! fall back to another architecture or rely on Rosetta implicitly.

/// Preserve the architecture-specific preference order while considering only
/// assets the caller has independently validated as usable.
pub(super) fn first_usable<'a>(names: &'a [String], mut usable: impl FnMut(&str) -> bool) -> Option<&'a str> {
    names.iter().map(String::as_str).find(|name| usable(name))
}

pub(super) fn macos_assets(kind: &str, arch: &str, tag: &str) -> Vec<String> {
    let version = tag.trim_start_matches('v');
    let mut names = match (kind, arch) {
        ("mihomo", "aarch64") => vec![format!("mihomo-darwin-arm64-{tag}.gz")],
        ("mihomo", "x86_64") => vec![format!("mihomo-darwin-amd64-{tag}.gz"), format!("mihomo-darwin-amd64-compatible-{tag}.gz")],
        ("xray", "aarch64") => vec!["Xray-macos-arm64-v8a.zip".into()],
        ("xray", "x86_64") => vec!["Xray-macos-64.zip".into()],
        ("sing_box", "aarch64") => vec![format!("sing-box-{version}-darwin-arm64.tar.gz")],
        ("sing_box", "x86_64") => vec![format!("sing-box-{version}-darwin-amd64.tar.gz")],
        ("shoes", "aarch64") => vec!["shoes-aarch64-apple-darwin.tar.gz".into()],
        ("shoes", "x86_64") => vec!["shoes-x86_64-apple-darwin.tar.gz".into()],
        ("meow", "aarch64") => vec![format!("meow-{tag}-aarch64-apple-darwin.tar.gz")],
        ("meow", "x86_64") => vec![format!("meow-{tag}-x86_64-apple-darwin.tar.gz")],
        ("vpcore", "aarch64") => vec![format!("vpcore-v{version}-aarch64-apple-darwin.tar.gz"),format!("vpcore-{tag}-aarch64-apple-darwin.tar.gz")],
        ("vpcore", "x86_64") => vec![format!("vpcore-v{version}-x86_64-apple-darwin.tar.gz"),format!("vpcore-{tag}-x86_64-apple-darwin.tar.gz")],
        _ => Vec::new(),
    };
    names.dedup(); names
}

/// Linux ARM64 names are kept separate from x86_64 assets.
pub(super) fn linux_arm64_assets(kind: &str, tag: &str) -> Vec<String> {
    let version=tag.trim_start_matches('v');
    let mut names=match kind {
        "mihomo" => vec![format!("mihomo-linux-arm64-{tag}.gz")],
        "xray" => vec!["Xray-linux-arm64-v8a.zip".into()],
        "sing_box" => vec![format!("sing-box-{version}-linux-arm64.tar.gz")],
        "shoes" => vec!["shoes-aarch64-unknown-linux-gnu.tar.gz".into(),"shoes-aarch64-unknown-linux-musl.tar.gz".into()],
        "meow" => vec![format!("meow-{tag}-aarch64-unknown-linux-gnu.tar.gz"),format!("meow-{tag}-aarch64-unknown-linux-musl.tar.gz")],
        "vpcore" => vec![format!("vpcore-v{version}-aarch64-unknown-linux-musl.tar.gz"),format!("vpcore-{tag}-aarch64-unknown-linux-musl.tar.gz")],
        _ => Vec::new(),
    };
    names.dedup(); names
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn selection_uses_only_usable_candidates_in_preference_order() {
        let names = vec!["preferred-arm64".to_owned(), "alternate-arm64".to_owned()];
        assert_eq!(first_usable(&names, |name| name == "alternate-arm64"), Some("alternate-arm64"));
        assert_eq!(first_usable(&names, |_| true), Some("preferred-arm64"));
        assert_eq!(first_usable(&names, |_| false), None);
        assert_eq!(first_usable(&[], |_| true), None);
    }
    #[test] fn apple_silicon_assets_are_native_and_never_intel() {
        for kind in ["mihomo","xray","sing_box","shoes","meow","vpcore"] {
            let values=macos_assets(kind,"aarch64","v1.2.3");assert!(!values.is_empty());
            assert!(values.iter().all(|s| !s.contains("amd64") && !s.contains("x86_64") && !s.contains("macos-64")));
        }
        assert_eq!(macos_assets("xray","aarch64","v1.2.3"),["Xray-macos-arm64-v8a.zip"]);
        assert_eq!(macos_assets("mihomo","aarch64","v1.2.3"),["mihomo-darwin-arm64-v1.2.3.gz"]);
    }
    #[test] fn intel_assets_never_select_apple_silicon() {
        for kind in ["mihomo","xray","sing_box","shoes","meow","vpcore"] {
            let values=macos_assets(kind,"x86_64","v1.2.3");assert!(!values.is_empty());
            assert!(values.iter().all(|s| !s.contains("arm64") && !s.contains("aarch64")));
        }
    }
    #[test] fn linux_arm_assets_never_select_x86() {
        for kind in ["mihomo","xray","sing_box","shoes","meow","vpcore"] {
            let names=linux_arm64_assets(kind,"v1.2.3");assert!(!names.is_empty());
            assert!(names.iter().all(|v|!v.contains("amd64")&&!v.contains("x86_64")));
        }
        assert!(linux_arm64_assets("unknown","v1.2.3").is_empty());
    }
    #[test] fn unknown_architecture_or_engine_has_no_fallback() {
        assert!(macos_assets("mihomo","riscv64","v1.2.3").is_empty());
        assert!(macos_assets("unknown","aarch64","v1.2.3").is_empty());
    }
    #[test] fn vpcore_tag_variants_do_not_duplicate_or_cross_architecture() {
        assert_eq!(macos_assets("vpcore","aarch64","v1.2.3"),["vpcore-v1.2.3-aarch64-apple-darwin.tar.gz"]);
        assert!(macos_assets("vpcore","x86_64","1.2.3").iter().all(|s|s.contains("x86_64")));
    }
}
