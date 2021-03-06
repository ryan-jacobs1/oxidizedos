[BITS 16]
section .apentry
global ap_entry
ap_entry:
cli
lgdt [trampoline_gdtDesc]
mov eax, cr0
or eax, 1
mov cr0, eax
jmp 0x08:appm


[BITS 32]
appm:
    ; Immediately setup stack, gdt, and paging
    lgdt [GDT64.Pointer]
    extern stack_top
    mov esp, stack_top
    mov eax, pml4
    mov cr3, eax

    ; Enable long mode
    mov eax, cr4                 
    or eax, 1 << 5               
    mov cr4, eax                 

    mov ecx, 0xC0000080          
    rdmsr                        
    or eax, 1 << 8               
    wrmsr    

    mov eax, cr0                
    or eax, 1 << 31              
    mov cr0, eax
    jmp GDT64.Code:APLongMode         


trampoline_gdt:
    
    dq 0

; __KERNEL_CS
	dw 0xFFFF
	dw 0
	db 0
	db 0b10011010
	db 0b11001111
	db 0

	; __KNLUSR_DS
	dw 0xFFFF
	dw 0
	db 0
	db 0b10010010
	db 0b11001111
	db 0
trampoline_gdtDesc:
	dw (trampoline_gdtDesc - trampoline_gdt) - 1
	dq trampoline_gdt















[BITS 32]
section .text
global longmode
longmode:
    extern pml4
    extern pdpt
    mov edi, pml4    ; Set the destination index to 0x1000.
    mov cr3, edi       ; Set control register 3 to the destination index.
    xor eax, eax       ; Nullify the A-register.
    mov ecx, 4096      ; Set the C-register to 4096.
    rep stosd          ; Clear the memory.
    mov edi, cr3       ; Set the destination index to control register 3.

    mov DWORD [edi], (pdpt + 0x3)      ; Set the uint32_t at the destination index to 0x2003.
 
    ; Identity map the entire 32 bit address space using huge pages
    mov edi, pdpt
    mov DWORD [edi], (0x3 | (1 << 7))
    mov DWORD [edi + 4], 0
    mov DWORD [edi + 8], (0x3 | (1 << 7))
    mov DWORD [edi + 12], 1
    mov DWORD [edi + 16], (0x3 | (1 << 7))
    mov DWORD [edi + 20], 2
    mov DWORD [edi + 24], (0x3 | (1 << 7))
    mov DWORD [edi + 28], 3

 

    mov eax, cr4                 ; Set the A-register to control register 4.
    or eax, 1 << 5               ; Set the PAE-bit, which is the 6th bit (bit 5).
    mov cr4, eax                 ; Set control register 4 to the A-register.

    mov ecx, 0xC0000080          ; Set the C-register to 0xC0000080, which is the EFER MSR.
    rdmsr                        ; Read from the model-specific register.
    or eax, 1 << 8               ; Set the LM-bit which is the 9th bit (bit 8).
    wrmsr    

    mov eax, cr0                 ; Set the A-register to control register 0.
    or eax, 1 << 31              ; Set the PG-bit, which is the 32nd bit (bit 31).
    mov cr0, eax                 ; Set control register 0 to the A-register.



    lgdt [GDT64.Pointer]         ; Load the 64-bit global descriptor table.
    jmp GDT64.Code:Realm64       ; Set the code segment and enter 64-bit long mode.

    GDT64:                           ; Global Descriptor Table (64-bit).
    .Null: equ $ - GDT64         ; The null descriptor.
    dw 0xFFFF                    ; Limit (low).
    dw 0                         ; Base (low).
    db 0                         ; Base (middle)
    db 0                         ; Access.
    db 1                         ; Granularity.
    db 0                         ; Base (high).
    .Code: equ $ - GDT64         ; The code descriptor.
    dw 0                         ; Limit (low).
    dw 0                         ; Base (low).
    db 0                         ; Base (middle)
    db 10011010b                 ; Access (exec/read).
    db 10101111b                 ; Granularity, 64 bits flag, limit19:16.
    db 0                         ; Base (high).
    .Data: equ $ - GDT64         ; The data descriptor.
    dw 0                         ; Limit (low).
    dw 0                         ; Base (low).
    db 0                         ; Base (middle)
    db 10010010b                 ; Access (read/write).
    db 00000000b                 ; Granularity.
    db 0                         ; Base (high).
    .Pointer:                    ; The GDT-pointer.
    dw $ - GDT64 - 1             ; Limit.
    dq GDT64                     ; Base.

    

; Use 64-bit.
[BITS 64]
 
Realm64:
    cli                           ; Clear the interrupt flag.
    mov ax, GDT64.Data            ; Set the A-register to the data descriptor.
    mov ds, ax                    ; Set the data segment to the A-register.
    mov es, ax                    ; Set the extra segment to the A-register.
    mov fs, ax                    ; Set the F-segment to the A-register.
    mov gs, ax                    ; Set the G-segment to the A-register.
    mov ss, ax                    ; Set the stack segment to the A-register.
    mov edi, 0xB8000              ; Set the destination index to 0xB8000.
    mov rax, 0x1F201F201F201F20   ; Set the A-register to 0x1F201F201F201F20.
    mov ecx, 500                  ; Set the C-register to 500.
    ;rep stosq                      Clear the screen.
    ; Switch to a new stack
    extern pick_stack
    call pick_stack
    extern multiboot_config
    mov edi, [multiboot_config]
    add edi, 8
    mov rsp, rax
    extern KERNEL_END
    mov rsi, KERNEL_END
    extern _start
    call _start
    hlt                           ; Halt the processor.

APLongMode:
    extern _ap_start
    extern ap_pick_stack
    call ap_pick_stack
    mov rsp, rax
    call _ap_start
    hlt