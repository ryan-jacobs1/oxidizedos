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