fn main() {
    
    cc::Build::new()
    .file("src/boot2.S")
    .flag("-c")
    .compile("boot2.o");

    cc::Build::new()
    .file("src/machine.S")
    .flag("-c")
    .compile("machine.o");


    cc::Build::new()
    .object("src/longmodeNasm.o")
    .compile("bootstrap.o");

}