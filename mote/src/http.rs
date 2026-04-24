use alloc::format;
use alloc::vec::Vec;
use blocking_network_stack::Socket;
use defmt::{error, info};
use embedded_io::{Read, Write};
use esp_hal::time::{Duration, Instant};
use esp_println::print;
use esp_radio::wifi::WifiDevice;
use smoltcp::wire::IpAddress;

use {esp_backtrace as _, esp_println as _};

pub fn http_loop(tcp_socket: &mut Socket<WifiDevice>) {
    tcp_socket.work();

    // === 0. open socket === //
    let (server_ip, server_port) = get_server_ip_address_and_port();
    tcp_socket.open(server_ip, server_port).unwrap_or_else(|e| {
        error!("TCP open socket error: {}", e);
        panic!();
    });
    loop {
        // === 1. send http request === //
        info!("Sending GET request to {}:{}", server_ip, server_port);
        tcp_socket.work();
        if let Err(e) = // we can't use unwrap_or_else because we need control flow control
            tcp_socket.write(format!("GET / HTTP/1.1\r\nHost:{server_ip}\r\n").as_bytes())
        {
            error!("GET request error: {} - retrying", e);
            continue;
        }

        if let Err(e) = tcp_socket.flush() {
            error!("TCP flush socket error: {} - retrying", e);
            continue;
        }

        // === 2. listen for http response === //
        info!("Request sent, waiting for response...");
        let timeout = Instant::now() + Duration::from_secs(20);
        let mut tcp_socket_buffer = [0u8; 512];
        // if tcp_socket.is_open() TODO <--
        while let Ok(len) = tcp_socket.read(&mut tcp_socket_buffer) {
            info!("reading");
            let Ok(part) = core::str::from_utf8(&tcp_socket_buffer[..len]) else {
                error!("TCP read socket error - retrying");
                continue;
            };
            print!("{part}");

            if Instant::now() > timeout {
                info!("GET timeout - retrying");
                continue;
            }
        }

        // === 3. close socket === //
        tcp_socket.disconnect();
        let mut timeout = Instant::now() + Duration::from_secs(5);
        while Instant::now() < timeout {
            tcp_socket.work();
        }

        timeout = Instant::now() + Duration::from_secs(10);
        while Instant::now() < timeout {}
    }
}

fn get_server_ip_address_and_port() -> (IpAddress, u16) {
    let server_ip_parts: Vec<u8> = env!("SERVER_IP")
        .split(".")
        .map(|p| p.parse::<u8>().unwrap())
        .collect();
    assert_eq!(
        server_ip_parts.len(),
        4,
        "Server IP must be a valid IPv4 address"
    );
    let server_ip: IpAddress = IpAddress::v4(
        server_ip_parts[0],
        server_ip_parts[1],
        server_ip_parts[2],
        server_ip_parts[3],
    );
    let server_port: u16 = env!("SERVER_PORT").parse().unwrap();
    (server_ip, server_port)
}
