default: (run "info" "info")

set dotenv-required

run wawaloglevel globalloglevel:
    @echo 'Running with levels {{wawaloglevel}} and {{globalloglevel}}. They should be "trace", "debug", "info" or "error"'
    RUST_LOG=wawa={{wawaloglevel}},{{globalloglevel}} ./wawa

transfer:
    cargo build --release
    patchelf --set-interpreter /usr/lib64/ld-linux-x86-64.so.2 target/release/wawa
    ssh $WAWA_SRV "systemctl stop wawa"
    scp target/release/wawa $WAWA_SRV_DEST 
    ssh $WAWA_SRV "systemctl start wawa"

update:
    cargo update
    just transfer
