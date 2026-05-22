use crate::cmdbuilder::CommandBuilder;
use crate::win::overlapped_pipe::{OverlappedHandle, OverlappedPipe, PipeAccess};
use crate::win::psuedocon::PsuedoCon;
use crate::{Child, MasterPty, PtyPair, PtySize, PtySystem, SlavePty};
use anyhow::Error;
use std::os::windows::io::AsRawHandle;
use std::sync::{Arc, Mutex};
use winapi::um::wincon::COORD;

#[derive(Default)]
pub struct ConPtySystem {}

impl PtySystem for ConPtySystem {
    fn openpty(&self, size: PtySize) -> anyhow::Result<PtyPair> {
        // Overlapped duplex named pipe with the client handle shared
        // between `hInput` and `hOutput` of CreatePseudoConsole — matches
        // Microsoft Terminal's ConptyConnection.cpp:406-407.
        //
        // VENDORED PATCH (oriterm): 8 MiB pipe buffer (was 128 KiB upstream)
        // — see `crates/portable-pty/README.md` for full rationale.
        //
        // 8 MiB pipe buffer (was 128 KiB) so notcurses-class graphics-flood
        // producers can blast a full ~3 MB initial-frame transmit (and
        // ~256 KB compressed delta-frames) into the pipe without blocking
        // before conhost reads. Smaller buffers cause the producer's
        // first `write()` to block on conhost's read-rate ceiling, which
        // notcurses' deadline-based frame dropper then registers as
        // "dropped" frames for the entire deadline-overrun window
        // (the prior 13-second initial-blast drain was the load-bearing
        // symptom). conhost's own internal buffer is still the next-tier
        // limit; the pipe-buffer bump removes the first-tier bottleneck.
        //
        // Destructure so we own `server` and `client` independently
        // (cannot drop the client field while the struct is still
        // partially borrowed).
        let OverlappedPipe { server, client } =
            OverlappedPipe::new(PipeAccess::Duplex, 8 * 1024 * 1024)?;

        // Pass the SAME client handle to both hInput and hOutput slots
        // (ConptyConnection.cpp:406-407 calls
        // ConptyCreatePseudoConsole with `pipe.client.get(), pipe.client.get()`).
        let client_handle = client.as_raw_handle();
        let con = PsuedoCon::new(
            COORD {
                X: size.cols as i16,
                Y: size.rows as i16,
            },
            client_handle,
            client_handle,
        )?;
        // ConPTY duplicates internally; release the parent end so the
        // read side sees EOF when the child exits.
        drop(client);

        // The single server handle is duplex. Clone it for the reader;
        // the original is held for the (lazily-taken) writer.
        let readable = server.try_clone()?;

        let master = ConPtyMasterPty {
            inner: Arc::new(Mutex::new(Inner {
                con,
                readable,
                writable: Some(server),
                size,
            })),
        };

        let slave = ConPtySlavePty {
            inner: master.inner.clone(),
        };

        Ok(PtyPair {
            master: Box::new(master),
            slave: Box::new(slave),
        })
    }
}

struct Inner {
    con: PsuedoCon,
    readable: OverlappedHandle,
    writable: Option<OverlappedHandle>,
    size: PtySize,
}

impl Inner {
    pub fn resize(
        &mut self,
        num_rows: u16,
        num_cols: u16,
        pixel_width: u16,
        pixel_height: u16,
    ) -> Result<(), Error> {
        self.con.resize(COORD {
            X: num_cols as i16,
            Y: num_rows as i16,
        })?;
        self.size = PtySize {
            rows: num_rows,
            cols: num_cols,
            pixel_width,
            pixel_height,
        };
        Ok(())
    }
}

#[derive(Clone)]
pub struct ConPtyMasterPty {
    inner: Arc<Mutex<Inner>>,
}

pub struct ConPtySlavePty {
    inner: Arc<Mutex<Inner>>,
}

impl MasterPty for ConPtyMasterPty {
    fn resize(&self, size: PtySize) -> anyhow::Result<()> {
        let mut inner = self.inner.lock().unwrap();
        inner.resize(size.rows, size.cols, size.pixel_width, size.pixel_height)
    }

    fn get_size(&self) -> Result<PtySize, Error> {
        let inner = self.inner.lock().unwrap();
        Ok(inner.size.clone())
    }

    fn try_clone_reader(&self) -> anyhow::Result<Box<dyn std::io::Read + Send>> {
        Ok(Box::new(
            self.inner.lock().unwrap().readable.try_clone()?,
        ))
    }

    fn take_writer(&self) -> anyhow::Result<Box<dyn std::io::Write + Send>> {
        Ok(Box::new(
            self.inner
                .lock()
                .unwrap()
                .writable
                .take()
                .ok_or_else(|| anyhow::anyhow!("writer already taken"))?,
        ))
    }
}

impl SlavePty for ConPtySlavePty {
    fn spawn_command(&self, cmd: CommandBuilder) -> anyhow::Result<Box<dyn Child + Send + Sync>> {
        let inner = self.inner.lock().unwrap();
        let child = inner.con.spawn_command(cmd)?;
        Ok(Box::new(child))
    }
}
