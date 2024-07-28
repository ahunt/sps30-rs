use super::shdlc::*;
use std::fmt;

// See page 6 of the datasheet for more details:
// https://sensirion.com/media/documents/8600FF88/64A3B8D6/Sensirion_PM_Sensors_Datasheet_SPS30.pdf
pub struct Measurement {
    // ug/m3
    mass_concentration_pm_1_0: f32,
    mass_concentration_pm_2_5: f32,
    mass_concentration_pm_4_0: f32,
    mass_concentration_pm_10_0: f32,
    // #/cm3
    number_concentration_pm_0_5: f32,
    number_concentration_pm_1_0: f32,
    number_concentration_pm_2_5: f32,
    number_concentration_pm_4_0: f32,
    number_concentration_pm_10_0: f32,
    // um
    typical_particle_size: f32,
}

impl Measurement {
    pub fn csv_header() -> String {
        String::from("Time,Mass Concentration PM1 (ug/m3),Mass Concentration PM2.5 (ug/m3),Mass Concentration PM4.0 (ug/m3),Mass Concentration PM10.0 (ug/m3),Number Concentration PM0.5 (#/cm3),Number Concentration PM1.0 (#/cm3),Number Concentration PM2.5 (#/cm3),Number Concentration PM4.0 (#/cm3),Number Concentration PM10.0 (#/cm3),Typical Particle Size (um)")
    }

    pub fn csv_row(&self) -> String {
        let date_time = time::OffsetDateTime::now_utc();
        // None of the time::format_description::well_known formats are actually well
        // known to e.g. gnuplot or LibreOffice (translation: good luck getting them
        // parsed).
        let format = time::macros::format_description!(
            version = 2,
            "[year]-[month]-[day]T[hour]:[minute]:[second]"
        );
        let formatted_date_time = date_time.format(&format).unwrap();
        format!(
            "{},{},{},{},{},{},{},{},{},{},{}",
            formatted_date_time,
            self.mass_concentration_pm_1_0,
            self.mass_concentration_pm_2_5,
            self.mass_concentration_pm_4_0,
            self.mass_concentration_pm_10_0,
            self.number_concentration_pm_0_5,
            self.number_concentration_pm_1_0,
            self.number_concentration_pm_2_5,
            self.number_concentration_pm_4_0,
            self.number_concentration_pm_10_0,
            self.typical_particle_size
        )
    }
}

impl fmt::Display for Measurement {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Measurement:
  Mass Concentrations (ug/m3):
    PM1={}
    PM2.5={}
    PM4={}
    PM10={}
  Number Concentrations (#/m3):
    PM0.5={}
    PM1={}
    PM2.5={}
    PM4={}
    PM10={}
  Typical Particle Size (um)={}",
            self.mass_concentration_pm_1_0,
            self.mass_concentration_pm_2_5,
            self.mass_concentration_pm_4_0,
            self.mass_concentration_pm_10_0,
            self.number_concentration_pm_0_5,
            self.number_concentration_pm_1_0,
            self.number_concentration_pm_2_5,
            self.number_concentration_pm_4_0,
            self.number_concentration_pm_10_0,
            self.typical_particle_size,
        )
    }
}

pub fn decode_measurement_frame(frame: &MisoFrame) -> Result<Measurement, String> {
    if frame.cmd != 0x03 {
        return Result::Err(String::from(
            "ReadMeasuredValues MISO frame must have cmd=0x03",
        ));
    }
    if frame.data.len() != 40 {
        return Result::Err(String::from(
            "ReadMeasuredValues MISO frame has unexpected length",
        ));
    }

    Result::Ok(Measurement {
        mass_concentration_pm_1_0: f32::from_be_bytes(frame.data[0..4].try_into().unwrap()),
        mass_concentration_pm_2_5: f32::from_be_bytes(frame.data[4..8].try_into().unwrap()),
        mass_concentration_pm_4_0: f32::from_be_bytes(frame.data[8..12].try_into().unwrap()),
        mass_concentration_pm_10_0: f32::from_be_bytes(frame.data[12..16].try_into().unwrap()),
        number_concentration_pm_0_5: f32::from_be_bytes(frame.data[16..20].try_into().unwrap()),
        number_concentration_pm_1_0: f32::from_be_bytes(frame.data[20..24].try_into().unwrap()),
        number_concentration_pm_2_5: f32::from_be_bytes(frame.data[24..28].try_into().unwrap()),
        number_concentration_pm_4_0: f32::from_be_bytes(frame.data[28..32].try_into().unwrap()),
        number_concentration_pm_10_0: f32::from_be_bytes(frame.data[32..36].try_into().unwrap()),
        typical_particle_size: f32::from_be_bytes(frame.data[36..40].try_into().unwrap()),
    })
}
