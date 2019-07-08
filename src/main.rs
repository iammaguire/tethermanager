extern crate libusb;
extern crate notify_rust;
extern crate psutil;

pub mod online;

use notify_rust::Notification;
use psutil::process;
use online::*;
use std::process::Command;
use std::time::Duration;
use std::thread;

static PHONE_CONNECTED: &'static str = "smartphonetrusted";
static PHONE_DISCONNECTED: &'static str = "smartphonedisconnected";

enum OpenVPNState {
    CONNECTED,
    RECONNECTING,
    DEAD
}

fn sleep(dur: u64) {
    thread::sleep(Duration::from_millis(dur));
}

fn show_notification(msg: &'static str, icon: &'static str, delay: u64) {
    thread::spawn(move || {
        sleep(delay);
        Notification::new()
            .summary("Tether Manager")
            .body(msg)
            .icon(icon)
            .show().unwrap();
    });         
}

fn start_openvpn() {
    Command::new("sudo")
        .arg("/home/meet/.config/android_tether/start_ovpn.sh")
        .spawn()
        .expect("Failed to start OpenVPN");
    sleep(1000);
    if let Ok(online) = online(None) {
        if online { show_notification("Initiated tether", PHONE_CONNECTED, 0) };
    }
}

fn kill_openvpn() {
    Command::new("sudo")
        .arg("/home/meet/.config/android_tether/end_ovpn.sh")
        .spawn()
        .expect("Failed to start OpenVPN");
    sleep(1000);
    if !openvpn_running() {
        show_notification("Severed tether", PHONE_DISCONNECTED, 0);
    }
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

fn start_timer(context: &libusb::Context, phone_vendor_id: u16, phone_product_id: u16, phone_product_id_hid: u16) {
    let mut phone_unplugged = true;
    let mut cur_hid = 0;
    
    if openvpn_running() {
        kill_openvpn();
    }
    
    loop {
        let mut openvpn_killed = !openvpn_running();
        for mut device in context.devices().unwrap().iter() {
            let device_desc = device.device_descriptor().unwrap();
            if device_desc.vendor_id() == phone_vendor_id && (device_desc.product_id() == phone_product_id_hid || device_desc.product_id() == phone_product_id) { // phone is plugged in
                if cur_hid == 0 {
                    cur_hid = device_desc.product_id();
                } else {
                    if cur_hid != device_desc.product_id() && !openvpn_killed {
                        restart_openvpn();
                        cur_hid = device_desc.product_id();
                    }
                }

                if openvpn_killed {
                    start_openvpn();
                    openvpn_killed = false;
                }
                
                phone_unplugged = false;
                break;
            } else {
                phone_unplugged = true;
            }
        }

        if phone_unplugged && !openvpn_killed {
            kill_openvpn();
            openvpn_killed = true;
        }

        if !phone_unplugged && !openvpn_killed && !online(None).unwrap() {
            restart_openvpn();
        }

        sleep(1000);
    }
}

fn main() {
    let phone_vendor_id = 0x0b05;
    let phone_product_id = 0x7770;
    let phone_product_id_hid = 0x7773;
    let mut context = libusb::Context::new().unwrap();
    start_timer(&context, phone_vendor_id, phone_product_id, phone_product_id_hid);
}