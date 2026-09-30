/** Renders the real Inspector against static project data; no KNX server or device is involved. */
import { createRoot } from "react-dom/client";
import Inspector from "../src/Inspector";
import type { ProjectTree } from "../src/bindings/ProjectTree";
import type { DeviceDetail } from "../src/bindings/DeviceDetail";
import { setSetting } from "../src/settingsStore";
import { UI_LANGUAGE_STORAGE_KEY } from "../src/uiLanguage";
import "../src/styles.css";

const language = new URLSearchParams(location.search).get("lang") === "de" ? "de" : "en";
const withChannels = new URLSearchParams(location.search).get("channels") === "1";
if (withChannels) document.documentElement.dataset.theme = "porcelain";
setSetting(UI_LANGUAGE_STORAGE_KEY, language);

const longName = "Main corridor switching actuator channel with a very long manufacturer-specific designation";
const longAddressName = "Workshop lighting group address with a name that must remain legible on a narrow display";
const tree: ProjectTree = {
  schema_version: 11,
  errors: 0,
  warnings: 0,
  can_undo: false,
  can_redo: false,
  is_modified: false,
  group_address_style: "ThreeLevel",
  installations: [{
    id: 1,
    name: "Installation",
    topology: [{
      id: 10,
      name: "Area",
      address: 1,
      lines: [{
        id: 11,
        name: "Line",
        address: 1,
        devices: [{ id: 42, name: longName, address: "1.1.12", description: null, com_object_count: 1 }],
      }],
    }],
    buildings: [],
    unassigned: [],
    group_addresses: [{ id: 9, name: longAddressName, address: "15/7/255", range: null, dpts: [], links: [] }],
    group_ranges: [],
  }],
};

const detail: DeviceDetail = {
  id: 42,
  name: longName,
  address: "1.1.12",
  description: null,
  product: { product_ref: null, program_ref: null, catalog: null, resolution: "NoReference" },
  com_objects: [{
    id: 7,
    number: 1,
    name: longName,
    dpt: "DPST-1-1",
    dpt_layer: null,
    description: null,
    description_layer: null,
    is_active: true,
    activation: "NotEvaluated",
    channel: null,
    program_dpt: null,
    dpt_text: null,
    function_text: null,
    read: false,
    write: true,
    transmit: false,
    update: false,
    communication: true,
    read_on_init: false,
    links: [
      { ga_id: 9, address: "15/7/255", name: longAddressName, direction: "Send" },
      { ga_id: 9, address: "15/7/255", name: longAddressName, direction: "Receive" },
    ],
  }],
};

// Synthetic, fully local projection evidence. These channel keys are opaque and
// the deliberately sparse order differs from the communication-object order.
const channelDetail: DeviceDetail = {
  ...detail,
  product: {
    product_ref: "P-MOCK",
    program_ref: "H2P-MOCK",
    resolution: "Resolved",
    catalog: {
      manufacturer_id: "M-MOCK",
      manufacturer_name: null,
      product_text: null,
      order_number: null,
      hardware_name: null,
      hardware_version: null,
      hardware_serial_number: null,
      catalog_item_name: null,
      catalog_item_number: null,
      application_program_id: "A-MOCK",
      application_name: null,
      application_number: null,
      application_version: null,
      mask_version: null,
    },
  },
  com_objects: [
    {
      ...detail.com_objects[0],
      channel: { key: "opaque:hall", kind: "Channel", text: "Hall outputs", name: null, number: "19", order: 42 },
      activation: "Active",
      function_text: "Switching lights",
      dpt_text: "Switch",
    },
    {
      ...detail.com_objects[0],
      id: 8, number: 2, name: "Thermometer", dpt: null, links: [],
      channel: { key: "opaque:probe", kind: "Channel", text: null, name: "Raw manufacturer name", number: "A-5", order: 19 },
      activation: "Active",
      program_dpt: "DPST-9-1",
      dpt_text: "Temperature",
      function_text: "Measured value",
    },
    { ...detail.com_objects[0], id: 9, number: 3, name: "Inactive object", dpt: null, links: [], activation: "Inactive", is_active: false },
    { ...detail.com_objects[0], id: 10, number: 4, name: "Uncertain object", dpt: null, links: [], activation: "Undetermined" },
    { ...detail.com_objects[0], id: 11, number: 5, name: "Unevaluated object", dpt: null, links: [], activation: "NotEvaluated" },
  ],
};

createRoot(document.getElementById("root")!).render(
  <main className="workbench issue09-browser-fixture">
    <Inspector
      selection={{ kind: "device", id: 42 }}
      tree={tree}
      deviceDetail={withChannels ? channelDetail : detail}
      onApplied={() => {}}
      onDeleted={() => {}}
    />
  </main>,
);
