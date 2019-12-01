/* The bootloader will look at this image and start execution at the symbol
   designated as the entry point. */
ENTRY(_entry)
 
/* Tell where the various sections of the object files will be put in the final
   kernel image. */
SECTIONS
{
	/* Begin putting sections at 1 MiB, a conventional place for kernels to be
	   loaded at by the bootloader. */
	. = 1M;

	
	.text ALIGN(4096):
	{
		*(.multiboot)
		*(.text1)
	}

	.bsbss ALIGN(4096):
	{
		*(.bsbss)
	}

	.text1 :
	{
		*(.text)
	}
	
	
 
	/* First put the multiboot header, as it is required to be put very early
	   early in the image or the bootloader won't recognize the file format.
	   Next we'll put the .text section. */



 
	/* Read-only data. */
	.rodata ALIGN(4096):
	{
		*(.rodata)
	}
 
	/* Read-write data (initialized) */
	.data ALIGN(4096):
	{
		*(.data)
	}
 
	/* Read-write data (uninitialized) and stack */
	.bss ALIGN(4096):
	{
		*(COMMON)
		*(.bss)
	}
 
	/* The compiler may produce other sections, by default it will put them in
	   a segment with the same name. Simply add stuff here as needed. */
}