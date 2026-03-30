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

pub fn setup_wifi<'a>(
    radio_controller: &'a esp_radio::Controller<'a>,
    wifi_peripheral: WIFI<'a>,
) -> Result<(WifiController<'a>, WifiDevice<'a>), WifiError> {
    let (wifi_controller, wifi_interfaces) =
        esp_radio::wifi::new(&radio_controller, wifi_peripheral, Default::default())?;

    let wifi_device = wifi_interfaces.sta;
    Ok((wifi_controller, wifi_device))
}

pub fn setup_tcp<'a>(
    wifi_device: &mut WifiDevice,
) -> (smoltcp::iface::Interface, [SocketStorage<'a>; 3]) {
    (
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
    )
}

pub fn build_networking_stack<'a>(
    wifi_device: WifiDevice<'a>,
    tcp_interface: smoltcp::iface::Interface,
    tcp_sockets: &'a mut [SocketStorage<'a>],
) -> Stack<'a, WifiDevice<'a>> {
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

    Stack::new(tcp_interface, wifi_device, socket_set, now, rng.random())
}

/// Connects to WiFi using SSID and Password set as environment variables
// TODO: use Result?
pub fn connect_wifi(wifi_controller: &mut WifiController) -> Option<WifiError> {
    wifi_controller
        .set_power_saving(esp_radio::wifi::PowerSaveMode::None)
        .ok()?;

    let client_config = ModeConfig::Client(
        ClientConfig::default()
            .with_ssid(WIFI_SSID.into())
            .with_password(WIFI_PASSWORD.into()),
    );

    wifi_controller.set_config(&client_config).ok()?;
    wifi_controller.start().ok()?;

    info!("WiFi controller scanning...");
    let scan_config = ScanConfig::default().with_max(10);
    wifi_controller.scan_with_config(scan_config).ok()?;

    info!("WiFi controller connecting...");
    wifi_controller.connect().ok()?;
    while !wifi_controller.is_connected().ok()? {}
    return None;
}

/* TODO: det ville måske være rart at have det hele i en funktion,
 * men det er lidt svært at få lifetimes til at fungere som jeg
 * vil have det

// much of this is based on https://esp32.implrust.com/wifi/sta-mode-access-website.html
// pub fn init<'a>(wifi_peripheral: WIFI<'a>) -> Result<Stack<'a, WifiDevice<'a>>, WifiBundleError> {
pub fn init<'a>(wifi_peripheral: WIFI<'a>) -> Result<WifiBundle<'a>, WifiBundleError> {
    let radio_controller: esp_radio::Controller<'a> = esp_radio::init()?;
    let (mut wifi_controller, interfaces) =
        esp_radio::wifi::new(&radio_controller, wifi_peripheral, Default::default())?;

    let mut device = interfaces.sta;

    let tcp_interface = smoltcp::iface::Interface::new(
        smoltcp::iface::Config::new(smoltcp::wire::HardwareAddress::Ethernet(
            smoltcp::wire::EthernetAddress::from_bytes(&device.mac_address()),
        )),
        &mut device,
        smoltcp::time::Instant::from_micros(
            esp_hal::time::Instant::now()
                .duration_since_epoch()
                .as_micros() as i64,
        ),
    );

    let mut socket_set_entries: [SocketStorage; 3] = Default::default();
    let mut socket_set = SocketSet::new(&mut socket_set_entries[..]);

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
    }; // zero-arg closure (call with `now();`)

    let stack = Stack::new(tcp_interface, device, socket_set, now, rng.random());

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

    info!("WiFi controller connecting...");
    wifi_controller.connect()?;
    while !wifi_controller.is_connected()? {}

    info!("WiFi controller connected, waiting for IP address...");
    while !stack.is_iface_up() {
        stack.work();
    }

    Ok(WifiBundle {
        wifi_controller: wifi_controller,
        stack: stack,
    })
    // Ok(stack)
}

// error stuff
#[derive(Format)]
pub enum WifiBundleError {
    WifiError(WifiError),
    InitializationError(InitializationError),
}
// we need to implement From<T> because it allows use to use the `?` operator,
impl From<WifiError> for WifiBundleError {
    fn from(value: WifiError) -> Self {
        WifiBundleError::WifiError(value)
    }
}

impl From<InitializationError> for WifiBundleError {
    fn from(value: InitializationError) -> Self {
        WifiBundleError::InitializationError(value)
    }
}

*/
