import { cpSync, existsSync, mkdirSync, readFileSync, statSync } from "node:fs";
import { resolve } from "node:path";

type WakerConfigRuntimeAssetsPluginOptions = {
  mountBase?: string;
};

const DEFAULT_MOUNT_BASE = "/waker-config/";

function normalizeMountBase(basePath: string): string {
  const withLeadingSlash = basePath.startsWith("/") ? basePath : `/${basePath}`;
  return withLeadingSlash.endsWith("/") ? withLeadingSlash : `${withLeadingSlash}/`;
}

function contentTypeForPath(filePath: string): string {
  if (filePath.endsWith(".js")) return "text/javascript; charset=utf-8";
  if (filePath.endsWith(".json")) return "application/json; charset=utf-8";
  if (filePath.endsWith(".wasm")) return "application/wasm";
  if (filePath.endsWith(".onnx")) return "application/octet-stream";
  if (filePath.endsWith(".data")) return "application/octet-stream";
  if (filePath.endsWith(".npy")) return "application/octet-stream";
  if (filePath.endsWith(".npz")) return "application/octet-stream";
  return "application/octet-stream";
}

function runtimeRootPath(): string {
  return resolve(import.meta.dirname, "../runtime");
}

function resolveRequestFilePath(runtimeRoot: string, mountBase: string, requestUrl: string | undefined): string | null {
  const parsedPath = new URL(requestUrl ?? "/", "http://localhost").pathname;
  if (!parsedPath.startsWith(mountBase)) return null;
  const relativePath = parsedPath.slice(mountBase.length);
  const absolutePath = resolve(runtimeRoot, relativePath);
  return absolutePath.startsWith(runtimeRoot) ? absolutePath : null;
}

export function wakerConfigRuntimeAssetsPlugin(options: WakerConfigRuntimeAssetsPluginOptions = {}) {
  const mountBase = normalizeMountBase(options.mountBase ?? DEFAULT_MOUNT_BASE);
  const runtimeRoot = runtimeRootPath();
  let buildOutputDir: string | null = null;

  return {
    name: "waker-config-runtime-assets",
    configResolved(config: { build: { outDir: string }; root: string }) {
      buildOutputDir = resolve(config.root, config.build.outDir);
    },
    configureServer(server: {
      middlewares: {
        use(
          handler: (
            req: { url?: string },
            res: {
              end(body?: string | Uint8Array): void;
              setHeader(name: string, value: string): void;
              statusCode: number;
            },
            next: () => void,
          ) => void,
        ): void;
      };
    }) {
      server.middlewares.use((req, res, next) => {
        const filePath = resolveRequestFilePath(runtimeRoot, mountBase, req.url);
        if (!filePath || !existsSync(filePath) || statSync(filePath).isDirectory()) {
          next();
          return;
        }
        res.statusCode = 200;
        res.setHeader("Content-Type", contentTypeForPath(filePath));
        res.setHeader("Cross-Origin-Opener-Policy", "same-origin");
        res.setHeader("Cross-Origin-Embedder-Policy", "require-corp");
        res.end(readFileSync(filePath));
      });
    },
    writeBundle() {
      if (!buildOutputDir) return;
      const destinationRoot = resolve(buildOutputDir, mountBase.replace(/^\/+/, "").replace(/\/+$/, ""));
      mkdirSync(buildOutputDir, { recursive: true });
      cpSync(runtimeRoot, destinationRoot, { recursive: true });
    },
  };
}
