use std::io::{Error, ErrorKind, Result};

#[derive(Clone, Debug, Default)]
pub struct Uplink {
    temperature: Option<f32>,
    co2: Option<u16>,
    battery_mv: Option<u16>,
    occupancy: Option<Occupancy>,
    external_digital: Option<bool>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Occupancy {
    NoBody,
    PendingOrPir,
    OccupiedOrHeat,
}

impl PartialEq for Uplink {
    fn eq(&self, other: &Self) -> bool {
        close(self.temperature, other.temperature, 0.1)
            && self.co2 == other.co2
            && self.battery_mv == other.battery_mv
            && self.occupancy == other.occupancy
            && self.external_digital == other.external_digital
    }
}

fn close(x: Option<f32>, y: Option<f32>, resolution: f32) -> bool {
    match (x, y) {
        (Some(a), Some(b)) => (a - b) * 2.0 < resolution && (b - a) * 2.0 < resolution,
        (Some(_), None) | (None, Some(_)) => false,
        (None, None) => true,
    }
}

struct Layout {
    bin_to: fn(&[u8], usize, &mut Uplink) -> Result<()>,
    identifier: u8,
    size: usize,
}

#[rustfmt::skip]
const LAYOUT: &[Layout] = &[
    Layout { identifier: 0x01, size: 2, bin_to: temperature },     //                         -3276.8°C --> 3276.7°C
    Layout { identifier: 0x02, size: 1, bin_to: no_decode },       // Humidity              ; 0-100%
    Layout { identifier: 0x03, size: 3, bin_to: no_decode },       // Acceleration          ; X,Y,Z -128 --> 127 +/-63=1G
    Layout { identifier: 0x04, size: 2, bin_to: no_decode },       // Light                 ; 0 --> 65535 Lux
    Layout { identifier: 0x05, size: 1, bin_to: no_decode },       // Motion                ; No of motion 0-255
    Layout { identifier: 0x06, size: 2, bin_to: co2 },             //                         0-65535 ppm
    Layout { identifier: 0x07, size: 2, bin_to: battery },         //                       ; 0-65535mV
    Layout { identifier: 0x08, size: 2, bin_to: no_decode },       // Analog1               ; 0-65535mV
    Layout { identifier: 0x09, size: 6, bin_to: no_decode },       // GPS                   ; latitude & longitude
    Layout { identifier: 0x0a, size: 2, bin_to: no_decode },       // Pulse1                ; relative pulse count
    Layout { identifier: 0x0b, size: 4, bin_to: no_decode },       // PulseAbs              ; no 0 --> 0xFFFFFFFF
    Layout { identifier: 0x0c, size: 2, bin_to: no_decode },       // External Temperature 1; -3276.5C --> 3276.5C
    Layout { identifier: 0x0d, size: 1, bin_to: external_digital },//                         1 or 0
    Layout { identifier: 0x0e, size: 2, bin_to: no_decode },       // External Distance     ; mm
    Layout { identifier: 0x0f, size: 1, bin_to: no_decode },       // Acceleration Motion   ; number of vibration/motion
    Layout { identifier: 0x10, size: 4, bin_to: no_decode },       // Internal And External Temperatures; -3276.5C --> 3276.5C
    Layout { identifier: 0x11, size: 1, bin_to: occupancy },       // Occupancy
    Layout { identifier: 0x12, size: 1, bin_to: no_decode },       // Waterleak             ; 0-255
    Layout { identifier: 0x13, size: 65, bin_to: no_decode },      // Grideye               ; 1 byte ref + 64 bytes external temperature
    Layout { identifier: 0x14, size: 4, bin_to: no_decode },       // Pressure              ; hPa
    Layout { identifier: 0x15, size: 2, bin_to: no_decode },       // Sound                 ; peak/avg
    Layout { identifier: 0x16, size: 2, bin_to: no_decode },       // Pulse2                ; 0 --> 0xFFFF
    Layout { identifier: 0x17, size: 4, bin_to: no_decode },       // Pulse2 Abs            ; No 0 --> 0xFFFFFFFF
    Layout { identifier: 0x18, size: 2, bin_to: no_decode },       // Analog2               ; Voltage in mV
    Layout { identifier: 0x19, size: 2, bin_to: no_decode },       // External Temperature2 ; -3276.5C --> 3276.5C
    Layout { identifier: 0x1a, size: 1, bin_to: no_decode },       // External Digital2     ; 1 or 0
    Layout { identifier: 0x1b, size: 4, bin_to: no_decode },       // External Analog       ; uV
    Layout { identifier: 0x1c, size: 2, bin_to: no_decode },       // TVOC                  ; ppb
    Layout { identifier: 0x3d, size: 4, bin_to: no_decode },       // Debug
];

impl Uplink {
    pub fn deserialize(input: &[u8]) -> Result<Self> {
        let mut output = Self::default();

        let mut i = 0;
        while i < input.len() {
            let mut identifier_found = false;
            for deserialise_pattern in LAYOUT {
                if input[i] == deserialise_pattern.identifier {
                    identifier_found = true;
                    verify_array_length(input, i, deserialise_pattern.size)?;
                    (deserialise_pattern.bin_to)(input, i + 1, &mut output)?;
                    i += deserialise_pattern.size;
                    break;
                }
            }

            verify_pattern_matches(input, i, identifier_found)?;

            i += 1;
        }

        Ok(output)
    }

    pub fn temperature(&self) -> Option<f32> {
        self.temperature
    }

    pub fn co2_ppm(&self) -> Option<u16> {
        self.co2
    }

    pub fn battery_voltage(&self) -> Option<f32> {
        self.battery_mv.map(|bmv| bmv as f32 * 0.001)
    }

    pub fn external_digital(&self) -> Option<bool> {
        self.external_digital
    }

    pub fn occupancy(&self) -> Option<Occupancy> {
        self.occupancy
    }
}

fn verify_array_length(input: &[u8], i: usize, pattern_size: usize) -> Result<()> {
    if input.len() <= i + pattern_size {
        return Err(Error::new(
            ErrorKind::InvalidData,
            format!(
                "{:?} does not look like an Elsys Uplink \
            (index {} has value {}, which is length {})",
                input, i, input[i], pattern_size
            ),
        ));
    }

    Ok(())
}

fn verify_pattern_matches(input: &[u8], i: usize, identifier_found: bool) -> Result<()> {
    if !identifier_found {
        return Err(Error::new(
            ErrorKind::InvalidData,
            format!(
                "{:?} does not look like an Elsys Uplink \
            (index {} has value {}, which is not an identifier)",
                input, i, input[i]
            ),
        ));
    }

    Ok(())
}

fn temperature(input: &[u8], i: usize, output: &mut Uplink) -> Result<()> {
    let temperature_x10_pos = ((input[i] as u16) << 8) | input[i + 1] as u16;
    let temperature_x10 = bin16_to_dec(temperature_x10_pos);
    output.temperature = Some(temperature_x10 as f32 * 0.1);
    Ok(())
}

fn co2(input: &[u8], i: usize, output: &mut Uplink) -> Result<()> {
    output.co2 = Some(((input[i] as u16) << 8) | input[i + 1] as u16);
    Ok(())
}

fn battery(input: &[u8], i: usize, output: &mut Uplink) -> Result<()> {
    output.battery_mv = Some(((input[i] as u16) << 8) | input[i + 1] as u16);
    Ok(())
}

fn external_digital(input: &[u8], i: usize, output: &mut Uplink) -> Result<()> {
    output.external_digital = match input[i] {
        0 => Some(false),
        1 => Some(true),
        _ => {
            return Err(Error::new(
                ErrorKind::InvalidData,
                format!(
                    "{:?}: index {} has value {}, which is not a window contact value",
                    input, i, input[i]
                ),
            ))
        }
    };
    Ok(())
}

fn occupancy(input: &[u8], i: usize, output: &mut Uplink) -> Result<()> {
    output.occupancy = match input[i] {
        0 => Some(Occupancy::NoBody),
        1 => Some(Occupancy::PendingOrPir),
        2 => Some(Occupancy::OccupiedOrHeat),
        _ => {
            return Err(Error::new(
                ErrorKind::InvalidData,
                format!(
                    "{:?}: index {} has value {}, which is not an occupancy value",
                    input, i, input[i]
                ),
            ))
        }
    };
    Ok(())
}

fn no_decode(_: &[u8], _: usize, _: &mut Uplink) -> Result<()> {
    Ok(())
}

fn bin16_to_dec(bin: u16) -> i16 {
    if 0x8000 & bin == 0 {
        bin as i16
    } else {
        let negative = -(0x010000 - bin as i64);
        negative as i16
    }
}

#[rustfmt::skip]
#[cfg(test)]
#[path = "./test_uplink.rs"]
mod test_uplink;
