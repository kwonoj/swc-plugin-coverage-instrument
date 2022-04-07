const { defaults } = require("@istanbuljs/schema");
import { COVERAGE_MAGIC_KEY, COVERAGE_MAGIC_VALUE } from "./constants";
import {
  Expression,
  Module,
  ObjectExpression,
  parseSync,
  Program,
  Property,
  SpreadElement,
} from "@swc/core";
import { Visitor } from "@swc/core/Visitor";

function getAst(code: any): Module {
  if (typeof code === "object" && typeof code.type === "string") {
    // Assume code is already a babel ast.
    return code;
  }

  if (typeof code !== "string") {
    throw new Error("Code must be a string");
  }

  return parseSync(code, { syntax: "ecmascript", script: true });
}

class CoverageReadVisitor extends Visitor {
  private coverageScope: any;
  public getCoverageScope() {
    return this.coverageScope;
  }

  public visitObjectProperty(
    n: Property | SpreadElement
  ): Property | SpreadElement {
    /*
    const { node } = path;
      if (
        !node.computed &&
        path.get("key").isIdentifier() &&
        node.key.name === COVERAGE_MAGIC_KEY
      ) {
        const magicValue = path.get("value").evaluate();
        if (!magicValue.confident || magicValue.value !== COVERAGE_MAGIC_VALUE) {
          return;
        }
        covScope =
          path.scope.getFunctionParent() || path.scope.getProgramParent();
        path.stop();
      }*/
    return n;
  }
}

//TODO: Should not rely on babel to parse & get initial coverage
export function readInitialCoverage(code: any) {
  const ast = getAst(code);

  let visitor = new CoverageReadVisitor();
  visitor.visitProgram(ast);

  let covScope = visitor.getCoverageScope();

  if (!covScope) {
    return null;
  }

  const result = {};

  for (const key of ["path", "hash", "gcv", "coverageData"]) {
    const binding = covScope.getOwnBinding(key);
    if (!binding) {
      return null;
    }
    const valuePath = binding.path.get("init");
    const value = valuePath.evaluate();
    if (!value.confident) {
      return null;
    }
    result[key] = value.value;
  }

  delete result.coverageData[MAGIC_KEY];
  delete result.coverageData.hash;

  return result;
}
