//! Runs real CLI plan and confirmed write entries against a declaring synthetic source.

use super::*;

const CATEGORY: &str = "customer-adjustable-parameters";
const MARKER: &str = r#"<Parameters><Parameter Id="M-FFF6_A-0001-01-0000_P-991" CustomerAdjustable="privateSentinel" /></Parameters>"#;

fn assert_refused(output: &Output) {
    let shown = text(output);
    assert!(!output.status.success(), "{shown}");
    assert!(shown.contains(CATEGORY), "{shown}");
    assert!(shown.contains("written to the device: no"), "{shown}");
    for private in ["privateSentinel", "P-991", "Static("] {
        assert!(
            !shown.contains(private),
            "new CLI refusal leaked source payload"
        );
    }
}

#[test]
fn plans_for_every_scope_refuse_and_readiness_and_coverage_keep_the_category() {
    for marker in [
        MARKER,
        r#"<Options CustomerAdjustableParameters="false" />"#,
    ] {
        let dir = tempfile::tempdir().unwrap();
        let (project, products) = inputs_with_static(dir.path(), marker);
        for partial in [
            None,
            Some("parameters"),
            Some("group-addresses"),
            Some("both"),
        ] {
            let mut args = vec!["device", "download", TARGET];
            if let Some(partial) = partial {
                args.extend(["--partial", partial]);
            }
            assert_refused(&knx(&args, &project, &products));
        }
        let readiness = knx(&["device", "readiness"], &project, &products);
        assert!(readiness.status.success());
        assert!(text(&readiness).contains(CATEGORY));
        assert!(!text(&readiness).contains("privateSentinel"));
        let coverage = Command::new(env!("CARGO_BIN_EXE_knx"))
            .args(["products", "coverage", "--product-db"])
            .arg(&products)
            .stdin(Stdio::null())
            .output()
            .unwrap();
        assert!(coverage.status.success());
        assert!(text(&coverage).contains("unsupported customer-adjustable-parameters: 1"));
    }
}

#[test]
fn a_confirmed_write_after_markerless_plan_then_source_change_opens_no_tunnel_or_frame() {
    for partial in [
        None,
        Some("parameters"),
        Some("group-addresses"),
        Some("both"),
    ] {
        let dir = tempfile::tempdir().unwrap();
        let (project, products_path) = inputs(dir.path(), false);
        let mut args = vec!["device", "download", TARGET];
        if let Some(partial) = partial {
            args.extend(["--partial", partial]);
        }
        let plain = knx(&args, &project, &products_path);
        assert!(plain.status.success(), "{}", text(&plain));
        assert!(text(&plain).contains("support: UNTESTED"));
        let products = knx_productdb::open_and_migrate(&products_path).unwrap();
        let bytes = program_xml(MARKER).into_bytes();
        let sha = knx_productdb::blob::sha256_hex(&bytes);
        knx_productdb::store_source_file(
            &products,
            &knx_productdb::SourceFile {
                source_path: "M-FFF6/current.xml".into(),
                manufacturer_id: Some("M-FFF6".into()),
                bytes,
            },
        )
        .unwrap();
        assert_eq!(
            products
                .execute(
                    "UPDATE application_program SET source_sha256=?1 WHERE id=?2",
                    [&sha, PROGRAM_ID]
                )
                .unwrap(),
            1
        );
        drop(products);
        let peer = UdpSocket::bind("127.0.0.1:0").unwrap();
        peer.set_nonblocking(true).unwrap();
        let gateway = peer.local_addr().unwrap().to_string();
        let output = Command::new(env!("CARGO_BIN_EXE_knx"))
            .args(&args)
            .arg("--project")
            .arg(&project)
            .arg("--product-db")
            .arg(&products_path)
            .arg("--activity-history")
            .arg(dir.path().join("history.sqlite"))
            .arg("--backup-dir")
            .arg(dir.path().join("backups"))
            .args([
                "--gateway",
                &gateway,
                "--confirm",
                &format!("I confirm download to {TARGET}"),
                "--accept-untested",
                &format!("I accept an untested download to {TARGET}"),
            ])
            .stdin(Stdio::null())
            .output()
            .unwrap();
        assert_refused(&output);
        // The CLI has already exited and joined; no sender can race this nonblocking observation.
        assert_eq!(
            peer.recv_from(&mut [0; 2048]).unwrap_err().kind(),
            std::io::ErrorKind::WouldBlock
        );
        assert!(!dir.path().join("backups").exists());
    }
}
