use anyhow::{bail, Result};
use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use std::sync::{Arc, Mutex, MutexGuard};
use goxlr_types::{ChannelName, FaderName};
use goxlr_usb::channelstate::ChannelState;
use goxlr_usb::device::base::FullGoXLRDevice;
use goxlr_usb::device::{find_devices, from_device};
use tokio::sync::mpsc;

pub(crate) struct GoXLR {
    handles: HashMap<DeviceLocal, Arc<Mutex<Box<dyn FullGoXLRDevice>>>>,
}

impl GoXLR {
    pub(crate) fn new() -> Self {
        GoXLR {
            handles: HashMap::new(),
        }
    }

    pub fn find_devices(&mut self) -> Vec<Device> {
        let devices = find_devices();
        let mut device_list: Vec<Device> = Vec::new();

        // Create handles for all devices..
        for device in devices {
            let local_device = DeviceLocal {
                bus_number: device.bus_number(),
                address: device.address(),
                identifier: device.identifier().clone(),
            };

            // Do we need a new handle, or to use an existing one?
            let mut handle = if self.handles.contains_key(&local_device) {
                self.handles
                    .get_mut(&local_device.clone())
                    .unwrap()
                    .lock()
                    .unwrap()
            } else {
                // We don't care about messages being sent out at this point, we're explicitly
                // going to ignore them and handle errors on-the-fly during the update.
                let (disconnect_sender, _) = mpsc::channel(32);
                let (event_sender, _) = mpsc::channel(32);

                // Create the Handle, the pause is only needed if we're waiting for the startup animation to finish, in this
                // context, we don't care.
                let handle = from_device(device.clone(), disconnect_sender, event_sender, true);
                if let Err(error) = &handle {
                    println!("Error: {}", error);
                    continue;
                }

                // Unwrap the Handle, and stop polling for events.
                let mut handle = handle.unwrap();
                handle.stop_polling();

                self.handles
                    .insert(local_device.clone(), Arc::new(Mutex::new(handle)));
                self.handles
                    .get_mut(&local_device.clone())
                    .unwrap()
                    .lock()
                    .unwrap()
            };

            if let Ok(descriptor) = handle.get_descriptor() {
                let device_type = match descriptor.product_id() {
                    goxlr_usb::PID_GOXLR_FULL => DeviceType::Full,
                    goxlr_usb::PID_GOXLR_MINI => DeviceType::Mini,
                    _ => continue,
                };
                if let Ok((device_serial, _)) = handle.get_serial_number() {
                    if device_serial.is_empty() {
                        println!("Nope.");
                        continue;
                    }
                    if let Ok(firmware) = handle.get_firmware_version() {
                        let version = VersionNumber(
                            firmware.firmware.0,
                            firmware.firmware.1,
                            firmware.firmware.2,
                            firmware.firmware.3,
                        );

                        device_list.push(Device {
                            device_type,
                            device_serial,
                            version,
                            goxlr_device: local_device.clone(),
                        });
                    }
                }
            } else {
                println!("Nope!");
            }
        }
        device_list
    }

    pub fn set_volume(&mut self, device: Device, channel: ChannelName, volume: u8) -> Result<()> {
        let mut handle = self.get_handle(device)?;
        handle.set_volume(channel, volume)?;

        Ok(())
    }

    pub fn assign_channel(&mut self, device: Device, fader: FaderName, channel: ChannelName) -> Result<()> {
        let mut handle = self.get_handle(device)?;
        handle.set_fader(fader, channel)?;

        Ok(())
    }

    pub fn get_volumes(&mut self, device: Device) -> Result<[u8; 4]> {
        let mut handle = self.get_handle(device)?;
        let buttons = handle.get_button_states()?;
        Ok(buttons.volumes)
    }

    pub fn set_mute_state(&mut self, device: Device, channel: ChannelName, mute: bool) -> Result<()> {
        let mut handle = self.get_handle(device)?;
        let state = if mute {
            ChannelState::Muted
        } else {
            ChannelState::Unmuted
        };

        handle.set_channel_state(channel, state)?;
        Ok(())
    }

    fn get_handle(&mut self, device: Device) -> Result<MutexGuard<Box<dyn FullGoXLRDevice>>> {
        let handle = self.handles.get_mut(&device.goxlr_device);
        let device = match handle {
            Some(device) => device,
            None => bail!("No Device?")
        };

        // Grab the Handle..
        // let arc = device.clone();
        // let handle = arc.lock().unwrap();

        Ok(device.lock().unwrap())
    }
}

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct DeviceLocal {
    pub(crate) bus_number: u8,
    pub(crate) address: u8,
    pub(crate) identifier: Option<String>,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Device {
    pub device_type: DeviceType,
    pub device_serial: String,
    pub version: VersionNumber,
    pub goxlr_device: DeviceLocal,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum DeviceType {
    Full,
    Mini,
}

// Tentatively Stolen :D
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct VersionNumber(pub u32, pub u32, pub Option<u32>, pub Option<u32>);

impl Display for VersionNumber {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if let Some(patch) = self.2 {
            if let Some(build) = self.3 {
                return write!(f, "{}.{}.{}.{}", self.0, self.1, patch, build);
            }
            return write!(f, "{}.{}.{}", self.0, self.1, patch);
        }

        write!(f, "{}.{}", self.0, self.1)
    }
}

impl std::fmt::Debug for VersionNumber {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Display::fmt(self, f)
    }
}