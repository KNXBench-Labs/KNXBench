/** Production-built universal support intake with fictional, intercepted device evidence. */
import "./browserTmp";
import {defineConfig} from "@playwright/test";
import base from "./playwright.config";
export default defineConfig({...base,testMatch:"**/universal-support.e2e.ts",webServer:{command:"npm run preview -- --host 127.0.0.1 --port 4173 --strictPort",url:"http://127.0.0.1:4173/",timeout:120000,reuseExistingServer:false}});
