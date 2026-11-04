//! Own a Linux raw 19200 8E1 tty, restoring settings and exclusivity on every close.

use std::fs::{File, OpenOptions};
use std::io;
use std::os::fd::{AsRawFd, RawFd};
use std::os::unix::fs::OpenOptionsExt;
use std::path::Path;
use std::pin::Pin;
use std::task::{ready, Context, Poll};
use tokio::io::unix::AsyncFd;
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};

struct Port {
    file: File,
    saved: libc::termios,
    exclusive: bool,
}

impl AsRawFd for Port {
    fn as_raw_fd(&self) -> RawFd {
        self.file.as_raw_fd()
    }
}

impl Drop for Port {
    fn drop(&mut self) {
        // TCSANOW never waits for a dead USB device to drain. Never touch modem lines.
        unsafe {
            libc::tcsetattr(self.as_raw_fd(), libc::TCSANOW, &self.saved);
            if self.exclusive {
                libc::ioctl(self.as_raw_fd(), libc::TIOCNXCL);
            }
        }
    }
}

pub(super) struct Tty {
    fd: AsyncFd<Port>,
    parity: ParityDecoder,
}

#[derive(Default)]
struct ParityDecoder {
    pending: u8,
}
impl ParityDecoder {
    fn byte(&mut self, byte: u8) -> io::Result<Option<u8>> {
        match (self.pending, byte) {
            (0, 0xff) => {
                self.pending = 1;
                Ok(None)
            }
            (0, byte) => Ok(Some(byte)),
            (1, 0xff) => {
                self.pending = 0;
                Ok(Some(0xff))
            }
            (1, 0) => {
                self.pending = 2;
                Ok(None)
            }
            _ => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "serial parity/framing error or break",
            )),
        }
    }
}

impl Tty {
    pub(super) fn open(path: &Path) -> io::Result<Self> {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .custom_flags(libc::O_NOCTTY | libc::O_NONBLOCK | libc::O_CLOEXEC)
            .open(path)?;
        let fd = file.as_raw_fd();
        let mut saved = unsafe { std::mem::zeroed::<libc::termios>() };
        if unsafe { libc::tcgetattr(fd, &mut saved) } != 0 {
            return Err(io::Error::last_os_error());
        }
        // flock also excludes privileged cooperating processes which bypass TIOCEXCL.
        if unsafe { libc::flock(fd, libc::LOCK_EX | libc::LOCK_NB) } != 0 {
            return Err(io::Error::last_os_error());
        }
        let mut exclusive: libc::c_int = 0;
        if unsafe { libc::ioctl(fd, libc::TIOCGEXCL, &mut exclusive) } != 0 {
            return Err(io::Error::last_os_error());
        }
        if exclusive != 0 {
            return Err(io::Error::new(
                io::ErrorKind::AlreadyExists,
                "tty is already exclusive",
            ));
        }
        let mut port = Port {
            file,
            saved,
            exclusive: false,
        };
        if unsafe { libc::ioctl(fd, libc::TIOCEXCL) } != 0 {
            return Err(io::Error::last_os_error());
        }
        port.exclusive = true;
        let mut settings = saved;
        unsafe {
            libc::cfmakeraw(&mut settings);
            libc::cfsetispeed(&mut settings, libc::B19200);
            libc::cfsetospeed(&mut settings, libc::B19200);
        }
        settings.c_cflag &= !(libc::CSIZE | libc::PARODD | libc::CSTOPB | libc::CRTSCTS);
        settings.c_cflag |= libc::CS8 | libc::PARENB | libc::CLOCAL | libc::CREAD;
        settings.c_iflag = libc::INPCK | libc::PARMRK;
        settings.c_oflag = 0;
        settings.c_lflag = 0;
        settings.c_cc[libc::VMIN] = 1;
        settings.c_cc[libc::VTIME] = 0;
        if unsafe { libc::tcsetattr(fd, libc::TCSANOW, &settings) } != 0 {
            return Err(io::Error::last_os_error());
        }
        let mut actual = settings;
        if unsafe { libc::tcgetattr(fd, &mut actual) } != 0 {
            return Err(io::Error::last_os_error());
        }
        let mask = libc::CSIZE
            | libc::PARENB
            | libc::PARODD
            | libc::CSTOPB
            | libc::CRTSCTS
            | libc::CLOCAL
            | libc::CREAD;
        if actual.c_cflag & mask != libc::CS8 | libc::PARENB | libc::CLOCAL | libc::CREAD
            || unsafe { libc::cfgetispeed(&actual) } != libc::B19200
            || unsafe { libc::cfgetospeed(&actual) } != libc::B19200
            || actual.c_iflag != libc::INPCK | libc::PARMRK
            || actual.c_oflag != 0
            || actual.c_lflag != 0
        {
            return Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "tty cannot provide raw 19200 8E1",
            ));
        }
        // Discard bytes preceding this session's reset, not any new protocol traffic.
        if unsafe { libc::tcflush(fd, libc::TCIFLUSH) } != 0 {
            return Err(io::Error::last_os_error());
        }
        // SAFETY: Port owns its File and never replaces/closes it before AsyncFd drops.
        Ok(Self {
            fd: unsafe { AsyncFd::register(port) }.map_err(io::Error::from)?,
            parity: ParityDecoder::default(),
        })
    }
}

impl AsyncRead for Tty {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        if buf.remaining() == 0 {
            return Poll::Ready(Ok(()));
        }
        let before = buf.filled().len();
        let this = self.get_mut();
        loop {
            let mut raw = [0u8; 256];
            let mut guard = ready!(this.fd.poll_read_ready(cx))?;
            let result = guard.try_io(|fd| {
                let n = unsafe {
                    libc::read(
                        fd.get_ref().as_raw_fd(),
                        raw.as_mut_ptr().cast(),
                        raw.len().min(buf.remaining()),
                    )
                };
                if n < 0 {
                    Err(io::Error::last_os_error())
                } else {
                    Ok(n as usize)
                }
            });
            match result {
                Ok(Ok(n)) => {
                    // PARMRK preserves UART errors; valid FF is doubled by the driver.
                    // Retain escape state across short reads rather than replacing an
                    // error with a zero byte that could masquerade as a bus ACK.
                    if n == 0 {
                        return if this.parity.pending == 0 {
                            Poll::Ready(Ok(()))
                        } else {
                            Poll::Ready(Err(io::Error::new(
                                io::ErrorKind::InvalidData,
                                "truncated serial parity marker",
                            )))
                        };
                    }
                    for byte in &raw[..n] {
                        if let Some(byte) = this.parity.byte(*byte)? {
                            buf.put_slice(&[byte]);
                        }
                    }
                    if buf.filled().len() > before {
                        return Poll::Ready(Ok(()));
                    }
                }
                Ok(Err(e)) if e.kind() == io::ErrorKind::Interrupted => continue,
                Ok(Err(e)) => return Poll::Ready(Err(e)),
                Err(_) => continue,
            }
        }
    }
}

impl AsyncWrite for Tty {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        loop {
            let mut guard = ready!(self.fd.poll_write_ready(cx))?;
            match guard.try_io(|fd| {
                let n = unsafe {
                    libc::write(fd.get_ref().as_raw_fd(), buf.as_ptr().cast(), buf.len())
                };
                if n < 0 {
                    Err(io::Error::last_os_error())
                } else {
                    Ok(n as usize)
                }
            }) {
                Ok(Err(e)) if e.kind() == io::ErrorKind::Interrupted => continue,
                Ok(result) => return Poll::Ready(result),
                Err(_) => continue,
            }
        }
    }
    fn poll_flush(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<io::Result<()>> {
        Poll::Ready(Ok(()))
    }
    fn poll_shutdown(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<io::Result<()>> {
        Poll::Ready(Ok(()))
    }
}

#[cfg(test)]
mod tests;
