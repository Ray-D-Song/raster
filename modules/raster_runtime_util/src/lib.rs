// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
pub mod text_decoder;
pub mod text_encoder;

use raster_runtime_logging::format_plain;
use raster_runtime_logging::format_values;
use raster_runtime_utils::{
    class::CUSTOM_INSPECT_SYMBOL_DESCRIPTION,
    module::{export_default, ModuleInfo},
};
use rquickjs::{
    function::Func,
    module::{Declarations, Exports, ModuleDef},
    prelude::Rest,
    ArrayBuffer, Class, Ctx, Function, Object, Result, Symbol, Value,
};
use text_decoder::TextDecoder;
use text_encoder::TextEncoder;

fn inherits<'js>(ctor: Function<'js>, super_ctor: Function<'js>) -> Result<()> {
    let super_proto: Object<'js> = super_ctor.get("prototype")?;
    let proto: Object<'js> = ctor.get("prototype")?;
    proto.set_prototype(Some(&super_proto))?;
    ctor.set("super_", super_ctor)?;
    Ok(())
}

fn create_promisify<'js>(ctx: &Ctx<'js>) -> Result<Function<'js>> {
    ctx.eval(
        r#"(function () {
  const kCustomPromisifiedSymbol = Symbol.for("nodejs.util.promisify.custom");

  function promisify(original) {
    if (typeof original !== "function") {
      throw new TypeError('The "original" argument must be of type function');
    }

    const custom = original[kCustomPromisifiedSymbol];
    if (custom !== undefined) {
      if (typeof custom !== "function") {
        throw new TypeError('The "util.promisify.custom" argument must be of type function');
      }
      return custom;
    }

    return function (...args) {
      return new Promise((resolve, reject) => {
        original.call(this, ...args, (error, value) => {
          if (error) reject(error);
          else resolve(value);
        });
      });
    };
  }
  promisify.custom = kCustomPromisifiedSymbol;
  return promisify;
})()"#,
    )
}

fn create_to_usv_string<'js>(ctx: &Ctx<'js>) -> Result<Function<'js>> {
    ctx.eval(
        r#"(function () {
  return function toUSVString(value) {
    const string = String(value);
    let result = "";
    for (let i = 0; i < string.length; i++) {
      const codeUnit = string.charCodeAt(i);
      if (codeUnit >= 0xd800 && codeUnit <= 0xdbff) {
        if (i + 1 < string.length) {
          const next = string.charCodeAt(i + 1);
          if (next >= 0xdc00 && next <= 0xdfff) {
            result += string[i] + string[i + 1];
            i += 1;
            continue;
          }
        }
        result += "\uFFFD";
        continue;
      }
      if (codeUnit >= 0xdc00 && codeUnit <= 0xdfff) {
        result += "\uFFFD";
        continue;
      }
      result += string[i];
    }
    return result;
  };
})()"#,
    )
}

fn create_debuglog<'js>(ctx: &Ctx<'js>) -> Result<Function<'js>> {
    ctx.eval(
        r#"(function () {
  const cache = new Map();

  function matchesPattern(set, pattern) {
    if (pattern === set) return true;
    if (pattern.endsWith("*")) {
      return set.startsWith(pattern.slice(0, -1));
    }
    return false;
  }

  function isEnabled(set) {
    const env = (globalThis.process && process.env && process.env.NODE_DEBUG) || "";
    for (const part of env.split(/[,\s]+/)) {
      if (!part) continue;
      if (matchesPattern(set, part.toUpperCase())) {
        return true;
      }
    }
    return false;
  }

  return function debuglog(set) {
    set = String(set).toUpperCase();
    if (cache.has(set)) {
      return cache.get(set);
    }

    const fn = function (...args) {
      if (!fn.enabled) return;
      const prefix = set + " " + (globalThis.process && process.pid != null ? process.pid : "") + " ";
      console.error(prefix + require("util").format(...args));
    };
    Object.defineProperty(fn, "enabled", {
      configurable: true,
      enumerable: true,
      get() {
        return isEnabled(set);
      },
    });
    cache.set(set, fn);
    return fn;
  };
})()"#,
    )
}

/// Shared `util.types` / `util/types` object, cached once per isolate via a
/// non-enumerable `Symbol.for` key so all entry points return the same object.
fn get_or_create_types_object<'js>(ctx: &Ctx<'js>) -> Result<Object<'js>> {
    install_type_predicates(ctx)?;
    ctx.eval(
        r#"(function () {
  const KEY = Symbol.for("nodejs.util.types");
  if (Object.prototype.hasOwnProperty.call(globalThis, KEY) && globalThis[KEY]) {
    return globalThis[KEY];
  }

  const types = {
    isProxy(value) {
      return typeof value === "object" && value !== null && globalThis.__rasterIsProxy
        ? globalThis.__rasterIsProxy(value)
        : false;
    },
    isPromise(value) {
      return typeof value === "object" && value !== null && globalThis.__rasterIsPromise
        ? globalThis.__rasterIsPromise(value)
        : false;
    },
    isArrayBuffer(value) {
      return typeof value === "object" && value !== null && globalThis.__rasterIsArrayBuffer
        ? globalThis.__rasterIsArrayBuffer(value)
        : false;
    },
    isAnyArrayBuffer(value) {
      return this.isArrayBuffer(value);
    },
    isSharedArrayBuffer() {
      return false;
    },
    isTypedArray(value) {
      return ArrayBuffer.isView(value) && !(value instanceof DataView);
    },
    isDataView(value) {
      return Object.prototype.toString.call(value) === "[object DataView]";
    },
    isUint8Array(value) {
      return Object.prototype.toString.call(value) === "[object Uint8Array]";
    },
    // Date brand check: works across realms, rejects plain objects and
    // Symbol.toStringTag forgeries (unlike Object.prototype.toString alone).
    isDate(value) {
      if (typeof value !== "object" || value === null) {
        return false;
      }
      try {
        Date.prototype.getTime.call(value);
        return true;
      } catch {
        return false;
      }
    },
  };

  Object.defineProperty(globalThis, KEY, {
    value: types,
    enumerable: false,
    configurable: true,
    writable: true,
  });
  return types;
})()"#,
    )
}

fn create_deprecate<'js>(ctx: &Ctx<'js>) -> Result<Function<'js>> {
    // Phase 1: identity wrapper with Node-compatible TypeError on bad args.
    // Runtime deprecation warnings are intentionally not emitted yet.
    ctx.eval(
        r#"(function () {
  return function deprecate(fn, message, code) {
    if (typeof fn !== "function") {
      throw new TypeError('The "fn" argument must be of type function. Received type ' + typeof fn);
    }
    // Keep this, arguments, return value, and function identity unchanged.
    return fn;
  };
})()"#,
    )
}

fn install_type_predicates(ctx: &Ctx<'_>) -> Result<()> {
    let globals = ctx.globals();
    if globals.contains_key("__rasterIsProxy")? {
        return Ok(());
    }

    globals.set(
        "__rasterIsProxy",
        Func::from(|value: Value| value.is_proxy()),
    )?;
    globals.set(
        "__rasterIsPromise",
        Func::from(|value: Value| value.is_promise()),
    )?;
    globals.set(
        "__rasterIsArrayBuffer",
        Func::from(|value: Value| ArrayBuffer::from_value(value).is_some()),
    )?;
    Ok(())
}

fn inspect_value<'js>(ctx: Ctx<'js>, value: Value<'js>) -> Result<String> {
    format_plain(ctx, false, rquickjs::prelude::Rest(vec![value]))
}

fn format_with_options<'js>(
    ctx: Ctx<'js>,
    options: Object<'js>,
    args: rquickjs::prelude::Rest<Value<'js>>,
) -> Result<String> {
    let colors = options
        .get::<_, Option<bool>>("colors")
        .ok()
        .flatten()
        .unwrap_or(false);
    format_values(&ctx, args, colors, false)
}

fn format_export<'js>(ctx: Ctx<'js>, args: Rest<Value<'js>>) -> Result<String> {
    let mut formatted = format_plain(ctx, true, args)?;
    if formatted.ends_with('\n') {
        formatted.pop();
    }
    if formatted.ends_with('\r') {
        formatted.pop();
    }
    Ok(formatted)
}

pub fn define_text_encoding_constructors(ctx: &Ctx<'_>) -> Result<()> {
    let globals = ctx.globals();
    if globals.contains_key("TextEncoder")? && globals.contains_key("TextDecoder")? {
        return Ok(());
    }

    Class::<TextEncoder>::define(&globals)?;
    Class::<TextDecoder>::define(&globals)?;
    Ok(())
}

pub struct UtilModule;

impl ModuleDef for UtilModule {
    fn declare(declare: &Declarations) -> Result<()> {
        declare.declare(stringify!(TextDecoder))?;
        declare.declare(stringify!(TextEncoder))?;
        declare.declare("format")?;
        declare.declare("inherits")?;
        declare.declare("promisify")?;
        declare.declare("inspect")?;
        declare.declare("formatWithOptions")?;
        declare.declare("debuglog")?;
        declare.declare("debug")?;
        declare.declare("toUSVString")?;
        declare.declare("types")?;
        declare.declare("deprecate")?;
        declare.declare("default")?;
        Ok(())
    }

    fn evaluate<'js>(ctx: &Ctx<'js>, exports: &Exports<'js>) -> Result<()> {
        export_default(ctx, exports, |default| {
            install_type_predicates(ctx)?;
            let globals = ctx.globals();

            let encoder: Function = globals.get(stringify!(TextEncoder))?;
            let decoder: Function = globals.get(stringify!(TextDecoder))?;
            let promisify = create_promisify(ctx)?;
            let to_usv_string = create_to_usv_string(ctx)?;
            let debuglog = create_debuglog(ctx)?;
            let types = get_or_create_types_object(ctx)?;
            let deprecate = create_deprecate(ctx)?;

            default.set(stringify!(TextEncoder), encoder)?;
            default.set(stringify!(TextDecoder), decoder)?;
            default.set("format", Func::from(format_export))?;
            default.set("inherits", Func::from(inherits))?;
            default.set("promisify", promisify)?;
            default.set("inspect", Func::from(inspect_value))?;
            default.set("formatWithOptions", Func::from(format_with_options))?;
            default.set("debuglog", debuglog.clone())?;
            default.set("debug", debuglog)?;
            default.set("toUSVString", to_usv_string)?;
            default.set("types", types)?;
            default.set("deprecate", deprecate)?;
            let inspect_symbol =
                Symbol::new_global(ctx.clone(), CUSTOM_INSPECT_SYMBOL_DESCRIPTION)?;
            let inspect_value: Value = default.get("inspect")?;
            inspect_value
                .as_object()
                .expect("inspect export")
                .set("custom", inspect_symbol)?;

            Ok(())
        })
    }
}

impl From<UtilModule> for ModuleInfo<UtilModule> {
    fn from(val: UtilModule) -> Self {
        ModuleInfo {
            name: "util",
            module: val,
        }
    }
}

/// `require("util/types")` / `require("node:util/types")` — same object as `util.types`.
pub struct UtilTypesModule;

impl ModuleDef for UtilTypesModule {
    fn declare(declare: &Declarations) -> Result<()> {
        declare.declare("isProxy")?;
        declare.declare("isPromise")?;
        declare.declare("isArrayBuffer")?;
        declare.declare("isAnyArrayBuffer")?;
        declare.declare("isSharedArrayBuffer")?;
        declare.declare("isTypedArray")?;
        declare.declare("isDataView")?;
        declare.declare("isUint8Array")?;
        declare.declare("isDate")?;
        declare.declare("default")?;
        Ok(())
    }

    fn evaluate<'js>(ctx: &Ctx<'js>, exports: &Exports<'js>) -> Result<()> {
        // Export the shared types object as `default` so CJS `require("util/types")`
        // returns the same object as `require("util").types` (builtin interop merges
        // named exports onto the default object when it is an object).
        let types = get_or_create_types_object(ctx)?;
        for key in types.keys::<String>() {
            let key = key?;
            let value: Value = types.get(&key)?;
            exports.export(key.as_str(), value)?;
        }
        exports.export("default", types)?;
        Ok(())
    }
}

impl From<UtilTypesModule> for ModuleInfo<UtilTypesModule> {
    fn from(val: UtilTypesModule) -> Self {
        ModuleInfo {
            name: "util/types",
            module: val,
        }
    }
}

pub fn init(ctx: &Ctx<'_>) -> Result<()> {
    define_text_encoding_constructors(ctx)
}
