use std::thread;

use clap::Parser;
use netdev::{get_default_gateway, get_default_interface};

use crate::AddressConfig;

#[derive(Parser)]
#[command(
    version,
    about = "Poison ARP caches",
    long_about = "Poison the ARP cache of a target to either block or MitM their requests"
)]
struct Cli {
    /// Block internet traffic to target
    #[arg(short, long, action = clap::ArgAction::SetTrue)]
    block: bool,
    /// IP of target to poison
    target_ip: String,
    /// MAC address of target to poison
    target_mac: String,
    /// IP of gateway
    gateway_ip: Option<String>,
    /// MAC address of gateway
    gateway_mac: Option<String>,
    /// Interface name of host machine to use
    interface_name: Option<String>,
}

pub fn start() {
    let cli = Cli::parse();

    let interface_name = if let Some(cli_interface_name) = cli.interface_name.as_deref() {
        cli_interface_name.to_string()
    } else if let Ok(default_interface) = get_default_interface() {
        default_interface.name
    } else {
        panic!("No default interface found");
    };

    let target_ip = cli.target_ip;
    let target_mac = cli.target_mac;

    let gateway_ip = if let Some(cli_gateway_ip) = cli.gateway_ip {
        cli_gateway_ip
    } else if let Ok(gateway) = get_default_gateway() {
        if gateway.ipv4.len() < 1 {
            panic!("Couldn't find ipv4 address for default gateway");
        };
        gateway.ipv4.get(0).unwrap().to_string()
    } else {
        panic!("Default gateway not found");
    };

    let gateway_mac = if let Some(cli_gateway_mac) = cli.gateway_mac {
        cli_gateway_mac
    } else if let Ok(gateway) = get_default_gateway() {
        gateway.mac_addr.to_string()
    } else {
        panic!("Default gateway not found");
    };

    let block = cli.block;

    let ac = AddressConfig::new(
        &interface_name,
        &target_ip,
        &target_mac,
        &gateway_ip,
        &gateway_mac,
    );

    let ac_arp = ac.clone();

    let handle = thread::spawn(move || ac_arp.intercept_arp());
    if !block {
        ac.route_intercepted_packets();
    } else {
        handle.join().unwrap();
    }
}
