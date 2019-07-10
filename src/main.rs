extern crate libusb;
extern crate notify_rust;
extern crate psutil;
extern crate signal_hook;
extern crate crossbeam_channel;

pub mod online;

use notify_rust::Notification;
use psutil::process;
use online::*;
use crossbeam_channel::{bounded, tick, Receiver, select};
use signal_hook::{iterator::Signals, SIGCONT, SIGTRAP};
use std::{process::Command, time::Duration, error::Error, thread};

static PHONE_CONNECTED: &'static str = "smartphonetrusted";
static PHONE_DISCONNECTED: &'static str = "smartphonedisconnected";

fn sleep(dur: u64) {
    thread::sleep(Duration::from_millis(dur));
}

fn show_notification(msg: &'static str) {
    thread::spawn(move || {
        Notification::new()
            .summary("Tether Manager")
            .body(msg)
            .icon(match online(None) {
                Ok(t) => match t {
                    true => PHONE_CONNECTED,
                    false => PHONE_DISCONNECTED
                },
                Err(_) => PHONE_DISCONNECTED
            })
            .show().unwrap();
    });         
}

fn start_openvpn() {
    Command::new("sudo")
        .arg("/home/meet/.config/android_tether/start_ovpn.sh")
        .spawn()
        .expect("Failed to start OpenVPN");
    sleep(1000);
}

fn kill_openvpn() {
    Command::new("sudo")
        .arg("/home/meet/.config/android_tether/end_ovpn.sh")
        .spawn()
        .expect("Failed to start OpenVPN");
    sleep(1000);
}

fn restart_openvpn() {
    kill_openvpn();
    start_openvpn();
}

fn openvpn_running() -> bool {
    if let Ok(processes) = process::all() {
        for p in &processes {
            if p.comm == "openvpn" {
                return true;
            }
        }
    }
    false
}

fn signal_hook() -> Result<(Receiver<()>, Receiver<()>), Box<Error>> {
    let (sender_restart, receiver_restart) = bounded(100);
    let (sender_toggle, receiver_toggle) = bounded(100);
    let signals = Signals::new(&[SIGTRAP, SIGCONT])?;
    thread::spawn(move || {
        for sig in signals.forever() {
            let _ = match sig {
                SIGCONT => sender_restart.send(()),
                SIGTRAP => sender_toggle.send(()),
                _ => Ok(())
            };
        }
    });
    Ok((receiver_restart, receiver_toggle))
}

fn online_notification(timeout_millis: u64) {
    if online(Some(Duration::from_millis(timeout_millis))).unwrap() { 
        show_notification("Connected");
    }
}

fn start_timer(context: &libusb::Context, phone_vendor_id: u16, phone_product_id: u16, phone_product_id_hid: u16) -> Result<(), Box<Error>> {
    let mut phone_unplugged = true;
    let mut cur_hid = 0;
    let mut paused = false;
    let (restart_event, toggle_event) = signal_hook()?;
    let ticks = tick(Duration::from_secs(4));

    if openvpn_running() {
        kill_openvpn();
    }
    
    loop {
        select! {
            recv(restart_event) -> _ => {
                if !paused {
                    show_notification("Restarting OpenVPN");
                    restart_openvpn();
                    online_notification(1000);
                }
            }
            recv(toggle_event) -> _ => {
                paused = !paused;
                if paused {
                    kill_openvpn();
                    show_notification("Pausing");
                } else {
                    show_notification("Resuming");
                    start_openvpn();
                    online_notification(1000);
                }
            }
            recv(ticks) -> _ => {
                if !paused {
                    let mut openvpn_killed = !openvpn_running();
                    for mut device in context.devices().unwrap().iter() {
                        let device_desc = device.device_descriptor().unwrap();
                        if device_desc.vendor_id() == phone_vendor_id && (device_desc.product_id() == phone_product_id_hid || device_desc.product_id() == phone_product_id) { // phone is plugged in
                            if cur_hid == 0 {
                                cur_hid = device_desc.product_id();
                            } else {
                                if cur_hid != device_desc.product_id() && !openvpn_killed { // handle case where user allows sharing data with pc
                                    restart_openvpn();
                                    cur_hid = device_desc.product_id();
                                }
                            }

                            if openvpn_killed {
                                start_openvpn();
                                openvpn_killed = false;
                                online_notification(10000)
                            }
                            
                            phone_unplugged = false;
                            break;
                        } else {
                            phone_unplugged = true;
                        }
                    }

                    if phone_unplugged && !openvpn_killed {
                        kill_openvpn();
                        show_notification("Disconnected");
                        openvpn_killed = true;
                    }

                    if !phone_unplugged && !openvpn_killed && !online(None).unwrap() {
                        restart_openvpn();
                        show_notification("Reconnected");
                    }
                }
            }
        }
    }
    Ok(())
}

fn main() -> Result<(), Box<Error>> {
    let phone_vendor_id = 0x0b05;
    let phone_product_id = 0x7770;
    let phone_product_id_hid = 0x7773;
    let mut context = libusb::Context::new().unwrap();
 
    start_timer(&context, phone_vendor_id, phone_product_id, phone_product_id_hid)?;
    Ok(())
}