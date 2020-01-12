all: iso build

.phony: iso build

build:
	mkdir BUILD_FILES
	cargo xbuild

clean:
	cargo clean
	rm -rf isodir
	rm -rf BUILD_FILES/*.o
	rm -f oxos.iso

iso: build
	mkdir -p isodir/boot/grub
	cp target/x86_64-oxos/debug/oxos isodir/boot/oxos.bin
	cp grub.cfg isodir/boot/grub/grub.cfg
	grub-mkrescue -o oxos.iso isodir

run: iso build
	qemu-system-x86_64 -smp 4 -cdrom oxos.iso -nographic --monitor none