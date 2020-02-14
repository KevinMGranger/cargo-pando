# -*- mode: ruby -*-
# vi: set ft=ruby :

Vagrant.configure("2") do |config|
  config.vm.provider "virtualbox" do |v|
    v.cpus = 2
  end

  config.vm.define "lin" do |lin|
    lin.vm.box = "centos/7"
    lin.vm.provision "shell", privileged: false, inline: <<-'SHELL'
      set -o errexit
      if [[ ! -a ~/.cargo/env ]]; then
        curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal
      fi
      source ~/.cargo/env
      sudo yum install -y gcc openssl-devel # openssl-libs
      which just &>/dev/null || cargo install just
    SHELL

    lin.vm.synced_folder ".", "/vagrant", type: "rsync",
      rsync__exclude: [".git", "target"]
  end

  # config.vm.define "bsd" do |bsd|
  #   # this is kinda buggy. PLus, idk what to do about:
  #   # $ fetch https://sh.rustup.rs
  #   # Certificate verification failed for /C=US/ST=Arizona/L=Scottsdale/O=Starfield Technologies, Inc./CN=Starfield Services Root Certificate Authority - G2
  #   # 34370633728:error:1416F086:SSL routines:tls_process_server_certificate:certificate verify failed:/usr/src/crypto/openssl/ssl/statem/statem_clnt.c:1915:
  #   # fetch: https://sh.rustup.rs: Authentication error
  #   bsd.vm.box = "freebsd/FreeBSD-12.1-RELEASE"
  #   bsd.vm.box_version = "2019.11.01"

  #   bsd.vm.synced_folder ".", "/vagrant", type: "rsync",
  #     rsync__exclude: [".git", "target"]
  # end
end
