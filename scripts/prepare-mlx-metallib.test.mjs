import assert from "node:assert/strict";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import test from "node:test";

import { findMetallibs } from "./prepare-mlx-metallib.mjs";

test("finds metallibs from current and legacy Qwen Cargo build directories", (t) => {
  const targetRoot = fs.mkdtempSync(path.join(os.tmpdir(), "qwenasr-metallib-"));
  t.after(() => fs.rmSync(targetRoot, { recursive: true, force: true }));

  const currentPath = createMetallib(targetRoot, "release", "qwen3-asr-rs-current");
  const legacyPath = createMetallib(targetRoot, "release", "qwen3_asr-legacy");
  createMetallib(targetRoot, "release", "unrelated-crate-build");

  assert.deepEqual(
    findMetallibs([targetRoot], ["release"]).sort(),
    [currentPath, legacyPath].sort(),
  );
});

test("only searches the requested Cargo profile", (t) => {
  const targetRoot = fs.mkdtempSync(path.join(os.tmpdir(), "qwenasr-metallib-"));
  t.after(() => fs.rmSync(targetRoot, { recursive: true, force: true }));

  const debugPath = createMetallib(targetRoot, "debug", "qwen3-asr-rs-debug");
  createMetallib(targetRoot, "release", "qwen3-asr-rs-release");

  assert.deepEqual(findMetallibs([targetRoot], ["debug"]), [debugPath]);
});

function createMetallib(targetRoot, profile, buildDirectory) {
  const metallibPath = path.join(targetRoot, profile, "build", buildDirectory, "out", "lib", "mlx.metallib");
  fs.mkdirSync(path.dirname(metallibPath), { recursive: true });
  fs.writeFileSync(metallibPath, buildDirectory);
  return metallibPath;
}
