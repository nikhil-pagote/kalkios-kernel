use core::fmt;
use spin::MutexGuard;

#[cfg(feature = "serial_debug")]
use super::device::serial::{SerialPort, COM1};
use crate::{
    devices::graphical_debug::{DebugDisplay, DEBUG_DISPLAY},
    log::{Log, LOG},
};

pub struct Writer<'a> {
    log: MutexGuard<'a, Option<Log>>,
    #[cfg(feature = "serial_debug")]
    serial: MutexGuard<'a, Option<SerialPort>>,
    display: MutexGuard<'a, Option<DebugDisplay>>,
}

impl<'a> Writer<'a> {
    pub fn new() -> Writer<'a> {
        Writer {
            log: LOG.lock(),
            display: DEBUG_DISPLAY.lock(),
            #[cfg(feature = "serial_debug")]
            serial: COM1.lock(),
        }
    }

    pub fn write(&mut self, buf: &[u8], preserve: bool) {
        if preserve {
            if let Some(ref mut log) = *self.log {
                log.write(buf);
            }
        }

        if let Some(ref mut display) = *self.display {
            let _ = display.write(buf);
        }

        #[cfg(feature = "serial_debug")]
        {
            if let Some(ref mut serial) = *self.serial {
                serial.write(buf);
            }
        }
    }
}

impl<'a> fmt::Write for Writer<'a> {
    fn write_str(&mut self, s: &str) -> Result<(), fmt::Error> {
        self.write(s.as_bytes(), true);
        Ok(())
    }
}
