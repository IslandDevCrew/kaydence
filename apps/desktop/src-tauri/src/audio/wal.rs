//! Write-ahead audio log (WAL) — non-negotiable #2: never lose a word.
//!
//! Samples hit this on-disk file *before* the ASR engine sees them
//! (`audio/AGENTS.md` invariant 2). A `kill -9` at any moment must leave a
//! recoverable, playable file — enforced by the crash-recovery test in
//! `tests/` below, which SIGKILLs a real child process mid-write.
//!
//! Format: standard 44-byte PCM WAV header + little-endian samples. The header
//! is written first with placeholder sizes; sizes are patched on `finalize()`.
//! Because a crash leaves stale placeholder sizes, `recover()` patches the
//! header from the *actual* file length — a truncated file is still playable
//! up to its last flushed sample. The header is hand-rolled (44 fixed bytes,
//! fully covered by tests) to keep the WAL core dependency-free.
//!
//! fsync discipline (invariant 2): flush+sync at every `SYNC_INTERVAL_SAMPLES`
//! boundary and at finalize. Between syncs the OS page cache may hold up to
//! ~5 s of audio; a *machine* crash can lose only that window, a *process*
//! crash loses nothing already `write()`ten.

use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

/// Engines expect 16 kHz mono f32 at the module boundary (`audio/AGENTS.md`);
/// the WAL stores i16 PCM (half the bytes, universally playable).
pub const SAMPLE_RATE: u32 = 16_000;
pub const CHANNELS: u16 = 1;
pub const BITS_PER_SAMPLE: u16 = 16;

/// Sync every 5 s of audio (invariant 2: "fsync at every 5 s mark").
pub const SYNC_INTERVAL_SAMPLES: u64 = (SAMPLE_RATE as u64) * 5;

const HEADER_LEN: u64 = 44;

/// Errors from the WAL layer. No `unwrap()` outside tests (root §9).
#[derive(Debug, thiserror::Error)]
pub enum WalError {
    #[error("wal io: {0}")]
    Io(#[from] std::io::Error),
    #[error("not a recoverable wal file: {0}")]
    NotRecoverable(String),
}

/// An open write-ahead session file. One per dictation session.
pub struct WalWriter {
    file: File,
    path: PathBuf,
    samples_written: u64,
    samples_since_sync: u64,
}

impl WalWriter {
    /// Create `sessions/{session}.wav` under `dir`, writing the header
    /// immediately so the file is identifiable from byte 0.
    pub fn create(dir: &Path, session: &str) -> Result<Self, WalError> {
        std::fs::create_dir_all(dir)?;
        let path = dir.join(format!("{session}.wav"));
        let mut file = OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .read(true)
            .open(&path)?;
        file.write_all(&wav_header(0))?;
        file.sync_all()?;
        Ok(Self {
            file,
            path,
            samples_written: 0,
            samples_since_sync: 0,
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn samples_written(&self) -> u64 {
        self.samples_written
    }

    /// Append f32 samples (capture format), stored as i16 PCM. Write-ahead:
    /// callers MUST invoke this before handing the same samples to the engine.
    pub fn append(&mut self, samples: &[f32]) -> Result<(), WalError> {
        // Convert on the capture task (not the RT callback — that only feeds
        // the ring buffer; see audio/AGENTS.md invariant 1).
        let mut buf = Vec::with_capacity(samples.len() * 2);
        for &s in samples {
            let clamped = (s.clamp(-1.0, 1.0) * i16::MAX as f32) as i16;
            buf.extend_from_slice(&clamped.to_le_bytes());
        }
        self.file.write_all(&buf)?;
        self.samples_written += samples.len() as u64;
        self.samples_since_sync += samples.len() as u64;
        if self.samples_since_sync >= SYNC_INTERVAL_SAMPLES {
            self.file.sync_data()?;
            self.samples_since_sync = 0;
        }
        Ok(())
    }

    /// Patch the header sizes and fsync. The file is complete and playable.
    pub fn finalize(mut self) -> Result<PathBuf, WalError> {
        let data_len = self.samples_written * 2;
        self.file.seek(SeekFrom::Start(0))?;
        self.file.write_all(&wav_header(data_len as u32))?;
        self.file.sync_all()?;
        Ok(self.path)
    }
}

/// Outcome of recovering one orphaned WAL file after a crash.
#[derive(Debug, PartialEq, Eq)]
pub struct Recovered {
    pub path: PathBuf,
    pub samples: u64,
}

#[derive(Debug, PartialEq)]
pub struct WalSamples {
    pub path: PathBuf,
    pub sample_rate: u32,
    pub samples: Vec<f32>,
}

/// Patch a (possibly crash-truncated) WAL file's header from its actual length
/// so it is playable, returning the recovered sample count. Idempotent.
pub fn recover(path: &Path) -> Result<Recovered, WalError> {
    let mut file = OpenOptions::new().read(true).write(true).open(path)?;
    let actual_len = file.metadata()?.len();
    if actual_len < HEADER_LEN {
        return Err(WalError::NotRecoverable(format!(
            "{}: {} bytes is shorter than a WAV header",
            path.display(),
            actual_len
        )));
    }
    let mut magic = [0u8; 4];
    file.read_exact(&mut magic)?;
    if &magic != b"RIFF" {
        return Err(WalError::NotRecoverable(format!(
            "{}: missing RIFF magic",
            path.display()
        )));
    }
    file.seek(SeekFrom::Start(0))?;
    let mut header = [0u8; HEADER_LEN as usize];
    file.read_exact(&mut header)?;
    validate_recoverable_header_shape(path, &header)?;
    // Truncate any torn trailing byte (i16 samples are 2 bytes).
    let data_len = (actual_len - HEADER_LEN) & !1;
    file.set_len(HEADER_LEN + data_len)?;
    file.seek(SeekFrom::Start(0))?;
    file.write_all(&wav_header(data_len as u32))?;
    file.sync_all()?;
    Ok(Recovered {
        path: path.to_path_buf(),
        samples: data_len / 2,
    })
}

/// Recover and read a WAL file back into the engine-facing 16 kHz mono f32
/// boundary. This is intentionally separate from `WalWriter`: ASR consumes only
/// audio that has already landed on disk.
pub fn read_samples(path: &Path) -> Result<WalSamples, WalError> {
    let recovered = recover(path)?;
    let bytes = std::fs::read(path)?;
    validate_wav_header(path, &bytes)?;
    let data_len = u32::from_le_bytes(bytes[40..44].try_into().unwrap()) as usize;
    let samples = bytes[HEADER_LEN as usize..HEADER_LEN as usize + data_len]
        .chunks_exact(2)
        .map(|chunk| {
            let sample = i16::from_le_bytes([chunk[0], chunk[1]]);
            (sample as f32 / i16::MAX as f32).clamp(-1.0, 1.0)
        })
        .collect::<Vec<_>>();

    debug_assert_eq!(samples.len() as u64, recovered.samples);
    Ok(WalSamples {
        path: recovered.path,
        sample_rate: SAMPLE_RATE,
        samples,
    })
}

/// Scan a sessions directory for WAL files and recover each in place.
/// Called on launch: orphans are then re-transcribed or surfaced by history/
/// (`history/AGENTS.md`). Unrecoverable files are skipped, never deleted —
/// deletion is the user's call (non-negotiable #2).
pub fn recover_dir(dir: &Path) -> Result<Vec<Recovered>, WalError> {
    let mut out = Vec::new();
    if !dir.exists() {
        return Ok(out);
    }
    for entry in std::fs::read_dir(dir)? {
        let path = entry?.path();
        if path.extension().and_then(|e| e.to_str()) == Some("wav") {
            if let Ok(r) = recover(&path) {
                out.push(r);
            }
        }
    }
    out.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(out)
}

/// The 44-byte canonical PCM WAV header for our fixed format.
fn wav_header(data_len: u32) -> [u8; 44] {
    let byte_rate = SAMPLE_RATE * CHANNELS as u32 * (BITS_PER_SAMPLE as u32 / 8);
    let block_align = CHANNELS * (BITS_PER_SAMPLE / 8);
    let mut h = [0u8; 44];
    h[0..4].copy_from_slice(b"RIFF");
    h[4..8].copy_from_slice(&(36 + data_len).to_le_bytes());
    h[8..12].copy_from_slice(b"WAVE");
    h[12..16].copy_from_slice(b"fmt ");
    h[16..20].copy_from_slice(&16u32.to_le_bytes()); // PCM fmt chunk size
    h[20..22].copy_from_slice(&1u16.to_le_bytes()); // PCM
    h[22..24].copy_from_slice(&CHANNELS.to_le_bytes());
    h[24..28].copy_from_slice(&SAMPLE_RATE.to_le_bytes());
    h[28..32].copy_from_slice(&byte_rate.to_le_bytes());
    h[32..34].copy_from_slice(&block_align.to_le_bytes());
    h[34..36].copy_from_slice(&BITS_PER_SAMPLE.to_le_bytes());
    h[36..40].copy_from_slice(b"data");
    h[40..44].copy_from_slice(&data_len.to_le_bytes());
    h
}

fn validate_wav_header(path: &Path, bytes: &[u8]) -> Result<(), WalError> {
    if bytes.len() < HEADER_LEN as usize {
        return Err(WalError::NotRecoverable(format!(
            "{}: {} bytes is shorter than a WAV header",
            path.display(),
            bytes.len()
        )));
    }

    if &bytes[0..4] != b"RIFF" || &bytes[8..12] != b"WAVE" || &bytes[12..16] != b"fmt " {
        return Err(WalError::NotRecoverable(format!(
            "{}: missing canonical WAV header",
            path.display()
        )));
    }
    if &bytes[36..40] != b"data" {
        return Err(WalError::NotRecoverable(format!(
            "{}: missing data chunk",
            path.display()
        )));
    }

    let pcm = u16::from_le_bytes(bytes[20..22].try_into().unwrap());
    let channels = u16::from_le_bytes(bytes[22..24].try_into().unwrap());
    let sample_rate = u32::from_le_bytes(bytes[24..28].try_into().unwrap());
    let bits = u16::from_le_bytes(bytes[34..36].try_into().unwrap());
    let data_len = u32::from_le_bytes(bytes[40..44].try_into().unwrap()) as usize;
    if pcm != 1 || channels != CHANNELS || sample_rate != SAMPLE_RATE || bits != BITS_PER_SAMPLE {
        return Err(WalError::NotRecoverable(format!(
            "{}: unsupported WAV format pcm={} channels={} rate={} bits={}",
            path.display(),
            pcm,
            channels,
            sample_rate,
            bits
        )));
    }
    if HEADER_LEN as usize + data_len != bytes.len() {
        return Err(WalError::NotRecoverable(format!(
            "{}: data length {} does not match file length {}",
            path.display(),
            data_len,
            bytes.len()
        )));
    }
    Ok(())
}

fn validate_recoverable_header_shape(
    path: &Path,
    header: &[u8; HEADER_LEN as usize],
) -> Result<(), WalError> {
    if &header[0..4] != b"RIFF"
        || &header[8..12] != b"WAVE"
        || &header[12..16] != b"fmt "
        || &header[36..40] != b"data"
    {
        return Err(WalError::NotRecoverable(format!(
            "{}: missing canonical WAV header",
            path.display()
        )));
    }

    let pcm = u16::from_le_bytes(header[20..22].try_into().unwrap());
    let channels = u16::from_le_bytes(header[22..24].try_into().unwrap());
    let sample_rate = u32::from_le_bytes(header[24..28].try_into().unwrap());
    let bits = u16::from_le_bytes(header[34..36].try_into().unwrap());
    if pcm != 1 || channels != CHANNELS || sample_rate != SAMPLE_RATE || bits != BITS_PER_SAMPLE {
        return Err(WalError::NotRecoverable(format!(
            "{}: unsupported WAV format pcm={} channels={} rate={} bits={}",
            path.display(),
            pcm,
            channels,
            sample_rate,
            bits
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp() -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "kaydence-wal-test-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    /// Parse the fixed header fields we write, for assertions.
    fn parse_header(bytes: &[u8]) -> (u32, u16, u16, u32) {
        let rate = u32::from_le_bytes(bytes[24..28].try_into().unwrap());
        let ch = u16::from_le_bytes(bytes[22..24].try_into().unwrap());
        let bits = u16::from_le_bytes(bytes[34..36].try_into().unwrap());
        let data_len = u32::from_le_bytes(bytes[40..44].try_into().unwrap());
        (rate, ch, bits, data_len)
    }

    #[test]
    fn finalize_produces_playable_wav_with_correct_sizes() {
        let dir = tmp();
        let mut w = WalWriter::create(&dir, "01TEST").unwrap();
        let samples: Vec<f32> = (0..SAMPLE_RATE).map(|i| (i as f32 * 0.001).sin()).collect();
        w.append(&samples).unwrap();
        let path = w.finalize().unwrap();

        let bytes = std::fs::read(&path).unwrap();
        assert_eq!(&bytes[0..4], b"RIFF");
        let (rate, ch, bits, data_len) = parse_header(&bytes);
        assert_eq!((rate, ch, bits), (SAMPLE_RATE, CHANNELS, BITS_PER_SAMPLE));
        assert_eq!(data_len as usize, samples.len() * 2);
        assert_eq!(bytes.len() as u64, HEADER_LEN + data_len as u64);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn recover_patches_a_torn_file_from_actual_length() {
        let dir = tmp();
        let mut w = WalWriter::create(&dir, "01TORN").unwrap();
        w.append(&vec![0.5f32; 4000]).unwrap();
        let path = w.path().to_path_buf();
        drop(w); // simulate crash: no finalize, header still says data_len=0

        // Tear the file: add one torn trailing byte like an interrupted write.
        let mut f = OpenOptions::new().append(true).open(&path).unwrap();
        f.write_all(&[0xAB]).unwrap();
        drop(f);

        let r = recover(&path).unwrap();
        assert_eq!(r.samples, 4000);
        let bytes = std::fs::read(&path).unwrap();
        let (_, _, _, data_len) = parse_header(&bytes);
        assert_eq!(data_len, 8000); // torn byte truncated, sizes patched
        assert_eq!(bytes.len() as u64, HEADER_LEN + 8000);

        // Idempotent.
        let r2 = recover(&path).unwrap();
        assert_eq!(r2.samples, 4000);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn recover_dir_skips_garbage_and_recovers_the_rest() {
        let dir = tmp();
        let mut w = WalWriter::create(&dir, "01GOOD").unwrap();
        w.append(&vec![0.1f32; 100]).unwrap();
        drop(w); // orphan
        std::fs::write(dir.join("junk.wav"), b"not a wav").unwrap();
        std::fs::write(dir.join("noise.txt"), b"ignored").unwrap();

        let recovered = recover_dir(&dir).unwrap();
        assert_eq!(recovered.len(), 1);
        assert_eq!(recovered[0].samples, 100);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn short_utterance_survives_without_finalize() {
        // Pitfall P1 floor: a 0.3 s clip written before a crash is recoverable.
        let dir = tmp();
        let n = (SAMPLE_RATE as usize) * 3 / 10; // 0.3 s
        let mut w = WalWriter::create(&dir, "01SHORT").unwrap();
        w.append(&vec![0.2f32; n]).unwrap();
        let path = w.path().to_path_buf();
        drop(w);
        let r = recover(&path).unwrap();
        assert_eq!(r.samples as usize, n);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn read_samples_recovers_and_decodes_wal_for_engine_input() {
        let dir = tmp();
        let expected = [-1.0f32, -0.5, 0.0, 0.5, 1.0];
        let mut w = WalWriter::create(&dir, "01READ").unwrap();
        w.append(&expected).unwrap();
        let path = w.path().to_path_buf();
        drop(w); // no finalize: read_samples must recover before reading

        let decoded = read_samples(&path).unwrap();

        assert_eq!(decoded.path, path);
        assert_eq!(decoded.sample_rate, SAMPLE_RATE);
        assert_eq!(decoded.samples.len(), expected.len());
        for (actual, expected) in decoded.samples.iter().zip(expected) {
            assert!((actual - expected).abs() < 0.001);
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn read_samples_rejects_wrong_wav_shape() {
        let dir = tmp();
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("bad.wav");
        let mut header = wav_header(0);
        header[22..24].copy_from_slice(&2u16.to_le_bytes());
        std::fs::write(&path, header).unwrap();

        let err = read_samples(&path).unwrap_err();

        assert!(matches!(err, WalError::NotRecoverable(_)));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
