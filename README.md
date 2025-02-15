# arp_poision
Poison the ARP cache of a target to either block or MitM their requests

## How to use
arp_poision [options] <target_ip> <target_mac> [gateway_ip] [gateway_mac] [interface_name]

Options:
- b, block: Block traffic from target, defaults to passing through requests to the gateway.

Arguments:

target_ip: IP address of the target 

target_mac: MAC address of the target

gateway_ip: IP address of the gateway on the LAN

gateway_mac: MAC address of the gateway on the LAN

interface_name: Interface on the host to send and recieve packets on

