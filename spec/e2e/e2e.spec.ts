import { assert } from "chai";
import { instrumentSync } from "../util/verifier";
import * as fs from "fs";
import * as path from "path";

// Custom transform test doesn't use plugin's exclude option
const tryDescribe = process.env.SWC_TRANSFORM_CUSTOM ? describe.skip : describe;

tryDescribe("e2e", () => {
  it("issue-274", () => {
    const testFiles = [
      {
        path: path.resolve(__dirname, "fixtures", "should-be-excluded.test.js"),
        shouldBeExcluded: true,
        description: "Test file with .test. extension should be excluded",
      },
      {
        path: path.resolve(__dirname, "fixtures", "should-be-included.js"),
        shouldBeExcluded: false,
        description: "Regular file should be included",
      },
    ];

    const exclusionPattern = [
      "**/node_modules/**",
      "**/dist/**",
      "**/test/**",
      "**/__tests__/**",
      "**/__mocks__/**",
      "**/*.{test,spec}.[jt]s",
      "**/*.{test,spec}.[c|m][jt]s",
      "**/*.{test,spec}.[jt]sx",
      "**/*.{test,spec}.[c|m][jt]sx",
    ];

    testFiles.forEach((testFile) => {
      // Read the actual file content
      const fileContent = fs.readFileSync(testFile.path, "utf8");

      // Transform the file using SWC with exclusion patterns
      const output = instrumentSync(fileContent, testFile.path, undefined, {
        unstableExclude: exclusionPattern,
      });

      if (testFile.shouldBeExcluded) {
        // File should be excluded - no coverage instrumentation
        assert.notInclude(
          output.code,
          "__coverage__",
          `${testFile.description}: File should not contain coverage variables`,
        );
        assert.notInclude(
          output.code,
          "cov_",
          `${testFile.description}: File should not contain coverage function calls`,
        );
      } else {
        // File should be included - should have coverage instrumentation
        assert.include(
          output.code,
          "__coverage__",
          `${testFile.description}: File should be instrumented with coverage`,
        );
        assert.include(
          output.code,
          "cov_",
          `${testFile.description}: File should contain coverage function calls`,
        );
      }
    });

    // Test cross-platform path normalization with different path formats
    const testCode = fs.readFileSync(testFiles[0].path, "utf8");
    const pathVariations = [
      "project/test/file.test.js", // Unix-style path
      "project\\test\\file.test.js", // Windows-style path
      "C:\\Users\\project\\test\\file.test.js", // Windows absolute path
      "/home/user/project/test/file.test.js", // Unix absolute path
    ];

    pathVariations.forEach((testPath) => {
      const output = instrumentSync(testCode, testPath, undefined, {
        unstableExclude: ["**/*.test.*"],
      });

      // All should be excluded since they all match the *.test.* pattern
      assert.notInclude(
        output.code,
        "__coverage__",
        `Path ${testPath} should be excluded regardless of separator style`,
      );
      assert.notInclude(
        output.code,
        "cov_",
        `Path ${testPath} should not contain coverage calls`,
      );
    });
  });
});
