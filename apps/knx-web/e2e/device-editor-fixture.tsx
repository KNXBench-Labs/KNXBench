/** Renders the real Inspector against static project data; no KNX server or device is involved. */
import { createRoot } from "react-dom/client";
import Inspector from "../src/Inspector";
import type { ProjectTree } from "../src/bindings/ProjectTree";
import type { DeviceDetail } from "../src/bindings/DeviceDetail";
import { setSetting } from "../src/settingsStore";
import { UI_LANGUAGE_STORAGE_KEY } from "../src/uiLanguage";
import "../src/styles.css";

const language = new URLSearchParams(location.search).get("lang") === "de" ? "de" : "en";
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

createRoot(document.getElementById("root")!).render(
  <main className="workbench issue09-browser-fixture">
    <Inspector
      selection={{ kind: "device", id: 42 }}
      tree={tree}
      deviceDetail={detail}
      onApplied={() => {}}
      onDeleted={() => {}}
    />
  </main>,
);
