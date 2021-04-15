use crate::machine;
use crate::println;

/*** FOR HELP UNDERSTANDING THE PCI GO TO wiki.osdev.org/PCI ***/

/* I/O port for PCI configuration */
pub static CONFIG_ADDRESS: u32 = 0xCF8;

/* I/O port for accessing CONFIG_DATA register */
pub static CONFIG_DATA: u32 = 0xCFC;

#[derive(Default)]
pub struct PCIDeviceHeader {
    pub vendor_id: u16,
    pub device_id: u16,
    pub command: u16,
    pub status: u16,
    pub revision_id: u8,
    pub prog_if: u8,
    pub subclass: u8,
    pub class_code: u8,
    pub cache_line_sz: u8,
    pub latency_timer: u8,
    pub header_type: u8,
    pub bist: u8,

    pub bus: u8,
    pub slot: u8,
}

impl PCIDeviceHeader {
    fn new(bus: u8, slot: u8) -> PCIDeviceHeader {
        let mut header = PCIDeviceHeader::default();
        header.bus = bus;
        header.slot = slot;
        header.read_in_header();

        header
    }

    fn read_in_header(&mut self) {
        let bus = self.bus;
        let slot = self.slot;

        self.vendor_id = config_read16(bus, slot, 0, 0);
        self.device_id = config_read16(bus, slot, 0, 2);
        self.command = config_read16(bus, slot, 0, 4);
        self.status = config_read16(bus, slot, 0, 6);
        self.revision_id = config_read8(bus, slot, 0, 8);
        self.prog_if = config_read8(bus, slot, 0, 9);
        self.subclass = config_read8(bus, slot, 0, 10);
        self.class_code = config_read8(bus, slot, 0, 11);
        self.cache_line_sz = config_read8(bus, slot, 0, 12);
        self.latency_timer = config_read8(bus, slot, 0, 13);
        self.header_type = config_read8(bus, slot, 0, 14);
        self.bist = config_read8(bus, slot, 0, 15);
    }

    fn print_header(&self) {
        println!("At bus {} and slot {} we have device:", self.bus, self.slot);
        println!("Device ID      : 0x{:x}", self.device_id);
        println!("Vendor ID      : 0x{:x}", self.vendor_id);
        println!("Command        : 0x{:x}", self.command);
        println!("Status         : 0x{:x}", self.status);
        println!("Revision ID    : 0x{:x}", self.revision_id);
        println!("Prog IF        : 0x{:x}", self.prog_if);
        println!("Class Code     : 0x{:x}", self.class_code);
        println!("Subclass       : 0x{:x}", self.subclass);
        println!("Cache Line Size: 0x{:x}", self.cache_line_sz);
        println!("Latency Timer  :   {}", self.latency_timer);
        println!("Header Type    : 0x{:x}", self.header_type);
        println!("BIST           : 0x{:x}", self.bist);
        if self.has_multiple_funcs() {
            println!("This device has multiple functions.");
        } else {
            println!("This device has only one function.");
        }
    }

    fn has_multiple_funcs(&self) -> bool {
        (self.header_type & (1 << 7)) > 0
    }

    fn write_to_command(&mut self, new_cmd: u16) {
        config_write16(self.bus, self.slot, 0, 4, new_cmd);
        self.read_in_header();
    }
}

// this is a class for pci devices with header type 00b, the most common
// type 01b headers are for PCI-to-PCI bridges
// type 02b headers are for PCI-to-CardBus bridges
#[derive(Default)]
pub struct PCI00DeviceInfo {
    pub header: PCIDeviceHeader,
    pub base_addr: [u32; 6], // 6 entries exist
    pub card_bus_cis_ptr: u32,
    pub subsystem_vendor_id: u16,
    pub subsystem_id: u16,
    pub expansion_rom_base_addr: u32,
    pub capabilities_ptr: u8,
    pub reserved1: u8,
    pub reserved2: u16,
    pub reserved3: u32,
    pub interrupt_line: u8,
    pub interrupt_pin: u8,
    pub acpi_pin_assignment: u8,
    pub acpi_intin_assignment: u8,
    pub min_grant: u8,
    pub max_latency: u8,
}

impl PCI00DeviceInfo {
    fn new(bus: u8, slot: u8) -> PCI00DeviceInfo {
        let header = PCIDeviceHeader::new(bus, slot);
        let mut device = PCI00DeviceInfo::default();
        device.header = header;
        device.read_in_device();

        device
    }

    fn read_in_device(&mut self) {
        self.header.read_in_header();
        let bus = self.header.bus;
        let slot = self.header.slot;

        for i in 0..6 {
            self.base_addr[i] = config_read32(bus, slot, 0, (16 + 4 * i) as u8);
        }
        self.card_bus_cis_ptr = config_read32(bus, slot, 0, 0x28);
        self.subsystem_vendor_id = config_read16(bus, slot, 0, 0x2c);
        self.subsystem_id = config_read16(bus, slot, 0, 0x2e);
        self.expansion_rom_base_addr = config_read32(bus, slot, 0, 0x30);
        self.capabilities_ptr = config_read8(bus, slot, 0, 0x34);
        self.reserved1 = config_read8(bus, slot, 0, 0x35);
        self.reserved2 = config_read16(bus, slot, 0, 0x36);
        self.reserved3 = config_read32(bus, slot, 0, 0x38);
        self.interrupt_line = config_read8(bus, slot, 0, 0x3c);
        self.interrupt_pin = config_read8(bus, slot, 0, 0x3d);
        self.min_grant = config_read8(bus, slot, 0, 0x3e);
        self.max_latency = config_read8(bus, slot, 0, 0x3f);
    }

    fn get_address_space_size(&self, bar_num: usize) -> u32 {
        let mut sz = 0;

        // if invalid BAR, return sz 0
        if self.base_addr[bar_num] == 0 {
            return 0;
        }
        // write all 1s to the BAR
        config_write32(
            self.header.bus,
            self.header.slot,
            0,
            (16 + 4 * bar_num) as u8,
            0xFFFFFFFF,
        );
        sz = config_read32(
            self.header.bus,
            self.header.slot,
            0,
            (16 + 4 * bar_num) as u8,
        );
        // mask info bits
        if (self.base_addr[bar_num] & 1) == 1 {
            sz &= 0xFFFFFFFC;
        } else {
            sz &= 0xFFFFFFF0;
        }

        sz = !sz;
        sz += 1;

        // write back original BAR value
        config_write32(
            self.header.bus,
            self.header.slot,
            0,
            (16 + 4 * bar_num) as u8,
            self.base_addr[bar_num],
        );

        sz
    }

    fn print_header(&self) {
        //self.header.print_header();
        println!("Specific info for header 00:");
        for i in 0..6 {
            println!("Base Address {}         : 0x{:x}", i, self.base_addr[i]);
        }
        println!("Cardbus CIS Ptr        : 0x{:x}", self.card_bus_cis_ptr);
        println!("Subsystem ID           : 0x{:x}", self.subsystem_id);
        println!("Subsystem Vendor ID    : 0x{:x}", self.subsystem_vendor_id);
        println!(
            "Expansion ROM Base Addr: 0x{:x}",
            self.expansion_rom_base_addr
        );
        println!("Capabilities PTR       : 0x{:x}", self.capabilities_ptr);
        println!("Reserved  8bits        : 0x{:x}", self.reserved1);
        println!("Reserved 16bits        : 0x{:x}", self.reserved2);
        println!("Reserved 32bits        : 0x{:x}", self.reserved3);
        println!("Interrupt PIN          : 0x{:x}", self.interrupt_pin);
        println!("Interrupt Line         : 0x{:x}", self.interrupt_line);
        println!("Interrupt PIN by MP    : 0x{:x}", self.acpi_pin_assignment);
        println!(
            "Intin by MP            : 0x{:x}",
            self.acpi_intin_assignment
        );
        println!("Min Grant              :   {}", self.min_grant);
        println!("Max Latency            :   {}", self.max_latency);
    }

    /*
     * too lazy to implement this
    fn printBARsVerbose(&self) {
        for i in 0..6 {
            // if this BAR is 0, print it does not exist
            if self.base_addr[i] == 0 {
                println!("Base Address Register {} is null", i);
            } else {
                println!("Base Address Register {} is a ", i);
                if ((self.base_addr[i] & 1) > 0) {
                    println!("I/O Space BAR: 0x{}",
                           (self.base_addr[i] & 0xFFFFFFFC));
                    println!("    Its address space is size: 0x{} bytes",
                           getAddressSpaceSize(i));
                } else {
                    println!("Memory Space BAR ");
                    if (((self.base_addr[i] >> 3) & 1) > 1 {
                        println!("that is prefetchable with register size ");
                    else
                        println!("that is not prefetchable with register size ");

                    if (((self.base_addr[i] >> 1) & 3) == 0) {
                        println!("32 bits: 0x{}", (self.base_addr[i] & 0xFFFFFFF0));
                        println!("    Its address space is size: 0x{} bytes",
                               getAddressSpaceSize(i));
                    } else if (((self.base_addr[i] >> 1) & 3) == 3)
                        println!("64 bits");
                    else
                        println!("unsupported");
                }
            }
        }
    } */
}

//TODO extern ArrList<PCI00DeviceInfo *> *pci00Devs;

/* Allows a device to register itself for an IRQ */
/*
class PCIDevice;
void registerDevice(PCIDevice *device);
class PCIDevice {
public:
    u8 intin = 0xFF;

    PCIDevice(PCI00DeviceInfo &info) : intin(info.acpi_intin_assignment)
    {
        registerDevice(this);
    }
    virtual ~PCIDevice()
    {}

    // methods used from this interface externally
    virtual bool wasInterrupted() = 0;
    virtual void handleInterrupt() = 0;
};

void init(void);
void initIrq(void);
*/

fn config_read8(bus: u8, slot: u8, func: u8, offset: u8) -> u8 {
    let lbus: u32 = bus as u32;
    let lslot: u32 = slot as u32;
    let lfunc: u32 = func as u32;
    let offset: u32 = offset as u32;

    // address configuration:
    //             31        30-24         23-16         15-11          10-8           7-0 (but bottom two bits always 0b00)
    //        enable bit | reserved | bus no.      | slot no.      | function no. | register offset
    let address: u32 = (1 << 31) | (lbus << 16) | (lslot << 11) | (lfunc << 8) | (offset & 0xff);
    let tmp: u8 = unsafe {
        // set address we want to read from
        machine::outl(CONFIG_ADDRESS, address);
        // read data now that address is specified
        machine::inb(CONFIG_DATA)
    };

    tmp
}
fn config_write8(bus: u8, slot: u8, func: u8, offset: u8, data: u8) {
    let lbus: u32 = bus as u32;
    let lslot: u32 = slot as u32;
    let lfunc: u32 = func as u32;
    let offset: u32 = offset as u32;
    let to_write: u32 = data as u32;

    // address configuration:
    //             31        30-24         23-16         15-11          10-8           7-0 (but bottom two bits always 0b00)
    //        enable bit | reserved | bus no.      | slot no.      | function no. | register offset
    let address: u32 = (1 << 31) | (lbus << 16) | (lslot << 11) | (lfunc << 8) | (offset & 0xff);
    unsafe {
        // set address we want to read from
        machine::outl(CONFIG_ADDRESS, address);
        // write data now that address is specified
        machine::outb(CONFIG_DATA, to_write);
    }
}

fn config_read16(bus: u8, slot: u8, func: u8, offset: u8) -> u16 {
    let lbus: u32 = bus as u32;
    let lslot: u32 = slot as u32;
    let lfunc: u32 = func as u32;
    let offset: u32 = offset as u32;

    // address configuration:
    //           31        30-24         23-16         15-11          10-8           7-0 (but bottom two bits always 0b00)
    //       enable bit | reserved | bus no.      | slot no.      | function no. | register offset
    let address: u32 = (1 << 31) | (lbus << 16) | (lslot << 11) | (lfunc << 8) | (offset & 0xfc);
    let tmp: u16 = unsafe {
        // set address we want to read from
        machine::outl(CONFIG_ADDRESS, address);
        // read data now that address is specified
        ((machine::inl(CONFIG_DATA) >> ((offset & 2) * 8)) & 0xFFFF) as u16
    };

    tmp
}
fn config_write16(bus: u8, slot: u8, func: u8, offset: u8, data: u16) {
    let lbus: u32 = bus as u32;
    let lslot: u32 = slot as u32;
    let lfunc: u32 = func as u32;
    let offset: u32 = offset as u32;
    let mut to_write: u32 = data as u32;

    // address configuration:
    //           31        30-24         23-16         15-11          10-8           7-0 (but bottom two bits always 0b00)
    //       enable bit | reserved | bus no.      | slot no.      | function no. | register offset
    let address: u32 = (1 << 31) | (lbus << 16) | (lslot << 11) | (lfunc << 8) | (offset & 0xfc);

    // if writing to upper half of a word, get lower half of word and build toWrite
    if (offset & 2) > 0 {
        to_write <<= 16;
        to_write |= config_read16(bus, slot, func, (offset & 0xfc) as u8) as u32;
    }
    // if writing to lower half of a word, get upper half of word and build toWrite
    else {
        to_write |=
            ((config_read16(bus, slot, func, ((offset & 0xfc) + 2) as u8) as u32) << 16) as u32;
    }
    unsafe {
        // set address we want to write to
        machine::outl(CONFIG_ADDRESS, address);
        // write data now that address is specified
        machine::outl(CONFIG_DATA, to_write);
    }
}

fn config_read32(bus: u8, slot: u8, func: u8, offset: u8) -> u32 {
    let lbus: u32 = bus as u32;
    let lslot: u32 = slot as u32;
    let lfunc: u32 = func as u32;
    let offset: u32 = offset as u32;

    // address configuration:
    //           31        30-24         23-16         15-11          10-8           7-0 (but bottom two bits always 0b00)
    //       enable bit | reserved | bus no.      | slot no.      | function no. | register offset
    let address: u32 = (1 << 31) | (lbus << 16) | (lslot << 11) | (lfunc << 8) | (offset & 0xfc);
    let tmp: u32 = unsafe {
        // set address we want to read from
        machine::outl(CONFIG_ADDRESS, address);
        // read data now that address is specified
        machine::inl(CONFIG_DATA)
    };

    tmp
}
fn config_write32(bus: u8, slot: u8, func: u8, offset: u8, data: u32) {
    let lbus: u32 = bus as u32;
    let lslot: u32 = slot as u32;
    let lfunc: u32 = func as u32;
    let offset: u32 = offset as u32;
    let to_write: u32 = data;

    // address configuration:
    //             31        30-24         23-16         15-11          10-8           7-0 (but bottom two bits always 0b00)
    //        enable bit | reserved | bus no.      | slot no.      | function no. | register offset
    let address: u32 = (1 << 31) | (lbus << 16) | (lslot << 11) | (lfunc << 8) | (offset & 0xfc);
    unsafe {
        // set address we want to read from
        machine::outl(CONFIG_ADDRESS, address);
        // write data now that address is specified
        machine::outl(CONFIG_DATA, to_write);
    }
}

fn check_vendor(bus: u8, slot: u8) -> u16 {
    // try and read first configuration register
    // if we get 0xFFFF then we know that the slot has no device since no device has vendor id 0xFFFF
    let vendor: u16 = config_read16(bus, slot, 0, 0);
    if vendor != 0xFFFF {
        // the slot has a device that exists
        let device: u16 = config_read16(bus, slot, 0, 2);

        println!(
            "The bus {} has slot {} with device {:x} and vendor {:x}",
            bus, slot, device, vendor
        );
        let tmp: Option<PCI00DeviceInfo> = get_00device_info(bus, slot);
        if let Some(dev) = tmp {
            dev.print_header();
            println!("\n");
        }
    }

    vendor
}

pub fn check_all_buses() {
    for bus in 0..256 {
        for slot in 0..32 {
            check_vendor(bus as u8, slot as u8);
        }
    }
}

/*
void loadDevice(u8 bus, u8 slot);
void loadAllPCI00Devices();
*/

// returns an Option: Some with a pci device with header 00 if found, None otherwise
fn get_00device_info(bus: u8, slot: u8) -> Option<PCI00DeviceInfo> {
    let header_type: u8 = (config_read16(bus, slot, 0, 14) & 0xFF) as u8;
    // if headertype00, return the more specific object
    if (header_type & 3) == 0 {
        Some(PCI00DeviceInfo::new(bus, slot))
    } else {
        None
    }
}

/*
PCI00DeviceInfo *find00Device(u16 device_id, u16 vendor_id);
PCI00DeviceInfo *get00Device(u8 bus, u8 slot);

void registerIrq(u8 intin);
*/
