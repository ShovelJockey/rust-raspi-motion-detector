use log::debug;
use std::{
    collections::HashMap,
    env::var,
    net::{IpAddr, SocketAddr},
    str::FromStr,
    sync::Arc,
};
use tokio::{net::UdpSocket, time::Duration};
use turn::auth::AuthHandler;
use turn::relay::relay_static::*;
use turn::server::{
    config::{ConnConfig, ServerConfig},
    Server,
};
use turn::Error;
use webrtc_util::vnet::net::*;

struct TurnAuthHandler {
    cred_map: HashMap<String, Vec<u8>>,
}

impl TurnAuthHandler {
    fn new(cred_map: HashMap<String, Vec<u8>>) -> Self {
        TurnAuthHandler { cred_map }
    }
}

impl AuthHandler for TurnAuthHandler {
    fn auth_handle(
        &self,
        username: &str,
        _realm: &str,
        _src_addr: SocketAddr,
    ) -> Result<Vec<u8>, Error> {
        if let Some(pw) = self.cred_map.get(username) {
            debug!("username={}, password={:?}", username, pw);
            Ok(pw.to_vec())
        } else {
            Err(Error::ErrFakeErr)
        }
    }
}

pub async fn create_turn_server() -> Result<Server, Error> {
    let port = var("TURN_PORT").expect("Valid turns port env");
    let conn = Arc::new(
        UdpSocket::bind(format!("0.0.0.0:{port}"))
            .await
            .expect("Failed to bind to turn port"),
    );

    let public_ip = var("TURN_PUBLIC_IP").expect("Valid public ip env");

    let conn_config = ConnConfig {
        conn,
        relay_addr_generator: Box::new(RelayAddressGeneratorStatic {
            relay_address: IpAddr::from_str(public_ip.as_str())?,
            address: "0.0.0.0".to_owned(),
            net: Arc::new(Net::new(None)),
        }),
    };

    let cred_map = HashMap::from([("user1".to_string(), b"password123".to_vec())]);

    let auth_handler = Arc::new(TurnAuthHandler::new(cred_map));

    let server_config = ServerConfig {
        conn_configs: vec![conn_config],
        realm: public_ip,
        auth_handler,
        channel_bind_timeout: Duration::from_secs(0),
        alloc_close_notify: None,
    };

    let server = Server::new(server_config).await?;
    debug!("Turn server starded without error");
    Ok(server)
}
