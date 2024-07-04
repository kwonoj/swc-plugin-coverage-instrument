# SWC-coverage-instrument

`swc-coverage-instrument` is a set of packages to support [istanbuljs](https://github.com/istanbuljs/istanbuljs) compatible coverage instrumentation in [SWC](https://github.com/swc-project/swc)'s transform passes. Instrumentation transform can be performed either via SWC's wasm-based plugin, or using custom passes in rust side transform chains.

## What does compatible exactly means?

This instrumentation will generate a data struct mimics istanbuljs's `FileCoverage` [object] (https://github.com/istanbuljs/istanbuljs/blob/c7693d4608979ab73ebb310e0a1647e2c51f31b6/packages/istanbul-lib-coverage/lib/file-coverage.js#L97=) conforms fixture test suite from istanbuljs itself.

However, this doesn't mean instrumentation supports exact same [interfaces](https://github.com/istanbuljs/istanbuljs/blob/c7693d4608979ab73ebb310e0a1647e2c51f31b6/packages/istanbul-lib-instrument/src/source-coverage.js#L37=) surrounding coverage object as well as supporting exact same options. There are some fundamental differences between runtime, and ast visitor architecture between different compilers does not allow identical behavior. This package will try `best attempt` as possible.

**NOTE: Package can have breaking changes without major semver bump**

While stablzing its interfaces, this package does not gaurantee semver compliant breaking changes yet. Please refer changelogs if you're encountering unexpected breaking behavior across versions.

# Usage

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

`InstrumentationOptions` is a subset of istanbul's instrumentation options. Refer [istanbul's option](https://github.com/istanbuljs/istanbuljs/blob/master/packages/istanbul-lib-instrument/src/instrumenter.js#L16-L27=) for the same configuration flags. However there are few exceptions or differences, referencing [InstrumentOptions](https://github.com/kwonoj/swc-plugin-coverage-instrument/blob/main/packages/swc-coverage-instrument/src/options/instrument_options.rs) will list all possible options.

For the logging, this package does not init any subscriber by itself. Caller should setup proper `tracing-subscriber` as needed.
