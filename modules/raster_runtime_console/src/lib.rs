// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use std::{
    collections::HashMap,
    io::{stderr, stdout, IsTerminal, Write},
    time::Instant,
};

use raster_runtime_logging::{build_formatted_string, FormatOptions, NEWLINE};
use raster_runtime_utils::module::{export_default, ModuleInfo};
use rquickjs::{
    function::Opt,
    module::{Declarations, Exports, ModuleDef},
    prelude::Rest,
    Class, Coerced, Ctx, Function, IntoJs, Object, Result, Value,
};

#[derive(rquickjs::class::Trace, rquickjs::JsLifetime)]
#[rquickjs::class]
pub struct Console {
    #[qjs(skip_trace)]
    counts: HashMap<String, u64>,
    #[qjs(skip_trace)]
    timers: HashMap<String, Instant>,
}

impl Default for Console {
    fn default() -> Self {
        Self::new()
    }
}

impl Console {
    fn label_string(_ctx: &Ctx<'_>, label: Opt<Value<'_>>) -> Result<String> {
        match label.0 {
            None => Ok("default".to_string()),
            Some(value) if value.is_undefined() => Ok("default".to_string()),
            Some(value) => Ok(value.get::<Coerced<String>>()?.0),
        }
    }
}

#[rquickjs::methods(rename_all = "camelCase")]
impl Console {
    #[qjs(constructor)]
    pub fn new() -> Self {
        Self {
            counts: HashMap::new(),
            timers: HashMap::new(),
        }
    }

    pub fn log<'js>(&self, ctx: Ctx<'js>, args: Rest<Value<'js>>) -> Result<()> {
        write_console_log(stdout(), &ctx, args, false)
    }
    pub fn clear(&self) {
        clear()
    }
    pub fn debug<'js>(&self, ctx: Ctx<'js>, args: Rest<Value<'js>>) -> Result<()> {
        write_console_log(stdout(), &ctx, args, false)
    }
    pub fn info<'js>(&self, ctx: Ctx<'js>, args: Rest<Value<'js>>) -> Result<()> {
        write_console_log(stdout(), &ctx, args, false)
    }
    pub fn trace<'js>(&self, ctx: Ctx<'js>, args: Rest<Value<'js>>) -> Result<()> {
        write_console_log(stdout(), &ctx, args, false)
    }
    pub fn error<'js>(&self, ctx: Ctx<'js>, args: Rest<Value<'js>>) -> Result<()> {
        write_console_log(stderr(), &ctx, args, false)
    }
    pub fn warn<'js>(&self, ctx: Ctx<'js>, args: Rest<Value<'js>>) -> Result<()> {
        write_console_log(stderr(), &ctx, args, false)
    }
    pub fn assert<'js>(
        &self,
        ctx: Ctx<'js>,
        expression: bool,
        args: Rest<Value<'js>>,
    ) -> Result<()> {
        if !expression {
            write_console_log(stderr(), &ctx, args, false)?;
        }
        Ok(())
    }

    pub fn count<'js>(&mut self, ctx: Ctx<'js>, label: Opt<Value<'js>>) -> Result<()> {
        let label = Self::label_string(&ctx, label)?;
        let count = self
            .counts
            .entry(label.clone())
            .and_modify(|c| *c += 1)
            .or_insert(1);
        write_console_log(
            stdout(),
            &ctx,
            Rest(vec![format!("{label}: {count}").into_js(&ctx)?]),
            false,
        )
    }

    pub fn count_reset<'js>(&mut self, ctx: Ctx<'js>, label: Opt<Value<'js>>) -> Result<()> {
        let label = Self::label_string(&ctx, label)?;
        self.counts.remove(&label);
        Ok(())
    }

    pub fn time<'js>(&mut self, ctx: Ctx<'js>, label: Opt<Value<'js>>) -> Result<()> {
        let label = Self::label_string(&ctx, label)?;
        self.timers.entry(label).or_insert_with(Instant::now);
        Ok(())
    }

    pub fn time_log<'js>(
        &mut self,
        ctx: Ctx<'js>,
        label: Opt<Value<'js>>,
        args: Rest<Value<'js>>,
    ) -> Result<()> {
        let label = Self::label_string(&ctx, label)?;
        if let Some(start) = self.timers.get(&label) {
            let elapsed = start.elapsed().as_millis();
            let mut values = vec![format!("{label}: {elapsed}ms").into_js(&ctx)?];
            values.extend(args.0);
            write_console_log(stdout(), &ctx, Rest(values), false)?;
        }
        Ok(())
    }

    pub fn time_end<'js>(&mut self, ctx: Ctx<'js>, label: Opt<Value<'js>>) -> Result<()> {
        let label = Self::label_string(&ctx, label)?;
        if let Some(start) = self.timers.remove(&label) {
            let elapsed = start.elapsed().as_millis();
            write_console_log(
                stdout(),
                &ctx,
                Rest(vec![format!("{label}: {elapsed}ms").into_js(&ctx)?]),
                false,
            )?;
        }
        Ok(())
    }

    pub fn dir<'js>(
        &self,
        ctx: Ctx<'js>,
        value: Value<'js>,
        options: Opt<Object<'js>>,
    ) -> Result<()> {
        let mut colors = stdout().is_terminal();
        if let Some(options) = options.0 {
            if let Ok(Some(enabled)) = options.get::<_, Option<bool>>("colors") {
                colors = enabled;
            }
        }
        write_console_log(stdout(), &ctx, Rest(vec![value]), colors)
    }
}

pub fn log_fatal<'js>(ctx: Ctx<'js>, args: Rest<Value<'js>>) -> Result<()> {
    write_console_log(stderr(), &ctx, args, false)
}

pub fn log_error<'js>(ctx: Ctx<'js>, args: Rest<Value<'js>>) -> Result<()> {
    write_console_log(stderr(), &ctx, args, false)
}

fn write_console_log<'js, T>(
    mut output: T,
    ctx: &Ctx<'js>,
    args: Rest<Value<'js>>,
    force_colors: bool,
) -> Result<()>
where
    T: Write + IsTerminal,
{
    let is_tty = output.is_terminal();
    let mut result = String::new();
    let mut options = FormatOptions::new(ctx, force_colors || is_tty, true)?;
    build_formatted_string(&mut result, ctx, args, &mut options)?;
    result.push(NEWLINE);
    let _ = output.write_all(result.as_bytes());
    Ok(())
}

fn clear() {
    let _ = stdout().write_all(b"\x1b[1;1H\x1b[0J");
}

pub struct ConsoleModule;

impl ModuleDef for ConsoleModule {
    fn declare(declare: &Declarations) -> Result<()> {
        declare.declare(stringify!(Console))?;
        declare.declare("default")?;
        Ok(())
    }

    fn evaluate<'js>(ctx: &Ctx<'js>, exports: &Exports<'js>) -> Result<()> {
        export_default(ctx, exports, |default| {
            let globals = ctx.globals();
            let constructor: Function = globals.get(stringify!(Console))?;
            default.set(stringify!(Console), constructor)?;
            Ok(())
        })
    }
}

impl From<ConsoleModule> for ModuleInfo<ConsoleModule> {
    fn from(val: ConsoleModule) -> Self {
        ModuleInfo {
            name: "console",
            module: val,
        }
    }
}

pub fn init(ctx: &Ctx<'_>) -> Result<()> {
    let globals = ctx.globals();

    Class::<Console>::define(&globals)?;

    ctx.eval::<(), _>(
        r#"(function () {
  const console = new Console();
  for (const name of [
    "assert",
    "clear",
    "count",
    "countReset",
    "debug",
    "dir",
    "error",
    "info",
    "log",
    "time",
    "timeEnd",
    "timeLog",
    "trace",
    "warn",
  ]) {
    console[name] = console[name].bind(console);
  }
  globalThis.console = console;
})();"#,
    )?;

    Ok(())
}
