// This file should be included in coverage instrumentation
// because it does NOT match any exclusion patterns

function add(a, b) {
  if (typeof a !== "number" || typeof b !== "number") {
    throw new Error("Both arguments must be numbers");
  }
  return a + b;
}

function multiply(x, y) {
  if (x === 0 || y === 0) {
    return 0;
  }
  return x * y;
}

function divide(dividend, divisor) {
  if (divisor === 0) {
    throw new Error("Cannot divide by zero");
  }
  return dividend / divisor;
}

const mathUtils = {
  isEven: function (num) {
    return num % 2 === 0;
  },

  isOdd: function (num) {
    return num % 2 !== 0;
  },

  factorial: function (n) {
    if (n < 0) return undefined;
    if (n === 0) return 1;
    return n * this.factorial(n - 1);
  },
};

if (typeof module !== "undefined") {
  module.exports = {
    add,
    multiply,
    divide,
    mathUtils,
  };
}
