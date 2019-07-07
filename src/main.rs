extern crate libusb;
extern crate eventual;
extern crate notify_rust;
use eventual::Timer;
use notify_rust::Notification;
use std::sync::mpsc::Sender;
use std::process::Command;
use std::thread;

fn start_openvpn() {
    Command::new("gksu")
        .arg("/home/meet/.config/android_tether/start_ovpn.sh")
        .spawn()
        .expect("Failed to start OpenVPN");
}

fn kill_openvpn() {
    let cmd = Command::new("gksu").arg("pkill openvpn").spawn().expect("Failed to kill OpenVPN");
}

fn start_timer(context: &libusb::Context, phone_vendor_id: u16, phone_product_id: u16, phone_product_id_hid: u16) {
    let timer = Timer::new();
    let ticks = timer.interval_ms(1000).iter();
    let mut phone_unplugged = true;
    let mut openvpn_killed = true;
    let mut cur_hid = 0;

    for _ in ticks {
        for mut device in context.devices().unwrap().iter() {
            let device_desc = device.device_descriptor().unwrap();
            if device_desc.vendor_id() == phone_vendor_id && (device_desc.product_id() == phone_product_id_hid || device_desc.product_id() == phone_product_id) { // phone is plugged in
                if cur_hid == 0 {
                    cur_hid = device_desc.product_id();
                } else {
                    if cur_hid != device_desc.product_id() {
                        kill_openvpn();
                        start_openvpn();
                        cur_hid = device_desc.product_id();
                    }
                }

                println!("Found phone!");
                if openvpn_killed{
                    start_openvpn();
                    println!("Started OpenVPN.");
                    Notification::new()
                        .summary("Started OpenVPN")
                        .body("I just started OpenVPN for you.")
                        .icon("firefox")
                        .show().unwrap();
                    openvpn_killed = false;
                }
                
                phone_unplugged = false;
                break;
            } else {
                phone_unplugged = true;
            }
        }
        println!("Phone unplugged: {}", phone_unplugged);

        if phone_unplugged && !openvpn_killed {
            kill_openvpn();
            openvpn_killed = true;
        }
    }
}

fn main() {
    let phone_vendor_id = 0x0b05;
    let phone_product_id = 0x7770;
    let phone_product_id_hid = 0x7773;
    let mut context = libusb::Context::new().unwrap();
    start_timer(&context, phone_vendor_id, phone_product_id, phone_product_id_hid);
}