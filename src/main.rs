use anyhow::{Context, Result, bail, ensure};
use ohmyloop::{
    audio::{Audio, write_wav},
    dsp::{Controls, Dsp},
    engine::{LoopRegion, render},
    playback::Player,
    session::Session,
};
use std::{
    env,
    io::{self, BufRead, Write},
    path::{Path, PathBuf},
    sync::{Arc, atomic::Ordering},
    thread,
    time::{Duration, Instant},
};

fn main() -> Result<()> {
    let args: Vec<String> = env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("play") => interactive(&args[1..]),
        Some("render") => render_command(&args[1..]),
        Some("probe") => probe(&args[1..]),
        Some("smoke") => smoke(&args[1..]),
        Some("demo") => write_wav(
            Path::new(args.get(1).context("Usage: demo output.wav")?),
            48_000,
            &Audio::demo().frames,
        ),
        Some("help" | "--help" | "-h") | None => {
            usage();
            Ok(())
        }
        Some(command) => bail!("Unknown command {command}; try --help"),
    }
}

fn usage() {
    println!(
        "OhMyLoop — audio risk prototype\n\n\
        ohmyloop play <song-file|--demo>\n\
        ohmyloop render <song-file|--demo> <output.wav> [--speed N] [--pitch N]\n\
          [--start SECONDS] [--loop A B] [--seconds N] [--fade MS]\n\
        ohmyloop probe [output-directory]\n\
        ohmyloop smoke [song-file|--demo]\n\
        ohmyloop demo output.wav\n\n\
        play starts paused; type help for transport commands.\n\
        speed: 0.25–2.0; pitch: -12–12 semitones (fractional values allowed)."
    );
}

fn source(value: &str) -> Result<Audio> {
    if value == "--demo" {
        Ok(Audio::demo())
    } else {
        Audio::load(Path::new(value))
    }
}

fn interactive(args: &[String]) -> Result<()> {
    ensure!(args.len() == 1, "Usage: play <file|--demo>");
    let song = &args[0];
    let player = Player::open(source(song)?, true)?;
    let mut session = Session::new(song.clone());
    println!(
        "{} | source {} Hz → output {} Hz | buffer {} frames (0 = default)\nSong: {:.3}s; paused. Type help.",
        player.device_name,
        player.source_rate,
        player.rate,
        player.device_buffer_frames,
        player.duration
    );
    if player.source_rate != player.rate {
        println!("Prototype fallback: linear sample-rate conversion; quality remains unvalidated.");
    }
    let stdin = io::stdin();
    let mut lines = stdin.lock().lines();
    loop {
        print!("ohmyloop> ");
        io::stdout().flush()?;
        let Some(line) = lines.next() else {
            break;
        };
        let line = line?;
        let tokens: Vec<&str> = line.split_whitespace().collect();
        if tokens
            .first()
            .is_some_and(|s| matches!(*s, "quit" | "q" | "exit"))
        {
            break;
        }
        if let Err(error) = transport_command(&tokens, &player, &mut session) {
            eprintln!("{error:#}");
        }
    }
    Ok(())
}

fn number(tokens: &[&str], index: usize, usage: &str) -> Result<f64> {
    tokens
        .get(index)
        .context(usage.to_string())?
        .parse()
        .context("Expected a number")
}

fn transport_command(tokens: &[&str], player: &Player, session: &mut Session) -> Result<()> {
    let Some(&command) = tokens.first() else {
        return Ok(());
    };
    match command {
        "help" => println!(
            "play | pause | speed 0.5 | pitch -3 | seek 10 | status\n\
            mark NAME [SECONDS] | markers | jump NAME | loop A B [FADE_MS] | loop off\n\
            save PATH | load PATH | quit\n\
            Loop A/B can be marker names or source seconds. Default seam fade: 5 ms.\n\
            Output position is estimated; it excludes downstream hardware latency."
        ),
        "play" => player.play()?,
        "pause" => player.pause(),
        "status" => println!(
            "{}\nspeed {:.3} | pitch {:.3} st | loop {:?}",
            player.status(),
            session.controls.speed,
            session.controls.semitones,
            session.active_loop
        ),
        "speed" | "pitch" => {
            let mut controls = session.controls;
            let value = number(tokens, 1, "Usage: speed N | pitch N")?;
            if command == "speed" {
                controls.speed = value;
            } else {
                controls.semitones = value;
            }
            player.set_controls(controls)?;
            session.controls = controls;
        }
        "seek" => player.seek(number(tokens, 1, "Usage: seek SECONDS")?)?,
        "mark" => {
            let name = tokens.get(1).context("Usage: mark NAME [SECONDS]")?;
            ensure!(!name.is_empty() && name.len() <= 128, "Invalid marker name");
            let seconds = if tokens.len() > 2 {
                number(tokens, 2, "Invalid marker position")?
            } else {
                player.shared.position() / player.rate as f64
            };
            ensure!(
                seconds.is_finite() && (0.0..=player.duration).contains(&seconds),
                "Marker outside song"
            );
            session.markers.insert(name.to_string(), seconds);
            println!("{name}: {seconds:.3}s in source timeline");
        }
        "markers" => {
            for (name, position) in &session.markers {
                println!("{name}: {position:.3}s");
            }
        }
        "jump" => player.seek(
            *session
                .markers
                .get(*tokens.get(1).context("Usage: jump NAME")?)
                .context("Unknown marker")?,
        )?,
        "loop" => {
            let region = if tokens.get(1) == Some(&"off") {
                None
            } else {
                let bound = |i: usize| -> Result<f64> {
                    let token = tokens.get(i).context("Usage: loop A B [FADE_MS]")?;
                    if let Some(value) = session.markers.get(*token) {
                        Ok(*value)
                    } else {
                        Ok(token.parse().context("Expected marker or seconds")?)
                    }
                };
                Some(LoopRegion {
                    start: bound(1)?,
                    end: bound(2)?,
                    fade_ms: if tokens.len() > 3 {
                        number(tokens, 3, "Invalid fade")?
                    } else {
                        5.0
                    },
                })
            };
            player.set_loop(region)?;
            session.active_loop = region;
        }
        "save" => session.save(
            Path::new(tokens.get(1).context("Usage: save PATH")?),
            player.duration,
        )?,
        "load" => {
            let loaded = Session::load(
                Path::new(tokens.get(1).context("Usage: load PATH")?),
                &session.song,
                player.duration,
            )?;
            player.set_controls(loaded.controls)?;
            player.set_loop(loaded.active_loop)?;
            *session = loaded;
        }
        _ => bail!("Unknown command; type help"),
    }
    Ok(())
}

fn render_command(args: &[String]) -> Result<()> {
    ensure!(
        args.len() >= 2,
        "Usage: render <file|--demo> <output.wav> [options]"
    );
    let audio = Arc::new(source(&args[0])?);
    let mut controls = Controls::default();
    let mut position = 0.0;
    let mut bounds = None;
    let mut seconds = None;
    let mut fade_ms = 5.0;
    let mut index = 2;
    while index < args.len() {
        let value = |offset: usize| -> Result<f64> {
            Ok(args
                .get(index + offset)
                .context("Missing option value")?
                .parse()?)
        };
        match args[index].as_str() {
            "--speed" => {
                controls.speed = value(1)?;
                index += 2;
            }
            "--pitch" => {
                controls.semitones = value(1)?;
                index += 2;
            }
            "--start" => {
                position = value(1)?;
                index += 2;
            }
            "--seconds" => {
                seconds = Some(value(1)?);
                index += 2;
            }
            "--fade" => {
                fade_ms = value(1)?;
                index += 2;
            }
            "--loop" => {
                bounds = Some((value(1)?, value(2)?));
                index += 3;
            }
            option => bail!("Unknown render option {option}"),
        }
    }
    let region = bounds.map(|(start, end)| LoopRegion {
        start,
        end,
        fade_ms,
    });
    if let Some(seconds) = seconds {
        ensure!(
            seconds.is_finite() && (0.001..=600.0).contains(&seconds),
            "Output seconds must be 0.001–600"
        );
    }
    let limit = seconds.map(|s| (s * audio.rate as f64).round() as usize);
    let started = Instant::now();
    let output = render(audio.clone(), controls, position, region, limit, true)?;
    write_wav(Path::new(&args[1]), audio.rate, &output)?;
    println!(
        "Rendered {:.3}s in {:.3}s → {}",
        output.len() as f64 / audio.rate as f64,
        started.elapsed().as_secs_f64(),
        args[1]
    );
    Ok(())
}

// Steady sine-wave estimate, not a general music pitch detector.
fn tone_hz(frames: &[[f32; 2]], rate: u32) -> f64 {
    let skip = frames.len() / 4;
    let central = &frames[skip..frames.len() - skip];
    let mut crossings = Vec::new();
    for (n, pair) in central.windows(2).enumerate() {
        if pair[0][0] <= 0.0 && pair[1][0] > 0.0 {
            crossings.push(n as f64 - pair[0][0] as f64 / (pair[1][0] - pair[0][0]) as f64);
        }
    }
    if crossings.len() < 2 {
        return 0.0;
    }
    (crossings.len() - 1) as f64 * rate as f64 / (crossings.last().unwrap() - crossings[0])
}

fn probe(args: &[String]) -> Result<()> {
    ensure!(args.len() <= 1, "Usage: probe [output-directory]");
    let directory = PathBuf::from(
        args.first()
            .map(String::as_str)
            .unwrap_or("artifacts/probe"),
    );
    std::fs::create_dir_all(&directory)?;
    let audio = Arc::new(Audio::tone(48_000, 4.0, 440.0));
    let mut rows = Vec::new();
    let matrix = [
        (1.0, 0.0),
        (0.5, 0.0),
        (1.0, 7.0),
        (0.5, -5.0),
        (0.25, -12.0),
        (2.0, 12.0),
    ];
    for finer in [true, false] {
        for (speed, semitones) in matrix {
            let controls = Controls { speed, semitones };
            let dsp = Dsp::new(audio.rate, controls, finer)?;
            let start = Instant::now();
            let output = render(audio.clone(), controls, 0.0, None, None, finer)?;
            let compute_seconds = start.elapsed().as_secs_f64();
            let duration = output.len() as f64 / audio.rate as f64;
            let expected_duration = 4.0 / speed;
            let hz = tone_hz(&output, audio.rate);
            let expected_hz = 440.0 * controls.pitch_scale();
            let cents_error = 1200.0 * (hz / expected_hz).log2();
            let passed = (duration - expected_duration).abs() < 0.05
                && cents_error.abs() < 10.0
                && output.iter().flatten().all(|x| x.is_finite());
            let engine = if finer { "R3" } else { "R2" };
            println!(
                "{engine} speed {speed:.2} pitch {semitones:+.1}: {duration:.4}s, {hz:.2} Hz, {cents_error:+.2} cents, render {compute_seconds:.3}s, {}",
                if passed { "PASS" } else { "FAIL" }
            );
            rows.push(serde_json::json!({ "engine": engine, "speed": speed, "semitones": semitones, "duration_seconds": duration,
                "expected_duration_seconds": expected_duration, "frequency_hz": hz, "expected_frequency_hz": expected_hz,
                "zero_crossing_error_cents": cents_error, "required_for_playback": finer, "compute_seconds": compute_seconds, "compute_per_audio_second": compute_seconds / duration,
                "startup_pad_frames": dsp.pad, "startup_delay_frames": dsp.delay, "passed": passed }));
        }
    }
    let demo = Arc::new(Audio::demo());
    write_wav(
        &directory.join("demo-original.wav"),
        demo.rate,
        &demo.frames,
    )?;
    for (name, controls, region) in [
        (
            "demo-half-speed.wav",
            Controls {
                speed: 0.5,
                semitones: 0.0,
            },
            None,
        ),
        (
            "demo-transposed.wav",
            Controls {
                speed: 1.0,
                semitones: -5.0,
            },
            None,
        ),
        (
            "demo-loop.wav",
            Controls {
                speed: 0.7,
                semitones: 3.0,
            },
            Some(LoopRegion {
                start: 1.17,
                end: 2.03,
                fade_ms: 5.0,
            }),
        ),
    ] {
        let limit = region.map(|_| 48_000 * 8);
        let output = render(demo.clone(), controls, 0.0, region, limit, true)?;
        write_wav(&directory.join(name), demo.rate, &output)?;
    }
    std::fs::write(
        directory.join("measurements.json"),
        serde_json::to_string_pretty(&serde_json::json!({
            "platform": env::consts::OS, "architecture": env::consts::ARCH, "rubberband": "4.0.0",
            "note": "Synthetic checks, not subjective quality or hardware latency measurements", "tone_checks": rows,
        }))?,
    )?;
    ensure!(
        rows.iter()
            .filter(|r| r["required_for_playback"] == true)
            .all(|r| r["passed"] == true),
        "R3 playback checks failed; see measurements.json (R2 is comparison only)"
    );
    println!(
        "Listening fixtures and measurements: {}",
        directory.display()
    );
    Ok(())
}

fn smoke(args: &[String]) -> Result<()> {
    ensure!(args.len() <= 1, "Usage: smoke [song-file|--demo]");
    let song = args.first().map(String::as_str).unwrap_or("--demo");
    let player = Player::open(source(song)?, true)?;
    ensure!(
        player.duration >= 4.0,
        "Smoke test needs at least four seconds of audio"
    );
    println!(
        "Device smoke test: {} ({} Hz; source {} Hz)",
        player.device_name, player.rate, player.source_rate
    );
    player.play()?;
    thread::sleep(Duration::from_secs(1));
    player.set_controls(Controls {
        speed: 0.5,
        semitones: 0.0,
    })?;
    thread::sleep(Duration::from_secs(1));
    player.set_controls(Controls {
        speed: 0.75,
        semitones: -5.0,
    })?;
    player.seek(1.0)?;
    thread::sleep(Duration::from_secs(1));
    player.set_loop(Some(LoopRegion {
        start: 1.17,
        end: 1.67,
        fade_ms: 5.0,
    }))?;
    thread::sleep(Duration::from_secs(2));
    player.pause();
    println!("{}", player.status());
    ensure!(
        player.shared.worker_error.lock().unwrap().is_none(),
        "Audio worker failed"
    );
    ensure!(
        player.shared.callbacks.load(Ordering::Relaxed) > 0,
        "Device made no callbacks"
    );
    ensure!(
        player.shared.output_frames.load(Ordering::Relaxed) > 0,
        "No audio reached callbacks"
    );
    ensure!(
        player.shared.device_errors.load(Ordering::Relaxed) == 0,
        "Device reported errors"
    );
    println!(
        "PASS: callback, live controls, seek, and looping executed. Listen separately for quality."
    );
    Ok(())
}
