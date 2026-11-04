//! Test tty configuration rollback and exclusive ownership on freshly allocated PTYs only.

use super::*;
use std::ffi::CStr;
use std::os::fd::{FromRawFd, OwnedFd};

fn pty() -> (OwnedFd, OwnedFd, std::path::PathBuf) {
    let mut master = -1;
    let mut slave = -1;
    let mut name = [0i8; 128];
    assert_eq!(
        unsafe {
            libc::openpty(
                &mut master,
                &mut slave,
                name.as_mut_ptr(),
                std::ptr::null(),
                std::ptr::null(),
            )
        },
        0
    );
    let path = unsafe { CStr::from_ptr(name.as_ptr()) }
        .to_str()
        .unwrap()
        .into();
    (
        unsafe { OwnedFd::from_raw_fd(master) },
        unsafe { OwnedFd::from_raw_fd(slave) },
        path,
    )
}
fn settings(fd: RawFd) -> libc::termios {
    let mut value = unsafe { std::mem::zeroed() };
    assert_eq!(unsafe { libc::tcgetattr(fd, &mut value) }, 0);
    value
}
fn assert_same(a: &libc::termios, b: &libc::termios) {
    assert_eq!(
        (a.c_iflag, a.c_oflag, a.c_cflag, a.c_lflag, a.c_line, a.c_cc),
        (b.c_iflag, b.c_oflag, b.c_cflag, b.c_lflag, b.c_line, b.c_cc)
    );
    assert_eq!(unsafe { libc::cfgetispeed(a) }, unsafe {
        libc::cfgetispeed(b)
    });
    assert_eq!(unsafe { libc::cfgetospeed(a) }, unsafe {
        libc::cfgetospeed(b)
    });
}
#[test]
fn parity_markers_preserve_ff_and_reject_uart_errors() {
    let mut decoder = ParityDecoder::default();
    assert_eq!(decoder.byte(0).unwrap(), Some(0));
    assert_eq!(decoder.byte(0xff).unwrap(), None);
    assert_eq!(decoder.byte(0xff).unwrap(), Some(0xff));
    for bad in [0, 1, 0x8b] {
        let mut decoder = ParityDecoder::default();
        assert_eq!(decoder.byte(0xff).unwrap(), None);
        assert_eq!(decoder.byte(0).unwrap(), None);
        assert_eq!(
            decoder.byte(bad).unwrap_err().kind(),
            io::ErrorKind::InvalidData
        );
    }
    let mut decoder = ParityDecoder::default();
    decoder.byte(0xff).unwrap();
    assert!(decoder.byte(1).is_err());
}

fn exclusive(fd: RawFd) -> libc::c_int {
    let mut value = 0;
    assert_eq!(unsafe { libc::ioctl(fd, libc::TIOCGEXCL, &mut value) }, 0);
    value
}

#[tokio::test]
async fn pty_parity_is_never_weakened_and_settings_are_restored() {
    let (_master, slave, path) = pty();
    let before = settings(slave.as_raw_fd());
    for _ in 0..3 {
        match Tty::open(&path) {
            Ok(port) => {
                let configured = settings(slave.as_raw_fd());
                assert_eq!(
                    configured.c_cflag & (libc::CSIZE | libc::PARENB | libc::PARODD | libc::CSTOPB),
                    libc::CS8 | libc::PARENB
                );
                assert_eq!(exclusive(slave.as_raw_fd()), 1);
                drop(port);
            }
            // Linux PTYs commonly clear PARENB or reject it. Production must refuse
            // that, not silently downgrade to 8N1 to make an integration test pass.
            Err(error) => assert!(
                matches!(
                    error.kind(),
                    io::ErrorKind::Unsupported | io::ErrorKind::InvalidInput
                ),
                "{error}"
            ),
        }
        assert_same(&before, &settings(slave.as_raw_fd()));
        assert_eq!(exclusive(slave.as_raw_fd()), 0);
        // A second independent open/lock proves the failed path released its fd/lock.
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .custom_flags(libc::O_NOCTTY | libc::O_NONBLOCK)
            .open(&path)
            .unwrap();
        assert_eq!(
            unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) },
            0
        );
    }
}

#[tokio::test]
async fn existing_exclusive_owner_is_not_reconfigured_or_unlocked() {
    let (_master, slave, path) = pty();
    let before = settings(slave.as_raw_fd());
    assert_eq!(unsafe { libc::ioctl(slave.as_raw_fd(), libc::TIOCEXCL) }, 0);
    assert!(Tty::open(&path).is_err());
    assert_eq!(exclusive(slave.as_raw_fd()), 1);
    assert_same(&before, &settings(slave.as_raw_fd()));
    assert_eq!(unsafe { libc::ioctl(slave.as_raw_fd(), libc::TIOCNXCL) }, 0);
}

#[tokio::test]
async fn existing_advisory_owner_is_not_reconfigured() {
    let (_master, slave, path) = pty();
    let before = settings(slave.as_raw_fd());
    assert_eq!(
        unsafe { libc::flock(slave.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) },
        0
    );
    assert!(Tty::open(&path).is_err());
    assert_same(&before, &settings(slave.as_raw_fd()));
    assert_eq!(exclusive(slave.as_raw_fd()), 0);
}

#[tokio::test]
async fn async_fd_roundtrip_drop_restores_and_releases_owned_pty() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let (master, slave, path) = pty();
    let saved = settings(slave.as_raw_fd());
    let fd = unsafe { libc::dup(slave.as_raw_fd()) };
    assert!(fd >= 0);
    let file = unsafe { File::from_raw_fd(fd) };
    // Exercise AsyncFd/RAII separately: a PTY need not support even parity. The
    // protocol actor tests use the identical parser/session over injected duplex.
    assert_eq!(
        unsafe { libc::fcntl(fd, libc::F_SETFL, libc::O_NONBLOCK) },
        0
    );
    assert_eq!(unsafe { libc::ioctl(fd, libc::TIOCEXCL) }, 0);
    let mut raw = saved;
    unsafe {
        libc::cfmakeraw(&mut raw);
    }
    raw.c_iflag = libc::INPCK | libc::PARMRK;
    assert_eq!(unsafe { libc::tcsetattr(fd, libc::TCSANOW, &raw) }, 0);
    let mut tty = Tty {
        fd: unsafe {
            AsyncFd::register(Port {
                file,
                saved,
                exclusive: true,
            })
        }
        .unwrap_or_else(|_| panic!("PTY registration failed")),
        parity: ParityDecoder::default(),
    };
    let input = [0xffu8, 3, 0x8b, 0];
    assert_eq!(
        unsafe { libc::write(master.as_raw_fd(), input.as_ptr().cast(), input.len()) },
        input.len() as isize
    );
    // A one-byte read splits the driver's doubled FF marker across syscalls.
    assert_eq!(tty.read_u8().await.unwrap(), 0xff);
    let mut rest = [0; 3];
    tty.read_exact(&mut rest).await.unwrap();
    assert_eq!(rest, [3, 0x8b, 0]);
    tty.write_all(&[1, 0xff, 0x8b]).await.unwrap();
    let mut output = [0u8; 3];
    assert_eq!(
        unsafe { libc::read(master.as_raw_fd(), output.as_mut_ptr().cast(), output.len()) },
        3
    );
    assert_eq!(output, [1, 0xff, 0x8b]);
    drop(tty);
    // No stale integer-fd assertion: parallel tests may reuse a closed fd.
    let reopened = OpenOptions::new()
        .read(true)
        .write(true)
        .custom_flags(libc::O_NOCTTY | libc::O_NONBLOCK)
        .open(path)
        .unwrap();
    assert_eq!(
        unsafe { libc::flock(reopened.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) },
        0
    );
    assert_same(&saved, &settings(slave.as_raw_fd()));
    assert_eq!(exclusive(slave.as_raw_fd()), 0);
}
