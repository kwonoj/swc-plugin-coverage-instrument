// This file should be excluded from coverage instrumentation
// because it matches the **/*.test.* pattern

function calculateSum(a, b) {
  return a + b;
}

function calculateProduct(x, y) {
  return x * y;
}

const testHelper = {
  setup: function () {
    console.log("Setting up test");
  },

  teardown: function () {
    console.log("Tearing down test");
  },
};

if (typeof module !== "undefined") {
  module.exports = {
    calculateSum,
    calculateProduct,
    testHelper,
  };
}
