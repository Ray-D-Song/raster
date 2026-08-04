// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
mod access;
mod chmod;
mod errors;
mod file_handle;
mod mkdir;
mod open;
mod read_dir;
mod read_file;
mod read_stream;
mod realpath;
mod rename;
mod rm;
mod stats;
mod symlink;
mod watch;
mod write_file;

use raster_runtime_events::Emitter;
use raster_runtime_utils::module::{export_default, ModuleInfo};
use rquickjs::{
    module::{Declarations, Exports, ModuleDef},
    prelude::{Async, Func},
    Function,
};
use rquickjs::{Class, Ctx, Object, Result, Value};

use self::access::{access, access_callback, access_sync};
use self::chmod::{chmod, chmod_sync};
use self::file_handle::FileHandle;
use self::mkdir::{mkdir, mkdir_sync, mkdtemp, mkdtemp_sync};
use self::open::open;
use self::read_dir::{read_dir, read_dir_sync, readdir_callback, Dirent};
use self::read_file::{read_file, read_file_sync};
use self::read_stream::create_create_read_stream;
use self::realpath::{realpath, realpath_promises, realpath_sync};
use self::rename::{rename, rename_sync};
use self::rm::{rmdir, rmdir_sync, rmfile, rmfile_sync};
use self::stats::{
    lstat_callback, lstat_fn, lstat_fn_sync, stat_callback, stat_fn, stat_fn_sync, Stats,
};
use self::symlink::{symlink, symlink_sync};
use self::watch::{watch, FSWatcher};
use self::write_file::{write_file, write_file_sync};

pub const CONSTANT_F_OK: u32 = 0;
pub const CONSTANT_R_OK: u32 = 4;
pub const CONSTANT_W_OK: u32 = 2;
pub const CONSTANT_X_OK: u32 = 1;

pub struct FsPromisesModule;

impl ModuleDef for FsPromisesModule {
    fn declare(declare: &Declarations) -> Result<()> {
        declare.declare("access")?;
        declare.declare("open")?;
        declare.declare("readFile")?;
        declare.declare("writeFile")?;
        declare.declare("rename")?;
        declare.declare("readdir")?;
        declare.declare("mkdir")?;
        declare.declare("mkdtemp")?;
        declare.declare("rm")?;
        declare.declare("rmdir")?;
        declare.declare("stat")?;
        declare.declare("lstat")?;
        declare.declare("constants")?;
        declare.declare("chmod")?;
        declare.declare("symlink")?;
        declare.declare("realpath")?;

        declare.declare("default")?;

        Ok(())
    }

    fn evaluate<'js>(ctx: &Ctx<'js>, exports: &Exports<'js>) -> Result<()> {
        let globals = ctx.globals();

        Class::<Dirent>::define(&globals)?;
        Class::<FileHandle>::define(&globals)?;
        Class::<Stats>::define(&globals)?;

        export_default(ctx, exports, |default| {
            export_promises(ctx, default)?;

            Ok(())
        })
    }
}

impl From<FsPromisesModule> for ModuleInfo<FsPromisesModule> {
    fn from(val: FsPromisesModule) -> Self {
        ModuleInfo {
            name: "fs/promises",
            module: val,
        }
    }
}

pub struct FsModule;

impl ModuleDef for FsModule {
    fn declare(declare: &Declarations) -> Result<()> {
        declare.declare("promises")?;
        declare.declare("access")?;
        declare.declare("accessSync")?;
        declare.declare("open")?;
        declare.declare("close")?;
        declare.declare("mkdirSync")?;
        declare.declare("mkdtempSync")?;
        declare.declare("readdir")?;
        declare.declare("readdirSync")?;
        declare.declare("readFile")?;
        declare.declare("readFileSync")?;
        declare.declare("writeFile")?;
        declare.declare("rmdirSync")?;
        declare.declare("rmSync")?;
        declare.declare("unlinkSync")?;
        declare.declare("stat")?;
        declare.declare("statSync")?;
        declare.declare("lstat")?;
        declare.declare("lstatSync")?;
        declare.declare("writeFileSync")?;
        declare.declare("watch")?;
        declare.declare("FSWatcher")?;
        declare.declare("constants")?;
        declare.declare("chmodSync")?;
        declare.declare("renameSync")?;
        declare.declare("symlinkSync")?;
        declare.declare("realpathSync")?;
        declare.declare("realpath")?;
        declare.declare("existsSync")?;
        declare.declare("createReadStream")?;

        declare.declare("default")?;

        Ok(())
    }

    fn evaluate<'js>(ctx: &Ctx<'js>, exports: &Exports<'js>) -> Result<()> {
        let globals = ctx.globals();

        Class::<Dirent>::define(&globals)?;
        Class::<FileHandle>::define(&globals)?;
        Class::<Stats>::define(&globals)?;
        let fs_watcher_ctor = Class::<FSWatcher>::create_constructor(ctx)?
            .expect("Can't create FSWatcher constructor");
        globals.set("FSWatcher", fs_watcher_ctor.clone())?;
        FSWatcher::add_event_emitter_prototype(ctx)?;

        export_default(ctx, exports, |default| {
            let promises = Object::new(ctx.clone())?;
            export_promises(ctx, &promises)?;
            export_constants(ctx, default)?;

            default.set("promises", promises)?;

            let access_fn = Function::new(ctx.clone(), access_callback)?;
            default.set("access", access_fn)?;
            default.set("accessSync", Func::from(access_sync))?;
            default.set("mkdirSync", Func::from(mkdir_sync))?;
            default.set("mkdtempSync", Func::from(mkdtemp_sync))?;
            let readdir_fn = Function::new(ctx.clone(), readdir_callback)?;
            default.set("readdir", readdir_fn)?;
            default.set("readdirSync", Func::from(read_dir_sync))?;
            // Callback-style open/close/readFile/writeFile with real handle tracking.
            let cb_helpers: Object = ctx.eval(
                r#"(function(){
  const fsp = require("fs/promises");
  const handles = new Map();

  function validateCallback(callback) {
    if (typeof callback !== "function") {
      throw new TypeError('The "cb" argument must be of type function');
    }
  }

  function onceCallback(callback) {
    let called = false;
    return function(...args) {
      if (called) return;
      called = true;
      callback(...args);
    };
  }

  function createEBADF(fd) {
    const err = new Error("EBADF: bad file descriptor, close");
    err.code = "EBADF";
    err.errno = -9;
    err.syscall = "close";
    err.path = undefined;
    err.fd = fd;
    return err;
  }

  function open(path, flags, mode, callback) {
    if (typeof flags === "function") {
      callback = flags;
      flags = "r";
      mode = undefined;
    } else if (typeof mode === "function") {
      callback = mode;
      mode = undefined;
    }
    validateCallback(callback);
    const cb = onceCallback(callback);
    fsp.open(path, flags, mode).then(
      (handle) => {
        // Use the real OS fd from FileHandle.fd (sync getter). Registry keeps
        // the handle alive so the fd is not closed until fs.close(fd).
        const fd = handle.fd;
        if (typeof fd !== "number" || !Number.isFinite(fd)) {
          const err = new Error("internal error: FileHandle.fd is not a number");
          handle.close().then(() => cb(err), () => cb(err));
          return;
        }
        if (handles.has(fd)) {
          // Collision on a live registry entry is an internal error — never
          // allocate synthetic fds that could mask the real descriptor.
          const err = new Error("internal error: fd collision in fs.open registry");
          handle.close().then(() => cb(err), () => cb(err));
          return;
        }
        handles.set(fd, handle);
        cb(null, fd);
      },
      (error) => cb(error)
    );
  }

  function close(fd, callback) {
    validateCallback(callback);
    const cb = onceCallback(callback);
    // Only registry-owned handles may be closed via fs.close(fd).
    const handle = handles.get(fd);
    if (!handle) {
      // Double close / unknown fd → Node EBADF (error-first callback).
      queueMicrotask(() => cb(createEBADF(fd)));
      return;
    }
    handles.delete(fd);
    handle.close().then(() => cb(null), (e) => cb(e));
  }

  function wrap(promiseFn) {
    return function(...args) {
      const callback = args[args.length - 1];
      validateCallback(callback);
      args.pop();
      const cb = onceCallback(callback);
      promiseFn(...args).then(
        (v) => cb(null, v),
        (e) => cb(e)
      );
    };
  }

  return {
    open,
    close,
    readFile: wrap((path, opts) => fsp.readFile(path, opts)),
    writeFile: wrap((path, data, opts) => fsp.writeFile(path, data, opts)),
  };
})()"#,
            )?;
            default.set("open", cb_helpers.get::<_, Value>("open")?)?;
            default.set("close", cb_helpers.get::<_, Value>("close")?)?;
            default.set("readFile", cb_helpers.get::<_, Value>("readFile")?)?;
            default.set("writeFile", cb_helpers.get::<_, Value>("writeFile")?)?;
            default.set("readFileSync", Func::from(read_file_sync))?;
            default.set("rmdirSync", Func::from(rmdir_sync))?;
            default.set("rmSync", Func::from(rmfile_sync))?;
            default.set("unlinkSync", Func::from(rmfile_sync))?;

            let stat_fn_export = Function::new(ctx.clone(), stat_callback)?;
            let lstat_fn_export = Function::new(ctx.clone(), lstat_callback)?;
            default.set("stat", stat_fn_export)?;
            default.set("statSync", Func::from(stat_fn_sync))?;
            default.set("lstat", lstat_fn_export)?;
            default.set("lstatSync", Func::from(lstat_fn_sync))?;
            default.set("writeFileSync", Func::from(write_file_sync))?;
            default.set("watch", Func::from(watch))?;
            default.set("FSWatcher", fs_watcher_ctor)?;
            default.set("chmodSync", Func::from(chmod_sync))?;
            default.set("renameSync", Func::from(rename_sync))?;
            default.set("symlinkSync", Func::from(symlink_sync))?;
            default.set("existsSync", Func::from(exists_sync))?;
            default.set("createReadStream", create_create_read_stream(ctx)?)?;

            let realpath_sync_fn = Function::new(ctx.clone(), realpath_sync)?;
            let realpath_sync_native = Function::new(ctx.clone(), realpath_sync)?;
            realpath_sync_fn.set("native", realpath_sync_native)?;
            default.set("realpathSync", realpath_sync_fn)?;

            let realpath_fn = Function::new(ctx.clone(), realpath)?;
            let realpath_native = Function::new(ctx.clone(), realpath)?;
            realpath_fn.set("native", realpath_native)?;
            default.set("realpath", realpath_fn)?;

            Ok(())
        })
    }
}

fn exists_sync(path: String) -> bool {
    std::fs::metadata(path).is_ok()
}

fn export_promises<'js>(ctx: &Ctx<'js>, exports: &Object<'js>) -> Result<()> {
    export_constants(ctx, exports)?;

    exports.set("access", Func::from(Async(access)))?;
    exports.set("open", Func::from(Async(open)))?;
    exports.set("readFile", Func::from(Async(read_file)))?;
    exports.set("writeFile", Func::from(Async(write_file)))?;
    exports.set("rename", Func::from(Async(rename)))?;
    exports.set("readdir", Func::from(Async(read_dir)))?;
    exports.set("mkdir", Func::from(Async(mkdir)))?;
    exports.set("mkdtemp", Func::from(Async(mkdtemp)))?;
    exports.set("rm", Func::from(Async(rmfile)))?;
    exports.set("rmdir", Func::from(Async(rmdir)))?;
    exports.set("stat", Func::from(Async(stat_fn)))?;
    exports.set("lstat", Func::from(Async(lstat_fn)))?;
    exports.set("chmod", Func::from(Async(chmod)))?;
    exports.set("symlink", Func::from(Async(symlink)))?;
    exports.set("realpath", Func::from(Async(realpath_promises)))?;

    Ok(())
}

/// Create the shared fs access-mode constants object (`F_OK`/`R_OK`/`W_OK`/`X_OK`).
///
/// Single source of truth for both `fs.constants` and the legacy `constants` module.
pub fn create_constants<'js>(ctx: &Ctx<'js>) -> Result<Object<'js>> {
    let constants = Object::new(ctx.clone())?;
    constants.set("F_OK", CONSTANT_F_OK)?;
    constants.set("R_OK", CONSTANT_R_OK)?;
    constants.set("W_OK", CONSTANT_W_OK)?;
    constants.set("X_OK", CONSTANT_X_OK)?;
    Ok(constants)
}

fn export_constants<'js>(ctx: &Ctx<'js>, exports: &Object<'js>) -> Result<()> {
    exports.set("constants", create_constants(ctx)?)?;
    Ok(())
}

impl From<FsModule> for ModuleInfo<FsModule> {
    fn from(val: FsModule) -> Self {
        ModuleInfo {
            name: "fs",
            module: val,
        }
    }
}
