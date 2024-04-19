use std::thread::sleep;
use std::time::Duration;
use anyhow::{Result, bail};
use colored::Colorize;
use enum_map::{enum_map, EnumMap};
use goxlr_types::{ChannelName, FaderName};
use goxlr_types::ChannelName::{Chat, Console, Game, Headphones, LineIn, LineOut, Mic, MicMonitor, Music, Sample, System};
use lazy_static::lazy_static;
use strum::IntoEnumIterator;

use crate::goxlr::{Device, DeviceType, GoXLR};

mod goxlr;
mod preflight;

// We intentionally skip 0 and 255 as 'possible' default volumes, so we can do
// separate Top / Bottom Tests later.
lazy_static! {
    static ref VOLUME_SET: EnumMap<ChannelName, u8> = enum_map! {
        Mic => 10,
        LineIn => 30,
        Console => 50,
        System => 70,
        Game => 90,
        Chat => 110,
        Sample => 130,
        Music => 245,
        Headphones => 170,
        MicMonitor => 190,
        LineOut => 210,
    };
}


fn main() -> Result<()> {
    info("Frosty's Shitty Simple Fader Tester..");

    if let Err(message) = preflight::status_check() {
        error(message.to_string().as_str());
        return Ok(());
    }

    info("Locating GoXLR Devices..");

    let mut goxlr = GoXLR::new();
    let devices = goxlr.find_devices();

    info("Found Devices:");
    println!("{:#?}", devices);

    if devices.is_empty() {
        error("No GoXLR Devices Detected");
        bail!("Run Failure");
    }
    if devices.len() > 1 {
        warn("More than one Device found, using first Detected Device");
    }

    let device = devices[0].clone();
    if device.device_type == DeviceType::Mini {
        error("This tool only works on Full GoXLR Devices");
        bail!("Run Failure");
    }

    // Ok, if we get here, we should be good to go. First thing to do is assign all the
    // channel volumes..
    for channel in ChannelName::iter() {
        goxlr.set_volume(device.clone(), channel, VOLUME_SET[channel])?;
    }

    volume_check(&mut goxlr, device.clone(), (Mic, LineIn, Console, System))?;
    volume_check(&mut goxlr, device.clone(), (Game, Chat, Sample, Music))?;
    volume_check(&mut goxlr, device.clone(), (Headphones, MicMonitor, LineOut, Mic))?;

    Ok(())
}

fn volume_check(goxlr: &mut GoXLR, device: Device, channels: (ChannelName, ChannelName, ChannelName, ChannelName)) -> Result<()> {
    let pause = 300;

    info("---");
    info(format!("Testing for Channels: {}, {}, {}, {}", channels.0, channels.1, channels.2, channels.3).as_str());
    info("---");

    // Step 1, assign the channels..
    info("Testing Fader Assignment..");
    goxlr.assign_channel(device.clone(), FaderName::A, channels.0)?;
    goxlr.assign_channel(device.clone(), FaderName::B, channels.1)?;
    goxlr.assign_channel(device.clone(), FaderName::C, channels.2)?;
    goxlr.assign_channel(device.clone(), FaderName::D, channels.3)?;

    // Wait 500ms for faders to move..
    sleep(Duration::from_millis(pause));
    let volumes = goxlr.get_volumes(device.clone())?;

    // These should Match the EnumMap above..
    test_volume(channels.0, volumes[0], VOLUME_SET[channels.0], 5);
    test_volume(channels.1, volumes[1], VOLUME_SET[channels.1], 5);
    test_volume(channels.2, volumes[2], VOLUME_SET[channels.2], 5);
    test_volume(channels.3, volumes[3], VOLUME_SET[channels.3], 5);


    // Next, set all channels to 100%..
    info("Testing Max Volume Reach");
    goxlr.set_volume(device.clone(), channels.0, 255)?;
    goxlr.set_volume(device.clone(), channels.1, 255)?;
    goxlr.set_volume(device.clone(), channels.2, 255)?;
    goxlr.set_volume(device.clone(), channels.3, 255)?;

    // Wait 500ms for faders to move..
    sleep(Duration::from_millis(pause));
    let volumes = goxlr.get_volumes(device.clone())?;

    // Check the Volume..
    test_volume(channels.0, volumes[0], 255, 0);
    test_volume(channels.1, volumes[1], 255, 0);
    test_volume(channels.2, volumes[2], 255, 0);
    test_volume(channels.3, volumes[3], 255, 0);

    // Next, set all channels to 0%..
    info("Testing Min Volume Reach");
    goxlr.set_volume(device.clone(), channels.0, 0)?;
    goxlr.set_volume(device.clone(), channels.1, 0)?;
    goxlr.set_volume(device.clone(), channels.2, 0)?;
    goxlr.set_volume(device.clone(), channels.3, 0)?;

    // Wait 500ms for faders to move..
    sleep(Duration::from_millis(pause));
    let volumes = goxlr.get_volumes(device.clone())?;

    // Check the Volume..
    test_volume(channels.0, volumes[0], 0, 0);
    test_volume(channels.1, volumes[1], 0, 0);
    test_volume(channels.2, volumes[2], 0, 0);
    test_volume(channels.3, volumes[3], 0, 0);

    // Finally, restore the original values..
    info("Testing Restore Volume");
    goxlr.set_volume(device.clone(), channels.0, VOLUME_SET[channels.0])?;
    goxlr.set_volume(device.clone(), channels.1, VOLUME_SET[channels.1])?;
    goxlr.set_volume(device.clone(), channels.2, VOLUME_SET[channels.2])?;
    goxlr.set_volume(device.clone(), channels.3, VOLUME_SET[channels.3])?;

    // Wait 500ms for faders to move..
    sleep(Duration::from_millis(pause));
    let volumes = goxlr.get_volumes(device.clone())?;

    // These should Match the EnumMap above..
    test_volume(channels.0, volumes[0], VOLUME_SET[channels.0], 5);
    test_volume(channels.1, volumes[1], VOLUME_SET[channels.1], 5);
    test_volume(channels.2, volumes[2], VOLUME_SET[channels.2], 5);
    test_volume(channels.3, volumes[3], VOLUME_SET[channels.3], 5);

    // Testing Mute and Unmute (Volumes shouldn't change)
    info("Testing Mute State Change (to Muted), fader should not move.");
    goxlr.set_mute_state(device.clone(), channels.0, true)?;
    goxlr.set_mute_state(device.clone(), channels.1, true)?;
    goxlr.set_mute_state(device.clone(), channels.2, true)?;
    goxlr.set_mute_state(device.clone(), channels.3, true)?;

    // We'll use the previous volumes to confirm position, the faders shouldn't have moved.
    let volumes_new = goxlr.get_volumes(device.clone())?;
    test_volume(channels.0, volumes_new[0], volumes[0], 0);
    test_volume(channels.1, volumes_new[1], volumes[1], 0);
    test_volume(channels.2, volumes_new[2], volumes[2], 0);
    test_volume(channels.3, volumes_new[3], volumes[3], 0);

    info("Testing Mute State Change (to Unmuted), fader should not move.");
    goxlr.set_mute_state(device.clone(), channels.0, false)?;
    goxlr.set_mute_state(device.clone(), channels.1, false)?;
    goxlr.set_mute_state(device.clone(), channels.2, false)?;
    goxlr.set_mute_state(device.clone(), channels.3, false)?;

    // We'll use the previous volumes to confirm position, the faders shouldn't have moved.
    let volumes_new = goxlr.get_volumes(device.clone())?;
    test_volume(channels.0, volumes_new[0], volumes[0], 0);
    test_volume(channels.1, volumes_new[1], volumes[1], 0);
    test_volume(channels.2, volumes_new[2], volumes[2], 0);
    test_volume(channels.3, volumes_new[3], volumes[3], 0);

    Ok(())
}

fn test_volume(channel: ChannelName, volume: u8, expected: u8, margin: i16) {
    let volume = volume as i16;
    let expected = expected as i16;
    if volume != expected {
        // The error Margin here is around 2%..
        if (expected - margin..=expected + margin).contains(&volume) {
            pass(format!("Volume Matched for {} - Expected {}, Received: {} (Error Margin: {})", channel, expected, volume, volume - expected).as_str());
            return;
        } else {
            fail(format!("Volume Check Failed for {} - Expected {}, Received: {}", channel, expected, volume).as_str());
            return;
        }
    }
    pass(format!("Volume Matched for {} - Value: {}", channel, volume).as_str());
}

fn info(message: &str) {
    println!("[{}] {}", "INFO".blue(), message);
}

fn warn(message: &str) {
    println!("[{}] {}", "WARN".yellow(), message);
}

fn error(message: &str) {
    println!("[{}] {}", "ERROR".red(), message);
}


fn pass(message: &str) {
    println!("[{}] {}", "PASS".green(), message);
}

fn fail(message: &str) {
    println!("[{}] {}", "FAIL".red(), message);
}
