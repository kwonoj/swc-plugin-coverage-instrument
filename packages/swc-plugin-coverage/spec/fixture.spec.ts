import * as path from "path";
import * as fs from "fs";
import * as yaml from "js-yaml";
import { create, instrumentSync } from "./util/verifier";
import * as guards from "./util/guards";
import { assert } from "chai";
import { getCoverageMagicConstants } from "../../istanbul-oxi-instrument-wasm/pkg/istanbul_oxi_instrument_wasm";

// dummy: initiate wasm compilation before any test runs
getCoverageMagicConstants();
instrumentSync(`console.log('boo')`, "anon");

const clone: typeof import("lodash.clone") = require("lodash.clone");

const dir = path.resolve(__dirname, "fixtures");
const files = fs.readdirSync(dir).filter((f) => {
  let match = true;
  if (process.env.FILTER) {
    match = new RegExp(`.*${process.env.FILTER}.*`).test(f);
  }
  return f.match(/\.yaml$/) && match;
});

function loadDocs() {
  const docs = [];
  files.forEach((f) => {
    const filePath = path.resolve(dir, f);
    const contents = fs.readFileSync(filePath, "utf8");
    try {
      yaml.loadAll(contents, (obj) => {
        obj.file = f;
        docs.push(obj);
      });
    } catch (ex) {
      docs.push({
        file: f,
        name: "loaderr",
        err: "Unable to load file [" + f + "]\n" + ex.message + "\n" + ex.stack,
      });
    }
  });
  return docs;
}

function generateTests(docs) {
  docs.forEach((doc) => {
    const guard = doc.guard;
    let skip = false;
    let skipText = "";

    if (guard && guards[guard]) {
      if (!guards[guard]()) {
        skip = true;
        skipText = "[SKIP] ";
      }
    }

    describe(skipText + doc.file + "/" + (doc.name || "suite"), () => {
      if (doc.err) {
        it("has errors", () => {
          console.error(doc.err);
          assert.ok(false, doc.err);
        });
      } else {
        (doc.tests || []).forEach((t) => {
          const fn = async function () {
            const genOnly = (doc.opts || {}).generateOnly;
            const noCoverage = (doc.opts || {}).noCoverage;
            const opts = doc.opts || {};
            opts.filename = path.resolve(__dirname, doc.file);
            opts.transformOptions = {
              isModule: doc?.instrumentOpts?.esModules,
            };
            const v = create(
              doc.code,
              opts,
              doc.instrumentOpts,
              doc.inputSourceMap
            );
            const test = clone(t);
            const args = test.args;
            const out = test.out;
            delete test.args;
            delete test.out;
            if (!genOnly && !noCoverage) {
              await v.verify(args, out, test);
            }
            if (noCoverage) {
              assert.equal(v.code, v.generatedCode);
            }
          };
          if (skip) {
            it.skip(t.name || "default test", fn);
          } else {
            it(t.name || "default test", fn);
          }
        });
      }
    });
  });
}

generateTests(loadDocs());
