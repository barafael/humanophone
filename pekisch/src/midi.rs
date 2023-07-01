use anyhow::Context;

use midir::{Ignore, MidiInput};
use midly::{live::LiveEvent, MidiMessage};
use tokio::sync::mpsc::{self};
use tracing::{info, warn};

use std::io::{stdin, stdout, Write};
use std::sync::{Arc, Condvar, Mutex};

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
pub fn forward(
    midi_tx: mpsc::Sender<MidiMessage>,
    index: Option<usize>,
    cond_var: Arc<(Mutex<bool>, Condvar)>,
) -> anyhow::Result<()> {
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
        (n, Some(index)) if index < n => &in_ports[index],
        (n, Some(index)) => {
            anyhow::bail!("Invalid index {index}, there are only {n} devices");
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
    };

    println!("\nOpening connection");
    let in_port_name = midi_in.port_name(in_port)?;

    let conn = midi_in
        .connect(
            in_port,
            "humanophone-midi-in",
            move |_stamp, message, cond| {
                let Some(msg) = on_midi(message) else { return };
                if let Err(e) = midi_tx.blocking_send(msg) {
                    warn!("Failed to forward midi message: {:?}", e.0);
                    let lock = &cond.0;
                    let cvar = &cond.1;
                    *lock.lock().unwrap() = true;
                    cvar.notify_one();
                }
            },
            Arc::clone(&cond_var),
        )
        // Somehow this doesn't want to be a proper StdError:
        .map_err(|e| anyhow::format_err!("{e}"))
        .context("Failed to connect to midi input")?;

    println!("Connection open, reading input from '{in_port_name}'");

    let lock = &cond_var.0;
    let cvar = &cond_var.1;
    let mut quit = lock.lock().unwrap();
    while !*quit {
        info!("Waiting for quit of midi connection");
        quit = cvar.wait(quit).unwrap();
    }

    println!("Closing connection");
    conn.close();
    Ok(())
}
