all: longmodeNasm.o boot.o

.PHONY: longmodeNasm.o boot.o

longmodeNasm.o:
	nasm longmodeNasm.S -o longmodeNasm.o -felf64

boot.o:
	nasm boot.S -o boot.o -felf64

clean:
	rm -rf *.o

