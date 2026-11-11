// Buffer instance and static methods ported from refs/node/lib/buffer.js and
// refs/node/lib/internal/buffer.js. The byte-level work (compare, search,
// fill, slice, write, swap) is done by `natives`; argument handling and
// error behavior follow Node.
(function (natives, Buffer) {
  "use strict";

  const kMaxLength = 2 ** 53 - 1;
  const proto = Buffer.prototype;
  const TypedArrayProto = Object.getPrototypeOf(Uint8Array.prototype);
  const typedArrayTag = Object.getOwnPropertyDescriptor(TypedArrayProto, Symbol.toStringTag).get;
  const isUint8Array = (v) => typedArrayTag.call(v) === "Uint8Array";
  const isArrayBufferView = (v) => ArrayBuffer.isView(v);
  const isNumber = (v) => typeof v === "number";

  // ---- errors (subset of lib/internal/errors.js) ----

  function makeError(Base, code, message) {
    const error = new Base(message);
    Object.defineProperty(error, "code", {
      value: code,
      enumerable: true,
      configurable: true,
      writable: true,
    });
    return error;
  }

  const inspect = (value, options) => globalThis.require("util").inspect(value, options);

  function determineSpecificType(value) {
    if (value === null) return "null";
    if (value === undefined) return "undefined";
    const type = typeof value;
    switch (type) {
      case "bigint":
        return `type bigint (${value}n)`;
      case "number":
        if (value === 0) return 1 / value === -Infinity ? "type number (-0)" : "type number (0)";
        if (value !== value) return "type number (NaN)";
        if (value === Infinity) return "type number (Infinity)";
        if (value === -Infinity) return "type number (-Infinity)";
        return `type number (${value})`;
      case "boolean":
        return value ? "type boolean (true)" : "type boolean (false)";
      case "symbol":
        return `type symbol (${String(value)})`;
      case "function":
        return `function ${value.name}`;
      case "object":
        if (value.constructor && "name" in value.constructor) {
          return `an instance of ${value.constructor.name}`;
        }
        return inspect(value, { depth: -1 });
      case "string":
        value.length > 28 && (value = `${value.slice(0, 25)}...`);
        if (value.indexOf("'") === -1) return `type string ('${value}')`;
        return `type string (${JSON.stringify(value)})`;
      default:
        value = inspect(value, { colors: false });
        if (value.length > 28) value = `${value.slice(0, 25)}...`;
        return `type ${type} (${value})`;
    }
  }

  const kTypes = ["string", "function", "number", "object", "Function", "Object", "boolean", "bigint", "symbol"];
  const classRegExp = /^([A-Z][a-z0-9]*)+$/;

  function formatList(array, type = "and") {
    return array.length < 3
      ? array.join(` ${type} `)
      : `${array.slice(0, -1).join(", ")}, ${type} ${array[array.length - 1]}`;
  }

  function invalidArgType(name, expected, actual) {
    if (!Array.isArray(expected)) expected = [expected];
    let msg = "The ";
    if (name.endsWith(" argument")) {
      msg += `${name} `;
    } else {
      msg += `"${name}" ${name.includes(".") ? "property" : "argument"} `;
    }
    msg += "must be ";
    const types = [];
    const instances = [];
    const other = [];
    for (const value of expected) {
      if (kTypes.includes(value)) types.push(value.toLowerCase());
      else if (classRegExp.exec(value) !== null) instances.push(value);
      else other.push(value);
    }
    if (instances.length > 0) {
      const pos = types.indexOf("object");
      if (pos !== -1) {
        types.splice(pos, 1);
        instances.push("Object");
      }
    }
    if (types.length > 0) {
      msg += `${types.length > 1 ? "one of type" : "of type"} ${formatList(types, "or")}`;
      if (instances.length > 0 || other.length > 0) msg += " or ";
    }
    if (instances.length > 0) {
      msg += `an instance of ${formatList(instances, "or")}`;
      if (other.length > 0) msg += " or ";
    }
    if (other.length > 0) {
      if (other.length > 1) {
        msg += `one of ${formatList(other, "or")}`;
      } else {
        if (other[0].toLowerCase() !== other[0]) msg += "an ";
        msg += `${other[0]}`;
      }
    }
    msg += `. Received ${determineSpecificType(actual)}`;
    return makeError(TypeError, "ERR_INVALID_ARG_TYPE", msg);
  }

  function addNumericalSeparator(val) {
    let res = "";
    let i = val.length;
    const start = val[0] === "-" ? 1 : 0;
    for (; i >= start + 4; i -= 3) res = `_${val.slice(i - 3, i)}${res}`;
    return `${val.slice(0, i)}${res}`;
  }

  function outOfRange(str, range, input) {
    let received;
    if (Number.isInteger(input) && Math.abs(input) > 2 ** 32) {
      received = addNumericalSeparator(String(input));
    } else if (typeof input === "bigint") {
      received = String(input);
      if (input > 2n ** 32n || input < -(2n ** 32n)) received = addNumericalSeparator(received);
      received += "n";
    } else {
      received = inspect(input);
    }
    return makeError(
      RangeError,
      "ERR_OUT_OF_RANGE",
      `The value of "${str}" is out of range. It must be ${range}. Received ${received}`
    );
  }

  function bufferOutOfBounds(name) {
    return makeError(
      RangeError,
      "ERR_BUFFER_OUT_OF_BOUNDS",
      name ? `"${name}" is outside of buffer bounds` : "Attempt to access memory outside buffer bounds"
    );
  }

  function unknownEncoding(encoding) {
    return makeError(TypeError, "ERR_UNKNOWN_ENCODING", `Unknown encoding: ${encoding}`);
  }

  function invalidArgValue(name, value, reason = "is invalid") {
    let inspected = inspect(value);
    if (inspected.length > 128) inspected = `${inspected.slice(0, 128)}...`;
    const type = name.includes(".") ? "property" : "argument";
    return makeError(TypeError, "ERR_INVALID_ARG_VALUE", `The ${type} '${name}' ${reason}. Received ${inspected}`);
  }

  function invalidBufferSize(bits) {
    return makeError(RangeError, "ERR_INVALID_BUFFER_SIZE", `Buffer size must be a multiple of ${bits}`);
  }

  // ---- validators (lib/internal/validators.js) ----

  function validateNumber(value, name, min = undefined, max) {
    if (typeof value !== "number") throw invalidArgType(name, "number", value);
    if (
      (min != null && value < min) ||
      (max != null && value > max) ||
      ((min != null || max != null) && Number.isNaN(value))
    ) {
      throw outOfRange(
        name,
        `${min != null ? `>= ${min}` : ""}${min != null && max != null ? " && " : ""}${max != null ? `<= ${max}` : ""}`,
        value
      );
    }
  }

  function validateInteger(value, name, min = Number.MIN_SAFE_INTEGER, max = Number.MAX_SAFE_INTEGER) {
    if (typeof value !== "number") throw invalidArgType(name, "number", value);
    if (!Number.isInteger(value)) throw outOfRange(name, "an integer", value);
    if (value < min || value > max) throw outOfRange(name, `>= ${min} && <= ${max}`, value);
  }

  const validateOffset = (value, name, min = 0, max = kMaxLength) => validateInteger(value, name, min, max);

  // ---- fill ----

  function normalizeEncoding(enc) {
    if (enc == null || enc === "utf8" || enc === "utf-8") return "utf8";
    return slowCases(enc);
  }

  function slowCases(enc) {
    switch (enc.length) {
      case 4:
        if (enc === "UTF8") return "utf8";
        if (enc === "ucs2" || enc === "UCS2") return "utf16le";
        enc = `${enc}`.toLowerCase();
        if (enc === "utf8") return "utf8";
        if (enc === "ucs2") return "utf16le";
        break;
      case 3:
        if (enc === "hex" || enc === "HEX" || `${enc}`.toLowerCase() === "hex") return "hex";
        break;
      case 5:
        if (enc === "ascii") return "ascii";
        if (enc === "ucs-2") return "utf16le";
        if (enc === "UTF-8") return "utf8";
        if (enc === "ASCII") return "ascii";
        if (enc === "UCS-2") return "utf16le";
        enc = `${enc}`.toLowerCase();
        if (enc === "utf-8") return "utf8";
        if (enc === "ascii") return "ascii";
        if (enc === "ucs-2") return "utf16le";
        break;
      case 6:
        if (enc === "base64") return "base64";
        if (enc === "latin1" || enc === "binary") return "latin1";
        if (enc === "BASE64") return "base64";
        if (enc === "LATIN1" || enc === "BINARY") return "latin1";
        enc = `${enc}`.toLowerCase();
        if (enc === "base64") return "base64";
        if (enc === "latin1" || enc === "binary") return "latin1";
        break;
      case 7:
        if (enc === "utf16le" || enc === "UTF16LE" || `${enc}`.toLowerCase() === "utf16le") return "utf16le";
        break;
      case 8:
        if (enc === "utf-16le" || enc === "UTF-16LE" || `${enc}`.toLowerCase() === "utf-16le") return "utf16le";
        break;
      case 9:
        if (enc === "base64url" || enc === "BASE64URL" || `${enc}`.toLowerCase() === "base64url") return "base64url";
        break;
      default:
        if (enc === "") return "utf8";
    }
    return undefined;
  }

  function validateString(value, name) {
    if (typeof value !== "string") throw invalidArgType(name, "string", value);
  }

  function _fill(buf, value, offset, end, encoding) {
    if (typeof value === "string") {
      if (offset === undefined || typeof offset === "string") {
        encoding = offset;
        offset = 0;
        end = buf.length;
      } else if (typeof end === "string") {
        encoding = end;
        end = buf.length;
      }

      const normalizedEncoding = normalizeEncoding(encoding);
      if (normalizedEncoding === undefined) {
        validateString(encoding, "encoding");
        throw unknownEncoding(encoding);
      }

      if (value.length === 0) {
        value = 0;
      } else if (value.length === 1) {
        if (normalizedEncoding === "utf8") {
          const code = value.charCodeAt(0);
          if (code < 128) value = code;
        } else if (normalizedEncoding === "latin1") {
          value = value.charCodeAt(0);
        }
      }
    } else {
      encoding = undefined;
    }

    if (offset === undefined) {
      offset = 0;
      end = buf.length;
    } else {
      validateOffset(offset, "offset");
      if (end === undefined) {
        end = buf.length;
      } else {
        validateOffset(end, "end", 0, buf.length);
      }
      if (offset >= end) return buf;
    }

    if (typeof value === "number") {
      const byteLen = buf.byteLength;
      const fillLength = end - offset;
      if (offset > end || fillLength + offset > byteLen) throw bufferOutOfBounds();
      TypedArrayProto.fill.call(buf, value, offset, end);
    } else {
      let source = value;
      if (typeof value !== "string" && !isArrayBufferView(value)) {
        source = (+value >>> 0) & 255;
        TypedArrayProto.fill.call(buf, source, offset, end);
        return buf;
      }
      const res = natives.fill(buf, source, offset, end, encoding);
      if (res < 0) {
        if (res === -1) throw invalidArgValue("value", value);
        throw bufferOutOfBounds();
      }
    }

    return buf;
  }

  proto.fill = function fill(value, offset, end, encoding) {
    return _fill(this, value, offset, end, encoding);
  };

  // ---- compare / equals ----

  function compare(buf1, buf2) {
    if (!isUint8Array(buf1)) throw invalidArgType("buf1", ["Buffer", "Uint8Array"], buf1);
    if (!isUint8Array(buf2)) throw invalidArgType("buf2", ["Buffer", "Uint8Array"], buf2);
    if (buf1 === buf2) return 0;
    return natives.compare(buf1, buf2);
  }
  Buffer.compare = compare;

  proto.equals = function equals(otherBuffer) {
    if (!isUint8Array(otherBuffer)) {
      throw invalidArgType("otherBuffer", ["Buffer", "Uint8Array"], otherBuffer);
    }
    if (this === otherBuffer) return true;
    const len = this.byteLength;
    if (len !== otherBuffer.byteLength) return false;
    return len === 0 || natives.compare(this, otherBuffer) === 0;
  };

  proto.compare = function compare(target, targetStart, targetEnd, sourceStart, sourceEnd) {
    if (!isUint8Array(target)) throw invalidArgType("target", ["Buffer", "Uint8Array"], target);
    if (arguments.length === 1) return natives.compare(this, target);

    if (targetStart === undefined) targetStart = 0;
    else validateOffset(targetStart, "targetStart");

    if (targetEnd === undefined) targetEnd = target.length;
    else validateOffset(targetEnd, "targetEnd", 0, target.length);

    if (sourceStart === undefined) sourceStart = 0;
    else validateOffset(sourceStart, "sourceStart");

    if (sourceEnd === undefined) sourceEnd = this.length;
    else validateOffset(sourceEnd, "sourceEnd", 0, this.length);

    if (sourceStart >= sourceEnd) return targetStart >= targetEnd ? 0 : -1;
    if (targetStart >= targetEnd) return 1;

    return natives.compareOffset(this, target, targetStart, sourceStart, targetEnd, sourceEnd);
  };
});
