/** Builds the isolated instruction editor fixture without deployment. */
import { build } from "vite";
import path from "node:path";
const outDir = process.argv[2];
if (!outDir || !path.isAbsolute(outDir)) throw new Error("An owned absolute fixture output directory is required");
await build({configFile:path.resolve("vite.config.ts"),cacheDir:path.join(outDir,".vite-cache"),build:{outDir,emptyOutDir:false,rollupOptions:{input:path.resolve("src/features/rtf/fixture.html")}}});
