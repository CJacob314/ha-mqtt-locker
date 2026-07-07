use std::{process::{Command, Stdio}, time::Duration};
use envconfig::Envconfig;
use rumqttc::{Client, Event, Incoming, MqttOptions, QoS};
use serde_json::json;

/// Stores config options as set in environment variables
#[derive(Envconfig)]
struct Config {
    #[envconfig(from = "LOCK_PROG", default = "swaylock")]
    lock_prog: String,

    #[envconfig(from = "LOCK_PROG_ARGS", default = "")]
    lock_prog_args: String,

    #[envconfig(from = "MQTT_BROKER_HOST", default = "192.168.0.10")]
    mqtt_broker_host: String,

    #[envconfig(from = "MQTT_BROKER_PORT", default = "1883")]
    mqtt_broker_port: u16,

    #[envconfig(from = "MQTT_BROKER_USERNAME", default = "")]
    mqtt_broker_username: String,

    #[envconfig(from = "MQTT_BROKER_PASSWORD", default = "")]
    mqtt_broker_password: String,

    #[envconfig(from = "HOME_ASSISTANT_AREA", default = "")]
    home_assistant_area: String,
}

fn main() {
    let Config { lock_prog, lock_prog_args, mqtt_broker_host, mqtt_broker_port, mqtt_broker_username, mqtt_broker_password, home_assistant_area } = Config::init_from_env().unwrap();

    let mut mqtt_options = MqttOptions::new("mqtt-locker", &mqtt_broker_host, mqtt_broker_port);
    mqtt_options.set_keep_alive(Duration::from_secs(5));
    mqtt_options.set_credentials(&mqtt_broker_username, &mqtt_broker_password);

    let (mut client, mut connection) = Client::new(mqtt_options, 10);
    client.subscribe("desktop/lock/set", QoS::AtMostOnce).unwrap();
    client.publish("homeassistant/button/desktop_lock/config", QoS::AtMostOnce, true, discovery_payload(&home_assistant_area)).unwrap();
    client.publish("desktop/lock/availability", QoS::AtMostOnce, true, "online").unwrap();

    for notification in connection.iter() {
        match notification {
            Ok(Event::Incoming(Incoming::Publish(publish))) => {
                if publish.topic != "desktop/lock/set" {
                    continue;
                }

                let payload = std::str::from_utf8(&publish.payload).unwrap_or("").trim();

                if payload == "LOCK" {
                    lock(&lock_prog, &lock_prog_args)
                }
            },
            Ok(_) => (), // Not currently handling anything else
            Err(err) => {
                eprintln!("MQTT error: {err}");
            }
        }
    }
}

/// Spawn the `LOCK_PROG` using `LOCK_PROG_ARGS` as a child process and await its exit
fn lock(lock_prog: &str, lock_prog_args: &str) {
    Command::new(lock_prog)
        .args(lock_prog_args.split_whitespace())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .stdin(Stdio::null())
        .spawn().unwrap().wait().unwrap();
}

fn discovery_payload(home_assistant_area: &str) -> Vec<u8> {
    serde_json::to_vec(&json!({
        "unique_id": "desktop_lock_button",
        "name": "Lock Desktop",
        "command_topic": "desktop/lock/set",
        "payload_press": "LOCK",
        "availability_topic": "desktop/lock/availability",
        "device": {
            "identifiers": ["desktop"],
            "name": "Desktop",
            "model": "Rust bridge",
            "suggested_area": home_assistant_area,
        }
    }))
    .unwrap()
}
