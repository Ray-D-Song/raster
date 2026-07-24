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

fn create_types_object<'js>(ctx: &Ctx<'js>) -> Result<Object<'js>> {
    ctx.eval(
        r#"(function () {
  return {
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
            let types = create_types_object(ctx)?;

            default.set(stringify!(TextEncoder), encoder)?;
            default.set(stringify!(TextDecoder), decoder)?;
            default.set("format", Func::from(format_export))?;
            default.set("inherits", Func::from(inherits))?;
            default.set("promisify", promisify)?;
            default.set("inspect", Func::from(inspect_value))?;
            default.set(
                "formatWithOptions",
                Func::from(format_with_options),
            )?;
            default.set("debuglog", debuglog.clone())?;
            default.set("debug", debuglog)?;
            default.set("toUSVString", to_usv_string)?;
            default.set("types", types)?;
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

pub fn init(ctx: &Ctx<'_>) -> Result<()> {
    define_text_encoding_constructors(ctx)
}
