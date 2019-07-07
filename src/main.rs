extern crate libusb;
extern crate eventual;
use eventual::Timer;
use std::sync::mpsc::Sender;
use std::process::Command;
use std::thread;

fn start_openvpn() {
    Command::new("adb")
        .arg("forward")
        .arg("tcp:41927")
        .arg("tcp:41927")
        .spawn();
    Command::new("openvpn")
        .arg("--ping")
        .arg("10")
        .arg("--ping-restart")
        .arg("30")
        .arg("--route")
        .arg("0.0.0.0")
        .arg("128.0.0.0")
        .arg("--route")
        .arg("128.0.0.0")
        .arg("128.0.0.0")
        .arg("--socket-flags")
        .arg("TCP_NODELAY")
        .arg("--dhcp-option")
        .arg("DNS")
        .arg("192.168.56.1")
        .arg("--proto")
        .arg("tcp-client")
        .arg("--ifconfig")
        .arg("192.168.56.2")
        .arg("192.168.56.1")
        .arg("--remote")
        .arg("127.0.0.1")
        .arg("41927")
        .arg("tcp-client")
        .arg("--dev")
        .arg("tun0")
        .spawn();
}

fn kill_openvpn() -> bool {
    let cmd = Command::new("pkill").arg("openvpn").output().expect("Failed to kill OpenVPN");
    cmd.status.success()
}

fn start_timer(context: &libusb::Context, phone_vendor_id: u16, phone_product_id: u16, phone_product_id_hid: u16) {
    let timer = Timer::new();
    let ticks = timer.interval_ms(1000).iter();
    let mut phone_unplugged = true;
    let mut openvpn_killed = true;

    for _ in ticks {
        for mut device in context.devices().unwrap().iter() {
            let device_desc = device.device_descriptor().unwrap();
            if device_desc.vendor_id() == phone_vendor_id && (device_desc.product_id() == phone_product_id_hid || device_desc.product_id() == phone_product_id) { // phone is plugged in
                if openvpn_killed {
                    start_openvpn();
                    println!("Started OpenVPN.");
                    openvpn_killed = false;
                }
                
                phone_unplugged = false;
                break;
            } else {
                phone_unplugged = true;
            }
        }

        if phone_unplugged && !openvpn_killed {
            openvpn_killed = kill_openvpn();
            println!("Phone unplugged. Killed OpenVPN: {}", openvpn_killed);
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