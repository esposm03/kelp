compile() {
    clang -target x86_64-unknown-linux-gnu "tests/$1.s" -o "target/$1.o" -c
}

compile a
compile b
