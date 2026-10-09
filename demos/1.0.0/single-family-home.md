# Single-Family Home (fictional demo)

Version 1.0.0. Original fictional offline exploration project. All device/program data are synthetic; none is firmware, certified, ETS-validated or suitable for a real device download.

## Start here

1. Unzip to a working folder. Keep a second, untouched copy for reset.
2. In KNXBench, open **Product catalog**, choose its import action and install `fictional-demo-devices.knxprod` from this download. This adds the fictional products; do not replace your product database. The same package serves all three demos and a byte-identical retry is deduplicated.
3. Open `single-family-home.knxdb` using **File → Open (.knxdb)…**. In the browser version, upload/select it through the file picker; the desktop offers a native picker for the same file. Choose English in Settings if desired; project content itself is English. Acceptance here uses the real web UI, not a new native-shell verification.
4. Explore **Buildings**, **Topology**, **Group addresses** and a device's **Communication objects**, **Parameters** and **Product data** tabs.

This project has 32 devices, 105 group addresses and 22 building-space nodes (including floors, rooms and distribution boards). Topology contains 2 lines, including the main line. Apparent device counts include illustrative couplers/interface; power supplies, cable lengths, bus-current budgets and real filter tables are NOT planned here.

## Short tour

- Buildings show rooms and the distribution boards holding their actuators. Open an actuator's description for exact channel-to-room assignments.
- Topology keeps one area and an explicit main TP line. The illustrative IP interface is a project participant, NOT a configured gateway. A child-line coupler is shown at its `1.line.0` address. This is an instructional relationship, not tested routing or isolation.
- Main group 0 = switched circuits; 1 = separately dimmed circuits; 2 = blinds; 3 = heating; 4 = occupancy; 5 = scene recall; 6 = central commands. Middle groups name building zones. Names distinguish each room's Command and Status signals.
- Wall controllers originate local commands. Actuators receive commands and originate status. Presence detector objects feed the configured lighting occupancy inputs. Heating controllers receive measured temperature/setpoint and expose valve status. These are declarations and links, NOT running control logic.
- Scene addresses use DPT 17.001 for recall only (no scene learning/storage); central off uses DPT 1.001 with value 0. No scene tables or actual output states are simulated.

## Five exercises (use a working copy)

1. **Trace an existing connection.** Find `0/0/1` — `Living room / Switch command`. Inspect its sender/receiver and their communication flags. Confirm DPT 1.001 on both ends. Do not connect to a bus.
2. **Edit the device description.** Find `1.1.19` — `Ground floor / Switch actuator 4-channel 02`. In Properties append `Practice note: reviewed offline.` to Description and leave the field (Tab) to commit. Device names are not editable in the current UI; this exercise uses the supported description command.
3. **Edit a supported parameter.** On that device's Parameters tab change `Delay after bus voltage recovery (s)` from 2 to 5. Leave the field (Tab) to commit; verify the stored value is 5. This edits project intent only, never a physical device.
4. **Add a group and a reserve-channel link.** In the explorer's Group Addresses branch add `0/0/200`, Name `Practice light / Switch command`, selecting the Switching / Ground floor middle range (`0/0/0–0/0/255`). Open the practice actuator's Communication objects tab, expand its reserve channel and object 4 (`Channel 2: Switch command`); choose the new group, choose Receive, then Link. The group initially has no declared DPT; after linking it **inherits DPT 1.001** from the object. That object is an unused reserve in the baseline. This practice group deliberately has no sender yet; it is an unfinished authoring exercise, not a working installation. Check for no DPT mismatch.
5. **Save and reopen.** Use Save as to create `single-family-home-practice.knxdb`. Reopen it and check the description note, parameter 5, new group and receiver link. Reset by reopening an untouched extracted copy; native projects auto-save some edits once they have a store path, so retain a separate original. Do not overwrite your own projects.

## Honest boundaries

Unassigned actuator channels and unused wall-controller functions are deliberate reserves. Product-program information and the parameter panel need the shared fictional catalogue; retained native values alone are not a complete catalogue experience. All download/load flags remain false. No bus traffic, live values, firmware, memory images, certified design, Secure, access control or fire alarm planning. Separate apartment groups/lines do not establish security boundaries. Never download these fictional programmes to real hardware.

Read `SHA256SUMS` to verify the extracted files. Native schema/build provenance is in the combined download's manifest; compatibility applies only to the actually recorded tested build, not every previous alpha.
