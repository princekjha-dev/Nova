pub mod bus;
pub mod device;
pub mod protocol;
pub mod registry;
pub mod store;

pub use bus::MessageBus;
pub use device::{Device, DeviceId, Permission, Platform};
pub use protocol::{
    ControlMessage, NovaMessage, PluginId, PROTOCOL_VERSION, WIRE_MAGIC,
    PLUGIN_AI, PLUGIN_CLIPBOARD, PLUGIN_COLLAB, PLUGIN_CONTROL, PLUGIN_FILES,
    PLUGIN_HANDOFF, PLUGIN_MIRROR, PLUGIN_NOTES, PLUGIN_REMOTE,
};
pub use registry::DeviceRegistry;
pub use store::DeviceStore;
