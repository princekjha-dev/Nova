use anyhow::{anyhow, Result};
use mdns_sd::{Receiver, ServiceDaemon, ServiceEvent, ServiceInfo};
use std::collections::{HashMap, HashSet};
use std::net::IpAddr;
use tracing::{debug, info, warn};

pub const NOVA_SERVICE_TYPE: &str = "_nova._tcp.local.";
pub const DEFAULT_NOVA_PORT: u16 = 53418;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiscoveryEvent {
    DeviceFound {
        device_id: String,
        device_name: String,
        fingerprint: String,
        features: Vec<u16>,
        addresses: HashSet<IpAddr>,
        port: u16,
    },
    DeviceLost {
        fullname: String,
    },
}

pub struct NovaMdnsAnnouncer {
    daemon: ServiceDaemon,
    fullname: String,
}

impl NovaMdnsAnnouncer {
    pub fn new(
        device_id: &str,
        device_name: &str,
        fingerprint: &str,
        features: &[u16],
        port: u16,
    ) -> Result<Self> {
        let daemon = ServiceDaemon::new()
            .map_err(|e| anyhow!("Failed to start mDNS daemon: {:?}", e))?;

        let mut properties = HashMap::new();
        properties.insert("id".to_string(), device_id.to_string());
        properties.insert("name".to_string(), device_name.to_string());
        properties.insert("v".to_string(), "1".to_string());
        properties.insert("fp".to_string(), fingerprint.to_string());
        properties.insert(
            "features".to_string(),
            features.iter().map(|f| f.to_string()).collect::<Vec<_>>().join(","),
        );

        let host_name = format!("{}.local.", sanitize_host(device_name));
        let service = ServiceInfo::new(
            NOVA_SERVICE_TYPE,
            device_name,
            &host_name,
            (), // automatically resolve IP
            port,
            properties,
        )
        .map_err(|e| anyhow!("Failed to build ServiceInfo: {:?}", e))?;

        let fullname = service.get_fullname().to_string();
        daemon.register(service)
            .map_err(|e| anyhow!("Failed to register mDNS service: {:?}", e))?;

        info!("mDNS service registered as {} on port {}", fullname, port);
        Ok(Self { daemon, fullname })
    }

    pub fn shutdown(&self) -> Result<()> {
        self.daemon.unregister(&self.fullname)
            .map_err(|e| anyhow!("Failed to unregister mDNS service: {:?}", e))?;
        Ok(())
    }
}

pub struct NovaMdnsBrowser {
    daemon: ServiceDaemon,
    receiver: Receiver<ServiceEvent>,
}

impl NovaMdnsBrowser {
    pub fn new() -> Result<Self> {
        let daemon = ServiceDaemon::new()
            .map_err(|e| anyhow!("Failed to start mDNS browser daemon: {:?}", e))?;
        let receiver = daemon.browse(NOVA_SERVICE_TYPE)
            .map_err(|e| anyhow!("Failed to browse mDNS service: {:?}", e))?;
        Ok(Self { daemon, receiver })
    }

    pub async fn next_event(&mut self) -> Option<DiscoveryEvent> {
        loop {
            match self.receiver.recv_async().await {
                Ok(ServiceEvent::ServiceResolved(info)) => {
                    let device_id = match info.get_property_val_str("id") {
                        Some(id) => id.to_string(),
                        None => continue,
                    };
                    let device_name = match info.get_property_val_str("name") {
                        Some(n) => n.to_string(),
                        None => continue,
                    };
                    let fingerprint = info.get_property_val_str("fp").unwrap_or("").to_string();
                    let features_str = info.get_property_val_str("features").unwrap_or("");
                    let features = features_str
                        .split(',')
                        .filter_map(|s| s.parse::<u16>().ok())
                        .collect();

                    let addresses = info.get_addresses().clone();
                    let port = info.get_port();

                    return Some(DiscoveryEvent::DeviceFound {
                        device_id,
                        device_name,
                        fingerprint,
                        features,
                        addresses,
                        port,
                    });
                }
                Ok(ServiceEvent::ServiceRemoved(_, fullname)) => {
                    return Some(DiscoveryEvent::DeviceLost { fullname });
                }
                Ok(_) => continue,
                Err(e) => {
                    warn!("mDNS browse error: {:?}", e);
                    return None;
                }
            }
        }
    }
}

fn sanitize_host(name: &str) -> String {
    name.chars()
        .map(|c| if c.is_alphanumeric() { c.to_ascii_lowercase() } else { '-' })
        .collect()
}
