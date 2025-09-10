use alloc::boxed::Box;
use fdt::Fdt;
use log::info;
use spin::Mutex;
use syscall::Mmio;

use crate::{
    devices::uart_16550,
    dtb::{
        diag_uart_range, get_interrupt, interrupt_parent,
        irqchip::{register_irq, InterruptHandler, IRQ_CHIP},
    },
    scheme::{
        debug::{debug_input, debug_notify},
        irq::irq_trigger,
    },
};

pub enum SerialPort {
    Ns16550u8(&'static mut uart_16550::SerialPort<Mmio<u8>>),
    Ns16550u32(&'static mut uart_16550::SerialPort<Mmio<u32>>),
}

impl SerialPort {
    pub fn receive(&mut self) {
        //TODO: make PL011 receive work the same way as NS16550
        match self {
            Self::Ns16550u8(inner) => {
                while let Some(c) = inner.receive() {
                    debug_input(c);
                }
                debug_notify();
            }
            Self::Ns16550u32(inner) => {
                while let Some(c) = inner.receive() {
                    debug_input(c);
                }
                debug_notify();
            }
        }
    }

    pub fn write(&mut self, buf: &[u8]) {
        match self {
            Self::Ns16550u8(inner) => inner.write(buf),
            Self::Ns16550u32(inner) => inner.write(buf),
        }
    }
}

pub static COM1: Mutex<Option<SerialPort>> = Mutex::new(None);

pub struct Com1Irq {}

impl InterruptHandler for Com1Irq {
    fn irq_handler(&mut self, irq: u32) {
        if let Some(ref mut serial_port) = *COM1.lock() {
            serial_port.receive();
        };
        unsafe {
            irq_trigger(irq as u8);
            IRQ_CHIP.irq_eoi(irq);
        }
    }
}

pub unsafe fn init_early(dtb: &Fdt) {
    unsafe {
        if COM1.lock().is_some() {
            // Hardcoded UART
            return;
        }

        if let Some((phys, size, skip_init, _cts, compatible)) = diag_uart_range(dtb) {
            let virt = crate::PHYS_OFFSET + phys;
            let serial_opt = if compatible.contains("ns16550a") {
                //TODO: get actual register size from device tree
                let serial_port = uart_16550::SerialPort::<Mmio<u8>>::new(virt);
                if !skip_init {
                    serial_port.init();
                }
                Some(SerialPort::Ns16550u8(serial_port))
            } else if compatible.contains("snps,dw-apb-uart") {
                //TODO: get actual register size from device tree
                let serial_port = uart_16550::SerialPort::<Mmio<u32>>::new(virt);
                if !skip_init {
                    serial_port.init();
                }
                Some(SerialPort::Ns16550u32(serial_port))
            } else {
                None
            };
            match serial_opt {
                Some(serial) => {
                    *COM1.lock() = Some(serial);
                    info!("UART {:?} at {:#X} size {:#X}", compatible, virt, size);
                }
                None => {
                    log::warn!(
                        "UART {:?} at {:#X} size {:#X}: no driver found",
                        compatible,
                        virt,
                        size
                    );
                }
            }
        }
    }
}

pub unsafe fn init(fdt: &Fdt) -> Option<()> {
    unsafe {
        if let Some(node) = fdt.find_compatible(&["ns16550a", "snps,dw-apb-uart"]) {
            let intr = get_interrupt(fdt, &node, 0).unwrap();
            let interrupt_parent = interrupt_parent(fdt, &node)?;
            let phandle = interrupt_parent.property("phandle")?.as_usize()? as u32;
            let ic_idx = IRQ_CHIP.phandle_to_ic_idx(phandle)?;

            let virq = IRQ_CHIP.irq_chip_list.chips[ic_idx]
                .ic
                .irq_xlate(intr)
                .unwrap();
            info!("serial_port virq = {}", virq);
            register_irq(virq as u32, Box::new(Com1Irq {}));
            IRQ_CHIP.irq_enable(virq as u32);
        }
        if let Some(ref mut _serial_port) = *COM1.lock() {
            // serial_port.enable_irq(); // FIXME receive int is enabled by default in 16550. Disable by default?
        }
        Some(())
    }
}
