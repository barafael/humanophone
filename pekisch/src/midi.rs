use anyhow::Context;

use midir::{Ignore, MidiInput};
use midly::{live::LiveEvent, MidiMessage};
use tokio::sync::mpsc::{self};
use tracing::warn;

use std::io::{stdin, stdout, Write};

fn on_midi(event: &[u8]) -> Option<MidiMessage> {
    let event = LiveEvent::parse(event).unwrap();
    match event {
        LiveEvent::Midi { message, .. } => match message {
            msg @ (MidiMessage::NoteOn { .. } | MidiMessage::NoteOff { .. }) => Some(msg),
            _ => None,
        },
        _ => None,
    }
}

/// Forwards note-on and note-off events from the selected midi interface to `midi_tx`.
pub fn forward(midi_tx: mpsc::Sender<MidiMessage>, index: Option<usize>) -> anyhow::Result<()> {
    let mut midi_in = MidiInput::new("pekisch reading input")?;
    midi_in.ignore(Ignore::None);

    // Get an input port (read from console if multiple are available)
    let in_ports = midi_in.ports();
    let in_port = match (in_ports.len(), index) {
        (0, _) => anyhow::bail!("No input port found"),
        (1, _) => {
            println!(
                "Choosing the only available input port: {}",
                midi_in
                    .port_name(&in_ports[0])
                    .context("Device disconnected")?
            );
            &in_ports[0]
        }
        (_, None) => {
            println!("\nAvailable input ports:");
            for (i, p) in in_ports.iter().enumerate() {
                println!(
                    "{i}: {}",
                    midi_in.port_name(p).context("Device disconnected")?
                );
            }
            print!("Please select input port: ");
            stdout().flush()?;
            let mut input = String::new();
            stdin()
                .read_line(&mut input)
                .context("Failed to read line")?;
            in_ports
                .get(input.trim().parse::<usize>()?)
                .context("Invalid input port selected")?
        }
        (n, Some(index)) if index < n => &in_ports[index],
        (n, Some(index)) => {
            anyhow::bail!("Invalid index {index}, there are only {n} devices");
        }
    };

    println!("\nOpening connection");
    let in_port_name = midi_in.port_name(in_port)?;

    // _conn_in needs to be a named parameter, because it needs to be kept alive until the end of the scope
    let conn = midi_in
        .connect(
            in_port,
            "humanophone-midi-in",
            move |_stamp, message, _| {
                let Some(msg) = on_midi(message) else { return };
                if let Err(e) = midi_tx.blocking_send(msg) {
                    warn!("Failed to forward midi message: {:?}", e.0);
                }
            },
            (),
        )
        // Somehow this doesn't want to be a proper StdError:
        .map_err(|e| anyhow::format_err!("{e}"))
        .context("Failed to connect to midi input")?;

    println!("Connection open, reading input from '{in_port_name}'");
    println!("(press enter to exit) ...");

    stdin().read_line(&mut String::new())?; // wait for next enter key press

    println!("Closing connection");
    conn.close();
    Ok(())
}
