use ssmarshal::serialize;
use usbd_hid::descriptor::{
    generator_prelude::*, MediaKeyboardReport, MouseReport, SystemControlReport,
};

#[gen_hid_descriptor(
    (collection = APPLICATION, usage_page = 0xFF60, usage = 0x61) = {
        (usage = 0x62, logical_min = 0x0) = {
            #[item_settings data,variable,absolute] input_data=input;
        };
        (usage = 0x63, logical_min = 0x0) = {
            #[item_settings data,variable,absolute] output_data=output;
        };
    }
)]
pub(crate) struct ViaReport {
    pub(crate) input_data: [u8; 32],
    pub(crate) output_data: [u8; 32],
}

/// Predefined report ids for composite hid report.
/// Should be same with `#[gen_hid_descriptor]`
/// DO NOT EDIT
#[repr(u8)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]

pub(crate) enum CompositeReportType {
    None = 0x00,
    Mouse = 0x01,
    Consumer = 0x02,
    System = 0x03,
}

impl CompositeReportType {
    fn from_u8(report_id: u8) -> Self {
        match report_id {
            0x01 => Self::Mouse,
            0x02 => Self::Consumer,
            0x03 => Self::System,
            _ => Self::None,
        }
    }
}

/// A composite hid report which contains mouse, consumer, system reports.
/// Report id is used to distinguish from them.
#[gen_hid_descriptor(
    (report_id = 0x01, collection = APPLICATION, usage_page = GENERIC_DESKTOP, usage = MOUSE) = {
        (collection = PHYSICAL, usage = POINTER) = {
            (usage_page = BUTTON, usage_min = BUTTON_1, usage_max = BUTTON_8) = {
                #[packed_bits 8] #[item_settings data,variable,absolute] buttons=input;
            };
            (usage_page = GENERIC_DESKTOP,) = {
                (usage = X,) = {
                    #[item_settings data,variable,relative] x=input;
                };
                (usage = Y,) = {
                    #[item_settings data,variable,relative] y=input;
                };
                (usage = WHEEL,) = {
                    #[item_settings data,variable,relative] wheel=input;
                };
            };
            (usage_page = CONSUMER,) = {
                (usage = AC_PAN,) = {
                    #[item_settings data,variable,relative] pan=input;
                };
            };
        };
    },
    (report_id = 0x02, collection = APPLICATION, usage_page = CONSUMER, usage = CONSUMER_CONTROL) = {
        (usage_page = CONSUMER, usage_min = 0x00, usage_max = 0x514) = {
            #[item_settings data,array,absolute,not_null] media_usage_id=input;
        };
    },
    (report_id = 0x03, collection = APPLICATION, usage_page = GENERIC_DESKTOP, usage = SYSTEM_CONTROL) = {
        (usage_min = 0x81, usage_max = 0xB7, logical_min = 1) = {
            #[item_settings data,array,absolute,not_null] system_usage_id=input;
        };
    }
)]
#[derive(Default)]
pub struct CompositeReport {
    pub(crate) buttons: u8,
    pub(crate) x: i8,
    pub(crate) y: i8,
    pub(crate) wheel: i8, // Scroll down (negative) or up (positive) this many units
    pub(crate) pan: i8,   // Scroll left (negative) or right (positive) this many units
    pub(crate) media_usage_id: u16,
    pub(crate) system_usage_id: u8,
}

impl CompositeReport {
    pub(crate) fn serialize(
        &self,
        mut data: &mut [u8],
        report_type: CompositeReportType,
    ) -> Result<usize, ssmarshal::Error> {
        // TODO: Optimize it
        // Use usbd-hid's report to do serialization, but not so efficient.
        match report_type {
            CompositeReportType::None => Ok(0),
            CompositeReportType::Mouse => {
                let mouse_report = MouseReport {
                    buttons: self.buttons,
                    x: self.x,
                    y: self.y,
                    wheel: self.wheel,
                    pan: self.pan,
                };
                Ok(serialize(&mut data, &mouse_report)?)
            }
            CompositeReportType::Consumer => {
                let consumer_report = MediaKeyboardReport {
                    usage_id: self.media_usage_id,
                };
                Ok(serialize(&mut data, &consumer_report)?)
            }
            CompositeReportType::System => {
                let system_report = SystemControlReport {
                    usage_id: self.system_usage_id as u8,
                };
                Ok(serialize(&mut data, &system_report)?)
            }
        }
    }
}