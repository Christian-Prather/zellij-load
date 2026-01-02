[group("build")]
build-all:
    just build-daemon
    just build-wasm

[group("build")]
build-daemon:
    cargo build --release --bin zellij_system_monitor --features native

[group("build")]
build-wasm:
    cargo build --target wasm32-wasip1 --release --bin plugin --features plugin

[group("run")]
run-daemon:
    cargo run --release --bin zellij_system_monitor --features native

[group("run")]
run-debug:
    cargo build --target wasm32-wasip1 --bin plugin --features plugin
    zellij action start-or-reload-plugin file:target/wasm32-wasip1/debug/plugin.wasm

[group("run")]
run:
    just build-all
    zellij action start-or-reload-plugin file:target/wasm32-wasip1/release/plugin.wasm

[group("install")]
install:
    just build-all
    cargo install --path . --bin zellij_system_monitor --features native

[group("install")]
uninstall:
    cargo uninstall --bin zellij_system_monitor
