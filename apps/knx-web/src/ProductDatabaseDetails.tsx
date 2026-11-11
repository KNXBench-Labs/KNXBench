/** Displays complete read-only product metadata without interpreting source literals. */
import type { ProductDetailSection } from "./bindings/ProductDetailSection";
import { useTranslate, type MessageKey } from "./i18n";

const FIELD_LABELS: Record<string, MessageKey> = {
  sha256: "deviceIdentity.database.field.sha256",
  scheme: "deviceIdentity.database.field.scheme",
  size: "deviceIdentity.database.field.size",
  "id": "deviceIdentity.database.field.id",
  "manufacturer_id": "deviceIdentity.database.field.manufacturer_id",
  "hardware_id": "deviceIdentity.database.field.hardware_id",
  "text": "deviceIdentity.database.field.text",
  "order_number": "deviceIdentity.database.field.order_number",
  "is_rail_mounted": "deviceIdentity.database.field.is_rail_mounted",
  "width_in_millimeter": "deviceIdentity.database.field.width_in_millimeter",
  "default_language": "deviceIdentity.database.field.default_language",
  "hash": "deviceIdentity.database.field.hash",
  "registration_status": "deviceIdentity.database.field.registration_status",
  "source_sha256": "deviceIdentity.database.field.source_sha256",
  "name": "deviceIdentity.database.field.name",
  "serial_number": "deviceIdentity.database.field.serial_number",
  "version_number": "deviceIdentity.database.field.version_number",
  "bus_current": "deviceIdentity.database.field.bus_current",
  "has_individual_address": "deviceIdentity.database.field.has_individual_address",
  "has_application_program": "deviceIdentity.database.field.has_application_program",
  "is_accessory": "deviceIdentity.database.field.is_accessory",
  "is_coupler": "deviceIdentity.database.field.is_coupler",
  "is_power_supply": "deviceIdentity.database.field.is_power_supply",
  "is_ip_enabled": "deviceIdentity.database.field.is_ip_enabled",
  "is_power_line_repeater": "deviceIdentity.database.field.is_power_line_repeater",
  "original_manufacturer": "deviceIdentity.database.field.original_manufacturer",
  "application_program_ref": "deviceIdentity.database.field.application_program_ref",
  "medium_types": "deviceIdentity.database.field.medium_types",
  "registration_number": "deviceIdentity.database.field.registration_number",
  "registration_signature": "deviceIdentity.database.field.registration_signature",
  "section_id": "deviceIdentity.database.field.section_id",
  "parent_id": "deviceIdentity.database.field.parent_id",
  "number": "deviceIdentity.database.field.number",
  "visible_description": "deviceIdentity.database.field.visible_description",
  "product_ref_id": "deviceIdentity.database.field.product_ref_id",
  "hardware2program_ref_id": "deviceIdentity.database.field.hardware2program_ref_id",
  "application_number": "deviceIdentity.database.field.application_number",
  "application_version": "deviceIdentity.database.field.application_version",
  "program_type": "deviceIdentity.database.field.program_type",
  "mask_version": "deviceIdentity.database.field.mask_version",
  "pei_type": "deviceIdentity.database.field.pei_type",
  "load_procedure_style": "deviceIdentity.database.field.load_procedure_style",
  "linkable": "deviceIdentity.database.field.linkable",
  "is_secure_enabled": "deviceIdentity.database.field.is_secure_enabled",
  "max_security_group_key_table_entries": "deviceIdentity.database.field.max_security_group_key_table_entries",
  "max_security_individual_address_entries": "deviceIdentity.database.field.max_security_individual_address_entries",
  "max_security_p2p_key_table_entries": "deviceIdentity.database.field.max_security_p2p_key_table_entries",
  "max_tunneling_user_entries": "deviceIdentity.database.field.max_tunneling_user_entries",
  "max_user_entries": "deviceIdentity.database.field.max_user_entries",
  "min_ets_version": "deviceIdentity.database.field.min_ets_version",
  "replaces_versions": "deviceIdentity.database.field.replaces_versions",
  "source_bytes": "deviceIdentity.database.field.source_bytes",
  "source_name": "deviceIdentity.database.field.source_name",
  "program_id": "deviceIdentity.database.field.program_id",
  "xpath": "deviceIdentity.database.field.xpath",
  "kind": "deviceIdentity.database.field.kind",
  "occurrences": "deviceIdentity.database.field.occurrences",
  "sample": "deviceIdentity.database.field.sample",
};
const SECTION_LABELS: Record<string, MessageKey> = {
  package: "deviceIdentity.database.section.package",
  "manufacturer": "deviceIdentity.database.section.manufacturer",
  "product": "deviceIdentity.database.section.product",
  "hardware": "deviceIdentity.database.section.hardware",
  "programLink": "deviceIdentity.database.section.programLink",
  "application": "deviceIdentity.database.section.application",
  "catalogItem": "deviceIdentity.database.section.catalogItem",
  "catalogSection": "deviceIdentity.database.section.catalogSection",
  "source": "deviceIdentity.database.section.source",
  "additionalAttributes": "deviceIdentity.database.section.additionalAttributes",
};

export function ProductDatabaseDetails({ sections }: { sections: ProductDetailSection[] }) {
  const t = useTranslate();
  return <div className="product-database-details">
    <h4>{t("deviceIdentity.database.title")}</h4>
    <p className="identity-note">{t("deviceIdentity.database.note")}</p>
    {sections.map((section) => <details className="identity-group" key={`${section.kind}:${section.id}`}>
      <summary>{SECTION_LABELS[section.kind] ? t(SECTION_LABELS[section.kind]) : section.kind} · <span className="mono">{section.id}</span></summary>
      <dl className="identity-fields">
        {section.fields.map((field, index) => <div className="identity-row" key={`${field.key}:${index}`}>
          <dt>{FIELD_LABELS[field.key] ? t(FIELD_LABELS[field.key]) : field.key}</dt>
          <dd>{field.value === "" ? t("deviceIdentity.database.empty") : field.value}</dd>
        </div>)}
      </dl>
    </details>)}
  </div>;
}
