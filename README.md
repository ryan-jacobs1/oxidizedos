# OxidizedOS


Dependencies:  
rust nightly (rustup override add nightly)  
cargo xbuild (Thanks to Philipp Oppermann) (cargo install cargo-xbuild)  
qemu  
GRUB 2 (sudo apt install grub-pc-bin)
xorriso  (sudo apt install xorriso)
nasm  
rustfmt-nightly (CFG_RELEASE_CHANNEL=nightly CFG_RELEASE=nightly CARGO_NET_GIT_FETCH_WITH_CLI=true cargo +nightly install --force --branch master --git https://github.com/rust-lang/rustfmt.git --features "rustfmt cargo-fmt")

halogen (cargo install halogen --git https://github.com/ryan-jacobs1/halogen --force)

# Running OxidizedOS  
halogen run  

# To run the tests  
cargo xtest  

# Blog
I blog about the development of OxidizedOS [here](https://ryan-jacobs1.github.io/).
