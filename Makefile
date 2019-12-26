all: iso build

.phony: iso build

build:
	make -C src
	cargo xbuild

clean:
	make -C src clean
	cargo clean
	rm -rf isodir
	rm -f oxos.iso

iso: build
	mkdir -p isodir/boot/grub
	cp target/x86_64-oxos/debug/oxos isodir/boot/oxos.bin
	cp grub.cfg isodir/boot/grub/grub.cfg
	grub-mkrescue -o oxos.iso isodir

run: iso build
	qemu-system-x86_64 -smp 2 -cdrom oxos.iso -nographic --monitor none