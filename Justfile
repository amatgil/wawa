default: (run "info" "info")

run wawaloglevel globalloglevel:
    @echo 'Running with levels {{wawaloglevel}} and {{globalloglevel}}. They should be "trace", "debug", "info" or "error"'
    RUST_LOG=wawa={{wawaloglevel}},{{globalloglevel}} ./wawa