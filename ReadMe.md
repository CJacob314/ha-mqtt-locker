# ha-mqtt-locker

`ha-mqtt-locker` is a small Rust service that lets Home Assistant lock a desktop over MQTT.

It exposes a Home Assistant MQTT `button` entity named `Lock Desktop`. When the
button is pressed, the service runs a configurable lock command. By default,
that command is `swaylock`.

## How It Works

The service connects to your MQTT broker, publishes a retained Home Assistant
discovery payload, and listens for commands on an MQTT topic.

This creates a Home Assistant *button*, **not a lock**. A Home Assistant lock
entity expects lock **and unlock** semantics, and `ha-mqtt-locker` intentionally only performs a lock.

## Requirements
- Rust and Cargo
- A running MQTT broker reachable from this machine
- Home Assistant with the MQTT integration enabled
- Systemd
  - I spawn the configured locking program using `systemd-run` to make sure environment variables have correct values, like `WAYLAND_DISPLAY` for instance.
- Your locking program must not daemonize itself on a fork-child and let the parent exit
  - That is, the spawned locking program must exit only after the display has been unlocked, not once the display has been locked

## Quick Install

Run `./install.sh` from the repo root.

This will:
- run `cargo install --path .`
- create config directories
- install the systemd user service to `~/.config/systemd/user`
- enable the service

You'll want to edit the environment file `~/.config/mqtt-locker/env` (or directly edit the systemd service file) before starting the service or restarting your computer.
