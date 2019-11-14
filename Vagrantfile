# -*- mode: ruby -*-
# vi: set ft=ruby :

Vagrant.configure("2") do |config|
  config.vm.box = "centos/7"

  config.vm.provider "virtualbox" do |v|
    v.cpus = 2
  end

  config.vm.provision "shell", privileged: false, inline: <<-'SHELL'
    set -o errexit
    if [[ ! -a ~/.cargo/env ]]; then
      curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal
    fi
    source ~/.cargo/env
    sudo yum install -y gcc openssl-devel # openssl-libs
    which just &>/dev/null || cargo install just
  SHELL

  config.vm.synced_folder ".", "/vagrant", type: "rsync",
    rsync__exclude: [".git", "target"]
end
