provision:
    vagrant up

list-machines:
    @echo lin self

# access da machine
access machine +CMD:
    #!/usr/bin/env fish
    switch (machine)
    case lin:
        vagrant ssh lin -c "{{CMD}}"

# debug-install:
#     which cargo-pando &>/dev/null || cargo install --debug --path .

# vagrant target:
#     vagrant ssh -c "cd /vagrant && just {{target}}"

# self-test:
#     cargo pando test --install

# sync:
#     vagrant rsync

# tctest:
#     cargo --version

# env:
#     env
