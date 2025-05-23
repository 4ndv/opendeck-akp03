use mirajazz::types::{ImageFormat, ImageMirroring, ImageMode, ImageRotation};

// Must be unique between all the plugins, 2 characters long and match `DeviceNamespace` field in `manifest.json`
pub const DEVICE_NAMESPACE: &str = "n3";

pub const ROW_COUNT: usize = 3;
pub const COL_COUNT: usize = 3;
pub const KEY_COUNT: usize = 9;
pub const ENCODER_COUNT: usize = 3;

pub const IMAGE_FORMAT: ImageFormat = ImageFormat {
    mode: ImageMode::JPEG,
    size: (60, 60),
    rotation: ImageRotation::Rot0,
    mirror: ImageMirroring::None,
};

#[derive(Debug, Clone)]
pub enum Kind {
    AKP03,
    AKP03E,
    AKP03R,
    N3EN,
}

pub const AJAZZ_VID: u16 = 0x0300;
pub const MIRABOX_VID: u16 = 0x6603;

pub const AKP03_PID: u16 = 0x1001;
pub const AKP03E_PID: u16 = 0x3002; // Not sure if it's rev 1 or 2, so for now map as rev 1
pub const AKP03R_PID: u16 = 0x1003;

pub const N3EN_PID: u16 = 0x1003;

impl Kind {
    /// Matches devices VID+PID pairs to correct kinds
    pub fn from_vid_pid(vid: u16, pid: u16) -> Option<Self> {
        match vid {
            AJAZZ_VID => match pid {
                AKP03_PID => Some(Kind::AKP03),
                AKP03E_PID => Some(Kind::AKP03E),
                AKP03R_PID => Some(Kind::AKP03R),
                _ => None,
            },

            MIRABOX_VID => match pid {
                N3EN_PID => Some(Kind::N3EN),
                _ => None,
            },

            _ => None,
        }
    }

    /// Returns true for devices that emitting two events per key press, instead of one
    /// Currently only one device does that
    pub fn supports_both_states(&self) -> bool {
        match &self {
            Self::N3EN => true,
            _ => false,
        }
    }

    /// There is no point relying on manufacturer/device names reported by the USB stack,
    /// so we return custom names for all the kinds of devices
    pub fn human_name(&self) -> String {
        match &self {
            Self::AKP03 => "Ajazz AKP03",
            Self::AKP03E => "Ajazz AKP03E",
            Self::AKP03R => "Ajazz AKP03R",
            Self::N3EN => "Mirabox N3EN",
        }
        .to_string()
    }
}

#[derive(Debug)]
pub struct CandidateDevice {
    pub id: String,
    pub vid: u16,
    pub pid: u16,
    pub serial: String,
    pub kind: Kind,
}
