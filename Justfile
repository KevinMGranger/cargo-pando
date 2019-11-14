debug-install:
    which cargo-pando &>/dev/null || cargo install --debug --path .

vagrant target:
    vagrant ssh -c "cd /vagrant && just {{target}}"

self-test:
    cargo pando test --install

sync:
    vagrant rsync