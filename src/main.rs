extern crate libusb;
extern crate eventual;
extern crate notify_rust;
use eventual::Timer;
use notify_rust::Notification;
use std::sync::mpsc::Sender;
use std::process::Command;
use std::time::Duration;
use std::thread;

static PHONE_CONNECTED: &'static str = "smartphonetrusted";
static PHONE_DISCONNECTED: &'static str = "smartphonedisconnected";

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
}

fn kill_openvpn() {
    Command::new("sudo")
        .arg("/home/meet/.config/android_tether/end_ovpn.sh")
        .spawn()
        .expect("Failed to start OpenVPN");
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
                        sleep(6000);
                        start_openvpn();
                        cur_hid = device_desc.product_id();
                    }
                }

                if openvpn_killed {
                    start_openvpn();
                    show_notification("Initiated tether", PHONE_CONNECTED, 0);
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
            show_notification("Severed tether", PHONE_DISCONNECTED, 0);
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