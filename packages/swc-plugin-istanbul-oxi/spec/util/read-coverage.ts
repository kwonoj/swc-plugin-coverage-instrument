const { defaults } = require("@istanbuljs/schema");
import {
  Declaration,
  FunctionDeclaration,
  Module,
  parseSync,
  Property,
  SpreadElement,
  VariableDeclaration,
} from "@swc/core";
import { Visitor } from "@swc/core/Visitor";
import { getCoverageMagicConstants } from "../../../istanbul-oxi-instrument-wasm/pkg";

const { key: COVERAGE_MAGIC_KEY, value: COVERAGE_MAGIC_VALUE } =
  getCoverageMagicConstants();

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
  private current: FunctionDeclaration | null = null;
  private coverageScope: FunctionDeclaration | null = null;
  public getCoverageScope(): FunctionDeclaration | null {
    return this.coverageScope;
  }

  public visitFunctionDeclaration(n: FunctionDeclaration): Declaration {
    this.current = n;
    super.visitFunctionDeclaration(n);
    this.current = null;
    return n;
  }

  public visitObjectProperty(
    n: Property | SpreadElement
  ): Property | SpreadElement {
    if (n.type !== "KeyValueProperty") {
      return n;
    }

    if (n.key.type === "Identifier" && n.key.value === COVERAGE_MAGIC_KEY) {
      if (
        n.value.type === "StringLiteral" &&
        n.value.value === COVERAGE_MAGIC_VALUE
      ) {
        this.coverageScope = this.current;
      }
    }
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

  const declarations = covScope.body.stmts
    .map(
      (stmt) =>
        (stmt.type === "VariableDeclaration"
          ? stmt
          : null) as any as VariableDeclaration
    )
    .filter(Boolean);

  for (const key of ["path", "hash", "gcv", "coverageData"]) {
    const binding = declarations.reduce((acc, value) => {
      if (!acc) {
        acc = value.declarations.find(
          (decl) => decl.id.type === "Identifier" && decl.id.value === key
        );
      }

      return acc;
    }, null as any);

    if (!binding) {
      return null;
    }

    const valuePath = binding.init;

    function setPropertiesRecursive(
      obj: Record<string, any>,
      binding: any,
      resultKey: string
    ) {
      if (binding?.value !== null && binding?.value !== undefined) {
        obj[resultKey] = binding?.value;
      } else if (binding?.properties) {
        obj[resultKey] = {};
        binding?.properties.forEach((p) => {
          setPropertiesRecursive(obj[resultKey], p.value, p.key.value);
        });
      } else if (binding?.elements) {
        binding?.elements.forEach((elem, idx) => {
          if (!Array.isArray(obj[resultKey])) {
            obj[resultKey] = [];
          }

          if (elem?.expression?.properties) {
            setPropertiesRecursive(obj[resultKey], elem?.expression, idx);
          } else if (
            elem?.expression?.value !== null &&
            elem?.expression?.value !== undefined
          ) {
            obj[resultKey].push(elem.expression?.value);
          }
        });
      }
    }

    setPropertiesRecursive(result, valuePath, key);
  }

  delete result.coverageData[COVERAGE_MAGIC_KEY];
  delete result.coverageData.hash;

  return result;
}
