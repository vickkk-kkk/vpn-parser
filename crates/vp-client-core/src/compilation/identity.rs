use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KernelKind {
    Mihomo,
    Xray,
    SingBox,
    Shoes,
    Meow,
    VpCore,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum KernelPreference {
    #[default]
    Auto,
    Mihomo,
    SingBox,
    Xray,
    Shoes,
    Meow,
    VpCore,
}

impl KernelPreference {
    pub fn kernel(self) -> Option<KernelKind> {
        match self {
            Self::Auto => None,
            Self::Mihomo => Some(KernelKind::Mihomo),
            Self::SingBox => Some(KernelKind::SingBox),
            Self::Xray => Some(KernelKind::Xray),
            Self::Shoes => Some(KernelKind::Shoes),
            Self::Meow => Some(KernelKind::Meow),
            Self::VpCore => Some(KernelKind::VpCore),
        }
    }
}

impl KernelKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Mihomo => "mihomo",
            Self::Xray => "xray",
            Self::SingBox => "sing_box",
            Self::Shoes => "shoes",
            Self::Meow => "meow",
            Self::VpCore => "vpcore",
        }
    }

    pub fn display_name(self) -> &'static str {
        match self {
            Self::Mihomo => "Mihomo",
            Self::Xray => "Xray",
            Self::SingBox => "Sing-box",
            Self::Shoes => "Shoes",
            Self::Meow => "Meow",
            Self::VpCore => "VP-Core",
        }
    }
}
