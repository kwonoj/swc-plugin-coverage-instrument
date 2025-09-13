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

  it("should normalize paths", () => {
    const code = `console.log('hello world');`;

    const output = instrumentSync(
      code,
      "C:\\Users\\project\\test\\index.test.ts",
      undefined,
      {
        unstableExclude: ["**/test/**"],
      },
    );

    assert.equal(
      output.code,
      `"use strict";
${code}
`,
    );
  });

  it("should preserve emotion styled component labels with template literals", () => {
    // This reproduces the issue from GitHub #247
    // Input: code AFTER emotion processing (as shown in the GitHub issue)
    const code = `export var TabsList = /*#__PURE__*/ styled(TabsListCore, {
  target: "ebt2y835",
  label: "TabsList"
})("margin:0 auto;width:fit-content;");`;

    const output = instrumentSync(code, "test-emotion.js");

    // Expected output: should preserve label like v0.0.20 did (from GitHub issue)
    // The key difference: label should remain "TabsList", not become ""
    const expectedOutput = `"use strict";
Object.defineProperty(exports, "__esModule", {
    value: true
});
Object.defineProperty(exports, "TabsList", {
    enumerable: true,
    get: function() {
        return TabsList;
    }
});
var TabsList = (cov_14220330533750098279().s[0]++, /*#__PURE__*/ styled(TabsListCore, {
    target: "ebt2y835",
    label: "TabsList"
})("margin:0 auto;width:fit-content;")); /*__coverage_data_json_comment__::{"all":false,"path":"test-emotion.js","statementMap":{"0":{"start":{"line":1,"column":36},"end":{"line":4,"column":38}}},"fnMap":{},"branchMap":{},"s":{"0":0},"f":{},"b":{}}*/ 
function cov_14220330533750098279() {
    var path = "test-emotion.js";
    var hash = "15339889637910252771";
    var global = new ((function(){}).constructor)("return this")();
    var gcv = "__coverage__";
    var coverageData = {
        all: false,
        path: "test-emotion.js",
        statementMap: {
            "0": {
                start: {
                    line: 1,
                    column: 36
                },
                end: {
                    line: 4,
                    column: 38
                }
            }
        },
        fnMap: {},
        branchMap: {},
        s: {
            "0": 0
        },
        f: {},
        b: {},
        _coverageSchema: "11020577277169172593",
        hash: "15339889637910252771"
    };
    var coverage = global[gcv] || (global[gcv] = {});
    if (!coverage[path] || coverage[path].$hash !== hash) {
        coverage[path] = coverageData;
    }
    var actualCoverage = coverage[path];
    {
        cov_14220330533750098279 = function() {
            return actualCoverage;
        };
    }
    return actualCoverage;
}
cov_14220330533750098279();`;

    // Compare whole output.code to the raw output as requested
    // This ensures emotion labels are preserved without explicitly asserting them
    assert.equal(
      output.code.trim(),
      expectedOutput.trim(),
      "Instrumented code should preserve emotion styled component label property",
    );
  });
});
