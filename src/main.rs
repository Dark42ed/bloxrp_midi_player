use std::{
    io::{stdin, stdout, Write},
    fs,
    collections::HashMap,
    thread::sleep,
    time::Duration
};
use midly::{
    Smf,
    TrackEventKind,
    MidiMessage,
};
use midi_reader_writer::{
    ConvertTicksToMicroseconds,
    midly_0_5::{
        merge_tracks
    }
};
use sysinputs::keyboard::{
    press_key,
    release_key,
    Physical,
    Key,
};
use device_query::{
    DeviceState,
    DeviceQuery,
    Keycode
};

const NOTES: &[u8] = "1!2@34$5%6^78*9(0qQwWeErtTyYuiIoOpPasSdDfgGhHjJklLzZxcCvVbBnm".as_bytes();
const SPECIAL_KEYS: &[u8] = "!@#$%^&*()".as_bytes();

// The amount we have to wait until we can be sure that the os has registered the shift key.
// The lower, the more consistent the timings between notes are,
// but also the more inaccurate black keys are.
const SHIFT_WAIT: Duration = Duration::from_millis(12);

fn main() {
    loop {
        let mut file_name = String::new();
    
        print!("Enter file name >");
        stdout().flush().unwrap();


        stdin().read_line(&mut file_name)
        .expect("Unable to read line");
        let file_name = file_name.trim();

        if let Ok(data) = fs::read(&file_name) {
            let smf = Smf::parse(&data).unwrap();
            play(smf);
        } else {
            println!("File {} doesn't exist!\n", file_name);
        }
    }    
}


fn play(smf: Smf) {
    // Toggle: Play where keydown toggles the key
    // Not toggle: Play where keydown presses and keyup releases
    // Some midi's are toggle and others are not.
    let mut toggle = true;
    for track in &smf.tracks {
        for event in track {
            if let TrackEventKind::Midi{message, ..} = event.kind {
                if let MidiMessage::NoteOff{..} = message {
                    toggle = false;
                    break;
                }
            }
        }
    }
    if toggle {
        println!("Keyup not detected, playing with toggle");
    } else {
        println!("Keyup detected, playing with on/off");
    }

    let mut tick_to_microseconds = ConvertTicksToMicroseconds::try_from(smf.header).unwrap();
    let mut last_tick: u64 = 0;
    let mut keys = HashMap::new();
    let mut shift = false;
    let device_state = DeviceState::new();

    sleep(Duration::from_secs(2));

    for (ticks, track_index, event) in merge_tracks(&smf.tracks) {
        let keydowns: Vec<Keycode> = device_state.get_keys();
        if keydowns.contains(&Keycode::Escape) {
            for key in keys.keys() {
                if *keys.get(key).unwrap() {
                    release_key(Key::Emulated(*key));
                }
            }
            if shift {
                release_key(Key::Physical(Physical::Shift));
            }
            break;
        }

        let ms = tick_to_microseconds.convert(ticks, &event);

        // TODO: Make a more complex sleeping system that gives better accuracy
        // and consistency for black keys.
        sleep(Duration::from_micros(ms - last_tick).saturating_sub(SHIFT_WAIT));
        last_tick = ms;
        
        play_event(event, &mut keys, toggle, &mut shift);
    }
}

#[inline(always)]
fn play_event(event: TrackEventKind, keys: &mut HashMap<char, bool>, toggle: bool, shift: &mut bool) {
    if let TrackEventKind::Midi{message, ..} = event {
        match message {
            MidiMessage::NoteOn {key: key_num, ..} => {
                let note = u8::from(key_num) - 36;
                if let Some(key) = NOTES.get(note as usize) {
                    let kchar = *key as char;
                    let up = kchar.is_uppercase() || SPECIAL_KEYS.contains(key);
                    if up && !*shift {
                        sleep(SHIFT_WAIT);
                        press_key(Key::Physical(Physical::Shift));
                        *shift = true;
                    } else if !up && *shift {
                        sleep(SHIFT_WAIT);
                        release_key(Key::Physical(Physical::Shift));
                        *shift = false;
                    }
                    
                    if toggle {
                        if *keys.get(&kchar).unwrap_or(&false) {
                            keys.insert(kchar, false);
                            send_event(kchar, false);
                        } else {
                            keys.insert(kchar, true);
                            send_event(kchar, true);
                        }
                    } else {
                        send_event(kchar, true);
                    }
                } else {
                }
            },
            MidiMessage::NoteOff{key: key_num, ..} => {
                let note = u8::from(key_num) - 36;
                if let Some(key) = NOTES.get(note as usize) {
                    let kchar = *key as char;
                    send_event(kchar, false);
                }
            }
            e => {println!("Unknown event {:?}", e);}
        }
    }
}

fn send_event(key: char, down: bool) {
    if down {
        press_key(Key::Emulated(key));
    } else {
        release_key(Key::Emulated(key));
    }
}