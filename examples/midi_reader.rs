use std::{fs::File, io::Read};
fn handle_division(division: u16) {
    if (division & 0b0) == 0 {
        // Metrical Mode
        let tpqn = division & 0x7FFF;
        println!("Metrical Mode: {} Ticks Per Quarter Note", tpqn);
    } else {
        // SMPTE Mode
        let high_byte = (division >> 8) as i8; // cast to i8 handles two's complement
        let low_byte = (division & 0x00FF) as u8;

        let fps = -high_byte; // e.g., -(-30) = 30
        let ticks_per_frame = low_byte;

        println!(
            "SMPTE Mode: {} FPS, {} Ticks Per Frame",
            fps, ticks_per_frame
        );
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut midi_file = File::open("/home/vinicius/github/GuitarTuner/bach_846.mid")?;
    let mut header = [0u8; 4];
    midi_file.read(&mut header)?;
    assert!(&header == b"MThd");
    let mut chunck_lenght = [0u8; 4];
    midi_file.read(&mut chunck_lenght)?;
    let chunck_size = u32::from_be_bytes(chunck_lenght);
    assert!(chunck_size == 6u32);
    let mut format_buffer = [0u8; 2];
    midi_file.read(&mut format_buffer)?;
    let format = u16::from_be_bytes(format_buffer);
    println!("{format}");
    let mut num_tracks_buffer = [0u8; 2];
    midi_file.read(&mut num_tracks_buffer)?;
    let num_tracks = u16::from_be_bytes(num_tracks_buffer);
    println!("{num_tracks}");
    let mut time_info = [0u8; 2];
    midi_file.read(&mut time_info)?;
    handle_division(u16::from_be_bytes(time_info));
    Ok(())
}
