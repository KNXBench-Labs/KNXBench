//! Secret-class columns are blanked in the stored payload and reported by count, never by value.

use knx_productdb::legacy::{is_secret_column, parse_exim, withhold_secret_values, SecretColumn};

const WITH_SECRETS: &str = "EX-IM\r\n\
N C:\\x\\ets.vd_\r\n\
K ETS3\r\n\
V 6.2\r\n\
H virtual_device\r\n\
-------------------------------------\r\n\
T 23 device\r\n\
C1 T23 1 4 N DEVICE_ID\r\n\
C2 T23 3 50 Y Device_Bcu_Password\r\n\
C3 T23 3 50 Y DEVICE_NAME\r\n\
R 1 T 23 device\r\n\
1\r\n\
Zaphod42\r\n\
\\\\Beeblebrox\r\n\
Kept\r\n\
R 2 T 23 device\r\n\
2\r\n\
\r\n\
Also kept\r\n\
R 3 T 23 device\r\n\
3\r\n\
Trillian\r\n\
\\\\\r\n\
\r\n\
XXX\r\n";

#[test]
fn only_columns_named_password_are_secret() {
    assert!(is_secret_column("DEVICE_BCU_PASSWORD"));
    assert!(is_secret_column("project_password"));
    assert!(is_secret_column("PassWord"));
    assert!(!is_secret_column("DEVICE_NAME"));
    assert!(!is_secret_column("PASS_WORD"));
}

#[test]
fn non_empty_secret_values_are_blanked_and_every_other_byte_is_kept() {
    let input = WITH_SECRETS.as_bytes();
    let withheld = withhold_secret_values(input).unwrap();
    assert_eq!(
        withheld.columns,
        [SecretColumn {
            table: "device".into(),
            column: "Device_Bcu_Password".into(),
            rows: 2,
        }]
    );
    let expected = WITH_SECRETS
        .replace("Zaphod42\r\n\\\\Beeblebrox\r\n", "\r\n")
        .replace("Trillian\r\n\\\\\r\n", "\r\n");
    assert_eq!(String::from_utf8(withheld.bytes.clone()).unwrap(), expected);
    let text = String::from_utf8_lossy(&withheld.bytes);
    assert!(!text.contains("Zaphod") && !text.contains("Beeblebrox") && !text.contains("Trillian"));

    // The stored copy still parses, with every other value unchanged.
    let doc = parse_exim(&withheld.bytes).unwrap();
    let table = doc.table("device").unwrap();
    assert_eq!(table.row_count(), 3);
    let names: Vec<_> = (0..3).map(|r| table.text(r, 2).into_owned()).collect();
    assert_eq!(names, ["Kept", "Also kept", ""]);
    assert!((0..3).all(|r| table.raw(r, 1).is_empty()));
}

#[test]
fn a_payload_without_secret_values_is_returned_unchanged() {
    let input = WITH_SECRETS
        .replace("Zaphod42\r\n\\\\Beeblebrox\r\n", "\r\n")
        .replace("Trillian\r\n\\\\\r\n", "\r\n");
    let withheld = withhold_secret_values(input.as_bytes()).unwrap();
    assert!(withheld.columns.is_empty());
    assert_eq!(withheld.bytes, input.as_bytes());
}

#[test]
fn a_malformed_payload_is_refused_not_half_blanked() {
    let truncated = WITH_SECRETS.replace("XXX\r\n", "");
    assert!(withhold_secret_values(truncated.as_bytes()).is_err());
}

#[test]
fn two_secret_columns_are_withheld_row_by_row_in_source_order() {
    let input = "EX-IM\r\nV 6.3\r\nH project\r\n\
-------------------------------------\r\n\
T 4 project\r\n\
C1 T4 1 4 N PROJECT_ID\r\n\
C2 T4 3 20 Y PROJECT_PASSWORD\r\n\
C3 T4 3 50 Y PROJECT_NAME\r\n\
C4 T4 3 20 Y PROJECT_BCU_PASSWORD\r\n\
R 1 T 4 project\r\n1\r\nfirst\r\nOne\r\nsecond\r\n\
R 2 T 4 project\r\n2\r\nthird\r\nTwo\r\nfourth\r\n\
XXX\r\n";
    let withheld = withhold_secret_values(input.as_bytes()).unwrap();
    let expected = input
        .replace("first", "")
        .replace("second", "")
        .replace("third", "")
        .replace("fourth", "");
    assert_eq!(String::from_utf8(withheld.bytes).unwrap(), expected);
    let counts: Vec<_> = withheld
        .columns
        .iter()
        .map(|c| (c.column.as_str(), c.rows))
        .collect();
    assert_eq!(
        counts,
        [("PROJECT_PASSWORD", 2), ("PROJECT_BCU_PASSWORD", 2)]
    );
}
