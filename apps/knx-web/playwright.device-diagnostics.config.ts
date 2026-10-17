/** Runs the built shell against the loopback-only diagnostic simulator server. */
// Needs KNX_DIAG_EVIDENCE, `cargo build --example diagnostic_simulator` and
// `npm run build` first. The generic fixture gate does not impersonate this proof.
import "./browserTmp";
import {defineConfig} from "@playwright/test";
import {resolve,join} from "node:path";
const evidence=process.env.KNX_DIAG_EVIDENCE;
if(!evidence)throw new Error("KNX_DIAG_EVIDENCE must name an owned evidence directory");
export default defineConfig({
 testDir:"./e2e",testMatch:"**/device-diagnostics.integration.ts",outputDir:join(evidence,"browser-artifacts"),
 fullyParallel:false,workers:1,retries:0,timeout:45000,reporter:"list",
 use:{baseURL:"http://127.0.0.1:4317",acceptDownloads:true,trace:"retain-on-failure",launchOptions:{executablePath:"/usr/bin/chromium"}},
 webServer:{command:"../../target/debug/examples/diagnostic_simulator",cwd:resolve("."),url:"http://127.0.0.1:4317/healthz",reuseExistingServer:false,timeout:30000,
  env:{KNX_DIAG_SIM_LISTEN:"127.0.0.1:4317",KNX_DIAG_SIM_DATA:join(evidence,"server-data"),KNX_DIAG_SIM_WEB:resolve("dist"),XDG_RUNTIME_DIR:join(evidence,"runtime")}},
});
