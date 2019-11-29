.intel_syntax noprefix


    mov edi, 0x1000    // Set the destination index to 0x1000.
    mov cr3, edi       // Set control register 3 to the destination index.
    xor eax, eax       // Nullify the A-register.
    mov ecx, 4096      // Set the C-register to 4096.
    rep stosd          // Clear the memory.
    mov edi, cr3       // Set the destination index to control register 3.

    movq [edi], 0x2003      // Set the uint32_t at the destination index to 0x2003.
    add edi, 0x1000              // Add 0x1000 to the destination index.
    movq [edi], 0x3003      // Set the uint32_t at the destination index to 0x3003.
    add edi, 0x1000              // Add 0x1000 to the destination index.
    movq [edi], 0x4003      // Set the uint32_t at the destination index to 0x4003.
    add edi, 0x1000              // Add 0x1000 to the destination index.

    mov ebx, 0x00000003          // Set the B-register to 0x00000003.
    mov ecx, 512                 // Set the C-register to 512.
 
.SetEntry:
    mov .wordORD [edi], ebx         // Set the uint32_t at the destination index to the B-register.
    add ebx, 0x1000              // Add 0x1000 to the B-register.
    add edi, 8                   // Add eight to the destination index.
    loop .SetEntry               // Set the next entry.

    mov eax, cr4                 // Set the A-register to control register 4.
    or eax, 1 << 5               // Set the PAE-bit, which is the 6th bit (bit 5).
    mov cr4, eax                 // Set control register 4 to the A-register.

    mov ecx, 0xC0000080          // Set the C-register to 0xC0000080, which is the EFER MSR.
    rdmsr                        // Read from the model-specific register.
    or eax, 1 << 8               // Set the LM-bit which is the 9th bit (bit 8).
    wrmsr                        // Write to the model-specific register.

    mov eax, cr0                 // Set the A-register to control register 0.
    or eax, 1 << 31              // Set the PG-bit, which is the 32nd bit (bit 31).
    mov cr0, eax                 // Set control register 0 to the A-register.

GDT64:                           // Global Descriptor Table (64-bit).
    GDT64Null = $ - GDT64         // The null descriptor.
    .word 0xFFFF                    // Limit (low).
    .word 0                         // Base (low).
    .byte 0                         // Base (middle)
    .byte 0                         // Access.
    .byte 1                         // Granularity.
    .byte 0                         // Base (high).
    GDT64Code = $ - GDT64         // The code descriptor.
    .word 0                         // Limit (low).
    .word 0                         // Base (low).
    .byte 0                         // Base (middle)
    .byte 0b10011010               // Access (exec/read).
    .byte 0b10101111                // Granularity, 64 bits flag, limit19:16.
    .byte 0                         // Base (high).
    GDT64Data = $ - GDT64         // The data descriptor.
    .word 0                         // Limit (low).
    .word 0                         // Base (low).
    .byte 0                         // Base (middle)
    .byte 0b10010010                 // Access (read/write).
    .byte 0b00000000                // Granularity.
    .byte 0                         // Base (high).
    GDT64Pointer:                    // The GDT-pointer.
    .word $ - GDT64 - 1             // Limit.
    .long GDT64                     // Base.

    lgdt [GDT64Pointer]         // Load the 64-bit global descriptor table.
    .att_syntax
    jmp $GDT64Code, $Realm64       // Set the code segment and enter 64-bit long mode.
    .intel_syntax noprefix

// Use 64-bit.
.code64
 
Realm64:
    cli                           // Clear the interrupt flag.
    mov ax, GDT64.Data            // Set the A-register to the data descriptor.
    mov ds, ax                    // Set the data segment to the A-register.
    mov es, ax                    // Set the extra segment to the A-register.
    mov fs, ax                    // Set the F-segment to the A-register.
    mov gs, ax                    // Set the G-segment to the A-register.
    mov ss, ax                    // Set the stack segment to the A-register.
    mov edi, 0xB8000              // Set the destination index to 0xB8000.
    mov rax, 0x1F201F201F201F20   // Set the A-register to 0x1F201F201F201F20.
    mov ecx, 500                  // Set the C-register to 500.
    rep stosq                     // Clear the screen.
    hlt                           // Halt the processor.