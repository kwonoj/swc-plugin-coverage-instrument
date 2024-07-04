import { assert } from "chai";
import { getCoverageMagicConstants } from "./swc-coverage-instrument-wasm/pkg/swc_coverage_instrument_wasm";
import { instrumentSync } from "./util/verifier";

// dummy: initiate wasm compilation before any test runs
getCoverageMagicConstants();
instrumentSync(`console.log('boo')`, "anon");

const tryDescribe = process.env.SWC_TRANSFORM_CUSTOM ? describe.skip : describe;

tryDescribe("Plugin options", () => {
  it("should able to exclude", () => {
    const code = `console.log('hello');`;

    const output = instrumentSync(
      code,
      "somepath/file/excluded.js",
      undefined,
      {
        unstableExclude: ["somepath/**/excluded.*"],
      },
    );

    assert.equal(
      output.code,
      `"use strict";
${code}
`,
    );
  });
});
