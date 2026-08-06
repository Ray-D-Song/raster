"use strict";

/**
 * Narrow better-sqlite3 teardown scenarios for differential compat testing.
 *
 * Usage:
 *   COMPAT_SCENARIO=addon-only node isolation.mjs
 *   COMPAT_SCENARIO=create-db node isolation.mjs
 *   COMPAT_SCENARIO=create-db-loop node isolation.mjs
 *   COMPAT_SCENARIO=exec node isolation.mjs
 *   COMPAT_SCENARIO=explicit-close node isolation.mjs
 *   COMPAT_SCENARIO=implicit node isolation.mjs
 */

const Database = require("better-sqlite3");

const scenario = process.env.COMPAT_SCENARIO ?? "full";

function done(message) {
  console.log(message);
}

switch (scenario) {
  case "addon-only":
    done("better-sqlite3 isolation OK: addon-only");
    break;

  case "create-db": {
    const db = new Database(":memory:");
    done("better-sqlite3 isolation OK: create-db");
    break;
  }

  case "create-db-loop": {
    for (let i = 0; i < 100; i++) {
      const db = new Database(":memory:");
      db.exec("CREATE TABLE t (id INTEGER PRIMARY KEY)");
      db.close();
    }
    done("better-sqlite3 isolation OK: create-db-loop");
    break;
  }

  case "create-db-null": {
    let db = new Database(":memory:");
    db = undefined;
    done("better-sqlite3 isolation OK: create-db-null");
    break;
  }

  case "exec": {
    const db = new Database(":memory:");
    db.exec("CREATE TABLE t (id INTEGER PRIMARY KEY)");
    done("better-sqlite3 isolation OK: exec");
    break;
  }

  case "explicit-close": {
    const db = new Database(":memory:");
    db.exec("CREATE TABLE t (id INTEGER PRIMARY KEY)");
    db.close();
    done("better-sqlite3 isolation OK: explicit-close");
    break;
  }

  case "implicit": {
    const db = new Database(":memory:");
    db.exec("CREATE TABLE t (id INTEGER PRIMARY KEY)");
    db.prepare("INSERT INTO t (id) VALUES (?)").run(1);
    done("better-sqlite3 isolation OK: implicit");
    break;
  }

  case "prepare": {
    const db = new Database(":memory:");
    db.exec("CREATE TABLE t (v TEXT)");
    const stmt = db.prepare("INSERT INTO t (v) VALUES (?)");
    stmt.run("x");
    stmt.get();
    done("better-sqlite3 isolation OK: prepare");
    break;
  }

  default:
    console.error(`unknown COMPAT_SCENARIO: ${scenario}`);
    process.exit(1);
}
