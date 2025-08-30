// src/path-utils.ts
import { fileURLToPath, pathToFileURL } from "node:url";
function pathToUri(filePath) {
  return pathToFileURL(filePath).toString();
}
function uriToPath(uri) {
  return fileURLToPath(uri);
}
export {
  uriToPath,
  pathToUri
};
