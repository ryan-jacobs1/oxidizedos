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

	# hlt()
	.global hlt
hlt:
	hlt
	ret


.intel_syntax noprefix

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

.global software_int
software_int:
	int 0xff
	ret