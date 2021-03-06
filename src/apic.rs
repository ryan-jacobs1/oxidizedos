use core::ops::Range;

use x86_64::instructions::port::Port;

use crate::machine;
use crate::println;

pub struct Apic {
    apic_base: usize,
}

impl Apic {
    const LAPIC_BASE_DEFAULT: usize = 0xFEE00000;
    const APIC_BASE_MSR: usize = 0x1B;
    const APIC_ENABLE: u32 = 1 << 11;
    const PIC1_DATA: u16 = 0x21;
    const PIC2_DATA: u16 = 0xa1;
    const INIT_IPI_MSG: u32 = 0x4500;
    const STARTUP_IPI_MSG: u32 = 0x4600;
    const PIT_FREQ: u32 = 1193182;

    /// Creates a new LAPIC at the default LAPIC address
    pub fn new() -> Self {
        Apic {
            apic_base: Apic::LAPIC_BASE_DEFAULT,
        }
    }

    pub fn with_base(apic_base: usize) -> Self {
        Apic {
            apic_base: (apic_base & 0xFFFFF000),
        }
    }

    /// Initializes the LAPIC by doing the following
    /// 1. Registering the Spurious Interrupt Vector with the LAPIC. By convention, this is 0xFF.
    /// 1. Disabling the PIC by masking IRQs
    /// 1. Enabling the LAPIC by writing to the appropriate MSR
    ///
    /// WARNING: Ensure that the PIC's IRQs have been remapped to >= 32.
    /// While the PIC's interrupts have been masked, spurious interrupts can still occur.
    /// If a spurious interrupt occurs while the IRQs have not been remapped, the IRQ will conflict
    /// with the hardware exception vectors.
    pub fn initialize(&self) {
        unsafe {
            self.write_register(ApicRegisterWritable::Spurious, 0x1FF)
                .unwrap();
        }
        Apic::disable_8259_pic();
        self.enable_apic();
    }

    // Disable the PIC by masking IRQs
    fn disable_8259_pic() {
        unsafe {
            let mut pic1_data_port: Port<u8> = Port::new(Apic::PIC1_DATA);
            let mut pic2_data_port: Port<u8> = Port::new(Apic::PIC2_DATA);
            pic1_data_port.write(0xff);
            pic2_data_port.write(0xff);
        }
    }

    // Enable the APIC by writing to the appropriate MSR
    fn enable_apic(&self) {
        unsafe {
            machine::wrmsr(
                (self.apic_base as u64) | (Apic::APIC_ENABLE as u64),
                Apic::APIC_BASE_MSR as u32,
            );
        }
    }

    /// Read a register from the APIC
    pub fn read_register(&self, reg: ApicRegisterReadable) -> Result<u32, ApicError> {
        let reg: ApicRegister = reg.into();
        let register_ptr = (self.apic_base + reg.get_offset()?) as *const u32;
        Ok(unsafe { core::ptr::read_volatile(register_ptr) })
    }

    /// Write to a register in the APIC.
    /// Unsafe because writing certain values to certain registers can fault.
    pub unsafe fn write_register(
        &self,
        reg: ApicRegisterWritable,
        val: u32,
    ) -> Result<(), ApicError> {
        let reg: ApicRegister = reg.into();
        let register_ptr = (self.apic_base + reg.get_offset()?) as *mut u32;
        Ok(core::ptr::write_volatile(register_ptr, val))
    }

    pub fn init_ipi(&self, lapic_id: u32) {
        unsafe {
            self.write_register(ApicRegisterWritable::InterruptCommand(1), lapic_id << 24)
                .unwrap();
            self.write_register(
                ApicRegisterWritable::InterruptCommand(0),
                Apic::INIT_IPI_MSG,
            )
            .unwrap();
        }
        while (self
            .read_register(ApicRegisterReadable::InterruptCommand(0))
            .unwrap()
            & (1 << 12))
            > 0
        {}
    }

    /// Sends a Startup IPI to lapic_id
    ///
    /// Arguments:
    /// reset: A function pointer for the given application processor to begin executing. Must be page aligned,
    /// and the physical page number must fit in 8 bits.
    pub fn startup_ipi(&self, lapic_id: u32, reset: unsafe extern "C" fn() -> !) {
        let reset_eip = reset as *const () as u32;
        unsafe {
            self.write_register(ApicRegisterWritable::InterruptCommand(1), lapic_id << 24)
                .unwrap();
            self.write_register(
                ApicRegisterWritable::InterruptCommand(0),
                Apic::STARTUP_IPI_MSG | (reset_eip >> 12),
            )
            .unwrap();
        }
        while (self
            .read_register(ApicRegisterReadable::InterruptCommand(0))
            .unwrap()
            & (1 << 12))
            > 0
        {}
    }

    /// Retrieves the ID of the core's LAPIC.
    /// The ID is a unique, per-core identifer of the LAPIC
    pub fn id(&self) -> usize {
        (self.read_register(ApicRegisterReadable::Id).unwrap() >> 24) as usize
    }

    pub fn calibrate(&self, hz: u32) -> u32 {
        let d = (Apic::PIT_FREQ / 20) as u8;
        let initial = 0xffffffff;
        let mut port_0: Port<u8> = Port::new(0x61);
        let mut port_1: Port<u8> = Port::new(0x43);
        let mut port_2: Port<u8> = Port::new(0x42);
        unsafe {
            self.write_register(ApicRegisterWritable::ApitLvtTimer, 0x00010000).unwrap();
            self.write_register(ApicRegisterWritable::ApitDivide, 0x00010000).unwrap();
            self.write_register(ApicRegisterWritable::ApitInitialCount, initial).unwrap();
            port_0.write(1);
            port_1.write(0b10110110);
            port_2.write(d);
            port_2.write(d >> 8);
        }
        let mut last = unsafe { port_0.read() & 0x20 };
        let mut changes = 0;

        while changes < 40 {
            let t = unsafe { port_0.read() & 0x20 };
            if t != last {
                changes += 1;
                last = t;
            }
        }
        let current_count = self.read_register(ApicRegisterReadable::ApitCurrentCount).unwrap();
        println!("current count {:x}", current_count);
        let diff = initial - current_count;
        unsafe {
            port_0.write(0);
        }
        println!("diff {:x}", diff);
        println!("APIT running at {} hz", diff);
        let counter = diff / hz;
        println!("apit counter: {}", counter);
        counter
    }
}

#[derive(Debug)]
pub enum ApicError {
    RegisterOutOfRange,
}

// All registers present in the LAPIC
enum ApicRegister {
    Id,
    Version,
    TaskPriority,
    ArbitrationPriority,
    ProcessorPriority,
    EOI,
    RemoteRead,
    LogicalDestination,
    DestinationFormat,
    Spurious,
    InService(usize),
    TriggerMode(usize),
    InteruptRequest(usize),
    ErrorStatus,
    CorrectedMachineCheckInterrupt,
    InterruptCommand(usize),
    ApitLvtTimer,
    LvtThermal,
    LvtPerformance,
    LvtLint0,
    LvtLint1,
    LvtError,
    ApitInitialCount,
    ApitCurrentCount,
    ApitDivide,
}

impl ApicRegister {
    fn get_offset_base(&self) -> usize {
        match self {
            ApicRegister::Id => 0x20,
            ApicRegister::Version => 0x30,
            ApicRegister::TaskPriority => 0x80,
            ApicRegister::ArbitrationPriority => 0x90,
            ApicRegister::ProcessorPriority => 0xA0,
            ApicRegister::EOI => 0xB0,
            ApicRegister::RemoteRead => 0xC0,
            ApicRegister::LogicalDestination => 0xD0,
            ApicRegister::DestinationFormat => 0xE0,
            ApicRegister::Spurious => 0xF0,
            ApicRegister::InService(_) => 0x100,
            ApicRegister::TriggerMode(_) => 0x180,
            ApicRegister::InteruptRequest(_) => 0x200,
            ApicRegister::ErrorStatus => 0x280,
            ApicRegister::CorrectedMachineCheckInterrupt => 0x2F0,
            ApicRegister::InterruptCommand(_) => 0x300,
            ApicRegister::ApitLvtTimer => 0x320,
            ApicRegister::LvtThermal => 0x330,
            ApicRegister::LvtPerformance => 0x340,
            ApicRegister::LvtLint0 => 0x350,
            ApicRegister::LvtLint1 => 0x360,
            ApicRegister::LvtError => 0x370,
            ApicRegister::ApitInitialCount => 0x380,
            ApicRegister::ApitCurrentCount => 0x390,
            ApicRegister::ApitDivide => 0x3E0,
        }
    }

    fn get_register_range(&self) -> Range<usize> {
        match self {
            ApicRegister::InService(_) => 0..8,
            ApicRegister::TriggerMode(_) => 0..8,
            ApicRegister::InteruptRequest(_) => 0..8,
            ApicRegister::InterruptCommand(_) => 0..2,
            _ => 0..1,
        }
    }

    fn register_in_range(&self, reg: usize) -> bool {
        self.get_register_range().contains(&reg)
    }

    pub fn get_offset(&self) -> Result<usize, ApicError> {
        let reg = match self {
            ApicRegister::InService(reg)
            | ApicRegister::TriggerMode(reg)
            | ApicRegister::InteruptRequest(reg)
            | ApicRegister::InterruptCommand(reg) => *reg,
            _ => 0,
        };
        if self.register_in_range(reg) {
            Ok(self.get_offset_base() + (0x10 * reg))
        } else {
            Err(ApicError::RegisterOutOfRange)
        }
    }
}

impl From<ApicRegisterReadable> for ApicRegister {
    fn from(reg: ApicRegisterReadable) -> Self {
        match reg {
            ApicRegisterReadable::Id => ApicRegister::Id,
            ApicRegisterReadable::Version => ApicRegister::Version,
            ApicRegisterReadable::TaskPriority => ApicRegister::TaskPriority,
            ApicRegisterReadable::ArbitrationPriority => ApicRegister::ArbitrationPriority,
            ApicRegisterReadable::ProcessorPriority => ApicRegister::ProcessorPriority,
            ApicRegisterReadable::RemoteRead => ApicRegister::RemoteRead,
            ApicRegisterReadable::LogicalDestination => ApicRegister::LogicalDestination,
            ApicRegisterReadable::DestinationFormat => ApicRegister::DestinationFormat,
            ApicRegisterReadable::Spurious => ApicRegister::Spurious,
            ApicRegisterReadable::InService(reg) => ApicRegister::InService(reg),
            ApicRegisterReadable::TriggerMode(reg) => ApicRegister::TriggerMode(reg),
            ApicRegisterReadable::InteruptRequest(reg) => ApicRegister::InteruptRequest(reg),
            ApicRegisterReadable::ErrorStatus => ApicRegister::ErrorStatus,
            ApicRegisterReadable::CorrectedMachineCheckInterrupt => {
                ApicRegister::CorrectedMachineCheckInterrupt
            }
            ApicRegisterReadable::InterruptCommand(reg) => ApicRegister::InterruptCommand(reg),
            ApicRegisterReadable::ApitLvtTimer => ApicRegister::ApitLvtTimer,
            ApicRegisterReadable::LvtThermal => ApicRegister::LvtThermal,
            ApicRegisterReadable::LvtPerformance => ApicRegister::LvtPerformance,
            ApicRegisterReadable::LvtLint0 => ApicRegister::LvtLint0,
            ApicRegisterReadable::LvtLint1 => ApicRegister::LvtLint1,
            ApicRegisterReadable::LvtError => ApicRegister::LvtError,
            ApicRegisterReadable::ApitInitialCount => ApicRegister::ApitInitialCount,
            ApicRegisterReadable::ApitCurrentCount => ApicRegister::ApitCurrentCount,
            ApicRegisterReadable::ApitDivide => ApicRegister::ApitDivide,
        }
    }
}

impl From<ApicRegisterWritable> for ApicRegister {
    fn from(reg: ApicRegisterWritable) -> Self {
        match reg {
            ApicRegisterWritable::Id => ApicRegister::Id,
            ApicRegisterWritable::TaskPriority => ApicRegister::TaskPriority,
            ApicRegisterWritable::EOI => ApicRegister::EOI,
            ApicRegisterWritable::LogicalDestination => ApicRegister::LogicalDestination,
            ApicRegisterWritable::DestinationFormat => ApicRegister::DestinationFormat,
            ApicRegisterWritable::Spurious => ApicRegister::Spurious,
            ApicRegisterWritable::CorrectedMachineCheckInterrupt => {
                ApicRegister::CorrectedMachineCheckInterrupt
            }
            ApicRegisterWritable::InterruptCommand(usize) => ApicRegister::InterruptCommand(usize),
            ApicRegisterWritable::ApitLvtTimer => ApicRegister::ApitLvtTimer,
            ApicRegisterWritable::LvtThermal => ApicRegister::LvtThermal,
            ApicRegisterWritable::LvtPerformance => ApicRegister::LvtPerformance,
            ApicRegisterWritable::LvtLint0 => ApicRegister::LvtLint0,
            ApicRegisterWritable::LvtLint1 => ApicRegister::LvtLint1,
            ApicRegisterWritable::LvtError => ApicRegister::LvtError,
            ApicRegisterWritable::ApitInitialCount => ApicRegister::ApitInitialCount,
            ApicRegisterWritable::ApitDivide => ApicRegister::ApitDivide,
        }
    }
}

/// A LAPIC Register that can be read
pub enum ApicRegisterReadable {
    Id,
    Version,
    TaskPriority,
    ArbitrationPriority,
    ProcessorPriority,
    RemoteRead,
    LogicalDestination,
    DestinationFormat,
    Spurious,
    InService(usize),
    TriggerMode(usize),
    InteruptRequest(usize),
    ErrorStatus,
    CorrectedMachineCheckInterrupt,
    InterruptCommand(usize),
    ApitLvtTimer,
    LvtThermal,
    LvtPerformance,
    LvtLint0,
    LvtLint1,
    LvtError,
    ApitInitialCount,
    ApitCurrentCount,
    ApitDivide,
}

/// A LAPIC Register that can be written to
pub enum ApicRegisterWritable {
    Id,
    TaskPriority,
    EOI,
    LogicalDestination,
    DestinationFormat,
    Spurious,
    CorrectedMachineCheckInterrupt,
    InterruptCommand(usize),
    ApitLvtTimer,
    LvtThermal,
    LvtPerformance,
    LvtLint0,
    LvtLint1,
    LvtError,
    ApitInitialCount,
    ApitDivide,
}
