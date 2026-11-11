/** Captures the fictional product fixture from the HTTP metadata regression. */
import type { DeviceProductNode } from "../../src/bindings/DeviceProductNode";
export const product: DeviceProductNode = {
  "catalog": {
    "application_name": "Fictional application",
    "application_number": "17",
    "application_program_id": "A-rich",
    "application_source_language": "en-US",
    "application_version": "003",
    "catalog_item_name": "First placement",
    "catalog_item_number": "CAT-1",
    "catalog_item_source_language": "en-US",
    "details": [
      {
        "fields": [
          {
            "key": "id",
            "value": "M-0083"
          },
          {
            "key": "name",
            "value": "M-0083"
          }
        ],
        "id": "M-0083",
        "kind": "manufacturer"
      },
      {
        "fields": [
          {
            "key": "id",
            "value": "P-rich"
          },
          {
            "key": "manufacturer_id",
            "value": "M-0083"
          },
          {
            "key": "hardware_id",
            "value": "H-rich"
          },
          {
            "key": "text",
            "value": "Fictional actuator"
          },
          {
            "key": "order_number",
            "value": "ORDER-004"
          },
          {
            "key": "is_rail_mounted",
            "value": "1"
          },
          {
            "key": "width_in_millimeter",
            "value": "72.5"
          },
          {
            "key": "default_language",
            "value": "en-US"
          },
          {
            "key": "hash",
            "value": "product-hash"
          },
          {
            "key": "registration_status",
            "value": "Registered"
          },
          {
            "key": "source_sha256",
            "value": "fixture-source"
          }
        ],
        "id": "P-rich",
        "kind": "product"
      },
      {
        "fields": [
          {
            "key": "id",
            "value": "H-rich"
          },
          {
            "key": "manufacturer_id",
            "value": "M-0083"
          },
          {
            "key": "name",
            "value": "Fictional hardware"
          },
          {
            "key": "serial_number",
            "value": "SER-007"
          },
          {
            "key": "version_number",
            "value": "02"
          },
          {
            "key": "bus_current",
            "value": "12.5"
          },
          {
            "key": "has_individual_address",
            "value": "1"
          },
          {
            "key": "has_application_program",
            "value": "1"
          },
          {
            "key": "is_accessory",
            "value": "0"
          },
          {
            "key": "is_coupler",
            "value": "0"
          },
          {
            "key": "is_power_supply",
            "value": "0"
          },
          {
            "key": "is_ip_enabled",
            "value": "1"
          },
          {
            "key": "is_power_line_repeater",
            "value": "0"
          },
          {
            "key": "original_manufacturer",
            "value": "M-0099"
          },
          {
            "key": "source_sha256",
            "value": "fixture-source"
          }
        ],
        "id": "H-rich",
        "kind": "hardware"
      },
      {
        "fields": [
          {
            "key": "id",
            "value": "HP-rich"
          },
          {
            "key": "manufacturer_id",
            "value": "M-0083"
          },
          {
            "key": "hardware_id",
            "value": "H-rich"
          },
          {
            "key": "application_program_ref",
            "value": "A-rich"
          },
          {
            "key": "medium_types",
            "value": "MT-0 MT-5"
          },
          {
            "key": "hash",
            "value": "link-hash"
          },
          {
            "key": "registration_number",
            "value": "REG-1"
          },
          {
            "key": "registration_status",
            "value": "Registered"
          },
          {
            "key": "registration_signature",
            "value": "signature-literal"
          },
          {
            "key": "source_sha256",
            "value": "fixture-source"
          }
        ],
        "id": "HP-rich",
        "kind": "programLink"
      },
      {
        "fields": [
          {
            "key": "id",
            "value": "A-rich"
          },
          {
            "key": "manufacturer_id",
            "value": "M-0083"
          },
          {
            "key": "name",
            "value": "Fictional application"
          },
          {
            "key": "application_number",
            "value": "17"
          },
          {
            "key": "application_version",
            "value": "003"
          },
          {
            "key": "program_type",
            "value": "ApplicationProgram"
          },
          {
            "key": "mask_version",
            "value": "MV-0701"
          },
          {
            "key": "pei_type",
            "value": "0"
          },
          {
            "key": "load_procedure_style",
            "value": "ProductSpecific"
          },
          {
            "key": "default_language",
            "value": "en-US"
          },
          {
            "key": "hash",
            "value": "program-hash"
          },
          {
            "key": "linkable",
            "value": "0"
          },
          {
            "key": "original_manufacturer",
            "value": "M-0099"
          },
          {
            "key": "source_sha256",
            "value": "fixture-source"
          },
          {
            "key": "is_secure_enabled",
            "value": "false"
          },
          {
            "key": "max_security_group_key_table_entries",
            "value": "16"
          },
          {
            "key": "max_security_individual_address_entries",
            "value": "32"
          },
          {
            "key": "max_security_p2p_key_table_entries",
            "value": "8"
          },
          {
            "key": "max_tunneling_user_entries",
            "value": "4"
          },
          {
            "key": "max_user_entries",
            "value": "5"
          },
          {
            "key": "min_ets_version",
            "value": "5.7"
          },
          {
            "key": "replaces_versions",
            "value": "1 2"
          }
        ],
        "id": "A-rich",
        "kind": "application"
      },
      {
        "fields": [
          {
            "key": "id",
            "value": "CI-a"
          },
          {
            "key": "manufacturer_id",
            "value": "M-0083"
          },
          {
            "key": "section_id",
            "value": "CS-child"
          },
          {
            "key": "name",
            "value": "First placement"
          },
          {
            "key": "number",
            "value": "CAT-1"
          },
          {
            "key": "visible_description",
            "value": "First description"
          },
          {
            "key": "product_ref_id",
            "value": "P-rich"
          },
          {
            "key": "hardware2program_ref_id",
            "value": "HP-rich"
          },
          {
            "key": "default_language",
            "value": "en-US"
          },
          {
            "key": "source_sha256",
            "value": "fixture-source"
          }
        ],
        "id": "CI-a",
        "kind": "catalogItem"
      },
      {
        "fields": [
          {
            "key": "id",
            "value": "CS-child"
          },
          {
            "key": "manufacturer_id",
            "value": "M-0083"
          },
          {
            "key": "parent_id",
            "value": "CS-root"
          },
          {
            "key": "name",
            "value": "Fictional section"
          },
          {
            "key": "number",
            "value": "02"
          },
          {
            "key": "visible_description",
            "value": "Section description"
          },
          {
            "key": "default_language",
            "value": "en-US"
          },
          {
            "key": "source_sha256",
            "value": "fixture-source"
          }
        ],
        "id": "CS-child",
        "kind": "catalogSection"
      },
      {
        "fields": [
          {
            "key": "id",
            "value": "CS-root"
          },
          {
            "key": "manufacturer_id",
            "value": "M-0083"
          },
          {
            "key": "name",
            "value": "Fictional catalogue"
          },
          {
            "key": "number",
            "value": "01"
          },
          {
            "key": "visible_description",
            "value": "Root description"
          },
          {
            "key": "default_language",
            "value": "en-US"
          },
          {
            "key": "source_sha256",
            "value": "fixture-source"
          }
        ],
        "id": "CS-root",
        "kind": "catalogSection"
      },
      {
        "fields": [
          {
            "key": "id",
            "value": "CI-b"
          },
          {
            "key": "manufacturer_id",
            "value": "M-0083"
          },
          {
            "key": "section_id",
            "value": "CS-root"
          },
          {
            "key": "name",
            "value": "Second placement"
          },
          {
            "key": "number",
            "value": "CAT-2"
          },
          {
            "key": "visible_description",
            "value": "Second description"
          },
          {
            "key": "product_ref_id",
            "value": "P-rich"
          },
          {
            "key": "hardware2program_ref_id",
            "value": "HP-rich"
          },
          {
            "key": "default_language",
            "value": "en-US"
          },
          {
            "key": "source_sha256",
            "value": "fixture-source"
          }
        ],
        "id": "CI-b",
        "kind": "catalogItem"
      },
      {
        "fields": [
          {
            "key": "source_sha256",
            "value": "fixture-source"
          },
          {
            "key": "source_bytes",
            "value": "6"
          },
          {
            "key": "source_name",
            "value": "fixture.xml"
          },
          {
            "key": "source_name",
            "value": "vendor.zip!/fixture.knxprod"
          }
        ],
        "id": "fixture-source",
        "kind": "source"
      },
      {
        "fields": [
          {
            "key": "xpath",
            "value": "/KNX/ManufacturerData/Manufacturer/Hardware/Hardware"
          },
          {
            "key": "kind",
            "value": "Attribute"
          },
          {
            "key": "name",
            "value": "FutureHardwareAttribute"
          },
          {
            "key": "occurrences",
            "value": "2"
          },
          {
            "key": "sample",
            "value": "Preserved future value"
          }
        ],
        "id": "fixture-source:1",
        "kind": "additionalAttributes"
      },
      {
        "fields": [
          {
            "key": "sha256",
            "value": "fixture-package"
          },
          {
            "key": "source_name",
            "value": "fixture.knxprod"
          },
          {
            "key": "scheme",
            "value": "11"
          },
          {
            "key": "size",
            "value": "3"
          }
        ],
        "id": "fixture-package",
        "kind": "package"
      }
    ],
    "hardware_name": "Fictional hardware",
    "hardware_serial_number": "SER-007",
    "hardware_version": "02",
    "manufacturer_id": "M-0083",
    "manufacturer_name": "Fictional Devices",
    "mask_version": "MV-0701",
    "order_number": "ORDER-004",
    "product_source_language": "en-US",
    "product_text": "Fictional actuator"
  },
  "product_ref": "P-rich",
  "program_ref": "HP-rich",
  "resolution": "Resolved"
};
