use blocking_network_stack::Stack;
use defmt::info;
use esp_hal::{peripherals::WIFI, rng::Rng};
use esp_radio::wifi::{
    ClientConfig, ModeConfig, ScanConfig, WifiController, WifiDevice, WifiError,
};
use smoltcp::iface::{SocketSet, SocketStorage};

const WIFI_SSID: &str = env!("MOTE_WIFI_SSID");
const WIFI_PASSWORD: &str = env!("MOTE_WIFI_PASSWORD");

// struct WifiBundle<'a> {
//     wifi_controller: WifiController<'a>,
//     stack: Stack<'a, WifiDevice<'a>>,
// }

/// Sets up WiFi peripherals and returns a WiFi (controller, device) tuple.
pub fn setup_wifi<'a>(
    radio_controller: &'a esp_radio::Controller<'a>,
    wifi_peripheral: WIFI<'a>,
) -> Result<(WifiController<'a>, WifiDevice<'a>), WifiError> {
    info!("Setting up WiFi peripherals...");
    let (wifi_controller, wifi_interfaces) =
        esp_radio::wifi::new(&radio_controller, wifi_peripheral, Default::default())?;

    let wifi_device = wifi_interfaces.sta;
    info!("WiFi peripherals setup complete");
    Ok((wifi_controller, wifi_device))
}

/// Sets up TCP sockets and returns a (`smoltcp` interface, `SocketStorage` array) tuple
pub fn setup_tcp<'a>(
    wifi_device: &mut WifiDevice,
) -> (smoltcp::iface::Interface, [SocketStorage<'a>; 3]) {
    info!("Setting up TCP sockets...");
    let result = (
        smoltcp::iface::Interface::new(
            smoltcp::iface::Config::new(smoltcp::wire::HardwareAddress::Ethernet(
                smoltcp::wire::EthernetAddress::from_bytes(&wifi_device.mac_address()),
            )),
            wifi_device,
            smoltcp::time::Instant::from_micros(
                esp_hal::time::Instant::now()
                    .duration_since_epoch()
                    .as_micros() as i64,
            ),
        ),
        Default::default(),
    );
    info!("TCP socket setup complete");
    result
}

/// Builds a blocking networking stack using a WiFi device and TCP interface/sockets.
pub fn build_networking_stack<'a>(
    wifi_device: WifiDevice<'a>,
    tcp_interface: smoltcp::iface::Interface,
    tcp_sockets: &'a mut [SocketStorage<'a>],
) -> Stack<'a, WifiDevice<'a>> {
    info!("Configuring networking stack...");
    let mut socket_set = SocketSet::new(&mut tcp_sockets[..]);

    let mut dhcp_socket = smoltcp::socket::dhcpv4::Socket::new();
    dhcp_socket.set_outgoing_options(&[smoltcp::wire::DhcpOption {
        kind: 12,
        data: b"co2Mote",
    }]);
    socket_set.add(dhcp_socket);

    let rng = Rng::new();
    let now = || {
        esp_hal::time::Instant::now()
            .duration_since_epoch()
            .as_millis()
    }; // zero-arg closure, I think...

    let stack = Stack::new(tcp_interface, wifi_device, socket_set, now, rng.random());
    info!("Networking stack configuration complete");

    stack
}

/// Connects to WiFi using SSID and Password set as environment variables
// TODO: use Result?
pub fn connect_wifi(wifi_controller: &mut WifiController) -> Result<(), WifiError> {
    wifi_controller.set_power_saving(esp_radio::wifi::PowerSaveMode::None)?;

    let client_config = ModeConfig::Client(
        ClientConfig::default()
            .with_ssid(WIFI_SSID.into())
            .with_password(WIFI_PASSWORD.into()),
    );

    wifi_controller.set_config(&client_config)?;
    wifi_controller.start()?;

    info!("WiFi controller scanning...");
    let scan_config = ScanConfig::default().with_max(10);
    wifi_controller.scan_with_config(scan_config)?;

    info!("WiFi scan complete, connecting...");
    wifi_controller.connect()?;
    while !wifi_controller.is_connected()? {}
    info!("WiFi connection successful");

    Ok(())
}

pub fn assign_ip_address(stack: &mut Stack<WifiDevice>) {
    info!("Assigning IP address...");

    // while !stack.is_iface_up() {
    //     stack.work();
    // }
    loop {
        stack.work();
        if stack.is_iface_up() {
            break;
        }
    }
    info!("IP address assigned");
}
