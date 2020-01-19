use std::process::{Command, Stdio};

fn main() {
    
    // Remove previous bootstrap asm files
    Command::new("rm")
    .stdout(Stdio::inherit())
    .args(&["-r", "BUILD_FILES"])
    .spawn()
    .expect("Failed to start")
    .wait()
    .expect("Failed to remove BUILD_FILES");
 
    Command::new("mkdir")
    .stdout(Stdio::inherit())
    .args(&["-p", "BUILD_FILES"])
    .spawn()
    .expect("Failed to start")
    .wait()
    .expect("Failed to create directory BUILD_FILES");
    
    /*
    cc::Build::new()
    .file("src/init/multiboot.S")
    .flag("-c")
    .compile("multiboot.o");
    */

    cc::Build::new()
    .file("src/machine.S")
    .flag("-c")
    .compile("machine.o");

    // Compile bootstrap assembly
    Command::new("nasm")
    .stdout(Stdio::inherit())
    .args(&["src/init/longmode.S", "-o", "BUILD_FILES/longmode.o", "-felf64"])
    .spawn()
    .expect("")
    .wait()
    .expect("failed to compile longmode.S");

    Command::new("nasm")
    .stdout(Stdio::inherit())
    .args(&["src/init/boot.S", "-o", "BUILD_FILES/boot.o", "-felf64"])
    .spawn()
    .expect("failed to run nasm")
    .wait()
    .expect("failed to compile boot.S");
    
    cc::Build::new()
    .object("BUILD_FILES/boot.o")
    .compile("boot.o");
    
    cc::Build::new()
    .object("BUILD_FILES/longmode.o")
    .compile("bootstrap.o");
    
    /*
    nasm_rs::compile_library_args("boot.a", &["src/init/boot.S", "src/init/longmode.S"], &["-felf64"]);
    nasm_rs::compile_library_args("longmode.a", &["src/init/longmode.S"], &["-felf64"]);
    */
}