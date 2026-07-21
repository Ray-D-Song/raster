# raster_runtime

raster_runtime is a lightweight JavaScript runtime built on QuickJS.

This project is forked from [LLRT](https://github.com/awslabs/llrt) and is maintained for the Raster project. Because this fork is diverging substantially from upstream LLRT, future development will continue through manual upstream synchronization where useful, or without upstream synchronization when the Raster runtime requirements differ.

It's built in Rust, utilizing QuickJS as the JavaScript engine, ensuring efficient memory usage and swift startup.

> [!WARNING]
> raster_runtime is an **experimental** package. It is subject to change and intended only for evaluation purposes.

> [!IMPORTANT]
> Even though raster_runtime supports [ES2023](https://262.ecma-international.org/14.0/) it's **NOT** a drop in replacement for Node.js. Consult [Compatibility matrix](#compatibility-matrix) and [API](API.md) for more details.
> All dependencies should be bundled for a `browser` platform.

## Testing & ensuring compatibility

The best way to ensure your code is compatible with raster_runtime is to write tests and execute them using the built-in test runner. The test runner currently supports Jest/Chai assertions. There are three main types of tests you can create:

Unit Tests

- Useful for validating specific modules and functions in isolation
- Allow focused testing of individual components

Web Platform Tests (WPT)

- Useful for validating raster_runtime’s behavior against standardized browser APIs and runtime expectations
- Ensure compatibility with web standards and cross-runtime environments
- Help verify alignment with WinterTC and broader JavaScript ecosystem
- For setup instructions and how to run WPT in raster_runtime, see [here](tests/wpt/README.md).

### Test runner

Test runner uses a lightweight Jest-like API and supports Jest/Chai assertions. For examples on how to implement tests for raster_runtime see the `/tests` folder of this repository.

To run tests, execute the `raster_runtime test` command. raster_runtime scans the current directory and sub-directories for files that ends with `*.test.js` or `*.test.mjs`. You can also provide a specific test directory to scan by using the `raster_runtime test -d <directory>` option.

The test runner also has support for filters. Using filters is as simple as adding additional command line arguments, i.e: `raster_runtime test crypto` will only run tests that match the filename containing `crypto`.

## Compatibility matrix

> [!NOTE]
> raster_runtime only support a fraction of the Node.js APIs. It is **NOT** a drop in replacement for Node.js, nor will it ever be. Below is a high level overview of partially supported APIs and modules. For more details consult the [API](API.md) documentation

| [Node.js API](https://nodejs.org/api/index.html) | Node.js | raster_runtime  |
| ------------------------------------------------ | ------- | ----- |
| node:assert                                      | ✔︎       | ✔︎️⚠️   |
| node:async_hooks                                 | ✔︎       | ✔︎️⚠️   |
| node:buffer                                      | ✔︎       | ✔︎️⚠️   |
| node:child_process                               | ✔︎       | ✔︎⚠️   |
| node:cluster                                     | ✔︎       | ✘     |
| node:console                                     | ✔︎       | ✔︎⚠️   |
| node:crypto                                      | ✔︎       | ✔︎⚠️   |
| node:dgram                                       | ✔︎       | ✘     |
| node:diagnostics_channel                         | ✔︎       | ✘     |
| node:dns                                         | ✔︎       | ✔︎⚠️   |
| node:events                                      | ✔︎       | ✔︎⚠️   |
| node:fs                                          | ✔︎       | ✔︎⚠️   |
| node:fs/promises                                 | ✔︎       | ✔︎⚠️   |
| node:http                                        | ✔︎       | ✔︎⚠️   |
| node:http2                                       | ✔︎       | ✘     |
| node:https                                       | ✔︎       | ✘⏱    |
| node:inspector                                   | ✔︎       | ✘     |
| node:inspector/promises                          | ✔︎       | ✘     |
| node:module                                      | ✔︎       | ✔︎⚠️   |
| node:net                                         | ✔︎       | ✔︎⚠️   |
| node:os                                          | ✔︎       | ✔︎⚠️   |
| node:path                                        | ✔︎       | ✔︎⚠️   |
| node:perf_hooks                                  | ✔︎       | ✔︎⚠️   |
| node:process                                     | ✔︎       | ✔︎⚠️   |
| node:querystring                                 | ✔︎       | ✘     |
| node:readline                                    | ✔︎       | ✘     |
| node:readline/promises                           | ✔︎       | ✘     |
| node:repl                                        | ✔︎       | ✘     |
| node:sqlite                                      | ✔︎       | ✘     |
| node:stream                                      | ✔︎       | ✔︎\*   |
| node:stream/promises                             | ✔︎       | ✔︎\*   |
| node:stream/web                                  | ✔︎       | ✔︎⚠️   |
| node:string_decoder                              | ✔︎       | ✔︎     |
| node:test                                        | ✔︎       | ✘     |
| node:timers                                      | ✔︎       | ✔︎⚠️   |
| node:tls                                         | ✔︎       | ✘⏱    |
| node:tty                                         | ✔︎       | ✔︎⚠️   |
| node:url                                         | ✔︎       | ✔︎⚠️   |
| node:util                                        | ✔︎       | ✔︎⚠️   |
| node:v8                                          | ✔︎       | ✘\*\* |
| node:vm                                          | ✔︎       | ✘     |
| node:wasi                                        | ✔︎       | ✘     |
| node:worker_threads                              | ✔︎       | ✘     |
| node:zlib                                        | ✔︎       | ✔︎⚠️   |

| [raster_runtime API](https://github.com/ray-d-song/raster_runtime/blob/main/API.md) | Node.js | raster_runtime |
| ------------------------------------------------------------ | ------- | ---- |
| raster_runtime:hex                                                     | ✘       | ✔︎    |
| raster_runtime:qjs                                                     | ✘       | ✔︎    |
| raster_runtime:util                                                    | ✘       | ✔︎    |
| raster_runtime:xml                                                     | ✘       | ✔︎    |

| [Web Platform API](https://min-common-api.proposal.wintertc.org/) | raster_runtime |
| ----------------------------------------------------------------- | ---- |
| COMPRESSION                                                       | ✘⏱   |
| CONSOLE                                                           | ✔︎⚠️  |
| DOM                                                               | ✔︎⚠️  |
| ECMASCRIPT                                                        | ✔︎⚠️  |
| ENCODING                                                          | ✔︎⚠️  |
| FETCH                                                             | ✔︎⚠️  |
| FILEAPI                                                           | ✔︎⚠️  |
| HR-TIME                                                           | ✔︎    |
| HTML                                                              | ✔︎⚠️  |
| STREAMS                                                           | ✔︎⚠️  |
| URL                                                               | ✔︎    |
| URLPATTERN                                                        | ✘⏱   |
| WASM-JS-API-2                                                     | ✘    |
| WASM-WEB-API-2                                                    | ✘    |
| WEBCRYPTO                                                         | ✔︎⚠️  |
| WEBIDL                                                            | ✔︎⚠️  |
| XHR                                                               | ✔︎⚠️  |

| Other features | raster_runtime |
| -------------- | ---- |
| async/await    | ✔︎    |
| esm            | ✔︎    |
| cjs            | ✔︎    |
| Intl           | ✔︎⚠️  |
| Temporal       | ✔︎⚠️  |

_⚠️ = partially supported in RASTER_RUNTIME_<br />
_⏱ = planned partial support_<br />
_\* = Not native_<br />
_\*\* = The `module.registerHooks()` API allows you to emulate some functionality. See also `example/register-hooks`._<br />

## Using node_modules (dependencies) with raster_runtime

Since raster_runtime is meant for performance critical application it's not recommended to deploy `node_modules` without bundling, minification and tree-shaking.

raster_runtime can work with any bundler of your choice. Below are some configurations for popular bundlers:

> [!WARNING]
> raster_runtime implements native modules that are largely compatible with the following external packages.
> By implementing the following conversions in the bundler's alias function, your application may be faster, but we recommend that you test thoroughly as they are not fully compatible.

| Node.js         | raster_runtime     |
| --------------- | -------- |
| fast-xml-parser | raster_runtime:xml |

### ESBuild

```shell
esbuild index.js --platform=browser --target=es2023 --format=esm --bundle --minify
```

### Rollup

```javascript
import resolve from "@rollup/plugin-node-resolve";
import commonjs from "@rollup/plugin-commonjs";
import terser from "@rollup/plugin-terser";

export default {
  input: "index.js",
  output: {
    file: "dist/bundle.js",
    format: "esm",
    sourcemap: true,
    target: "es2023",
  },
  plugins: [resolve(), commonjs(), terser()],
};
```

### Webpack

```javascript
import TerserPlugin from "terser-webpack-plugin";
import nodeExternals from "webpack-node-externals";

export default {
  entry: "./index.js",
  output: {
    path: "dist",
    filename: "bundle.js",
    libraryTarget: "module",
  },
  target: "web",
  mode: "production",
  resolve: {
    extensions: [".js"],
  },
  externals: [nodeExternals()],
  optimization: {
    minimize: true,
    minimizer: [
      new TerserPlugin({
        terserOptions: {
          ecma: 2023,
        },
      }),
    ],
  },
};
```

## Running TypeScript with raster_runtime

Same principle as dependencies applies when using TypeScript. TypeScript must be bundled and transpiled into ES2023 JavaScript.

> [!NOTE]
> raster_runtime will not support running TypeScript without transpilation. This is by design for performance reasons. Transpiling requires CPU and memory that adds latency and cost during execution. This can be avoided if done ahead of time during deployment.

## Rationale

What justifies the introduction of another JavaScript runtime in light of existing options such as [Node.js](https://nodejs.org/en), [Bun](https://bun.sh) & [Deno](https://deno.com/)?

Node.js, Bun, and Deno represent highly proficient JavaScript runtimes. However, they are designed with general-purpose applications in mind and depend on a ([Just-In-Time compiler (JIT)](https://en.wikipedia.org/wiki/Just-in-time_compilation) for dynamic code compilation and optimization during execution. While JIT compilation offers substantial long-term performance advantages, it carries a computational and memory overhead.

In contrast, raster_runtime distinguishes itself by not incorporating a JIT compiler, a strategic decision that yields two significant advantages:

A) JIT compilation is a notably sophisticated technological component, introducing increased system complexity and contributing substantially to the runtime's overall size.

B) Without the JIT overhead, raster_runtime conserves both CPU and memory resources that can be more efficiently allocated to code execution tasks, thereby reducing application startup times.

## Limitations

There are many cases where raster_runtime shows notable performance drawbacks compared with JIT-powered runtimes, such as large data processing, Monte Carlo simulations or performing tasks with hundreds of thousands or millions of iterations. raster_runtime is most effective when applied to smaller programs such as data transformation, real time processing, authorization, and validation. It is designed to complement existing components rather than serve as a comprehensive replacement for everything. Notably, given its supported APIs are based on Node.js specification, transitioning back to alternative solutions requires minimal code adjustments.

## Building from source

1. Clone code and cd to directory

```
git clone git@github.com:ray-d-song/raster_runtime.git
cd raster_runtime
```

2. Install git submodules

```
git submodule update --init --checkout
```

3. Install rust

```
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | bash -s -- -y
source "$HOME/.cargo/env"
```

4. Install dependencies

```
# MacOS
brew install zig make cmake zstd node corepack

# Ubuntu
sudo apt -y install make zstd
sudo snap install zig --classic --beta

# Windows WSL2 (requires systemd to be enabled*)
sudo apt -y install cmake g++ gcc make zip zstd
sudo snap install zig --classic --beta

# Windows WSL2 (If Node.js is not yet installed)
sudo curl -o- https://raw.githubusercontent.com/nvm-sh/nvm/master/install.sh | bash
nvm install --lts
```

_\* See [Microsoft Devblogs](https://devblogs.microsoft.com/commandline/systemd-support-is-now-available-in-wsl/#how-can-you-get-systemd-on-your-machine)_

5. Install Node.js packages

```
corepack enable
yarn
```

6. Install generate libs and setup rust targets & toolchains

```
make stdlib && make libs
```

> [!NOTE]
> If these commands exit with an error that says `can't cd to zstd/lib`,
> you've not cloned this repository recursively. Run `git submodule update --init` to download the submodules and run the commands above again.

7. Build binaries for Linux container-style deployment

```
# for arm64, use
make raster_runtime-container-arm64
# or for x86-64, use
make raster_runtime-container-x64
```

8. Optionally build for your local machine (Mac or Linux)

```
make release
```

You should now have a release binary for your target platform.

## Crypto and TLS Backend Options

raster_runtime supports multiple cryptographic backends for both the crypto module and TLS connections. These can be configured via Cargo features.

### Crypto Provider Features

| Feature                 | Description                                                       |
| ----------------------- | ----------------------------------------------------------------- |
| `crypto-rust` (default) | Pure Rust crypto using RustCrypto crates                          |
| `crypto-ring`           | Ring-only crypto (limited algorithm support)                      |
| `crypto-ring-rust`      | Ring for hashing/HMAC, RustCrypto for everything else             |
| `crypto-graviola`       | Graviola-only crypto (limited algorithm support)                  |
| `crypto-graviola-rust`  | Graviola for hashing/HMAC/AES-GCM, RustCrypto for everything else |
| `crypto-openssl`        | OpenSSL-based crypto                                              |

### TLS Backend Features

| Feature              | Description                                   |
| -------------------- | --------------------------------------------- |
| `tls-ring` (default) | rustls with ring crypto                       |
| `tls-graviola`       | rustls with graviola crypto                   |
| `tls-openssl`        | OpenSSL for TLS                               |

### Building with Different Backends

```bash
# Default (crypto-rust + tls-ring)
cargo build --release

# Using OpenSSL for both crypto and TLS
cargo build --release --no-default-features --features "macro,crypto-openssl,tls-openssl"

# Using Graviola for both crypto and TLS
cargo build --release --no-default-features --features "macro,crypto-graviola-rust,tls-graviola"
```

## Environment Variables

### `RASTER_RUNTIME_ASYNC_HOOKS=value`

When using asynchronous hooks, the hooking function inside QuickJS is activated. This is disabled by default as there is concern that it may have a significant impact on performance.

By setting this environment variable to `1`, the asynchronous hook function can be enabled, allowing you to track asynchronous processing using the `async_hooks` module.

### `RASTER_RUNTIME_EXTRA_CA_CERTS=file`

Load extra certificate authorities from a PEM encoded file

### `RASTER_RUNTIME_GC_THRESHOLD_MB=value`

Set a memory threshold in MB for garbage collection. Default threshold is 20MB

### `RASTER_RUNTIME_HTTP_VERSION=value`

Extends the HTTP request version. By default, only HTTP/1.1 is enabled. Specifying '2' will enable HTTP/1.1 and HTTP/2.

### `RASTER_RUNTIME_LOG=[target][=][level][,...]`

Filter the log output by target module, level, or both (using `=`). Log levels are case-insensitive and will also enable any higher priority logs.

Log levels in descending priority order:

- `Error`
- `Warn | Warning`
- `Info`
- `Debug`
- `Trace`

Example filters:

- `warn` will enable all warning and error logs
- `raster_runtime_core::vm=trace` will enable all logs in the `raster_runtime_core::vm` module
- `warn,raster_runtime_core::vm=trace` will enable all logs in the `raster_runtime_core::vm` module and all warning and error logs in other modules

### `RASTER_RUNTIME_NET_ALLOW="host[ ...]"`

Space-delimited list of hosts or socket paths which should be allowed for network connections. Network connections will be denied for any host or socket path missing from this list. Set an empty list to deny all connections

### `RASTER_RUNTIME_NET_DENY="host[ ...]"`

Space-delimited list of hosts or socket paths which should be denied for network connections

### `RASTER_RUNTIME_NET_POOL_IDLE_TIMEOUT=value`

Set a timeout in seconds for idle sockets being kept-alive. Default timeout is 15 seconds

### `RASTER_RUNTIME_PLATFORM=value`

Used to explicitly specify a preferred platform for the Node.js package resolver. The default is `browser`. If `node` is specified, "node" takes precedence in the search path. If a value other than `browser` or `node` is specified, it will behave as if "browser" was specified.

### `RASTER_RUNTIME_REGISTER_HOOKS=file`

If you want to enable a hooking mechanism that is mostly compatible with Node.js's `module.registerHooks()`, specify the js file name in this environment variable.

We provide a concrete example in `example/register-hooks`. Hook files can also be specified using the `--import` option.

### `RASTER_RUNTIME_TLS_VERSION=value`

Set the TLS version to be used for network connections. By default only TLS 1.2 is enabled. TLS 1.3 can also be enabled by setting this variable to `1.3`

## Security

Please report security issues privately to Ray-D-Song at
<songraysmail@gmail.com>. See
[CONTRIBUTING](CONTRIBUTING.md#security-issue-notifications) for more
information.

## License

This library is licensed under the Apache-2.0 License. See the [LICENSE](LICENSE) file.
