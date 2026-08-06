// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use raster_runtime_utils::module::ModuleInfo;
use rquickjs::{
    module::{Declarations, Exports, ModuleDef},
    prelude::Opt,
    Ctx, Exception, Function, Result, Type, Value,
};

fn throw_assertion(ctx: &Ctx, message: Opt<Value>, default: &str) -> Result<()> {
    if let Some(obj) = message.0 {
        match obj.type_of() {
            Type::String => {
                let msg = obj.as_string().unwrap().to_string().unwrap();
                return Err(Exception::throw_message(ctx, &msg));
            },
            Type::Exception => return Err(obj.as_exception().cloned().unwrap().throw()),
            _ => {},
        }
    }
    Err(Exception::throw_message(ctx, default))
}

fn ok(ctx: Ctx, value: Value, message: Opt<Value>) -> Result<()> {
    match value.type_of() {
        Type::Bool if value.as_bool().unwrap() => {
            return Ok(());
        },
        Type::Float | Type::Int if value.as_number().unwrap() != 0.0 => {
            return Ok(());
        },
        Type::String if !value.as_string().unwrap().to_string().unwrap().is_empty() => {
            return Ok(());
        },
        Type::Array
        | Type::BigInt
        | Type::Constructor
        | Type::Exception
        | Type::Function
        | Type::Proxy
        | Type::Symbol
        | Type::Object => {
            return Ok(());
        },
        _ => {},
    }

    throw_assertion(
        &ctx,
        message,
        "AssertionError: The expression was evaluated to a falsy value",
    )
}

fn values_strictly_equal<'a>(actual: &Value<'a>, expected: &Value<'a>) -> Result<bool> {
    if actual.is_undefined() && expected.is_undefined() {
        return Ok(true);
    }
    if actual.is_null() && expected.is_null() {
        return Ok(true);
    }
    if actual.is_bool() && expected.is_bool() {
        return Ok(actual.as_bool() == expected.as_bool());
    }
    if actual.is_number() && expected.is_number() {
        return Ok(actual.as_number().unwrap().to_bits() == expected.as_number().unwrap().to_bits());
    }
    if actual.is_string() && expected.is_string() {
        let a = actual.as_string().unwrap().to_string()?;
        let b = expected.as_string().unwrap().to_string()?;
        return Ok(a == b);
    }
    if actual.type_of() == Type::BigInt && expected.type_of() == Type::BigInt {
        return Ok(actual == expected);
    }
    if actual.is_symbol() && expected.is_symbol() {
        return Ok(actual == expected);
    }
    Ok(actual == expected)
}

fn equal<'js>(
    ctx: Ctx<'js>,
    actual: Value<'js>,
    expected: Value<'js>,
    message: Opt<Value<'js>>,
) -> Result<()> {
    if values_strictly_equal(&actual, &expected)? {
        return Ok(());
    }
    throw_assertion(
        &ctx,
        message,
        "AssertionError: Expected values to be strictly equal",
    )
}

fn make_strict_assert<'js>(ctx: &Ctx<'js>) -> Result<Function<'js>> {
    let strict = Function::new(ctx.clone(), ok)?.with_name("ok")?;
    let equal_fn = Function::new(ctx.clone(), equal)?.with_name("equal")?;
    strict.set("ok", strict.clone())?;
    strict.set("equal", equal_fn.clone())?;
    strict.set("strictEqual", equal_fn)?;
    Ok(strict)
}

pub struct AssertModule;

impl ModuleDef for AssertModule {
    fn declare(declare: &Declarations) -> Result<()> {
        declare.declare("ok")?;
        declare.declare("default")?;
        Ok(())
    }

    fn evaluate<'js>(ctx: &Ctx<'js>, exports: &Exports<'js>) -> Result<()> {
        let ok_function = Function::new(ctx.clone(), ok)?.with_name("ok")?;
        ok_function.set("ok", ok_function.clone())?;
        let strict = make_strict_assert(ctx)?;
        ok_function.set("strict", strict)?;

        exports.export("ok", ok_function.clone())?;
        exports.export("default", ok_function)?;
        Ok(())
    }
}

impl From<AssertModule> for ModuleInfo<AssertModule> {
    fn from(val: AssertModule) -> Self {
        ModuleInfo {
            name: "assert",
            module: val,
        }
    }
}

pub struct AssertStrictModule;

impl ModuleDef for AssertStrictModule {
    fn declare(declare: &Declarations) -> Result<()> {
        declare.declare("default")?;
        Ok(())
    }

    fn evaluate<'js>(ctx: &Ctx<'js>, exports: &Exports<'js>) -> Result<()> {
        let strict = make_strict_assert(ctx)?;
        exports.export("default", strict)?;
        Ok(())
    }
}

impl From<AssertStrictModule> for ModuleInfo<AssertStrictModule> {
    fn from(val: AssertStrictModule) -> Self {
        ModuleInfo {
            name: "assert/strict",
            module: val,
        }
    }
}
