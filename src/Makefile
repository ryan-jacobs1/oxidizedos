all: longmode.o boot.o

.PHONY: longmode.o boot.o

longmode.o:
	nasm init/longmode.S -o longmode.o -felf64

boot.o:
	nasm init/boot.S -o boot.o -felf64

clean:
	rm -rf *.o

