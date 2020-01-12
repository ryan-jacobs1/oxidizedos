use crate::println;
use crate::machine;

/*** FOR HELP UNDERSTANDING THE PCI GO TO wiki.osdev.org/PCI ***/

/* I/O port for PCI configuration */
pub static CONFIG_ADDRESS: u32 = 0xCF8;

/* I/O port for accessing CONFIG_DATA register */
pub static CONFIG_DATA: u32 = 0xCFC;

#[derive(Default)]
pub struct PCIDeviceHeader {
    pub vendorID: u16,
    pub deviceID: u16,
    pub command: u16,
    pub status: u16,
    pub revisionID: u8,
    pub progIF: u8,
    pub subclass: u8,
    pub classCode: u8,
    pub cacheLineSz: u8,
    pub latencyTimer: u8,
    pub headerType: u8,
    pub BIST: u8,

    pub bus: u8,
    pub slot: u8,
}

impl PCIDeviceHeader {
    fn new(bus: u8, slot: u8) -> PCIDeviceHeader {
        let mut header = PCIDeviceHeader::default();
        header.bus = bus;
        header.slot = slot;
        header.readInHeader();

        header
    }

    fn readInHeader(&mut self) {
        let bus = self.bus;
        let slot = self.slot;

		self.vendorID = configRead16(bus, slot, 0, 0);
		self.deviceID = configRead16(bus, slot, 0, 2);
		self.command = configRead16(bus, slot, 0, 4);
		self.status = configRead16(bus, slot, 0, 6);
		self.revisionID = configRead8(bus, slot, 0, 8);
		self.progIF = configRead8(bus, slot, 0, 9);
		self.subclass = configRead8(bus, slot, 0, 10);
		self.classCode = configRead8(bus, slot, 0, 11);
		self.cacheLineSz = configRead8(bus, slot, 0, 12);
		self.latencyTimer = configRead8(bus, slot, 0, 13);
		self.headerType = configRead8(bus, slot, 0, 14);
		self.BIST = configRead8(bus, slot, 0, 15);
    }

    fn printHeader(&self) {
        println!("At bus {} and slot {} we have device:", self.bus, self.slot);
		println!("Device ID      : 0x{}", self.deviceID);
		println!("Vendor ID      : 0x{}", self.vendorID);
		println!("Command        : 0x{}", self.command);
		println!("Status         : 0x{}", self.status);
		println!("Revision ID    : 0x{}", self.revisionID);
		println!("Prog IF        : 0x{}", self.progIF);
		println!("Class Code     : 0x{}", self.classCode);
		println!("Subclass       : 0x{}", self.subclass);
		println!("Cache Line Size: 0x{}", self.cacheLineSz);
		println!("Latency Timer  :   {}", self.latencyTimer);
		println!("Header Type    : 0x{}", self.headerType);
		println!("BIST           : 0x{}", self.BIST);
		if self.hasMultipleFuncs() {
            println!("This device has multiple functions.");
        } else {
            println!("This device has only one function.");
        }
    }

    fn hasMultipleFuncs(&self) -> bool {
        (self.headerType & (1 << 7)) > 0
    }

    fn writeToCommand(&mut self, newCmd: u16) {
        configWrite16(self.bus, self.slot, 0, 4, newCmd);
		self.readInHeader();
    }
}

// this is a class for pci devices with header type 00b, the most common
// type 01b headers are for PCI-to-PCI bridges
// type 02b headers are for PCI-to-CardBus bridges
#[derive(Default)]
pub struct PCI00DeviceInfo {
    pub header: PCIDeviceHeader,
    pub baseAddr: [u32; 6], // 6 entries exist
    pub cardBusCISPtr: u32,
    pub subsystemVendorID: u16,
    pub subsystemID: u16,
    pub expansionROMBaseAddr: u32,
    pub capabilitiesPtr: u8,
    pub reserved1: u8,
    pub reserved2: u16,
    pub reserved3: u32,
    pub interruptLine: u8,
    pub interruptPIN: u8,
    pub mpPinAssignment: u8,
    pub mpIntinAssignment: u8,
    pub minGrant: u8,
    pub maxLatency: u8,
}

impl PCI00DeviceInfo {
    fn new (bus: u8, slot: u8) -> PCI00DeviceInfo {
        let header = PCIDeviceHeader::new(bus, slot);
        let mut device = PCI00DeviceInfo::default();
        device.header = header;
        device.readInDevice();
        
        device
    }

    fn readInDevice(&mut self) {
        self.header.readInHeader();
        let bus = self.header.bus;
        let slot = self.header.slot;

        for i in 0..6 {
            self.baseAddr[i] = configRead32(bus, slot, 0, (16 + 4 * i) as u8);
        }
		self.cardBusCISPtr = configRead32(bus, slot, 0, 0x28);
		self.subsystemVendorID = configRead16(bus, slot, 0, 0x2c);
		self.subsystemID = configRead16(bus, slot, 0, 0x2e);
		self.expansionROMBaseAddr = configRead32(bus, slot, 0, 0x30);
		self.capabilitiesPtr = configRead8(bus, slot, 0, 0x34);
		self.reserved1 = configRead8(bus, slot, 0, 0x35);
		self.reserved2 = configRead16(bus, slot, 0, 0x36);
		self.reserved3 = configRead32(bus, slot, 0, 0x38);
		self.interruptLine = configRead8(bus, slot, 0, 0x3c);
		self.interruptPIN = configRead8(bus, slot, 0, 0x3d);
		self.minGrant = configRead8(bus, slot, 0, 0x3e);
		self.maxLatency = configRead8(bus, slot, 0, 0x3f);
    }

    fn getAddressSpaceSize(&self, barNum: usize) -> u32 {
        let mut sz = 0;

		// if invalid BAR, return sz 0
		if self.baseAddr[barNum] == 0 {
            return 0;
        }
		// write all 1s to the BAR
		configWrite32(self.header.bus, self.header.slot, 0, (16 + 4 * barNum) as u8, 0xFFFFFFFF);
		sz = configRead32(self.header.bus, self.header.slot, 0, (16 + 4 * barNum) as u8);
		// mask info bits
		if (self.baseAddr[barNum] & 1) == 1 {
            sz &= 0xFFFFFFFC;
        } else {
            sz &= 0xFFFFFFF0;
        }

		sz = !sz;
		sz += 1;

		// write back original BAR value
		configWrite32(self.header.bus, self.header.slot, 0, (16 + 4 * barNum) as u8, self.baseAddr[barNum]);

		sz
    }

    fn printHeader(&self) {
        self.header.printHeader();
		println!("Specific info for header 00:");
		for i in 0..6 {
            println!("Base Address {}         : 0x{}", i, self.baseAddr[i]);
        }
		println!("Cardbus CIS Ptr        : 0x{}", self.cardBusCISPtr);
		println!("Subsystem ID           : 0x{}", self.subsystemID);
		println!("Subsystem Vendor ID    : 0x{}", self.subsystemVendorID);
		println!("Expansion ROM Base Addr: 0x{}", self.expansionROMBaseAddr);
		println!("Capabilities PTR       : 0x{}", self.capabilitiesPtr);
		println!("Reserved  8bits        : 0x{}", self.reserved1);
		println!("Reserved 16bits        : 0x{}", self.reserved2);
		println!("Reserved 32bits        : 0x{}", self.reserved3);
		println!("Interrupt PIN          : 0x{}", self.interruptPIN);
		println!("Interrupt Line         : 0x{}", self.interruptLine);
		println!("Interrupt PIN by MP    : 0x{}", self.mpPinAssignment);
		println!("Intin by MP            : 0x{}", self.mpIntinAssignment);
		println!("Min Grant              :   {}", self.minGrant);
		println!("Max Latency            :   {}", self.maxLatency);
    }

    /*
     * too lazy to implement this
    fn printBARsVerbose(&self) {
        for i in 0..6 {
			// if this BAR is 0, print it does not exist
			if self.baseAddr[i] == 0 {
                println!("Base Address Register {} is null", i);
            } else {
				println!("Base Address Register {} is a ", i);
				if ((self.baseAddr[i] & 1) > 0) {
					println!("I/O Space BAR: 0x{}",
						   (self.baseAddr[i] & 0xFFFFFFFC));
                    println!("    Its address space is size: 0x{} bytes",
						   getAddressSpaceSize(i));
				} else {
					println!("Memory Space BAR ");
					if (((self.baseAddr[i] >> 3) & 1) > 1 {
                        println!("that is prefetchable with register size ");
					else
                        println!("that is not prefetchable with register size ");

					if (((self.baseAddr[i] >> 1) & 3) == 0) {
						println!("32 bits: 0x{}", (self.baseAddr[i] & 0xFFFFFFF0));
						println!("    Its address space is size: 0x{} bytes",
							   getAddressSpaceSize(i));
					} else if (((self.baseAddr[i] >> 1) & 3) == 3)
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

    PCIDevice(PCI00DeviceInfo &info) : intin(info.mpIntinAssignment)
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

fn configRead8(bus: u8, slot: u8, func: u8, offset: u8) -> u8 {
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
fn configWrite8(bus: u8, slot: u8, func: u8, offset: u8, data: u8) {
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

fn configRead16(bus: u8, slot: u8, func: u8, offset: u8) -> u16 {
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
fn configWrite16(bus: u8, slot: u8, func: u8, offset: u8, data: u16) {
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
        to_write |= configRead16(bus, slot, func, (offset & 0xfc) as u8) as u32;
    }
    // if writing to lower half of a word, get upper half of word and build toWrite
    else {
        to_write |= ((configRead16(bus, slot, func, ((offset & 0xfc) + 2) as u8) as u32) << 16) as u32;
    }
    unsafe {
        // set address we want to write to
        machine::outl(CONFIG_ADDRESS, address);
        // write data now that address is specified
        machine::outl(CONFIG_DATA, to_write);
    }
}

fn configRead32(bus: u8, slot: u8, func: u8, offset: u8) -> u32 {
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
fn configWrite32(bus: u8, slot: u8, func: u8, offset: u8, data: u32) {
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

fn checkVendor(bus: u8, slot: u8) -> u16 {
    // try and read first configuration register
    // if we get 0xFFFF then we know that the slot has no device since no device has vendor id 0xFFFF
    let vendor: u16 = configRead16(bus, slot, 0, 0);
    if vendor != 0xFFFF {
        // the slot has a device that exists
        let device: u16 = configRead16(bus, slot, 0, 2);

        println!("The bus {} has slot {} with device {} and vendor {}", bus,
               slot, device, vendor);
        let tmp: Option<PCI00DeviceInfo> = get00DeviceInfo(bus, slot);
        if let Some(dev) = tmp {
            dev.printHeader();
            println!("\n");
        }
    }

    vendor
}

pub fn checkAllBuses() {
    for bus in 0..256 {
        for slot in 0..32 {
            checkVendor(bus as u8, slot as u8);
        }
    }
}

/*
void loadDevice(u8 bus, u8 slot);
void loadAllPCI00Devices();
*/

// returns an Option: Some with a pci device with header 00 if found, None otherwise
fn get00DeviceInfo(bus: u8, slot: u8) -> Option<PCI00DeviceInfo> {
    let headerType: u8 = (configRead16(bus, slot, 0, 14) & 0xFF) as u8;
    // if headerType00, return the more specific object
    if (headerType & 3) == 0 {
        Some(PCI00DeviceInfo::new(bus, slot))
    } else {
        None
    }
}

/*
PCI00DeviceInfo *find00Device(u16 deviceID, u16 vendorID);
PCI00DeviceInfo *get00Device(u8 bus, u8 slot);

void registerIrq(u8 intin);
*/