use alloc::format;
use alloc::vec::Vec;
use blocking_network_stack::{IoError, Socket};
use defmt::{error, info};
use embedded_io::{Read, Write};
use esp_println::print;
use esp_radio::wifi::WifiDevice;
use smoltcp::wire::IpAddress;

pub fn send_post(tcp_socket: &mut Socket<WifiDevice>, body: &str) -> Result<(), IoError> {
    let (server_ip, server_port) = get_server_ip_address_and_port();

    // === 0. open socket === //
    info!("Opening TCP socket to {}:{}", server_ip, server_port);
    tcp_socket.open(server_ip, server_port)?;

    // === 1. send http request === //
    info!("Sending POST request to {}:{}", server_ip, server_port);
    tcp_socket.work();

    let req = format!(
        "\
    POST /api/data HTTP/1.1\r\n\
    Host:{}:{}\r\n\
    Connection: close\r\n\
    Content-Type: application/json\r\n\
    Content-Length: {}\r\n\r\n\
    {}
    \r\n",
        server_ip,
        server_port,
        body.len(),
        body
    );

    tcp_socket.write(req.as_bytes())?;

    tcp_socket.flush()?;

    // === 2. listen for http response === //
    info!("Request sent, waiting for response...");
    let mut tcp_socket_buffer = [0u8; 512];
    print!("\n");
    while let Ok(len) = tcp_socket.read(&mut tcp_socket_buffer) {
        let Ok(part) = core::str::from_utf8(&tcp_socket_buffer[..len]) else {
            error!("TCP read socket error");
            return Err(IoError::SocketClosed); // TODO: this is the wrong error type :)
        };
        print!("{part}");
    }

    // === 3. close socket === //
    info!("Closing TCP socket...");
    tcp_socket.disconnect();
    tcp_socket.close();
    Ok(())
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
