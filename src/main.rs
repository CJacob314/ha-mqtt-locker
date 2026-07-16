use envconfig::Envconfig;
use rumqttc::{Client, ConnectReturnCode, Event, Incoming, LastWill, MqttOptions, QoS};
use serde_json::json;
use std::{
    process::{Command, Stdio},
    sync::mpsc::{self, TrySendError},
    thread,
    time::Duration,
};

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
    let Config {
        lock_prog,
        lock_prog_args,
        mqtt_broker_host,
        mqtt_broker_port,
        mqtt_broker_username,
        mqtt_broker_password,
        home_assistant_area,
    } = Config::init_from_env().unwrap();

    // Sync channel with no buffer since the worker thread will either be available -> we lock,
    // or unavailable -> we ignore the lock command
    let (tx, rx) = mpsc::sync_channel::<()>(0);

    thread::scope(move |scope| {
        // Spawn worker thread (which will fork to run the lock program and wait on that child)
        scope.spawn(move || {
            while rx.recv().is_ok() {
                lock(&lock_prog, &lock_prog_args)
            }
        });

        let mut mqtt_options = MqttOptions::new("mqtt-locker", &mqtt_broker_host, mqtt_broker_port);
        mqtt_options.set_keep_alive(Duration::from_secs(30));
        mqtt_options.set_credentials(&mqtt_broker_username, &mqtt_broker_password);
        mqtt_options.set_last_will(LastWill::new("desktop/lock/availability", "offline", QoS::AtMostOnce, true));

        let (client, mut connection) = Client::new(mqtt_options, 10);

        for notification in connection.iter() {
            match notification {
                Ok(Event::Incoming(Incoming::ConnAck(conn_ack))) => {
                    if conn_ack.code != ConnectReturnCode::Success {
                        panic!("ConnAck packet had a failure code: {:?}", conn_ack.code);
                    }

                    /* Successful connect. Subscribe to receive LOCK commands and advertise discovery. */
                    client
                        .subscribe("desktop/lock/set", QoS::AtMostOnce)
                        .unwrap();
                    client
                        .publish(
                            "homeassistant/button/desktop_lock/config",
                            QoS::AtMostOnce,
                            true,
                            discovery_payload(&home_assistant_area),
                        )
                        .unwrap();
                    client
                        .publish("desktop/lock/availability", QoS::AtMostOnce, true, "online")
                        .unwrap();
                }
                Ok(Event::Incoming(Incoming::Publish(publish))) => {
                    if publish.topic != "desktop/lock/set" {
                        continue;
                    }

                    let payload = std::str::from_utf8(&publish.payload).unwrap_or("").trim();

                    if payload == "LOCK" {
                        // Attempt to tell the worker thread to lock
                        match tx.try_send(()) {
                            Ok(()) => (), // worker thread successfully received message
                            Err(TrySendError::Full(())) => {
                                eprintln!("Ignoring LOCK command: computer already locked")
                            }
                            Err(TrySendError::Disconnected(())) => panic!(
                                "ERROR: Worker thread unexpectedly disconnected from sync channel"
                            ),
                        }
                    }
                }
                Ok(_) => (), // Not currently handling anything else
                Err(err) => {
                    eprintln!("MQTT error: {err}");
                }
            }
        }
    });
}

/// Spawn the `LOCK_PROG` using `LOCK_PROG_ARGS` as a child process and await its exit
fn lock(lock_prog: &str, lock_prog_args: &str) {
    println!("Received LOCK command: locking computer.");
    Command::new(lock_prog)
        .args(lock_prog_args.split_whitespace())
        .stdout(Stdio::null())
        .stdin(Stdio::null())
        .spawn()
        .unwrap()
        .wait()
        .unwrap();
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
