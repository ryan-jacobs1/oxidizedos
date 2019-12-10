fn main() {
    
    
    cc::Build::new()
    .file("src/init/multiboot.S")
    .flag("-c")
    .compile("multiboot.o");
    

    cc::Build::new()
    .file("src/machine.S")
    .flag("-c")
    .compile("machine.o");

    cc::Build::new()
    .object("src/boot.o")
    .compile("boot.o");
    
    cc::Build::new()
    .object("src/longmode.o")
    .compile("bootstrap.o");
    
}