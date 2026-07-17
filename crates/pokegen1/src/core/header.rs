//! Save-header fields: trainer name and playtime.

use serde::{Deserialize, Serialize};

use crate::core::sram::{self, SaveData};
use crate::core::text;

/// In-game playtime, read from the save header.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Playtime {
    /// Hours played.
    pub hours: u8,
    /// Minutes played (0..=59 in normal play).
    pub minutes: u8,
}

/// Decode the trainer's name from the save header.
pub fn trainer_name(save: &SaveData) -> String {
    text::decode_string(save.slice(sram::NAME, sram::NAME_LEN))
}

/// Read the playtime (hours and minutes) from the save header.
pub fn playtime(save: &SaveData) -> Playtime {
    Playtime {
        hours: save.read_u8(sram::PLAYTIME_HOURS),
        minutes: save.read_u8(sram::PLAYTIME_MINUTES),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::test_support::{blank_sram, seed};

    #[test]
    fn trainer_name_decodes_seeded_name() {
        let mut buf = blank_sram();
        // "RED" in the Gen-1 charmap, 0x50-terminated.
        seed(&mut buf, sram::NAME, &[0x92, 0x87, 0x80, 0x94, 0x8D, 0x50]);
        let save = SaveData::new(buf);
        assert_eq!(trainer_name(&save), "RED");
    }

    #[test]
    fn playtime_reads_hours_and_minutes() {
        let mut buf = blank_sram();
        seed(&mut buf, sram::PLAYTIME_HOURS, &[24]);
        seed(&mut buf, sram::PLAYTIME_MINUTES, &[12]);
        let save = SaveData::new(buf);
        assert_eq!(
            playtime(&save),
            Playtime {
                hours: 24,
                minutes: 12
            }
        );
    }
}
