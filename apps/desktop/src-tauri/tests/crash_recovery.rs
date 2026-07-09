//! Crash-recovery suite (PRD P0-4 hard gate; non-negotiable #2).
//!
//! Proves the write-ahead property with a REAL process kill: a child process
//! (this same test binary, re-invoked with an env marker) streams audio into a
//! `WalWriter` and is SIGKILLed mid-write — no destructors, no finalize, the
//! moral equivalent of `kill -9` mid-dictation. The parent then recovers the
//! orphan and asserts the audio survived.

use kaydence_lib::audio::wal;
use std::io::Read;
use std::path::PathBuf;
use std::time::{Duration, Instant};

const CHILD_ENV: &str = "KAYDENCE_WAL_CHILD_DIR";
/// Child appends this many samples per write (20 ms @ 16 kHz).
const CHUNK: usize = 320;
/// Parent kills once the file holds at least this many samples (0.5 s).
const KILL_THRESHOLD_SAMPLES: u64 = 8_000;

/// Child mode: not a real test in normal runs (returns immediately when the
/// env marker is absent). When re-invoked by `killing_the_writer_mid_dictation
/// _loses_no_flushed_audio` with the marker set, it writes forever until killed.
#[test]
fn crash_recovery_child_writer_process() {
    let Ok(dir) = std::env::var(CHILD_ENV) else {
        return; // normal test run: nothing to do
    };
    let dir = PathBuf::from(dir);
    let mut w = wal::WalWriter::create(&dir, "01CRASH").expect("child: create wal");
    let chunk = vec![0.25f32; CHUNK];
    loop {
        w.append(&chunk).expect("child: append");
        // Real capture paces at the device rate; pace the child similarly so
        // the parent's kill lands genuinely mid-stream.
        std::thread::sleep(Duration::from_millis(2));
    }
}

#[test]
fn crash_recovery_killing_the_writer_mid_dictation_loses_no_flushed_audio() {
    let dir = std::env::temp_dir().join(format!("kaydence-crash-test-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("mk tmp");
    let wal_path = dir.join("01CRASH.wav");

    // Re-invoke this test binary in child mode, filtered to the writer "test".
    let exe = std::env::current_exe().expect("current_exe");
    let mut child = std::process::Command::new(exe)
        .args([
            "crash_recovery_child_writer_process",
            "--exact",
            "--nocapture",
        ])
        .env(CHILD_ENV, &dir)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .expect("spawn child writer");

    // Wait for the WAL to grow past the threshold, then SIGKILL mid-write.
    let deadline = Instant::now() + Duration::from_secs(30);
    let threshold_bytes = 44 + KILL_THRESHOLD_SAMPLES * 2;
    loop {
        if let Ok(md) = std::fs::metadata(&wal_path) {
            if md.len() >= threshold_bytes {
                break;
            }
        }
        assert!(
            Instant::now() < deadline,
            "child never reached {KILL_THRESHOLD_SAMPLES} samples"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
    child.kill().expect("SIGKILL child"); // kill(2) SIGKILL on unix
    child.wait().expect("reap child");

    let len_at_kill = std::fs::metadata(&wal_path).expect("wal exists").len();

    // Recover the orphaned session directory, as app launch would.
    let recovered = wal::recover_dir(&dir).expect("recover_dir");
    assert_eq!(recovered.len(), 1, "exactly one orphan recovered");
    let r = &recovered[0];
    assert!(
        r.samples >= KILL_THRESHOLD_SAMPLES,
        "recovered {} samples, expected >= {KILL_THRESHOLD_SAMPLES}",
        r.samples
    );
    // Recovery never invents data: sample count fits within the killed file.
    assert!(r.samples * 2 + 44 <= len_at_kill + 1, "no invented samples");

    // The recovered file is a playable WAV: RIFF magic + header sizes that
    // match the actual byte length.
    let mut f = std::fs::File::open(&r.path).expect("open recovered");
    let mut header = [0u8; 44];
    f.read_exact(&mut header).expect("read header");
    assert_eq!(&header[0..4], b"RIFF");
    let data_len = u32::from_le_bytes(header[40..44].try_into().expect("4 bytes")) as u64;
    let actual = std::fs::metadata(&r.path).expect("md").len();
    assert_eq!(actual, 44 + data_len, "header matches actual length");
    assert_eq!(data_len / 2, r.samples);

    let _ = std::fs::remove_dir_all(&dir);
}
