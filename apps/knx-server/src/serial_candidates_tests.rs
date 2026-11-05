//! Exercise the real admission algorithm with injected entries, metadata and synthetic symlinks.

use super::*;
use std::cell::Cell;
use std::collections::HashMap;

#[derive(Clone)]
struct Node {
    symlink: bool,
    target: PathBuf,
    character: bool,
    error: Option<io::ErrorKind>,
}
#[derive(Default)]
struct Synthetic {
    names: Vec<OsString>,
    nodes: HashMap<PathBuf, Node>,
    directory_error: Option<io::ErrorKind>,
    entry_error: Option<usize>,
    visited: std::rc::Rc<Cell<usize>>,
    metadata_calls: Cell<usize>,
}
impl Synthetic {
    fn add(&mut self, name: &str, target: &str) {
        self.names.push(name.into());
        self.nodes.insert(
            Path::new(ALIASES).join(name),
            Node {
                symlink: true,
                target: target.into(),
                character: true,
                error: None,
            },
        );
    }
    fn scan(&self) -> SerialCandidates {
        scan(self, Path::new(ALIASES), Path::new(DEVICES))
    }
}
impl Filesystem for Synthetic {
    fn entries(&self, _: &Path) -> io::Result<Entries> {
        if let Some(error) = self.directory_error {
            return Err(error.into());
        }
        let names = self.names.clone();
        let entry_error = self.entry_error;
        let visited = self.visited.clone();
        Ok(Box::new(names.into_iter().enumerate().map(
            move |(i, name)| {
                visited.set(visited.get() + 1);
                if entry_error == Some(i) {
                    Err(io::ErrorKind::Other.into())
                } else {
                    Ok(name)
                }
            },
        )))
    }
    fn is_symlink(&self, path: &Path) -> io::Result<bool> {
        self.metadata_calls.set(self.metadata_calls.get() + 1);
        let node = &self.nodes[path];
        if let Some(error) = node.error {
            return Err(error.into());
        }
        Ok(node.symlink)
    }
    fn canonicalize(&self, path: &Path) -> io::Result<PathBuf> {
        Ok(self.nodes[path].target.clone())
    }
    fn is_character_device(&self, path: &Path) -> io::Result<bool> {
        Ok(self
            .nodes
            .values()
            .filter(|node| node.target == path)
            .all(|node| node.character))
    }
}
fn available(paths: &[(&str, &str)], omitted_entries: usize, truncated: bool) -> SerialCandidates {
    SerialCandidates::Available {
        candidates: paths
            .iter()
            .map(|(name, target)| SerialCandidate {
                path: format!("{ALIASES}/{name}"),
                target: (*target).into(),
            })
            .collect(),
        omitted_entries,
        truncated,
    }
}

#[test]
fn serial_candidates_sorts_and_deduplicates_by_target_independent_of_entry_order() {
    let mut fs = Synthetic::default();
    fs.add("usb-z", "/dev/ttyUSB1");
    fs.add("usb-b", "/dev/ttyACM0");
    fs.add("usb-a", "/dev/ttyUSB1");
    let expected = available(
        &[("usb-a", "/dev/ttyUSB1"), ("usb-b", "/dev/ttyACM0")],
        1,
        false,
    );
    assert_eq!(fs.scan(), expected);
    fs.names.reverse();
    assert_eq!(fs.scan(), expected);
}
#[test]
fn serial_candidates_filters_unsafe_names_before_metadata() {
    let mut fs = Synthetic {
        names: [
            "pci-a",
            "usb-",
            " usb-a",
            "usb-a ",
            "usb-a\n",
            "usb-a\u{85}",
            "usb-a/b",
            "usb-a\\b",
        ]
        .map(OsString::from)
        .into(),
        ..Default::default()
    };
    fs.names.push(format!("usb-{}", "a".repeat(252)).into());
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStringExt;
        fs.names.push(OsString::from_vec(b"usb-\xff".to_vec()));
    }
    assert_eq!(fs.scan(), available(&[], fs.names.len(), false));
    assert_eq!(fs.metadata_calls.get(), 0);
}
#[test]
fn serial_candidates_filters_dangling_regular_nonchar_and_outside_targets() {
    let mut fs = Synthetic::default();
    for (name, target) in [
        ("usb-regular", "/dev/ttyUSB0"),
        ("usb-dangling", "/dev/ttyUSB1"),
        ("usb-nonchar", "/dev/ttyACM0"),
        ("usb-outside", "/elsewhere/ttyUSB0"),
        ("usb-nested", "/dev/nested/ttyACM1"),
        ("usb-suffix", "/dev/ttyUSB1evil"),
        ("usb-no-digits", "/dev/ttyACM"),
        ("usb-unicode-digit", "/dev/ttyACM１"),
        ("usb-other-tty", "/dev/ttyS0"),
        ("usb-ok", "/dev/ttyUSB20"),
    ] {
        fs.add(name, target);
    }
    fs.nodes
        .get_mut(&Path::new(ALIASES).join("usb-regular"))
        .unwrap()
        .symlink = false;
    fs.nodes
        .get_mut(&Path::new(ALIASES).join("usb-dangling"))
        .unwrap()
        .error = Some(io::ErrorKind::NotFound);
    fs.nodes
        .get_mut(&Path::new(ALIASES).join("usb-nonchar"))
        .unwrap()
        .character = false;
    assert_eq!(
        fs.scan(),
        available(&[("usb-ok", "/dev/ttyUSB20")], 9, false)
    );
}
#[test]
fn serial_candidates_scan_and_row_limits_are_disclosed_without_traversing_tail() {
    let mut fs = Synthetic::default();
    for i in 0..(MAX_SCANNED_ENTRIES + 20) {
        fs.add(&format!("usb-{i:04}"), &format!("/dev/ttyUSB{i}"));
    }
    let SerialCandidates::Available {
        candidates,
        omitted_entries,
        truncated,
    } = fs.scan()
    else {
        panic!("available")
    };
    assert!(truncated);
    assert_eq!(candidates.len(), 64);
    assert_eq!(omitted_entries, 512 - 64);
    assert_eq!(fs.visited.get(), 513);
    assert_eq!(fs.metadata_calls.get(), 512);
    assert_eq!(
        candidates.first().unwrap().path,
        format!("{ALIASES}/usb-0000")
    );
    assert_eq!(
        candidates.last().unwrap().path,
        format!("{ALIASES}/usb-0063")
    );
    let mut rows = Synthetic::default();
    for i in 0..65 {
        rows.add(&format!("usb-{i:03}"), &format!("/dev/ttyACM{i}"));
    }
    let SerialCandidates::Available {
        candidates,
        omitted_entries,
        truncated,
    } = rows.scan()
    else {
        panic!("available")
    };
    assert!(truncated);
    assert_eq!(candidates.len(), 64);
    assert_eq!(omitted_entries, 1);
    let mut exact = Synthetic::default();
    for i in 0..MAX_SCANNED_ENTRIES {
        exact.names.push(format!("pci-{i}").into());
    }
    assert_eq!(exact.scan(), available(&[], 512, false));
}
#[test]
fn serial_candidates_empty_missing_permissions_and_io_are_distinct() {
    assert_eq!(Synthetic::default().scan(), available(&[], 0, false));
    for (error, reason) in [
        (io::ErrorKind::NotFound, UnavailableReason::MissingDirectory),
        (
            io::ErrorKind::PermissionDenied,
            UnavailableReason::PermissionDenied,
        ),
        (io::ErrorKind::Other, UnavailableReason::Io),
    ] {
        let fs = Synthetic {
            directory_error: Some(error),
            ..Default::default()
        };
        assert_eq!(fs.scan(), SerialCandidates::Unavailable { reason });
    }
    let mut fs = Synthetic::default();
    fs.add("usb-ok", "/dev/ttyUSB0");
    fs.add("usb-error", "/dev/ttyUSB1");
    fs.entry_error = Some(1);
    assert_eq!(
        fs.scan(),
        SerialCandidates::Unavailable {
            reason: UnavailableReason::Io
        }
    );
    fs.entry_error = None;
    for (error, reason) in [
        (
            io::ErrorKind::PermissionDenied,
            UnavailableReason::PermissionDenied,
        ),
        (io::ErrorKind::Other, UnavailableReason::Io),
    ] {
        fs.nodes
            .get_mut(&Path::new(ALIASES).join("usb-error"))
            .unwrap()
            .error = Some(error);
        assert_eq!(fs.scan(), SerialCandidates::Unavailable { reason });
    }
}

#[cfg(target_os = "linux")]
#[test]
fn serial_candidates_real_synthetic_filesystem_never_opens_targets() {
    use std::os::unix::fs::symlink;
    let root = tempfile::tempdir().unwrap();
    let devices = root.path().join("dev");
    let aliases = devices.join("serial/by-id");
    std::fs::create_dir_all(&aliases).unwrap();
    // The scratch root itself may be a symlink; compare canonical target parents like production.
    let devices = std::fs::canonicalize(&devices).unwrap();
    // Only create regular synthetic files. Inject character-device metadata, never mknod a device.
    std::fs::write(devices.join("ttyUSB1"), b"must not be read").unwrap();
    symlink("../../ttyUSB1", aliases.join("usb-good")).unwrap();
    symlink("../../ttyACM2", aliases.join("usb-dangling")).unwrap();
    std::fs::write(aliases.join("usb-not-a-link"), b"inert").unwrap();
    assert_eq!(
        scan(&LocalFilesystem, &aliases, &devices),
        available(&[], 3, false)
    );
    struct CharacterMetadata;
    impl Filesystem for CharacterMetadata {
        fn entries(&self, p: &Path) -> io::Result<Entries> {
            LocalFilesystem.entries(p)
        }
        fn is_symlink(&self, p: &Path) -> io::Result<bool> {
            LocalFilesystem.is_symlink(p)
        }
        fn canonicalize(&self, p: &Path) -> io::Result<PathBuf> {
            LocalFilesystem.canonicalize(p)
        }
        fn is_character_device(&self, _: &Path) -> io::Result<bool> {
            Ok(true)
        }
    }
    assert_eq!(
        scan(&CharacterMetadata, &aliases, &devices),
        available(&[("usb-good", "/dev/ttyUSB1")], 2, false)
    );
    std::fs::remove_dir_all(&aliases).unwrap();
    assert_eq!(
        scan(&LocalFilesystem, &aliases, &devices),
        SerialCandidates::Unavailable {
            reason: UnavailableReason::MissingDirectory
        }
    );
}

#[tokio::test]
async fn serial_candidates_http_exact_contract_off_reactor_and_no_connector_or_session_effects() {
    use crate::{AppState, BusSessionError, BusTunnel, GatewayConnector};
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use std::{
        future::Future,
        net::SocketAddrV4,
        pin::Pin,
        sync::{
            atomic::{AtomicUsize, Ordering},
            Arc,
        },
    };
    use tower::ServiceExt;
    struct DenyConnector;
    impl GatewayConnector for DenyConnector {
        fn connect_tunnel(
            &self,
            _: SocketAddrV4,
        ) -> Pin<Box<dyn Future<Output = Result<Box<dyn BusTunnel>, BusSessionError>> + Send + '_>>
        {
            panic!("listing must not connect IP")
        }
        fn connect_serial<'a>(
            &'a self,
            _: &'a crate::bus_serial::SerialTarget,
        ) -> Pin<Box<dyn Future<Output = Result<Box<dyn BusTunnel>, BusSessionError>> + Send + 'a>>
        {
            panic!("listing must not open serial")
        }
        fn discover(
            &self,
        ) -> Pin<
            Box<
                dyn Future<Output = Result<Vec<knx_net::DiscoveredGateway>, BusSessionError>>
                    + Send
                    + '_,
            >,
        > {
            panic!("listing must not discover")
        }
    }
    struct Scanner {
        response: SerialCandidates,
        calls: Arc<AtomicUsize>,
        reactor: std::thread::ThreadId,
    }
    impl SerialCandidateScanner for Scanner {
        fn scan(&self) -> SerialCandidates {
            assert_ne!(
                std::thread::current().id(),
                self.reactor,
                "blocking filesystem work on reactor"
            );
            self.calls.fetch_add(1, Ordering::SeqCst);
            self.response.clone()
        }
    }
    let rows = [
        (
            available(&[("usb-test", "/dev/ttyACM1")], 2, true),
            serde_json::json!({"status":"available","candidates":[{"path":"/dev/serial/by-id/usb-test","target":"/dev/ttyACM1"}],"truncated":true,"omittedEntries":2}),
        ),
        (
            available(&[], 0, false),
            serde_json::json!({"status":"available","candidates":[],"truncated":false,"omittedEntries":0}),
        ),
        (
            SerialCandidates::Unavailable {
                reason: UnavailableReason::MissingDirectory,
            },
            serde_json::json!({"status":"unavailable","reason":"missingDirectory"}),
        ),
        (
            SerialCandidates::Unavailable {
                reason: UnavailableReason::PermissionDenied,
            },
            serde_json::json!({"status":"unavailable","reason":"permissionDenied"}),
        ),
        (
            SerialCandidates::Unavailable {
                reason: UnavailableReason::Io,
            },
            serde_json::json!({"status":"unavailable","reason":"io"}),
        ),
        (
            SerialCandidates::Unsupported,
            serde_json::json!({"status":"unsupported"}),
        ),
    ];
    for (response, expected) in rows {
        let calls = Arc::new(AtomicUsize::new(0));
        let state = Arc::new(AppState {
            connector: Box::new(DenyConnector),
            serial_candidates: Arc::new(Scanner {
                response,
                calls: calls.clone(),
                reactor: std::thread::current().id(),
            }),
            ..Default::default()
        });
        let app = crate::app(state.clone(), None);
        assert_eq!(
            calls.load(Ordering::SeqCst),
            0,
            "router construction must not enumerate"
        );
        let session = state.bus_session.lock().await;
        let download = state.device_download.lock().await;
        let programming = state.address_programming.lock().await;
        let next_id = state.next_bus_session_id.load(Ordering::SeqCst);
        let request = Request::builder()
            .uri("/api/bus/serial-candidates")
            .body(Body::empty())
            .unwrap();
        let response = tokio::time::timeout(
            std::time::Duration::from_secs(2),
            app.clone().oneshot(request),
        )
        .await
        .unwrap()
        .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let bytes = axum::body::to_bytes(response.into_body(), 65536)
            .await
            .unwrap();
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&bytes).unwrap(),
            expected
        );
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        assert!(session.is_none());
        assert!(download.is_none());
        assert!(programming.is_none());
        assert_eq!(state.next_bus_session_id.load(Ordering::SeqCst), next_id);
        assert!(state.project.lock().unwrap().is_none());
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/bus/serial-candidates")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED);
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }
}
