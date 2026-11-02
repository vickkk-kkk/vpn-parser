//! Geo category checks over explicitly supplied data; no path selection or I/O.
//!
//! This is the upstream best-effort category scanner, not a new protobuf decoder.
//! Unavailable input remains distinguishable from a successfully scanned absence.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GeoKind {
    Site,
    Ip,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GeoUnavailable {
    Missing,
    Unreadable,
    Malformed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CategoryLookup {
    Present,
    Absent,
    Unavailable(GeoUnavailable),
}

impl CategoryLookup {
    /// Preserve upstream defer-to-engine policy when the inventory is unavailable.
    pub fn allows_legacy(self) -> bool {
        !matches!(self, Self::Absent)
    }
}

pub trait GeoProvider {
    fn lookup(&self, kind: GeoKind, code: &str) -> CategoryLookup;
}

/// Native adapters supply actual asset/file bytes or the reason acquisition failed.
/// There is deliberately no default or empty-inventory fallback.
pub struct BytesGeoProvider<'a> {
    pub site: Result<&'a [u8], GeoUnavailable>,
    pub ip: Result<&'a [u8], GeoUnavailable>,
}

impl GeoProvider for BytesGeoProvider<'_> {
    fn lookup(&self, kind: GeoKind, code: &str) -> CategoryLookup {
        let normalized = normalize_geo_code(code);
        if kind == GeoKind::Ip
            && normalized.len() == 2
            && normalized.chars().all(|c| c.is_ascii_alphabetic())
        {
            return CategoryLookup::Present;
        }
        let data = match kind {
            GeoKind::Site => self.site,
            GeoKind::Ip => self.ip,
        };
        match data {
            Ok(bytes) => dat_category_lookup(bytes, normalized),
            Err(reason) => CategoryLookup::Unavailable(reason),
        }
    }
}

pub fn normalize_geo_code(code: &str) -> &str {
    if code.eq_ignore_ascii_case("lan") {
        "private"
    } else {
        code
    }
}

/// Case-insensitive lookup using the original best-effort scanner.
/// Structural conditions that previously returned true now report unavailable;
/// legacy callers still permit them via `allows_legacy`.
pub fn dat_category_lookup(data: &[u8], code: &str) -> CategoryLookup {
    let code = code.as_bytes();
    let mut pos = 0usize;
    while pos < data.len() {
        // 顶层是重复的 Category 消息（field 1, length-delimited）；
        // 出现别的 tag 说明结构对不上，交给内核裁决。
        if data[pos] != 0x0a {
            return CategoryLookup::Unavailable(GeoUnavailable::Malformed);
        }
        pos += 1;
        let mut length = 0usize;
        let mut shift = 0u32;
        loop {
            let Some(&b) = data.get(pos) else {
                return CategoryLookup::Unavailable(GeoUnavailable::Malformed);
            };
            pos += 1;
            length |= ((b & 0x7f) as usize) << shift;
            if b & 0x80 == 0 {
                break;
            }
            shift += 7;
            if shift > 28 {
                return CategoryLookup::Unavailable(GeoUnavailable::Malformed);
            }
        }
        let Some(chunk) = data.get(pos..pos + length) else {
            return CategoryLookup::Unavailable(GeoUnavailable::Malformed);
        };
        pos += length;
        // Category 内部 field 1（0x0a）是类别码字符串
        if chunk.first() == Some(&0x0a) {
            if let Some(&code_len) = chunk.get(1) {
                if let Some(found) = chunk.get(2..2 + code_len as usize) {
                    if found.eq_ignore_ascii_case(code) {
                        return CategoryLookup::Present;
                    }
                }
            }
        }
    }
    CategoryLookup::Absent
}

#[cfg(test)]
mod tests {
    use super::*;

    fn build_dat(codes: &[&str]) -> Vec<u8> {
        let mut out = Vec::new();
        for code in codes {
            let inner_len = 2 + code.len();
            out.push(0x0a);
            out.push(inner_len as u8);
            out.push(0x0a);
            out.push(code.len() as u8);
            out.extend_from_slice(code.as_bytes());
        }
        out
    }

    #[test]
    fn original_scanner_case_insensitivity_and_absence() {
        let bytes = build_dat(&["cn", "private", "category-ads-all"]);
        for category in ["cn", "CN", "CATEGORY-ADS-ALL"] {
            assert_eq!(
                dat_category_lookup(&bytes, category),
                CategoryLookup::Present
            );
        }
        for category in ["lan", "LAN", "missing"] {
            assert_eq!(
                dat_category_lookup(&bytes, category),
                CategoryLookup::Absent
            );
        }
        // Empty supplied bytes are an actual scanned absence, not missing data.
        assert_eq!(dat_category_lookup(&[], "private"), CategoryLookup::Absent);
    }

    #[test]
    fn corruption_is_explicit_but_preserves_leniency() {
        for bytes in [
            &[0x08, 0x01, 0x02][..],
            &[0x0a][..],
            &[0x0a, 20, 0x0a][..],
            &[0x0a, 0x80, 0x80, 0x80, 0x80, 0x80][..],
        ] {
            let result = dat_category_lookup(bytes, "cn");
            assert_eq!(
                result,
                CategoryLookup::Unavailable(GeoUnavailable::Malformed)
            );
            assert!(result.allows_legacy());
        }
        // Preserve the original scanner's early-match behavior, even if later bytes
        // are corrupt, and its treatment of unrecognized category contents.
        let mut found = build_dat(&["cn"]);
        found.push(0x08);
        assert_eq!(dat_category_lookup(&found, "cn"), CategoryLookup::Present);
        assert_eq!(
            dat_category_lookup(&[0x0a, 1, 0x08], "cn"),
            CategoryLookup::Absent
        );
    }

    #[test]
    fn supplied_inventory_normalizes_lan_and_preserves_geoip_bypass() {
        let bytes = build_dat(&["private"]);
        let provider = BytesGeoProvider {
            site: Ok(&bytes),
            ip: Ok(&bytes),
        };
        assert_eq!(
            provider.lookup(GeoKind::Site, "LAN"),
            CategoryLookup::Present
        );
        assert_eq!(provider.lookup(GeoKind::Ip, "lan"), CategoryLookup::Present);
        assert_eq!(provider.lookup(GeoKind::Site, "cn"), CategoryLookup::Absent);
        assert_eq!(provider.lookup(GeoKind::Ip, "cn"), CategoryLookup::Present);
        assert!(!provider.lookup(GeoKind::Site, "cn").allows_legacy());
    }

    #[test]
    fn acquisition_failures_do_not_become_empty_inventory() {
        let provider = BytesGeoProvider {
            site: Err(GeoUnavailable::Missing),
            ip: Err(GeoUnavailable::Unreadable),
        };
        assert_eq!(
            provider.lookup(GeoKind::Site, "cn"),
            CategoryLookup::Unavailable(GeoUnavailable::Missing)
        );
        assert_eq!(
            provider.lookup(GeoKind::Ip, "LAN"),
            CategoryLookup::Unavailable(GeoUnavailable::Unreadable)
        );
        assert_eq!(provider.lookup(GeoKind::Ip, "ZZ"), CategoryLookup::Present);
        assert!(provider.lookup(GeoKind::Site, "cn").allows_legacy());
        assert!(provider.lookup(GeoKind::Ip, "LAN").allows_legacy());
    }
}
