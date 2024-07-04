# SWC-coverage-instrument

`swc-coverage-instrument` is a set of packages to support [istanbuljs](https://github.com/istanbuljs/istanbuljs) compatible coverage instrumentation in [SWC](https://github.com/swc-project/swc)'s transform passes. Instrumentation transform can be performed either via SWC's wasm-based plugin, or using custom passes in rust side transform chains.

## What does compatible exactly means?

This instrumentation will generate a data struct mimics istanbuljs's `FileCoverage` [object](https://github.com/istanbuljs/istanbuljs/blob/c7693d4608979ab73ebb310e0a1647e2c51f31b6/packages/istanbul-lib-coverage/lib/file-coverage.js#L97=) conforms fixture test suite from istanbuljs itself.

However, this doesn't mean instrumentation supports exact same [interfaces](https://github.com/istanbuljs/istanbuljs/blob/c7693d4608979ab73ebb310e0a1647e2c51f31b6/packages/istanbul-lib-instrument/src/source-coverage.js#L37=) surrounding coverage object as well as supporting exact same options. There are some fundamental differences between runtime, and ast visitor architecture between different compilers does not allow identical behavior. This package will try `best attempt` as possible.

**NOTE: Package can have breaking changes without major semver bump**

Given SWC's plugin interface itself is under experimental stage does not gaurantee semver-based major bump yet, this package also does not gaurantee semver compliant breaking changes yet. Please refer changelogs if you're encountering unexpected breaking behavior across versions.

# Usage

## Using SWC's wasm-based experimental plugin

First, install package via npm:

```
npm install --save-dev swc-plugin-coverage-instrument
```

Then add plugin into swc's configuration:

```
const pluginOptions: InstrumentationOptions = {...}

jsc: {
  ...
  experimental: {
    plugins: [
      ["swc-plugin-coverage-instrument", pluginOptions]
    ]
  }
}
```

`InstrumentationOptions` is a subset of istanbul's instrumentation options. Refer [istanbul's option](https://github.com/istanbuljs/istanbuljs/blob/master/packages/istanbul-lib-instrument/src/instrumenter.js#L16-L27=) for the same configuration flags. However there are few exceptions or differences, referencing [InstrumentOptions](https://github.com/kwonoj/swc-plugin-coverage-instrument/blob/4689fc9d281e11c875edd2376e8d92819472b9fe/packages/swc-coverage-instrument/src/options/instrument_options.rs#L22-L33) will list all possible options.

```
interface InstrumentationOptions {
  coverageVariable?: String,
  compact?: bool,
  reportLogic?: bool,
  ignoreClassMethods?: Array<String>,
  inputSourceMap?: object,
  instrumentLog: {
    // Currently there aren't logs other than spans.
    // Enabling >= info can display span traces.
    level: 'trace' | 'warn' | 'error' | 'info'
    // Emits spans along with any logs
    // Only effective if level sets higher than info.
    enableTrace: bool
  },
  unstableExclude?: Array<String>
}
```

## Using custom transform pass in rust

There is a single interface exposed to create a visitor for the transform, which you can pass into `before_custom_pass`.

```
let visitor = swc_coverage_instrument::create_coverage_instrumentation_visitor(
    source_map: std::sync::Arc<SourceMapper>,
    comments: C,
    instrument_options: InstrumentOptions,
    filename: String,
);

let fold = as_folder(visitor);
```

# Building / Testing

This package runs istanbuljs' fixture tests against SWC with its wasm plugin & custom transform both. `spec` contains set of the fixtures & unit test to run it, as well as supplimental packages to interop between instrumentation visitor to node.js runtime. `swc-coverage-instrument-wasm` exposes `FileCoverageInterop` allows to consume `FileCoverage` struct inside of js, and `swc-coverage-custom-transform` is an example implementation to run `before_custom_pass` with `swc-coverage-instrument` visitor.

Few npm scripts are supported for wrapping those setups.

- `build:all`: Build all relative packages as debug build.
- `test`: Runs unit test for wasm plugin & custom transform.
- `test:debug`: Runs unit test, but only for `debug-test.yaml` fixture. This is mainly for local dev debugging for individual test fixture behavior.
