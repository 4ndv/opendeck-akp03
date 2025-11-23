use mirajazz::{error::MirajazzError, types::DeviceInput};

use crate::mappings::{ENCODER_COUNT, KEY_COUNT};

pub fn process_input(input: u8, state: u8) -> Result<DeviceInput, MirajazzError> {
    log::info!("Processing input: {}, {}", input, state);

    match input {
        (0..=6) | 0x25 | 0x30 | 0x31 => read_button_press(input, state),
        0x90 | 0x91 | 0x50 | 0x51 | 0x60 | 0x61 => read_encoder_value(input),
        0x33..=0x35 => read_encoder_press(input, state),
        _ => Err(MirajazzError::BadData),
    }
}

fn read_button_states(states: &[u8]) -> Vec<bool> {
    let mut bools = vec![];

    for i in 0..KEY_COUNT {
        bools.push(states[i + 1] != 0);
    }

    bools
}

fn read_button_press(input: u8, state: u8) -> Result<DeviceInput, MirajazzError> {
    let mut button_states = vec![0x01];
    button_states.extend(vec![0u8; KEY_COUNT + 1]);

    if input == 0 {
        return Ok(DeviceInput::ButtonStateChange(read_button_states(
            &button_states,
        )));
    }

    let pressed_index: usize = match input {
        // Six buttons with displays
        (1..=6) => input as usize,
        // Three buttons without displays
        0x25 => 7,
        0x30 => 8,
        0x31 => 9,
        _ => return Err(MirajazzError::BadData),
    };

    button_states[pressed_index] = state;

    Ok(DeviceInput::ButtonStateChange(read_button_states(
        &button_states,
    )))
}

fn read_encoder_value(input: u8) -> Result<DeviceInput, MirajazzError> {
    let mut encoder_values = vec![0i8; ENCODER_COUNT];

    let (encoder, value): (usize, i8) = match input {
        // Left encoder
        0x90 => (0, -1),
        0x91 => (0, 1),
        // Middle (top) encoder
        0x50 => (1, -1),
        0x51 => (1, 1),
        // Right encoder
        0x60 => (2, -1),
        0x61 => (2, 1),
        _ => return Err(MirajazzError::BadData),
    };

    encoder_values[encoder] = value;
    Ok(DeviceInput::EncoderTwist(encoder_values))
}

fn read_encoder_press(input: u8, state: u8) -> Result<DeviceInput, MirajazzError> {
    let mut encoder_states = vec![false; ENCODER_COUNT];

    let encoder: usize = match input {
        0x33 => 0, // Left encoder
        0x35 => 1, // Middle (top) encoder
        0x34 => 2, // Right encoder
        _ => return Err(MirajazzError::BadData),
    };

    encoder_states[encoder] = state != 0;
    Ok(DeviceInput::EncoderStateChange(encoder_states))
}
