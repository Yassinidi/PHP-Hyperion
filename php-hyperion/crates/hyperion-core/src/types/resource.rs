//! PHP resources — the opaque handles returned by `fopen`, `opendir`, and
//! friends.
//!
//! A resource is not an object: `is_object()` is false, `gettype()` says
//! `"resource"`, and there is no class behind it. Symfony's `StreamOutput`
//! constructor rejects anything that fails `is_resource()`, so Laravel's
//! console cannot be built without this type existing for real.
//!
//! Every resource carries a small integer id, which is what `echo $handle`
//! and `var_dump` show ("Resource id #3"). Ids come from a per-process
//! counter starting at 1, matching PHP's own numbering closely enough that
//! scripts printing them stay readable.

use std::io::{Read, Seek, SeekFrom, Write};
use std::sync::atomic::{AtomicU32, Ordering};

static NEXT_RESOURCE_ID: AtomicU32 = AtomicU32::new(1);

/// What a resource actually wraps.
///
/// The three standard streams are variants rather than `File` handles so that
/// writes to `php://stdout` can be routed into the fibre's output buffer —
/// interleaving correctly with `echo` — instead of racing it on fd 1.
pub struct PhpProcess {
    pub command: String,
    pub child: std::process::Child,
    pub exit_code: Option<i32>,
    pub running: bool,
}

pub enum ResourceKind {
    /// `php://stdout` / `STDOUT`. Writes go to the script's output buffer.
    Stdout,
    /// `php://stderr` / `STDERR`. Writes go straight to fd 2, which is never
    /// part of the program's output.
    Stderr,
    /// `php://stdin` / `STDIN`.
    Stdin,
    /// A real file on disk.
    File(std::fs::File),
    /// `php://memory` and `php://temp`: an in-memory read/write buffer.
    Memory { buf: Vec<u8>, pos: usize },
    /// `php://input` and other read-only synthetic streams, pre-filled.
    Data { buf: Vec<u8>, pos: usize },
    /// Process spawned by `proc_open`
    Process(Box<PhpProcess>),
    /// Process input pipe (ChildStdin)
    PipeStdin(std::process::ChildStdin),
    /// Process output pipe (ChildStdout)
    PipeStdout(std::process::ChildStdout),
    /// Process error pipe (ChildStderr)
    PipeStderr(std::process::ChildStderr),
    /// Process opened via popen (Child + ChildStdout)
    PopenRead(std::process::Child, std::process::ChildStdout),
    /// Process opened via popen (Child + ChildStdin)
    PopenWrite(std::process::Child, std::process::ChildStdin),
    /// cURL single handle
    Curl(usize),
    /// cURL multi handle
    CurlMulti(usize),
    /// The resource has been `fclose`d. Kept rather than freed so a stale
    /// handle reports `is_resource() === false` instead of dangling.
    Closed,
}

pub struct PhpResource {
    pub id: u32,
    pub kind: ResourceKind,
    /// The path or pseudo-path this was opened with, for error messages and
    /// `stream_get_meta_data`.
    pub uri: String,
    /// True once a read has hit end-of-stream, which is what `feof` reports.
    /// PHP's `feof` is *not* "position == length": it only becomes true after
    /// a read attempt has come up short, and scripts loop on that distinction.
    pub eof: bool,
    /// `a`/`a+` modes append regardless of the seek position.
    pub append: bool,
}

impl PhpResource {
    pub fn new(kind: ResourceKind, uri: impl Into<String>) -> Self {
        Self {
            id: NEXT_RESOURCE_ID.fetch_add(1, Ordering::Relaxed),
            kind,
            uri: uri.into(),
            eof: false,
            append: false,
        }
    }

    /// `get_resource_type()`. The strings match PHP so that code branching on
    /// them (rare, but it exists) behaves the same.
    pub fn type_name(&self) -> &'static str {
        match self.kind {
            ResourceKind::Closed => "Unknown",
            ResourceKind::Process(_) => "process",
            ResourceKind::Curl(_) => "curl",
            ResourceKind::CurlMulti(_) => "curl_multi",
            _ => "stream",
        }
    }

    pub fn is_open(&self) -> bool {
        !matches!(self.kind, ResourceKind::Closed)
    }

    /// True when writes should be captured into the fibre's output buffer
    /// rather than handled here. The caller owns that buffer, so it has to do
    /// the append itself.
    pub fn writes_to_script_output(&self) -> bool {
        matches!(self.kind, ResourceKind::Stdout)
    }

    /// Write `data`, returning the number of bytes taken. `Stdout` returns
    /// `None` to signal that the caller must route the bytes through the
    /// script's output buffer instead — see `writes_to_script_output`.
    pub fn write(&mut self, data: &[u8]) -> Option<usize> {
        match &mut self.kind {
            ResourceKind::Stdout => None,
            ResourceKind::Stderr => {
                let mut err = std::io::stderr();
                let _ = err.write_all(data);
                let _ = err.flush();
                Some(data.len())
            }
            ResourceKind::File(f) => {
                if self.append {
                    let _ = f.seek(SeekFrom::End(0));
                }
                match f.write(data) {
                    Ok(n) => Some(n),
                    Err(_) => Some(0),
                }
            }
            ResourceKind::Memory { buf, pos } => {
                // Writing past the end zero-fills the gap, as PHP does.
                if *pos > buf.len() {
                    buf.resize(*pos, 0);
                }
                let end = *pos + data.len();
                if end > buf.len() {
                    buf.resize(end, 0);
                }
                buf[*pos..end].copy_from_slice(data);
                *pos = end;
                Some(data.len())
            }
            ResourceKind::PipeStdin(ref mut stdin) => {
                match stdin.write(data) {
                    Ok(n) => Some(n),
                    Err(_) => Some(0),
                }
            }
            ResourceKind::PopenWrite(_, ref mut stdin) => {
                match stdin.write(data) {
                    Ok(n) => Some(n),
                    Err(_) => Some(0),
                }
            }
            // Read-only, process handle, or closed streams accept nothing.
            ResourceKind::Stdin
            | ResourceKind::Data { .. }
            | ResourceKind::Process(_)
            | ResourceKind::PipeStdout(_)
            | ResourceKind::PipeStderr(_)
            | ResourceKind::PopenRead(_, _)
            | ResourceKind::Curl(_)
            | ResourceKind::CurlMulti(_)
            | ResourceKind::Closed => Some(0),
        }
    }

    /// Read up to `len` bytes. A short read sets `eof`.
    pub fn read(&mut self, len: usize) -> Vec<u8> {
        let out = match &mut self.kind {
            ResourceKind::Stdin => {
                let mut buf = vec![0u8; len];
                match std::io::stdin().read(&mut buf) {
                    Ok(n) => {
                        buf.truncate(n);
                        buf
                    }
                    Err(_) => Vec::new(),
                }
            }
            ResourceKind::File(f) => {
                let mut buf = vec![0u8; len];
                match f.read(&mut buf) {
                    Ok(n) => {
                        buf.truncate(n);
                        buf
                    }
                    Err(_) => Vec::new(),
                }
            }
            ResourceKind::Memory { buf, pos } | ResourceKind::Data { buf, pos } => {
                let start = (*pos).min(buf.len());
                let end = (start + len).min(buf.len());
                *pos = end;
                buf[start..end].to_vec()
            }
            ResourceKind::PipeStdout(ref mut stdout) => {
                let mut buf = vec![0u8; len];
                match stdout.read(&mut buf) {
                    Ok(n) => {
                        buf.truncate(n);
                        buf
                    }
                    Err(_) => Vec::new(),
                }
            }
            ResourceKind::PipeStderr(ref mut stderr) => {
                let mut buf = vec![0u8; len];
                match stderr.read(&mut buf) {
                    Ok(n) => {
                        buf.truncate(n);
                        buf
                    }
                    Err(_) => Vec::new(),
                }
            }
            ResourceKind::PopenRead(_, ref mut stdout) => {
                let mut buf = vec![0u8; len];
                match stdout.read(&mut buf) {
                    Ok(n) => {
                        buf.truncate(n);
                        buf
                    }
                    Err(_) => Vec::new(),
                }
            }
            ResourceKind::Stdout
            | ResourceKind::Stderr
            | ResourceKind::PipeStdin(_)
            | ResourceKind::PopenWrite(_, _)
            | ResourceKind::Process(_)
            | ResourceKind::Curl(_)
            | ResourceKind::CurlMulti(_)
            | ResourceKind::Closed => Vec::new(),
        };
        if out.len() < len {
            self.eof = true;
        }
        out
    }

    /// Read through the next newline, inclusive, as `fgets` does. `None` at
    /// end of stream, which is how `fgets` reports it (`false` in userland).
    pub fn read_line(&mut self, max: Option<usize>) -> Option<Vec<u8>> {
        let mut line = Vec::new();
        loop {
            if let Some(m) = max {
                // `fgets($h, $n)` stops after n-1 bytes, leaving room for the
                // terminator PHP's C implementation writes.
                if m > 0 && line.len() + 1 >= m {
                    break;
                }
            }
            let byte = self.read(1);
            if byte.is_empty() {
                break;
            }
            line.push(byte[0]);
            if byte[0] == b'\n' {
                break;
            }
        }
        if line.is_empty() {
            None
        } else {
            Some(line)
        }
    }

    /// Everything from the current position to the end.
    pub fn read_to_end(&mut self) -> Vec<u8> {
        let mut out = Vec::new();
        match &mut self.kind {
            ResourceKind::File(f) => {
                let _ = f.read_to_end(&mut out);
            }
            ResourceKind::Stdin => {
                let _ = std::io::stdin().read_to_end(&mut out);
            }
            ResourceKind::Memory { buf, pos } | ResourceKind::Data { buf, pos } => {
                let start = (*pos).min(buf.len());
                out = buf[start..].to_vec();
                *pos = buf.len();
            }
            ResourceKind::PipeStdout(ref mut stdout) => {
                let _ = stdout.read_to_end(&mut out);
            }
            ResourceKind::PipeStderr(ref mut stderr) => {
                let _ = stderr.read_to_end(&mut out);
            }
            ResourceKind::PopenRead(_, ref mut stdout) => {
                let _ = stdout.read_to_end(&mut out);
            }
            ResourceKind::Stdout
            | ResourceKind::Stderr
            | ResourceKind::PipeStdin(_)
            | ResourceKind::PopenWrite(_, _)
            | ResourceKind::Process(_)
            | ResourceKind::Curl(_)
            | ResourceKind::CurlMulti(_)
            | ResourceKind::Closed => {}
        }
        self.eof = true;
        out
    }

    /// `stream_get_meta_data()['seekable']`. The console streams are pipes as
    /// far as a script is concerned, so only files and buffers say yes.
    pub fn is_seekable(&self) -> bool {
        matches!(
            self.kind,
            ResourceKind::File(_) | ResourceKind::Memory { .. } | ResourceKind::Data { .. }
        )
    }

    pub fn tell(&mut self) -> Option<usize> {
        match &mut self.kind {
            ResourceKind::File(f) => f.stream_position().ok().map(|p| p as usize),
            ResourceKind::Memory { pos, .. } | ResourceKind::Data { pos, .. } => Some(*pos),
            _ => None,
        }
    }

    /// `fseek`. `whence` uses PHP's SEEK_SET/CUR/END values (0/1/2).
    pub fn seek(&mut self, offset: i64, whence: i32) -> bool {
        // Seeking clears EOF even if the new position is still at the end:
        // PHP only re-raises the flag on the next short read.
        self.eof = false;
        match &mut self.kind {
            ResourceKind::File(f) => {
                let from = match whence {
                    1 => SeekFrom::Current(offset),
                    2 => SeekFrom::End(offset),
                    _ => SeekFrom::Start(offset.max(0) as u64),
                };
                f.seek(from).is_ok()
            }
            ResourceKind::Memory { buf, pos } | ResourceKind::Data { buf, pos } => {
                let base = match whence {
                    1 => *pos as i64,
                    2 => buf.len() as i64,
                    _ => 0,
                };
                let target = base + offset;
                if target < 0 {
                    return false;
                }
                *pos = target as usize;
                true
            }
            _ => false,
        }
    }

    pub fn flush(&mut self) -> bool {
        match &mut self.kind {
            ResourceKind::File(f) => f.flush().is_ok(),
            ResourceKind::Stderr => std::io::stderr().flush().is_ok(),
            ResourceKind::PipeStdin(ref mut stdin) => stdin.flush().is_ok(),
            ResourceKind::PopenWrite(_, ref mut stdin) => stdin.flush().is_ok(),
            // Stdout is the script's own buffer; the SAPI decides when it goes
            // out, so there is nothing to push here.
            _ => true,
        }
    }

    pub fn truncate(&mut self, size: usize) -> bool {
        match &mut self.kind {
            ResourceKind::File(f) => f.set_len(size as u64).is_ok(),
            ResourceKind::Memory { buf, .. } => {
                buf.resize(size, 0);
                true
            }
            _ => false,
        }
    }

    /// Total length where that is knowable, for `fstat`/`stream_get_contents`.
    pub fn len(&self) -> Option<usize> {
        match &self.kind {
            ResourceKind::File(f) => f.metadata().ok().map(|m| m.len() as usize),
            ResourceKind::Memory { buf, .. } | ResourceKind::Data { buf, .. } => Some(buf.len()),
            _ => None,
        }
    }

    pub fn close(&mut self) {
        self.flush();
        self.kind = ResourceKind::Closed;
    }
}

/// Translate a PHP `fopen` mode string into open options.
///
/// Returns `None` for a mode PHP itself would reject. The `b`/`t` suffixes are
/// accepted and ignored — there is no text-mode translation on the platforms
/// this engine targets.
pub fn open_options(mode: &str) -> Option<(std::fs::OpenOptions, bool, bool)> {
    let m: String = mode.chars().filter(|c| *c != 'b' && *c != 't').collect();
    let mut opts = std::fs::OpenOptions::new();
    let mut append = false;
    // (readable, writable) as userland sees it, for rejecting bad fwrite/fread.
    let (readable, writable) = match m.as_str() {
        "r" => {
            opts.read(true);
            (true, false)
        }
        "r+" => {
            opts.read(true).write(true);
            (true, true)
        }
        "w" => {
            opts.write(true).create(true).truncate(true);
            (false, true)
        }
        "w+" => {
            opts.read(true).write(true).create(true).truncate(true);
            (true, true)
        }
        "a" => {
            opts.write(true).create(true).append(true);
            append = true;
            (false, true)
        }
        "a+" => {
            opts.read(true).append(true).create(true);
            append = true;
            (true, true)
        }
        "x" => {
            opts.write(true).create_new(true);
            (false, true)
        }
        "x+" => {
            opts.read(true).write(true).create_new(true);
            (true, true)
        }
        "c" => {
            opts.write(true).create(true);
            (false, true)
        }
        "c+" => {
            opts.read(true).write(true).create(true);
            (true, true)
        }
        _ => return None,
    };
    let _ = (readable, writable);
    Some((opts, append, readable))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn memory_stream_round_trips() {
        let mut r = PhpResource::new(
            ResourceKind::Memory {
                buf: Vec::new(),
                pos: 0,
            },
            "php://memory",
        );
        assert_eq!(r.write(b"hello\nworld"), Some(11));
        assert!(r.seek(0, 0));
        assert_eq!(r.read_line(None).unwrap(), b"hello\n");
        assert_eq!(r.read_to_end(), b"world");
    }

    #[test]
    fn eof_only_after_a_short_read() {
        let mut r = PhpResource::new(
            ResourceKind::Data {
                buf: b"ab".to_vec(),
                pos: 0,
            },
            "php://input",
        );
        assert_eq!(r.read(2), b"ab");
        // Exactly-consumed is not yet EOF in PHP; the next read sets it.
        assert!(!r.eof);
        assert!(r.read(1).is_empty());
        assert!(r.eof);
    }

    #[test]
    fn writing_past_the_end_zero_fills() {
        let mut r = PhpResource::new(
            ResourceKind::Memory {
                buf: Vec::new(),
                pos: 0,
            },
            "php://memory",
        );
        r.write(b"ab");
        r.seek(4, 0);
        r.write(b"z");
        r.seek(0, 0);
        assert_eq!(r.read_to_end(), b"ab\0\0z");
    }

    #[test]
    fn closed_resource_reports_shut() {
        let mut r = PhpResource::new(ResourceKind::Stderr, "php://stderr");
        assert!(r.is_open());
        assert_eq!(r.type_name(), "stream");
        r.close();
        assert!(!r.is_open());
        assert_eq!(r.type_name(), "Unknown");
    }

    #[test]
    fn ids_are_distinct_and_start_above_zero() {
        let a = PhpResource::new(ResourceKind::Stderr, "php://stderr");
        let b = PhpResource::new(ResourceKind::Stderr, "php://stderr");
        assert!(a.id >= 1);
        assert_ne!(a.id, b.id);
    }

    #[test]
    fn unknown_mode_is_rejected() {
        assert!(open_options("q").is_none());
        assert!(open_options("rb").is_some());
        assert!(open_options("w+b").is_some());
    }
}
