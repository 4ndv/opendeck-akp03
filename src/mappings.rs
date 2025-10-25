use mirajazz::{
    device::DeviceQuery,
    types::{HidDeviceInfo, ImageFormat, ImageMirroring, ImageMode, ImageRotation},
};

// Must be unique between all the plugins, 2 characters long and match `DeviceNamespace` field in `manifest.json`
pub const DEVICE_NAMESPACE: &str = "n3";

pub const ROW_COUNT: usize = 3;
pub const COL_COUNT: usize = 3;
pub const KEY_COUNT: usize = 9;
pub const ENCODER_COUNT: usize = 3;

#[derive(Debug, Clone)]
pub enum Kind {
    Akp03,
    Akp03E,
    Akp03R,
    Akp03Erev2,
    N3,
    N3EN,
    SoomfonSE,
    MSDTWO,
    TreasLinN3,
    RedragonSS551,
}

pub const AJAZZ_VID: u16 = 0x0300;
pub const MIRABOX_VID: u16 = 0x6603;
pub const N3_VID: u16 = 0x6602;
pub const SOOMFON_VID: u16 = 0x1500;
pub const MARS_GAMING_VID: u16 = 0x0B00;
pub const TREASLIN_VID: u16 = 0x5548;
pub const REDRAGON_VID: u16 = 0x0200;

pub const AKP03_PID: u16 = 0x1001;
pub const AKP03E_PID: u16 = 0x1002;
pub const AKP03R_PID: u16 = 0x1003;
pub const AKP03E_REV2_PID: u16 = 0x3002;
pub const N3_PID: u16 = 0x1002;
pub const N3EN_PID: u16 = 0x1003;
pub const SOOMFON_SE_PID: u16 = 0x3001;
pub const MSD_TWO_PID: u16 = 0x1001;
pub const TREASLIN_N3_PID: u16 = 0x1001;
pub const REDRAGON_SS551_PID: u16 = 0x2000;

// Map all queries to usage page 65440 and usage id 1 for now
pub const AKP03_QUERY: DeviceQuery = DeviceQuery::new(65440, 1, AJAZZ_VID, AKP03_PID);
pub const AKP03E_QUERY: DeviceQuery = DeviceQuery::new(65440, 1, AJAZZ_VID, AKP03E_PID);
pub const AKP03R_QUERY: DeviceQuery = DeviceQuery::new(65440, 1, AJAZZ_VID, AKP03R_PID);
pub const AKP03E_REV2_QUERY: DeviceQuery = DeviceQuery::new(65440, 1, AJAZZ_VID, AKP03E_REV2_PID);
pub const N3_QUERY: DeviceQuery = DeviceQuery::new(65440, 1, N3_VID, N3_PID);
pub const N3EN_QUERY: DeviceQuery = DeviceQuery::new(65440, 1, MIRABOX_VID, N3EN_PID);
pub const SOOMFON_SE_QUERY: DeviceQuery = DeviceQuery::new(65440, 1, SOOMFON_VID, SOOMFON_SE_PID);
pub const MSD_TWO_QUERY: DeviceQuery = DeviceQuery::new(65440, 1, MARS_GAMING_VID, MSD_TWO_PID);
pub const TREASLIN_QUERY: DeviceQuery = DeviceQuery::new(65440, 1, TREASLIN_VID, TREASLIN_N3_PID);
pub const REDRAGON_SS551_QUERY: DeviceQuery = DeviceQuery::new(65440, 1, REDRAGON_VID, REDRAGON_SS551_PID);

pub const QUERIES: [DeviceQuery; 10] = [
    AKP03_QUERY,
    AKP03E_QUERY,
    AKP03R_QUERY,
    AKP03E_REV2_QUERY,
    N3_QUERY,
    N3EN_QUERY,
    SOOMFON_SE_QUERY,
    MSD_TWO_QUERY,
    TREASLIN_QUERY,
    REDRAGON_SS551_QUERY,
];

impl Kind {
    /// Matches devices VID+PID pairs to correct kinds
    pub fn from_vid_pid(vid: u16, pid: u16) -> Option<Self> {
        match vid {
            AJAZZ_VID => match pid {
                AKP03_PID => Some(Kind::Akp03),
                AKP03E_PID => Some(Kind::Akp03E),
                AKP03R_PID => Some(Kind::Akp03R),
                AKP03E_REV2_PID => Some(Kind::Akp03Erev2),
                _ => None,
            },

            N3_VID => match pid {
                N3_PID => Some(Kind::N3),
                _ => None,
            },

            SOOMFON_VID => match pid {
                SOOMFON_SE_PID => Some(Kind::SoomfonSE),
                _ => None,
            },

            MIRABOX_VID => match pid {
                N3EN_PID => Some(Kind::N3EN),
                _ => None,
            },

            MARS_GAMING_VID => match pid {
                MSD_TWO_PID => Some(Kind::MSDTWO),
                _ => None,
            },

            TREASLIN_VID => match pid {
                TREASLIN_N3_PID => Some(Kind::TreasLinN3),
                _ => None,
            },

            REDRAGON_VID => match pid {
                REDRAGON_SS551_PID => Some(Kind::RedragonSS551),
                _ => None,
            },

            _ => None,
        }
    }

    /// There is no point relying on manufacturer/device names reported by the USB stack,
    /// so we return custom names for all the kinds of devices
    pub fn human_name(&self) -> String {
        match &self {
            Self::Akp03 => "Ajazz AKP03",
            Self::Akp03E => "Ajazz AKP03E",
            Self::Akp03R => "Ajazz AKP03R",
            Self::Akp03Erev2 => "Ajazz AKP03E (rev. 2)",
            Self::N3 => "Mirabox N3",
            Self::N3EN => "Mirabox N3EN",
            Self::SoomfonSE => "Soomfon Stream Controller SE",
            Self::MSDTWO => "Mars Gaming MSD-TWO",
            Self::TreasLinN3 => "TreasLin N3",
            Self::RedragonSS551 => "Redragon Skyrider SS-551"
        }
        .to_string()
    }

    /// Returns protocol version for device
    pub fn protocol_version(&self) -> usize {
        match self {
            Self::N3EN => 3,
            Self::Akp03Erev2 => 3,
            Self::SoomfonSE => 3,
            Self::TreasLinN3 => 3,
            Self::RedragonSS551 => 3,
            _ => 2,
        }
    }

    pub fn image_format(&self) -> ImageFormat {
        if self.protocol_version() == 3 {
            return ImageFormat {
                mode: ImageMode::JPEG,
                size: (60, 60),
                rotation: ImageRotation::Rot90,
                mirror: ImageMirroring::None,
            };
        }

        return ImageFormat {
            mode: ImageMode::JPEG,
            size: (60, 60),
            rotation: ImageRotation::Rot0,
            mirror: ImageMirroring::None,
        };
    }
}

#[derive(Debug, Clone)]
pub struct CandidateDevice {
    pub id: String,
    pub dev: HidDeviceInfo,
    pub kind: Kind,
}
