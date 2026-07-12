"use strict";

const test = require("node:test");
const assert = require("node:assert/strict");
const {
  binaryVersion,
  downloadUrl,
  releaseBaseUrl,
  releaseVersion,
  targetFor,
} = require("../lib/platform");
const { binaryVersion: pinnedBinaryVersion } = require("../package.json");

test("maps supported platforms to release assets", () => {
  assert.deepEqual(targetFor("linux", "x64"), {
    asset: "rgotify-x86_64.tar.gz",
    binary: "rgotify",
  });
  assert.deepEqual(targetFor("win32", "x64"), {
    asset: "rgotify-windows-x86_64.tar.gz",
    binary: "rgotify.exe",
  });
});

test("rejects unsupported platforms", () => {
  assert.throws(() => targetFor("darwin", "arm64"), /Unsupported platform/);
});

test("uses pinned binary version as the binary tag by default", () => {
  assert.equal(binaryVersion(), pinnedBinaryVersion);
  assert.equal(releaseVersion({}), `v${pinnedBinaryVersion}`);
});

test("allows release tag and repo overrides", () => {
  const env = { GOTIFY_RMCP_BINARY_VERSION: "v9.9.9", GOTIFY_RMCP_REPO: "example/gotify-rmcp" };
  assert.equal(releaseBaseUrl(env), "https://github.com/example/gotify-rmcp/releases/download");
  assert.equal(downloadUrl(targetFor("linux", "x64"), env), "https://github.com/example/gotify-rmcp/releases/download/v9.9.9/rgotify-x86_64.tar.gz");
});
