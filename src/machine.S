
	
	# outb(int port, int val)
	.global outb
outb:
	mov %rdi, %rdx
	mov %rsi, %rax
	outb %al, %dx
	ret

	# int inb(int port)
	.global inb
inb:
	mov %rdi, %rdx
	inb %dx, %al
	and $0xff, %rax
	ret

	# outw(int port, int val)
	.global outw
outw:
	mov %rdi, %rdx;
	mov %rsi, %rax;
	out %eax, %dx
	ret

	# int inw(int port)
	.global inw
inw:
	mov %rdi, %rdx;
	in %dx, %ax;
	and $0xffff, %rax
	ret

	# outl(int port, int val)
	.global outl
outl:
	mov %rdi, %rdx;
	mov %rsi, %rax;
	outl %eax, %edx
	ret

	# int inl(int port)
	.global inl
inl:
	mov %rdi, %rdx;
	inl %edx, %eax;
	ret

	# hlt()
	.global hlt
hlt:
	hlt
	ret


.intel_syntax noprefix

	.macro SAVE_CALLER_REGS
	push rdi
	push rsi
	push rdx
	push rcx
	push rax
	push r8
	push r9
	push r10
	push r11
	.endm

	.macro RESTORE_CALLER_REGS
	pop r11
	pop r10
	pop r9
	pop r8
	pop rax
	pop rcx
	pop rdx
	pop rsi
	pop rdi
	.endm


# context_switch(current: TCB, next: TCB)
.global context_switch
context_switch:
	push rbx
	push rbp
	push r12
	push r13
	push r14
	push r15

	mov rax, cr2
	push rax
	pushfq

	mov [rdi], rsp
	mov rsp, [rsi]
	cli
	popfq
	pop rax
	mov cr2, rax

	pop r15
	pop r14
	pop r13
	pop r12
	pop rbp
	pop rbx

	ret



.global load_cr3
load_cr3:
	mov cr3, rdi
	ret

.global rdmsr
rdmsr:
	mov ecx, edi
	xor eax, eax
	rdmsr
	shl edx, 32
	or rax, rdx
	ret

.global wrmsr
wrmsr:
	mov ecx, esi  # the msr number
	mov eax, edi  # low 32 bits
	mov rdx, rdi  # need the high 32 bits, so we move all the bits
	shr rdx, 32   # shift the high 32 bits into the correct position
	wrmsr
	ret

.global lidt
lidt:
	lidt [rdi]
	ret

.global spurious_handler
spurious_handler:
	.extern interrupt_test
	call interrupt_test
	iretq

.global _apit_handler
_apit_handler:
	SAVE_CALLER_REGS
	.extern apit_handler
	call apit_handler
	RESTORE_CALLER_REGS
	iretq

.global software_int
software_int:
	int 0xff
	ret

.global cli
cli:
	cli
	ret

.global sti
sti:
	sti
	ret

.global get_flags
get_flags:
	pushfq
	pop rax
	ret

.global get_rsp
get_rsp:
	mov rax, rsp
	ret